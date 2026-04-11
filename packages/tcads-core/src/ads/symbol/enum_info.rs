pub use super::AdsAttribute;
use crate::ads::symbol::error::AdsTypeInfoError;

/// One variant of a TwinCAT enum type.
///
/// The byte width of `value` matches the parent [`AdsDataTypeInfo::size`](super::AdsTypeInfo::size)
/// field. For example, 1 byte for `BYTE`-based enums, 2 for `WORD`, 4 for `DWORD`.
///
/// # Wire Format
///
/// Parsed in two separate passes. The standard section appears in the enum info
/// block; the extended section appears later when
/// [`AdsDataTypeFlags::EXTENDED_ENUM_INFOS`](super::AdsTypeFlags::EXTENDED_ENUM_INFOS) is set.
///
/// ## Standard section
///
/// | Offset | Size          | Field      | Description                    |
/// |--------|---------------|------------|--------------------------------|
/// | 0      | 1             | `name_len` | Byte length of name excl. null |
/// | 1      | `name_len + 1`| `name`     | Windows-1252, null-terminated  |
/// | varies | `value_size`  | `value`    | Raw LE bytes, width = enum size|
///
/// ## Extended section
///
/// | Offset | Size           | Field         | Description                    |
/// |--------|----------------|---------------|--------------------------------|
/// | 0      | 2              | `entry_length`| Total size of this section     |
/// | 2      | 1              | `comment_len` | Byte length of comment         |
/// | 3      | 1              | `attr_count`  | Number of attributes           |
/// | 4      | `comment_len+1`| `comment`     | Windows-1252, null-terminated  |
/// | varies | varies         | `attributes`  | [`AdsAttribute`] entries       |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsEnumInfo {
    name: String,
    value: Vec<u8>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    comment: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<AdsAttribute>,
}

impl AdsEnumInfo {
    /// Creates a new [`AdsEnumInfo`] instance with just a name and value.
    pub fn new(name: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            comment: String::new(),
            attributes: Vec::new(),
        }
    }

    /// Creates a fully populated [`AdsEnumInfo`] instance including extended metadata.
    pub fn new_ext(
        name: impl Into<String>,
        value: impl Into<Vec<u8>>,
        comment: impl Into<String>,
        attributes: impl Into<Vec<AdsAttribute>>,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            comment: comment.into(),
            attributes: attributes.into(),
        }
    }

    /// Consumes the struct and returns it with updated extended metadata.
    pub fn with_ext(
        mut self,
        comment: impl Into<String>,
        attributes: impl Into<Vec<AdsAttribute>>,
    ) -> Self {
        self.comment = comment.into();
        self.attributes = attributes.into();
        self
    }

    /// Consumes the struct and returns it with an updated comment.
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = comment.into();
        self
    }

    /// Consumes the struct and returns it with updated attributes.
    pub fn with_attributes(mut self, attributes: impl Into<Vec<AdsAttribute>>) -> Self {
        self.attributes = attributes.into();
        self
    }

    /// The name of the enum variant (e.g., "E_MachineState").
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The raw byte value of this variant. The length matches the parent Enum's byte size.
    pub fn value(&self) -> &[u8] {
        &self.value
    }

    /// Optional comment attached to this variant.
    pub fn comment(&self) -> &str {
        &self.comment
    }

    /// Optional attributes attached to this variant.
    pub fn attributes(&self) -> &[AdsAttribute] {
        &self.attributes
    }

    /// Parses an Enum Info entry from a byte slice and extracts the name and value.
    ///
    /// Note: `value_size` is required because the raw value length is dictated by the parent type.
    pub fn try_from_slice(
        data: &[u8],
        value_size: usize,
    ) -> Result<(Self, usize), AdsTypeInfoError> {
        if data.is_empty() {
            return Err(AdsTypeInfoError::TooShort {
                expected: 1,
                got: 0,
            });
        }

        let name_len = data[0] as usize;
        let name_end = 1 + name_len + 1;

        let total_size = name_end + value_size;
        if data.len() < total_size {
            return Err(AdsTypeInfoError::TooShort {
                expected: total_size,
                got: data.len(),
            });
        }

        let (name, _, _) = encoding_rs::WINDOWS_1252.decode(&data[1..name_end.saturating_sub(1)]);
        let value = data[name_end..total_size].to_vec();

        Ok((
            Self {
                name: name.into_owned(),
                value,
                comment: String::new(),
                attributes: Vec::new(),
            },
            total_size,
        ))
    }

    /// Parsing the trailing Extended Enum (comment + attributes) payload and mutates the struct.
    /// Returns the number of bytes consumed so the cursor can advance.
    pub fn extend_from_slice(&mut self, data: &[u8]) -> Result<usize, AdsTypeInfoError> {
        const MIN_LENGTH: usize = 4;

        if data.len() < MIN_LENGTH {
            return Err(AdsTypeInfoError::TooShort {
                expected: MIN_LENGTH,
                got: data.len(),
            });
        }

        let entry_length = u16::from_le_bytes([data[0], data[1]]) as usize;
        if data.len() < entry_length {
            return Err(AdsTypeInfoError::EntryLengthMismatch {
                expected: entry_length,
                got: data.len(),
            });
        }

        let comment_len = data[2] as usize;
        let attr_count = data[3] as usize;

        let mut pos = MIN_LENGTH;

        let comment_end = pos + comment_len + 1;
        if comment_len > 0 {
            let (c, _, _) =
                encoding_rs::WINDOWS_1252.decode(&data[pos..comment_end.saturating_sub(1)]);
            self.comment = c.into_owned();
        }
        pos = comment_end;

        self.attributes.reserve(attr_count);
        for _ in 0..attr_count {
            let attr = AdsAttribute::try_from_slice(&data[pos..])?;
            pos += attr.wire_size();
            self.attributes.push(attr);
        }

        Ok(entry_length)
    }

    /// Serializes the enum info into a tuple of byte vectors for standard and extended metadata.
    ///
    /// Returns `(standard_bytes, extended_bytes)`:
    /// - **`standard_bytes`**: The base wire format (name length, name, null terminator, raw value).
    /// - **`extended_bytes`**: The extended metadata (entry length, comment, attributes).
    ///   If the enum has no comment or attributes, this vector will be empty.
    pub fn to_vec(&self) -> (Vec<u8>, Vec<u8>) {
        let mut standard = Vec::new();

        let (name_enc, _, _) = encoding_rs::WINDOWS_1252.encode(&self.name);

        standard.push(name_enc.len().min(255) as u8);
        standard.extend_from_slice(&name_enc);
        standard.push(0); // Null terminator
        standard.extend_from_slice(&self.value);

        let mut extended = Vec::new();

        if !self.comment.is_empty() || !self.attributes.is_empty() {
            extended.extend_from_slice(&[0, 0]);

            let (comment_enc, _, _) = encoding_rs::WINDOWS_1252.encode(&self.comment);

            extended.push(comment_enc.len().min(255) as u8);
            extended.push(self.attributes.len().min(255) as u8);

            if !comment_enc.is_empty() {
                extended.extend_from_slice(&comment_enc);
            }
            extended.push(0); // The parser always requires a null terminator

            for attr in &self.attributes {
                extended.extend_from_slice(&attr.to_vec());
            }

            let len = extended.len() as u16;
            extended[0..2].copy_from_slice(&len.to_le_bytes());
        }

        (standard, extended)
    }
}

