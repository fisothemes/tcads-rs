use super::error::GuidParseError;
use std::fmt;
use std::str::FromStr;

/// A 16-byte Globally Unique Identifier (GUID) as used in TwinCAT ADS type info entries.
///
/// Present in [`AdsDataTypeInfo`](super::AdsDataTypeInfo) when
/// [`AdsDataTypeFlags::TYPE_GUID`](super::AdsDataTypeFlags::TYPE_GUID) is set.
/// The bytes are stored in the order they appear on the wire without any field
/// decomposition. TwinCAT treats GUIDs as opaque identifiers for type versioning.
///
/// # Wire Format
///
/// | Offset | Size | Field   | Description              |
/// |--------|------|---------|--------------------------|
/// | 0      | 16   | `bytes` | Raw GUID bytes, as-is    |
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Guid([u8; 16]);

impl Guid {
    pub const LENGTH: usize = 16;

    /// Creates a new instance of the [GUID`]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Generates a random GUID using the `rand` crate.
    pub fn random() -> Self {
        let bytes: [u8; 16] = rand::random();
        Self::new(bytes)
    }

    /// Returns the inner byte array.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Consumes the `Guid` and returns the inner byte array.
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Parses from a slice.
    pub fn try_from_slice(slice: &[u8]) -> Result<Self, GuidParseError> {
        Guid::try_from(slice)
    }

    /// Returns `true` if the GUID is all zeros.
    pub fn is_nil(&self) -> bool {
        self.0 == [0u8; 16]
    }
}

impl AsRef<[u8]> for Guid {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 16]> for Guid {
    fn from(bytes: [u8; 16]) -> Self {
        Self::new(bytes)
    }
}

impl From<Guid> for [u8; 16] {
    fn from(guid: Guid) -> Self {
        guid.into_bytes()
    }
}

impl TryFrom<&[u8]> for Guid {
    type Error = GuidParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != Self::LENGTH {
            return Err(GuidParseError::InvalidLength(bytes.len()));
        }
        let mut guid = [0u8; 16];
        guid.copy_from_slice(bytes);
        Ok(Self::new(guid))
    }
}

impl fmt::Debug for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, &b) in self.0.iter().enumerate() {
            if i == 4 || i == 6 || i == 8 || i == 10 {
                write!(f, "-")?;
            }
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for Guid {
    type Err = GuidParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().replace('-', "");

        if s.len() != 32 {
            return Err(GuidParseError::InvalidLength(s.len()));
        }

        let mut bytes = [0u8; 16];
        for (i, chunk) in s.as_bytes().chunks_exact(2).enumerate() {
            let byte = u8::from_str_radix(
                std::str::from_utf8(chunk).map_err(|_| GuidParseError::InvalidHex)?,
                16,
            )
            .map_err(|_| GuidParseError::InvalidHex)?;
            bytes[i] = byte;
        }
        Ok(Guid::new(bytes))
    }
}

impl serde::Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> serde::Deserialize<'de> for Guid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            Ok(Guid::from_str(&s).map_err(serde::de::Error::custom)?)
        } else {
            let bytes = <[u8; 16]>::deserialize(deserializer)?;
            Ok(Guid::new(bytes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BYTES: [u8; 16] = [
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE,
        0xF0,
    ];

    #[test]
    fn test_from_into_array() {
        let guid: Guid = TEST_BYTES.into();
        let bytes: [u8; 16] = guid.into();
        assert_eq!(bytes, TEST_BYTES);
    }

    #[test]
    fn test_display_and_debug() {
        let guid = Guid::new(TEST_BYTES);
        let expected = "12345678-9abc-def0-1234-56789abcdef0";

        assert_eq!(format!("{}", guid), expected);
        assert_eq!(format!("{:?}", guid), expected);
    }

    #[test]
    fn test_from_str_success() {
        let expected = Guid::new(TEST_BYTES);

        // Standard format with hyphens
        assert_eq!(
            "12345678-9abc-def0-1234-56789abcdef0"
                .parse::<Guid>()
                .unwrap(),
            expected
        );

        // Without hyphens
        assert_eq!(
            "123456789abcdef0123456789abcdef0".parse::<Guid>().unwrap(),
            expected
        );

        // Mixed case + whitespace
        assert_eq!(
            "  12345678-9ABC-DEF0-1234-56789ABCDEF0  "
                .parse::<Guid>()
                .unwrap(),
            expected
        );
    }

    #[test]
    fn test_serde_json_roundtrip() {
        let original = Guid::new(TEST_BYTES);
        let json = serde_json::to_string(&original).unwrap();

        assert_eq!(json, "\"12345678-9abc-def0-1234-56789abcdef0\"");

        let deserialized: Guid = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, original);
    }
}
