use tcads_core::errors::{AdsError, AdsReturnCode};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Network Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ADS Protocol Error: {0}")]
    Protocol(#[from] AdsError),
    #[error("Target returned ADS Error: {0:?}")]
    Ads(AdsReturnCode),
    #[error("Operation Timed Out")]
    Timeout,
    #[error("Connection Closed")]
    ConnectionClosed,
    #[error("Unexpected response type")]
    UnexpectedResponse,
}

pub type Result<T> = std::result::Result<T, ClientError>;
