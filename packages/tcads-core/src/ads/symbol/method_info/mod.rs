pub mod flags;
pub mod param_info;
pub mod return_type;

use super::error::AdsTypeInfoError;
use super::*;
pub use flags::{AdsMethodFlags, AdsMethodParamFlags};
pub use param_info::AdsMethodParamInfo;
pub use return_type::AdsMethodReturnTypeInfo;

// Represents an RPC Method attached to a TwinCAT Struct/Function Block.
///
/// # Wire Format
///
/// | Offset | Size | Field               | Description                            |
/// |--------|------|---------------------|----------------------------------------|
/// | 0      | 4    | `entry_length`      | Total size including dynamic tail      |
/// | 4      | 4    | `version`           | Method metadata version                |
/// | 8      | 4    | `vtable_index`      | Virtual table index                    |
/// | 12     | 4    | `return_size`       | Return value byte size                 |
/// | 16     | 4    | `return_align_size` | Return value alignment                 |
/// | 20     | 4    | `reserved`          | Ignored                                |
/// | 24     | 16   | `return_type_guid`  | GUID of the return type                |
/// | 40     | 4    | `return_data_type`  | Base type ID of the return             |
/// | 44     | 4    | `flags`             | Method flags (e.g. Callable)           |
/// | 48     | 2    | `name_len`          | Byte length of method name excl. null  |
/// | 50     | 2    | `return_type_len`   | Byte length of return name excl. null  |
/// | 52     | 2    | `comment_len`       | Byte length of comment excl. null      |
/// | 54     | 2    | `param_count`       | Number of parameters                   |
/// | 56     | dyn  | `Strings`           | Windows-1252, null-terminated          |
/// | dyn    | dyn  | `Parameters`        | Array of `AdsMethodParamInfo`          |
/// | dyn    | dyn  | `Attributes`        | Only if `ATTRIBUTES` flag is set       |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsMethodInfo {
    version: u32,
    vtable_index: u32,
    flags: AdsMethodFlags,
    name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    comment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    return_type: Option<AdsMethodReturnTypeInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    parameters: Vec<AdsMethodParamInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<AdsAttribute>,
}

impl AdsMethodInfo {
    /// Fixed size of the method header before dynamic attributes.
    pub const MIN_LENGTH: usize = 56;

    /// The method metadata version.
    pub fn version(&self) -> u32 {
        self.version
    }
    /// The virtual table index of the method.
    pub fn vtable_index(&self) -> u32 {
        self.vtable_index
    }
    /// The method flags (e.g. Callable).
    pub fn flags(&self) -> AdsMethodFlags {
        self.flags
    }
    /// The name of the method.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Optional comment attached to the method in the PLC code.
    pub fn comment(&self) -> &str {
        &self.comment
    }
    /// Method return type information, if applicable.
    pub fn return_type(&self) -> Option<&AdsMethodReturnTypeInfo> {
        self.return_type.as_ref()
    }
    /// Method parameter information.
    pub fn parameters(&self) -> &[AdsMethodParamInfo] {
        &self.parameters
    }
    /// Optional attributes (pragmas) attached to the method.
    pub fn attributes(&self) -> &[AdsAttribute] {
        &self.attributes
    }

    /// Parses an RPC Method from a byte slice.
    /// Returns the parsed struct and the total entry length consumed.
    pub fn try_from_slice(data: &[u8]) -> Result<(Self, usize), AdsTypeInfoError> {
        if data.len() < Self::MIN_LENGTH {
            return Err(AdsTypeInfoError::TooShort {
                expected: Self::MIN_LENGTH,
                got: data.len(),
            });
        }

        let entry_length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < entry_length {
            return Err(AdsTypeInfoError::EntryLengthMismatch {
                expected: entry_length,
                got: data.len(),
            });
        }

        let entry = &data[..entry_length];

        let version = u32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]);
        let vtable_index = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);

        let return_size = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);
        let return_align_size = u32::from_le_bytes([entry[16], entry[17], entry[18], entry[19]]);
        let return_type_guid = Guid::try_from_slice(&entry[24..40])?;
        let return_data_type = AdsDataTypeId::try_from_slice(&entry[40..44])?;

        let flags = AdsMethodFlags::try_from(&entry[44..48])?;

        let name_len = u16::from_le_bytes([entry[48], entry[49]]) as usize;
        let return_type_len = u16::from_le_bytes([entry[50], entry[51]]) as usize;
        let comment_len = u16::from_le_bytes([entry[52], entry[53]]) as usize;
        let param_count = u16::from_le_bytes([entry[54], entry[55]]) as usize;

        let mut pos = Self::MIN_LENGTH;

        let name_end = pos + name_len + 1;
        let ret_end = name_end + return_type_len + 1;
        let comment_end = ret_end + comment_len + 1;

        let (name, _, _) =
            encoding_rs::WINDOWS_1252.decode(&entry[pos..name_end.saturating_sub(1)]);
        let (return_type_str, _, _) =
            encoding_rs::WINDOWS_1252.decode(&entry[name_end..ret_end.saturating_sub(1)]);
        let (comment, _, _) =
            encoding_rs::WINDOWS_1252.decode(&entry[ret_end..comment_end.saturating_sub(1)]);

        pos = comment_end;

        let return_type = if return_size == 0 && return_type_str.is_empty() {
            None
        } else {
            Some(AdsMethodReturnTypeInfo::new(
                return_size,
                return_align_size,
                return_data_type,
                return_type_guid,
                return_type_str.into_owned(),
            ))
        };

        let mut parameters = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            let (param, bytes_consumed) = AdsMethodParamInfo::try_from_slice(&entry[pos..])?;
            pos += bytes_consumed;
            parameters.push(param);
        }

        let mut attributes = Vec::new();
        if flags.has_attributes() {
            if pos + 2 <= entry_length {
                let attr_count = u16::from_le_bytes([entry[pos], entry[pos + 1]]) as usize;
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
                version,
                vtable_index,
                flags,
                name: name.into_owned(),
                comment: comment.into_owned(),
                return_type,
                parameters,
                attributes,
            },
            entry_length,
        ))
    }
}

impl TryFrom<&[u8]> for AdsMethodInfo {
    type Error = AdsTypeInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let (info, _) = Self::try_from_slice(value)?;
        Ok(info)
    }
}
