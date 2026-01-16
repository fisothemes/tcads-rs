//! Definition of ADS Request Payloads

use std::io::{self, Read, Write};

/// Payload for `CommandId::AdsRead`.
///
/// Direction: Client -> Server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsReadRequest {
    index_group: u32,
    index_offset: u32,
    length: u32,
}

impl AdsReadRequest {
    pub const SIZE: usize = 12;

    pub fn new(index_group: u32, index_offset: u32, length: u32) -> Self {
        Self {
            index_group,
            index_offset,
            length,
        }
    }

    /// Returns the Index Group of the data which should be read.
    pub fn index_group(&self) -> u32 {
        self.index_group
    }

    /// Returns the Index Offset of the data which should be read.
    pub fn index_offset(&self) -> u32 {
        self.index_offset
    }

    /// Returns the length of the data (in bytes) which should be read.
    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.index_group.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())?;
        Ok(())
    }

    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 12];
        r.read_exact(&mut buf)?;
        Ok(Self {
            index_group: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            index_offset: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
            length: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
        })
    }
}
