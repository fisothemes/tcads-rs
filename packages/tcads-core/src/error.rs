use crate::ams::AmsError;

/// Main error type for the crate
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("AMS error: {0}")]
    /// Errors specific to AMS protocol
    Ams(#[from] AmsError),
    // #[error("Protocol error: {0}")]
    // Protocol(#[from] ProtocolError),
}
