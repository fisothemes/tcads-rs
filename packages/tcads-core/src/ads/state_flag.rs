use super::error::StateFlagError;
use core::ops::{BitAnd, BitOr, BitOrAssign, Not};
use std::fmt;

/// AMS State Flags (16-bit bitfield) wrapper.
///
/// Contains information about the exchange (Request/Response) and the transport (TCP/UDP).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct StateFlag(pub u16);

impl StateFlag {
    /// The length of the State Flag in bytes.
    pub const LENGTH: usize = 2;

    /// Bit 0: RESPONSE
    ///
    /// 0 = This message is a *request*
    /// 1 = This message is a *response*
    ///
    /// Set by the receiver when replying to a request. Used to match
    /// responses to `invoke_id`. For normal ADS traffic:
    /// - Request: ADS_COMMAND
    /// - Response: ADS_COMMAND | RESPONSE
    pub const RESPONSE: u16 = 0x0001;
    /// Bit 1: NO_RETURN
    ///
    /// Tells the receiver to *not send a response* for this command.
    ///
    /// Used for "fire-and-forget" operations where the sender does not
    /// care about success/failure or wants to avoid round-trip latency.
    /// Commonly combined with UDP or broadcast.
    ///
    /// **Note:** You will never receive an error or confirmation if this is set.
    pub const NO_RETURN: u16 = 0x0002;
    /// Bit 2: ADS_COMMAND
    ///
    /// Marks this AMS frame as carrying an ADS command (READ, WRITE, etc.).
    /// Must be set for all normal ADS requests and responses.
    ///
    /// If this bit is *not* set, the frame is interpreted as an AMS
    /// router/system command instead of an application-level ADS command.
    pub const ADS_COMMAND: u16 = 0x0004;
    /// Bit 3: SYS_COMMAND
    ///
    /// Marks this frame as a system/router-level command rather than
    /// an application ADS command.
    ///
    /// Used internally by the AMS router for management and infrastructure
    /// messages. Normal ADS clients should not set this.
    pub const SYS_COMMAND: u16 = 0x0008;
    /// Bit 4: HIGH_PRIORITY
    ///
    /// Requests priority handling by the router/runtime.
    /// Intended for time-critical or real-time control paths.
    ///
    /// Rarely used in standard ADS client traffic.
    pub const HIGH_PRIORITY: u16 = 0x0010;
    /// Bit 5: TIMESTAMP
    ///
    /// Indicates that an additional 8-byte timestamp is appended to the
    /// payload. The receiver must account for the increased data length.
    ///
    /// Only meaningful if both sender and receiver explicitly expect
    /// and parse the timestamp.
    pub const TIMESTAMP: u16 = 0x0020;
    /// Bit 6: UDP
    ///
    /// 0 = Transport is TCP (default, reliable)
    /// 1 = Transport is UDP (unreliable, lower latency)
    ///
    /// UDP is typically used for real-time, broadcast, or discovery traffic
    /// and is often combined with NO_RETURN.
    pub const UDP: u16 = 0x0040;
    /// Bit 7: INIT_CMD
    ///
    /// Marks a command sent during TwinCAT/AMS initialization.
    /// Used during startup before the system is fully operational.
    ///
    /// Not used for normal runtime ADS communication.
    pub const INIT_CMD: u16 = 0x0080;
    /// Bit 15: BROADCAST
    ///
    /// Sends the command to all reachable nodes rather than a single target.
    /// Typically combined with UDP and often with NO_RETURN.
    ///
    /// Used for discovery, announcements, or router-level signalling.
    pub const BROADCAST: u16 = 0x8000;

    /// Creates a new generic set of flags from a raw u16.
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    /// Standard ADS request over TCP (most common).
    /// Bits: ADS_COMMAND
    pub const fn tcp_ads_request() -> Self {
        Self(Self::ADS_COMMAND)
    }

    /// Standard ADS response over TCP.
    /// Bits: ADS_COMMAND | RESPONSE
    pub const fn tcp_ads_response() -> Self {
        Self(Self::ADS_COMMAND | Self::RESPONSE)
    }

    /// Standard ADS request over UDP.
    /// Bits: ADS_COMMAND | UDP
    pub const fn udp_ads_request() -> Self {
        Self(Self::ADS_COMMAND | Self::UDP)
    }

    /// Standard ADS response over UDP.
    /// Bits: ADS_COMMAND | UDP | RESPONSE
    pub const fn udp_ads_response() -> Self {
        Self(Self::ADS_COMMAND | Self::UDP | Self::RESPONSE)
    }

