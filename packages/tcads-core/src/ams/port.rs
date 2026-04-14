/// AMS port number.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct AmsPort(u16);

impl AmsPort {
    /// The length of an AMS port in bytes.
    pub const LENGTH: usize = 2;

    /// Port number of the ADS router.
    pub const ADS_ROUTER: Self = Self(1);

    /// Port number of the licence server.
    pub const LICENCE_SERVER: Self = Self(30);

    /// Port number of the standard-logger.
    pub const LOGGER: Self = Self(100);
    /// Port number of the TwinCAT event logger.
    pub const EVENT_LOGGER: Self = Self(110);

    /// Port number of the TwinCAT 2.xx IO PLC task.
    pub const TC2_IO_PLC_TASK: Self = Self(300);
    /// Port number of the TwinCAT 3 IO PLC task 0.
    pub const TC3_IO_PLC_TASK_0: Self = Self(350);
    /// Port number of the TwinCAT 3 IO PLC task 1.
    pub const TC3_IO_PLC_TASK_1: Self = Self(351);
    /// Port number of the TwinCAT 3 IO PLC task 2.
    pub const TC3_IO_PLC_TASK_2: Self = Self(352);
    /// Port number of the TwinCAT 3 IO PLC task 3.
    pub const TC3_IO_PLC_TASK_3: Self = Self(353);

    /// Port number of TwinCAT 2.xx PLC runtime system 1.
    pub const TC2_PLC_RUNTIME_1: Self = Self(801);
    /// Port number of TwinCAT 2.xx PLC runtime system 2.
    pub const TC2_PLC_RUNTIME_2: Self = Self(811);
    /// Port number of TwinCAT 2.xx PLC runtime system 3.
    pub const TC2_PLC_RUNTIME_3: Self = Self(821);
    /// Port number of TwinCAT 2.xx PLC runtime system 4.
    pub const TC2_PLC_RUNTIME_4: Self = Self(831);

    /// Port number of TwinCAT 3 PLC runtime system 1.
    pub const TC3_PLC_RUNTIME_1: Self = Self(851);
    /// Port number of TwinCAT 3 PLC runtime system 2.
    pub const TC3_PLC_RUNTIME_2: Self = Self(852);
    /// Port number of TwinCAT 3 PLC runtime system 3.
    pub const TC3_PLC_RUNTIME_3: Self = Self(853);
    /// Port number of TwinCAT 3 PLC runtime system 4.
    pub const TC3_PLC_RUNTIME_4: Self = Self(854);

    /// Port number of the AMS logger.
    pub const AMS_LOGGER: Self = Self(10502);

    /// Port number of the TwinCAT system service.
    pub const SYSTEM_SERVICE: Self = Self(10000);

    /// Creates a new AMS port.
    pub const fn new(port: u16) -> Self {
        Self(port)
    }

    /// Returns the port number as a u16.
    pub const fn as_u16(&self) -> u16 {
        self.0
    }

    /// Converts the port to a byte array.
    pub const fn to_bytes(&self) -> [u8; AmsPort::LENGTH] {
        self.0.to_le_bytes()
    }

    /// Converts a byte array to an AMS port.
    pub const fn from_bytes(bytes: [u8; AmsPort::LENGTH]) -> Self {
        Self(u16::from_le_bytes(bytes))
    }
}

impl From<u16> for AmsPort {
    fn from(port: u16) -> Self {
        Self(port)
    }
}

impl From<AmsPort> for u16 {
    fn from(port: AmsPort) -> Self {
        port.0
    }
}

impl From<[u8; AmsPort::LENGTH]> for AmsPort {
    fn from(bytes: [u8; AmsPort::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<&[u8; AmsPort::LENGTH]> for AmsPort {
    fn from(bytes: &[u8; AmsPort::LENGTH]) -> Self {
        Self::from_bytes(*bytes)
    }
}

impl From<AmsPort> for [u8; AmsPort::LENGTH] {
    fn from(port: AmsPort) -> Self {
        port.to_bytes()
    }
}

impl std::fmt::Display for AmsPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for AmsPort {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Default for AmsPort {
    fn default() -> Self {
        Self::TC3_PLC_RUNTIME_1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ams_port_from_u16() {
        let port = AmsPort::new(1234);
        assert_eq!(port.as_u16(), 1234);
    }

    #[test]
    fn ams_port_to_bytes() {
        let port = AmsPort::new(1234);
        assert_eq!(port.to_bytes(), [0xD2, 0x04]);
    }

    #[test]
    fn ams_port_from_bytes() {
        let port = AmsPort::from_bytes([0xD2, 0x04]);
        assert_eq!(port.as_u16(), 1234);
    }
}
