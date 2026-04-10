use super::error::AdsTypeInfoError;
use super::{
    AdsAttribute, AdsDataTypeArrayInfo, AdsDataTypeFlags, AdsDataTypeId, AdsEnumInfo,
    AdsMethodInfo, AdsRefactorInfo, Guid,
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
    method_infos: Vec<AdsMethodInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<AdsAttribute>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    enum_infos: Vec<AdsEnumInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    refactor_infos: Vec<AdsRefactorInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    platform_pointer_size: Option<u8>,
}

impl AdsDataTypeInfo {
    /// Byte size of the fixed header.
    pub const MIN_LENGTH: usize = 42;

    pub fn with_platform_pointer_size(mut self, size: impl Into<Option<u8>>) -> Self {
        self.platform_pointer_size = size.into();
        self
    }

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
    /// RPC method info. Non-empty when [`AdsDataTypeFlags::METHOD_INFOS`] is set.
    pub fn method_infos(&self) -> &[AdsMethodInfo] {
        &self.method_infos
    }
    /// Pragma key-value attributes. Non-empty when [`AdsDataTypeFlags::ATTRIBUTES`] is set.
    pub fn attributes(&self) -> &[AdsAttribute] {
        &self.attributes
    }
    /// Enum info. Non-empty when [`AdsDataTypeFlags::ENUM_INFOS`] is set.
    pub fn enum_infos(&self) -> &[AdsEnumInfo] {
        &self.enum_infos
    }
    /// Refactoring history. Non-empty when [`AdsDataTypeFlags::REFACTOR_INFO`] is set.
    pub fn refactor_infos(&self) -> &[AdsRefactorInfo] {
        &self.refactor_infos
    }
    /// Size of the platform-specific pointer (i.e. 32-bit = 4 bytes, 64-bit = 8 bytes).
    /// Must be set manually and is only needed for working out if the [`AdsDataTypeCategory`]
    /// is an interface.
    pub fn platform_pointer_size(&self) -> Option<u8> {
        self.platform_pointer_size
    }

    /// Set the platform-specific pointer size. Without this, the [`AdsDataTypeCategory`]
    /// for an interface will be inferred.
    pub fn set_platform_pointer_size(&mut self, size: impl Into<Option<u8>>) {
        self.platform_pointer_size = size.into();
    }

    /// Evaluates the metadata and structure to determine the high-level category of this type.
    pub fn category(&self) -> AdsDataTypeCategory {
        // Pointers
        if self.flags.is_plc_pointer_type()
            || self.name == "PVOID"
            || self.name == "PCCH"
            || self.name.starts_with("POINTER TO")
        {
            return AdsDataTypeCategory::Pointer;
        }

        // References
        if self.flags.is_reference_to() || self.name.starts_with("REFERENCE TO") {
            return AdsDataTypeCategory::Reference;
        }

        // Arrays
        if !self.array_infos.is_empty() {
            return AdsDataTypeCategory::Array;
        }

        // Enums
        if self.flags.has_enum_infos() {
            return AdsDataTypeCategory::Enum;
        }

        // Sub-ranges
        if self.name.contains("..") && self.name.contains('(') && self.name.contains(')') {
            return AdsDataTypeCategory::SubRange;
        }

        // Alias
        if !self.type_name.is_empty() && self.sub_items.is_empty() && self.method_infos.is_empty() {
            return AdsDataTypeCategory::Alias;
        }

        match self.type_id {
            // Primitives
            AdsDataTypeId::Int8
            | AdsDataTypeId::UInt8
            | AdsDataTypeId::Int16
            | AdsDataTypeId::UInt16
            | AdsDataTypeId::Int32
            | AdsDataTypeId::UInt32
            | AdsDataTypeId::Int64
            | AdsDataTypeId::UInt64
            | AdsDataTypeId::Real32
            | AdsDataTypeId::Real64
            | AdsDataTypeId::Real80
            | AdsDataTypeId::Bit => return AdsDataTypeCategory::Primitive,
            // Strings
            AdsDataTypeId::String | AdsDataTypeId::WString => return AdsDataTypeCategory::String,
            _ => {
                if matches!(
                    self.name.as_str(),
                    "TOD"
                        | "DT"
                        | "TIME"
                        | "DATE"
                        | "LTIME"
                        | "TIME_OF_DAY"
                        | "DATE_AND_TIME"
                        | "UXINT"
                        | "XINT"
                        | "XWORD"
                        | "__UXINT"
                        | "__XINT"
                        | "__XWORD"
                ) {
                    return AdsDataTypeCategory::Primitive;
                }
            }
        }

        // Unions (Check for memory offset overlap among sub-items)
        if !self.sub_items.is_empty()
            && self.type_name.is_empty()
            && self.method_infos.is_empty()
            && self.has_field_offset_overlap()
        {
            return AdsDataTypeCategory::Union;
        }

        // Complex Types: Interfaces, Function Blocks, and Structs
        if !self.sub_items.is_empty() || !self.method_infos.is_empty() {
            // FBs and Interfaces often expose methods
            if !self.method_infos.is_empty() {
                // Check for Interface implementations
                if self
                    .attributes
                    .iter()
                    .any(|attr| attr.name() == "TcImplements")
                {
                    return AdsDataTypeCategory::FunctionBlock;
                }
                if self.sub_items.is_empty() {
                    return AdsDataTypeCategory::Interface;
                }
                // Stable Rust alternative to let_chains
                if self
                    .platform_pointer_size
                    .is_some_and(|plat_size| self.size == plat_size as u32)
                {
                    return AdsDataTypeCategory::Interface;
                }
                return AdsDataTypeCategory::FunctionBlock;
            }

            // If the first user-defined sub-item does NOT start at offset 0, it has hidden state (FB)
            if self
                .sub_items
                .first()
                .is_some_and(|child| child.offset() > 0)
            {
                return AdsDataTypeCategory::FunctionBlock;
            }

            return AdsDataTypeCategory::Struct;
        }

        // Things are getting funky now...
        if self.sub_items.is_empty() {
            if !self.name.is_empty() && self.size == 0 {
                return AdsDataTypeCategory::Struct;
            }

            // Check for Interface implementations
            if self
                .attributes
                .iter()
                .any(|attr| attr.name() == "TcImplements")
            {
                return AdsDataTypeCategory::FunctionBlock;
            }

            if self.size.is_multiple_of(4) && self.size > 8 {
                return AdsDataTypeCategory::FunctionBlock;
            }

            if self.size == 4 || self.size == 8 {
                return AdsDataTypeCategory::Interface;
            }
        }

        AdsDataTypeCategory::None
    }

