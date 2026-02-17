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
    /// Invalid command format or content
    #[error("Invalid ADS command: {0}")]
    InvalidCommand(#[from] AdsCommandError),
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
pub enum AdsCommandError {
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}
