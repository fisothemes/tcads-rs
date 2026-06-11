use super::{IndexGroup, IndexOffset, SumError};
use crate::AdsReturnCode;

/// A request to simultaneously write and read a variable in a single PLC cycle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// Consumes the request and returns a fully owned request.
    pub fn into_owned(self) -> SumReadWriteRequestOwned {
        SumReadWriteRequestOwned::new(
            self.index_group,
            self.index_offset,
            self.read_length,
            self.write_data,
        )
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

    /// Serializes the 16-byte header (IndexGroup, IndexOffset, ReadLength, WriteLength)
    /// to a byte buffer.
    pub fn write_header_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.header_to_bytes());
    }

    /// Parses a slice and tries to deserialize it as a Sum Read/Write request.
    pub fn try_from_slice(bytes: &'a [u8]) -> Result<Self, SumError> {
        if bytes.len() < Self::HEADER_LENGTH {
            return Err(SumError::HeaderTooShort {
                expected: Self::HEADER_LENGTH,
                got: bytes.len(),
            });
        }

        let index_group = IndexGroup::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let index_offset = IndexOffset::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let read_length = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let write_len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;

        if bytes.len() < Self::HEADER_LENGTH + write_len {
            return Err(SumError::TooShort {
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

/// A fully owned request to simultaneously write and read a variable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadWriteRequestOwned {
    index_group: IndexGroup,
    index_offset: IndexOffset,
    read_length: u32,
    write_data: Vec<u8>,
}

impl SumReadWriteRequestOwned {
    /// Creates a new [`SumReadWriteRequestOwned`] with the given parameters.
    pub fn new(
        index_group: IndexGroup,
        index_offset: IndexOffset,
        read_length: u32,
        write_data: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            index_group,
            index_offset,
            read_length,
            write_data: write_data.into(),
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
    pub fn write_data(&self) -> &[u8] {
        &self.write_data
    }

    /// Returns a purely borrowed view of the request.
    pub fn as_borrowed(&self) -> SumReadWriteRequest<'_> {
        SumReadWriteRequest {
            index_group: self.index_group,
            index_offset: self.index_offset,
            read_length: self.read_length,
            write_data: &self.write_data,
        }
    }
}

/// A wrapper for an ADS Sum Read/Write response.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadWriteResponse<'a> {
    buffer: &'a [u8],
    request_count: usize,
}

impl<'a> SumReadWriteResponse<'a> {
    /// Creates a new [`SumReadWriteResponse`] from a raw buffer and a slice of requests.
    pub fn new(buffer: &'a [u8], requests: &[SumReadWriteRequest<'_>]) -> Self {
        Self {
            buffer,
            request_count: requests.len(),
        }
    }

    /// Iterates over the batch results, parsing the network buffer lazily
    /// and yielding zero-copy slices of the valid data.
    pub fn iter(&self) -> impl Iterator<Item = Result<&'a [u8], AdsReturnCode>> {
        SumReadWriteResponseIter::new(self.buffer, self.request_count)
    }

    /// Returns a reference to the raw underlying network buffer.
    pub fn buffer(&self) -> &[u8] {
        self.buffer
    }

    /// Returns the number of requests that were part of this response.
    pub fn request_count(&self) -> usize {
        self.request_count
    }

    /// Consumes the response and returns the raw underlying network buffer.
    pub fn into_vec(self) -> Vec<u8> {
        self.buffer.into()
    }

    /// Consumes the response and returns a fully owned response.
    pub fn into_owned(self) -> SumReadWriteResponseOwned {
        let request_count = self.request_count();
        SumReadWriteResponseOwned {
            buffer: self.buffer.into(),
            request_count,
        }
    }
}

impl<'a> From<SumReadWriteResponse<'a>> for Vec<u8> {
    fn from(response: SumReadWriteResponse) -> Self {
        response.into_vec()
    }
}

impl<'a> From<&'a SumReadWriteResponseOwned> for SumReadWriteResponse<'a> {
    fn from(response: &'a SumReadWriteResponseOwned) -> Self {
        response.as_borrowed()
    }
}

/// A wrapper for a fully owned ADS Sum Read/Write response.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadWriteResponseOwned {
    buffer: Vec<u8>,
    request_count: usize,
}

impl SumReadWriteResponseOwned {
    /// Creates a new [`SumReadWriteResponseOwned`] from a raw buffer and a slice of requests.
    pub fn new(buffer: Vec<u8>, requests: &[SumReadWriteRequest<'_>]) -> Self {
        Self {
            buffer,
            request_count: requests.len(),
        }
    }

    /// Iterates over the batch results, parsing the network buffer lazily
    /// and yielding zero-copy slices of the valid data.
    pub fn iter(&self) -> impl Iterator<Item = Result<&[u8], AdsReturnCode>> {
        SumReadWriteResponseIter::new(&self.buffer, self.request_count)
    }

    /// Returns a reference to the raw underlying network buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Returns the number of requests that were part of this response.
    pub fn request_count(&self) -> usize {
        self.request_count
    }

    /// Consumes the response and returns the raw underlying network buffer.
    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }

