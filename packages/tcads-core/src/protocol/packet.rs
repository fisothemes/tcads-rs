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

impl AmsPacket<Vec<u8>> {
    /// Reads a full packet from a reader (e.g. TCP Stream).
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

        // Read the Body (AMS Header (32 bytes) + Data)
        let mut body_buf = vec![0u8; total_len];
        r.read_exact(&mut body_buf)?;

        // Parse AMS Header
        let header = AmsHeader::try_from(&body_buf[0..AMS_HEADER_LEN])?;

        // Extract Data (rest, everything after the 32-byte AMS header)
        let content = body_buf[AMS_HEADER_LEN..].to_vec();

        // Perform a sanity check (make sure TCP length doesn't claim one thing and the AMS Header another)
        if content.len() as u32 != header.length() {
            return Err(AdsError::MalformedPacket(
                "TCP length doesn't match AMS Header length",
            ))?;
        }

        Ok(Self { header, content })
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
    fn test_packet_roundtrip() {
        let header = create_test_header();
        let content = vec![1, 2, 3, 4];
        let packet = AmsPacket::new(header, content.clone());

        let mut buffer = Vec::new();
        packet.write_to(&mut buffer).unwrap();

        let parsed = AmsPacket::read_from(&mut buffer.as_slice()).unwrap();
        assert_eq!(parsed.header(), packet.header());
        assert_eq!(parsed.content(), &content);
    }
}