    /// Creates `StateFlag` from a 2-byte array (Little Endian).
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from(bytes)
    }

    /// Converts flags to a 2-byte array (Little Endian).
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        (*self).into()
    }

    /// Tries to parse a `StateFlag` from a byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, StateFlagError> {
        bytes.try_into()
    }

    /// True if the RESPONSE bit is set (Server -> Client).
    pub fn is_response(&self) -> bool {
        (self.0 & Self::RESPONSE) != 0
    }

    /// True if the RESPONSE bit is not set (Client -> Server).
    pub fn is_request(&self) -> bool {
        !self.is_response()
    }

    /// True if the UDP bit is set.
    pub fn is_udp(&self) -> bool {
        (self.0 & Self::UDP) != 0
    }

    /// True if the UDP bit is not set (implies TCP).
    pub fn is_tcp(&self) -> bool {
        !self.is_udp()
    }

    /// True if this is an ADS command message (Should be true for all ADS traffic).
    pub fn is_ads_command(&self) -> bool {
        (self.0 & Self::ADS_COMMAND) != 0
    }

    /// True if the "System Command" bit is set.
    pub fn is_system_command(&self) -> bool {
        (self.0 & Self::SYS_COMMAND) != 0
    }

    /// True if the High Priority bit is set.
    pub fn is_high_priority(&self) -> bool {
        (self.0 & Self::HIGH_PRIORITY) != 0
    }

    /// True if the Timestamp bit is set.
    pub fn has_timestamp_added(&self) -> bool {
        (self.0 & Self::TIMESTAMP) != 0
    }

    /// True if No Return bit is set.
    pub fn is_no_return(&self) -> bool {
        (self.0 & Self::NO_RETURN) != 0
    }

    /// True if Init Command bit is set.
    pub fn is_init_command(&self) -> bool {
        (self.0 & Self::INIT_CMD) != 0
    }

    /// True if Broadcast bit is set.
    pub fn is_broadcast(&self) -> bool {
        (self.0 & Self::BROADCAST) != 0
    }
}

impl From<u16> for StateFlag {
    fn from(val: u16) -> Self {
        Self(val)
    }
}

impl From<StateFlag> for u16 {
    fn from(flag: StateFlag) -> Self {
        flag.0
    }
}

impl From<[u8; Self::LENGTH]> for StateFlag {
    fn from(value: [u8; Self::LENGTH]) -> Self {
        Self(u16::from_le_bytes(value))
    }
}

impl From<StateFlag> for [u8; StateFlag::LENGTH] {
    fn from(value: StateFlag) -> Self {
        value.0.to_le_bytes()
    }
}

impl TryFrom<&[u8]> for StateFlag {
    type Error = StateFlagError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < StateFlag::LENGTH {
            return Err(StateFlagError::UnexpectedLength {
                expected: StateFlag::LENGTH,
                got: value.len(),
            });
        }
        Ok(Self::from([value[0], value[1]]))
    }
}

