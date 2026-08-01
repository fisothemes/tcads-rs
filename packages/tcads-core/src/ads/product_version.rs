use super::error::AdsProductVersionError;
use std::fmt;

/// The System Service's product version, as returned by a
/// [`SYSTEM_SERVICE_PRODUCT_VERSION`](super::IndexGroup::SYSTEM_SERVICE_PRODUCT_VERSION) read.
///
/// # Wire Format
///
/// The response is a 16-byte buffer, but only the first 8 bytes carry data; the
/// remainder is reserved. Those 8 bytes hold four little-endian `u16` fields, and
/// **the field order on the wire does not match the version's display order**:
///
/// | Offset | Size | Field |
/// |--------|------|-------|
/// | 0      | 2    | `v2`  |
/// | 2      | 2    | `v1`  |
/// | 4      | 2    | `v4`  |
/// | 6      | 2    | `v3`  |
///
/// Displayed/formatted as `v1.v2.v3.v4`. This reordering is carried over from the
/// historical `adstool` decode and preserved here as-is, since it reflects how the
/// System Service actually lays the fields out on the wire.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct AdsProductVersion {
    v1: u16,
    v2: u16,
    v3: u16,
    v4: u16,
}

impl AdsProductVersion {
    /// Minimum number of bytes needed to decode an [`AdsProductVersion`].
    ///
    /// The System Service's response is padded to 16 bytes, but only the first 8
    /// are meaningful; a buffer of at least [`Self::MIN_LENGTH`] bytes is sufficient.
    pub const MIN_LENGTH: usize = 8;

    /// Creates a new [`AdsProductVersion`] from its four display-order fields.
    pub const fn new(v1: u16, v2: u16, v3: u16, v4: u16) -> Self {
        Self { v1, v2, v3, v4 }
    }

    /// Returns the first version field.
    pub fn v1(&self) -> u16 {
        self.v1
    }

    /// Returns the second version field.
    pub fn v2(&self) -> u16 {
        self.v2
    }

    /// Returns the third version field.
    pub fn v3(&self) -> u16 {
        self.v3
    }

    /// Returns the fourth version field.
    pub fn v4(&self) -> u16 {
        self.v4
    }

    /// Tries to parse an [`AdsProductVersion`] from a byte slice.
    ///
    /// `data` must be at least [`Self::MIN_LENGTH`] bytes; anything past that (e.g. the
    /// reserved tail of a 16-byte System Service response) is ignored.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsProductVersionError> {
        if data.len() < Self::MIN_LENGTH {
            return Err(AdsProductVersionError::TooShort {
                expected: Self::MIN_LENGTH,
                got: data.len(),
            });
        }

        let v2 = u16::from_le_bytes([data[0], data[1]]);
        let v1 = u16::from_le_bytes([data[2], data[3]]);
        let v4 = u16::from_le_bytes([data[4], data[5]]);
        let v3 = u16::from_le_bytes([data[6], data[7]]);

        Ok(Self { v1, v2, v3, v4 })
    }
}

impl TryFrom<&[u8]> for AdsProductVersion {
    type Error = AdsProductVersionError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

impl fmt::Display for AdsProductVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.v1, self.v2, self.v3, self.v4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_reordered_wire_fields() {
        let mut data = Vec::new();
        data.extend_from_slice(&2u16.to_le_bytes()); // v2
        data.extend_from_slice(&3u16.to_le_bytes()); // v1
        data.extend_from_slice(&5u16.to_le_bytes()); // v4
        data.extend_from_slice(&4025u16.to_le_bytes()); // v3
        data.extend_from_slice(&[0u8; 8]); // reserved tail, ignored

        let version = AdsProductVersion::try_from_slice(&data).unwrap();

        assert_eq!(version, AdsProductVersion::new(3, 2, 4025, 5));
        assert_eq!(version.to_string(), "3.2.4025.5");
    }

    #[test]
    fn rejects_short_payload() {
        let err = AdsProductVersion::try_from_slice(&[0u8; 4]).unwrap_err();
        assert!(matches!(
            err,
            AdsProductVersionError::TooShort {
                expected: 8,
                got: 4
            }
        ));
    }
}