    /// Checks if the sub-items of this data type overlap in memory (indicative of a `UNION`).
    fn has_field_offset_overlap(&self) -> bool {
        if self.sub_items.len() <= 1 {
            return false;
        }

        let mut max_bit_pos = 0;
        for sub in &self.sub_items {
            // Skip properties and statics as they don't occupy linear instance memory
            if sub.flags().is_prop_item() || sub.flags().is_static() {
                continue;
            }

            let bit_offset = if sub.flags().is_bit_values() {
                sub.offset()
            } else {
                sub.offset() * 8
            };
            if bit_offset < max_bit_pos {
                return true; // Overlap detected!
            }
            let bit_size = if sub.flags().is_bit_values() {
                sub.size()
            } else {
                sub.size() * 8
            };
            max_bit_pos += bit_size;
        }
        false
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

        let mut method_infos = Vec::new();
        if flags.has_method_infos() {
            let method_count = u16::from_le_bytes([entry[pos], entry[pos + 1]]) as usize;
            pos += 2;

            method_infos.reserve(method_count);
            for _ in 0..method_count {
                let (method, bytes_consumed) = AdsMethodInfo::try_from_slice(&entry[pos..])?;
                pos += bytes_consumed;
                method_infos.push(method);
            }
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
                offset,
                type_id,
                flags,
                name: name.to_string(),
                type_name: type_name.to_string(),
                comment: comment.to_string(),
                array_infos,
                sub_items,
                guid,
                method_infos,
                attributes,
                enum_infos: enums,
                refactor_infos,
                platform_pointer_size: None,
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

/// Category of a TwinCAT Data Type or Instance.
///
/// This provides a high-level classification of the memory layout, simplifying
/// how consumers interpret the nested fields and properties of a PLC variable.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Default,
)]
pub enum AdsDataTypeCategory {
    /// Uninitialized or unknown data type.
    #[default]
    None,
    /// Simple base data type (e.g., `BOOL`, `INT`, `REAL`).
    Primitive,
    /// Type alias pointing to a base type.
    Alias,
    /// Enumeration type.
    Enum,
    /// Array data type.
    Array,
    /// Structure data type.
    Struct,
    /// Function block (POU) instance.
    FunctionBlock,
    /// Program (POU) instance.
    Program,
    /// Function (POU) instance.
    Function,
    /// Sub-range type.
    SubRange,
    /// String type (e.g., `STRING`, `WSTRING`).
    String,
    /// Bitset type.
    Bitset,
    /// Pointer type (`POINTER TO ...`, `PVOID`).
    Pointer,
    /// Union type (overlapping memory fields).
    Union,
    /// Reference type (`REFERENCE TO ...`).
    Reference,
    /// Interface pointer.
    Interface,
}

/// An iterator that safely consumes a contiguous byte blob of multiple TwinCAT Data Types and
/// parses them into [`AdsDataTypeInfo`]s.
///
/// Because each Data Type has a dynamic length, this iterator lazily reads the length header
/// of the current entry, parses it, and advances the cursor to the exact start of the next entry.
///
/// # Fault Tolerance
///
/// If an individual Data Type fails to parse (yielding an `Err`), the iterator uses the wire-format
/// length header to safely skip the corrupted entry. Subsequent calls to `.next()` will correctly
/// align with the next valid Data Type in the stream.
pub struct AdsDataTypeIterator<'a> {
    data: &'a [u8],
    cursor: usize,
    platform_pointer_size: Option<u8>,
}

