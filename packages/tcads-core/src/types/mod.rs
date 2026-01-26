//! Rust representations of ADS-specific data types.
//!
//! This module provides type-safe wrappers for the primitive data types used in the ADS protocol.
//! It ensures correct serialization and handles the conversion between Rust's native types
//! (like [`std::time::SystemTime`](std::time::SystemTime) or `utf-8` strings) and the binary formats expected by the PLC.
//!
//! # Common Types
//!
//! * **Addressing**: `AmsNetId` and `AmsAddr` provide strong typing for network routing.
//! * **Strings**: `AdsString<N>` handles the fixed-length, Windows-1252 encoded strings common in IEC 61131-3.
//! * **Time**: `WindowsFiletime` bridges the gap between Rust's `SystemTime` and the 64-bit tick count used by Windows/TwinCAT.
//!
//! # Example
//!
//! ```rust
//! use tcads_core::types::{AmsNetId, AdsString, WindowsFiletime};
//! use std::str::FromStr;
//! use std::time::SystemTime;
//!
//! // 1. Parsing Network IDs
//! let netid = AmsNetId::from_str("5.1.2.3.1.1").unwrap();
//!
//! // 2. Handling Legacy Strings (Windows-1252)
//! // Maps to STRING(80) in PLC (requires 81 bytes for null terminator)
//! let s: AdsString<81> = AdsString::try_from("Hello TwinCAT").unwrap();
//! assert_eq!(s.as_str(), "Hello TwinCAT");
//!
//! // 3. Working with Time
//! let now = SystemTime::now();
//! let filetime = WindowsFiletime::from(now);
//! ```

pub mod addr;
pub mod filetime;
pub mod netid;
pub mod string;

pub use addr::*;
pub use filetime::*;
pub use netid::*;
pub use string::*;
