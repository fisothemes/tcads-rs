use super::error::RouterStateError;

/// AMS Router state codes.
///
/// Represents the operational state of the AMS Router.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum RouterState {
    /// Router is stopped (0)
    Stop,
    /// Router is started/running (1)
    Start,
    /// Router/route was removed (2)
    Removed,
    /// Unknown state
    Unknown(u32),
}

impl RouterState {
    /// Length of the [`RouterState`] in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new [`RouterState`] from a byte array.
    pub fn from_bytes(bytes: [u8; RouterState::LENGTH]) -> Self {
        u32::from_le_bytes(bytes).into()
    }

    /// Converts the current instance into a byte array.
    pub fn to_bytes(&self) -> [u8; RouterState::LENGTH] {
        (*self).into()
    }

    /// Creates a new [`RouterState`] from a byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, RouterStateError> {
        bytes.try_into()
    }
}

impl From<u32> for RouterState {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Stop,
            1 => Self::Start,
            2 => Self::Removed,
            n => Self::Unknown(n),
        }
    }
}

impl From<RouterState> for u32 {
    fn from(value: RouterState) -> Self {
        match value {
            RouterState::Stop => 0,
            RouterState::Start => 1,
            RouterState::Removed => 2,
            RouterState::Unknown(n) => n,
        }
    }
}

impl From<[u8; RouterState::LENGTH]> for RouterState {
    fn from(bytes: [u8; RouterState::LENGTH]) -> Self {
        u32::from_le_bytes(bytes).into()
    }
}

impl From<RouterState> for [u8; RouterState::LENGTH] {
    fn from(value: RouterState) -> Self {
        let value: u32 = value.into();
        value.to_le_bytes()
    }
}

impl TryFrom<&[u8]> for RouterState {
    type Error = RouterStateError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != RouterState::LENGTH {
            return Err(RouterStateError::InvalidBufferSize {
                expected: RouterState::LENGTH,
                got: bytes.len(),
            });
        }
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).into())
    }
}

impl std::fmt::Display for RouterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stop => write!(f, "Stop"),
            Self::Start => write!(f, "Start"),
            Self::Removed => write!(f, "Removed"),
            Self::Unknown(_) => write!(f, "Unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_conversion() {
        assert_eq!(RouterState::from(0), RouterState::Stop);
        assert_eq!(RouterState::from(1), RouterState::Start);
        assert_eq!(RouterState::from(2), RouterState::Removed);
        assert_eq!(RouterState::from(3), RouterState::Unknown(3));
    }

    #[test]
    fn test_state_bytes() {
        assert_eq!(RouterState::Stop.to_bytes(), [0, 0, 0, 0]);
        assert_eq!(RouterState::Start.to_bytes(), [1, 0, 0, 0]);
        assert_eq!(RouterState::Removed.to_bytes(), [2, 0, 0, 0]);
        assert_eq!(RouterState::Unknown(100).to_bytes(), [100, 0, 0, 0]);
    }

    #[test]
    fn test_state_from_bytes() {
        assert_eq!(RouterState::from_bytes([0, 0, 0, 0]), RouterState::Stop);
        assert_eq!(RouterState::from_bytes([1, 0, 0, 0]), RouterState::Start);
        assert_eq!(RouterState::from_bytes([2, 0, 0, 0]), RouterState::Removed);
        assert_eq!(
            RouterState::from_bytes([100, 0, 0, 0]),
            RouterState::Unknown(100)
        );
    }

    #[test]
    fn test_state_try_from_slice() {
        assert_eq!(
            RouterState::try_from_slice(&[0, 0, 0, 0]).unwrap(),
            RouterState::Stop
        );
        assert_eq!(
            RouterState::try_from_slice(&[1, 0, 0, 0]).unwrap(),
            RouterState::Start
        );
        assert_eq!(
            RouterState::try_from_slice(&[2, 0, 0, 0]).unwrap(),
            RouterState::Removed
        );
        assert_eq!(
            RouterState::try_from_slice(&[100, 0, 0, 0]).unwrap(),
            RouterState::Unknown(100)
        );
    }

    #[test]
    fn test_serde_router_state_roundtrip() {
        let state = RouterState::Start;
        let s = serde_json::to_string(&state).unwrap();
        assert_eq!(state, serde_json::from_str(&s).unwrap());
    }
}