impl TryFrom<&[u8]> for AdsEnumInfo {
    type Error = AdsTypeInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let (info, _) = Self::try_from_slice(value, 0)?;
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enum_builders() {
        let enum_info = AdsEnumInfo::new("PS_None", [0x00])
            .with_comment("No persistent variables restored")
            .with_attributes([AdsAttribute::new("Display", "None")]);

        assert_eq!(enum_info.name(), "PS_None");
        assert_eq!(enum_info.value(), &[0x00]);
        assert_eq!(enum_info.comment(), "No persistent variables restored");
        assert_eq!(enum_info.attributes().len(), 1);
    }

    #[test]
    fn test_parse_standard_enum() {
        let bytes = [7, b'P', b'S', b'_', b'N', b'o', b'n', b'e', 0, 0x00];

        let (info, consumed) = AdsEnumInfo::try_from_slice(&bytes, 1).unwrap();

        assert_eq!(consumed, 10);
        assert_eq!(info.name(), "PS_None");
        assert_eq!(info.value(), &[0x00]);
        assert!(info.comment().is_empty());
        assert!(info.attributes().is_empty());
    }

    #[test]
    fn test_roundtrip_standard_enum() {
        let original = AdsEnumInfo::new("PS_All", vec![0x01]);

        let (std_bytes, ext_bytes) = original.to_vec();

        assert!(ext_bytes.is_empty());

        let (parsed, _) = AdsEnumInfo::try_from_slice(&std_bytes, 1).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_roundtrip_extended_enum() {
        let original = AdsEnumInfo::new_ext(
            "PS_Partial",
            [0x02],
            "Fewer variables were restored",
            [AdsAttribute::new("State", "Warning")],
        );

        let (std_bytes, ext_bytes) = original.to_vec();

        let (mut parsed, _) = AdsEnumInfo::try_from_slice(&std_bytes, 1).unwrap();

        assert_eq!(parsed.name(), "PS_Partial");
        assert_eq!(parsed.value(), &[0x02]);
        assert!(parsed.comment().is_empty());

        assert!(!ext_bytes.is_empty());
        let consumed = parsed.extend_from_slice(&ext_bytes).unwrap();

        assert_eq!(consumed, ext_bytes.len());

        assert_eq!(original, parsed);
    }
}
