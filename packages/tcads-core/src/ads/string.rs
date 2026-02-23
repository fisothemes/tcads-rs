use super::error::AdsStringError;
use encoding_rs::WINDOWS_1252;
use std::borrow::Cow;
use std::fmt;
use std::ops::{Index, IndexMut};

/// A fixed-length ADS string.
///
/// Maps to `STRING(K)` in the PLC, where `N = K + 1`.
/// The generic parameter `N` represents the **total byte size** of the buffer,
/// including the null terminator.
///
/// # Example
/// * PLC: `STRING(80)` (holds 80 chars + 1 null)
/// * Rust: `AdsString<81>`
///
/// # Encoding
/// Handles conversion between Rust UTF-8 and PLC Windows-1252 (CP1252) automatically.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdsString<const N: usize>([u8; N]);

impl<const N: usize> AdsString<N> {
    /// Creates a new empty string (all zeros).
    pub const fn new() -> Self {
        Self([0; N])
    }

    /// Returns the raw byte array (Windows-1252 encoded).
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Returns the bytes up to the first null terminator.
    pub fn as_bytes_until_nul(&self) -> &[u8] {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(N);
        &self.0[..end]
    }

    /// Iterates over the raw bytes of the string.
    pub fn bytes(&self) -> std::slice::Iter<'_, u8> {
        self.as_bytes_until_nul().iter()
    }

    /// Decodes the string content into a Rust UTF-8 string (lossy).
    ///
    /// This handles the CP1252 -> UTF-8 conversion.
    /// Returns `Cow::Borrowed` if the content is ASCII, or `Cow::Owned` if decoding was required
    pub fn as_str(&self) -> Cow<'_, str> {
        let (cow, _, _) = WINDOWS_1252.decode(self.as_bytes_until_nul());
        cow
    }

    /// Returns the length of the string (excluding null terminator).
    pub fn len(&self) -> usize {
        self.as_bytes_until_nul().len()
    }

    /// Returns `true` if the string is empty (contains no characters).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total capacity of the string buffer (\[N\]).
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Appends a string slice to the end of this string.
    ///
    /// Returns an error if the string exceeds the capacity or contains invalid characters.
    pub fn push_str(&mut self, s: &str) -> Result<(), AdsStringError> {
        let (encoded, _, has_errors) = WINDOWS_1252.encode(s);
        if has_errors {
            return Err(AdsStringError::EncodingError);
        }

        let current_len = self.len();
        let available = N - 1 - current_len;

        if encoded.len() > available {
            return Err(AdsStringError::TooLong {
                expected: available,
                got: encoded.len(),
            });
        }

        self.0[current_len..current_len + encoded.len()].copy_from_slice(&encoded);
        self.0[current_len + encoded.len()] = 0;

        Ok(())
    }

    /// Appends a single character to the end of this string.
    pub fn push(&mut self, c: char) -> Result<(), AdsStringError> {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        self.push_str(s)
    }

    /// Truncates the string to `new_len`.
    ///
    /// If `new_len` is greater than the current length, this does nothing.
    pub fn truncate(&mut self, new_len: usize) {
        if new_len < self.len() {
            self.0[new_len] = 0;
        }
    }

    /// Clears the string (sets length to 0).
    pub fn clear(&mut self) {
        self.0[0] = 0;
    }
}

impl<const N: usize> Index<usize> for AdsString<N> {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<const N: usize> IndexMut<usize> for AdsString<N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<'a, const N: usize> IntoIterator for &'a AdsString<N> {
    type Item = &'a u8;
    type IntoIter = std::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.bytes()
    }
}

impl<const N: usize> fmt::Write for AdsString<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s).map_err(|_| fmt::Error)
    }
}

impl<const N: usize> From<[u8; N]> for AdsString<N> {
    fn from(bytes: [u8; N]) -> Self {
        Self(bytes)
    }
}

impl<const N: usize> From<&[u8; N]> for AdsString<N> {
    fn from(bytes: &[u8; N]) -> Self {
        Self(*bytes)
    }
}

impl<const N: usize> TryFrom<&str> for AdsString<N> {
    type Error = AdsStringError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (encoded_bytes, _, has_errors) = WINDOWS_1252.encode(value);

        if has_errors {
            return Err(AdsStringError::EncodingError);
        }

        if encoded_bytes.len() >= N {
            return Err(AdsStringError::TooLong {
                expected: N - 1,
                got: encoded_bytes.len(),
            });
        }

        let mut buf = [0u8; N];
        buf[..encoded_bytes.len()].copy_from_slice(&encoded_bytes);

        Ok(Self(buf))
    }
}

impl<const N: usize> AsRef<[u8]> for AdsString<N> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes_until_nul()
    }
}

impl<const N: usize> fmt::Display for AdsString<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_str())
    }
}

impl<const N: usize> fmt::Debug for AdsString<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_str())
    }
}

impl<const N: usize> Default for AdsString<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fmt::Write;

    #[test]
    fn test_encoding_roundtrip() {
        // "€" is 0x80 in CP1252, but 3 bytes in UTF-8
        let input = "Cost: 5€";
        let s: AdsString<20> = AdsString::try_from(input).expect("Should fit");

        assert_eq!(s.as_str(), input);

        // Verify internal storage is CP1252 (0x80) not UTF-8
        let bytes = s.as_bytes_until_nul();
        assert!(bytes.contains(&0x80));
    }

    #[test]
    fn test_length_limit() {
        // AdsString<5> can hold 4 chars + 1 null
        let s: Result<AdsString<5>, _> = AdsString::try_from("1234");
        assert!(s.is_ok());

        let s: Result<AdsString<5>, _> = AdsString::try_from("12345");
        assert!(matches!(s, Err(AdsStringError::TooLong { .. })));
    }

    #[test]
    fn test_push_and_capacity() {
        let mut s: AdsString<10> = AdsString::new(); // Max 9 chars

        s.push_str("Hello").unwrap();
        assert_eq!(s.len(), 5);
        assert_eq!(s.as_str(), "Hello");

        s.push('!').unwrap();
        assert_eq!(s.as_str(), "Hello!");

        // Try to overflow
        let err = s.push_str("1234"); // 6 + 4 = 10 (Too big, needs 11 bytes for null)
        assert!(err.is_err());
    }

    #[test]
    fn test_indexing() {
        let mut s: AdsString<10> = AdsString::try_from("ABC").unwrap();
        assert_eq!(s[0], 65); // 'A'

        s[0] = 66; // 'B'
        assert_eq!(s.as_str(), "BBC");
    }

    #[test]
    fn test_format_macro() {
        let mut s: AdsString<20> = AdsString::new();
        write!(s, "Val: {}", 42).unwrap();
        assert_eq!(s.as_str(), "Val: 42");
    }

    #[test]
    fn test_iteration() {
        let s: AdsString<10> = AdsString::try_from("Hi").unwrap();
        let bytes: Vec<u8> = s.bytes().cloned().collect();
        assert_eq!(bytes, vec![b'H', b'i']);
    }
}
