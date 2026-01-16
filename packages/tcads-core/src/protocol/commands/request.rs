//! Definition of ADS Request Payloads

use std::io::{self, Read, Write};

/// Payload for [`CommandId::AdsRead`](super::CommandId::AdsRead).
///
/// Direction: Client -> Server
///
/// # Layout
/// - **Index Group:** 4 bytes
/// - **Index Offset:** 4 bytes
/// - **Length:** 4 bytes (How many bytes to read)
///
/// ```text
/// [ Index Group (4) ] [ Index Offset (4) ] [ Length (4) ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsReadRequest {
    index_group: u32,
    index_offset: u32,
    length: u32,
}

impl AdsReadRequest {
    /// Size of the fixed header of the request.
    pub const SIZE: usize = 12;

    /// Creates a new AdsReadRequest.
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

    /// Writes the fixed header of the request.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.index_group.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the request.
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
