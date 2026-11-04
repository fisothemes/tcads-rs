use std::io;
use std::sync::Arc;
use tcads_core::ams::{AddrError, NetIdError};
use tcads_core::{AdsError, AdsReturnCode, AmsAddr, AmsError, AmsPort, ProtocolError};

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
    #[error(
        "AMS router closed the connection during PortConnect for port {port} \
        (the port is probably in use)"
    )]
    PortConnectClosed { port: AmsPort },
    #[error("AMS router did not answer PortConnect for port {port} in time")]
    PortConnectTimeout { port: AmsPort },
    #[error("requested port {requested}, but the AMS router assigned {assigned}")]
    PortMismatch {
        requested: AmsPort,
        assigned: AmsAddr,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(Arc::new(err))
    }
}
