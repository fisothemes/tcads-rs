use super::{ProtocolError, validate_ads_command, validate_ams_command};
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, LogMessageType, StateFlag, WindowsFileTime,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;
use encoding_rs::WINDOWS_1252;
use std::borrow::Cow;

/// A zero-copy view of an ADS Logger Write Request (Command `0x0008`).
///
/// Borrows `task_name`, `message`, and `arg` directly from the [`AmsFrame`] that
/// was parsed, avoiding allocation. This is the preferred type for servers receiving
/// log messages when PLC code calls `ADSLOGSTR` or similar.
///
/// For cases where the request must be stored or sent across a channel, convert to
/// [`AdsLoggerWriteRequestOwned`] via [`into_owned`](Self::into_owned) or
/// [`to_owned`](Self::to_owned).
///
/// # Usage
/// * **Client:** Sends log messages to the TwinCAT system logger on ADS port 100.
/// * **Server:** Receives log messages when PLC code calls `ADSLOGSTR` or similar.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsDeviceNotification`](AdsCommand::AdsDeviceNotification) (`0x0008`)
///
/// This is a non-standard use of the `AdsDeviceNotification` command repurposed by
/// Beckhoff for logger writes. **No response is sent by the logger.**
///
/// # Wire Format (ADS data, after the 32-byte ADS header)
///
/// | Offset | Size | Field       | Description                                               |
/// |--------|------|-------------|-----------------------------------------------------------|
/// | 0      | 4    | `Length`    | Size of the ADS payload (excl. this field)                |
/// | 4      | 4    | `Stamps`    | Always `0`                                                |
/// | 8      | 8    | `FileTime`  | [`WindowsFileTime`]                                       |
/// | 16     | 8    | `Reserved`  | Always `0`                                                |
/// | 24     | 4    | `MsgArgLen` | Combined byte length of `Message` + `Arg` incl. nulls     |
/// | 28     | 4    | `MsgType`   | [`LogMessageType`] raw value                              |
/// | 32     | 20   | `TaskName`  | Sender name (Windows-1252, null-padded)                   |
/// | 52     | n    | `Message`   | Format string (Windows-1252 or UTF-8, null-terminated)    |
/// | 52+n   | m    | `Arg`       | Format argument (Windows-1252 or UTF-8, null-terminated)  |
///
/// `Message` and `Arg` are encoded as UTF-8 when [`LogMessageType::UTF8`] is set,
/// otherwise as Windows-1252. `TaskName` is always Windows-1252.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsLoggerWriteRequest<'a> {
    header: AdsHeader,
    timestamp: WindowsFileTime,
    message_type: LogMessageType,
    task_name: Cow<'a, str>,
    message: Cow<'a, str>,
    arg: Cow<'a, str>,
}

impl<'a> AdsLoggerWriteRequest<'a> {
    /// Minimum size of the ADS payload in bytes.
    pub const MIN_PAYLOAD_SIZE: usize = 52;
    /// Maximum length of the task name field in bytes, including the null terminator.
    ///
    /// # Note
    ///
    /// This is specific to the logger write wire format. The logger read (notification)
    /// wire format uses a fixed 16-byte sender field, the difference is a Beckhoff quirk.
    pub const TASK_NAME_LEN: usize = 20;

    /// Tries to parse a request from an [`AmsFrame`].
    pub fn try_from_frame(frame: &'a AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Returns the timestamp.
    pub fn timestamp(&self) -> WindowsFileTime {
        self.timestamp
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the [`LogMessageType`].
    pub fn message_type(&self) -> LogMessageType {
        self.message_type
    }

    /// Returns the task name.
    pub fn task_name(&self) -> &str {
        self.task_name.as_ref()
    }

    /// Returns the message string.
    pub fn message(&self) -> &str {
        self.message.as_ref()
    }

    /// Returns the message string argument.
    pub fn arg(&self) -> &str {
        self.arg.as_ref()
    }

    /// Converts this view into an owned [`AdsLoggerWriteRequestOwned`], copying the message.
    pub fn into_owned(self) -> AdsLoggerWriteRequestOwned {
        AdsLoggerWriteRequestOwned::new(
            *self.header.target(),
            *self.header.source(),
            self.timestamp,
            self.message_type,
            self.task_name(),
            self.message(),
            self.arg(),
        )
    }

    /// Clones this view into an owned [`AdsLoggerWriteRequestOwned`], copying the message.
    pub fn to_owned(&self) -> AdsLoggerWriteRequestOwned {
        AdsLoggerWriteRequestOwned::new(
            *self.header.target(),
            *self.header.source(),
            self.timestamp,
            self.message_type,
            self.task_name(),
            self.message(),
            self.arg(),
        )
    }

    /// Parses only the ADS data payload (after the ADS header).
    ///
    /// Returns `(timestamp, message_type, task_name, message, arg)`.
    pub fn parse_payload(
        data: &'a [u8],
    ) -> Result<
        (
            WindowsFileTime,
            LogMessageType,
            Cow<'a, str>,
            Cow<'a, str>,
            Cow<'a, str>,
        ),
        ProtocolError,
    > {
        if data.len() < Self::MIN_PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE,
                got: data.len(),
            }
            .into());
        }

