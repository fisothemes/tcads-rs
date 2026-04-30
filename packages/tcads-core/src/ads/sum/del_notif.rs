use super::SumError;
use crate::AdsReturnCode;
use std::fmt;

/// A zero-copy, lazy-evaluating wrapper for an ADS Sum Delete Notification response.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SumDeleteNotificationResponse {
    buffer: Vec<u8>,
}

impl SumDeleteNotificationResponse {
    /// Creates a new [`SumDeleteNotificationResponse`] from a raw buffer.
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

    /// Creates a new [`SumDeleteNotificationResponse`] with no data.
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
    pub fn iter(&self) -> SumDeleteNotificationIter<'_> {
        SumDeleteNotificationIter {
            response: self,
            cursor: 0,
        }
    }
}

impl fmt::Debug for SumDeleteNotificationResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a> IntoIterator for &'a SumDeleteNotificationResponse {
    type Item = AdsReturnCode;
    type IntoIter = SumDeleteNotificationIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An immutable iterator over a [`SumDeleteNotificationResponse`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SumDeleteNotificationIter<'a> {
    response: &'a SumDeleteNotificationResponse,
    cursor: usize,
}

impl<'a> SumDeleteNotificationIter<'a> {
    /// Creates a new [`SumDeleteNotificationIter`] from a [`SumDeleteNotificationResponse`].
    pub fn new(response: &'a SumDeleteNotificationResponse) -> Self {
        Self {
            response,
            cursor: 0,
        }
    }

    /// Sets the cursor position.
    pub fn with_cursor(mut self, cursor: usize) -> Self {
        self.cursor = cursor;
        self
    }

    /// Returns the current cursor position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }
}

impl<'a> Iterator for SumDeleteNotificationIter<'a> {
    type Item = AdsReturnCode;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.response.get(self.cursor);
        self.cursor += 1;
        value
    }
}
