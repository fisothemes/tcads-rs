use super::error::AdsDeviceVersionError;
use std::fmt;
use std::fmt::Debug;

/// An ADS Device's version number.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AdsDeviceVersion {
    major: u8,
    minor: u8,
    build: u16,
}

impl AdsDeviceVersion {
    /// The length of the ADS Device Version in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new instance of the [`AdsDeviceVersion`]
    pub fn new(major: u8, minor: u8, build: u16) -> Self {
        Self {
            major,
            minor,
            build,
        }
    }

    /// Creates a new [`AdsDeviceVersion`] from a 4-byte array.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from(bytes)
    }

    /// Converts the [`AdsDeviceVersion`] into a 4-byte array.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        (*self).into()
    }

    /// Tries to parse a 4-byte array into an [`AdsDeviceVersion`].
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AdsDeviceVersionError> {
        Self::try_from(bytes)
    }

    /// Returns the major version number.
    pub fn major(&self) -> u8 {
        self.major
    }

    /// Returns the minor version number.
    pub fn minor(&self) -> u8 {
        self.minor
    }

    /// Returns the build version number.
    pub fn build(&self) -> u16 {
        self.build
    }
}

impl From<[u8; Self::LENGTH]> for AdsDeviceVersion {
    fn from(bytes: [u8; Self::LENGTH]) -> Self {
        Self {
            major: bytes[0],
            minor: bytes[1],
            build: u16::from_le_bytes([bytes[2], bytes[3]]),
        }
    }
}

impl From<&[u8; Self::LENGTH]> for AdsDeviceVersion {
    fn from(bytes: &[u8; Self::LENGTH]) -> Self {
        Self::from(*bytes)
    }
}

impl From<AdsDeviceVersion> for [u8; AdsDeviceVersion::LENGTH] {
    fn from(version: AdsDeviceVersion) -> Self {
        let build_le = version.build.to_le_bytes();
        [version.major, version.minor, build_le[0], build_le[1]]
    }
}

impl From<&AdsDeviceVersion> for [u8; AdsDeviceVersion::LENGTH] {
    fn from(version: &AdsDeviceVersion) -> Self {
        (*version).into()
    }
}

impl TryFrom<&[u8]> for AdsDeviceVersion {
    type Error = AdsDeviceVersionError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < Self::LENGTH {
            return Err(AdsDeviceVersionError::UnexpectedLength {
                expected: Self::LENGTH,
                got: bytes.len(),
            });
        }
        Ok(Self::from([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

impl Debug for AdsDeviceVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.build)
    }
}

impl fmt::Display for AdsDeviceVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.build)
    }
}