        let length = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;

        if data.len() - 4 < length {
            return Err(AdsError::UnexpectedDataLength {
                expected: length,
                got: data.len() - 4,
            }
            .into());
        }

        let timestamp = WindowsFileTime::try_from_slice(&data[8..16]).map_err(AdsError::from)?;
        let msg_arg_len = u32::from_le_bytes(data[24..28].try_into().unwrap()) as usize;
        let message_type = LogMessageType::try_from_bytes(&data[28..32]).map_err(AdsError::from)?;
        let (task_name, _, _) = WINDOWS_1252.decode(&data[32..32 + Self::TASK_NAME_LEN]);

        let strings_end = Self::MIN_PAYLOAD_SIZE + msg_arg_len;
        if data.len() < strings_end {
            return Err(AdsError::UnexpectedDataLength {
                expected: strings_end,
                got: data.len(),
            }
            .into());
        }

        let strings_data = &data[Self::MIN_PAYLOAD_SIZE..Self::MIN_PAYLOAD_SIZE + msg_arg_len];

        let msg_end = strings_data
            .iter()
            .position(|&b| b == 0x00)
            .unwrap_or(strings_data.len());
        let msg_bytes = &strings_data[..msg_end];

        let arg_start = (msg_end + 1).min(strings_data.len());
        let arg_bytes = &strings_data[arg_start..];
        let arg_end = arg_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(arg_bytes.len());
        let arg_bytes = &arg_bytes[..arg_end];

        let (message, arg) = if message_type.is_utf8() {
            (
                String::from_utf8_lossy(msg_bytes),
                String::from_utf8_lossy(arg_bytes),
            )
        } else {
            let (msg, _, _) = WINDOWS_1252.decode(msg_bytes);
            let (arg, _, _) = WINDOWS_1252.decode(arg_bytes);
            (msg, arg)
        };

        Ok((timestamp, message_type, task_name, message, arg))
    }
}

impl<'a> TryFrom<&'a AmsFrame> for AdsLoggerWriteRequest<'a> {
    type Error = ProtocolError;

    fn try_from(value: &'a AmsFrame) -> Result<Self, Self::Error> {
        validate_ams_command(value, AmsCommand::AdsCommand)?;
        let (header, payload) = AdsHeader::parse_prefix(value.payload()).map_err(AdsError::from)?;
        validate_ads_command(&header, AdsCommand::AdsDeviceNotification)?;

        let (timestamp, message_type, task_name, message, arg) = Self::parse_payload(payload)?;

        Ok(Self {
            header,
            timestamp,
            message_type,
            task_name,
            message,
            arg,
        })
    }
}

/// A fully owned ADS Logger Write Request (Command `0x0008`).
///
/// Owns its string fields, making it suitable for storage, sending across channels,
/// or constructing log writes on a client.
///
/// # Usage
///
/// * **Client:** Construct with [`new`](Self::new) and send via
///   [`write_frame`](crate::io::blocking::AmsStream::write_frame) (or similar).
/// * **Server:** Obtained by calling [`AdsLoggerWriteRequest::into_owned`] or
///   [`to_owned`](AdsLoggerWriteRequest::to_owned) after parsing an incoming frame.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsLoggerWriteRequestOwned {
    header: AdsHeader,
    timestamp: WindowsFileTime,
    message_type: LogMessageType,
    task_name: [u8; AdsLoggerWriteRequest::TASK_NAME_LEN],
    message: Vec<u8>,
    arg: Vec<u8>,
}

