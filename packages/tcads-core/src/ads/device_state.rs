use super::error::AdsStateError;

/// The device status of the ADS device.
///
/// # Note
///
/// The documentation is extremely unclear about the meaning of this value.
///
/// - **For a TwinCAT PLC:** It is almost always `0`.
/// - **For Custom ADS Servers:** If you write your own ADS Server,
///   you can put whatever status flags you want in there
///   (e.g. bitmask for "Overheating", "Door Open").
pub type DeviceState = u16;

/// The ADS State of the device.
///
/// Describes the current operating state (e.g. Run, Stop, Config).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl AdsState {
    /// The length of the ADS State in bytes.
    pub const LENGTH: usize = 2;

    /// Creates a new `AdsState` from a 2-byte array (Little Endian).
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from(bytes)
    }

    /// Converts the state to a 2-byte array (Little Endian).
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        (*self).into()
    }

    /// Tries to parse an `AdsState` from a byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AdsStateError> {
        bytes.try_into()
    }
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

impl From<[u8; Self::LENGTH]> for AdsState {
    fn from(bytes: [u8; Self::LENGTH]) -> Self {
        u16::from_le_bytes(bytes).into()
    }
}

impl From<AdsState> for [u8; AdsState::LENGTH] {
    fn from(state: AdsState) -> Self {
        u16::from(state).to_le_bytes()
    }
}

impl TryFrom<&[u8]> for AdsState {
    type Error = AdsStateError;

    fn try_from(bytes: &[u8]) -> Result<Self, <AdsState as TryFrom<&[u8]>>::Error> {
        if bytes.len() < AdsState::LENGTH {
            return Err(AdsStateError::UnexpectedLength {
                expected: AdsState::LENGTH,
                got: bytes.len(),
            });
        }
        Ok(Self::from([bytes[0], bytes[1]]))
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
    fn test_ads_state_bytes() {
        assert_eq!(AdsState::Idle.to_bytes(), [1, 0]);
    }

    #[test]
    fn test_ads_state_from_bytes() {
        assert_eq!(AdsState::from_bytes([1, 0]), AdsState::Idle);
    }

    #[test]
    fn test_ads_state_try_from_slice() {
        assert_eq!(AdsState::try_from_slice(&[1, 0]).unwrap(), AdsState::Idle);
    }
}
