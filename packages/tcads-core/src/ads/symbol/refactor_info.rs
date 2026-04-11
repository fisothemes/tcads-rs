use super::error::AdsTypeInfoError;

/// Refactoring history entry attached to a TwinCAT data type.
///
/// Present in [`AdsDataTypeInfo`](super::AdsDataTypeInfo) when
/// [`AdsDataTypeFlags::REFACTOR_INFO`](super::AdsTypeFlags::REFACTOR_INFO) is set.
///
/// # Note
///
/// When parsing, multiple entries may be chained. So parse until [`next_refactor_info`](Self::next_refactor_info)
/// returns `false`.
///
/// # Wire Format
///
/// | Offset | Size          | Field              | Description                                       |
/// |--------|---------------|--------------------|---------------------------------------------------|
/// | 0      | 4             | packed             | `refactor_count` (bits masked by `0x01111111`)    |
/// |        |               |                    | `next_refactor_info` (bit 24, mask `0x01000000`)  |
/// | 4      | 2             | `name_length`      | Byte length of name excl. null                    |
/// | 6      | `name_len + 1`| `name`             | Windows-1252, null-terminated                     |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsRefactorInfo {
    /// Refactor version counter, extracted from bits masked by `0x01111111`.
    refactor_count: u32,
    next_refactor_info: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl AdsRefactorInfo {
    /// Minimum wire size in bytes (4 packed + 2 name_length + 1 null terminator).
    pub const MIN_LENGTH: usize = 7;

    /// Mask for extracting `refactor_count` from the packed `u32`.
    pub const REFACTOR_COUNT_MASK: u32 = 0x01111111;

    /// Mask for extracting the `next_refactor_info` flag from the packed `u32`.
    pub const NEXT_REFACTOR_INFO_MASK: u32 = 0x01000000;

    /// Refactor version counter.
    pub fn refactor_count(&self) -> u32 {
        self.refactor_count
    }

    /// Returns `true` if another [`AdsRefactorInfo`] entry follows in the wire stream.
    pub fn next_refactor_info(&self) -> bool {
        self.next_refactor_info
    }

    /// The name associated with this refactor entry, if present.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the wire size of this entry in bytes when serialised with the given
    /// `has_next` flag. The flag affects the packed `u32` value but not the length.
    pub fn wire_size(&self) -> usize {
        let name_len = self.name.as_deref().map_or(0, str::len);
        6 + name_len + 1 // 4 packed + 2 name_length + name + null
    }

    /// Serializes this entry into bytes.
    ///
    /// `has_next` controls the [`NEXT_REFACTOR_INFO_MASK`](Self::NEXT_REFACTOR_INFO_MASK)
    /// bit in the packed field, and the caller is responsible for setting it correctly
    /// based on whether another entry follows. Use [`entries_to_vec`](Self::entries_to_vec)
    /// to serialise a full chain without managing this manually
    pub fn to_vec_with_next(&self, has_next: bool) -> Vec<u8> {
        let entry_length = self.wire_size();

        let mut packed = self.refactor_count & Self::REFACTOR_COUNT_MASK;
        if has_next {
            packed |= Self::NEXT_REFACTOR_INFO_MASK;
        }

        let (name_enc, name_len) = match &self.name {
            Some(n) => {
                let (enc, _, _) = encoding_rs::WINDOWS_1252.encode(n);
                let len = enc.len();
                (enc.into_owned(), len)
            }
            None => (Vec::new(), 0),
        };

        let mut buf = Vec::with_capacity(entry_length);

        buf.extend_from_slice(&packed.to_le_bytes());
        buf.extend_from_slice(&(name_len as u16).to_le_bytes());
        buf.extend_from_slice(&name_enc);
        buf.push(0);

        buf
    }

    /// Serializes a slice of [`AdsRefactorInfo`] entries as a complete `REFACTOR_INFO`
    /// section, including a `u16` count prefix.
    ///
    /// The [`next_refactor_info`](Self::next_refactor_info) chain flag is set automatically
    /// on all entries except the last.
    pub fn entries_to_vec(entries: &[Self]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(entries.iter().map(|e| e.wire_size()).sum::<usize>() + 2);

        buf.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        for (i, entry) in entries.iter().enumerate() {
            let has_next = i < entries.len().saturating_sub(1);
            buf.extend_from_slice(&entry.to_vec_with_next(has_next));
        }
        buf
    }

    /// Parses an [`AdsRefactorInfo`] from a byte slice.
    /// Returns the parsed struct and the number of bytes consumed.
    pub fn try_from_slice(data: &[u8]) -> Result<(Self, usize), AdsTypeInfoError> {
        if data.len() < Self::MIN_LENGTH {
            return Err(AdsTypeInfoError::TooShort {
                expected: Self::MIN_LENGTH,
                got: data.len(),
            });
        }

        let packed = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let refactor_count = packed & Self::REFACTOR_COUNT_MASK;
        let next_refactor_info = (packed & Self::NEXT_REFACTOR_INFO_MASK) != 0;

        let name_length = u16::from_le_bytes(data[4..6].try_into().unwrap()) as usize;
        let name_end = 6 + name_length + 1;

        if data.len() < name_end {
            return Err(AdsTypeInfoError::TooShort {
                expected: name_end,
                got: data.len(),
            });
        }

        let name = if name_length > 0 {
            let (s, _, _) = encoding_rs::WINDOWS_1252.decode(&data[6..6 + name_length]);
            Some(s.into_owned())
        } else {
            None
        };

        Ok((
            Self {
                refactor_count,
                next_refactor_info,
                name,
            },
            name_end,
        ))
    }

    /// Parses a chain of [`AdsRefactorInfo`] entries from a byte slice, following
    /// [`next_refactor_info`](Self::next_refactor_info) links until the chain ends.
    ///
    /// Returns the parsed entries and the total number of bytes consumed.
    pub fn parse_chain(data: &[u8]) -> Result<(Vec<Self>, usize), AdsTypeInfoError> {
        let mut entries = Vec::new();
        let mut pos = 0;

        loop {
            let (entry, consumed) = Self::try_from_slice(&data[pos..])?;
            let has_next = entry.next_refactor_info();
            entries.push(entry);
            pos += consumed;
            if !has_next {
                break;
            }
        }

        Ok((entries, pos))
    }
}

