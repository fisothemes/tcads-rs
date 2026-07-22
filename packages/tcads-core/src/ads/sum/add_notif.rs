use super::{AdsNotificationAttrib, IndexGroup, IndexOffset, SumError};
use crate::{AdsReturnCode, NotificationHandle};
use std::fmt;

/// A request to subscribe to a variable's changes via a batch Sum Command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SumAddNotificationRequest {
    index_group: IndexGroup,
    index_offset: IndexOffset,
    attributes: AdsNotificationAttrib,
}

impl SumAddNotificationRequest {
    /// The fixed byte length of this request (4 + 4 + 16 bytes).
    pub const LENGTH: usize = 40;

    /// Creates a new instance of a [`SumAddNotificationRequest`].
    pub const fn new(
        index_group: IndexGroup,
        index_offset: IndexOffset,
        attributes: AdsNotificationAttrib,
    ) -> Self {
        Self {
            index_group,
            index_offset,
            attributes,
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

    /// The transmission mode and cycle times for the notification.
    pub const fn attributes(&self) -> &AdsNotificationAttrib {
        &self.attributes
    }

    /// Writes this request to a byte buffer.
    pub fn write_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_bytes());
    }

    /// Reads a [`SumAddNotificationRequest`] from a byte buffer.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self {
            index_group: IndexGroup::from_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            index_offset: IndexOffset::from_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            attributes: AdsNotificationAttrib::from_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15], bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21],
                bytes[22], bytes[23],
            ]),
        }
    }

    /// Converts this request to a byte buffer.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.index_group.to_bytes());
        buf[4..8].copy_from_slice(&self.index_offset.to_bytes());
        buf[8..24].copy_from_slice(&self.attributes.to_bytes());
        buf
    }

    /// Parses a slice of bytes into a [`SumAddNotificationRequest`].
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

impl From<SumAddNotificationRequest> for [u8; SumAddNotificationRequest::LENGTH] {
    fn from(req: SumAddNotificationRequest) -> Self {
        req.to_bytes()
    }
}

impl From<[u8; SumAddNotificationRequest::LENGTH]> for SumAddNotificationRequest {
    fn from(bytes: [u8; SumAddNotificationRequest::LENGTH]) -> Self {
        SumAddNotificationRequest::from_bytes(bytes)
    }
}

impl TryFrom<&[u8]> for SumAddNotificationRequest {
    type Error = SumError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        SumAddNotificationRequest::try_from_slice(bytes)
    }
}

/// A zero-copy, lazy-evaluating wrapper for an ADS Sum Add Notification response.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SumAddNotificationResponse {
    buffer: Vec<u8>,
}

impl SumAddNotificationResponse {
    /// 4 bytes for AdsReturnCode + 4 bytes for NotificationHandle
    pub const ITEM_LENGTH: usize = 8;

    /// Creates a new [`SumAddNotificationResponse`] from a raw buffer.
    /// Returns [`Err`] if the buffer is not a multiple of [`Self::ITEM_LENGTH`].
    pub fn new(buf: impl Into<Vec<u8>>) -> Result<Self, SumError> {
        let buffer = buf.into();

        if !buffer.len().is_multiple_of(Self::ITEM_LENGTH) {
            return Err(SumError::UnexpectedLength {
                expected: buffer.len() + (Self::ITEM_LENGTH - (buffer.len() % Self::ITEM_LENGTH)),
                got: buffer.len(),
            });
        }

        Ok(Self { buffer })
    }

    /// Creates a new [`SumAddNotificationResponse`] with no data.
    pub fn empty() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Returns a reference to the raw underlying network buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Returns the number of return codes in the response.
    pub fn len(&self) -> usize {
        self.buffer.len() / Self::ITEM_LENGTH
    }

    /// Returns `true` if the response is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Returns the return code at the given index.
    pub fn get(&self, index: usize) -> Option<Result<NotificationHandle, AdsReturnCode>> {
        if index >= self.len() {
            return None;
        }

        let offset = index * Self::ITEM_LENGTH;
        let chunk = &self.buffer[offset..offset + Self::ITEM_LENGTH];

        let code = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let handle_val = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);

        match AdsReturnCode::from(code) {
            AdsReturnCode::Ok => Some(Ok(NotificationHandle::from(handle_val))),
            err => Some(Err(err)),
        }
    }

    pub fn iter(&self) -> SumAddNotificationIter<'_> {
        SumAddNotificationIter {
            response: self,
            cursor: 0,
        }
    }
}

impl fmt::Debug for SumAddNotificationResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a> IntoIterator for &'a SumAddNotificationResponse {
    type Item = Result<NotificationHandle, AdsReturnCode>;
    type IntoIter = SumAddNotificationIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SumAddNotificationIter<'a> {
    response: &'a SumAddNotificationResponse,
    cursor: usize,
}

impl<'a> Iterator for SumAddNotificationIter<'a> {
    type Item = Result<NotificationHandle, AdsReturnCode>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.response.get(self.cursor);
        if value.is_some() {
            self.cursor += 1;
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AdsTransMode;

    #[test]
    fn test_add_notif_request_padding() {
        let req = SumAddNotificationRequest::new(
            0xF005.into(),
            1234.into(),
            AdsNotificationAttrib::new(4, AdsTransMode::ServerCycle, 0, 100_0000),
        );

        let bytes = req.to_bytes();

        // CRITICAL: Must be exactly 40 bytes to prevent DeviceInvalidSize
        assert_eq!(bytes.len(), 40);
        assert_eq!(SumAddNotificationRequest::LENGTH, 40);

        // Verify the 16 bytes of reserved padding at the end are all zeros
        for &byte in &bytes[24..40] {
            assert_eq!(byte, 0, "Reserved padding bytes must be 0");
        }
    }

    #[test]
    fn test_add_notif_response_parsing() {
        // 8 bytes per item: [Error Code (4)] + [Handle (4)]
        let buffer = vec![
            // Item 1: NoError (0), Handle (42)
            0x00, 0x00, 0x00, 0x00, 0x2A, 0x00, 0x00, 0x00,
            // Item 2: AdsErrDeviceSymbolNotFound (1808 = 0x0710), Handle (0)
            0x10, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let resp = SumAddNotificationResponse::new(buffer).unwrap();
        let mut iter = resp.iter();

        assert_eq!(iter.next(), Some(Ok(NotificationHandle::from(42))));
        assert_eq!(
            iter.next(),
            Some(Err(AdsReturnCode::AdsErrDeviceSymbolNotFound))
        );
        assert_eq!(iter.next(), None);
    }
}
