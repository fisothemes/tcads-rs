use super::error::AdsSymbolInfoError;
use crate::{
    AdsAttribute, AdsRefactorInfo, AdsSymbol2Flags, AdsSymbolFlags, AdsTypeId, Guid, IndexGroup,
    IndexOffset,
};

/// Information about a TwinCAT ADS Symbol/variable.
///
/// # Wire Format
/// | Offset | Size | Field            | Description                                  |
/// |--------|------|------------------|----------------------------------------------|
/// | 0      | 4    | `entry_length`   | Total size including dynamic tail            |
/// | 4      | 4    | `index_group`    | Index Group (e.g. 0x4020 for %M)             |
/// | 8      | 4    | `index_offset`   | Index Offset within the group                |
/// | 12     | 4    | `size`           | Total byte size (0 = bit value)              |
/// | 16     | 4    | `data_type_id`   | AdsDataTypeId                                |
/// | 20     | 2    | `flags`          | Symbol flags ([`AdsSymbolFlags`])            |
/// | 22     | 2    | `reserved`       | Legacy array dimension count (always 0)      |
/// | 24     | 2    | `name_len`       | Length of symbol name excl. null             |
/// | 26     | 2    | `type_len`       | Length of type name excl. null               |
/// | 28     | 2    | `comment_len`    | Length of comment excl. null                 |
/// | 30     | dyn  | `Strings`        | Windows-1252, null-terminated                |
/// | dyn    | dyn  | `type_guid`      | 16-byte GUID if flag is set                  |
/// | dyn    | dyn  | `Attributes`     | Prefixed by `u16` count if flag is set       |
/// | dyn    | dyn  | `extended_flags` | 4 bytes ([`AdsSymbol2Flags`]) if flag is set |
/// | dyn    | dyn  | `refactor_infos` | Chained [`AdsRefactorInfo`] if extended flag |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsSymbolInfo {
    index_group: IndexGroup,
    index_offset: IndexOffset,
    size: u32,
    data_type_id: AdsTypeId,
    #[serde(skip_serializing)]
    flags: AdsSymbolFlags,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    extended_flags: Option<AdsSymbol2Flags>,
    name: String,
    type_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    comment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    guid: Option<Guid>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<AdsAttribute>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    refactor_infos: Vec<AdsRefactorInfo>,
    #[serde(skip)]
    reserved: usize,
}

impl AdsSymbolInfo {
    /// Byte size of the fixed header.
    pub const MIN_LENGTH: usize = 30;

    /// The Index Group of the memory area where this symbol is located.
    pub fn index_group(&self) -> u32 {
        self.index_group
    }
    /// The byte offset within the Index Group where this symbol starts.
    pub fn index_offset(&self) -> u32 {
        self.index_offset
    }
    /// The byte size of this symbol's data (0 if it is a bit value).
    pub fn size(&self) -> u32 {
        self.size
    }
    /// The primitive data type identifier.
    pub fn data_type_id(&self) -> AdsTypeId {
        self.data_type_id
    }
    /// The standard symbol flags.
    pub fn flags(&self) -> AdsSymbolFlags {
        self.flags
    }
    /// The extended symbol flags (if present).
    pub fn extended_flags(&self) -> Option<AdsSymbol2Flags> {
        self.extended_flags
    }
    /// The full instance path of the symbol (e.g. `"MAIN.stMotor"`).
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The data type name of the symbol (e.g. `"ST_Motor"`).
    pub fn type_name(&self) -> &str {
        &self.type_name
    }
    /// The comment attached to the symbol in the PLC source code.
    pub fn comment(&self) -> &str {
        &self.comment
    }
    /// A unique identifier for the symbol's type, if available.
    pub fn guid(&self) -> Option<&Guid> {
        self.guid.as_ref()
    }
    /// Attributes and Pragmas attached to this symbol.
    pub fn attributes(&self) -> &[AdsAttribute] {
        &self.attributes
    }
    /// Refactoring history of this symbol.
    pub fn refactor_infos(&self) -> &[AdsRefactorInfo] {
        &self.refactor_infos
    }

