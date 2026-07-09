use crate::Integer;
use std::fmt;
use tcads_core::ads::{AdsSymbolInfoError, AdsTypeInfoError};

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Type mismatch: expected {expected}")]
    TypeMismatch { expected: String },
    #[error(
        "Shape mismatch: Rust struct expected {expected} fields, but PLC type has {got} fields"
    )]
    ShapeMismatch { expected: usize, got: usize },
    #[error("Size mismatch: data buffer has {got} bytes, but PLC type requires {expected} bytes")]
    SizeMismatch { expected: usize, got: usize },
    #[error("Type not found in provider: {0}")]
    TypeNotFound(String),
    #[error("Invalid byte length for type. Got {0} bytes.")]
    InvalidByteLength(usize),
    #[error("Unsupported ADS type: {0:?}")]
    UnsupportedType(tcads_core::ads::AdsTypeId),
    #[error("Enum discriminant {0} not found in type {1}")]
    UnknownEnumDiscriminant(Integer, String),
    #[error("Enum variant '{0}' not found in type {1}")]
    UnknownUnknownEnumVariant(String, String),
    #[error("I/O or encoding error: {0}")]
    Custom(String),
    #[error(transparent)]
    TypeInfo(#[from] AdsTypeInfoError),
    #[error(transparent)]
    SymbolInfo(#[from] AdsSymbolInfoError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}
