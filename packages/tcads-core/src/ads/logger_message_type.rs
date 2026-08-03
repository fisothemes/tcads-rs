//

use super::LogMessageTypeError;
use bitflags::bitflags;
use std::fmt;

bitflags! {
    /// Message type of ADS logger device message.
    ///
    /// `MessageType` corresponds to the `msgCtrlMask` parameter used in TwinCAT's
    /// [`ADSLOGSTR`](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_system/31033611.html?id=9189897725322916238)
    /// and related logging functions. It determines the severity of the
    /// message (e.g. Hint, Warning, Error) and where the message is dispatched
    /// (e.g. the TwinCAT "Error List" window, a log file, or a message box).
    ///
    /// Since this is a bitmask, multiple flags can be combined.
    ///
    /// # Wire Format
    /// - 4 bytes, Little Endian `u32`.
    #[derive(
        serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    )]
    #[repr(transparent)]
    pub struct LogMessageType: u32 {
        /// Informational hint messages.
        const HINT = 0x01;
        /// Warning messages.
        const WARNING = 0x02;
        /// Error messages.
        const ERROR = 0x04;
        /// Log messages (write to a log file).
        const LOG = 0x10;
        /// Message box pop-up.
        const MSGBOX = 0x20;
        /// Resource string message.
        const RESOURCE = 0x40;
        /// Plain string message.
        const STRING = 0x80;
        /// UTF-8 encoded string message.
        const UTF8 = 0x1000;
    }
}

impl LogMessageType {
    /// The length of the Message Type in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new [`LogMessageType`] from a raw `u32`, retaining any unrecognized bits.
    pub const fn new(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }

    /// Creates from a 4-byte little-endian array.
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bits_retain(u32::from_le_bytes(bytes))
    }

    /// Converts to a 4-byte little-endian array.
    pub const fn to_bytes(self) -> [u8; Self::LENGTH] {
        self.bits().to_le_bytes()
    }

    /// Returns the raw `u32` value.
    pub const fn as_raw(self) -> u32 {
        self.bits()
    }

    /// Returns `true` if the [`HINT`](Self::HINT) flag is set.
    pub const fn is_hint(self) -> bool {
        self.contains(Self::HINT)
    }
    /// Returns `true` if the [`WARNING`](Self::WARNING) flag is set.
    pub const fn is_warning(self) -> bool {
        self.contains(Self::WARNING)
    }
    /// Returns `true` if the [`ERROR`](Self::ERROR) flag is set.
    pub const fn is_error(self) -> bool {
        self.contains(Self::ERROR)
    }
    /// Returns `true` if the [`LOG`](Self::LOG) flag is set.
    pub const fn is_log(self) -> bool {
        self.contains(Self::LOG)
    }
    /// Returns `true` if the [`MSGBOX`](Self::MSGBOX) flag is set.
    pub const fn is_msgbox(self) -> bool {
        self.contains(Self::MSGBOX)
    }
    /// Returns `true` if the [`RESOURCE`](Self::RESOURCE) flag is set.
    pub const fn is_resource(self) -> bool {
        self.contains(Self::RESOURCE)
    }
    /// Returns `true` if the [`STRING`](Self::STRING) flag is set.
    pub const fn is_string(self) -> bool {
        self.contains(Self::STRING)
    }
    /// Returns `true` if the [`UTF8`](Self::UTF8) flag is set.
    pub const fn is_utf8(self) -> bool {
        self.contains(Self::UTF8)
    }
}

impl From<u32> for LogMessageType {
    fn from(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }
}

impl From<LogMessageType> for u32 {
    fn from(flags: LogMessageType) -> Self {
        flags.bits()
    }
}

impl From<[u8; LogMessageType::LENGTH]> for LogMessageType {
    fn from(bytes: [u8; LogMessageType::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<LogMessageType> for [u8; LogMessageType::LENGTH] {
    fn from(flags: LogMessageType) -> Self {
        flags.to_bytes()
    }
}

impl TryFrom<&[u8]> for LogMessageType {
    type Error = LogMessageTypeError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < Self::LENGTH {
            return Err(LogMessageTypeError::UnexpectedLength {
                expected: Self::LENGTH,
                got: value.len(),
            });
        }
        let bytes = [value[0], value[1], value[2], value[3]];
        Ok(Self::from_bytes(bytes))
    }
}

impl fmt::Display for LogMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}

impl fmt::Debug for LogMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(LogMessageType))
            .field(&format_args!("{:#010X}", self.bits()))
            .field(&format_args!("{}", self))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_flags() {
        let flags = LogMessageType::new(0x00001006); // ERROR (0x04) | WARNING (0x02) | UTF8 (0x1000)
        assert!(flags.is_error());
        assert!(flags.is_warning());
        assert!(flags.is_utf8());
        assert!(!flags.is_hint());
    }

    #[test]
    fn roundtrip_bytes() {
        let flags = LogMessageType::WARNING | LogMessageType::LOG | LogMessageType::UTF8;
        assert_eq!(LogMessageType::from_bytes(flags.to_bytes()), flags);
    }
}
