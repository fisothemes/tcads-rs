use super::error::AdsError;

/// ADS Command IDs used within the AMS Header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdsCommand {
    /// Invalid command ID
    Invalid,
    /// Read the name and the version number of the ADS device (0x0001)
    AdsReadDeviceInfo,
    /// Read data from the ADS device. The data is addressed by the Index Group and Index Offset (0x0002)
    AdsRead,
    /// Write data to the ADS device. The data is addressed by the Index Group and Index Offset (0x0003)
    AdsWrite,
    /// Read the ADS status and the device status of the ADS device (0x0004)
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

impl AdsCommand {
    /// The length of the ADS Command ID in bytes.
    pub const LENGTH: usize = 2;

    /// Creates a new `AdsCommand` from a 2-byte array (Little Endian).
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from(bytes)
    }

    /// Converts the command to a 2-byte array (Little Endian).
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        (*self).into()
    }

    /// Tries to parse an `AdsCommand` from a byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AdsError> {
        bytes.try_into()
    }
}

impl From<u16> for AdsCommand {
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

impl From<AdsCommand> for u16 {
    fn from(value: AdsCommand) -> Self {
        match value {
            AdsCommand::Invalid => 0x0000,
            AdsCommand::AdsReadDeviceInfo => 0x0001,
            AdsCommand::AdsRead => 0x0002,
            AdsCommand::AdsWrite => 0x0003,
            AdsCommand::AdsReadState => 0x0004,
            AdsCommand::AdsWriteControl => 0x0005,
            AdsCommand::AdsAddDeviceNotification => 0x0006,
            AdsCommand::AdsDeleteDeviceNotification => 0x0007,
            AdsCommand::AdsDeviceNotification => 0x0008,
            AdsCommand::AdsReadWrite => 0x0009,
            AdsCommand::Other(n) => n,
        }
    }
}

impl From<[u8; Self::LENGTH]> for AdsCommand {
    fn from(bytes: [u8; Self::LENGTH]) -> Self {
        u16::from_le_bytes(bytes).into()
    }
}

impl From<AdsCommand> for [u8; AdsCommand::LENGTH] {
    fn from(cmd: AdsCommand) -> Self {
        u16::from(cmd).to_le_bytes()
    }
}

impl TryFrom<&[u8]> for AdsCommand {
    type Error = AdsError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < AdsCommand::LENGTH {
            return Err(AdsError::InvalidBufferSize {
                item: "AdsCommand",
                expected: AdsCommand::LENGTH,
                found: bytes.len(),
            });
        }
        Ok(Self::from([bytes[0], bytes[1]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_id_conversion() {
        assert_eq!(AdsCommand::from(0x0001), AdsCommand::AdsReadDeviceInfo);
        assert_eq!(AdsCommand::from(0x0009), AdsCommand::AdsReadWrite);
        assert_eq!(AdsCommand::from(0x00FF), AdsCommand::Other(0x00FF));
        assert_eq!(AdsCommand::from(0), AdsCommand::Invalid);
    }

    #[test]
    fn test_command_id_from_u16() {
        assert_eq!(u16::from(AdsCommand::AdsReadDeviceInfo), 0x0001);
        assert_eq!(u16::from(AdsCommand::AdsReadWrite), 0x0009);
        assert_eq!(u16::from(AdsCommand::Other(123)), 123);
    }

    #[test]
    fn test_command_id_ord() {
        assert!(AdsCommand::AdsReadDeviceInfo < AdsCommand::AdsReadWrite);
    }

    #[test]
    fn test_command_id_bytes() {
        assert_eq!(AdsCommand::AdsReadDeviceInfo.to_bytes(), [0x01, 0x00]);
    }

    #[test]
    fn test_command_id_from_bytes() {
        assert_eq!(
            AdsCommand::from_bytes([0x01, 0x00]),
            AdsCommand::AdsReadDeviceInfo
        );
    }

    #[test]
    fn test_command_id_try_from_slice() {
        assert_eq!(
            AdsCommand::try_from_slice(&[0x01, 0x00]).unwrap(),
            AdsCommand::AdsReadDeviceInfo
        );
    }
}
