use std::array::TryFromSliceError;
use std::fmt;

/// A handle identifying a remote file opened via the TwinCAT System Service.
///
/// # Wire Format
/// 4 bytes, little-endian `u32`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AdsFileHandle([u8; Self::LENGTH]);

impl AdsFileHandle {
    /// The length of a [`AdsFileHandle`] on the wire.
    pub const LENGTH: usize = 4;

    pub fn new(handle: u32) -> Self {
        Self(handle.to_le_bytes())
    }

    /// Creates a [`AdsFileHandle`] from a 4-byte array (little-endian).
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(bytes)
    }

    /// Tries to parse an [`AdsFileHandle`] from a byte slice.
    ///
    /// Returns an error if the slice is shorter than 4 bytes.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, TryFromSliceError> {
        bytes.try_into()
    }

    /// Returns the raw 4-byte little-endian representation.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.0
    }

    /// Returns the handle value as a `u32`.
    pub fn as_u32(&self) -> u32 {
        u32::from_le_bytes(self.0)
    }

    /// Returns the handle value as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<u32> for AdsFileHandle {
    fn from(value: u32) -> Self {
        Self(value.to_le_bytes())
    }
}

impl From<AdsFileHandle> for u32 {
    fn from(value: AdsFileHandle) -> Self {
        u32::from_le_bytes(value.0)
    }
}

impl From<[u8; AdsFileHandle::LENGTH]> for AdsFileHandle {
    fn from(bytes: [u8; AdsFileHandle::LENGTH]) -> Self {
        Self(bytes)
    }
}

impl From<AdsFileHandle> for [u8; AdsFileHandle::LENGTH] {
    fn from(handle: AdsFileHandle) -> Self {
        handle.0
    }
}

impl TryFrom<&[u8]> for AdsFileHandle {
    type Error = TryFromSliceError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; AdsFileHandle::LENGTH] = bytes.try_into()?;
        Ok(Self::from_bytes(bytes))
    }
}

impl fmt::Debug for AdsFileHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileHandle(0x{:08X})", self.as_u32())
    }
}

impl serde::Serialize for AdsFileHandle {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u32(self.as_u32())
    }
}

impl<'de> serde::Deserialize<'de> for AdsFileHandle {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(AdsFileHandle::from(u32::deserialize(d)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_u32_roundtrip() {
        let handle = AdsFileHandle::from(0x0000_001A_u32);
        assert_eq!(handle.as_u32(), 0x0000_001A);
        assert_eq!(u32::from(handle), 0x0000_001A);
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        let bytes = [0x1A, 0x00, 0x00, 0x00];
        let handle = AdsFileHandle::from_bytes(bytes);
        assert_eq!(handle.to_bytes(), bytes);
        assert_eq!(handle.as_u32(), 0x0000_001A);
    }

    #[test]
    fn test_try_from_slice_valid() {
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let handle = AdsFileHandle::try_from_slice(&bytes).unwrap();
        assert_eq!(handle.as_u32(), 0x04030201);
    }

    #[test]
    fn test_equality_and_hash() {
        use std::collections::HashMap;

        let h1 = AdsFileHandle::from(42_u32);
        let h2 = AdsFileHandle::from(42_u32);
        let h3 = AdsFileHandle::from(99_u32);

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);

        let mut map = HashMap::new();
        map.insert(h1, "handler_a");
        assert_eq!(map[&h2], "handler_a");
        assert!(!map.contains_key(&h3));
    }

    #[test]
    fn test_serde_file_handle_serialize() {
        let handle = AdsFileHandle::from(42_u32);
        let s = serde_json::to_string(&handle).unwrap();
        assert_eq!(s, "42");
    }

    #[test]
    fn test_serde_file_handle_deserialize() {
        let handle: AdsFileHandle = serde_json::from_str("42").unwrap();
        assert_eq!(handle, AdsFileHandle::from(42_u32));
    }

    #[test]
    fn test_serde_file_handle_roundtrip() {
        let original = AdsFileHandle::from(0x0000_001A_u32);
        let s = serde_json::to_string(&original).unwrap();
        let back: AdsFileHandle = serde_json::from_str(&s).unwrap();
        assert_eq!(original, back);
    }
}
