//! Definition of ADS Command IDs and their Payload structures.

use crate::errors::AdsError;
use std::io::{self, Read, Write};

/// The ADS Command ID identifies the type of request/response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum CommandId {
    /// Invalid command ID
    #[default]
    Invalid,
    /// Read the name and the version number of the ADS device (0x0001)
    AdsReadDeviceInfo,
    /// Read data from the ADS device. The data is addressed by the Index Group and Index Offset (0x0002)
    AdsRead,
    /// Write data to the ADS device. The data is addressed by the Index Group and Index Offset (0x0003)
    AdsWrite,
    // Read the ADS status and the device status of the ADS device (0x0004)
    AdsReadState,
    /// Change the ADS status and the device status of the ADS device. (0x0005)
    AdsWriteControl,
    /// Add a notification to the ADS device (0x0006).
    /// Data will be sent when the variable changes.
    AdsAddDeviceNotification,
    /// Delete a notification from the ADS device (0x0007).
    AdsDeleteDeviceNotification,
    /// Notification of a change in the ADS device. (0x0008)
    /// Note: This is usually sent Server -> Client.
    AdsDeviceNotification,
    /// Writes data to the ADS device and reads data back immediately (0x0009)
    AdsReadWrite,
    /// A command ID not known to this library version, probably an internal command.
    Other(u16),
}

impl From<u16> for CommandId {
    fn from(value: u16) -> Self {
        match value {
            0x0000 => Self::Invalid,
            0x0001 => Self::AdsReadDeviceInfo,
            0x0002 => Self::AdsRead,
            0x0003 => Self::AdsWrite,
            0x0004 => Self::AdsReadState,
            0x0005 => Self::AdsWriteControl,
            0x0006 => Self::AdsAddDeviceNotification,
            0x0007 => Self::AdsDeleteDeviceNotification,
            0x0008 => Self::AdsDeviceNotification,
            0x0009 => Self::AdsReadWrite,
            n => Self::Other(n),
        }
    }
}

impl From<CommandId> for u16 {
    fn from(value: CommandId) -> Self {
        match value {
            CommandId::Invalid => 0x0000,
            CommandId::AdsReadDeviceInfo => 0x0001,
            CommandId::AdsRead => 0x0002,
            CommandId::AdsWrite => 0x0003,
            CommandId::AdsReadState => 0x0004,
            CommandId::AdsWriteControl => 0x0005,
            CommandId::AdsAddDeviceNotification => 0x0006,
            CommandId::AdsDeleteDeviceNotification => 0x0007,
            CommandId::AdsDeviceNotification => 0x0008,
            CommandId::AdsReadWrite => 0x0009,
            CommandId::Other(n) => n,
        }
    }
}

impl PartialOrd for CommandId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CommandId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        u16::from(*self).cmp(&u16::from(*other))
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_id_conversion() {
        assert_eq!(CommandId::from(0x0001), CommandId::AdsReadDeviceInfo);
        assert_eq!(CommandId::from(0x0009), CommandId::AdsReadWrite);
        assert_eq!(CommandId::from(0x00FF), CommandId::Other(0x00FF));
        assert_eq!(CommandId::from(0), CommandId::Invalid);
    }

    #[test]
    fn test_command_id_from_u16() {
        assert_eq!(u16::from(CommandId::AdsReadDeviceInfo), 0x0001);
        assert_eq!(u16::from(CommandId::AdsReadWrite), 0x0009);
        assert_eq!(u16::from(CommandId::Other(123)), 123);
    }

    #[test]
    fn test_command_id_ord() {
        assert!(CommandId::AdsReadDeviceInfo < CommandId::AdsReadWrite);
    }
}