impl BitOr for StateFlag {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for StateFlag {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for StateFlag {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl Not for StateFlag {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl fmt::Debug for StateFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateFlag({:#06X}", self.0)?;

        if self.0 != 0 {
            f.write_str(": ")?;
            let mut first = true;

            macro_rules! check_flag {
                ($check:expr, $name:literal) => {
                    if $check {
                        if !first {
                            f.write_str(" | ")?;
                        }
                        f.write_str($name)?;
                        first = false;
                    }
                };
            }

            check_flag!(self.is_response(), "RESPONSE");
            check_flag!(self.is_ads_command(), "ADS_COMMAND");
            check_flag!(self.is_udp(), "UDP");
            check_flag!(self.is_system_command(), "SYS_COMMAND");
            check_flag!(self.is_high_priority(), "HIGH_PRIORITY");
            check_flag!(self.is_broadcast(), "BROADCAST");
            check_flag!(self.is_init_command(), "INIT_CMD");
            check_flag!(self.is_no_return(), "NO_RETURN");
            check_flag!(self.has_timestamp_added(), "TIMESTAMP");

            let known = StateFlag::RESPONSE
                | StateFlag::ADS_COMMAND
                | StateFlag::UDP
                | StateFlag::SYS_COMMAND
                | StateFlag::HIGH_PRIORITY
                | StateFlag::TIMESTAMP
                | StateFlag::INIT_CMD
                | StateFlag::BROADCAST
                | StateFlag::NO_RETURN;

            if (self.0 & !known) != 0 {
                if !first {
                    f.write_str(" | ")?;
                }
                f.write_str("UNKNOWN")?;
            }
        }

        f.write_str(")")
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

    pub const fn with_mask(self, mask: u16) -> Self {
        Self(StateFlag(self.0.0 | mask))
    }

    pub const fn without_mask(self, mask: u16) -> Self {
        Self(StateFlag(self.0.0 & !mask))
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

    const RAW_RESPONSE: u16 = 0x0001;
    const RAW_NO_RETURN: u16 = 0x0002;
    const RAW_ADS_COMMAND: u16 = 0x0004;
    const RAW_UDP: u16 = 0x0040;
    const RAW_BROADCAST: u16 = 0x8000;

    #[test]
    fn test_tcp_ads_request() {
        let flag = StateFlag::tcp_ads_request();
        assert_eq!(flag.0, RAW_ADS_COMMAND);
        assert!(flag.is_ads_command());
        assert!(flag.is_request());
        assert!(flag.is_tcp());
        assert!(!flag.is_response());
        assert!(!flag.is_udp());
    }

    #[test]
    fn test_tcp_ads_response() {
        let flag = StateFlag::tcp_ads_response();
        assert_eq!(flag.0, RAW_ADS_COMMAND | RAW_RESPONSE);
        assert!(flag.is_ads_command());
        assert!(flag.is_response());
        assert!(flag.is_tcp());
        assert!(!flag.is_request());
        assert!(!flag.is_udp());
    }

    #[test]
    fn test_udp_ads_request() {
        let flag = StateFlag::udp_ads_request();
        assert_eq!(flag.0, RAW_ADS_COMMAND | RAW_UDP);
        assert!(flag.is_ads_command());
        assert!(flag.is_request());
        assert!(flag.is_udp());
        assert!(!flag.is_response());
        assert!(!flag.is_tcp());
    }

    #[test]
    fn test_udp_ads_response() {
        let flag = StateFlag::udp_ads_response();
        assert_eq!(flag.0, RAW_ADS_COMMAND | RAW_UDP | RAW_RESPONSE);
        assert!(flag.is_ads_command());
        assert!(flag.is_response());
        assert!(flag.is_udp());
        assert!(!flag.is_request());
        assert!(!flag.is_tcp());
    }

    #[test]
    fn test_builder_tcp_ads_request() {
        let flag = StateFlagBuilder::new(0)
            .ads_command()
            .request()
            .tcp()
            .build();
        assert_eq!(flag.0, RAW_ADS_COMMAND);
        assert!(flag.is_ads_command());
        assert!(flag.is_request());
        assert!(flag.is_tcp());
    }

    #[test]
    fn test_builder_tcp_ads_response() {
        let flag = StateFlagBuilder::new(0)
            .ads_command()
            .response()
            .tcp()
            .build();
        assert_eq!(flag.0, RAW_ADS_COMMAND | RAW_RESPONSE);
        assert!(flag.is_ads_command());
        assert!(flag.is_response());
        assert!(flag.is_tcp());
    }

    #[test]
    fn test_builder_udp_ads_request() {
        let flag = StateFlagBuilder::new(0)
            .ads_command()
            .request()
            .udp()
            .build();
        assert_eq!(flag.0, RAW_ADS_COMMAND | RAW_UDP);
        assert!(flag.is_ads_command());
        assert!(flag.is_request());
        assert!(flag.is_udp());
    }

    #[test]
    fn test_builder_udp_ads_response() {
        let flag = StateFlagBuilder::new(0)
            .ads_command()
            .response()
            .udp()
            .build();
        assert_eq!(flag.0, RAW_ADS_COMMAND | RAW_UDP | RAW_RESPONSE);
        assert!(flag.is_ads_command());
        assert!(flag.is_response());
        assert!(flag.is_udp());
    }

    #[test]
    fn test_builder_no_return_request() {
        let flag = StateFlagBuilder::new(0)
            .ads_command()
            .request()
            .tcp()
            .no_return()
            .build();
        assert_eq!(flag.0, RAW_ADS_COMMAND | RAW_NO_RETURN);
        assert!(flag.is_ads_command());
        assert!(flag.is_request());
        assert!(flag.is_no_return());
        assert!(flag.is_tcp());
    }

    #[test]
    fn test_builder_broadcast_udp_request() {
        let flag = StateFlagBuilder::new(0)
            .ads_command()
            .request()
            .udp()
            .broadcast()
            .build();
        assert_eq!(flag.0, RAW_ADS_COMMAND | RAW_UDP | RAW_BROADCAST);
        assert!(flag.is_ads_command());
        assert!(flag.is_request());
        assert!(flag.is_udp());
        assert!(flag.is_broadcast());
    }

    #[test]
    fn test_from_bytes() {
        let bytes = [0x01, 0x02];
        let flag = StateFlag::from_bytes(bytes);
        assert_eq!(flag.0, 0x0201);
    }

    #[test]
    fn test_to_bytes() {
        let flag = StateFlag::from_bytes([0x01, 0x02]);
        assert_eq!(flag.to_bytes(), [0x01, 0x02]);
    }

    #[test]
    fn test_try_from_slice() {
        let bytes = [0x01, 0x02];
        let flag = StateFlag::try_from_slice(&bytes[..]).unwrap();
        assert_eq!(flag.0, 0x0201);
    }
}
