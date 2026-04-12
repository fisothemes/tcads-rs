use super::error::AdsTypeInfoError;

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
    name: String,
    value: String,
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
mod tests {
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
