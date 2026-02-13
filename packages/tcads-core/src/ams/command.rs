use super::error::AmsCommandError;

/// AMS Router Command Identifiers.
/// These identify the type of the packet at the TCP/router level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u16)]
pub enum AmsCommand {
    /// Standard ADS command (Read, Write, etc.) (`0x000`)
    #[default]
    AdsCommand = 0x0000,
    /// Close an AMS port (`0x0001`)
    PortClose = 0x0001,
    /// Register/Connect an AMS port (`0x1000`)
    PortConnect = 0x1000,
    /// Router Notification (`0x1001`)
    RouterNotification = 0x1001,
    /// Get Local NetId (`0x1002`)
    GetLocalNetId = 0x1002,
    /// Unknown/Unsupported command
    Unknown(u16),
}

impl AmsCommand {
    /// The length of an AMS command identifier in bytes.
    pub const LENGTH: usize = 2;

    /// Creates a new [`AmsCommand`] from a byte array.
    pub fn from_bytes(bytes: [u8; AmsCommand::LENGTH]) -> Self {
        u16::from_le_bytes(bytes).into()
    }

    /// Converts the current instance into a byte array.
    pub fn to_bytes(&self) -> [u8; AmsCommand::LENGTH] {
        (*self).into()
    }

    /// Creates a new [`AmsCommand`] from a byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AmsCommandError> {
        bytes.try_into()
    }
}

impl From<AmsCommand> for u16 {
    fn from(value: AmsCommand) -> Self {
        match value {
            AmsCommand::AdsCommand => 0x0000,
            AmsCommand::PortClose => 0x0001,
            AmsCommand::PortConnect => 0x1000,
            AmsCommand::RouterNotification => 0x1001,
            AmsCommand::GetLocalNetId => 0x1002,
            AmsCommand::Unknown(n) => n,
        }
    }
}

impl From<u16> for AmsCommand {
    fn from(value: u16) -> Self {
        match value {
            0x0000 => Self::AdsCommand,
            0x0001 => Self::PortClose,
            0x1000 => Self::PortConnect,
            0x1001 => Self::RouterNotification,
            0x1002 => Self::GetLocalNetId,
            n => Self::Unknown(n),
        }
    }
}

impl From<[u8; AmsCommand::LENGTH]> for AmsCommand {
    fn from(bytes: [u8; AmsCommand::LENGTH]) -> Self {
        u16::from_le_bytes(bytes).into()
    }
}

impl From<AmsCommand> for [u8; AmsCommand::LENGTH] {
    fn from(value: AmsCommand) -> Self {
        let value: u16 = value.into();
        value.to_le_bytes()
    }
}

impl TryFrom<&[u8]> for AmsCommand {
    type Error = AmsCommandError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != AmsCommand::LENGTH {
            return Err(AmsCommandError::InvalidBufferSize {
                expected: AmsCommand::LENGTH,
                got: bytes.len(),
            });
        }
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_conversion() {
        assert_eq!(AmsCommand::from(0x0000), AmsCommand::AdsCommand);
        assert_eq!(AmsCommand::from(0x0001), AmsCommand::PortClose);
        assert_eq!(AmsCommand::from(0x1000), AmsCommand::PortConnect);
        assert_eq!(AmsCommand::from(0x1001), AmsCommand::RouterNotification);
        assert_eq!(AmsCommand::from(0x1002), AmsCommand::GetLocalNetId);
        assert_eq!(AmsCommand::from(0x1234), AmsCommand::Unknown(0x1234));
    }
}
