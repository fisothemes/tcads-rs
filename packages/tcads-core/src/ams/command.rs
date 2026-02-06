/// AMS Router Command Identifiers.
/// These identify the type of the packet at the TCP/router level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u16)]
pub enum AmsCommand {
    /// Standard ADS command (Read, Write, etc.)
    #[default]
    AdsCommand = 0x0000,
    /// Close an AMS port
    PortClose = 0x0001,
    /// Register/Connect an AMS port
    PortConnect = 0x1000,
    /// Router Notification
    RouterNotification = 0x1001,
    /// Get Local NetId
    GetLocalNetId = 0x1002,
    /// Unknown/Unsupported command
    Unknown(u16),
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
