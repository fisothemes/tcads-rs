use super::error::StateFlagError;
use bitflags::bitflags;
use std::fmt;

bitflags! {
    /// AMS State Flags (16-bit bitfield) wrapper.
    ///
    /// Contains information about the exchange (Request/Response) and the transport (TCP/UDP).
    #[derive(
        serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    )]
    #[repr(transparent)]
    pub struct StateFlag: u16 {
        /// This message is a response (Client <- Server).
        const RESPONSE = 0x0001;
        /// Fire-and-forget; do not send a response.
        const NO_RETURN = 0x0002;
        /// Marks this AMS frame as carrying an ADS command (READ, WRITE, etc.).
        const ADS_COMMAND = 0x0004;
        /// Marks this frame as a system/router-level command.
        const SYS_COMMAND = 0x0008;
        /// Requests priority handling by the router/runtime.
        const HIGH_PRIORITY = 0x0010;
        /// Indicates that an additional 8-byte timestamp is appended to the payload.
        const TIMESTAMP = 0x0020;
        /// Transport is UDP (unreliable, lower latency).
        const UDP = 0x0040;
        /// Marks a command sent during TwinCAT/AMS initialization.
        const INIT_CMD = 0x0080;
        /// Sends the command to all reachable nodes rather than a single target.
        const BROADCAST = 0x8000;
    }
}

impl StateFlag {
    /// The length of the State Flag in bytes.
    pub const LENGTH: usize = 2;

    /// Creates a new generic set of flags from a raw `u16`, retaining unrecognized bits.
    pub const fn new(raw: u16) -> Self {
        Self::from_bits_retain(raw)
    }

    /// Creates `StateFlag` from a 2-byte array (Little Endian).
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bits_retain(u16::from_le_bytes(bytes))
    }

    /// Converts flags to a 2-byte array (Little Endian).
    pub const fn to_bytes(self) -> [u8; Self::LENGTH] {
        self.bits().to_le_bytes()
    }

    /// Standard ADS request over TCP (most common).
    pub const fn tcp_ads_request() -> Self {
        Self::ADS_COMMAND
    }

    /// Standard ADS response over TCP.
    pub const fn tcp_ads_response() -> Self {
        Self::from_bits_retain(Self::ADS_COMMAND.bits() | Self::RESPONSE.bits())
    }

    /// Standard ADS request over UDP.
    pub const fn udp_ads_request() -> Self {
        Self::from_bits_retain(Self::ADS_COMMAND.bits() | Self::UDP.bits())
    }

    /// Standard ADS response over UDP.
    pub const fn udp_ads_response() -> Self {
        Self::from_bits_retain(Self::ADS_COMMAND.bits() | Self::UDP.bits() | Self::RESPONSE.bits())
    }

    /// Returns the raw `u16` value.
    pub const fn as_raw(self) -> u16 {
        self.bits()
    }

    /// True if the RESPONSE bit is set (Server -> Client).
    pub const fn is_response(self) -> bool {
        self.contains(Self::RESPONSE)
    }

    /// True if the RESPONSE bit is not set (Client -> Server).
    pub const fn is_request(self) -> bool {
        !self.is_response()
    }

    /// True if the UDP bit is set.
    pub const fn is_udp(self) -> bool {
        self.contains(Self::UDP)
    }

    /// True if the UDP bit is not set (implies TCP).
    pub const fn is_tcp(self) -> bool {
        !self.is_udp()
    }

    /// True if this is an ADS command message.
    pub const fn is_ads_command(self) -> bool {
        self.contains(Self::ADS_COMMAND)
    }

    /// True if the "System Command" bit is set.
    pub const fn is_system_command(self) -> bool {
        self.contains(Self::SYS_COMMAND)
    }

    /// True if the High Priority bit is set.
    pub const fn is_high_priority(self) -> bool {
        self.contains(Self::HIGH_PRIORITY)
    }

    /// True if the Timestamp bit is set.
    pub const fn has_timestamp_added(self) -> bool {
        self.contains(Self::TIMESTAMP)
    }

    /// True if No Return bit is set.
    pub const fn is_no_return(self) -> bool {
        self.contains(Self::NO_RETURN)
    }

    /// True if Init Command bit is set.
    pub const fn is_init_command(self) -> bool {
        self.contains(Self::INIT_CMD)
    }

    /// True if Broadcast bit is set.
    pub const fn is_broadcast(self) -> bool {
        self.contains(Self::BROADCAST)
    }
}

