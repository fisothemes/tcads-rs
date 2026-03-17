use super::error::RunTimeTypeError;
use std::fmt::{self, Display};

/// The runtime type of the TwinCAT system.
///
/// Returned by [`GetRuntimeTypeResponse`](crate::protocol::GetRuntimeTypeResponse).
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum RuntimeType {
    /// Real-time runtime (standard TwinCAT on Windows).
    Rt,
    /// User-mode runtime (TC/BSD, TC/RTOS, etc.).
    Um,
    /// An unknown runtime type value.
    Unknown(u32),
}

impl RuntimeType {
    /// The length of the runtime type in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new [`RuntimeType`] from a 4-byte array (Little Endian).
    pub fn from_bytes(bytes: [u8; RuntimeType::LENGTH]) -> Self {
        u32::from_le_bytes(bytes).into()
    }

    /// Converts the current instance into a 4-byte array (Little Endian).
    pub fn to_bytes(&self) -> [u8; RuntimeType::LENGTH] {
        (*self).into()
    }

    /// Creates a new [`RuntimeType`] from a byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, RunTimeTypeError> {
        bytes.try_into()
    }
}

impl From<u32> for RuntimeType {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Rt,
            1 => Self::Um,
            n => Self::Unknown(n),
        }
    }
}

impl From<RuntimeType> for u32 {
    fn from(value: RuntimeType) -> Self {
        match value {
            RuntimeType::Rt => 0,
            RuntimeType::Um => 1,
            RuntimeType::Unknown(n) => n,
        }
    }
}

impl From<[u8; RuntimeType::LENGTH]> for RuntimeType {
    fn from(bytes: [u8; RuntimeType::LENGTH]) -> Self {
        u32::from_le_bytes(bytes).into()
    }
}

impl From<RuntimeType> for [u8; RuntimeType::LENGTH] {
    fn from(value: RuntimeType) -> Self {
        value.to_bytes()
    }
}

impl TryFrom<&[u8]> for RuntimeType {
    type Error = RunTimeTypeError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != RuntimeType::LENGTH {
            return Err(RunTimeTypeError::InvalidBufferSize {
                expected: RuntimeType::LENGTH,
                got: bytes.len(),
            });
        }
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).into())
    }
}

impl Display for RuntimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rt => write!(f, "RT"),
            Self::Um => write!(f, "UM"),
            Self::Unknown(n) => write!(f, "Unknown ({n})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_type_conversion() {
        assert_eq!(RuntimeType::from(0), RuntimeType::Rt);
        assert_eq!(RuntimeType::from(1), RuntimeType::Um);
        assert_eq!(RuntimeType::from(2), RuntimeType::Unknown(2));
    }

    #[test]
    fn test_runtime_type_to_u32() {
        assert_eq!(0, u32::from(RuntimeType::Rt));
        assert_eq!(1, u32::from(RuntimeType::Um));
        assert_eq!(2, u32::from(RuntimeType::Unknown(2)));
    }
}
