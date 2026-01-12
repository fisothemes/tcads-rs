use super::netid::AmsNetId;
use std::fmt;

pub type AmsPort = u16;

/// An address in the ADS network (AMS Net ID and AMS Port No.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AmsAddr {
    net_id: AmsNetId,
    port: AmsPort,
}

impl AmsAddr {
    /// Creates a new ADS address.
    pub fn new(net_id: AmsNetId, port: AmsPort) -> Self {
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
