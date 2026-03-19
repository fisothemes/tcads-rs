use super::MessageType;
use tcads_core::{AmsPort, WindowsFileTime};

/// TwinCAT logger entry.
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
    pub const MIN_LENGTH: usize = 16;

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

impl TryFrom<&[u8]> for LogEntry {
    type Error = crate::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() < Self::MIN_LENGTH {
            return Err(crate::Error::InvalidPayload);
        }

        todo!()
    }
}
