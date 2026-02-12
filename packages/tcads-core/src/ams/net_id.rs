use super::error::NetIdError;
use std::fmt;
use std::str::FromStr;

/// Length of the AMS Net ID (6 bytes)
pub const NETID_LEN: usize = 6;

/// Length of the AMS port (2 bytes)
pub const AMS_PORT_LEN: usize = 2;

/// A 6-byte identifier for an ADS device (e.g. `172.16.17.10.1.1`).
///
/// # Notes
///
/// The **AMS Net ID** is purely logical and usually has no relation to the IP address.
/// It is configured at the target system. At the PC this TwinCAT System Control is used.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AmsNetId([u8; NETID_LEN]);

impl AmsNetId {
    /// Create a new AmsNetId from the given octets.
    pub const fn new(oct1: u8, oct2: u8, oct3: u8, oct4: u8, oct5: u8, oct6: u8) -> Self {
        Self([oct1, oct2, oct3, oct4, oct5, oct6])
    }

    /// Converts the current instance into a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Converts the current instance into a byte array.
    pub fn to_bytes(&self) -> [u8; NETID_LEN] {
        self.0
    }

    /// Converts the given byte array into an [`AmsNetId`].
    pub fn from_bytes(bytes: [u8; NETID_LEN]) -> Self {
        Self(bytes)
    }

    /// Converts the given byte slice into an [`AmsNetId`].
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, NetIdError> {
        Self::try_from(bytes)
    }
}

impl From<[u8; NETID_LEN]> for AmsNetId {
    /// Convert an array of 6 bytes into an [`AmsNetId`].
    fn from(value: [u8; NETID_LEN]) -> Self {
        Self(value)
    }
}

impl TryFrom<&[u8]> for AmsNetId {
    type Error = NetIdError;

    /// Convert a slice of bytes into an [`AmsNetId`].
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != NETID_LEN {
            return Err(NetIdError::InvalidBufferSize {
                expected: NETID_LEN,
                got: bytes.len(),
            });
        }

        let mut arr = [0u8; NETID_LEN];
        arr.copy_from_slice(&bytes[..NETID_LEN]);
        Ok(Self(arr))
    }
}

impl FromStr for AmsNetId {
    type Err = NetIdError;

    /// Parse AMS Net ID from string like `"192.168.1.1.1.1"`
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = [0u8; NETID_LEN];
        let mut parts = s.split('.');

        for (i, byte) in bytes.iter_mut().enumerate() {
            let part = parts.next().ok_or_else(|| NetIdError::WrongOctetCount {
                expected: NETID_LEN,
                got: i,
            })?;

            *byte = part.parse::<u8>().map_err(|_| NetIdError::InvalidOctet {
                position: i,
                value: part.to_string(),
            })?;
        }

        let extra_count = parts.count();
        if extra_count > 0 {
            return Err(NetIdError::WrongOctetCount {
                expected: NETID_LEN,
                got: NETID_LEN + extra_count,
            });
        }

        Ok(Self(bytes))
    }
}

impl From<AmsNetId> for [u8; NETID_LEN] {
    fn from(value: AmsNetId) -> Self {
        value.0
    }
}

impl fmt::Display for AmsNetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}.{}.{}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::Debug for AmsNetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_netid() {
        let netid: AmsNetId = "192.168.1.1.1.1".parse().unwrap();
        assert_eq!(netid.as_bytes(), &[192, 168, 1, 1, 1, 1]);
    }

    #[test]
    fn parse_invalid_octet_count() {
        let err = "192.168.1.1".parse::<AmsNetId>().unwrap_err();
        assert!(matches!(err, NetIdError::WrongOctetCount { .. }));
    }

    #[test]
    fn parse_invalid_octet_value() {
        let err = "192.168.1.256.1.1".parse::<AmsNetId>().unwrap_err();
        assert!(matches!(err, NetIdError::InvalidOctet { .. }));
    }

    #[test]
    fn try_from_bytes() {
        let bytes = [192, 168, 1, 1, 1, 1];
        let netid = AmsNetId::try_from(&bytes[..]).unwrap();
        assert_eq!(netid.as_bytes(), &bytes);
    }

    #[test]
    fn try_from_short_buffer() {
        let bytes = [192, 168, 1];
        let err = AmsNetId::try_from(&bytes[..]).unwrap_err();
        assert!(matches!(err, NetIdError::InvalidBufferSize { .. }));
    }
}
