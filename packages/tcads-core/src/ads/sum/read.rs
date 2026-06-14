use super::{IndexGroup, IndexOffset, SumError};
use crate::AdsReturnCode;

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
    pub const fn new(index_group: IndexGroup, index_offset: IndexOffset, length: u32) -> Self {
        Self {
            index_group,
            index_offset,
            length,
        }
    }

    /// The Index Group of the target variable.
    pub const fn index_group(&self) -> IndexGroup {
        self.index_group
    }

    /// The Index Offset of the target variable.
    pub const fn index_offset(&self) -> IndexOffset {
        self.index_offset
    }

    /// The expected maximum length of the data to read in bytes.
    pub const fn length(&self) -> u32 {
        self.length
    }

    /// Writes this request to a byte buffer.
    pub fn write_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_bytes());
    }

    /// Reads a [`SumReadRequest`] from a byte buffer.
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self {
            index_group: IndexGroup::from_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            index_offset: IndexOffset::from_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            length: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        }
    }

    /// Converts this request to a byte buffer.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.index_group.to_bytes());
        buf[4..8].copy_from_slice(&self.index_offset.to_bytes());
        buf[8..12].copy_from_slice(&self.length.to_le_bytes());
        buf
    }

    /// Parses a slice of bytes into a [`SumReadRequest`].
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, SumError> {
        if bytes.len() != Self::LENGTH {
            return Err(SumError::UnexpectedLength {
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
    type Error = SumError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        SumReadRequest::try_from_slice(bytes)
    }
}

/// A zero-copy wrapper for an ADS Sum Read response.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadResponse<'a> {
    buffer: &'a [u8],
    request_count: usize,
}

impl<'a> SumReadResponse<'a> {
    /// Creates a new [`SumReadResponse`] from a raw buffer and a slice of requests.
    pub const fn new(buffer: &'a [u8], requests: &'a [SumReadRequest]) -> Self {
        Self {
            buffer,
            request_count: requests.len(),
        }
    }

    /// Iterates over the batch results, parsing the network buffer lazily
    /// and yielding zero-copy slices of the valid data.
    pub fn iter(&self) -> impl Iterator<Item = Result<&'a [u8], AdsReturnCode>> {
        SumReadResponseIter::new(self.buffer, self.request_count)
    }

    /// Returns a reference to the raw underlying network buffer.
    pub const fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Returns the number of requests that were part of this response.
    pub const fn request_count(&self) -> usize {
        self.request_count
    }

    /// Converts this partially borrowed response into a fully owned response
    /// by cloning the request slice.
    pub fn into_owned(self) -> SumReadResponseOwned {
        SumReadResponseOwned {
            buffer: self.buffer.into(),
            request_count: self.request_count,
        }
    }

    /// Consumes the response and returns the raw underlying network buffer.
    pub fn into_vec(self) -> Vec<u8> {
        self.buffer.into()
    }
}

impl<'a> From<SumReadResponse<'a>> for Vec<u8> {
    fn from(response: SumReadResponse<'a>) -> Self {
        response.into_vec()
    }
}

impl<'a> From<&'a SumReadResponseOwned> for SumReadResponse<'a> {
    fn from(response: &'a SumReadResponseOwned) -> Self {
        response.as_borrowed()
    }
}

impl<'a> From<SumReadResponse<'a>> for SumReadResponseOwned {
    fn from(response: SumReadResponse<'a>) -> Self {
        response.into_owned()
    }
}

/// A zero-copy wrapper for an ADS Sum Read response that fully owns
/// its request metadata and network buffer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadResponseOwned {
    buffer: Vec<u8>,
    request_count: usize,
}

impl SumReadResponseOwned {
    /// Creates a new [`SumReadResponseOwned`] from a raw buffer and a slice of requests.
    pub const fn new(buffer: Vec<u8>, requests: &[SumReadRequest]) -> Self {
        Self {
            buffer,
            request_count: requests.len(),
        }
    }

