//! Definition of ADS Request Payloads

use std::io::{self, Read, Write};

use crate::types::enums::AdsState;

/// Payload for [`CommandId::AdsRead`](super::CommandId::AdsRead).
///
/// Direction: Client -> Server
///
/// A request to read data from an ADS device.
/// The data is addressed by the Index Group and the Index Offset
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

/// Payload Header for [`CommandId::AdsWrite`](super::CommandId::AdsWrite).
///
/// Direction: Client -> Server
///
/// A request to write data to an ADS device.
/// The data is addressed by the Index Group and the Index Offset
///
/// # Layout
/// - **Index Group:** 4 bytes
/// - **Index Offset:** 4 bytes
/// - **Length:** 4 bytes (Size of the data to write)
///
/// # Usage
/// This struct parses the *fixed header* of the request.
/// The data to be written immediately follows this structure in the stream.
///
/// ```text
/// [ Index Group (4) ] [ Index Offset (4) ] [ Length (4) ] [ Data (n bytes...) ]
/// ^-----------------------------------------------------^
///              AdsWriteRequest parses this
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsWriteRequest {
    index_group: u32,
    index_offset: u32,
    length: u32,
}

impl AdsWriteRequest {
    /// Size of the fixed header of the request.
    pub const SIZE: usize = 12;

    pub fn new(index_group: u32, index_offset: u32, length: u32) -> Self {
        Self {
            index_group,
            index_offset,
            length,
        }
    }

    /// Returns the Index Group in which the data should be written.
    pub fn index_group(&self) -> u32 {
        self.index_group
    }

    /// Returns the Index Offset in which the data should be written.
    pub fn index_offset(&self) -> u32 {
        self.index_offset
    }

    /// Returns the length of the data (in bytes) which are to be written.
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

/// Payload Header for [`CommandId::AdsReadWrite`](super::CommandId::AdsReadWrite).
///
/// Direction: Client -> Server
///
/// A request to write data to an ADS device and immediately read data back.
///
/// # Layout
/// - **Index Group:** 4 bytes
/// - **Index Offset:** 4 bytes
/// - **Read Length:** 4 bytes (Bytes expected in response)
/// - **Write Length:** 4 bytes (Bytes to write)
///
/// # Usage
/// This struct parses the *fixed header* of the request.
/// The data to be written immediately follows this structure in the stream.
///
/// ```text
/// [ Group (4) ] [ Offset (4) ] [ ReadLen (4) ] [ WriteLen (4) ] [ Write Data (n bytes...) ]
/// ^-----------------------------------------------------------^
///                AdsReadWriteRequest parses this
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsReadWriteRequest {
    index_group: u32,
    index_offset: u32,
    read_length: u32,
    write_length: u32,
}

impl AdsReadWriteRequest {
    /// Size of the fixed header of the request.
    pub const SIZE: usize = 16;

    pub fn new(index_group: u32, index_offset: u32, read_length: u32, write_length: u32) -> Self {
        Self {
            index_group,
            index_offset,
            read_length,
            write_length,
        }
    }

    /// Returns the Index Group in which the data should be written.
    pub fn index_group(&self) -> u32 {
        self.index_group
    }

    /// Returns the Index Offset in which the data should be written.
    pub fn index_offset(&self) -> u32 {
        self.index_offset
    }

    /// Returns the length of the data (in bytes) which are to be read.
    pub fn read_length(&self) -> u32 {
        self.read_length
    }

    /// Returns the length of the data (in bytes) which are to be written.
    pub fn write_length(&self) -> u32 {
        self.write_length
    }

    /// Writes the fixed header of the request.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.index_group.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        w.write_all(&self.read_length.to_le_bytes())?;
        w.write_all(&self.write_length.to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the request.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 16];
        r.read_exact(&mut buf)?;
        Ok(Self {
            index_group: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            index_offset: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
            read_length: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            write_length: u32::from_le_bytes(buf[12..16].try_into().unwrap()),
        })
    }
}

/// Payload for [`CommandId::AdsWriteControl`](super::CommandId::AdsWriteControl).
///
/// Direction: Client -> Server
///
/// Changes the ADS state and Device state of the target. Additionally, it is possible to
/// send data to the target to transfer further information. These data were not analysed
/// from the current ADS devices (PLC, NC, ...).
///
/// # Layout
/// - **ADS State:** 2 bytes (The target state to switch to)
/// - **Device State:** 2 bytes (Usually 0)
/// - **Length:** 4 bytes (Size of additional data)
///
/// ```text
/// [ AdsState (2) ] [ DevState (2) ] [ Length (4) ] [ Data... ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsWriteControlRequest {
    ads_state: AdsState,
    device_state: u16,
    length: u32,
}

impl AdsWriteControlRequest {
    /// Size of the fixed header of the request.
    pub const SIZE: usize = 8;

    pub fn new(ads_state: AdsState, device_state: u16, length: u32) -> Self {
        Self {
            ads_state,
            device_state,
            length,
        }
    }

    /// Returns the ADS state which should be set on the target.
    pub fn ads_state(&self) -> AdsState {
        self.ads_state
    }

    /// Returns the Device state which should be set on the target.
    pub fn device_state(&self) -> u16 {
        self.device_state
    }

    /// Returns the length of the additional data which should be sent to the target.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Writes the fixed header of the request.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&u16::from(self.ads_state).to_le_bytes())?;
        w.write_all(&self.device_state.to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the request.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self {
            ads_state: AdsState::from(u16::from_le_bytes(buf[0..2].try_into().unwrap())),
            device_state: u16::from_le_bytes(buf[2..4].try_into().unwrap()),
            length: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
        })
    }
}
