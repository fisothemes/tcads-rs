pub mod blocking;
pub mod tokio;

/// Index group for fetching a single data type entry by name.
pub const DATATYPE_INFO_BY_NAME_INDEX_GROUP: u32 = 0xF011;

/// Index group for fetching the full data type blob.
pub const DATATYPE_UPLOAD_INDEX_GROUP: u32 = 0xF00E;

/// Index group for fetching symbol and data type upload info,
pub const SYMBOL_UPLOAD_INFO_INDEX_GROUP: u32 = 0xF00F;
