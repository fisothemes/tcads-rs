pub mod command;
pub mod device_state;
pub mod device_version;
pub mod error;
pub mod filetime;
pub mod header;
pub mod log_entry;
pub mod logger_message_type;
pub mod notification_handle;
pub mod return_codes;
pub mod state_flag;
pub mod string;
pub mod symbol;
pub mod trans_mode;

pub use command::AdsCommand;
pub use device_state::{AdsState, DeviceState};
pub use device_version::AdsDeviceVersion;
pub use error::{
    AdsCommandError, AdsDeviceVersionError, AdsError, AdsHeaderError, AdsNotificationHandleError,
    AdsReturnCodeError, AdsStateError, AdsStringError, AdsTransModeError, LogEntryError,
    LogMessageTypeError, StateFlagError, WindowsFileTimeError,
};
pub use filetime::WindowsFileTime;
pub use header::AdsHeader;
pub use log_entry::LogEntry;
pub use logger_message_type::LogMessageType;
pub use notification_handle::NotificationHandle;
pub use return_codes::AdsReturnCode;
pub use state_flag::StateFlag;
pub use string::AdsString;
pub use symbol::{
    AdsAttribute, AdsDataTypeArrayInfo, AdsDataTypeCategory, AdsDataTypeFlags, AdsDataTypeId,
    AdsDataTypeInfo, AdsDataTypeIterator, AdsDataTypeIteratorOwned, AdsEnumInfo, AdsMethodFlags,
    AdsMethodInfo, AdsMethodParamFlags, AdsMethodParamInfo, AdsMethodReturnTypeInfo,
    AdsRefactorInfo, AdsSymbolUploadFlags, AdsSymbolUploadInfo, AdsSymbolUploadInfoV1,
    AdsSymbolUploadInfoV2, AdsSymbolUploadInfoV3, Guid,
};
pub use trans_mode::AdsTransMode;

pub type IndexGroup = u32;
pub type IndexOffset = u32;

pub type InvokeId = u32;
