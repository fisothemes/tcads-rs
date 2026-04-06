use super::error::AdsTypeInfoError;
use super::{AdsAttribute, AdsDataTypeId, AdsMethodParamFlags, Guid};

/// Represents a parameter for an RPC Method attached to a TwinCAT Struct/Function Block.
///
/// # Wire Format
///
/// | Offset | Size | Field             | Description                         |
/// |--------|------|-------------------|-------------------------------------|
/// | 0      | 4    | `entry_length`    | Total size including dynamic tail   |
/// | 4      | 4    | `size`            | Parameter size in bytes             |
/// | 8      | 4    | `align_size`      | Alignment size                      |
/// | 12     | 4    | `type_id`         | Base data type ID                   |
/// | 16     | 4    | `flags`           | IN, OUT, BY_REF, etc.               |
/// | 20     | 4    | `reserved`        | Ignored                             |
/// | 24     | 16   | `type_guid`       | GUID of the parameter type          |
/// | 40     | 2    | `length_is_param` | Index of array length parameter + 1 |
/// | 42     | 2    | `name_len`        | Byte length of name excl. null      |
/// | 44     | 2    | `type_name_len`   | Byte length of type name excl. null |
/// | 46     | 2    | `comment_len`     | Byte length of comment excl. null   |
/// | 48     | dyn  | `Strings`         | Windows-1252, null-terminated       |
/// | dyn    | dyn  | `Attributes`      | Only if `ATTRIBUTES` flag is set    |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsMethodParamInfo {
    size: u32,
    align_size: u32,
    type_id: AdsDataTypeId,
    flags: AdsMethodParamFlags,
    guid: Guid,
    length_is_param: u16,
    name: String,
    type_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    comment: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<AdsAttribute>,
}

impl AdsMethodParamInfo {
    /// Fixed size of the method parameter header before dynamic attributes.
    pub const MIN_LENGTH: usize = 48;

    /// Creates a new instance of [`AdsMethodParamInfo`] without a comment or attribute.
    pub fn new(
        size: u32,
        align_size: u32,
        type_id: AdsDataTypeId,
        flags: AdsMethodParamFlags,
        guid: Guid,
        length_is_param: u16,
        name: impl Into<String>,
        type_name: impl Into<String>,
    ) -> Self {
        Self {
            size,
            align_size,
            type_id,
            flags,
            guid,
            length_is_param,
            name: name.into(),
            type_name: type_name.into(),
            comment: String::new(),
            attributes: Vec::new(),
        }
    }

