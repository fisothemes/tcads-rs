use super::error::AdsTypeInfoError;
use super::{
    AdsAttribute, AdsDataTypeArrayInfo, AdsDataTypeFlags, AdsDataTypeId, AdsEnumInfo, Guid,
};

/// TwinCAT ADS data type info.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsDataTypeInfo {
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    array_infos: Vec<AdsDataTypeArrayInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sub_items: Vec<AdsDataTypeInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    guid: Option<Guid>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<AdsAttribute>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    enum_infos: Vec<AdsEnumInfo>,
}

impl AdsDataTypeInfo {
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
    /// Array dimension bounds. Non-empty only for array types.
    pub fn array_infos(&self) -> &[AdsDataTypeArrayInfo] {
        &self.array_infos
    }
    /// Struct fields, fully inlined. Non-empty only for struct types.
    pub fn sub_items(&self) -> &[AdsDataTypeInfo] {
        &self.sub_items
    }
    /// 16-byte type GUID. Present when [`AdsDataTypeFlags::TYPE_GUID`] is set.
    pub fn guid(&self) -> Option<&Guid> {
        self.guid.as_ref()
    }
    /// Pragma key-value attributes. Non-empty when [`AdsDataTypeFlags::ATTRIBUTES`] is set.
    pub fn attributes(&self) -> &[AdsAttribute] {
        &self.attributes
    }

    /// Parses an [`AdsTypeInfo`] from a byte slice.
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

        // Work within the declared boundary
        let entry = &data[..entry_length];

        let version = u32::from_le_bytes(entry[4..8].try_into().unwrap());
        let hash_value = u32::from_le_bytes(entry[8..12].try_into().unwrap());
        let type_hash_value = u32::from_le_bytes(entry[12..16].try_into().unwrap());
        let size = u32::from_le_bytes(entry[16..20].try_into().unwrap());
        let offset = u32::from_le_bytes(entry[20..24].try_into().unwrap());
        let type_id = AdsDataTypeId::from(u32::from_le_bytes(entry[24..28].try_into().unwrap()));
        let flags = AdsDataTypeFlags::from(u32::from_le_bytes(entry[28..32].try_into().unwrap()));
        let name_length = u16::from_le_bytes(entry[32..34].try_into().unwrap()) as usize;
        let type_length = u16::from_le_bytes(entry[34..36].try_into().unwrap()) as usize;
        let comment_length = u16::from_le_bytes(entry[36..38].try_into().unwrap()) as usize;
        let array_dim_count = u16::from_le_bytes(entry[38..40].try_into().unwrap()) as usize;
        let sub_item_count = u16::from_le_bytes(entry[40..42].try_into().unwrap()) as usize;

        let mut pos = Self::MIN_LENGTH;

        // Null-terminated strings
        let name_end = pos + name_length + 1;
        let type_end = name_end + type_length + 1;
        let comment_end = type_end + comment_length + 1;

        if (entry_length) < comment_end {
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

        let mut array_infos = Vec::with_capacity(array_dim_count);
        for _ in 0..array_dim_count {
            array_infos.push(AdsDataTypeArrayInfo::try_from_slice(
                &entry[pos..pos + AdsDataTypeArrayInfo::LENGTH],
            )?);
            pos += AdsDataTypeArrayInfo::LENGTH;
        }

        let mut sub_items = Vec::with_capacity(sub_item_count);
        for _ in 0..sub_item_count {
            let (sub_item, sub_item_entry_length) = AdsDataTypeInfo::try_from_slice(&entry[pos..])?;
            pos += sub_item_entry_length;
            sub_items.push(sub_item);
        }

        let mut guid = None;
        if flags.has_type_guid() && pos + 16 <= entry_length {
            guid = Some(Guid::try_from_slice(&entry[pos..pos + 16])?);
            pos += Guid::LENGTH;
        }

        if flags.has_copy_mask() {
            // Legacy section, skip it
            pos = (pos + size as usize).min(entry_length);
        }

        if flags.has_method_infos() {
            todo!("Method info section not yet implemented");
        }

        let mut attributes = Vec::new();
        if flags.has_attributes() && pos + 2 <= entry_length {
            let attr_count = u16::from_le_bytes(entry[pos..pos + 2].try_into().unwrap()) as usize;
            pos += 2;

            attributes.reserve(attr_count);
            for _ in 0..attr_count {
                let attr = AdsAttribute::try_from_slice(&entry[pos..])?;
                pos += attr.wire_size();
                attributes.push(attr);
            }
        }

        let mut enums = Vec::new();
        if flags.has_enum_infos() && pos + 2 <= entry_length {
            let enum_count = u16::from_le_bytes([entry[pos], entry[pos + 1]]) as usize;
            pos += 2;
            enums.reserve(enum_count);
            for _ in 0..enum_count {
                let (enum_info, bytes_consumed) =
                    AdsEnumInfo::try_from_slice(&entry[pos..], size as usize)?;
                pos += bytes_consumed;
                enums.push(enum_info);
            }
        }

        if flags.has_refactor_info() {
            todo!("Refactor info section not yet implemented");
        }

        if flags.has_extended_flags() {
            pos += 4;
        }

        if flags.is_variant() {
            todo!("Variant type not yet implemented");
        }

        if flags.has_extended_enum_infos() {
            for enum_info in &mut enums {
                if pos + 4 <= entry_length {
                    let bytes_consumed = enum_info.extend_from_slice(&entry[pos..])?;
                    pos += bytes_consumed;
                }
            }
        }

        if flags.has_software_protection_levels() {
            todo!("Software protection levels section not yet implemented");
        }

        Ok((
            Self {
                version,
                hash_value,
                type_hash_value,
                size,
                offset,
                type_id,
                flags,
                name: name.to_string(),
                type_name: type_name.to_string(),
                comment: comment.to_string(),
                array_infos,
                sub_items,
                guid,
                attributes,
                enum_infos: enums,
            },
            entry_length,
        ))
    }
}

impl TryFrom<&[u8]> for AdsDataTypeInfo {
    type Error = AdsTypeInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let (info, _) = Self::try_from_slice(value)?;
        Ok(info)
    }
}
