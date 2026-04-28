use tcads_core::IndexGroup;

pub mod blocking;
pub mod tokio;

pub const SUM_READ_INDEX_GROUP: IndexGroup = 0xF080;
pub const SUM_WRITE_INDEX_GROUP: IndexGroup = 0xF081;
pub const SUM_READ_WRITE_INDEX_GROUP: IndexGroup = 0xF082;
pub const SUM_READ_EX_INDEX_GROUP: IndexGroup = 0xF083;
pub const SUM_READ_EX_2_INDEX_GROUP: IndexGroup = 0xF084;
pub const SUM_ADD_NOTIFICATION_INDEX_GROUP: IndexGroup = 0xF085;
pub const SUM_DELETE_NOTIFICATION_INDEX_GROUP: IndexGroup = 0xF086;