    /// Adds a comment to the parameter.
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = comment.into();
        self
    }

    /// Adds attributes to the parameter.
    pub fn with_attributes(mut self, attributes: impl Into<Vec<AdsAttribute>>) -> Self {
        self.attributes = attributes.into();
        if !self.attributes.is_empty() {
            self.flags |= AdsMethodParamFlags::ATTRIBUTES;
        }
        self
    }

    /// The byte size of the parameter.
    pub fn size(&self) -> u32 {
        self.size
    }
    /// The alignment requirement of the parameter.
    pub fn align_size(&self) -> u32 {
        self.align_size
    }
    /// The base data type identifier of the parameter.
    pub fn type_id(&self) -> AdsDataTypeId {
        self.type_id
    }
    /// The parameter flags (e.g., IN, OUT, BY_REF).
    pub fn flags(&self) -> AdsMethodParamFlags {
        self.flags
    }
    /// The globally unique identifier for this parameter's type, if applicable.
    pub fn guid(&self) -> &Guid {
        &self.guid
    }
    /// If this parameter is a generic pointer (e.g. `PVOID`) or a dynamic array,
    /// this returns the `0`-based index of the *other* parameter in the method signature
    /// that defines the byte length to be marshalled over ADS.
    ///
    /// Returns `None` if this parameter has a static length.
    pub fn dynamic_length_param_index(&self) -> Option<usize> {
        if self.length_is_param == 0 {
            None
        } else {
            // TwinCAT stores this as (index + 1) to reserve 0 for "none"
            Some((self.length_is_param - 1) as usize)
        }
    }
    /// The name of the parameter.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The name of the parameter's data type.
    pub fn type_name(&self) -> &str {
        &self.type_name
    }
    /// Optional comment attached to the parameter in the PLC code.
    pub fn comment(&self) -> &str {
        &self.comment
    }
    /// Optional attributes (pragmas) attached to the parameter.
    pub fn attributes(&self) -> &[AdsAttribute] {
        &self.attributes
    }

    /// Returns the wire size of this parameter entry in bytes.
    pub fn wire_size(&self) -> usize {
        let strings = self.name.len() + 1 + self.type_name.len() + 1 + self.comment.len() + 1;
        let attrs = if self.attributes.is_empty() {
            0
        } else {
            2 + self.attributes.iter().map(|a| a.wire_size()).sum::<usize>()
        };
        Self::MIN_LENGTH + strings + attrs
    }

    /// Serializes method parameter info into bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let entry_length = self.wire_size() as u32;

        let mut buf = Vec::with_capacity(self.wire_size());

        buf.extend_from_slice(&entry_length.to_le_bytes());
        buf.extend_from_slice(&self.size.to_le_bytes());
        buf.extend_from_slice(&self.align_size.to_le_bytes());
        buf.extend_from_slice(&u32::from(self.type_id).to_le_bytes());
        buf.extend_from_slice(&self.flags.to_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // reserved
        buf.extend_from_slice(self.guid.as_bytes());
        buf.extend_from_slice(&self.length_is_param.to_le_bytes());
        buf.extend_from_slice(&(self.name.len() as u16).to_le_bytes());
        buf.extend_from_slice(&(self.type_name.len() as u16).to_le_bytes());
        buf.extend_from_slice(&(self.comment.len() as u16).to_le_bytes());

        buf.extend_from_slice(self.name.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.type_name.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.comment.as_bytes());
        buf.push(0);

        if !self.attributes.is_empty() {
            buf.extend_from_slice(&(self.attributes.len() as u16).to_le_bytes());
            for attr in &self.attributes {
                buf.extend_from_slice(&attr.to_vec());
            }
        }

        buf
    }

    /// Parses an RPC Method Parameter from a byte slice.
    /// Returns the parsed struct and the total entry length consumed.
    pub fn try_from_slice(data: &[u8]) -> Result<(Self, usize), AdsTypeInfoError> {
        if data.len() < Self::MIN_LENGTH {
            return Err(AdsTypeInfoError::TooShort {
                expected: Self::MIN_LENGTH,
                got: data.len(),
            });
        }

        let entry_length = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
        if data.len() < entry_length {
            return Err(AdsTypeInfoError::EntryLengthMismatch {
                expected: entry_length,
                got: data.len(),
            });
        }

        let entry = &data[..entry_length];

        let size = u32::from_le_bytes(entry[4..8].try_into().unwrap());
        let align_size = u32::from_le_bytes(entry[8..12].try_into().unwrap());
        let type_id = AdsDataTypeId::from(u32::from_le_bytes(entry[12..16].try_into().unwrap()));
        let flags = AdsMethodParamFlags::new(u32::from_le_bytes(entry[16..20].try_into().unwrap()));

        let mut guid_bytes = [0u8; 16];
        guid_bytes.copy_from_slice(&entry[24..40]);
        let guid = Guid::new(guid_bytes);

        let length_is_param = u16::from_le_bytes(entry[40..42].try_into().unwrap());

        let name_len = u16::from_le_bytes(entry[42..44].try_into().unwrap()) as usize;
        let type_name_len = u16::from_le_bytes(entry[44..46].try_into().unwrap()) as usize;
        let comment_len = u16::from_le_bytes(entry[46..48].try_into().unwrap()) as usize;

        let mut pos = Self::MIN_LENGTH;

        let name_end = pos + name_len + 1;
        let type_end = name_end + type_name_len + 1;
        let comment_end = type_end + comment_len + 1;

        let (name, _, _) =
            encoding_rs::WINDOWS_1252.decode(&entry[pos..name_end.saturating_sub(1)]);
        let (type_name, _, _) =
            encoding_rs::WINDOWS_1252.decode(&entry[name_end..type_end.saturating_sub(1)]);
        let (comment, _, _) =
            encoding_rs::WINDOWS_1252.decode(&entry[type_end..comment_end.saturating_sub(1)]);

        pos = comment_end;

        let mut attributes = Vec::new();
        if flags.has_attributes() {
            if pos + 2 <= entry_length {
                let attr_count =
                    u16::from_le_bytes(entry[pos..pos + 2].try_into().unwrap()) as usize;
                pos += 2;

                attributes.reserve(attr_count);
                for _ in 0..attr_count {
                    let attr = AdsAttribute::try_from_slice(&entry[pos..])?;
                    pos += attr.wire_size();
                    attributes.push(attr);
                }
            }
        }

        Ok((
            Self {
                size,
                align_size,
                type_id,
                flags,
                guid,
                length_is_param,
                name: name.into_owned(),
                type_name: type_name.into_owned(),
                comment: comment.into_owned(),
                attributes,
            },
            entry_length,
        ))
    }
}

