use super::{IndexGroup, IndexOffset, SumError};
use crate::AdsReturnCode;
use std::fmt;

/// A request to write data to a variable as part of a batch Sum Command.
///
/// Uses a zero-copy slice reference (`&'a [u8]`) to avoid heap allocations
/// when sending fast, cyclic data payloads.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumWriteRequest<'a> {
    index_group: IndexGroup,
    index_offset: IndexOffset,
    data: &'a [u8],
}

impl<'a> SumWriteRequest<'a> {
    /// The fixed byte length of the header for this request.
    pub const HEADER_LENGTH: usize = 12;

    /// Creates a new instance of [`SumWriteRequest`] with the given parameters.
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

    /// Parses a slice of bytes into [`SumWriteRequest`].
    pub fn try_from_slice(bytes: &'a [u8]) -> Result<Self, SumError> {
        if bytes.len() < Self::HEADER_LENGTH {
            return Err(SumError::HeaderTooShort {
                expected: Self::HEADER_LENGTH,
                got: bytes.len(),
            });
        }

        let index_group = IndexGroup::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let index_offset = IndexOffset::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let data_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;

        if bytes.len() < Self::HEADER_LENGTH + data_len {
            return Err(SumError::PayloadTooShort {
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

/// A zero-copy, lazy-evaluating wrapper for an ADS Sum Write response.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SumWriteResponse {
    buffer: Vec<u8>,
}

impl SumWriteResponse {
    /// Creates a new [`SumWriteResponse`] from a raw buffer.
    /// Returns [`Err`] if the buffer is not a multiple of [`AdsReturnCode::LENGTH`].
    pub fn new(buf: impl Into<Vec<u8>>) -> Result<Self, SumError> {
        let buffer = buf.into();

        if !buffer.len().is_multiple_of(AdsReturnCode::LENGTH) {
            return Err(SumError::UnexpectedLength {
                expected: buffer.len()
                    + (AdsReturnCode::LENGTH - (buffer.len() % AdsReturnCode::LENGTH)),
                got: buffer.len(),
            });
        }

        Ok(Self { buffer })
    }

    /// Creates a new [`SumWriteResponse`] with no data.
    pub fn empty() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Returns a reference to the raw underlying network buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Returns the number of return codes in the response.
    pub fn len(&self) -> usize {
        self.buffer.len() / AdsReturnCode::LENGTH
    }

    /// Returns `true` if the response is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Returns the return code at the given index.
    pub fn get(&self, index: usize) -> Option<AdsReturnCode> {
        if index >= self.len() {
            return None;
        }

        let offset = index * AdsReturnCode::LENGTH;
        let code = u32::from_le_bytes(self.buffer[offset..offset + 4].try_into().unwrap());
        Some(AdsReturnCode::from(code))
    }

    /// Returns an iterator over the return codes.
    pub fn iter(&self) -> SumWriteIter<'_> {
        SumWriteIter {
            response: self,
            cursor: 0,
        }
    }
}

impl fmt::Debug for SumWriteResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a> IntoIterator for &'a SumWriteResponse {
    type Item = AdsReturnCode;
    type IntoIter = SumWriteIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An immutable iterator over a `SumWriteResponse`.
pub struct SumWriteIter<'a> {
    response: &'a SumWriteResponse,
    cursor: usize,
}

impl<'a> Iterator for SumWriteIter<'a> {
    type Item = AdsReturnCode;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.response.get(self.cursor);
        self.cursor += 1;
        value
    }
}
