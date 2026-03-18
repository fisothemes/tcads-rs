use core::ops::{BitAnd, BitOr, BitOrAssign, Not};
use std::fmt;

/// Message type of ADS logger device message.
///
/// `MessageType` corresponds to the `msgCtrlMask` parameter used in TwinCAT's
/// [`ADSLOGSTR`](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_system/31033611.html?id=9189897725322916238)
/// and related logging functions. It determines the severity of the
/// message (e.g. Hint, Warning, Error) and where the message is dispatched
/// (e.g. the TwinCAT "Error List" window, a log file, or a message box).
///
/// Since this is a bitmask, multiple flags can be combined using the bitwise
/// OR (`|`) operator to, for example, send a warning both to the static log
/// file and the runtime message window.
///
/// # Examples
///
/// ```
/// use tcads_client::devices::logger::MessageType;
///
/// // Create a mask for a warning that should also be logged to a file
/// let mask = MessageType::new(MessageType::WARNING | MessageType::LOG);
///
/// assert!(mask.is_warning());
/// assert!(mask.is_log());
/// assert!(!mask.is_error());
/// ```
///
/// # Wire Format
/// - 4 bytes, Little Endian `u32`.
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct MessageType(u32);

impl MessageType {
    /// The length of the Message Type in bytes.
    pub const LENGTH: usize = 4;
    /// Informational hint messages.
    pub const HINT: u32 = 0x01;
    /// Warning messages.
    pub const WARNING: u32 = 0x02;
    /// Error messages.
    pub const ERROR: u32 = 0x04;
    /// Log messages (write to a log file).
    pub const LOG: u32 = 0x10;
    /// Message box pop-up.
    pub const MSGBOX: u32 = 0x20;
    /// Resource string message.
    pub const RESOURCE: u32 = 0x40;
    /// Plain string message.
    pub const STRING: u32 = 0x80;
    /// UTF-8 encoded string message.
    pub const UTF8: u32 = 0x1000;

    /// Creates a new [`MessageType`] set of flags from a raw u32.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Creates a [`MessageType`] from a 4-byte array (Little Endian).
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(u32::from_le_bytes(bytes))
    }

    /// Converts [`MessageType`] to a 4-byte array (Little Endian).
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.0.to_le_bytes()
    }

    /// Returns the raw value of the [`MessageType`].
    pub const fn as_raw(self) -> u32 {
        self.0
    }

    /// Returns `true` if the [`MessageType::HINT`] flag has been set.
    pub fn is_hint(self) -> bool {
        self.0 & Self::HINT != 0
    }
    /// Returns `true` if the [`MessageType::WARNING`] flag has been set.
    pub fn is_warning(self) -> bool {
        self.0 & Self::WARNING != 0
    }
    /// Returns `true` if the [`MessageType::ERROR`] flag has been set.
    pub fn is_error(self) -> bool {
        self.0 & Self::ERROR != 0
    }
    /// Returns `true` if the [`MessageType::LOG`] flag has been set.
    pub fn is_log(self) -> bool {
        self.0 & Self::LOG != 0
    }
    /// Returns `true` if the [`MessageType::MSGBOX`] flag has been set.
    pub fn is_msgbox(self) -> bool {
        self.0 & Self::MSGBOX != 0
    }
    /// Returns `true` if the [`MessageType::RESOURCE`] flag has been set.
    pub fn is_resource(self) -> bool {
        self.0 & Self::RESOURCE != 0
    }
    /// Returns `true` if the [`MessageType::STRING`] flag has been set.
    pub fn is_string(self) -> bool {
        self.0 & Self::STRING != 0
    }
    /// Returns `true` if the [`MessageType::UTF8`] flag has been set.
    pub fn is_utf8(self) -> bool {
        self.0 & Self::UTF8 != 0
    }
}

impl From<u32> for MessageType {
    fn from(val: u32) -> Self {
        Self(val)
    }
}

impl From<MessageType> for u32 {
    fn from(msg_type: MessageType) -> Self {
        msg_type.0
    }
}

impl From<[u8; Self::LENGTH]> for MessageType {
    fn from(value: [u8; Self::LENGTH]) -> Self {
        Self::from_bytes(value)
    }
}

impl From<MessageType> for [u8; MessageType::LENGTH] {
    fn from(value: MessageType) -> Self {
        value.to_bytes()
    }
}