impl TryFrom<&[u8]> for AdsMethodParamInfo {
    type Error = AdsTypeInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let (info, _) = Self::try_from_slice(value)?;
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real capture bytes for nA (param[0] of Sum method in FB_Calc).
    // Confirmed: size=8, align=8, type_id=21 (ULINT), flags=IN, name="nA", type="ULINT"
    #[rustfmt::skip]
    fn na_bytes() -> Vec<u8> {
        vec![
            58, 0, 0, 0, // entry_length = 58
            8, 0, 0, 0, // size = 8
            8, 0, 0, 0, // align_size = 8
            21, 0, 0, 0, // type_id = UInt64 (ULINT)
            1, 0, 0, 0, // flags = IN
            0, 0, 0, 0, // reserved
            // guid: 95190718-0000-0000-0000-00000000000b
            149, 25, 7, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 11,
            0, 0, // length_is_param = 0
            2, 0, // name_len = 2
            5, 0, // type_name_len = 5
            0, 0, // comment_len = 0
            110, 65, 0, // "nA\0"
            85, 76, 73, 78, 84, 0, // "ULINT\0"
            0, // comment "\0"
        ]
    }

    // Real capture bytes for nOut (param[2] of Sum, has comment).
    // Confirmed: size=8, flags=OUT, name="nOut", type="LINT", comment=" The sum of the inputs"
    #[rustfmt::skip]
    fn nout_bytes() -> Vec<u8> {
        vec![
            81, 0, 0, 0, // entry_length = 81
            8, 0, 0, 0, // size = 8
            8, 0, 0, 0, // align_size = 8
            20, 0, 0, 0, // type_id = Int64 (LINT)
            2, 0, 0, 0, // flags = OUT
            0, 0, 0, 0, // reserved
            // guid: 95190718-0000-0000-0000-00000000000c
            149, 25, 7, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12,
            0, 0, // length_is_param = 0
            4, 0, // name_len = 4
            4, 0, // type_name_len = 4
            22, 0, // comment_len = 22
            110, 79, 117, 116, 0, // "nOut\0"
            76, 73, 78, 84, 0, // "LINT\0"
            // " The sum of the inputs\0"
            32, 84, 104, 101, 32, 115, 117, 109, 32, 111, 102, 32, 116, 104, 101, 32, 105, 110, 112,
            117, 116, 115, 0,
        ]
    }

