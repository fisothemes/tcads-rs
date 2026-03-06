use std::io;
use std::sync::mpsc::SendError;
use std::sync::{Arc, PoisonError};
use tcads_core::ads::AdsReturnCode;
use tcads_core::protocol::ProtocolError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
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

impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
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
        assert!(matches!(err, Error::PoisonedLock(_)));
    }
}
