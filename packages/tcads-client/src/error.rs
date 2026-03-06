use std::io;
use std::sync::{Arc, PoisonError};
use tcads_core::ads::AdsReturnCode;
use tcads_core::protocol::ProtocolError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(Arc<io::Error>),
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

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(Arc::new(err))
    }
}
