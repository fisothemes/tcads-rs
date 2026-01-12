//! AMS State Flags

use std::fmt;

/// A type-safe wrapper for the 16-bit AMS State Flags.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct StateFlags(u16);

impl StateFlags {
    /// Bit 1: Response (0 = Request, 1 = Response)
    pub const MASK_RESPONSE: u16 = 0x0001;
    /// Bit 3: ADS Command (Always 1 for ADS)
    pub const MASK_COMMAND: u16 = 0x0004;
    /// Bit 7: UDP (0 = TCP, 1 = UDP)
    pub const MASK_UDP: u16 = 0x0040;

    /// Creates a new generic set of flags from a raw u16.
    pub const fn new(val: u16) -> Self {
        Self(val)
    }

    /// Creates the standard flags for a Client REQUEST.
    /// (ADS Command = 1, Response = 0)
    pub const fn request() -> Self {
        Self(Self::MASK_COMMAND)
    }

    /// Creates the standard flags for a Server RESPONSE.
    /// (ADS Command = 1, Response = 1)
    pub const fn response() -> Self {
        Self(Self::MASK_COMMAND | Self::MASK_RESPONSE)
    }

    /// Returns true if this is a Response packet (Server -> Client).
    pub fn is_response(&self) -> bool {
        (self.0 & Self::MASK_RESPONSE) != 0
    }

    /// Returns true if the UDP flag is set.
    pub fn is_udp(&self) -> bool {
        (self.0 & Self::MASK_UDP) != 0
    }

    /// Returns true if the TCP flag is set.
    pub fn is_tcp(&self) -> bool {
        !self.is_udp()
    }

    /// Returns true if this is a Request packet (Client -> Server).
    pub fn is_request(&self) -> bool {
        !self.is_response()
    }
}

impl From<StateFlags> for u16 {
    fn from(flags: StateFlags) -> Self {
        flags.0
    }
}

impl From<u16> for StateFlags {
    fn from(val: u16) -> Self {
        Self(val)
    }
}

impl fmt::Debug for StateFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateFlags({:#06X}: ", self.0)?;
        let mut first = true;

        if self.is_response() {
            f.write_str("RESPONSE")?;
            first = false;
        }

        if (self.0 & Self::MASK_COMMAND) != 0 {
            if !first {
                f.write_str(" | ")?;
            }
            f.write_str("COMMAND")?;
            first = false;
        }

        if self.is_udp() {
            if !first {
                f.write_str(" | ")?;
            }
            f.write_str("UDP")?;
            first = false;
        }

        let known_mask = Self::MASK_RESPONSE | Self::MASK_COMMAND | Self::MASK_UDP;
        if (self.0 & !known_mask) != 0 {
            if !first {
                f.write_str(" | ")?;
            }
            f.write_str("UNKNOWN_BITS")?;
        }

        f.write_str(")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_constructors() {
        // Request: COMMAND (0x0004)
        let req = StateFlags::request();
        assert_eq!(u16::from(req), 0x0004);
        assert!(req.is_request());
        assert!(!req.is_response());

        // Response: COMMAND | RESPONSE (0x0004 | 0x0001 = 0x0005)
        let res = StateFlags::response();
        assert_eq!(u16::from(res), 0x0005);
        assert!(res.is_response());
        assert!(!res.is_request());
    }

    #[test]
    fn test_conversions() {
        let flags: StateFlags = 0x0005.into();
        assert!(flags.is_response());

        let raw: u16 = flags.into();
        assert_eq!(raw, 0x0005);
    }

    #[test]
    fn test_debug_formatting() {
        // Standard Request
        let req = StateFlags::request();
        assert_eq!(format!("{:?}", req), "StateFlags(0x0004: COMMAND)");

        // Standard Response
        let res = StateFlags::response();
        assert_eq!(
            format!("{:?}", res),
            "StateFlags(0x0005: RESPONSE | COMMAND)"
        );

        // UDP Request
        let udp = StateFlags::new(StateFlags::MASK_COMMAND | StateFlags::MASK_UDP);
        assert_eq!(format!("{:?}", udp), "StateFlags(0x0044: COMMAND | UDP)");

        // Unknown Bits (e.g. 0xFFFF)
        // Should show known flags + UNKNOWN_BITS
        let unknown = StateFlags::new(0xFFFF);
        assert_eq!(
            format!("{:?}", unknown),
            "StateFlags(0xFFFF: RESPONSE | COMMAND | UDP | UNKNOWN_BITS)"
        );

        // Empty (Invalid state technically, but should print safely)
        let empty = StateFlags::new(0);
        assert_eq!(format!("{:?}", empty), "StateFlags(0x0000: )");
    }
}