    /// Parses an [`AdsSymbolInfo`] from a byte slice.
    ///
    /// Returns the parsed struct and the total number of bytes consumed (the `entry_length`).
    pub fn try_from_slice(data: &[u8]) -> Result<(Self, usize), AdsSymbolInfoError> {
        if data.len() < Self::MIN_LENGTH {
            return Err(AdsSymbolInfoError::TooShort {
                expected: Self::MIN_LENGTH,
                got: data.len(),
            });
        }

        let entry_length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        if data.len() < entry_length {
            return Err(AdsSymbolInfoError::EntryLengthMismatch {
                expected: entry_length,
                got: data.len(),
            });
        }

        let entry = &data[..entry_length];

        let index_group = IndexGroup::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]);
        let index_offset = IndexOffset::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);
        let size = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);
        let data_type_id = AdsTypeId::from([entry[16], entry[17], entry[18], entry[19]]);
        let flags = AdsSymbolFlags::from_bytes([entry[20], entry[21]]);
        // entry[22], entry[23] are reserved (legacy array dimension count)
        let name_length = u16::from_le_bytes([entry[24], entry[25]]) as usize;
        let type_name_length = u16::from_le_bytes([entry[26], entry[27]]) as usize;
        let comment_length = u16::from_le_bytes([entry[28], entry[29]]) as usize;

        let mut pos = Self::MIN_LENGTH;

        let name_end = pos + name_length + 1;
        let type_end = name_end + type_name_length + 1;
        let comment_end = type_end + comment_length + 1;

        if entry_length < comment_end {
            return Err(AdsSymbolInfoError::EntryLengthMismatch {
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

        let mut guid = None;
        if flags.has_type_guid() && pos + Guid::LENGTH <= entry_length {
            guid = Some(Guid::try_from_slice(&entry[pos..pos + Guid::LENGTH]).unwrap_or_default());
            pos += Guid::LENGTH;
        }

        let mut attributes = Vec::new();
        if flags.has_attributes() && pos + 2 <= entry_length {
            let attr_count = u16::from_le_bytes([entry[pos], entry[pos + 1]]) as usize;
            pos += 2;

            attributes.reserve(attr_count);
            for _ in 0..attr_count {
                if let Ok(attr) = AdsAttribute::try_from_slice(&entry[pos..]) {
                    pos += attr.wire_size();
                    attributes.push(attr);
                } else {
                    break; // Malformed attribute block, safely exit parsing
                }
                if pos >= entry_length {
                    break;
                }
            }
        }

        let mut extended_flags = None;
        let mut refactor_infos = Vec::new();

        if flags.has_extended_flags() && pos + 4 <= entry_length {
            let ext = AdsSymbol2Flags::from_bytes([
                entry[pos],
                entry[pos + 1],
                entry[pos + 2],
                entry[pos + 3],
            ]);
            extended_flags = Some(ext);
            pos += 4;

            if ext.has_refactor_info()
                && pos < entry_length
                && let Ok((infos, consumed)) =
                    AdsRefactorInfo::parse_chain(&entry[pos..entry_length])
            {
                refactor_infos = infos;
                pos += consumed;
            }
        }

        if pos > entry_length {
            return Err(AdsSymbolInfoError::EntryLengthMismatch {
                expected: entry_length,
                got: pos,
            });
        }

        Ok((
            Self {
                index_group,
                index_offset,
                size,
                data_type_id,
                flags,
                extended_flags,
                name: name.to_string(),
                type_name: type_name.to_string(),
                comment: comment.to_string(),
                guid,
                attributes,
                refactor_infos,
                reserved: entry_length.saturating_sub(pos),
            },
            entry_length,
        ))
    }
}

impl TryFrom<&[u8]> for AdsSymbolInfo {
    type Error = AdsSymbolInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let (info, _) = Self::try_from_slice(value)?;
        Ok(info)
    }
}

