#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsError {
    #[error("Buffer too small for ADS Header: expected {expected}, found {found}")]
    HeaderBufferTooSmall { expected: usize, found: usize },

    #[error("Buffer too small for {item}: expected {expected}, found {found}")]
    InvalidBufferSize {
        item: &'static str,
        expected: usize,
        found: usize,
    },
    /// Invalid command format or content
    #[error("Invalid ADS command: {0}")]
    InvalidCommand(#[from] AdsCommandError),
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsReturnCodeError {
    /// Unexpected buffer size
    #[error("Invalid buffer size: expected {expected} bytes, got {got}")]
    InvalidBufferSize { expected: usize, got: usize },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum StateFlagError {
    /// Unexpected buffer size
    #[error("Invalid buffer size: expected {expected} bytes, got {got}")]
    InvalidBufferSize { expected: usize, got: usize },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsCommandError {
    /// Unexpected buffer size
    #[error("Invalid buffer size: expected {expected} bytes, got {got}")]
    InvalidBufferSize { expected: usize, got: usize },
}