    #[test]
    fn parses_na_from_real_capture() {
        let (param, consumed) = AdsMethodParamInfo::try_from_slice(&na_bytes()).unwrap();
        assert_eq!(consumed, 58);
        assert_eq!(param.name(), "nA");
        assert_eq!(param.type_name(), "ULINT");
        assert_eq!(param.size(), 8);
        assert_eq!(param.align_size(), 8);
        assert_eq!(param.type_id(), AdsDataTypeId::UInt64);
        assert!(param.flags().is_input());
        assert!(!param.flags().is_output());
        assert!(param.comment().is_empty());
        assert!(param.attributes().is_empty());
        assert!(param.dynamic_length_param_index().is_none());
    }

    #[test]
    fn parses_nout_from_real_capture() {
        let (param, consumed) = AdsMethodParamInfo::try_from_slice(&nout_bytes()).unwrap();
        assert_eq!(consumed, 81);
        assert_eq!(param.name(), "nOut");
        assert_eq!(param.type_name(), "LINT");
        assert_eq!(param.size(), 8);
        assert_eq!(param.type_id(), AdsDataTypeId::Int64);
        assert!(param.flags().is_output());
        assert!(!param.flags().is_input());
        assert_eq!(param.comment(), " The sum of the inputs");
        assert!(param.attributes().is_empty());
    }

    #[test]
    fn parses_guid_correctly() {
        let (param, _) = AdsMethodParamInfo::try_from_slice(&na_bytes()).unwrap();
        let expected: Guid = "95190718-0000-0000-0000-00000000000b".parse().unwrap();
        assert_eq!(*param.guid(), expected);
    }

    #[test]
    fn new_has_empty_comment_and_attrs() {
        let guid = Guid::default();
        let param = AdsMethodParamInfo::new(
            4,
            4,
            AdsDataTypeId::UInt32,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nVal",
            "UDINT",
        );
        assert!(param.comment().is_empty());
        assert!(param.attributes().is_empty());
        assert_eq!(param.name(), "nVal");
        assert_eq!(param.type_name(), "UDINT");
    }

    #[test]
    fn with_comment_sets_comment() {
        let guid = Guid::default();
        let param = AdsMethodParamInfo::new(
            4,
            4,
            AdsDataTypeId::UInt32,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nVal",
            "UDINT",
        )
        .with_comment("the value");
        assert_eq!(param.comment(), "the value");
    }

    #[test]
    fn with_attributes_sets_attributes() {
        let guid = Guid::default();
        let param = AdsMethodParamInfo::new(
            4,
            4,
            AdsDataTypeId::UInt32,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nVal",
            "UDINT",
        )
        .with_attributes([AdsAttribute::new("unit", "mm")]);
        assert_eq!(param.attributes().len(), 1);
        assert_eq!(param.attributes()[0].name(), "unit");
        assert_eq!(param.attributes()[0].value(), "mm");
    }

    #[test]
    fn dynamic_length_none_when_zero() {
        let guid = Guid::default();
        let param = AdsMethodParamInfo::new(
            8,
            8,
            AdsDataTypeId::UInt64,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nA",
            "ULINT",
        );
        assert!(param.dynamic_length_param_index().is_none());
    }

    #[test]
    fn dynamic_length_some_when_nonzero() {
        let guid = Guid::default();
        let param = AdsMethodParamInfo::new(
            8,
            8,
            AdsDataTypeId::UInt64,
            AdsMethodParamFlags::IN,
            guid,
            2,
            "pData",
            "PVOID",
        );
        // stored as index+1, so 2 means param at index 1
        assert_eq!(param.dynamic_length_param_index(), Some(1));
    }

    #[test]
    fn dynamic_length_first_param_is_index_zero() {
        let guid = Guid::default();
        let param = AdsMethodParamInfo::new(
            8,
            8,
            AdsDataTypeId::UInt64,
            AdsMethodParamFlags::IN,
            guid,
            1,
            "pData",
            "PVOID",
        );
        assert_eq!(param.dynamic_length_param_index(), Some(0));
    }

