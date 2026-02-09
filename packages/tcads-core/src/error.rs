use crate::ams::AmsError;
use crate::protocol::ProtocolError;

/// Main error type for the crate
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("AMS error: {0}")]
    /// Errors specific to AMS core implementation
    Ams(#[from] AmsError),
    /// Errors specific to the AMS protocol
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}
