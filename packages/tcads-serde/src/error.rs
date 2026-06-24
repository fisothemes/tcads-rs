#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Shape mismatch: expected {expected} fields, but PLC struct has {got}")]
    ShapeMismatch { expected: usize, got: usize },
    #[error("Type not found in cache: {0}")]
    TypeNotFound(String),
    #[error("Invalid byte length for primitive. Got {0} bytes.")]
    InvalidByteLength(usize),
    #[error("Serde error: {0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}
