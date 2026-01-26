#![doc = include_str!("../README.md")]

/// TCP framing and low-level byte stream handling.
pub mod codec;

/// Standard port numbers, header lengths, and size limits.
pub mod constants;

/// ADS Return Codes and library-specific error types.
pub mod errors;

/// The "must-have" imports for using this library.
pub mod prelude;

/// The core wire-format definitions.
pub mod protocol;

/// Rust wrappers for ADS primitives.
pub mod types;
