use crate::ams::{AmsCommand, AmsError};
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("AMS Error: {0}")]
    Ams(#[from] AmsError),
    #[error("Unexpected AMS Command: expected {expected:?}, got {actual:?}")]
    UnexpectedCommand {
        expected: AmsCommand,
        actual: AmsCommand,
    },
    #[error("Unexpected Length: expected {expected}, got {actual}")]
    UnexpectedLength { expected: usize, actual: usize },
}
