use super::{IndexGroup, IndexOffset, SumUpError};

/// A request to simultaneously write and read a variable in a single PLC cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SumReadWriteRequest<'a> {
    index_group: IndexGroup,
    index_offset: IndexOffset,
    read_length: u32,
    write_data: &'a [u8],
}

impl<'a> SumReadWriteRequest<'a> {
    /// The fixed byte length of the header for this request.
    pub const HEADER_LENGTH: usize = 16;

    pub fn new(
        index_group: IndexGroup,
        index_offset: IndexOffset,
        read_length: u32,
        write_data: &'a [u8],
    ) -> Self {
        Self {
            index_group,
            index_offset,
            read_length,
            write_data,
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

    /// The expected maximum length of the data to read back.
    pub fn read_length(&self) -> u32 {
        self.read_length
    }

    /// The raw byte data to write.
    pub fn write_data(&self) -> &'a [u8] {
        self.write_data
    }

    /// Serializes the 16-byte header (IndexGroup, IndexOffset, ReadLength, WriteLength).
    pub fn header_to_bytes(&self) -> [u8; 16] {
        let mut buf = [0; Self::HEADER_LENGTH];
        buf[0..4].copy_from_slice(&self.index_group.to_le_bytes());
        buf[4..8].copy_from_slice(&self.index_offset.to_le_bytes());
        buf[8..12].copy_from_slice(&self.read_length.to_le_bytes());
        buf[12..16].copy_from_slice(&(self.write_data.len() as u32).to_le_bytes());
        buf
    }

    pub fn write_header_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.header_to_bytes());
    }

    pub fn try_from_slice(bytes: &'a [u8]) -> Result<Self, SumUpError> {
        if bytes.len() < Self::HEADER_LENGTH {
            return Err(SumUpError::HeaderTooShort {
                expected: Self::HEADER_LENGTH,
                got: bytes.len(),
            });
        }

        let index_group = IndexGroup::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let index_offset = IndexOffset::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let read_length = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let write_len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;

        if bytes.len() < Self::HEADER_LENGTH + write_len {
            return Err(SumUpError::TooShort {
                expected: Self::HEADER_LENGTH + write_len,
                got: bytes.len(),
            });
        }

        Ok(Self {
            index_group,
            index_offset,
            read_length,
            write_data: &bytes[Self::HEADER_LENGTH..Self::HEADER_LENGTH + write_len],
        })
    }
}
