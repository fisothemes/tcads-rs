use std::io;
use tcads_core::ads::AdsReturnCode;
use tcads_core::protocol::ProtocolError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    AdsReturnCode(#[from] AdsReturnCode),
    #[error("Disconnected")]
    Disconnected,
}

pub type Result<T> = std::result::Result<T, Error>;