    /// Returns a borrowed view of the response.
    pub fn as_borrowed(&self) -> SumReadWriteResponse<'_> {
        SumReadWriteResponse {
            buffer: &self.buffer,
            request_count: self.request_count,
        }
    }
}

impl From<SumReadWriteResponseOwned> for Vec<u8> {
    fn from(response: SumReadWriteResponseOwned) -> Self {
        response.into_vec()
    }
}

impl<'a> From<SumReadWriteResponse<'a>> for SumReadWriteResponseOwned {
    fn from(response: SumReadWriteResponse) -> Self {
        response.into_owned()
    }
}

impl<'a> IntoIterator for &'a SumReadWriteResponseOwned {
    type Item = Result<&'a [u8], AdsReturnCode>;
    type IntoIter = SumReadWriteResponseIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SumReadWriteResponseIter::new(&self.buffer, self.request_count)
    }
}

/// An iterator over the results of a Sum Read-Write response.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadWriteResponseIter<'a> {
    buffer: &'a [u8],
    request_count: usize,
    data_offset: usize,
    current_idx: usize,
}

impl<'a> SumReadWriteResponseIter<'a> {
    /// Creates a new [`SumReadWriteResponseIter`] from a raw buffer and a request count.
    pub fn new(buffer: &'a [u8], request_count: usize) -> Self {
        Self {
            buffer,
            request_count,
            data_offset: request_count * 8,
            current_idx: 0,
        }
    }
}

impl<'a> Iterator for SumReadWriteResponseIter<'a> {
    type Item = Result<&'a [u8], AdsReturnCode>;

    fn next(&mut self) -> Option<Self::Item> {
        utils::parse_next_response(
            self.buffer,
            self.request_count,
            &mut self.current_idx,
            &mut self.data_offset,
        )
    }
}

pub mod utils {
    use super::*;

    pub fn parse_next_response<'a>(
        buffer: &'a [u8],
        request_count: usize,
        current_idx: &mut usize,
        data_offset: &mut usize,
    ) -> Option<Result<&'a [u8], AdsReturnCode>> {
        if *current_idx >= request_count {
            return None;
        }

        let err_offset = *current_idx * 8;
        let err_code = AdsReturnCode::from_bytes([
            buffer[err_offset],
            buffer[err_offset + 1],
            buffer[err_offset + 2],
            buffer[err_offset + 3],
        ]);

        let returned_len = u32::from_le_bytes([
            buffer[err_offset + 4],
            buffer[err_offset + 5],
            buffer[err_offset + 6],
            buffer[err_offset + 7],
        ]) as usize;

        let chunk = &buffer[*data_offset..*data_offset + returned_len];

        *data_offset += returned_len;
        *current_idx += 1;

        match err_code {
            AdsReturnCode::Ok => Some(Ok(chunk)),
            _ => Some(Err(err_code)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AdsReturnCode;

    #[test]
    fn test_read_write_request_header() {
        let data: &[u8] = b"MAIN.bTest\0";
        let req = SumReadWriteRequest::new(0xF003, 0, 4, data);

        let header = req.header_to_bytes();
        assert_eq!(header.len(), 16);
        assert_eq!(SumReadWriteRequest::HEADER_LENGTH, 16);

        // Verify write length was encoded correctly in the last 4 bytes of the header
        let encoded_write_len = u32::from_le_bytes(header[12..16].try_into().unwrap());
        assert_eq!(encoded_write_len as usize, data.len());
    }

    #[test]
    fn test_read_write_response_misalignment_prevention() {
        let mut buffer = Vec::new();
        // Item 1 Header: AdsErrDeviceSymbolNotFound (1808 = 0x0710), Returned Length: 0
        buffer.extend_from_slice(&[0x10, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        // Item 2 Header: Success (0), Returned Length: 4
        buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00]);

        // Data block
        let handle_bytes = [137, 0, 0, 84];
        buffer.extend_from_slice(&handle_bytes);

        let reqs = vec![
            SumReadWriteRequest::new(0xF003, 0, 4, b"BAD\0"),
            SumReadWriteRequest::new(0xF003, 0, 4, b"GOOD\0"),
        ];

        let response = SumReadWriteResponse::new(&buffer, &reqs);
        let mut iter = response.iter();

        assert_eq!(
            iter.next(),
            Some(Err(AdsReturnCode::AdsErrDeviceSymbolNotFound))
        );
        assert_eq!(iter.next(), Some(Ok(handle_bytes.as_slice())));
        assert_eq!(iter.next(), None);
    }
}
