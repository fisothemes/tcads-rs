use super::{LOGGER_DATA_LEN, MAX_TASK_NAME_LEN, MessageType};
use tcads_core::ads::{AdsString, StateFlag};
use tcads_core::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, AmsAddr, AmsCommand, AmsFrame, AmsPort,
    WindowsFileTime,
};

const MAX_MSG_LEN: usize = LOGGER_DATA_LEN as usize - LogEntry::MIN_LENGTH;

/// TwinCAT logger entry.
///
/// # Wire Format
///
/// | Offset | Size | Field                                        |
/// |--------|------|----------------------------------------------|
/// | 0      | 8    | Timestamp (Windows FILETIME, LE u64)         |
/// | 8      | 4    | [`MessageType`] mask (LE u32)                |
/// | 12     | 2    | Sender ADS port (LE u16)                     |
/// | 14     | 2    | Unknown                                      |
/// | 16     | 16   | Sender name (fixed, null-padded)             |
/// | 32     | 4    | Message length in bytes, excl. null (LE u32) |
/// | 36     | n    | Message (Windows-1252, null-terminated)      |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct LogEntry {
    timestamp: WindowsFileTime,
    message_type: MessageType,
    sender_port: AmsPort,
    sender: String,
    message: String,
}

impl LogEntry {
    /// Minimum length of a valid log entry payload in bytes.
    ///
    /// 8 (timestamp) + 4 (message type) + 2 (port) + 2 (unknown)
    /// + 16 (sender) + 4 (message length) = 3
    pub const MIN_LENGTH: usize = 36;

    /// Create a new log entry.
    pub fn new(
        timestamp: WindowsFileTime,
        message_type: MessageType,
        sender_port: AmsPort,
        sender: String,
        message: String,
    ) -> Self {
        Self {
            timestamp,
            message_type,
            sender_port,
            sender,
            message,
        }
    }

    /// Tries to parse a [`LogEntry`] from a slice of bytes (Little Endian).
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, crate::Error> {
        Self::try_from(bytes)
    }

    /// Converts a [`LogEntry`] to a slice of bytes (Little Endian).
    pub fn to_bytes(&self) -> Vec<u8> {
        self.into()
    }

    /// Timestamp of the log entry.
    pub fn timestamp(&self) -> WindowsFileTime {
        self.timestamp
    }

    /// Message type flags.
    pub fn message_type(&self) -> MessageType {
        self.message_type
    }

    /// ADS port of the sender (e.g. 10000 for TwinCAT System, 350 for PLC Task).
    pub fn sender_port(&self) -> AmsPort {
        self.sender_port
    }

    /// Name of the sender (e.g. `"TwinCAT System"`, `"PlcTask1"`).
    pub fn sender(&self) -> &str {
        &self.sender
    }

