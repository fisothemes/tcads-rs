/// AMS Router Command Identifiers.
/// These identify the type of the packet at the TCP/router level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u16)]
pub enum AmsRouterCommand {
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

impl From<AmsRouterCommand> for u16 {
    fn from(value: AmsRouterCommand) -> Self {
        match value {
            AmsRouterCommand::AdsCommand => 0x0000,
            AmsRouterCommand::PortClose => 0x0001,
            AmsRouterCommand::PortConnect => 0x1000,
            AmsRouterCommand::RouterNotification => 0x1001,
            AmsRouterCommand::GetLocalNetId => 0x1002,
            AmsRouterCommand::Unknown(n) => n,
        }
    }
}

impl From<u16> for AmsRouterCommand {
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
        assert_eq!(AmsRouterCommand::from(0x0000), AmsRouterCommand::AdsCommand);

        assert_eq!(AmsRouterCommand::from(0x0001), AmsRouterCommand::PortClose);

        assert_eq!(
            AmsRouterCommand::from(0x1000),
            AmsRouterCommand::PortConnect
        );

        assert_eq!(
            AmsRouterCommand::from(0x1001),
            AmsRouterCommand::RouterNotification
        );

        assert_eq!(
            AmsRouterCommand::from(0x1002),
            AmsRouterCommand::GetLocalNetId
        );

        assert_eq!(
            AmsRouterCommand::from(0x1234),
            AmsRouterCommand::Unknown(0x1234)
        );
    }
}
