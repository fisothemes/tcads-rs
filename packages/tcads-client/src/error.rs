use std::io;
use std::sync::mpsc::{RecvError, RecvTimeoutError, SendError};
use std::sync::{Arc, PoisonError};
use tcads_core::ads::{AdsSymbolInfoError, AdsTypeInfoError};
use tcads_core::ams::{AddrError, NetIdError};
use tcads_core::{AdsError, AdsReturnCode, AmsError, ProtocolError};
#[cfg(feature = "tokio")]
use tokio::sync::mpsc::error::SendError as TokioSendError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] Arc<io::Error>),
    #[error(transparent)]
    AdsError(#[from] AdsError),
    #[error(transparent)]
    AmsError(#[from] AmsError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    AdsReturnCode(#[from] AdsReturnCode),
    #[error(transparent)]
    NetIdError(#[from] NetIdError),
    #[error(transparent)]
    AmsAddrError(#[from] AddrError),
    #[error(transparent)]
    AdsTypeInfoError(#[from] AdsTypeInfoError),
    #[error(transparent)]
    AdsSymbolInfoError(#[from] AdsSymbolInfoError),
    #[error("Disconnected")]
    Disconnected,
    #[error("Timed out")]
    Timeout,
    #[error("Poisoned lock")]
    PoisonedLock,
    #[error("Invalid payload")]
    InvalidPayload,
    #[error(transparent)]
    Serde(#[from] tcads_serde::Error),
    #[error("Handle for symbol '{0}' is no longer valid")]
    HandleInvalidated(Arc<str>),
    #[error("method '{method_name}' not found on type '{type_name}'")]
    MethodNotFound {
        type_name: Arc<str>,
        method_name: Arc<str>,
    },
    #[error("method '{method_name}' exists but is not callable over ADS")]
    MethodNotCallable { method_name: Arc<str> },
}

pub type Result<T> = std::result::Result<T, Error>;

impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Self {
        Error::PoisonedLock
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(Arc::new(err))
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
        Error::Disconnected
    }
}

impl From<RecvError> for Error {
    fn from(_: RecvError) -> Self {
        Error::Disconnected
    }
}

impl From<RecvTimeoutError> for Error {
    fn from(err: RecvTimeoutError) -> Self {
        match err {
            RecvTimeoutError::Timeout => Error::Timeout,
            RecvTimeoutError::Disconnected => Error::Disconnected,
        }
    }
}

#[cfg(feature = "tokio")]
impl<T> From<TokioSendError<T>> for Error {
    fn from(_: TokioSendError<T>) -> Self {
        Error::Disconnected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn io_error_converts_and_clones() {
        let err = Error::from(io::Error::new(io::ErrorKind::ConnectionRefused, "refused"));
        let cloned = err.clone();
        assert!(matches!(cloned, Error::Io(_)));
        assert!(err.to_string().contains("refused"));
    }

    #[test]
    fn poison_error_converts() {
        let mutex = Mutex::new(0u32);
        let _ = std::panic::catch_unwind(|| {
            let _guard = mutex.lock().unwrap();
            panic!("poison the lock");
        });
        let err = Error::from(mutex.lock().unwrap_err());
        assert!(matches!(err, Error::PoisonedLock));
    }
}