impl TryFrom<&[u8]> for AdsRefactorInfo {
    type Error = AdsTypeInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let (info, _) = Self::try_from_slice(value)?;
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_entry_bytes(packed: u32, name: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&packed.to_le_bytes());
        buf.extend_from_slice(&(name.len() as u16).to_le_bytes());
        buf.extend_from_slice(name.as_bytes());
        buf.push(0); // null terminator
        buf
    }

    #[test]
    fn parses_single_entry_no_next() {
        // refactor_count = 1 (bit 0 set), next_refactor_info = false
        let bytes = single_entry_bytes(0x00000001, "OldName");
        let (entry, consumed) = AdsRefactorInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(entry.refactor_count(), 0x00000001);
        assert!(!entry.next_refactor_info());
        assert_eq!(entry.name(), Some("OldName"));
    }

    #[test]
    fn parses_entry_with_next_flag() {
        // next_refactor_info = true (bit 24 set via 0x01000000)
        let bytes = single_entry_bytes(0x01000001, "FirstName");
        let (entry, consumed) = AdsRefactorInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert!(entry.next_refactor_info());
        assert_eq!(entry.name(), Some("FirstName"));
        // refactor_count includes bit 24 since it overlaps the mask
        assert_eq!(entry.refactor_count(), 0x01000001 & 0x01111111);
    }

    #[test]
    fn parse_chain_two_entries() {
        let mut bytes = single_entry_bytes(0x01000001, "OriginalName"); // next = true
        bytes.extend(single_entry_bytes(0x00000002, "RenamedOnce")); // next = false

        let (entries, consumed) = AdsRefactorInfo::parse_chain(&bytes).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(consumed, bytes.len());
        assert_eq!(entries[0].name(), Some("OriginalName"));
        assert!(entries[0].next_refactor_info());
        assert_eq!(entries[1].name(), Some("RenamedOnce"));
        assert!(!entries[1].next_refactor_info());
    }

    #[test]
    fn parse_chain_stops_at_false() {
        // Two entries but extra trailing bytes — chain should stop after second entry
        let mut bytes = single_entry_bytes(0x01000001, "First"); // next = true
        bytes.extend(single_entry_bytes(0x00000001, "Second")); // next = false
        bytes.extend(single_entry_bytes(0x00000001, "ShouldNotBeParsed"));

        let (entries, consumed) = AdsRefactorInfo::parse_chain(&bytes).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(consumed < bytes.len()); // didn't consume the third entry
    }

    #[test]
    fn wire_size_matches_serialised_length() {
        let entry = AdsRefactorInfo {
            refactor_count: 1,
            next_refactor_info: false,
            name: Some("OldName".into()),
        };
        let buf = entry.to_vec_with_next(false);
        assert_eq!(entry.wire_size(), buf.len());
    }

    #[test]
    fn wire_size_empty_name_matches_serialised_length() {
        let entry = AdsRefactorInfo {
            refactor_count: 0,
            next_refactor_info: false,
            name: None,
        };
        let buf = entry.to_vec_with_next(false);
        assert_eq!(entry.wire_size(), buf.len());
    }

    #[test]
    fn write_into_sets_next_flag_when_has_next_true() {
        let entry = AdsRefactorInfo {
            refactor_count: 1,
            next_refactor_info: false, // stored value doesn't matter
            name: Some("A".to_string()),
        };
        let buf = entry.to_vec_with_next(true); // override: has_next = true
        let packed = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        assert_ne!(packed & 0x01000000, 0);
    }

    #[test]
    fn roundtrip_single_entry() {
        let entry = AdsRefactorInfo {
            refactor_count: 0x00000001,
            next_refactor_info: false,
            name: Some("OriginalName".to_string()),
        };
        let buf = entry.to_vec_with_next(false);
        let (parsed, consumed) = AdsRefactorInfo::try_from_slice(&buf).unwrap();
        assert_eq!(consumed, buf.len());
        assert_eq!(parsed.refactor_count(), entry.refactor_count());
        assert!(!parsed.next_refactor_info());
        assert_eq!(parsed.name(), Some("OriginalName"));
    }

    #[test]
    fn roundtrip_none_name() {
        let entry = AdsRefactorInfo {
            refactor_count: 0,
            next_refactor_info: false,
            name: None,
        };
        let buf = entry.to_vec_with_next(false);
        let (parsed, _) = AdsRefactorInfo::try_from_slice(&buf).unwrap();
        assert!(parsed.name().is_none());
    }

    #[test]
    fn write_chain_prefix_count() {
        let entries = vec![
            AdsRefactorInfo {
                refactor_count: 1,
                next_refactor_info: false,
                name: Some("A".to_string()),
            },
            AdsRefactorInfo {
                refactor_count: 2,
                next_refactor_info: false,
                name: Some("B".to_string()),
            },
        ];
        let buf = AdsRefactorInfo::entries_to_vec(&entries);

        // First 2 bytes = count = 2
        let count = u16::from_le_bytes(buf[0..2].try_into().unwrap());
        assert_eq!(count, 2);
    }

    #[test]
    fn write_chain_roundtrip() {
        let entries = vec![
            AdsRefactorInfo {
                refactor_count: 0x00000001,
                next_refactor_info: false,
                name: Some("OriginalName".to_string()),
            },
            AdsRefactorInfo {
                refactor_count: 0x00000011,
                next_refactor_info: false,
                name: Some("RenamedOnce".to_string()),
            },
            AdsRefactorInfo {
                refactor_count: 0x00000111,
                next_refactor_info: false,
                name: None,
            },
        ];
        let buf = AdsRefactorInfo::entries_to_vec(&entries);

        let (parsed, _) = AdsRefactorInfo::parse_chain(&buf[2..]).unwrap();
        assert_eq!(parsed.len(), 3);

        assert_eq!(parsed[0].refactor_count(), 0x01000001);
        assert_eq!(parsed[0].name(), Some("OriginalName"));
        assert!(parsed[0].next_refactor_info());

        assert_eq!(parsed[1].refactor_count(), 0x01000011);
        assert_eq!(parsed[1].name(), Some("RenamedOnce"));
        assert!(parsed[1].next_refactor_info());

        assert_eq!(parsed[2].refactor_count(), 0x00000111);
        assert_eq!(parsed[2].name(), None);
        assert!(!parsed[2].next_refactor_info());
    }

    #[test]
    fn too_short_returns_err() {
        assert!(matches!(
            AdsRefactorInfo::try_from_slice(&[0u8; 4]).unwrap_err(),
            AdsTypeInfoError::TooShort {
                expected: 7,
                got: 4
            }
        ));
    }

    #[test]
    fn name_longer_than_slice_returns_err() {
        let mut bytes = vec![0u8; 7];
        // name_length = 100 but only 7 bytes total
        bytes[4] = 100;
        bytes[5] = 0;
        assert!(matches!(
            AdsRefactorInfo::try_from_slice(&bytes).unwrap_err(),
            AdsTypeInfoError::TooShort { .. }
        ));
    }

    #[test]
    fn empty_slice_returns_err() {
        assert!(AdsRefactorInfo::try_from_slice(&[]).is_err());
    }
}
