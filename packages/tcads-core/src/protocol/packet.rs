use super::header::{AmsHeader, AmsTcpHeader};
use crate::constants::{AMS_HEADER_LEN, AMS_TCP_HEADER_LEN};
use crate::errors::AdsError;
use std::io::{self, Read, Write};

/// An ADS/AMS Packet.
///
/// This struct represents the logical packet:
/// - **AMS Header** (Routing, Command ID, Error Code)
/// - **Content** (The payload/data)
///
/// # Note
///
/// When serialized via `write_to`, you must automatically prepend the 6-byte
/// **AMS/TCP Header** required for network transmission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmsPacket<B = Vec<u8>> {
    header: AmsHeader,
    content: B,
}

impl<B> AmsPacket<B> {
    /// Creates a new packet.
    pub fn new(header: AmsHeader, content: B) -> Self {
        Self { header, content }
    }

    /// Returns a reference to the AMS Header.
    pub fn header(&self) -> &AmsHeader {
        &self.header
    }

    /// Returns a reference to the packet's content.
    pub fn content(&self) -> &B {
        &self.content
    }

    /// Returns the packet's header and content, consuming the packet.
    pub fn into_parts(self) -> (AmsHeader, B) {
        (self.header, self.content)
    }
}

impl<B: AsRef<[u8]>> AmsPacket<B> {
    /// Writes the full wire format: TCP Header + AMS Header + Content.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<usize> {
        let content = self.content.as_ref();

        // Write TCP Header
        AmsTcpHeader::new((AMS_HEADER_LEN + content.len()) as u32).write_to(w)?;
        // Write AMS Header
        self.header.write_to(w)?;
        // Write Content
        w.write_all(content)?;

        Ok(AMS_TCP_HEADER_LEN + AMS_HEADER_LEN + content.len())
    }
}

impl<B: From<Vec<u8>>> AmsPacket<B> {
    /// Decodes a byte stream into a new `AmsPacket`.
    ///
    /// This method allocates a new vector sized exactly to the incoming payload,
    /// then converts it into the type `B`.
    pub fn read_from<R: Read>(r: &mut R) -> Result<Self, AdsError> {
        // Read TCP Header (6 bytes)
        let mut tcp_buf = [0u8; AMS_TCP_HEADER_LEN];
        r.read_exact(&mut tcp_buf)?;
        let tcp_header = AmsTcpHeader::from(&tcp_buf);

        let total_len = tcp_header.length() as usize;

        if total_len < AMS_HEADER_LEN {
            return Err(AdsError::MalformedPacket(
                "TCP length smaller than AMS Header",
            ));
        }

        // Read AMS Header (32 bytes)
        let mut header_buf = [0u8; AMS_HEADER_LEN];
        r.read_exact(&mut header_buf)?;
        let header = AmsHeader::try_from(&header_buf[..])?;

        // Calculate content length
        let content_len = total_len - AMS_HEADER_LEN;

        // Sanity Check: TCP Frame vs AMS Header
        if (content_len as u32) != header.length() {
            return Err(AdsError::MalformedPacket(
                "TCP length doesn't match AMS Header length",
            ))?;
        }

        // Read Content (Remaining bytes)
        let mut content_buf = vec![0u8; content_len];
        r.read_exact(&mut content_buf)?;

        // Convert extracted data into B
        let content = B::from(content_buf);

        Ok(Self { header, content })
    }
}