impl AdsLoggerWriteRequestOwned {
    /// Creates a new logger write request.
    ///
    /// `task_name` is encoded as Windows-1252 and truncated to
    /// [`TASK_NAME_LEN`](AdsLoggerWriteRequest::TASK_NAME_LEN) - 1 bytes if it
    /// exceeds that limit.
    ///
    /// `message` and `arg` are encoded as Windows-1252, or written as raw UTF-8
    /// bytes if [`LogMessageType::UTF8`] is set in `message_type`.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        timestamp: WindowsFileTime,
        message_type: LogMessageType,
        task_name: impl AsRef<str>,
        message: impl AsRef<str>,
        arg: impl AsRef<str>,
    ) -> Self {
        let task_name = Self::encode_task_name(task_name.as_ref());
        let (message, arg) = Self::encode_strings(message.as_ref(), arg.as_ref(), message_type);

        let data_len = AdsLoggerWriteRequest::MIN_PAYLOAD_SIZE + message.len() + 1 + arg.len() + 1;

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsDeviceNotification,
            StateFlag::ADS_COMMAND.into(),
            data_len as u32,
            AdsReturnCode::Ok,
            0,
        );

        Self {
            header,
            timestamp,
            message_type,
            task_name,
            message,
            arg,
        }
    }

    /// Consumes the request and converts it into an [`AmsFrame`].
    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(&self)
    }

    /// Serialises the request into an [`AmsFrame`].
    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the timestamp.
    pub fn timestamp(&self) -> WindowsFileTime {
        self.timestamp
    }

    /// Returns the [`LogMessageType`].
    pub fn message_type(&self) -> LogMessageType {
        self.message_type
    }

    /// Returns the task name, decoded from Windows-1252.
    pub fn task_name(&self) -> Cow<'_, str> {
        let (s, _, _) = WINDOWS_1252.decode(&self.task_name);
        s
    }

    /// Returns the message string.
    ///
    /// Decoded as UTF-8 if [`LogMessageType::UTF8`] is set, otherwise as Windows-1252.
    pub fn message(&self) -> Cow<'_, str> {
        Self::decode_string(&self.message, self.message_type)
    }

    /// Returns the message argument string.
    ///
    /// Decoded as UTF-8 if [`LogMessageType::UTF8`] is set, otherwise as Windows-1252.
    pub fn arg(&self) -> Cow<'_, str> {
        Self::decode_string(&self.arg, self.message_type)
    }

    /// Borrows this request as a zero-copy [`AdsLoggerWriteRequest`].
    pub fn as_view(&self) -> AdsLoggerWriteRequest<'_> {
        AdsLoggerWriteRequest {
            header: self.header.clone(),
            timestamp: self.timestamp,
            message_type: self.message_type,
            task_name: self.task_name(),
            message: self.message(),
            arg: self.arg(),
        }
    }

    /// Decodes a byte slice as UTF-8 or Windows-1252 based on the message type flag.
    fn decode_string(bytes: &[u8], message_type: LogMessageType) -> Cow<'_, str> {
        if message_type.is_utf8() {
            String::from_utf8_lossy(bytes)
        } else {
            let (s, _, _) = WINDOWS_1252.decode(bytes);
            s
        }
    }

    /// Encodes message and arg according to the UTF-8 flag.
    ///
    /// Task name is always Windows-1252 regardless of the UTF-8 flag.
    fn encode_strings(
        message: &str,
        arg: &str,
        message_type: LogMessageType,
    ) -> (Vec<u8>, Vec<u8>) {
        if message_type.is_utf8() {
            (message.as_bytes().to_vec(), arg.as_bytes().to_vec())
        } else {
            let (msg, _, _) = WINDOWS_1252.encode(message);
            let (arg, _, _) = WINDOWS_1252.encode(arg);
            (msg.into_owned(), arg.into_owned())
        }
    }

    /// Encodes the task name as Windows-1252, truncated and null-padded to
    /// [`TASK_NAME_LEN`](AdsLoggerWriteRequest::TASK_NAME_LEN) bytes.
    fn encode_task_name(task_name: &str) -> [u8; AdsLoggerWriteRequest::TASK_NAME_LEN] {
        let (encoded, _, _) = WINDOWS_1252.encode(task_name);
        let mut buf = [0u8; AdsLoggerWriteRequest::TASK_NAME_LEN];
        let len = encoded.len().min(AdsLoggerWriteRequest::TASK_NAME_LEN - 1);
        buf[..len].copy_from_slice(&encoded[..len]);
        buf
    }
}

