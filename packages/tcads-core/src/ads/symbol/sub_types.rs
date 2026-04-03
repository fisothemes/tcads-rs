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
mod array_info_tests {
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

/// A key-value attribute attached to a type or symbol entry.
///
/// Corresponds to `{attribute 'key' := 'value'}` pragmas in TwinCAT structured text.
///
/// # Wire Format
///
/// | Offset | Size | Field                                  |
/// |--------|------|----------------------------------------|
/// | 0      | 1    | `name_len` (u8, excl. null)            |
/// | 1      | 1    | `value_len` (u8, excl. null)           |
/// | 2      | n+1  | `name` (null-terminated Windows-1252)  |
/// | 3+n    | m+1  | `value` (null-terminated Windows-1252) |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsAttribute {
    pub name: String,
    pub value: String,
}

impl AdsAttribute {
    /// Minimum wire size: 2 length bytes + 2 null terminators.
    pub const MIN_LENGTH: usize = 4;

    /// Creates a new instance of [`AdsAttribute`].
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Returns the attribute key/name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the attribute value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the wire size of this attribute in bytes.
    pub fn wire_size(&self) -> usize {
        2 + self.name.len() + 1 + self.value.len() + 1
    }

    /// Parses from a byte slice.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsTypeInfoError> {
        if data.len() < Self::MIN_LENGTH {
            return Err(AdsTypeInfoError::TooShort {
                expected: Self::MIN_LENGTH,
                got: data.len(),
            });
        }
        let name_len = data[0] as usize;
        let value_len = data[1] as usize;
        let name_end = 2 + name_len + 1;
        let value_end = name_end + value_len + 1;

        if data.len() < value_end {
            return Err(AdsTypeInfoError::UnexpectedLength {
                expected: value_end,
                got: data.len(),
            });
        }

        // TwinCAT strings are Windows-1252, not strictly UTF-8.
        // This prevents panics on characters like '°C' or umlauts.
        let (name, _, _) = encoding_rs::WINDOWS_1252.decode(&data[2..name_end.saturating_sub(1)]);
        let (value, _, _) =
            encoding_rs::WINDOWS_1252.decode(&data[name_end..value_end.saturating_sub(1)]);

        Ok(Self {
            name: name.into_owned(),
            value: value.into_owned(),
        })
    }

    /// Serializes this attribute to a byte vector.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.wire_size());

        // Truncate lengths to 255 to fit in u8 per the protocol spec
        buf.push(self.name.len().min(255) as u8);
        buf.push(self.value.len().min(255) as u8);

        buf.extend_from_slice(self.name.as_bytes());
        buf.push(0); // Null terminator

        buf.extend_from_slice(self.value.as_bytes());
        buf.push(0); // Null terminator

        buf
    }
}

impl TryFrom<&[u8]> for AdsAttribute {
    type Error = AdsTypeInfoError;
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

impl From<AdsAttribute> for Vec<u8> {
    fn from(attr: AdsAttribute) -> Self {
        attr.to_vec()
    }
}

#[cfg(test)]
mod attribute_tests {
    use super::*;

    #[test]
    fn parses_attribute() {
        // attribute 'unit' := 'mm'
        // name_len: 4, value_len: 2
        // "unit\0" "mm\0"
        let bytes = [4, 2, b'u', b'n', b'i', b't', 0, b'm', b'm', 0];

        let attr = AdsAttribute::try_from_slice(&bytes).unwrap();
        assert_eq!(attr.name(), "unit");
        assert_eq!(attr.value(), "mm");
        assert_eq!(attr.wire_size(), 10);
    }

    #[test]
    fn parses_windows_1252_encoding() {
        // attribute 'unit' := '°C' (degree symbol is 0xB0 in Windows-1252)
        let bytes = [4, 2, b'u', b'n', b'i', b't', 0, 0xB0, b'C', 0];

        let attr = AdsAttribute::try_from_slice(&bytes).unwrap();
        assert_eq!(attr.value(), "°C");
    }

    #[test]
    fn roundtrip() {
        let original = AdsAttribute::new("DisplayMinValue", "0");
        let bytes = original.to_vec();
        let parsed = AdsAttribute::try_from_slice(&bytes).unwrap();
        assert_eq!(original, parsed);
    }
}
