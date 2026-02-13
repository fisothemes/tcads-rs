#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AdsError {
    #[error("Buffer too small for ADS Header: expected {expected}, found {found}")]
    HeaderBufferTooSmall { expected: usize, found: usize },

    #[error("Buffer too small for {item}: expected {expected}, found {found}")]
    InvalidBufferSize {
        item: &'static str,
        expected: usize,
        found: usize,
    },
}