impl From<&AdsLoggerWriteRequestOwned> for AmsFrame {
    fn from(value: &AdsLoggerWriteRequestOwned) -> Self {
        let msg_arg_len = value.message.len() + 1 + value.arg.len() + 1;
        let data_len = AdsLoggerWriteRequest::MIN_PAYLOAD_SIZE + msg_arg_len;

        let mut data = Vec::with_capacity(data_len);
        data.extend_from_slice(&(data_len.saturating_sub(4) as u32).to_le_bytes()); // Length
        data.extend_from_slice(&[0u8; 4]); // Stamps = 0
        data.extend_from_slice(&value.timestamp.to_bytes()); // FileTime
        data.extend_from_slice(&[0u8; 8]); // Reserved
        data.extend_from_slice(&(msg_arg_len as u32).to_le_bytes()); // MsgArgLen
        data.extend_from_slice(&value.message_type.to_bytes()); // MsgType
        data.extend_from_slice(&value.task_name); // TaskName
        data.extend_from_slice(&value.message); // Message
        data.push(0u8); // null
        data.extend_from_slice(&value.arg); // Arg
        data.push(0u8); // null

        let mut payload = Vec::with_capacity(AdsHeader::LENGTH + data.len());
        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&data);

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsLoggerWriteRequestOwned> for AmsFrame {
    fn from(value: AdsLoggerWriteRequestOwned) -> Self {
        AmsFrame::from(&value)
    }
}

impl<'a> From<AdsLoggerWriteRequest<'a>> for AdsLoggerWriteRequestOwned {
    fn from(value: AdsLoggerWriteRequest<'a>) -> Self {
        value.into_owned()
    }
}

impl<'a> From<&'a AdsLoggerWriteRequestOwned> for AdsLoggerWriteRequest<'a> {
    fn from(value: &'a AdsLoggerWriteRequestOwned) -> Self {
        value.as_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsNetId;

    fn make_addrs() -> (AmsAddr, AmsAddr) {
        let target = AmsAddr::new(AmsNetId::new(192, 168, 1, 1, 1, 1), 100);
        let source = AmsAddr::new(AmsNetId::new(192, 168, 1, 10, 1, 1), 33000);
        (target, source)
    }

    fn make_owned(
        message_type: LogMessageType,
        msg: &str,
        arg: &str,
    ) -> AdsLoggerWriteRequestOwned {
        let (target, source) = make_addrs();
        AdsLoggerWriteRequestOwned::new(
            target,
            source,
            WindowsFileTime::now(),
            message_type,
            "RustTask",
            msg,
            arg,
        )
    }

    #[test]
    fn roundtrip_windows1252() {
        let owned = make_owned(
            LogMessageType::new(LogMessageType::HINT | LogMessageType::LOG),
            "Hello from tcads-rs",
            "some arg",
        );
        let frame = owned.to_frame();
        let view = AdsLoggerWriteRequest::try_from_frame(&frame).unwrap();

        assert_eq!(view.message_type(), owned.message_type());
        assert_eq!(view.message(), owned.message());
        assert_eq!(view.arg(), owned.arg());
    }

    #[test]
    fn roundtrip_utf8() {
        let owned = make_owned(
            LogMessageType::new(LogMessageType::HINT | LogMessageType::UTF8),
            "héllo wörld",
            "ärg",
        );
        let frame = owned.to_frame();
        let view = AdsLoggerWriteRequest::try_from_frame(&frame).unwrap();

        assert_eq!(view.message(), "héllo wörld");
        assert_eq!(view.arg(), "ärg");
        assert!(view.message_type().is_utf8());
    }

    #[test]
    fn empty_arg_roundtrip() {
        let owned = make_owned(
            LogMessageType::new(LogMessageType::ERROR),
            "Something failed",
            "",
        );
        let frame = owned.to_frame();
        let view = AdsLoggerWriteRequest::try_from_frame(&frame).unwrap();

        assert_eq!(view.message(), "Something failed");
        assert_eq!(view.arg(), "");
    }

    #[test]
    fn as_view_borrows_without_allocation() {
        let owned = make_owned(LogMessageType::new(LogMessageType::HINT), "msg", "arg");
        let view = owned.as_view();

        assert_eq!(view.message().as_ptr(), owned.message().as_ptr());
        assert_eq!(view.arg().as_ptr(), owned.arg().as_ptr());
    }

    #[test]
    fn payload_too_short_returns_err() {
        let short = AmsFrame::new(AmsCommand::AdsCommand, vec![0u8; 10]);
        assert!(AdsLoggerWriteRequest::try_from_frame(&short).is_err());
    }
}