    /// The log message text.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Builds an [`AmsFrame`] that writes a log message to the TwinCAT logger (port 100).
///
/// This replicates the wire format of [`ADSLOGSTR`](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_system/31033611.html?id=9189897725322916238)
/// confirmed from capture analysis.
///
/// # Wire layout (ADS data, after the 32-byte ADS header)
///
/// The frame sent to the logger is essentially a non-standard [`AdsDeviceNotification`](tcads_core::protocol::AdsDeviceNotification)
/// whose Stamps field is set to `0` and has ADS data of size `Length` following it.
///
/// Size this format is non-standard, the function is `pub(super)` to avoid issues in the future.
///
/// | Offset | Size | Field Name       | Description                                               |
/// |--------|------|------------------|-----------------------------------------------------------|
/// | 0      | 4    | `Length`         | Size in bytes of the ADS payload (excludes its own field) |
/// | 4      | 4    | `Stamps`         | Number of notification stamps (always `0`)                |
/// | 8      | 8    | `Filetime`       | [`WindowsFileTime`]                                       |
/// | 16     | 8    | Reserved         | always `0`                                                |
/// | 24     | 4    | Msg + Arg Length | String length of format string + format specifier string  |
/// | 28     | 4    | Msg Type         | [`MessageType`] raw value                                 |
/// | 32     | 20   | Task Name        | Sender name as a Windows-1252 null terminated string      |
/// | 52     | n    | Message          | A null terminated Windows-1252 string                     |
/// | n + 1  | n    | Arg              | Argument for the format specifier (Windows-1252)          |
///
/// ## Note
///
/// The `Msg + Arg Length` field only needs to be greater than the `Message` length for TwinCAT
/// to perform substitution; the actual message length is bounded by the ADS header `length` field.
pub(super) fn entry_to_frame(target: AmsAddr, source: AmsAddr, entry: LogEntry) -> AmsFrame {
    let mut data = Vec::with_capacity(52);

    let stamps = [0u8; 4];
    let filetime = entry.timestamp.to_bytes();
    let reserved = [0u8; 8];
    let message_type = entry.message_type.to_bytes();
    let task_name =
        AdsString::<MAX_TASK_NAME_LEN>::try_from(entry.sender.as_str()).unwrap_or_else(|_| {
            // Encode best-effort then truncate to fit the buffer.
            let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(entry.sender.as_str());
            AdsString::from(encoded.as_ref())
        });

    let (message, _, _) = encoding_rs::WINDOWS_1252.encode(&entry.message);
    // Ignoring the `Arg` length because it's unnecessary in Rust.
    // Using `+ 1` for the null terminator.
    let message_arg_len = (message.len() as u32 + 1).to_le_bytes();

    data.extend_from_slice(&stamps);
    data.extend_from_slice(&filetime);
    data.extend_from_slice(&reserved);
    data.extend_from_slice(&message_arg_len);
    data.extend_from_slice(&message_type);
    data.extend_from_slice(task_name.as_bytes());
    data.extend_from_slice(message.as_ref());
    data.push(0);

    let header = AdsHeader::new(
        target,
        source,
        AdsCommand::AdsDeviceNotification,
        StateFlag::ADS_COMMAND.into(),
        4 + data.len() as u32,
        AdsReturnCode::Ok,
        0,
    );

    let mut payload = Vec::with_capacity(AdsHeader::LENGTH + 4 + data.len());

    payload.extend_from_slice(&header.to_bytes());
    payload.extend_from_slice(&(data.len() as u32).to_le_bytes());
    payload.extend_from_slice(&data);

    AmsFrame::new(AmsCommand::AdsCommand, payload)
}

impl TryFrom<&[u8]> for LogEntry {
    type Error = crate::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() < Self::MIN_LENGTH {
            return Err(crate::Error::InvalidPayload);
        }

        let timestamp = WindowsFileTime::try_from_slice(&data[0..8]).map_err(AdsError::from)?;
        let message_type = MessageType::try_from(&data[8..12])?;
        let sender_port = AmsPort::from_le_bytes([data[12], data[13]]);
        let sender = AdsString::<16>::from(&data[16..32]).to_string();

        let msg_len = u32::from_le_bytes(
            data[32..36]
                .try_into()
                .map_err(|_| crate::Error::InvalidPayload)?,
        ) as usize;

        let msg_end = 36 + msg_len;
        if data.len() < msg_end {
            return Err(crate::Error::InvalidPayload);
        }

        let msg_data = &data[36..msg_end];
        let message = if message_type.is_utf8() {
            String::from_utf8_lossy(msg_data)
                .trim_end_matches('\0')
                .to_owned()
        } else {
            AdsString::<MAX_MSG_LEN>::from(msg_data).to_string()
        };

        Ok(Self::new(
            timestamp,
            message_type,
            sender_port,
            sender,
            message,
        ))
    }
}

