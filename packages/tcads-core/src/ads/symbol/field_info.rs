use super::error::AdsTypeInfoError;
use super::{AdsArrayInfo, AdsAttribute, AdsTypeFlags, AdsTypeId, Guid};

/// Type information about a specific member or variable instance within a parent
/// `Union`, `Struct`, or `Function Block`.
///
/// # Wire Format
///
/// | Offset | Size | Field             | Description                              |
/// |--------|------|-------------------|------------------------------------------|
/// | 0      | 4    | `entry_length`    | Total size including dynamic tail        |
/// | 16     | 4    | `size`            | Byte size of this specific field         |
/// | 20     | 4    | `offset`          | Relative byte offset in the parent type  |
/// | 24     | 4    | `type_id`         | Base type ID ([`AdsTypeId`])             |
/// | 28     | 4    | `flags`           | Field flags (e.g. Static, Property)      |
/// | 32     | 2    | `name_len`        | Length of name excl. null                |
/// | 34     | 2    | `type_len`        | Length of type_name excl. null           |
/// | 36     | 2    | `comment_len`     | Length of comment excl. null             |
/// | 38     | 2    | `array_dim_count` | Usually 0 for fields                     |
/// | 40     | 2    | `sub_item_count`  | Usually 0 for fields                     |
/// | 42     | dyn  | `Strings`         | Windows-1252, null-terminated            |
/// | dyn    | dyn  | `type_guid`       | 16-byte GUID if flag is set              |
/// | dyn    | dyn  | `Attributes`      | Prefixed by `u16` count if flag is set   |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsFieldInfo {
    version: u32,
    hash_value: u32,
    type_hash_value: u32,
    size: u32,
    offset: u32,
    type_id: AdsTypeId,
    #[serde(skip_serializing)]
    flags: AdsTypeFlags,
    name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    type_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    comment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    guid: Option<Guid>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<AdsAttribute>,
}

impl AdsFieldInfo {
    /// Byte size of the fixed header.
    pub const MIN_LENGTH: usize = 42;

    /// Structure version.
    pub fn version(&self) -> u32 {
        self.version
    }
    /// Hash of this type for change detection, or getter code offset.
    pub fn hash_value(&self) -> u32 {
        self.hash_value
    }
    /// Hash of the base type, or setter code offset.
    pub fn type_hash_value(&self) -> u32 {
        self.type_hash_value
    }
    /// Byte size of the type. In bits if [`AdsTypeFlags::BIT_VALUES`] is set.
    pub fn size(&self) -> u32 {
        self.size
    }
    /// Byte offset of this field within its parent struct/function block/array.
    /// Zero for root type entries.
    pub fn offset(&self) -> u32 {
        self.offset
    }
    /// Primitive type identifier of the base or element type.
    pub fn data_type(&self) -> AdsTypeId {
        self.type_id
    }
    /// Type flags.
    pub fn flags(&self) -> AdsTypeFlags {
        self.flags
    }
    /// The type or field name (e.g. `"UDINT"`, `"CycleTime"`, `"PLC.PlcTaskSystemInfo"`).
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The base or element type name (e.g. `"UDINT"` for an alias or reference to UDINT).
    /// Empty for primitive root types.
    pub fn type_name(&self) -> &str {
        &self.type_name
    }
    /// Optional comment from the PLC source.
    pub fn comment(&self) -> &str {
        &self.comment
    }
    /// 16-byte type GUID. Present when [`AdsTypeFlags::TYPE_GUID`] is set.
    pub fn guid(&self) -> Option<&Guid> {
        self.guid.as_ref()
    }
    /// Pragma key-value attributes. Non-empty when [`AdsTypeFlags::ATTRIBUTES`] is set.
    pub fn attributes(&self) -> &[AdsAttribute] {
        &self.attributes
    }

    /// Computes and returns the wire size in bytes.
    pub fn wire_size(&self) -> usize {
        let mut size = Self::MIN_LENGTH;

        let (name_bytes, _, _) = encoding_rs::WINDOWS_1252.encode(self.name());
        let (type_name_bytes, _, _) = encoding_rs::WINDOWS_1252.encode(self.type_name());
        let (comment_bytes, _, _) = encoding_rs::WINDOWS_1252.encode(self.comment());

        size += name_bytes.len() + 1;
        size += type_name_bytes.len() + 1;
        size += comment_bytes.len() + 1;

        if self.flags.has_type_guid() {
            size += Guid::LENGTH;
        }

        if self.flags.has_copy_mask() {
            size += self.size as usize;
        }

        if self.flags().has_attributes() {
            size += 2;
            size += self
                .attributes
                .iter()
                .map(|attr| attr.wire_size())
                .sum::<usize>();
        }

        size
    }

