use std::io;
use std::sync::PoisonError;
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
    #[error("Poisoned Lock: {0}")]
    PoisonedLock(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl<T> From<PoisonError<T>> for Error {
    fn from(err: PoisonError<T>) -> Self {
        Error::PoisonedLock(err.to_string())
    }
}
