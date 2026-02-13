use super::ads::AdsError;
use crate::ams::{AmsCommand, AmsError};
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("AMS Error: {0}")]
    Ams(#[from] AmsError),
    #[error("ADS Error: {0}")]
    Ads(#[from] AdsError),
    #[error("Unexpected AMS Command: expected {expected:?}, got {got:?}")]
    UnexpectedCommand {
        expected: AmsCommand,
        got: AmsCommand,
    },
    #[error("Unexpected Length: expected {expected}, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
}
