use super::error::AdsTypeInfoError;

/// Describes one dimension of an array type.
///
/// # Wire Format
///
/// 8 bytes fixed. Field names from `TcAdsDef.h` (`AdsDatatypeArrayInfo`):
///
/// | Offset | Size | Field           | Description                          |
/// |--------|------|-----------------|--------------------------------------|
/// | 0      | 4    | `lower_bound`   | Lower bound (LE i32)                 |
/// | 4      | 4    | `element_count` | Number of elements (LE u32)          |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AdsDataTypeArrayInfo {
    lower_bound: i32,
    element_count: u32,
}

impl AdsDataTypeArrayInfo {
    /// Wire size in bytes.
    pub const LENGTH: usize = 8;

    /// Creates a new instance of [`AdsDataTypeArrayInfo`].
    pub const fn new(lower_bound: i32, element_count: u32) -> Self {
        Self {
            lower_bound,
            element_count,
        }
    }

    /// Lower bound of this array dimension.
    /// Typically 0 or 1 in IEC 61131-3, but can be negative.
    pub fn lower_bound(&self) -> i32 {
        self.lower_bound
    }

    /// Number of elements in this dimension.
    pub fn element_count(&self) -> u32 {
        self.element_count
    }

    /// Parses from a byte slice.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsTypeInfoError> {
        if data.len() != Self::LENGTH {
            return Err(AdsTypeInfoError::TooShort {
                expected: Self::LENGTH,
                got: data.len(),
            });
        }
        Ok(Self {
            lower_bound: i32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            element_count: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        })
    }

    // Serializes to a fixed 8-byte array.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0u8; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.lower_bound.to_le_bytes());
        buf[4..8].copy_from_slice(&self.element_count.to_le_bytes());
        buf
    }
}

impl TryFrom<&[u8]> for AdsDataTypeArrayInfo {
    type Error = AdsTypeInfoError;
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

impl From<AdsDataTypeArrayInfo> for [u8; AdsDataTypeArrayInfo::LENGTH] {
    fn from(info: AdsDataTypeArrayInfo) -> Self {
        info.to_bytes()
    }
}

impl From<&AdsDataTypeArrayInfo> for [u8; AdsDataTypeArrayInfo::LENGTH] {
    fn from(info: &AdsDataTypeArrayInfo) -> Self {
        info.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Confirmed from ARRAY [0..7] OF BYTE capture: lower_bound=0, elements=8
    fn real_bytes() -> [u8; 8] {
        [0, 0, 0, 0, 8, 0, 0, 0]
    }

    #[test]
    fn parses_real_capture() {
        let info = AdsDataTypeArrayInfo::try_from_slice(&real_bytes()).unwrap();
        assert_eq!(info.lower_bound(), 0);
        assert_eq!(info.element_count(), 8);
    }

    #[test]
    fn roundtrip() {
        let original = AdsDataTypeArrayInfo::try_from_slice(&real_bytes()).unwrap();
        let bytes = original.to_bytes();
        let parsed = AdsDataTypeArrayInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn negative_lower_bound_roundtrips() {
        // PLC arrays can start at negative indices e.g. ARRAY [-5..5] OF INT
        let info = AdsDataTypeArrayInfo::new(-5, 11);
        let bytes = info.to_bytes();
        let parsed = AdsDataTypeArrayInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(parsed.lower_bound(), -5);
        assert_eq!(parsed.element_count(), 11);
    }

    #[test]
    fn too_short_returns_err() {
        assert!(matches!(
            AdsDataTypeArrayInfo::try_from_slice(&[0u8; 7]).unwrap_err(),
            AdsTypeInfoError::TooShort {
                expected: 8,
                got: 7
            }
        ));
    }
}