/// An iterator that safely consumes a contiguous byte blob of multiple TwinCAT ADS Symbol information
/// and parses then into [`AdsSymbolInfo`].
///
/// Because each Symbol Information has a dynamic length, this iterator lazily reads the length header
/// of the current entry, parses it, and advances the cursor to the exact start of the next entry.
///
/// # Fault Tolerance
///
/// If an individual Symbol Information fails to parse (yielding an `Err`), the iterator uses the
/// wire-format length header to safely skip the corrupted entry. Subsequent calls to `.next()` will
/// correctly align with the next valid Data Type in the stream.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdsSymbolInfoIterator<'a> {
    data: &'a [u8],
    cursor: usize,
}

impl<'a> AdsSymbolInfoIterator<'a> {
    /// Creates a new borrowed iterator from a raw byte payload containing multiple symbol information.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, cursor: 0 }
    }

    /// Converts this borrowed iterator into an owned iterator by copying the underlying slice.
    ///
    /// The iteration state (cursor position) is preserved. Any elements already yielded
    /// by this iterator will not be yielded by the new owned iterator.
    pub fn into_owned(self) -> AdsSymbolInfoIteratorOwned {
        AdsSymbolInfoIteratorOwned {
            buffer: self.data.into(),
            cursor: self.cursor,
        }
    }

    /// Creates an owned iterator by cloning the underlying slice.
    ///
    /// The iteration state (cursor position) is preserved in the cloned iterator.
    pub fn to_owned(&self) -> AdsSymbolInfoIteratorOwned {
        AdsSymbolInfoIteratorOwned {
            buffer: self.data.to_vec(),
            cursor: self.cursor,
        }
    }
}

impl<'a> Iterator for AdsSymbolInfoIterator<'a> {
    type Item = Result<AdsSymbolInfo, AdsSymbolInfoError>;

    fn next(&mut self) -> Option<Self::Item> {
        utils::parse_next_entry(self.data, &mut self.cursor)
    }
}

/// An owned iterator that lazily parses TwinCAT ADS Symbol information from a heap-allocated byte buffer.
///
/// This is highly efficient for large PLC memory blobs (which can exceed several megabytes).
/// The network payload is stored once, and heavy string decoding and struct allocations
/// only occur when `.next()` is called.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdsSymbolInfoIteratorOwned {
    buffer: Vec<u8>,
    cursor: usize,
}

impl AdsSymbolInfoIteratorOwned {
    /// Creates a new owned iterator from a raw byte payload.
    pub fn new(buffer: Vec<u8>) -> Self {
        Self { buffer, cursor: 0 }
    }

    /// Returns a borrowed view of this iterator.
    ///
    /// The returned [`AdsSymbolInfoIterator`] shares the same cursor position. However, advancing
    /// the view will *not* advance the cursor of this owned iterator.
    pub fn as_view(&self) -> AdsSymbolInfoIterator<'_> {
        AdsSymbolInfoIterator {
            data: &self.buffer,
            cursor: self.cursor,
        }
    }
}

impl Iterator for AdsSymbolInfoIteratorOwned {
    type Item = Result<AdsSymbolInfo, AdsSymbolInfoError>;

    fn next(&mut self) -> Option<Self::Item> {
        utils::parse_next_entry(&self.buffer, &mut self.cursor)
    }
}

pub mod utils {
    use super::*;

    /// Shared logic for advancing a cursor and parsing a single [`AdsSymbolInfo`] from a blob.
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
    ) -> Option<Result<AdsSymbolInfo, AdsSymbolInfoError>> {
        if *cursor >= data.len() {
            return None;
        }

        if *cursor + 4 > data.len() {
            let err = AdsSymbolInfoError::TooShort {
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
            let err = AdsSymbolInfoError::EntryLengthMismatch {
                expected: entry_length,
                got: data.len() - *cursor,
            };
            *cursor = data.len(); // Poison iterator
            return Some(Err(err));
        }

        let entry_slice = &data[*cursor..*cursor + entry_length];
        *cursor += entry_length;

        Some(AdsSymbolInfo::try_from(entry_slice))
    }
}