impl From<u16> for StateFlag {
    fn from(raw: u16) -> Self {
        Self::from_bits_retain(raw)
    }
}

impl From<StateFlag> for u16 {
    fn from(flags: StateFlag) -> Self {
        flags.bits()
    }
}

impl From<[u8; StateFlag::LENGTH]> for StateFlag {
    fn from(bytes: [u8; StateFlag::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<StateFlag> for [u8; StateFlag::LENGTH] {
    fn from(flags: StateFlag) -> Self {
        flags.to_bytes()
    }
}

impl TryFrom<&[u8]> for StateFlag {
    type Error = StateFlagError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < Self::LENGTH {
            return Err(StateFlagError::UnexpectedLength {
                expected: Self::LENGTH,
                got: value.len(),
            });
        }
        let bytes = [value[0], value[1]];
        Ok(Self::from_bytes(bytes))
    }
}

impl fmt::Display for StateFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}

impl fmt::Debug for StateFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(StateFlag))
            .field(&format_args!("{:#06X}", self.bits()))
            .field(&format_args!("{}", self))
            .finish()
    }
}

/// A "bit mutator" for StateFlag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StateFlagBuilder(StateFlag);

impl StateFlagBuilder {
    pub const fn new(raw: u16) -> Self {
        Self(StateFlag::new(raw))
    }

    pub const fn from_flag(flag: StateFlag) -> Self {
        Self(flag)
    }

    pub const fn with_mask(self, mask: StateFlag) -> Self {
        Self(StateFlag::from_bits_retain(self.0.bits() | mask.bits()))
    }

    pub const fn without_mask(self, mask: StateFlag) -> Self {
        Self(StateFlag::from_bits_retain(self.0.bits() & !mask.bits()))
    }

    pub const fn response(self) -> Self {
        self.with_mask(StateFlag::RESPONSE)
    }

    pub const fn request(self) -> Self {
        self.without_mask(StateFlag::RESPONSE)
    }

    pub const fn udp(self) -> Self {
        self.with_mask(StateFlag::UDP)
    }

    pub const fn tcp(self) -> Self {
        self.without_mask(StateFlag::UDP)
    }

    pub const fn ads_command(self) -> Self {
        self.with_mask(StateFlag::ADS_COMMAND)
    }

    pub const fn system_command(self) -> Self {
        self.with_mask(StateFlag::SYS_COMMAND)
    }

    pub const fn high_priority(self) -> Self {
        self.with_mask(StateFlag::HIGH_PRIORITY)
    }

    pub const fn timestamp_added(self) -> Self {
        self.with_mask(StateFlag::TIMESTAMP)
    }

    pub const fn no_return(self) -> Self {
        self.with_mask(StateFlag::NO_RETURN)
    }

    pub const fn init_command(self) -> Self {
        self.with_mask(StateFlag::INIT_CMD)
    }

    pub const fn broadcast(self) -> Self {
        self.with_mask(StateFlag::BROADCAST)
    }

    pub const fn build(self) -> StateFlag {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_udp_request() {
        let flag = StateFlag::new(0x0044); // ADS_COMMAND (0x04) | UDP (0x40)
        assert!(flag.is_ads_command());
        assert!(flag.is_udp());
        assert!(flag.is_request());
        assert!(!flag.is_response());
        assert!(!flag.is_tcp());
    }

    #[test]
    fn roundtrip_bytes() {
        let flag = StateFlag::ADS_COMMAND | StateFlag::UDP | StateFlag::RESPONSE;
        assert_eq!(StateFlag::from_bytes(flag.to_bytes()), flag);
    }

    #[test]
    fn builder_methods() {
        let flag = StateFlagBuilder::new(0)
            .ads_command()
            .request()
            .udp()
            .broadcast()
            .build();

        assert!(flag.is_ads_command());
        assert!(flag.is_request());
        assert!(flag.is_udp());
        assert!(flag.is_broadcast());
        assert!(!flag.is_tcp());
    }
}
