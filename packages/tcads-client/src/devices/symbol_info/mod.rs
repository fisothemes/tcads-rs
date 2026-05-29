pub mod blocking;
pub mod tokio;

/// Index group for fetching symbol and data type upload info,
pub const SYMBOL_UPLOAD_INFO_INDEX_GROUP: u32 = 0xF00F;

/// Index group for fetching the full symbol table blob,
pub const SYMBOL_INFO_UPLOAD_INDEX_GROUP: u32 = 0xF00B;

/// Index group for fetching symbol info by name,
pub const SYMBOL_INFO_BY_NAME_EX_INDEX_GROUP: u32 = 0xF009;
