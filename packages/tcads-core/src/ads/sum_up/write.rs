use super::{IndexGroup, IndexOffset, SumUpError};

/// A request to write data to a variable as part of a batch Sum Command.
///
/// Uses a zero-copy slice reference (`&'a [u8]`) to avoid heap allocations
/// when sending fast, cyclic data payloads.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumWriteReq<'a> {
    index_group: IndexGroup,
    index_offset: IndexOffset,
    data: &'a [u8],
}

impl<'a> SumWriteReq<'a> {
    /// The fixed byte length of the header for this request.
    pub const HEADER_LENGTH: usize = 12;

    /// Creates a new instance of [`SumWriteReq`] with the given parameters.
    pub fn new(index_group: IndexGroup, index_offset: IndexOffset, data: &'a [u8]) -> Self {
        Self {
            index_group,
            index_offset,
            data,
        }
    }

    /// The Index Group of the target variable.
    pub fn index_group(&self) -> IndexGroup {
        self.index_group
    }

    /// The Index Offset of the target variable.
    pub fn index_offset(&self) -> IndexOffset {
        self.index_offset
    }

    /// The raw byte data to write.
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Serializes the 12-byte header (IndexGroup, IndexOffset, DataLength).
    pub fn header_to_bytes(&self) -> [u8; 12] {
        let mut buf = [0; Self::HEADER_LENGTH];
        buf[0..4].copy_from_slice(&self.index_group.to_le_bytes());
        buf[4..8].copy_from_slice(&self.index_offset.to_le_bytes());
        buf[8..12].copy_from_slice(&(self.data.len() as u32).to_le_bytes());
        buf
    }

    /// Writes the header to a buffer. (the caller must append Data separately
    /// to comply with the Sum Command wire format).
    pub fn write_header_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.header_to_bytes());
    }

    /// Parses a slice of bytes into [`SumWriteReq`].
    pub fn try_from_slice(bytes: &'a [u8]) -> Result<Self, SumUpError> {
        if bytes.len() < Self::HEADER_LENGTH {
            return Err(SumUpError::HeaderTooShort {
                expected: Self::HEADER_LENGTH,
                got: bytes.len(),
            });
        }

        let index_group = IndexGroup::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let index_offset = IndexOffset::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let data_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;

        if bytes.len() < Self::HEADER_LENGTH + data_len {
            return Err(SumUpError::PayloadTooShort {
                expected: Self::HEADER_LENGTH + data_len,
                got: bytes.len(),
            });
        }

        Ok(Self {
            index_group,
            index_offset,
            data: &bytes[Self::HEADER_LENGTH..Self::HEADER_LENGTH + data_len],
        })
    }
}
