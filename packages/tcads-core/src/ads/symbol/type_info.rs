use super::error::AdsTypeInfoError;
use super::{
    AdsArrayInfo, AdsAttribute, AdsEnumInfo, AdsFieldInfo, AdsMethodInfo, AdsRefactorInfo,
    AdsTypeFlags, AdsTypeId, Guid,
};

/// TwinCAT ADS data type info.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsTypeInfo {
    version: u32,
    hash_value: u32,
    type_hash_value: u32,
    size: u32,
    type_id: AdsTypeId,
    #[serde(skip_serializing)]
    flags: AdsTypeFlags,
    name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    type_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    comment: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    array_infos: Vec<AdsArrayInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    field_infos: Vec<AdsFieldInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    guid: Option<Guid>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    method_infos: Vec<AdsMethodInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<AdsAttribute>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    enum_infos: Vec<AdsEnumInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    refactor_infos: Vec<AdsRefactorInfo>,
}

impl AdsTypeInfo {
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
    /// Primitive type identifier of the base or element type.
    pub fn type_id(&self) -> AdsTypeId {
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
    /// Array dimension bounds. Non-empty only for array types.
    pub fn array_infos(&self) -> &[AdsArrayInfo] {
        &self.array_infos
    }
    /// Struct fields, fully inlined. Non-empty only for struct types.
    pub fn field_infos(&self) -> &[AdsFieldInfo] {
        &self.field_infos
    }
    /// 16-byte type GUID. Present when [`AdsTypeFlags::TYPE_GUID`] is set.
    pub fn guid(&self) -> Option<&Guid> {
        self.guid.as_ref()
    }
    /// RPC method info. Non-empty when [`AdsTypeFlags::METHOD_INFOS`] is set.
    pub fn method_infos(&self) -> &[AdsMethodInfo] {
        &self.method_infos
    }
    /// Pragma key-value attributes. Non-empty when [`AdsTypeFlags::ATTRIBUTES`] is set.
    pub fn attributes(&self) -> &[AdsAttribute] {
        &self.attributes
    }
    /// Enum info. Non-empty when [`AdsTypeFlags::ENUM_INFOS`] is set.
    pub fn enum_infos(&self) -> &[AdsEnumInfo] {
        &self.enum_infos
    }
    /// Refactoring history. Non-empty when [`AdsTypeFlags::REFACTOR_INFO`] is set.
    pub fn refactor_infos(&self) -> &[AdsRefactorInfo] {
        &self.refactor_infos
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

        let entry = &data[..entry_length];

        let version = u32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]);
        let hash_value = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);
        let type_hash_value = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);
        let size = u32::from_le_bytes([entry[16], entry[17], entry[18], entry[19]]);
        // Skipped offset because it's always zero for a root type.
        let type_id = AdsTypeId::from([entry[24], entry[25], entry[26], entry[27]]);
        let flags = AdsTypeFlags::from([entry[28], entry[29], entry[30], entry[31]]);
        let name_length = u16::from_le_bytes([entry[32], entry[33]]) as usize;
        let type_name_length = u16::from_le_bytes([entry[34], entry[35]]) as usize;
        let comment_length = u16::from_le_bytes([entry[36], entry[37]]) as usize;
        let array_dim_count = u16::from_le_bytes([entry[38], entry[39]]) as usize;
        let field_count = u16::from_le_bytes([entry[40], entry[41]]) as usize;

        let mut pos = Self::MIN_LENGTH;

        // Null-terminated strings
        let name_end = pos + name_length + 1;
        let type_end = name_end + type_name_length + 1;
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
            array_infos.push(AdsArrayInfo::try_from_slice(
                &entry[pos..pos + AdsArrayInfo::LENGTH],
            )?);
            pos += AdsArrayInfo::LENGTH;
        }

        let mut field_infos = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            let (sub_item, sub_item_entry_length) = AdsFieldInfo::try_from_slice(&entry[pos..])?;
            pos += sub_item_entry_length;
            field_infos.push(sub_item);
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

        let mut method_infos = Vec::new();
        if flags.has_method_infos() {
            let method_count = u16::from_le_bytes([entry[pos], entry[pos + 1]]) as usize;
            pos += 2;

            method_infos.reserve(method_count);
            for _ in 0..method_count {
                if pos + 4 <= entry_length {
                    break;
                }
                let (method, bytes_consumed) = AdsMethodInfo::try_from_slice(&entry[pos..])?;
                pos += bytes_consumed;
                method_infos.push(method);
            }
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
                if pos >= entry_length {
                    break;
                }
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
                if pos >= entry_length {
                    break;
                }
            }
        }

        let mut refactor_infos = Vec::new();
        if flags.has_refactor_info() {
            let (infos, consumed) = AdsRefactorInfo::parse_chain(&entry[pos..entry_length])?;
            refactor_infos = infos;
            pos += consumed;
        }

        if flags.has_extended_flags() {
            pos += 4;
        }

        if flags.is_variant() && pos + 2 <= entry_length {
            // Skip variant/deref type info for now
            let deref_count = u16::from_le_bytes([entry[pos], entry[pos + 1]]) as usize;
            pos += 2;

            for _ in 0..deref_count {
                if pos + 16 <= entry_length {
                    pos += 16;
                }
            }
        }

        if flags.has_extended_enum_infos() {
            for enum_info in &mut enums {
                if pos + 4 <= entry_length {
                    let bytes_consumed = enum_info.extend_from_slice(&entry[pos..])?;
                    pos += bytes_consumed;
                }
            }
        }

        Ok((
            Self {
                version,
                hash_value,
                type_hash_value,
                size,
                type_id,
                flags,
                name: name.to_string(),
                type_name: type_name.to_string(),
                comment: comment.to_string(),
                array_infos,
                field_infos,
                guid,
                method_infos,
                attributes,
                enum_infos: enums,
                refactor_infos,
            },
            entry_length,
        ))
    }
}

