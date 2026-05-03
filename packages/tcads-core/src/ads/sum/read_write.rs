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

/// A zero-copy wrapper for an ADS Sum Read/Write response.
pub struct SumReadWriteResponse<'a, 'b> {
    buffer: Vec<u8>,
    requests: &'a [SumReadWriteRequest<'b>],
}

impl<'a, 'b> SumReadWriteResponse<'a, 'b> {
    /// Creates a new [`SumReadWriteResponse`] from a raw buffer and a slice of requests.
    pub fn new(buffer: Vec<u8>, requests: &'a [SumReadWriteRequest<'b>]) -> Self {
        Self { buffer, requests }
    }

    /// Iterates over the batch results, parsing the network buffer lazily
    /// and yielding zero-copy slices of the valid data.
    pub fn iter(&self) -> impl Iterator<Item = Result<&[u8], AdsReturnCode>> + '_ {
        self.as_view().into_iter()
    }

    /// Returns a purely borrowed view of the response.
    pub fn as_view(&self) -> SumReadWriteView<'_, 'b> {
        SumReadWriteView::new(&self.buffer, self.requests)
    }

    /// Returns the slice of requests that were part of this response.
    pub fn requests(&self) -> &'a [SumReadWriteRequest<'b>] {
        self.requests
    }

    /// Returns a reference to the raw underlying network buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Consumes the response and returns the raw underlying network buffer.   
    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }

    /// Consumes the response and returns the raw underlying network buffer and the requests.
    pub fn into_parts(self) -> (Vec<u8>, &'a [SumReadWriteRequest<'b>]) {
        (self.buffer, self.requests)
    }

    /// Converts this partially borrowed response into a fully owned response.
    pub fn into_owned(self) -> SumReadWriteResponseOwned {
        let requests_owned = self
            .requests
            .iter()
            .map(|req| req.clone().into_owned())
            .collect();
        SumReadWriteResponseOwned::new(self.buffer, requests_owned)
    }
}

/// A zero-copy wrapper for an ADS Sum Read/Write response that fully owns its buffer and requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SumReadWriteResponseOwned {
    buffer: Vec<u8>,
    requests: Vec<SumReadWriteRequestOwned>,
}

impl SumReadWriteResponseOwned {
    /// Creates a new [`SumReadWriteResponseOwned`] from a raw buffer and a slice of requests.
    pub fn new(buffer: Vec<u8>, requests: Vec<SumReadWriteRequestOwned>) -> Self {
        Self { buffer, requests }
    }

    /// Iterates over the batch results, parsing the network buffer lazily
    /// and yielding zero-copy slices of the valid data.
    pub fn iter(&self) -> impl Iterator<Item = Result<&[u8], AdsReturnCode>> + '_ {
        self.as_view().into_iter()
    }

    /// Returns a purely borrowed view of the response.
    pub fn as_view(&self) -> SumReadWriteViewOwned<'_> {
        SumReadWriteViewOwned::new(&self.buffer, &self.requests)
    }

    /// Returns the slice of the request that are part of the response.
    pub fn requests(&self) -> &[SumReadWriteRequestOwned] {
        &self.requests
    }

    /// Returns a reference to the underlying network buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Consumes the response and returns the raw underlying network buffer and the requests.
    pub fn into_parts(self) -> (Vec<u8>, Vec<SumReadWriteRequestOwned>) {
        (self.buffer, self.requests)
    }
}

/// A borrowed view of an ADS Sum Read/Write response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SumReadWriteView<'a, 'b> {
    buffer: &'a [u8],
    requests: &'a [SumReadWriteRequest<'b>],
}

impl<'a, 'b> SumReadWriteView<'a, 'b> {
    /// Creates a new [`SumReadWriteView`] from a raw buffer and a slice of requests.
    pub fn new(buffer: &'a [u8], requests: &'a [SumReadWriteRequest<'b>]) -> Self {
        Self { buffer, requests }
    }

    /// Takes the view by value and lazily parses the network buffer,
    /// yielding the error code and a zero-copy slice of the read data.
    pub fn into_iter(self) -> impl Iterator<Item = Result<&'a [u8], AdsReturnCode>> {
        let n = self.requests.len();
        let mut current_idx = 0;

        let mut data_offset = n * 4;

        let buffer = self.buffer;
        let requests = self.requests;

        std::iter::from_fn(move || {
            if current_idx >= n {
                return None;
            }

            let req = &requests[current_idx];
            let chunk_len = req.read_length() as usize;

            let err_offset = current_idx * 4;
            let err_code = AdsReturnCode::from_bytes([
                buffer[err_offset],
                buffer[err_offset + 1],
                buffer[err_offset + 2],
                buffer[err_offset + 3],
            ]);

            let chunk = &buffer[data_offset..data_offset + chunk_len];

            data_offset += chunk_len;
            current_idx += 1;

            match err_code {
                AdsReturnCode::Ok => Some(Ok(chunk)),
                _ => Some(Err(err_code)),
            }
        })
    }
}

/// A borrowed view of an ADS Sum Read/Write owned response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SumReadWriteViewOwned<'a> {
    buffer: &'a [u8],
    requests: &'a [SumReadWriteRequestOwned],
}

impl<'a> SumReadWriteViewOwned<'a> {
    /// Creates a new [`SumReadWriteViewOwned`] from a raw buffer and a slice of requests.
    pub fn new(buffer: &'a [u8], requests: &'a [SumReadWriteRequestOwned]) -> Self {
        Self { buffer, requests }
    }

    /// Takes the view by value and lazily parses the network buffer,
    /// yielding the error code and a zero-copy slice of the read data.
    pub fn into_iter(self) -> impl Iterator<Item = Result<&'a [u8], AdsReturnCode>> {
        let n = self.requests.len();
        let mut current_idx = 0;

        let mut data_offset = n * 4;

        let buffer = self.buffer;
        let requests = self.requests;

        std::iter::from_fn(move || {
            if current_idx >= n {
                return None;
            }

            let req = &requests[current_idx];
            let chunk_len = req.read_length() as usize;

            let err_offset = current_idx * 4;
            let err_code = AdsReturnCode::from_bytes([
                buffer[err_offset],
                buffer[err_offset + 1],
                buffer[err_offset + 2],
                buffer[err_offset + 3],
            ]);

            let chunk = &buffer[data_offset..data_offset + chunk_len];

            data_offset += chunk_len;
            current_idx += 1;

            match err_code {
                AdsReturnCode::Ok => Some(Ok(chunk)),
                _ => Some(Err(err_code)),
            }
        })
    }
}