impl<B: AsMut<[u8]> + AsRef<[u8]>> AmsPacket<B> {
    /// Reads a packet from the stream into the existing content buffer.
    ///
    /// Returns `Ok(payload_len)` indicating how many bytes of the buffer were used.
    pub fn read_into<R: Read>(&mut self, r: &mut R) -> Result<usize, AdsError> {
        // Read TCP Header (6 bytes)
        let mut tcp_buf = [0u8; AMS_TCP_HEADER_LEN];
        r.read_exact(&mut tcp_buf)?;
        let tcp_header = AmsTcpHeader::from(&tcp_buf);

        let total_len = tcp_header.length() as usize;
        if total_len < AMS_HEADER_LEN {
            return Err(AdsError::MalformedPacket("TCP length too short"));
        }

        // Read AMS Header (32 bytes)
        let mut header_buf = [0u8; AMS_HEADER_LEN];
        r.read_exact(&mut header_buf)?;

        self.header = AmsHeader::try_from(&header_buf[..])?;

        // Calculate content length
        let content_len = total_len - AMS_HEADER_LEN;

        // Sanity Check: TCP Frame vs AMS Header
        if (content_len as u32) != self.header.length() {
            return Err(AdsError::MalformedPacket("Length Mismatch"));
        }

        // Buffer Capacity Check
        if content_len > self.content.as_ref().len() {
            return Err(AdsError::MalformedPacket("Packet too large for buffer"));
        }

        // Read content and write into the slice range that corresponds to the payload
        let dest = &mut self.content.as_mut()[0..content_len];
        r.read_exact(dest)?;

        // Return the number of bytes written to the content buffer
        Ok(content_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AdsReturnCode;
    use crate::protocol::commands::CommandId;
    use crate::protocol::state_flags::StateFlag;

    fn create_test_header() -> AmsHeader {
        AmsHeader::new(
            "127.0.0.1.1.1:851".parse().unwrap(),
            "192.168.0.2.1.1:40000".parse().unwrap(),
            CommandId::AdsRead,
            StateFlag::tcp_ads_response(),
            4,
            AdsReturnCode::Ok,
            12_345,
        )
    }

    #[test]
    fn test_packet_roundtrip_with_read_from() {
        let header = create_test_header();
        let content = vec![1, 2, 3, 4];
        let packet = AmsPacket::new(header, content.clone());

        let mut buffer = Vec::new();
        packet.write_to(&mut buffer).unwrap();

        let parsed: AmsPacket<Vec<u8>> = AmsPacket::read_from(&mut buffer.as_slice()).unwrap();
        assert_eq!(parsed.header(), packet.header());
        assert_eq!(parsed.content(), &content);
    }

    #[test]
    fn test_read_from_constructs_valid_packet() {
        let mut stream = Vec::new();
        let header = create_test_header(); // Uses 32 bytes
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF]; // 4 bytes

        let total_len = AMS_HEADER_LEN as u32 + payload.len() as u32;
        stream.extend_from_slice(&0u16.to_le_bytes()); // Reserved
        stream.extend_from_slice(&total_len.to_le_bytes()); // Length

        header.write_to(&mut stream).unwrap();
        stream.extend_from_slice(&payload);

        let mut reader = &stream[..];
        let packet: AmsPacket<Vec<u8>> =
            AmsPacket::read_from(&mut reader).expect("Should read successfully");

        assert_eq!(packet.header().command_id(), CommandId::AdsRead);
        assert_eq!(packet.content(), &payload);
        assert_eq!(
            packet.header().length(),
            4,
            "Header length should match payload"
        );
    }

    #[test]
    fn test_read_into_stack_buffer() {
        let mut stream = Vec::new();
        let incoming_header = create_test_header();
        let payload = vec![0xCA, 0xFE, 0xBA, 0xBE];

        let total_ams_len = AMS_HEADER_LEN as u32 + payload.len() as u32;
        stream.extend_from_slice(&0u16.to_le_bytes());
        stream.extend_from_slice(&total_ams_len.to_le_bytes());
        incoming_header.write_to(&mut stream).unwrap();
        stream.extend_from_slice(&payload);

        // Setup "Heapless" Environment
        let mut raw_buffer = [0u8; 128];

        let dummy_header = create_test_header();
        let mut packet = AmsPacket::new(dummy_header, &mut raw_buffer[..]);

        let mut reader = &stream[..];
        let bytes_read = packet
            .read_into(&mut reader)
            .expect("Should read into buffer");

        assert_eq!(bytes_read, 4, "Should return exactly the payload size");
        assert_eq!(&packet.content()[0..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
        assert_eq!(packet.header().invoke_id(), 12345);
    }
}
