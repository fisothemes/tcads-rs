use super::error::AdsTransModeError;

/// The transition mode for Device Notifications.
///
/// Determines when the server sends a notification to the client.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum AdsTransMode {
    /// No transmission.
    None,
    /// Cyclic transmission. The server checks the variable cyclically.
    ClientCycle,
    /// Transmission on change. The server checks the variable cyclically and sends only if it changed.
    ClientOnChange,
    /// Cyclic transmission (Server-driven). (Not commonly used by clients).
    ServerCycle,
    /// Transmission on change (Server-driven). (Not commonly used by clients).
    ServerOnChange,
    /// Unknown/Custom mode.
    Unknown(u32),
}

impl AdsTransMode {
    /// The length of the ADS Transmission Mode in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new `AdsTransMode` from a 4-byte array (Little Endian).
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from(bytes)
    }

    /// Converts the mode to a 4-byte array (Little Endian).
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        (*self).into()
    }

    /// Tries to parse an `AdsTransMode` from a byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AdsTransModeError> {
        bytes.try_into()
    }
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

impl From<[u8; Self::LENGTH]> for AdsTransMode {
    fn from(bytes: [u8; Self::LENGTH]) -> Self {
        u32::from_le_bytes(bytes).into()
    }
}

impl From<AdsTransMode> for [u8; AdsTransMode::LENGTH] {
    fn from(mode: AdsTransMode) -> Self {
        u32::from(mode).to_le_bytes()
    }
}

impl TryFrom<&[u8]> for AdsTransMode {
    type Error = AdsTransModeError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < AdsTransMode::LENGTH {
            return Err(AdsTransModeError::UnexpectedLength {
                expected: AdsTransMode::LENGTH,
                got: bytes.len(),
            });
        }
        Ok(Self::from([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ads_trans_mode_conversion() {
        assert_eq!(AdsTransMode::from(0), AdsTransMode::None);
        assert_eq!(u32::from(AdsTransMode::None), 0);

        assert_eq!(AdsTransMode::from(1), AdsTransMode::ClientCycle);
        assert_eq!(u32::from(AdsTransMode::ClientCycle), 1);

        assert_eq!(AdsTransMode::from(2), AdsTransMode::ClientOnChange);
        assert_eq!(u32::from(AdsTransMode::ClientOnChange), 2);

        assert_eq!(AdsTransMode::from(3), AdsTransMode::ServerCycle);
        assert_eq!(u32::from(AdsTransMode::ServerCycle), 3);

        assert_eq!(AdsTransMode::from(4), AdsTransMode::ServerOnChange);
        assert_eq!(u32::from(AdsTransMode::ServerOnChange), 4);

        assert_eq!(AdsTransMode::from(100), AdsTransMode::Unknown(100));
        assert_eq!(u32::from(AdsTransMode::Unknown(100)), 100);
    }

    #[test]
    fn test_ads_trans_mode_bytes() {
        assert_eq!(AdsTransMode::ClientOnChange.to_bytes(), [2, 0, 0, 0]);
    }

    #[test]
    fn test_ads_trans_mode_from_bytes() {
        assert_eq!(
            AdsTransMode::from_bytes([2, 0, 0, 0]),
            AdsTransMode::ClientOnChange
        );
    }

    #[test]
    fn test_ads_trans_mode_try_from_slice() {
        assert_eq!(
            AdsTransMode::try_from_slice(&[2, 0, 0, 0]).unwrap(),
            AdsTransMode::ClientOnChange
        );
    }

    #[test]
    fn test_serde_trans_mode_roundtrip() {
        let mode = AdsTransMode::ClientCycle;
        let s = serde_json::to_string(&mode).unwrap();
        assert_eq!(mode, serde_json::from_str(&s).unwrap());
    }
}