impl<'a> AdsDataTypeIterator<'a> {
    /// Creates a new borrowed iterator from a raw byte payload containing multiple Data Types.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            cursor: 0,
            platform_pointer_size: None,
        }
    }

    /// Adds a platform pointer size to the borrowed iterator.
    pub fn with_platform_pointer_size(
        mut self,
        platform_pointer_size: impl Into<Option<u8>>,
    ) -> Self {
        self.platform_pointer_size = platform_pointer_size.into();
        self
    }

    /// Converts this borrowed iterator into an owned iterator by copying the underlying slice.
    ///
    /// The iteration state (cursor position) is preserved. Any elements already yielded
    /// by this iterator will not be yielded by the new owned iterator.
    pub fn into_owned(self) -> AdsDataTypeIteratorOwned {
        AdsDataTypeIteratorOwned {
            buffer: self.data.into(),
            cursor: self.cursor,
            platform_pointer_size: self.platform_pointer_size,
        }
    }

    /// Creates an owned iterator by cloning the underlying slice.
    ///
    /// The iteration state (cursor position) is preserved in the cloned iterator.
    pub fn to_owned(&self) -> AdsDataTypeIteratorOwned {
        AdsDataTypeIteratorOwned {
            buffer: self.data.to_vec(),
            cursor: self.cursor,
            platform_pointer_size: self.platform_pointer_size,
        }
    }
}

impl<'a> Iterator for AdsDataTypeIterator<'a> {
    type Item = Result<AdsDataTypeInfo, AdsTypeInfoError>;

    fn next(&mut self) -> Option<Self::Item> {
        utils::parse_next_entry(self.data, &mut self.cursor, self.platform_pointer_size)
    }
}

/// An owned iterator that lazily parses TwinCAT Data Types from a heap-allocated byte buffer.
///
/// This is highly efficient for large PLC memory blobs (which can exceed several megabytes).
/// The network payload is stored once, and heavy string decoding and struct allocations
/// only occur when `.next()` is called.
pub struct AdsDataTypeIteratorOwned {
    buffer: Vec<u8>,
    cursor: usize,
    platform_pointer_size: Option<u8>,
}

impl AdsDataTypeIteratorOwned {
    /// Creates a new owned iterator from a raw byte payload.
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
            cursor: 0,
            platform_pointer_size: None,
        }
    }

    /// Adds a platform pointer size to the owned iterator.
    pub fn with_platform_pointer_size(
        mut self,
        platform_pointer_size: impl Into<Option<u8>>,
    ) -> Self {
        self.platform_pointer_size = platform_pointer_size.into();
        self
    }

    /// Returns a borrowed view of this iterator.
    ///
    /// The returned [`AdsDataTypeIterator`] shares the same cursor position. However, advancing
    /// the view will *not* advance the cursor of this owned iterator.
    pub fn as_view(&self) -> AdsDataTypeIterator<'_> {
        AdsDataTypeIterator {
            data: &self.buffer,
            cursor: self.cursor,
            platform_pointer_size: self.platform_pointer_size,
        }
    }
}

impl Iterator for AdsDataTypeIteratorOwned {
    type Item = Result<AdsDataTypeInfo, AdsTypeInfoError>;

    fn next(&mut self) -> Option<Self::Item> {
        utils::parse_next_entry(&self.buffer, &mut self.cursor, self.platform_pointer_size)
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
        platform_pointer_size: Option<u8>,
    ) -> Option<Result<AdsDataTypeInfo, AdsTypeInfoError>> {
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

        let result = AdsDataTypeInfo::try_from(entry_slice);

        if result.is_err() {
            return Some(result);
        }

        let dt = result.unwrap();

        Some(Ok(dt.with_platform_pointer_size(platform_pointer_size)))
    }
}