    /// Serializes this field info to a byte vector.
    pub fn to_vec(&self) -> Vec<u8> {
        let (name_bytes, _, _) = encoding_rs::WINDOWS_1252.encode(self.name());
        let (type_name_bytes, _, _) = encoding_rs::WINDOWS_1252.encode(self.type_name());
        let (comment_bytes, _, _) = encoding_rs::WINDOWS_1252.encode(self.comment());

        let mut entry_len = Self::MIN_LENGTH
            + name_bytes.len()
            + 1
            + type_name_bytes.len()
            + 1
            + comment_bytes.len()
            + 1;

        if self.flags.has_type_guid() {
            entry_len += Guid::LENGTH;
        }

        if self.flags.has_copy_mask() {
            entry_len += self.size as usize;
        }

        let mut attr_bytes = Vec::new();
        if self.flags.has_attributes() {
            attr_bytes.extend_from_slice(&(self.attributes.len() as u16).to_le_bytes());
            attr_bytes.extend(self.attributes.iter().flat_map(|a| a.to_vec()));
            entry_len += attr_bytes.len();
        };

        let mut buf = Vec::with_capacity(entry_len);

        // Header
        buf.extend_from_slice(&(entry_len as u32).to_le_bytes());
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&self.hash_value.to_le_bytes());
        buf.extend_from_slice(&self.type_hash_value.to_le_bytes());
        buf.extend_from_slice(&self.size.to_le_bytes());
        buf.extend_from_slice(&self.offset.to_le_bytes());
        buf.extend_from_slice(&self.type_id.to_bytes());
        buf.extend_from_slice(&self.flags.to_bytes());
        // String lengths (excl. null)
        buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(&(type_name_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(&(comment_bytes.len() as u16).to_le_bytes());
        // Counts (Array/Sub-items are 0 for FieldInfo)
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes());

        // Dynamic Strings
        buf.extend_from_slice(&name_bytes);
        buf.push(0);
        buf.extend_from_slice(&type_name_bytes);
        buf.push(0);
        buf.extend_from_slice(&comment_bytes);
        buf.push(0);

        if self.flags.has_type_guid() {
            buf.extend_from_slice(
                self.guid
                    .as_ref()
                    .map_or(Guid::default().as_bytes(), |g| g.as_bytes()),
            );
        }

        if self.flags.has_copy_mask() {
            buf.resize(buf.len() + self.size as usize, 0);
        }

        if self.flags.has_attributes() {
            buf.extend_from_slice(&attr_bytes);
        }

        buf
    }

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
        let hash_value = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);
        let type_hash_value = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);
        let size = u32::from_le_bytes([entry[16], entry[17], entry[18], entry[19]]);
        let offset = u32::from_le_bytes([entry[20], entry[21], entry[22], entry[23]]);
        let type_id = AdsTypeId::from([entry[24], entry[25], entry[26], entry[27]]);
        let flags = AdsTypeFlags::from([entry[28], entry[29], entry[30], entry[31]]);
        let name_length = u16::from_le_bytes([entry[32], entry[33]]) as usize;
        let type_name_length = u16::from_le_bytes([entry[34], entry[35]]) as usize;
        let comment_length = u16::from_le_bytes([entry[36], entry[37]]) as usize;
        let array_dim_count = u16::from_le_bytes([entry[38], entry[39]]) as usize;
        let sub_item_count = u16::from_le_bytes([entry[40], entry[41]]) as usize;

        let mut pos = Self::MIN_LENGTH;

        // Null-terminated strings
        let name_end = pos + name_length + 1;
        let type_end = name_end + type_name_length + 1;
        let comment_end = type_end + comment_length + 1;

        if entry_length < comment_end {
            return Err(AdsTypeInfoError::EntryLengthMismatch {
                expected: comment_end,
                got: entry_length,
            });
        }

        let (name, _, _) =
            encoding_rs::WINDOWS_1252.decode(&entry[pos..name_end.saturating_sub(1)]);
        let (type_name, _, _) =
            encoding_rs::WINDOWS_1252.decode(&entry[name_end..type_end.saturating_sub(1)]);
        let (comment, _, _) =
            encoding_rs::WINDOWS_1252.decode(&entry[type_end..comment_end.saturating_sub(1)]);

        pos = comment_end;

        // Skip array_infos
        // The array_infos count should be 0, but we're being cautious.
        pos += AdsArrayInfo::LENGTH * array_dim_count;

        // Skip sub_items this way because their length is dynamic.
        // For a field item the count should be 0, but we're being cautious.
        for _ in 0..sub_item_count {
            if pos + 4 > entry_length {
                break;
            }

            let sub_item_length =
                u32::from_le_bytes([entry[pos], entry[pos + 1], entry[pos + 2], entry[pos + 3]])
                    as usize;
            pos += sub_item_length;
        }

        let mut guid = None;
        if flags.has_type_guid() && pos + Guid::LENGTH <= entry_length {
            guid = Some(Guid::try_from_slice(&entry[pos..pos + Guid::LENGTH])?);
            pos += Guid::LENGTH;
        }

        if flags.has_copy_mask() {
            // Legacy section, skip it
            pos = (pos + size as usize).min(entry_length);
        }

        // Skip method_infos
        // Shouldn't be any, but we're being cautious again.
        if flags.has_method_infos() && pos + 2 <= entry_length {
            let method_count = u16::from_le_bytes([entry[pos], entry[pos + 1]]);
            pos += 2;
            for _ in 0..method_count {
                if pos + 4 > entry_length {
                    break;
                }
                let method_length = u32::from_le_bytes([
                    entry[pos],
                    entry[pos + 1],
                    entry[pos + 2],
                    entry[pos + 3],
                ]) as usize;
                pos += method_length;
            }
        }

        // Grab the attributes
        let mut attributes = Vec::new();
        if flags.has_attributes() && pos + 2 <= entry_length {
            let attribute_count = u16::from_le_bytes([entry[pos], entry[pos + 1]]) as usize;
            pos += 2;

            attributes.reserve(attribute_count);
            for _ in 0..attribute_count {
                let attr = AdsAttribute::try_from_slice(&entry[pos..])?;
                pos += attr.wire_size();
                attributes.push(attr);
                if pos >= entry_length {
                    break;
                }
            }
        }

        // Skip the rest of the entry

        Ok((
            Self {
                version,
                hash_value,
                type_hash_value,
                size,
                offset,
                type_id,
                flags,
                name: name.into_owned(),
                type_name: type_name.into_owned(),
                comment: comment.into_owned(),
                guid,
                attributes,
            },
            entry_length,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured bytes
    #[rustfmt::skip]
    fn complex_field_bytes() -> &'static [u8] {
        const BYTES: [u8; 147] = [
            147, 0, 0, 0, 1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 56, 0, 0, 0, 19, 0, 0, 0,
            130, 16, 0, 0, 9, 0, 25, 0, 33, 0, 0, 0, 0, 0, 79, 117, 116, 70, 105, 101, 108, 100,
            50, 0, 84, 99, 50, 95, 83, 121, 115, 116, 101, 109, 46, 69, 95, 84, 99, 77, 101, 109,
            111, 114, 121, 65, 114, 101, 97, 0, 32, 84, 104, 105, 115, 32, 105, 115, 32, 97, 32,
            99, 111, 109, 109, 101, 110, 116, 32, 102, 111, 114, 32, 79, 117, 116, 70, 105, 101,
            108, 100, 50, 33, 0, 17, 70, 164, 80, 70, 136, 246, 162, 120, 191, 230, 194, 31, 223,
            55, 174, 1, 0, 11, 2, 104, 101, 108, 108, 111, 32, 116, 104, 101, 114, 101, 0, 52, 50, 0
        ];
        &BYTES
    }

    #[test]
    fn test_parse_complex_field_capture() {
        let data = complex_field_bytes();
        let (field, consumed) =
            AdsFieldInfo::try_from_slice(data).expect("Should parse complex field");

        assert_eq!(consumed, 147);
        assert_eq!(field.name(), "OutField2");
        assert_eq!(field.type_name(), "Tc2_System.E_TcMemoryArea");
        assert_eq!(field.comment(), " This is a comment for OutField2!");
        assert_eq!(field.offset(), 56);
        assert_eq!(field.size(), 4);

        // Flags check: 0x1082
        assert!(field.flags().has_attributes());
        assert!(field.flags().has_type_guid());

        // Attribute check
        assert_eq!(field.attributes().len(), 1);
        assert_eq!(field.attributes()[0].name(), "hello there");
        assert_eq!(field.attributes()[0].value(), "42");

        // GUID check
        assert!(field.guid().is_some());
    }

    #[test]
    fn test_complex_field_wire_size() {
        let data = complex_field_bytes();
        let (field, _) = AdsFieldInfo::try_from_slice(data).unwrap();

        // The computed wire size must exactly match the entry_length from the capture
        assert_eq!(field.wire_size(), 147);
    }

    #[test]
    fn test_complex_field_roundtrip() {
        let data = complex_field_bytes();
        let (field, _) = AdsFieldInfo::try_from_slice(data).unwrap();

        let serialized = field.to_vec();

        // Verify byte-for-byte symmetry
        assert_eq!(serialized, data);
    }
}
