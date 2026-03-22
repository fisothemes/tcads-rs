pub mod blocking;
pub mod log_entry;
pub mod message_type;
pub mod tokio;

pub use log_entry::LogEntry;
pub use message_type::MessageType;

use crate::{IndexGroup, IndexOffset};

/// ADS port for the TwinCAT system logger.
pub const LOGGER_PORT: u16 = 100;

/// Index group for logger notifications.
pub const LOGGER_INDEX_GROUP: IndexGroup = 1;

/// Index offset for logger notifications.
pub const LOGGER_INDEX_OFFSET: IndexOffset = 0xFFFF;

/// Maximum size of a single logger notification payload.
pub const LOGGER_DATA_LEN: u32 = 1024;

/// Maximum length of the task name field on the write log entry wire (null terminator included).
pub const MAX_TASK_NAME_LEN: usize = 20;
