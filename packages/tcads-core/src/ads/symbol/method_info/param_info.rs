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
