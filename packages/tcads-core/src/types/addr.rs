//! Structures and implementations for representing and handling ADS network addresses.
//! An ADS network address consists of an AMS Net ID and an AMS port number.

use super::netid::AmsNetId;
use crate::errors::ParseAmsAddrError;
use std::fmt;
use std::str::FromStr;

pub type AmsPort = u16;

/// An address in the ADS network (AMS Net ID and AMS Port No.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AmsAddr {
    net_id: AmsNetId,
    port: AmsPort,
}

impl AmsAddr {
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
}

impl fmt::Display for AmsAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.net_id, self.port)
    }
}

impl FromStr for AmsAddr {
    type Err = ParseAmsAddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (netid_str, port_str) = s.rsplit_once(':').ok_or(ParseAmsAddrError::Format)?;

        let net_id = AmsNetId::from_str(netid_str).map_err(ParseAmsAddrError::NetId)?;
        let port = port_str
            .parse::<AmsPort>()
            .map_err(ParseAmsAddrError::Port)?;

        Ok(Self { net_id, port })
    }
}

impl From<(AmsNetId, AmsPort)> for AmsAddr {
    fn from((net_id, port): (AmsNetId, AmsPort)) -> Self {
        Self::new(net_id, port)
    }
}

impl From<AmsAddr> for (AmsNetId, AmsPort) {
    fn from(addr: AmsAddr) -> Self {
        (addr.net_id(), addr.port())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_addr() {
        let addr: AmsAddr = "5.1.2.3.1.1:851".parse().unwrap();
        assert_eq!(addr.port(), 851);
        assert_eq!(addr.net_id().0, [5, 1, 2, 3, 1, 1]);
    }

    #[test]
    fn test_amsaddr_to_tuple_conversion() {
        let tuple: (AmsNetId, AmsPort) =
            AmsAddr::new(AmsNetId([172, 16, 17, 32, 1, 1]), 851).into();
        assert_eq!(tuple, (AmsNetId([172, 16, 17, 32, 1, 1]), 851));
    }

    #[test]
    fn test_amsaddr_from_tuple_conversion() {
        let addr: AmsAddr = (AmsNetId([172, 16, 17, 32, 1, 1]), 851).into();
        assert_eq!(addr, AmsAddr::new(AmsNetId([172, 16, 17, 32, 1, 1]), 851));
    }
}