impl TryFrom<&[u8]> for AdsTypeInfo {
    type Error = AdsTypeInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let (info, _) = Self::try_from_slice(value)?;
        Ok(info)
    }
}

/// An iterator that safely consumes a contiguous byte blob of multiple TwinCAT Data Types and
/// parses them into [`AdsTypeInfo`]s.
///
/// Because each Data Type has a dynamic length, this iterator lazily reads the length header
/// of the current entry, parses it, and advances the cursor to the exact start of the next entry.
///
/// # Fault Tolerance
///
/// If an individual Data Type fails to parse (yielding an `Err`), the iterator uses the wire-format
/// length header to safely skip the corrupted entry. Subsequent calls to `.next()` will correctly
/// align with the next valid Data Type in the stream.
pub struct AdsTypeInfoIterator<'a> {
    data: &'a [u8],
    cursor: usize,
}

impl<'a> AdsTypeInfoIterator<'a> {
    /// Creates a new borrowed iterator from a raw byte payload containing multiple Data Types.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, cursor: 0 }
    }

    /// Converts this borrowed iterator into an owned iterator by copying the underlying slice.
    ///
    /// The iteration state (cursor position) is preserved. Any elements already yielded
    /// by this iterator will not be yielded by the new owned iterator.
    pub fn into_owned(self) -> AdsTypeInfoIteratorOwned {
        AdsTypeInfoIteratorOwned {
            buffer: self.data.into(),
            cursor: self.cursor,
        }
    }

    /// Creates an owned iterator by cloning the underlying slice.
    ///
    /// The iteration state (cursor position) is preserved in the cloned iterator.
    pub fn to_owned(&self) -> AdsTypeInfoIteratorOwned {
        AdsTypeInfoIteratorOwned {
            buffer: self.data.to_vec(),
            cursor: self.cursor,
        }
    }
}

impl<'a> Iterator for AdsTypeInfoIterator<'a> {
    type Item = Result<AdsTypeInfo, AdsTypeInfoError>;

    fn next(&mut self) -> Option<Self::Item> {
        utils::parse_next_entry(self.data, &mut self.cursor)
    }
}

/// An owned iterator that lazily parses TwinCAT Data Types from a heap-allocated byte buffer.
///
/// This is highly efficient for large PLC memory blobs (which can exceed several megabytes).
/// The network payload is stored once, and heavy string decoding and struct allocations
/// only occur when `.next()` is called.
pub struct AdsTypeInfoIteratorOwned {
    buffer: Vec<u8>,
    cursor: usize,
}

impl AdsTypeInfoIteratorOwned {
    /// Creates a new owned iterator from a raw byte payload.
    pub fn new(buffer: Vec<u8>) -> Self {
        Self { buffer, cursor: 0 }
    }

    /// Returns a borrowed view of this iterator.
    ///
    /// The returned [`AdsTypeInfoIterator`] shares the same cursor position. However, advancing
    /// the view will *not* advance the cursor of this owned iterator.
    pub fn as_view(&self) -> AdsTypeInfoIterator<'_> {
        AdsTypeInfoIterator {
            data: &self.buffer,
            cursor: self.cursor,
        }
    }
}

impl Iterator for AdsTypeInfoIteratorOwned {
    type Item = Result<AdsTypeInfo, AdsTypeInfoError>;

    fn next(&mut self) -> Option<Self::Item> {
        utils::parse_next_entry(&self.buffer, &mut self.cursor)
    }
}

pub mod utils {
    use super::*;

    /// Shared logic for advancing a cursor and parsing a single Data Type from a byte blob.
    ///
    /// # Behavior
    ///
    /// - If the buffer is severely truncated or lacks a length header, it poisons the cursor
    ///   to prevent infinite iteration loops.
    /// - If the struct parser fails but the boundaries are intact, it yields an `Err` but safely
    ///   advances the cursor to the next struct.
    pub fn parse_next_entry(
        data: &[u8],
        cursor: &mut usize,
    ) -> Option<Result<AdsTypeInfo, AdsTypeInfoError>> {
        if *cursor >= data.len() {
            return None;
        }

        if *cursor + 4 > data.len() {
            let err = AdsTypeInfoError::TooShort {
                expected: *cursor + 4,
                got: data.len(),
            };
            *cursor = data.len(); // Poison iterator to prevent infinite loop
            return Some(Err(err));
        }

        let entry_length = u32::from_le_bytes([
            data[*cursor],
            data[*cursor + 1],
            data[*cursor + 2],
            data[*cursor + 3],
        ]) as usize;

        if *cursor + entry_length > data.len() {
            let err = AdsTypeInfoError::EntryLengthMismatch {
                expected: entry_length,
                got: data.len() - *cursor,
            };
            *cursor = data.len(); // Poison iterator
            return Some(Err(err));
        }

        let entry_slice = &data[*cursor..*cursor + entry_length];

        *cursor += entry_length;

        Some(AdsTypeInfo::try_from(entry_slice))
    }
}