    #[test]
    fn wire_size_matches_to_vec_length() {
        let guid = Guid::default();
        let param = AdsMethodParamInfo::new(
            4,
            4,
            AdsDataTypeId::UInt32,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nVal",
            "UDINT",
        )
        .with_comment("a value");
        assert_eq!(param.wire_size(), param.to_bytes().len());
    }

    #[test]
    fn wire_size_with_attributes_matches_to_vec_length() {
        let guid = Guid::default();
        let param = AdsMethodParamInfo::new(
            4,
            4,
            AdsDataTypeId::UInt32,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nVal",
            "UDINT",
        )
        .with_attributes([
            AdsAttribute::new("unit", "mm"),
            AdsAttribute::new("min", "0"),
        ]);
        assert_eq!(param.wire_size(), param.to_bytes().len());
    }

    #[test]
    fn roundtrip_no_comment_no_attrs() {
        let guid: Guid = "95190718-0000-0000-0000-00000000000b".parse().unwrap();
        let original = AdsMethodParamInfo::new(
            8,
            8,
            AdsDataTypeId::UInt64,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nA",
            "ULINT",
        );
        let bytes = original.to_bytes();
        let (parsed, consumed) = AdsMethodParamInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed, original);
    }

    #[test]
    fn roundtrip_with_comment() {
        let guid: Guid = "95190718-0000-0000-0000-00000000000c".parse().unwrap();
        let original = AdsMethodParamInfo::new(
            8,
            8,
            AdsDataTypeId::Int64,
            AdsMethodParamFlags::OUT,
            guid,
            0,
            "nOut",
            "LINT",
        )
        .with_comment(" The sum of the inputs");
        let bytes = original.to_bytes();
        let (parsed, consumed) = AdsMethodParamInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed, original);
    }

    #[test]
    fn roundtrip_with_attributes_sets_attributes_flag() {
        let guid = Guid::default();
        let original = AdsMethodParamInfo::new(
            4,
            4,
            AdsDataTypeId::UInt32,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nCount",
            "UDINT",
        )
        .with_attributes([AdsAttribute::new("unit", "items")]);
        let bytes = original.to_bytes();
        let (parsed, consumed) = AdsMethodParamInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed.attributes().len(), 1);
        assert!(parsed.flags().has_attributes());
    }

    #[test]
    fn roundtrip_real_na_bytes() {
        let (original, _) = AdsMethodParamInfo::try_from_slice(&na_bytes()).unwrap();
        let bytes = original.to_bytes();
        let (parsed, _) = AdsMethodParamInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn roundtrip_real_nout_bytes() {
        let (original, _) = AdsMethodParamInfo::try_from_slice(&nout_bytes()).unwrap();
        let bytes = original.to_bytes();
        let (parsed, _) = AdsMethodParamInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn too_short_header_returns_err() {
        assert!(matches!(
            AdsMethodParamInfo::try_from_slice(&[0u8; 10]).unwrap_err(),
            AdsTypeInfoError::TooShort {
                expected: 48,
                got: 10
            }
        ));
    }

    #[test]
    fn entry_length_mismatch_returns_err() {
        let mut bytes = na_bytes();
        // claim entry is 200 bytes but only 58 are provided
        bytes[0] = 200;
        bytes[1] = 0;
        bytes[2] = 0;
        bytes[3] = 0;
        assert!(matches!(
            AdsMethodParamInfo::try_from_slice(&bytes).unwrap_err(),
            AdsTypeInfoError::EntryLengthMismatch {
                expected: 200,
                got: 58
            }
        ));
    }

    #[test]
    fn empty_slice_returns_err() {
        assert!(AdsMethodParamInfo::try_from_slice(&[]).is_err());
    }
}
