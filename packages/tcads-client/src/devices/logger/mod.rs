pub mod blocking;
pub mod tokio;

use tcads_core::{IndexGroup, IndexOffset, LogEntry, LogMessageType};

/// Index group for logger notifications.
pub const LOGGER_INDEX_GROUP: IndexGroup = 1;

/// Index offset for logger notifications.
pub const LOGGER_INDEX_OFFSET: IndexOffset = 0xFFFF;
