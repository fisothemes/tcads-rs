/// Beckhoff AMS Router commands carried in the AMS/TCP header `reserved` field.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Default)]
pub enum AmsRouterCommand {
    /// Used for ADS commands (0x0000)
    #[default]
    AdsCommand = 0x0000,
    /// Port close command (0x0001)
    PortClose = 0x0001,
    /// Port connect command (0x1000)
    PortConnect = 0x1000,
    /// Router notification (0x1001)
    RouterNotif = 0x1001,
    /// Request local AmsNetId (0x1002)
    GetLocalNetId = 0x1002,
    /// Unknown command
    Other(u16),
}

impl From<AmsRouterCommand> for u16 {
    fn from(value: AmsRouterCommand) -> Self {
        match value {
            AmsRouterCommand::AdsCommand => 0x0000,
            AmsRouterCommand::PortClose => 0x0001,
            AmsRouterCommand::PortConnect => 0x1000,
            AmsRouterCommand::RouterNotif => 0x1001,
            AmsRouterCommand::GetLocalNetId => 0x1002,
            AmsRouterCommand::Other(n) => n,
        }
    }
}

impl From<u16> for AmsRouterCommand {
    fn from(value: u16) -> Self {
        match value {
            0x0000 => Self::AdsCommand,
            0x0001 => Self::PortClose,
            0x1000 => Self::PortConnect,
            0x1001 => Self::RouterNotif,
            0x1002 => Self::GetLocalNetId,
            n => Self::Other(n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_conversion() {
        assert_eq!(
            AmsRouterCommand::from(0x1001),
            AmsRouterCommand::RouterNotif
        );
    }
}