impl BitOr for MessageType {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for MessageType {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for MessageType {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl Not for MessageType {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl fmt::Debug for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageType({:#04X}", self.0)?;

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

            check_flag!(self.is_hint(), "HINT");
            check_flag!(self.is_warning(), "WARNING");
            check_flag!(self.is_error(), "ERROR");
            check_flag!(self.is_log(), "LOG");
            check_flag!(self.is_msgbox(), "MSGBOX");
            check_flag!(self.is_resource(), "RESOURCE");
            check_flag!(self.is_string(), "STRING");
            check_flag!(self.is_utf8(), "UTF8");

            let known = MessageType::HINT
                | MessageType::WARNING
                | MessageType::ERROR
                | MessageType::LOG
                | MessageType::MSGBOX
                | MessageType::RESOURCE
                | MessageType::STRING
                | MessageType::UTF8;

            if (self.0 & !known) != 0 {
                if !first {
                    f.write_str(" | ")?;
                }
                f.write_str("Unknown")?;
            }
        }

        f.write_str(")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_as_raw() {
        let msg_type = MessageType::new(0x42);
        assert_eq!(msg_type.as_raw(), 0x42);
    }

    #[test]
    fn test_from_bytes() {
        let bytes = [0x21, 0x43, 0x00, 0x00];
        let msg_type = MessageType::from_bytes(bytes);
        assert_eq!(msg_type.as_raw(), 0x4321);
    }

    #[test]
    fn test_to_bytes() {
        let msg_type = MessageType::new(0x1234);
        let bytes = msg_type.to_bytes();
        assert_eq!(bytes, [0x34, 0x12, 0x00, 0x00]);
    }

    #[test]
    fn test_from_u32() {
        let msg_type = MessageType::from(0x80);
        assert_eq!(msg_type.as_raw(), 0x80);
    }

    #[test]
    fn test_into_u32() {
        let msg_type = MessageType::new(0x40);
        let value: u32 = msg_type.into();
        assert_eq!(value, 0x40);
    }

    #[test]
    fn test_from_byte_array() {
        let bytes = [0xFF, 0x00, 0x10, 0x00];
        let msg_type = MessageType::from(bytes);
        assert_eq!(msg_type.as_raw(), 0x1000FF);
    }

    #[test]
    fn test_into_byte_array() {
        let msg_type = MessageType::new(0x1040);
        let bytes: [u8; MessageType::LENGTH] = msg_type.into();
        assert_eq!(bytes, [0x40, 0x10, 0x00, 0x00]);
    }

    #[test]
    fn test_flag_checks() {
        let hint = MessageType::new(MessageType::HINT);
        assert!(hint.is_hint());
        assert!(!hint.is_warning());

        let warning = MessageType::new(MessageType::WARNING);
        assert!(warning.is_warning());
        assert!(!warning.is_error());

        let error = MessageType::new(MessageType::ERROR);
        assert!(error.is_error());
        assert!(!error.is_log());

        let log = MessageType::new(MessageType::LOG);
        assert!(log.is_log());
        assert!(!log.is_msgbox());

        let msgbox = MessageType::new(MessageType::MSGBOX);
        assert!(msgbox.is_msgbox());
        assert!(!msgbox.is_resource());

        let resource = MessageType::new(MessageType::RESOURCE);
        assert!(resource.is_resource());
        assert!(!resource.is_string());

        let string = MessageType::new(MessageType::STRING);
        assert!(string.is_string());
        assert!(!string.is_utf8());

        let utf8 = MessageType::new(MessageType::UTF8);
        assert!(utf8.is_utf8());
        assert!(!utf8.is_hint());
    }

    #[test]
    fn test_bitor() {
        let msg_type = MessageType::new(MessageType::HINT) | MessageType::new(MessageType::WARNING);
        assert!(msg_type.is_hint());
        assert!(msg_type.is_warning());
        assert!(!msg_type.is_error());
    }

    #[test]
    fn test_bitor_assign() {
        let mut msg_type = MessageType::new(MessageType::ERROR);
        msg_type |= MessageType::new(MessageType::LOG);
        assert!(msg_type.is_error());
        assert!(msg_type.is_log());
        assert!(!msg_type.is_hint());
    }

    #[test]
    fn test_bitand() {
        let msg_type = MessageType::new(MessageType::HINT | MessageType::WARNING);
        let result = msg_type & MessageType::new(MessageType::HINT);
        assert_eq!(result.as_raw(), MessageType::HINT);
    }

    #[test]
    fn test_not() {
        let msg_type = MessageType::new(MessageType::HINT);
        let inverted = !msg_type;
        assert_eq!(inverted.as_raw(), !MessageType::HINT);
    }

    #[test]
    fn test_debug_single_flag() {
        let msg_type = MessageType::new(MessageType::HINT);
        let debug_str = format!("{:?}", msg_type);
        assert!(debug_str.contains("HINT"));
        assert!(debug_str.contains("0x01") || debug_str.contains("0x0001"));
    }

    #[test]
    fn test_debug_multiple_flags() {
        let msg_type = MessageType::new(MessageType::ERROR | MessageType::LOG | MessageType::UTF8);
        let debug_str = format!("{:?}", msg_type);
        assert!(debug_str.contains("ERROR"));
        assert!(debug_str.contains("LOG"));
        assert!(debug_str.contains("UTF8"));
        assert!(debug_str.contains("|"));
    }

    #[test]
    fn test_debug_unknown_flags() {
        let msg_type = MessageType::new(0x8000);
        let debug_str = format!("{:?}", msg_type);
        assert!(debug_str.contains("Unknown"));
    }

    #[test]
    fn test_debug_zero_value() {
        let msg_type = MessageType::new(0);
        let debug_str = format!("{:?}", msg_type);
        assert!(debug_str.contains("MessageType"));
        assert!(debug_str.contains("0x00") || debug_str.contains("0x0000"));
    }
}
