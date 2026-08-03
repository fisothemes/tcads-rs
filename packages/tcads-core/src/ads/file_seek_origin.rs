/// Reference point for a [`SYSTEM_SERVICE_FSEEK`](super::IndexGroup::SYSTEM_SERVICE_FSEEK)
/// request's seek position.
#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub enum AdsFileSeekOrigin {
    /// Seek from the beginning of the file.
    #[default]
    Set = 0,
    /// Seek from the current position of the file pointer.
    Current = 1,
    /// Seek from the end of the file.
    End = 2,
}

impl AdsFileSeekOrigin {
    /// Returns [`AdsFileSeekOrigin`] as a `u32`.
    pub const fn as_u32(self) -> u32 {
        match self {
            Self::Set => 0,
            Self::Current => 1,
            Self::End => 2,
        }
    }

    /// Serializes the seek origin into a 4-byte little-endian array.
    pub const fn to_bytes(self) -> [u8; 4] {
        (self as u32).to_le_bytes()
    }
}

impl From<AdsFileSeekOrigin> for u32 {
    fn from(value: AdsFileSeekOrigin) -> Self {
        value.as_u32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_set() {
        assert_eq!(AdsFileSeekOrigin::default(), AdsFileSeekOrigin::Set);
        assert_eq!(AdsFileSeekOrigin::default().as_u32(), 0);
    }

    #[test]
    fn roundtrips_known_values() {
        for (raw, variant) in [
            (0u32, AdsFileSeekOrigin::Set),
            (1, AdsFileSeekOrigin::Current),
            (2, AdsFileSeekOrigin::End),
        ] {
            assert_eq!(variant.as_u32(), raw);
        }
    }
}
