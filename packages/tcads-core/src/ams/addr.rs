//! Structures and implementations for representing and handling ADS network addresses.
//! An ADS network address consists of an AMS Net ID and an AMS port number.

use super::error::AddrError;
use super::net_id::AmsNetId;
use std::fmt;
use std::str::FromStr;

/// AMS port number
pub type AmsPort = u16;

/// An address in the ADS network (AMS Net ID + AMS Port No.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AmsAddr {
    net_id: AmsNetId,
    port: AmsPort,
}

impl AmsAddr {
    /// The length of an AMS address in bytes.
    pub const LENGTH: usize = 8;

    /// Creates a new ADS address.
    pub const fn new(net_id: AmsNetId, port: AmsPort) -> Self {
        Self { net_id, port }
    }

    /// Returns the AMS Net ID.
    pub fn net_id(&self) -> AmsNetId {
        self.net_id
    }

    /// Returns the AMS port number.
    pub fn port(&self) -> AmsPort {
        self.port
    }

    /// Converts the current instance into a byte array.
    pub fn to_bytes(&self) -> [u8; AmsAddr::LENGTH] {
        self.into()
    }

    /// Converts the given byte array into an [`AmsAddr`].
    pub fn from_bytes(bytes: [u8; AmsAddr::LENGTH]) -> Self {
        Self::from(bytes)
    }

    /// Converts the given byte slice into an [`AmsAddr`].
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AddrError> {
        Self::try_from(bytes)
    }
}

impl From<(AmsNetId, AmsPort)> for AmsAddr {
    fn from((net_id, port): (AmsNetId, AmsPort)) -> Self {
        Self::new(net_id, port)
    }
}

impl From<AmsAddr> for (AmsNetId, AmsPort) {
    fn from(addr: AmsAddr) -> Self {
        (addr.net_id, addr.port)
    }
}

impl From<&AmsAddr> for [u8; AmsAddr::LENGTH] {
    fn from(value: &AmsAddr) -> Self {
        let mut buf = [0u8; AmsAddr::LENGTH];

        buf[..AmsNetId::LENGTH].copy_from_slice(value.net_id.as_bytes());
        buf[AmsNetId::LENGTH..].copy_from_slice(&value.port.to_le_bytes());

        buf
    }
}

impl From<[u8; AmsAddr::LENGTH]> for AmsAddr {
    /// Converts an array of 8 bytes (6 bytes NetId + 2 bytes port in little-endian) into an [`AmsAddr`].
    fn from(value: [u8; AmsAddr::LENGTH]) -> Self {
        Self::try_from(&value[..]).unwrap()
    }
}

impl TryFrom<&[u8]> for AmsAddr {
    type Error = AddrError;

    /// Converts a slice of bytes into an [`AmsAddr`]. The slice must contain at least 8 bytes.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < AmsAddr::LENGTH {
            return Err(AddrError::BufferTooSmall {
                expected: AmsAddr::LENGTH,
                found: bytes.len(),
            });
        }

        let net_id = AmsNetId::try_from(&bytes[..6])?;
        let port = AmsPort::from_le_bytes([bytes[6], bytes[7]]);

        Ok(Self { net_id, port })
    }
}

impl FromStr for AmsAddr {
    type Err = AddrError;

    /// Parse AMS address from string like `"192.168.1.1.1.1:851"`
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (netid_str, port_str) = s.rsplit_once(':').ok_or(AddrError::MissingSeparator)?;

        let net_id = netid_str.parse::<AmsNetId>()?;
        let port = port_str
            .parse::<AmsPort>()
            .map_err(|_| AddrError::InvalidPort(port_str.to_string()))?;

        Ok(Self { net_id, port })
    }
}

impl fmt::Display for AmsAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.net_id, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_addr() {
        let addr: AmsAddr = "192.168.137.1.1.1:32818".parse().unwrap();
        assert_eq!(addr.net_id.as_bytes(), &[192, 168, 137, 1, 1, 1]);
        assert_eq!(addr.port, 32818);
    }

    #[test]
    fn parse_missing_separator() {
        let err = "192.168.1.1.1.1".parse::<AmsAddr>().unwrap_err();
        assert!(matches!(err, AddrError::MissingSeparator));
    }

    #[test]
    fn parse_invalid_port() {
        let err = "192.168.1.1.1.1:99999".parse::<AmsAddr>().unwrap_err();
        assert!(matches!(err, AddrError::InvalidPort(_)));
    }

    #[test]
    fn try_from_bytes() {
        let bytes = [192, 168, 137, 1, 1, 1, 0x32, 0x80]; // port 32818 in LE
        let addr = AmsAddr::try_from(&bytes[..]).unwrap();
        assert_eq!(addr.net_id.as_bytes(), &[192, 168, 137, 1, 1, 1]);
        assert_eq!(addr.port, 32818);
    }

    #[test]
    fn try_from_short_buffer() {
        let bytes = [192, 168, 1];
        let err = AmsAddr::try_from(&bytes[..]).unwrap_err();
        assert!(matches!(err, AddrError::BufferTooSmall { .. }));
    }

    #[test]
    fn roundtrip() {
        let original = "192.168.137.1.1.1:32818";
        let addr: AmsAddr = original.parse().unwrap();
        assert_eq!(addr.to_string(), original);
    }
}
