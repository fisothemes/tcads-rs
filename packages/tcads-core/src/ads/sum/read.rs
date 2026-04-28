use super::{IndexGroup, IndexOffset, SumUpError};

/// A request to read a variable as part of a batch Sum Command.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadRequest {
    index_group: IndexGroup,
    index_offset: IndexOffset,
    length: u32,
}

impl SumReadRequest {
    /// The fixed byte length of this request in the ADS payload.
    pub const LENGTH: usize = 12;

    /// Creates a new [`SumReadRequest`] with the given parameters.
    pub fn new(index_group: IndexGroup, index_offset: IndexOffset, length: u32) -> Self {
        Self {
            index_group,
            index_offset,
            length,
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

    /// The expected maximum length of the data to read in bytes.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Writes this request to a byte buffer.
    pub fn write_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.index_group.to_le_bytes());
        buf.extend_from_slice(&self.index_offset.to_le_bytes());
        buf.extend_from_slice(&self.length.to_le_bytes());
    }

    /// Reads a [`SumReadRequest`] from a byte buffer.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self {
            index_group: IndexGroup::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            index_offset: IndexOffset::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            length: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        }
    }

    /// Converts this request to a byte buffer.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.index_group.to_le_bytes());
        buf[4..8].copy_from_slice(&self.index_offset.to_le_bytes());
        buf[8..12].copy_from_slice(&self.length.to_le_bytes());
        buf
    }

    /// Parses a slice of bytes into a [`SumReadRequest`].
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, SumUpError> {
        if bytes.len() != Self::LENGTH {
            return Err(SumUpError::UnexpectedLength {
                expected: Self::LENGTH,
                got: bytes.len(),
            });
        }
        let mut buf = [0u8; Self::LENGTH];
        buf.copy_from_slice(bytes);
        Ok(Self::from_bytes(buf))
    }
}

impl From<SumReadRequest> for [u8; SumReadRequest::LENGTH] {
    fn from(req: SumReadRequest) -> Self {
        req.to_bytes()
    }
}

impl From<[u8; SumReadRequest::LENGTH]> for SumReadRequest {
    fn from(bytes: [u8; SumReadRequest::LENGTH]) -> Self {
        SumReadRequest::from_bytes(bytes)
    }
}

impl TryFrom<&[u8]> for SumReadRequest {
    type Error = SumUpError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        SumReadRequest::try_from_slice(bytes)
    }
}

use crate::AdsReturnCode;

/// A zero-copy, lazy-evaluating wrapper for an ADS Sum Read response.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadResponse<'a> {
    buffer: Vec<u8>,
    requests: &'a [SumReadRequest],
}

impl<'a> SumReadResponse<'a> {
    /// Creates a new [`SumReadResponse`] from a raw buffer and a slice of requests.
    pub fn new(buf: impl Into<Vec<u8>>, requests: &'a [SumReadRequest]) -> Self {
        Self {
            buffer: buf.into(),
            requests,
        }
    }

    /// Iterates over the batch results, parsing the network buffer lazily
    /// and yielding zero-copy slices of the valid data.
    pub fn iter(&self) -> impl Iterator<Item = (AdsReturnCode, &[u8])> + '_ {
        self.as_view().into_iter()
    }

    /// Returns a purely borrowed view of the response.
    pub fn as_view(&self) -> SumReadView<'_> {
        SumReadView::new(&self.buffer, self.requests)
    }

    /// Returns the slice of requests that were part of this response.
    pub fn requests(&self) -> &[SumReadRequest] {
        self.requests
    }

    /// Returns a reference to the raw underlying network buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Converts this partially borrowed response into a fully owned response
    /// by cloning the request slice.
    pub fn into_owned(self) -> SumReadResponseOwned {
        SumReadResponseOwned::new(self.buffer, self.requests)
    }

    /// Consumes the response and returns the raw underlying network buffer.
    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }

    /// Consumes the response and returns the raw underlying network buffer and the requests.
    pub fn into_parts(self) -> (Vec<u8>, &'a [SumReadRequest]) {
        (self.buffer, self.requests)
    }
}

/// A zero-copy wrapper for an ADS Sum Read response that fully owns
/// its request metadata and network buffer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadResponseOwned {
    buffer: Vec<u8>,
    requests: Vec<SumReadRequest>,
}

impl SumReadResponseOwned {
    pub fn new(buf: impl Into<Vec<u8>>, requests: impl Into<Vec<SumReadRequest>>) -> Self {
        Self {
            buffer: buf.into(),
            requests: requests.into(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (AdsReturnCode, &[u8])> + '_ {
        self.as_view().into_iter()
    }

    /// Returns a purely borrowed view of the response.
    pub fn as_view(&self) -> SumReadView<'_> {
        SumReadView::new(&self.buffer, &self.requests)
    }

    /// Returns the slice of requests that were part of this response.
    pub fn requests(&self) -> &[SumReadRequest] {
        &self.requests
    }

    /// Returns a reference to the raw underlying network buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Consumes the response and returns the raw underlying network buffer and the requests.
    pub fn into_parts(self) -> (Vec<u8>, Vec<SumReadRequest>) {
        (self.buffer, self.requests)
    }
}

/// A purely borrowed view of an ADS Sum Read response.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadView<'a> {
    buffer: &'a [u8],
    requests: &'a [SumReadRequest],
}

impl<'a> SumReadView<'a> {
    /// Creates a new [`SumReadView`] from a raw buffer and a slice of requests.
    pub fn new(buffer: &'a [u8], requests: &'a [SumReadRequest]) -> Self {
        Self { buffer, requests }
    }

    /// Takes the view by value and returns an iterator yielding zero-copy slices.
    pub fn into_iter(self) -> impl Iterator<Item = (AdsReturnCode, &'a [u8])> {
        let n = self.requests.len();
        let mut current_idx = 0;
        let mut data_offset = n * 8;

        let buffer = self.buffer;
        let requests = self.requests;

        std::iter::from_fn(move || {
            if current_idx >= n {
                return None;
            }

            let req = &requests[current_idx];
            let expected_len = req.length() as usize;
            let header_offset = current_idx * 8;

            let err_code =
                u32::from_le_bytes(buffer[header_offset..header_offset + 4].try_into().unwrap());
            let valid_len = u32::from_le_bytes(
                buffer[header_offset + 4..header_offset + 8]
                    .try_into()
                    .unwrap(),
            ) as usize;

            let safe_len = valid_len.min(expected_len);
            let chunk = &buffer[data_offset..data_offset + safe_len];

            data_offset += expected_len;
            current_idx += 1;

            Some((AdsReturnCode::from(err_code), chunk))
        })
    }
}
