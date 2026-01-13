use super::header::{AmsHeader, AmsTcpHeader};
use crate::errors::AdsError;
use std::io::{self, Read, Write};

/// An ADS/AMS Packet.
///
/// This struct represents the logical packet:
/// - **AMS Header** (Routing, Command ID, Error Code)
/// - **Content** (The payload/data)
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
        todo!()
    }
}

impl AmsPacket<Vec<u8>> {
    /// Reads a full packet from a reader (e.g. TCP Stream).
    pub fn read_from<R: Read>(r: &mut R) -> Result<Self, AdsError> {
        todo!()
    }
}