impl From<&LogEntry> for Vec<u8> {
    fn from(value: &LogEntry) -> Self {
        let sender_ads = AdsString::<16>::try_from(value.sender.as_str())
            .unwrap_or_else(|_| AdsString::<16>::from(value.sender.as_bytes()));

        let msg_encoded: Vec<u8> = if value.message_type.is_utf8() {
            value.message.as_bytes().to_vec()
        } else {
            let ads = AdsString::<1024>::try_from(value.message.as_str())
                .unwrap_or_else(|_| AdsString::<1024>::from(value.message.as_bytes()));
            ads.as_bytes_until_nul().to_vec()
        };

        let msg_len = msg_encoded.len() as u32;

        let mut buf = Vec::with_capacity(LogEntry::MIN_LENGTH + msg_encoded.len() + 1);

        buf.extend_from_slice(&value.timestamp.to_bytes());
        buf.extend_from_slice(&value.message_type.to_bytes());
        buf.extend_from_slice(&value.sender_port.to_le_bytes());
        buf.extend_from_slice(&[0u8; 2]);
        buf.extend_from_slice(sender_ads.as_bytes());
        buf.extend_from_slice(&msg_len.to_le_bytes());
        buf.extend_from_slice(&msg_encoded);
        buf.push(0u8);

        buf
    }
}

impl From<LogEntry> for Vec<u8> {
    fn from(value: LogEntry) -> Self {
        (&value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_bytes() -> Vec<u8> {
        vec![
            80, 92, 253, 99, 128, 182, 220, 1, // timestamp
            17, 0, 0, 0, // message type = HINT | LOG
            94, 1, // sender port = 350
            0, 0, // unknown
            80, 108, 99, 84, 97, 115, 107, 0, 0, 0, 0, 0, 0, 0, 0, 0, // sender = "PlcTask"
            9, 0, 0, 0, // message length = 9
            65, 65, 65, 65, 40, 49, 48, 49, 41, 0, // message = "AAAA(101)"
        ]
    }

    #[test]
    fn decodes_known_payload() {
        let entry = LogEntry::try_from(test_bytes().as_slice()).unwrap();
        assert_eq!(entry.sender(), "PlcTask");
        assert_eq!(entry.message(), "AAAA(101)");
        assert_eq!(entry.sender_port(), 350);
        assert!(entry.message_type().is_hint());
        assert!(entry.message_type().is_log());
        assert!(!entry.message_type().is_error());
    }

    #[test]
    fn returns_err_for_short_payload() {
        assert!(LogEntry::try_from([0u8; 35].as_slice()).is_err());
    }

    #[test]
    fn returns_err_for_truncated_message() {
        let mut data = test_bytes();
        // Claim message is 100 bytes but don't provide them
        data[32..36].copy_from_slice(&100u32.to_le_bytes());
        assert!(LogEntry::try_from(data.as_slice()).is_err());
    }

    #[test]
    fn returns_err_for_empty_payload() {
        assert!(LogEntry::try_from([].as_slice()).is_err());
    }

    #[test]
    fn roundtrips_through_bytes() {
        let original = LogEntry::try_from(test_bytes().as_slice()).unwrap();
        let bytes = original.to_bytes();
        let decoded = LogEntry::try_from(bytes.as_slice()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn from_impl_matches_to_bytes() {
        let entry = LogEntry::try_from(test_bytes().as_slice()).unwrap();
        let via_method = entry.to_bytes();
        let via_from: Vec<u8> = Vec::from(&entry);
        assert_eq!(via_method, via_from);
    }

    #[test]
    fn roundtrips_utf8_message() {
        let mut data = test_bytes();
        // Set UTF8 flag (0x1000) in addition to existing flags
        let flags = (MessageType::HINT | MessageType::LOG | MessageType::UTF8).to_le_bytes();
        data[8..12].copy_from_slice(&flags);
        // Replace message bytes with a UTF-8 string containing a non-ASCII char
        let utf8_msg = "héllo".as_bytes();
        let msg_len = utf8_msg.len() as u32;
        // Rebuild from offset 32 onwards
        data.truncate(32);
        data.extend_from_slice(&msg_len.to_le_bytes());
        data.extend_from_slice(utf8_msg);
        data.push(0);

        let entry = LogEntry::try_from(data.as_slice()).unwrap();
        assert_eq!(entry.message(), "héllo");
        assert!(entry.message_type().is_utf8());

        // Roundtrip
        let bytes = entry.to_bytes();
        let decoded = LogEntry::try_from(bytes.as_slice()).unwrap();
        assert_eq!(decoded.message(), "héllo");
    }
}
