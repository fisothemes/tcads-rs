use crate::errors::AdsStringError;
use encoding_rs::WINDOWS_1252;
use std::borrow::Cow;
use std::fmt;

/// A fixed-length ADS string.
///
/// Maps to `STRING(N-1)` in the PLC.
/// Example: `STRING(80)` requires `AdsString<81>`.
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

    /// Decodes the string content into a Rust UTF-8 string (lossy).
    ///
    /// This handles the CP1252 -> UTF-8 conversion.
    pub fn as_str(&self) -> Cow<'_, str> {
        let (cow, _, _) = WINDOWS_1252.decode(self.as_bytes_until_nul());
        cow
    }

    /// Returns the length of the string (excluding null terminator).
    pub fn len(&self) -> usize {
        self.as_bytes_until_nul().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub const fn capacity(&self) -> usize {
        N
    }
}

impl<const N: usize> From<[u8; N]> for AdsString<N> {
    fn from(bytes: [u8; N]) -> Self {
        Self(bytes)
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
        write!(f, "AdsString<{}>({:?})", N, self.as_str())
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
}
