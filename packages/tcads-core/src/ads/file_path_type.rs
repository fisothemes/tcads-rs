/// The base location used to resolve remote file paths.
///
/// Most of the time, you will use [`Generic`](Self::Generic) to provide a standard,
/// absolute file path (e.g. `C:\Logs\data.txt`). The other variants act as convenient
/// shortcuts to TwinCAT's internal system directories, allowing you to access boot files
/// without needing to know their exact physical location on the target IPC.
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
pub enum AdsFilePathType {
    /// A standard operating system file path.
    #[default]
    Generic,
    /// Shortcut to the TwinCAT boot project directory.
    BootProject,
    /// Shortcut to the TwinCAT boot data directory.
    BootData,
    /// Shortcut to the TwinCAT boot path directory.
    BootPath,
    /// A base location not defined in this library.
    Unknown(u16),
}

impl AdsFilePathType {
    /// Length of the [`AdsFilePathType`] on the wire as bytes.
    pub const LENGTH: usize = 2;

    /// Creates a new instance of [`AdsFilePathType`] from a 2-byte array (little-endian).
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_u16(u16::from_le_bytes(bytes))
    }

    /// Returns [`AdsFilePathType`] as a 2-byte little-endian representation.
    pub const fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.as_u16().to_le_bytes()
    }

    /// Creates a new instance of [`AdsFilePathType`] from a `u16`
    pub const fn from_u16(value: u16) -> Self {
        match value {
            1 => Self::Generic,
            2 => Self::BootProject,
            3 => Self::BootData,
            4 => Self::BootPath,
            n => Self::Unknown(n),
        }
    }

    /// Returns [`AdsFilePathType`] as a `u16` representation.
    pub const fn as_u16(self) -> u16 {
        match self {
            Self::Generic => 1,
            Self::BootProject => 2,
            Self::BootData => 3,
            Self::BootPath => 4,
            Self::Unknown(n) => n,
        }
    }
}

impl From<u16> for AdsFilePathType {
    fn from(value: u16) -> Self {
        Self::from_u16(value)
    }
}

impl From<AdsFilePathType> for u16 {
    fn from(value: AdsFilePathType) -> Self {
        value.as_u16()
    }
}

impl From<AdsFilePathType> for u32 {
    fn from(value: AdsFilePathType) -> Self {
        value.as_u16() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_generic() {
        assert_eq!(AdsFilePathType::default(), AdsFilePathType::Generic);
        assert_eq!(AdsFilePathType::default().as_u16(), 1);
    }

    #[test]
    fn roundtrips_known_values() {
        for (raw, variant) in [
            (1u16, AdsFilePathType::Generic),
            (2, AdsFilePathType::BootProject),
            (3, AdsFilePathType::BootData),
            (4, AdsFilePathType::BootPath),
        ] {
            assert_eq!(AdsFilePathType::from(raw), variant);
            assert_eq!(variant.as_u16(), raw);
        }
    }

    #[test]
    fn unknown_value_roundtrips() {
        assert_eq!(AdsFilePathType::from(99), AdsFilePathType::Unknown(99));
        assert_eq!(AdsFilePathType::Unknown(99).as_u16(), 99);
    }
}
