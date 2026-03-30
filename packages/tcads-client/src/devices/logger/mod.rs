pub mod blocking;
pub mod tokio;

pub(crate) use crate::{IndexGroup, IndexOffset};
pub use crate::{LogEntry, LogMessageType};

/// ADS port for the TwinCAT system logger.
pub const LOGGER_PORT: u16 = 100;

/// Index group for logger notifications.
pub const LOGGER_INDEX_GROUP: IndexGroup = 1;

/// Index offset for logger notifications.
pub const LOGGER_INDEX_OFFSET: IndexOffset = 0xFFFF;
