/// The transition mode for Device Notifications.
///
/// Determines when the server sends a notification to the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)] // Wire format is 4 bytes
pub enum AdsTransMode {
    /// No transmission.
    None = 0,
    /// Cyclic transmission. The server checks the variable cyclically.
    ClientCycle = 1,
    /// Transmission on change. The server checks the variable cyclically and sends only if it changed.
    ClientOnChange = 2,
    /// Cyclic transmission (Server-driven). (Not commonly used by clients).
    ServerCycle = 3,
    /// Transmission on change (Server-driven). (Not commonly used by clients).
    ServerOnChange = 4,
    /// Unknown/Custom mode.
    Unknown(u32),
}

impl From<u32> for AdsTransMode {
    fn from(val: u32) -> Self {
        match val {
            0 => Self::None,
            1 => Self::ClientCycle,
            2 => Self::ClientOnChange,
            3 => Self::ServerCycle,
            4 => Self::ServerOnChange,
            n => Self::Unknown(n),
        }
    }
}

impl From<AdsTransMode> for u32 {
    fn from(val: AdsTransMode) -> Self {
        match val {
            AdsTransMode::None => 0,
            AdsTransMode::ClientCycle => 1,
            AdsTransMode::ClientOnChange => 2,
            AdsTransMode::ServerCycle => 3,
            AdsTransMode::ServerOnChange => 4,
            AdsTransMode::Unknown(n) => n,
        }
    }
}

/// The ADS State of the device.
///
/// Describes the current operating state (e.g. Run, Stop, Config).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)] // Wire format is 2 bytes
pub enum AdsState {
    Invalid = 0,
    Idle = 1,
    Reset = 2,
    Init = 3,
    Start = 4,
    Run = 5,
    Stop = 6,
    SaveCfg = 7,
    LoadCfg = 8,
    PowerFailure = 9,
    PowerGood = 10,
    Error = 11,
    Shutdown = 12,
    Suspend = 13,
    Resume = 14,
    Config = 15,
    Reconfig = 16,
    Stopping = 17,
    /// System is incompatible.
    Incompatible = 18,
    /// System Exception.
    Exception = 19,
    /// A state not defined in the library.
    Unknown(u16),
}

impl From<u16> for AdsState {
    fn from(val: u16) -> Self {
        match val {
            0 => Self::Invalid,
            1 => Self::Idle,
            2 => Self::Reset,
            3 => Self::Init,
            4 => Self::Start,
            5 => Self::Run,
            6 => Self::Stop,
            7 => Self::SaveCfg,
            8 => Self::LoadCfg,
            9 => Self::PowerFailure,
            10 => Self::PowerGood,
            11 => Self::Error,
            12 => Self::Shutdown,
            13 => Self::Suspend,
            14 => Self::Resume,
            15 => Self::Config,
            16 => Self::Reconfig,
            17 => Self::Stopping,
            18 => Self::Incompatible,
            19 => Self::Exception,
            n => Self::Unknown(n),
        }
    }
}

impl From<AdsState> for u16 {
    fn from(val: AdsState) -> Self {
        match val {
            AdsState::Invalid => 0,
            AdsState::Idle => 1,
            AdsState::Reset => 2,
            AdsState::Init => 3,
            AdsState::Start => 4,
            AdsState::Run => 5,
            AdsState::Stop => 6,
            AdsState::SaveCfg => 7,
            AdsState::LoadCfg => 8,
            AdsState::PowerFailure => 9,
            AdsState::PowerGood => 10,
            AdsState::Error => 11,
            AdsState::Shutdown => 12,
            AdsState::Suspend => 13,
            AdsState::Resume => 14,
            AdsState::Config => 15,
            AdsState::Reconfig => 16,
            AdsState::Stopping => 17,
            AdsState::Incompatible => 18,
            AdsState::Exception => 19,
            AdsState::Unknown(n) => n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ads_state_conversion() {
        assert_eq!(AdsState::from(5), AdsState::Run);
        assert_eq!(u16::from(AdsState::Run), 5);

        assert_eq!(AdsState::from(100), AdsState::Unknown(100));
        assert_eq!(u16::from(AdsState::Unknown(100)), 100);
    }

    #[test]
    fn test_trans_mode_conversion() {
        assert_eq!(AdsTransMode::from(1), AdsTransMode::ClientCycle);
        assert_eq!(u32::from(AdsTransMode::ClientCycle), 1);

        assert_eq!(AdsTransMode::from(99), AdsTransMode::Unknown(99));
        assert_eq!(u32::from(AdsTransMode::Unknown(99)), 99);
    }
}