    /// Iterates over the batch results, parsing the network buffer lazily
    /// and yielding zero-copy slices of the valid data.
    pub fn iter(&self) -> impl Iterator<Item = Result<&[u8], AdsReturnCode>> {
        SumReadResponseIter::new(&self.buffer, self.request_count)
    }

    /// Returns a reference to the raw underlying network buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Returns the number of requests that were part of this response.
    pub const fn request_count(&self) -> usize {
        self.request_count
    }

    /// Consumes the response and returns the raw underlying network buffer.
    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }

    /// Returns a borrowed view of the response.
    pub fn as_borrowed(&self) -> SumReadResponse<'_> {
        SumReadResponse {
            buffer: &self.buffer,
            request_count: self.request_count,
        }
    }
}

impl From<SumReadResponseOwned> for Vec<u8> {
    fn from(response: SumReadResponseOwned) -> Self {
        response.into_vec()
    }
}

impl<'a> IntoIterator for &'a SumReadResponseOwned {
    type Item = Result<&'a [u8], AdsReturnCode>;
    type IntoIter = SumReadResponseIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SumReadResponseIter::new(&self.buffer, self.request_count)
    }
}

/// An iterator over the results of an ADS Sum Read response.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadResponseIter<'a> {
    buffer: &'a [u8],
    request_count: usize,
    data_offset: usize,
    current_idx: usize,
}

impl<'a> SumReadResponseIter<'a> {
    /// Creates a new [`SumReadResponseIter`] from a raw buffer and a request count.
    pub fn new(buffer: &'a [u8], request_count: usize) -> Self {
        Self {
            buffer,
            request_count,
            data_offset: request_count * 8,
            current_idx: 0,
        }
    }
}

impl<'a> Iterator for SumReadResponseIter<'a> {
    type Item = Result<&'a [u8], AdsReturnCode>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_idx >= self.request_count {
            return None;
        }

        let header = self.current_idx * 8;
        let err_code = AdsReturnCode::from_bytes([
            self.buffer[header],
            self.buffer[header + 1],
            self.buffer[header + 2],
            self.buffer[header + 3],
        ]);
        let returned_len = u32::from_le_bytes([
            self.buffer[header + 4],
            self.buffer[header + 5],
            self.buffer[header + 6],
            self.buffer[header + 7],
        ]) as usize;

        let chunk = &self.buffer[self.data_offset..self.data_offset + returned_len];
        self.data_offset += returned_len;
        self.current_idx += 1;

        match err_code {
            AdsReturnCode::Ok => Some(Ok(chunk)),
            _ => Some(Err(err_code)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_request_layout() {
        let req = SumReadRequest::new(0x4020.into(), 0.into(), 4);
        assert_eq!(req.to_bytes().len(), 12);
        assert_eq!(SumReadRequest::LENGTH, 12);
    }

    #[test]
    fn test_read_response_misalignment_prevention() {
        let mut buffer = Vec::new();
        // Item 1 Header: Failed (Error 1795), Returned Length: 0
        buffer.extend_from_slice(&[0x03, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        // Item 2 Header: Success (0), Returned Length: 4
        buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00]);

        // Data payload (Only 4 bytes total because Item 1 returned 0 bytes)
        buffer.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);

        let reqs = vec![
            SumReadRequest::new(0x4020.into(), 0.into(), 4),
            SumReadRequest::new(0x4020.into(), 4.into(), 4),
        ];

        let response = SumReadResponse::new(&buffer, &reqs);
        let mut iter = response.iter();

        assert_eq!(
            iter.next(),
            Some(Err(AdsReturnCode::AdsErrDeviceInvalidOffset))
        );
        assert_eq!(iter.next(), Some(Ok([0xAA, 0xBB, 0xCC, 0xDD].as_slice())));
        assert_eq!(iter.next(), None);
    }
}
