use crate::ams::{AMS_TCP_HEADER_LEN, AmsCommand, AmsTcpHeader};
use std::io::{self, Read, Write};

/// Maximum allowed AMS frame/packet size (64KB) to prevent allocation attacks.
pub const AMS_FRAME_MAX_LEN: usize = 65535 - AMS_TCP_HEADER_LEN;

/// A single AMS frame/packet consisting of a header and a payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AmsFrame {
    header: AmsTcpHeader,
    payload: Vec<u8>,
}

impl AmsFrame {
    /// Creates a new frame with the given command and payload.
    ///
    /// Accepts `Vec<u8>` (move) or `&[u8]` (copy).
    /// # Note
    ///
    /// The payload length is clamped [u32::MAX] to prevent overflows.
    pub fn new<B: Into<Vec<u8>>>(command: AmsCommand, payload: B) -> Self {
        let payload = payload.into();
        let payload_len = payload.len().min(u32::MAX as usize) as u32;
        Self {
            header: AmsTcpHeader::new(command, payload_len),
            payload,
        }
    }

    /// Creates a new frame with the given command and empty payload.
    pub fn empty(command: AmsCommand) -> Self {
        Self::new(command, Vec::new())
    }

    /// Returns the frame's header.
    pub fn header(&self) -> &AmsTcpHeader {
        &self.header
    }

    /// Returns the frame's payload.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Splits the frame into its header and payload.
    pub fn into_parts(self) -> (AmsTcpHeader, Vec<u8>) {
        (self.header, self.payload)
    }

    /// Returns the frame as a byte vector.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(AMS_TCP_HEADER_LEN + self.payload.len());
        vec.extend_from_slice(&self.header.to_bytes());
        vec.extend_from_slice(&self.payload);
        vec
    }

    /// Reads a frame from a reader and returns it.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let header = AmsTcpHeader::read_from(r)?;

        if header.length() as usize > AMS_FRAME_MAX_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Payload too large: {} bytes (max {AMS_FRAME_MAX_LEN} bytes)",
                    header.length()
                ),
            ));
        }

        let mut payload = vec![0u8; header.length() as usize];
        r.read_exact(&mut payload)?;

        Ok(Self { header, payload })
    }

    /// Reads a frame's payload into the provided mutable slice.
    ///
    /// # Note
    ///
    /// The buffer is payload-only (no header bytes), and only the first AMS/TCP header length bytes are filled.
    ///
    /// Errors if the buffer is too small.
    pub fn read_into<R: Read>(r: &mut R, payload_buf: &mut [u8]) -> io::Result<AmsTcpHeader> {
        let header = AmsTcpHeader::read_from(r)?;

        let expected_len = header.length() as usize;

        if expected_len > payload_buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Buffer too small for payload: need at least {} bytes, got {} bytes",
                    expected_len,
                    payload_buf.len()
                ),
            ));
        }

        r.read_exact(&mut payload_buf[..expected_len])?;

        Ok(header)
    }

    /// Writes a frame into a writer.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header.write_to(w)?;
        w.write_all(&self.payload)
    }

    /// Writes a frame given as bytes into a writer.
    ///
    /// # Note
    ///
    /// The buffer must start with a TCP header and contain at least `header.length()` bytes of payload.
    /// Extra bytes in the buffer are ignored.
    pub fn write_into<W: Write>(w: &mut W, buf: &[u8]) -> io::Result<()> {
        if buf.len() < AMS_TCP_HEADER_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Buffer too small for header: need at least {} bytes, got {}",
                    AMS_TCP_HEADER_LEN,
                    buf.len()
                ),
            ));
        }

        let header = AmsTcpHeader::try_from_slice(&buf[..AMS_TCP_HEADER_LEN])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        let actual_payload_len = buf.len() - AMS_TCP_HEADER_LEN;
        let expected_payload_len = header.length() as usize;

        if actual_payload_len < expected_payload_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Buffer too small for payload: need at least {} bytes, got {}",
                    expected_payload_len, actual_payload_len
                ),
            ));
        }

        w.write_all(&buf[..AMS_TCP_HEADER_LEN + expected_payload_len])
    }
}

impl From<(AmsCommand, Vec<u8>)> for AmsFrame {
    fn from((command, payload): (AmsCommand, Vec<u8>)) -> Self {
        Self::new(command, payload)
    }
}

impl From<AmsFrame> for (AmsCommand, Vec<u8>) {
    fn from(frame: AmsFrame) -> Self {
        (frame.header().command(), frame.payload().to_vec())
    }
}

impl From<AmsFrame> for Vec<u8> {
    fn from(frame: AmsFrame) -> Self {
        frame.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_from_reads_header_and_payload() {
        // [Port Connect (0x1000)] [length (4)] [payload (AA BB CC DD)]
        let data = [0x00, 0x10, 0x04, 0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD];

        let mut cursor = Cursor::new(data);

        let (header, payload) = AmsFrame::read_from(&mut cursor).unwrap().split();

        assert_eq!(header.command(), AmsCommand::PortConnect);
        assert_eq!(header.length(), 4);
        assert_eq!(payload, &[0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn read_from_rejects_payloads_over_max() {
        let header = AmsTcpHeader::new(AmsCommand::PortConnect, (AMS_FRAME_MAX_LEN + 1) as u32);

        let mut data = Vec::new();
        data.extend_from_slice(&header.to_bytes());

        let err = AmsFrame::read_from(&mut Cursor::new(data)).unwrap_err();

        assert_eq!(
            err.kind(),
            io::ErrorKind::InvalidData,
            "The error we got as string {err}"
        );
    }

    #[test]
    fn read_into_reads_payload_only() {
        // [Port Connect (0x0001)] [length (4)] [payload (01 02 03 04)]
        let data = [0x01, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04];
        let mut cursor = Cursor::new(data);

        let mut buf = [0xAAu8; 8];
        let header = AmsFrame::read_into(&mut cursor, &mut buf).unwrap();

        assert_eq!(header.command(), AmsCommand::PortClose);
        assert_eq!(&buf[..4], &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(&buf[4..], &[0xAAu8; 4]);
    }

    #[test]
    fn write_to_roundtrips_with_read_from() {
        let payload = vec![9u8, 8, 7];
        let frame = AmsFrame::new(AmsCommand::RouterNotification, payload.clone());

        let mut out = Vec::new();
        frame.write_to(&mut out).unwrap();

        let mut cursor = Cursor::new(out);
        let parsed = AmsFrame::read_from(&mut cursor).unwrap();
        assert_eq!(parsed.header().command(), AmsCommand::RouterNotification);
        assert_eq!(parsed.payload(), payload.as_slice());
    }

    #[test]
    fn write_into_ignores_extra_payload_bytes() {
        // [Port Connect (0x0001)] [length (3)] [payload (10 20 30 EE FF 00)]
        let data = [
            0x00, 0x10, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20, 0x30, 0xEE, 0xFF, 0x00,
        ];

        let mut out = Vec::new();
        AmsFrame::write_into(&mut out, &data).unwrap();

        assert_eq!(out.len(), AMS_TCP_HEADER_LEN + 3);
        assert_eq!(&out[..], &data[..AMS_TCP_HEADER_LEN + 3]);
    }
}
