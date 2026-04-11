pub mod flags;
pub mod param_info;
pub mod return_type;

use super::error::AdsTypeInfoError;
use super::*;
pub use flags::{AdsMethodFlags, AdsMethodParamFlags};
pub use param_info::AdsMethodParamInfo;
pub use return_type::AdsMethodReturnTypeInfo;

/// Represents an RPC Method attached to a TwinCAT Struct/Function Block.
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

    /// Creates a new void method (no return type, no parameters, no comment).
    pub fn new(vtable_index: u32, flags: AdsMethodFlags, name: impl Into<String>) -> Self {
        Self {
            version: 1,
            vtable_index,
            flags,
            name: name.into(),
            comment: String::new(),
            return_type: None,
            parameters: Vec::new(),
            attributes: Vec::new(),
        }
    }

    /// Returns this method with the given comment.
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = comment.into();
        self
    }

    /// Returns this method with the given return type.
    pub fn with_return_type(mut self, return_type: AdsMethodReturnTypeInfo) -> Self {
        self.return_type = Some(return_type);
        self
    }

    /// Returns this method with the given parameters.
    pub fn with_parameters(mut self, parameters: impl Into<Vec<AdsMethodParamInfo>>) -> Self {
        self.parameters = parameters.into();
        self
    }

    /// Returns this method with the given attributes.
    pub fn with_attributes(mut self, attributes: impl Into<Vec<AdsAttribute>>) -> Self {
        self.attributes = attributes.into();
        if !self.attributes.is_empty() {
            self.flags |= AdsMethodFlags::ATTRIBUTES;
        }
        self
    }

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

    /// Returns the wire size of this entry in bytes.
    pub fn wire_size(&self) -> usize {
        let return_type_name_len = self.return_type.as_ref().map_or(0, |r| r.name().len());

        let strings = self.name.len() + 1 + return_type_name_len + 1 + self.comment.len() + 1;
        let params: usize = self.parameters.iter().map(|p| p.wire_size()).sum();
        let attrs = if self.attributes.is_empty() {
            0
        } else {
            2 + self.attributes.iter().map(|a| a.wire_size()).sum::<usize>()
        };
        Self::MIN_LENGTH + strings + params + attrs
    }

    /// Serializes method info into bytes.
    pub fn to_vec(&self) -> Vec<u8> {
        let entry_length = self.wire_size() as u32;

        let (return_size, return_align_size, return_guid, return_data_type, return_type_name) =
            match &self.return_type {
                Some(r) => (
                    r.size(),
                    r.align_size(),
                    r.guid().clone(),
                    r.type_id(),
                    r.name().to_owned(),
                ),
                None => (0, 0, Guid::default(), AdsTypeId::Void, String::new()),
            };

        let mut buf = Vec::with_capacity(self.wire_size());

        buf.extend_from_slice(&entry_length.to_le_bytes());
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&self.vtable_index.to_le_bytes());
        buf.extend_from_slice(&return_size.to_le_bytes());
        buf.extend_from_slice(&return_align_size.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // reserved
        buf.extend_from_slice(return_guid.as_bytes());
        buf.extend_from_slice(&u32::from(return_data_type).to_le_bytes());
        buf.extend_from_slice(&self.flags.to_bytes());
        buf.extend_from_slice(&(self.name.len() as u16).to_le_bytes());
        buf.extend_from_slice(&(return_type_name.len() as u16).to_le_bytes());
        buf.extend_from_slice(&(self.comment.len() as u16).to_le_bytes());
        buf.extend_from_slice(&(self.parameters.len() as u16).to_le_bytes());

        buf.extend_from_slice(self.name.as_bytes());
        buf.push(0);
        buf.extend_from_slice(return_type_name.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.comment.as_bytes());
        buf.push(0);

        for param in &self.parameters {
            buf.extend_from_slice(&param.to_vec());
        }

        if !self.attributes.is_empty() {
            buf.extend_from_slice(&(self.attributes.len() as u16).to_le_bytes());
            for attr in &self.attributes {
                buf.extend_from_slice(&attr.to_vec());
            }
        }

        buf
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
        let return_data_type = AdsTypeId::try_from_slice(&entry[40..44])?;

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
        if flags.has_attributes() && pos + 2 <= entry_length {
            let attr_count = u16::from_le_bytes([entry[pos], entry[pos + 1]]) as usize;
            pos += 2;

            attributes.reserve(attr_count);
            for _ in 0..attr_count {
                let attr = AdsAttribute::try_from_slice(&entry[pos..])?;
                pos += attr.wire_size();
                attributes.push(attr);
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

impl From<&AdsMethodInfo> for Vec<u8> {
    fn from(value: &AdsMethodInfo) -> Self {
        value.to_vec()
    }
}

impl From<AdsMethodInfo> for Vec<u8> {
    fn from(value: AdsMethodInfo) -> Self {
        value.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real capture: the `Sum` method from FB_Calc (starts at byte 142, length 276).
    #[rustfmt::skip]
    fn sum_method_bytes() -> Vec<u8> {
        vec![
            20, 1, 0, 0, // entry_length = 276
            1, 0, 0, 0, // version = 1
            7, 0, 0, 0, // vtable_index = 7
            0, 0, 0, 0, // return_size = 0
            0, 0, 0, 0, // return_align_size = 0
            0, 0, 0, 0, // reserved
            0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0, // return_type_guid = all zeros
            0, 0, 0, 0, // return_data_type = Void
            1, 0, 0, 0, // flags = PLC_CALLING_CONVENTION
            3, 0, // name_len = 3
            0, 0, // return_type_len = 0
            17, 0,  // comment_len = 17
            3, 0, // param_count = 3
            83, 117, 109, 0, // "Sum\0"
            0, // return_type "\0"
            // " Sums two values.\0"
            32, 83, 117, 109, 115, 32, 116, 119, 111, 32, 118, 97, 108, 117, 101, 115, 46, 0,
            // param nA (58 bytes)
            58, 0, 0, 0, 8, 0, 0, 0, 8, 0, 0, 0, 21, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
            149, 25, 7, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 11,
            0, 0, 2, 0, 5, 0, 0, 0, 110, 65, 0, 85, 76, 73, 78, 84, 0, 0,
            // param nB (58 bytes)
            58, 0, 0, 0, 8, 0, 0, 0, 8, 0, 0, 0, 21, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
            149, 25, 7, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 11,
            0, 0, 2, 0, 5, 0, 0, 0, 110, 66, 0, 85, 76, 73, 78, 84, 0, 0,
            // param nOut (81 bytes)
            81, 0, 0, 0, 8, 0, 0, 0, 8, 0, 0, 0, 20, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
            149, 25, 7, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12,
            0, 0, 4, 0, 4, 0, 22, 0, 110, 79, 117, 116, 0, 76, 73, 78, 84, 0,
            32, 84, 104, 101, 32, 115, 117, 109, 32, 111, 102, 32, 116, 104, 101,
            32, 105, 110, 112, 117, 116, 115, 0,
        ]
    }

    #[test]
    fn parses_sum_method_from_real_capture() {
        let (method, consumed) = AdsMethodInfo::try_from_slice(&sum_method_bytes()).unwrap();
        assert_eq!(consumed, 276);
        assert_eq!(method.name(), "Sum");
        assert_eq!(method.comment(), " Sums two values.");
        assert_eq!(method.vtable_index(), 7);
        assert_eq!(method.version(), 1);
        assert!(method.flags().is_plc_calling_convention());
        assert!(method.return_type().is_none());
        assert_eq!(method.parameters().len(), 3);
        assert!(method.attributes().is_empty());
    }

    #[test]
    fn parses_parameters_correctly() {
        let (method, _) = AdsMethodInfo::try_from_slice(&sum_method_bytes()).unwrap();
        let params = method.parameters();

        assert_eq!(params[0].name(), "nA");
        assert_eq!(params[0].type_name(), "ULINT");
        assert!(params[0].flags().is_input());

        assert_eq!(params[1].name(), "nB");
        assert_eq!(params[1].type_name(), "ULINT");
        assert!(params[1].flags().is_input());

        assert_eq!(params[2].name(), "nOut");
        assert_eq!(params[2].type_name(), "LINT");
        assert!(params[2].flags().is_output());
        assert_eq!(params[2].comment(), " The sum of the inputs");
    }

    #[test]
    fn new_creates_void_method() {
        let method = AdsMethodInfo::new(3, AdsMethodFlags::PLC_CALLING_CONVENTION, "Run");
        assert_eq!(method.name(), "Run");
        assert_eq!(method.vtable_index(), 3);
        assert_eq!(method.version(), 1);
        assert!(method.return_type().is_none());
        assert!(method.parameters().is_empty());
        assert!(method.comment().is_empty());
        assert!(method.attributes().is_empty());
    }

    #[test]
    fn with_comment_sets_comment() {
        let method = AdsMethodInfo::new(0, AdsMethodFlags::PLC_CALLING_CONVENTION, "Run")
            .with_comment("Starts the machine");
        assert_eq!(method.comment(), "Starts the machine");
    }

    #[test]
    fn with_return_type_sets_return_type() {
        let guid: Guid = "95190718-0000-0000-0000-000000000008".parse().unwrap();
        let ret = AdsMethodReturnTypeInfo::new(4, 4, AdsTypeId::UInt32, guid, "UDINT");
        let method = AdsMethodInfo::new(0, AdsMethodFlags::PLC_CALLING_CONVENTION, "GetCount")
            .with_return_type(ret);
        let rt = method.return_type().unwrap();
        assert_eq!(rt.name(), "UDINT");
        assert_eq!(rt.size(), 4);
    }

    #[test]
    fn with_parameters_sets_parameters() {
        let guid = Guid::default();
        let param = AdsMethodParamInfo::new(
            4,
            4,
            AdsTypeId::UInt32,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nVal",
            "UDINT",
        );
        let method = AdsMethodInfo::new(0, AdsMethodFlags::PLC_CALLING_CONVENTION, "SetValue")
            .with_parameters([param]);
        assert_eq!(method.parameters().len(), 1);
        assert_eq!(method.parameters()[0].name(), "nVal");
    }

    #[test]
    fn with_attributes_sets_attributes() {
        let method = AdsMethodInfo::new(0, AdsMethodFlags::PLC_CALLING_CONVENTION, "Init")
            .with_attributes([AdsAttribute::new("no_explicit_call", "true")]);
        assert_eq!(method.attributes().len(), 1);
        assert_eq!(method.attributes()[0].name(), "no_explicit_call");
    }

    #[test]
    fn wire_size_matches_to_vec_length() {
        let method = AdsMethodInfo::new(7, AdsMethodFlags::PLC_CALLING_CONVENTION, "Sum")
            .with_comment(" Sums two values.");
        assert_eq!(method.wire_size(), method.to_vec().len());
    }

    #[test]
    fn wire_size_with_params_matches_to_vec_length() {
        let guid = Guid::default();
        let param = AdsMethodParamInfo::new(
            4,
            4,
            AdsTypeId::UInt32,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nVal",
            "UDINT",
        );
        let method = AdsMethodInfo::new(0, AdsMethodFlags::PLC_CALLING_CONVENTION, "Set")
            .with_parameters([param]);
        assert_eq!(method.wire_size(), method.to_vec().len());
    }

    #[test]
    fn roundtrip_void_no_params() {
        let original = AdsMethodInfo::new(3, AdsMethodFlags::PLC_CALLING_CONVENTION, "Reset")
            .with_comment("Resets state");
        let bytes = original.to_vec();
        let (parsed, consumed) = AdsMethodInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed, original);
    }

    #[test]
    fn roundtrip_with_parameters() {
        let guid: Guid = "95190718-0000-0000-0000-00000000000b".parse().unwrap();
        let param = AdsMethodParamInfo::new(
            8,
            8,
            AdsTypeId::UInt64,
            AdsMethodParamFlags::IN,
            guid,
            0,
            "nA",
            "ULINT",
        );
        let original = AdsMethodInfo::new(7, AdsMethodFlags::PLC_CALLING_CONVENTION, "Sum")
            .with_comment(" Sums two values.")
            .with_parameters([param]);
        let bytes = original.to_vec();
        let (parsed, consumed) = AdsMethodInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed, original);
    }

    #[test]
    fn roundtrip_with_return_type() {
        let guid: Guid = "95190718-0000-0000-0000-000000000008".parse().unwrap();
        let ret = AdsMethodReturnTypeInfo::new(4, 4, AdsTypeId::UInt32, guid, "UDINT");
        let original = AdsMethodInfo::new(1, AdsMethodFlags::PLC_CALLING_CONVENTION, "GetCount")
            .with_return_type(ret);
        let bytes = original.to_vec();
        let (parsed, consumed) = AdsMethodInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        let rt = parsed.return_type().unwrap();
        assert_eq!(rt.name(), "UDINT");
        assert_eq!(rt.size(), 4);
    }

    #[test]
    fn roundtrip_with_attributes_sets_flag() {
        let original = AdsMethodInfo::new(0, AdsMethodFlags::PLC_CALLING_CONVENTION, "Init")
            .with_attributes([AdsAttribute::new("no_explicit_call", "true")]);
        let bytes = original.to_vec();
        let (parsed, _) = AdsMethodInfo::try_from_slice(&bytes).unwrap();
        assert!(parsed.flags().has_attributes());
        assert_eq!(parsed.attributes().len(), 1);
        assert_eq!(parsed.attributes()[0].value(), "true");
    }

    #[test]
    fn roundtrip_real_sum_method() {
        let (original, _) = AdsMethodInfo::try_from_slice(&sum_method_bytes()).unwrap();
        let bytes = original.to_vec();
        let (parsed, consumed) = AdsMethodInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed.name(), original.name());
        assert_eq!(parsed.comment(), original.comment());
        assert_eq!(parsed.parameters().len(), original.parameters().len());
    }

    #[test]
    fn too_short_returns_err() {
        assert!(matches!(
            AdsMethodInfo::try_from_slice(&[0u8; 10]).unwrap_err(),
            AdsTypeInfoError::TooShort {
                expected: 56,
                got: 10
            }
        ));
    }

    #[test]
    fn entry_length_mismatch_returns_err() {
        let mut bytes = sum_method_bytes();
        bytes[0] = 255;
        bytes[1] = 255;
        bytes[2] = 0;
        bytes[3] = 0;
        assert!(matches!(
            AdsMethodInfo::try_from_slice(&bytes).unwrap_err(),
            AdsTypeInfoError::EntryLengthMismatch { .. }
        ));
    }

    #[test]
    fn empty_slice_returns_err() {
        assert!(AdsMethodInfo::try_from_slice(&[]).is_err());
    }
}
