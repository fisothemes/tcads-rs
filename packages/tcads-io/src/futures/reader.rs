use futures::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};
use std::io;
use tcads_core::{AMS_FRAME_MAX_LEN, AmsFrame, AmsTcpHeader};

/// A buffered reader specialised for parsing AMS frames from an asynchronous byte stream.
///
/// This struct wraps an underlying async reader in a [`BufReader`] to minimise system calls
/// when reading the 6-byte [AMS/TCP header](AmsTcpHeader) and the variable-length payload.
pub struct AmsReader<R: AsyncRead + Unpin> {
    reader: BufReader<R>,
}

impl<R: AsyncRead + Unpin> AmsReader<R> {
    /// Creates a new AmsReader with [default buffering](BufReader::new).
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    /// Creates a new [`AmsReader`] with a specific buffer capacity.
    pub fn with_capacity(reader: R, capacity: usize) -> Self {
        Self {
            reader: BufReader::with_capacity(capacity, reader),
        }
    }

    /// Reads a single AMS frame from the underlying stream.
    ///
    /// Steps:
    /// 1. Detect clean EOF (returns `UnexpectedEof`).
    /// 2. Read the 6-byte AMS/TCP header.
    /// 3. Validate payload length against [`AMS_FRAME_MAX_LEN`].
    /// 4. Read the exact payload.
    pub async fn read_frame(&mut self) -> io::Result<AmsFrame> {
        if self.reader.fill_buf().await?.is_empty() {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }

        let mut header_buf = [0u8; AmsTcpHeader::LENGTH];
        self.reader.read_exact(&mut header_buf).await?;
        let header = AmsTcpHeader::from(header_buf);

        let payload_len = header.length() as usize;
        if payload_len > AMS_FRAME_MAX_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Payload too large: {} bytes (max {})",
                    payload_len, AMS_FRAME_MAX_LEN
                ),
            ));
        }

        let mut payload = vec![0u8; payload_len];
        self.reader.read_exact(&mut payload).await?;

        Ok(AmsFrame::from_parts(header, payload))
    }

    /// Consumes the reader, returning the underlying stream.
    ///
    /// # Note
    ///
    /// Any leftover data in the internal buffer is discarded.
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }

    /// Returns a reference to the underlying reader.
    ///
    /// # Note
    ///
    /// It is not advisable to directly read from the underlying reader
    pub fn get_ref(&self) -> &R {
        self.reader.get_ref()
    }

    /// Returns a mutable reference to the underlying reader.
    ///
    /// # Note
    ///
    /// It is inadvisable to directly read from the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        self.reader.get_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use futures::io::Cursor;
    use tcads_core::AmsCommand;

    #[test]
    fn read_frame_reads_one_frame() {
        let data = AmsFrame::new(AmsCommand::PortConnect, [0x01, 0x02]).to_vec();
        let mut reader = AmsReader::new(Cursor::new(data));

        let frame = block_on(reader.read_frame()).expect("Should read valid frame");

        assert_eq!(frame.header().command(), AmsCommand::PortConnect);
        assert_eq!(frame.header().length(), 2);
        assert_eq!(frame.payload(), &[0x01, 0x02]);
    }

    #[test]
    fn test_clean_eof() {
        let mut reader = AmsReader::new(Cursor::new(Vec::<u8>::new()));

        let err = block_on(reader.read_frame()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn test_dirty_eof_in_header() {
        // Only 2 bytes of a 6-byte header
        let partial = vec![0x00, 0x10];
        let mut reader = AmsReader::new(Cursor::new(partial));

        let err = block_on(reader.read_frame()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn test_payload_too_large() {
        let mut header = [0u8; AmsTcpHeader::LENGTH];
        let bad_len = (AMS_FRAME_MAX_LEN as u32 + 1).to_le_bytes();
        header[2..6].copy_from_slice(&bad_len);

        let mut reader = AmsReader::new(Cursor::new(header.to_vec()));

        let err = block_on(reader.read_frame()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Payload too large"));
    }

    #[test]
    fn read_multiple_frames() {
        let mut data = Vec::new();
        data.extend_from_slice(&AmsFrame::new(AmsCommand::AdsCommand, [0xAA]).to_vec());
        data.extend_from_slice(&AmsFrame::new(AmsCommand::GetLocalNetId, [0xBB, 0xCC]).to_vec());

        let mut reader = AmsReader::new(Cursor::new(data));

        let f1 = block_on(reader.read_frame()).unwrap();
        assert_eq!(f1.header().command(), AmsCommand::AdsCommand);
        assert_eq!(f1.payload(), &[0xAA]);

        let f2 = block_on(reader.read_frame()).unwrap();
        assert_eq!(f2.header().command(), AmsCommand::GetLocalNetId);
        assert_eq!(f2.payload(), &[0xBB, 0xCC]);

        let err = block_on(reader.read_frame()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }
}
