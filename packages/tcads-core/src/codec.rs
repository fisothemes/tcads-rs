use crate::errors::AdsError;
use crate::protocol::packet::AmsPacket;
use std::io::{Read, Write};

/// Stateless Codec for reading and writing ADS Packets over a stream (e.g. TCP).
///
/// The AMS Codec handles this framing logic:
/// - **Writing:** Calculates the length, prepends the TCP header, and serializes the packet.
/// - **Reading:** Reads the TCP header first to determine how many bytes to read next, ensuring a
///   complete message is parsed.
pub struct AmsCodec;

impl AmsCodec {
    /// Encodes a Packet into the Writer (e.g. TcpStream).
    ///
    /// This is generic over `B`, so it works with `AmsPacket<Vec<u8>>`, `AmsPacket<&[u8]>`,
    /// or any other type that can be viewed as a byte slice.
    pub fn write<W: Write, B: AsRef<[u8]>>(
        w: &mut W,
        packet: &AmsPacket<B>,
    ) -> Result<usize, AdsError> {
        let bytes_written = packet.write_to(w)?;
        w.flush()?;
        Ok(bytes_written)
    }

    /// Decodes a Packet from a Reader (e.g. TcpStream).
    ///
    /// This is generic over `B`, allowing you to return `AmsPacket<Vec<u8>>` or
    /// any other type that implements `From<Vec<u8>>`.
    pub fn read<R: Read, B: From<Vec<u8>>>(r: &mut R) -> Result<AmsPacket<B>, AdsError> {
        AmsPacket::read_from(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AdsReturnCode;
    use crate::protocol::commands::CommandId;
    use crate::protocol::header::AmsHeader;
    use crate::protocol::state_flags::StateFlag;
    use std::io::Cursor;

    fn create_test_header() -> AmsHeader {
        AmsHeader::new(
            "1.2.3.4.1.1:851".parse().unwrap(),
            "10.20.30.40.1.1:30000".parse().unwrap(),
            CommandId::AdsRead,
            StateFlag::tcp_ads_response(),
            4,
            AdsReturnCode::Ok,
            12_345,
        )
    }

    #[test]
    fn test_codec_write() {
        let payload = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let header = create_test_header();
        let packet = AmsPacket::new(header, payload);

        // Simulate TcpStream
        let mut buffer = Vec::new();
        let mut writer = Cursor::new(&mut buffer);

        let written_bytes = AmsCodec::write(&mut writer, &packet).unwrap();

        // Verifying length.
        // Expected Length = TCP Header (6) + AMS Header (32) + Payload (4) = 42
        assert_eq!(written_bytes, 42);
        assert_eq!(buffer.len(), 42);

        // Checking TCP Header: [Reserved(0,0), Length(36, 0, 0, 0)]
        // Length = 32 (AMS Header) + 4 (Payload) = 36 (0x24)
        assert_eq!(&buffer[0..6], &[0x00, 0x00, 0x24, 0x00, 0x00, 0x00]);

        // Check a known part of AMS Header (Command ID at offset 6+16=22)
        // CommandId::AdsRead = 2
        assert_eq!(buffer[22], 0x02);
        assert_eq!(buffer[23], 0x00);

        // Check Payload at the end
        assert_eq!(&buffer[38..42], &[0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn test_codec_read() {
        let mut buffer = Vec::new();

        let header = create_test_header();
        let payload = vec![0x11, 0x22, 0x33, 0x44];
        let packet = AmsPacket::new(header, payload.clone());

        let mut writer = Cursor::new(&mut buffer);
        AmsCodec::write(&mut writer, &packet).unwrap();

        let mut reader = Cursor::new(&buffer);

        let read_packet: AmsPacket<Vec<u8>> =
            AmsCodec::read(&mut reader).expect("Codec read failed");

        assert_eq!(read_packet.header().invoke_id(), 12_345);
        assert_eq!(read_packet.content(), &payload);
    }
}
