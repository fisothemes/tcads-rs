use super::error::AdsTypeInfoError;
use super::{AdsAttribute, AdsDataTypeArrayInfo, AdsDataTypeFlags, AdsDataTypeId, Guid};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsFieldInfo {
    version: u32,
    hash_value: u32,
    type_hash_value: u32,
    size: u32,
    offset: u32,
    type_id: AdsDataTypeId,
    #[serde(skip_serializing)]
    flags: AdsDataTypeFlags,
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
    /// Byte size of the type. In bits if [`AdsDataTypeFlags::BIT_VALUES`] is set.
    pub fn size(&self) -> u32 {
        self.size
    }
    /// Byte offset of this field within its parent struct/function block/array.
    /// Zero for root type entries.
    pub fn offset(&self) -> u32 {
        self.offset
    }
    /// Primitive type identifier of the base or element type.
    pub fn data_type(&self) -> AdsDataTypeId {
        self.type_id
    }
    /// Type flags.
    pub fn flags(&self) -> AdsDataTypeFlags {
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
    /// 16-byte type GUID. Present when [`AdsDataTypeFlags::TYPE_GUID`] is set.
    pub fn guid(&self) -> Option<&Guid> {
        self.guid.as_ref()
    }
    /// Pragma key-value attributes. Non-empty when [`AdsDataTypeFlags::ATTRIBUTES`] is set.
    pub fn attributes(&self) -> &[AdsAttribute] {
        &self.attributes
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
        let type_id = AdsDataTypeId::from([entry[24], entry[25], entry[26], entry[27]]);
        let flags = AdsDataTypeFlags::from([entry[28], entry[29], entry[30], entry[31]]);
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
        pos += AdsDataTypeArrayInfo::LENGTH * array_dim_count;

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
