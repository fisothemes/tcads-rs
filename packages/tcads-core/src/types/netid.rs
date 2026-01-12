use crate::errors::ParseNetIdError;
use std::fmt;
use std::str::FromStr;

/// A 6-byte identifier for an ADS device (e.g. `172.16.17.10.1.1`).
///
/// # Notes
///
/// The **AMS Net Id** is purely logical and usually has no relation to the IP address.
/// It is configured at the target system. At the PC for this the TwinCAT System Control is used.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AmsNetId(pub [u8; 6]);

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

impl FromStr for AmsNetId {
    type Err = ParseNetIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 6 {
            return Err(ParseNetIdError);
        }

        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = part.parse().map_err(|_| ParseNetIdError)?;
        }

        Ok(Self(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_netid() {
        assert_eq!(
            AmsNetId::from_str("172.16.17.32.1.1").unwrap(),
            AmsNetId([172, 16, 17, 32, 1, 1])
        );
    }
}
