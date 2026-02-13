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
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsCommandError {
    #[error("Unexpected length: expected {expected} bytes, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}
