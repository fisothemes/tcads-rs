use crate::constants::{AMS_HEADER_LEN, AMS_TCP_HEADER_LEN};
use crate::errors::AdsError;
use crate::protocol::header::{AmsHeader, AmsTcpHeader};
use crate::protocol::packet::AmsPacket;
use std::io::{Read, Write};

/// Stateless Codec for reading and writing ADS Packets over a stream (e.g. TCP).
///
/// The AMS Codec handles this framing logic:
/// - **Writing:** Calculates the length, prepends the TCP header, and serializes the packet.
/// - **Reading:** Reads the TCP header first to determine how many bytes to read next, ensuring a
/// complete message is parsed.
pub struct AmsCodec;

impl AmsCodec {
    /// Encodes a Packet into the Writer (e.g. TcpStream)
    pub fn write<W: Write>(w: &mut W, packet: &AmsPacket<Vec<u8>>) -> Result<usize, AdsError> {
        let content_len = packet.content().len();

        let total_packet_len = (AMS_HEADER_LEN + content_len) as u32;
        let tcp_header = AmsTcpHeader::new(total_packet_len);

        tcp_header.write_to(w)?;
        packet.header().write_to(w)?;
        w.write_all(packet.content())?;
        w.flush()?;

        Ok(AMS_TCP_HEADER_LEN + AMS_HEADER_LEN + content_len)
    }

    /// Decodes a Packet from a Reader (e.g. TcpStream)
    pub fn read<R: Read>(r: &mut R) -> Result<AmsPacket<Vec<u8>>, AdsError> {
        todo!()
    }
}
