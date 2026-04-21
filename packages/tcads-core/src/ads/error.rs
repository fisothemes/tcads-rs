use crate::ads::StateFlag;

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsError {
    /// Invalid ADS header format or content
    #[error("Invalid ADS header: {0}")]
    InvalidAdsHeader(#[from] AdsHeaderError),
    /// Invalid ADS return code format or content.
    /// This is not the [AdsReturnCode](super::AdsReturnCode).
    /// Just errors to do with formatting into a valid return code.
    #[error("Invalid ADS return code: {0}")]
    InvalidAdsReturnCode(#[from] AdsReturnCodeError),
    /// Invalid AMS state flag format or content
    #[error("Invalid AMS state flag: {0}")]
    InvalidStateFlag(#[from] StateFlagError),
    /// Invalid ADS state format or content.
    #[error("Invalid ADS state: {0}")]
    InvalidAdsState(#[from] AdsStateError),
    /// Invalid ADS notification transmission mode format or content.
    #[error("Invalid ADS notification transmission mode: {0}")]
    InvalidAdsTransMode(#[from] AdsTransModeError),
    #[error("Invalid ADS notification attribute: {0}")]
    InvalidAdsNotificationAttrib(#[from] AdsNotificationAttribError),
    /// Invalid command format or content
    #[error("Invalid ADS command: {0}")]
    InvalidCommand(#[from] AdsCommandError),
    /// Invalid ADS device version format or content.
    #[error("Invalid ADS device version: {0}")]
    InvalidAdsDeviceVersion(#[from] AdsDeviceVersionError),
    /// Invalid ADS string format or content.
    #[error("Invalid ADS string: {0}")]
    InvalidAdsString(#[from] AdsStringError),
    /// Invalid ADS notification handle format or content.
    #[error("Invalid ADS notification handle: {0}")]
    InvalidAdsNotificationHandle(#[from] AdsNotificationHandleError),
    /// Invalid Windows file time format or content.
    #[error("Invalid Windows file time: {0}")]
    InvalidWindowsFileTime(#[from] WindowsFileTimeError),
    /// Invalid ADS log message type format or content.
    #[error("Invalid ADS log message type: {0}")]
    InvalidLogMessageType(#[from] LogMessageTypeError),
    #[error("Invalid ADS log entry: {0}")]
    InvalidLogEntry(#[from] LogEntryError),
    /// Invalid ADS symbol upload info format or content.
    #[error("Invalid ADS symbol upload info: {0}")]
    InvalidAdsSymbolUploadInfo(#[from] AdsSymbolUploadInfoError),
    /// Invalid ADS type info format or content.
    #[error("Invalid ADS type info: {0}")]
    InvalidAdsTypeInfo(#[from] AdsTypeInfoError),
    /// Invalid ADS data type format or content.
    #[error("Invalid GUID: {0}")]
    InvalidGuid(#[from] GuidParseError),
    #[error("Invalid SumUp command: {0}")]
    InvalidSumUpCommand(#[from] SumUpError),
    /// Invalid ADS data length format or content (not header or return code).
    #[error("Unexpected data length: expected {expected} bytes, got {got} bytes")]
    UnexpectedDataLength { expected: usize, got: usize },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsHeaderError {
    #[error("Unexpected length: expected {expected} bytes, got {got} bytes")]
    UnexpectedLength { expected: usize, got: usize },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsReturnCodeError {
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum StateFlagError {
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
    #[error("Unexpected state flag: expected one of {expected:?}, got {got:?}")]
    UnexpectedStateFlag {
        expected: Vec<StateFlag>,
        got: StateFlag,
    },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsStateError {
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsDeviceVersionError {
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsTransModeError {
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsNotificationAttribError {
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsCommandError {
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

/// Error returned when parsing an AdsString fails.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsStringError {
    #[error("Invalid length: expected {expected} bytes, got {got}")]
    TooLong { expected: usize, got: usize },
    #[error("String contains characters not supported by Windows-1252 encoding")]
    EncodingError,
    #[error("Invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("Invalid ADS string: {0}")]
    Other(String),
}

/// Error returned when parsing an [NotificationHandle](super::NotificationHandle) fails.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsNotificationHandleError {
    #[error("unexpected length: expected {expected}, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

/// Error returned when parsing a Windows file time fails.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum WindowsFileTimeError {
    #[error("unexpected length: expected {expected}, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

/// Error returned when parsing a [LogMessageType](super::LogMessageType) fails.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum LogMessageTypeError {
    #[error("unexpected length: expected {expected}, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

/// Error returned when parsing a [LogEntry](super::LogEntry) fails.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum LogEntryError {
    #[error("Payload too short: expected at least {expected} bytes, got {got}")]
    PayloadTooShort { expected: usize, got: usize },
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

/// Error returned when parsing [`AdsSymbolUploadInfo`](super::AdsSymbolUploadInfo) fails.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsSymbolUploadInfoError {
    #[error("Payload too short: expected at least {expected} bytes, got {got}")]
    TooShort { expected: usize, got: usize },
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}

/// Error returned when parsing an [`AdsTypeInfo`](super::AdsTypeInfo) entry fails.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsTypeInfoError {
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
    #[error("Payload too short: expected at least {expected} bytes, got {got}")]
    TooShort { expected: usize, got: usize },
    #[error("Entry length mismatch: entry claims {expected} bytes but only {got} available")]
    EntryLengthMismatch { expected: usize, got: usize },
    #[error("Invalid UTF-8 in field '{field}'")]
    InvalidUtf8 { field: &'static str },
    #[error("Invalid GUID: {0}")]
    InvalidGuid(#[from] GuidParseError),
}

/// Error returned when parsing a [`Guid`](super::Guid) fails.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum GuidParseError {
    #[error("Invalid GUID length: expected 32 hex characters (16 bytes), got {0}")]
    InvalidLength(usize),
    #[error("Invalid hexadecimal characters in GUID")]
    InvalidHex,
}

/// Error returned when parsing a SumUp command fails.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum SumUpError {
    #[error("Payload too short: expected at least {expected} bytes, got {got}")]
    TooShort { expected: usize, got: usize },
    #[error("Invalid header: expected {expected} bytes, got {got}")]
    HeaderTooShort { expected: usize, got: usize },
    #[error("Invalid payload: expected at least {expected} bytes, got {got}")]
    PayloadTooShort { expected: usize, got: usize },
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}
