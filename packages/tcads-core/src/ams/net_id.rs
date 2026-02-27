use super::error::NetIdError;
use std::fmt;
use std::str::FromStr;

/// Length of the AMS port (2 bytes)
pub const AMS_PORT_LEN: usize = 2;

/// A 6-byte identifier for an ADS device (e.g. `172.16.17.10.1.1`).
///
/// # Notes
///
/// The **AMS Net ID** is purely logical and usually has no relation to the IP address.
/// It is configured at the target system. At the PC this TwinCAT System Control is used.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AmsNetId([u8; AmsNetId::LENGTH]);

impl AmsNetId {
    /// The length of an AMS Net ID in bytes.
    pub const LENGTH: usize = 6;

    /// Create a new AmsNetId from the given octets.
    pub const fn new(oct1: u8, oct2: u8, oct3: u8, oct4: u8, oct5: u8, oct6: u8) -> Self {
        Self([oct1, oct2, oct3, oct4, oct5, oct6])
    }

    /// Converts the current instance into a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Converts the current instance into a byte array.
    pub fn to_bytes(&self) -> [u8; AmsNetId::LENGTH] {
        self.0
    }

    /// Converts the given byte array into an [`AmsNetId`].
    pub fn from_bytes(bytes: [u8; AmsNetId::LENGTH]) -> Self {
        Self(bytes)
    }

    /// Converts the given byte slice into an [`AmsNetId`].
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, NetIdError> {
        Self::try_from(bytes)
    }
}

impl From<[u8; AmsNetId::LENGTH]> for AmsNetId {
    /// Convert an array of 6 bytes into an [`AmsNetId`].
    fn from(value: [u8; AmsNetId::LENGTH]) -> Self {
        Self(value)
    }
}

impl TryFrom<&[u8]> for AmsNetId {
    type Error = NetIdError;

    /// Convert a slice of bytes into an [`AmsNetId`].
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != AmsNetId::LENGTH {
            return Err(NetIdError::InvalidBufferSize {
                expected: AmsNetId::LENGTH,
                got: bytes.len(),
            });
        }

        let mut arr = [0u8; AmsNetId::LENGTH];
        arr.copy_from_slice(&bytes[..AmsNetId::LENGTH]);
        Ok(Self(arr))
    }
}

impl FromStr for AmsNetId {
    type Err = NetIdError;

    /// Parse AMS Net ID from string like `"192.168.1.1.1.1"`
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = [0u8; AmsNetId::LENGTH];
        let mut parts = s.split('.');

        for (i, byte) in bytes.iter_mut().enumerate() {
            let part = parts.next().ok_or(NetIdError::WrongOctetCount {
                expected: AmsNetId::LENGTH,
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
                expected: AmsNetId::LENGTH,
                got: AmsNetId::LENGTH + extra_count,
            });
        }

        Ok(Self(bytes))
    }
}

impl From<AmsNetId> for [u8; AmsNetId::LENGTH] {
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

impl serde::Serialize for AmsNetId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for AmsNetId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = <&str>::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
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

    #[test]
    fn test_serde_ams_net_id_serialize() {
        let id = AmsNetId::new(192, 168, 0, 1, 1, 1);
        let s = serde_json::to_string(&id).unwrap();
        assert_eq!(s, r#""192.168.0.1.1.1""#);
    }

    #[test]
    fn test_serde_ams_net_id_deserialize() {
        let id: AmsNetId = serde_json::from_str(r#""192.168.0.1.1.1""#).unwrap();
        assert_eq!(id, AmsNetId::new(192, 168, 0, 1, 1, 1));
    }

    #[test]
    fn test_serde_ams_net_id_roundtrip() {
        let original = AmsNetId::new(10, 0, 0, 1, 1, 1);
        let s = serde_json::to_string(&original).unwrap();
        let roundtripped: AmsNetId = serde_json::from_str(&s).unwrap();
        assert_eq!(original, roundtripped);
    }

    #[test]
    fn test_serde_ams_net_id_invalid_string() {
        let err = serde_json::from_str::<AmsNetId>(r#""not.a.valid.netid""#);
        assert!(err.is_err());
    }
}
