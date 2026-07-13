//! # TwinCAT ADS Serde
//!
//! This crate provides a [`serde`](https://serde.rs) `Serializer`/`Deserializer` pair for
//! **TwinCAT ADS** byte layouts, driven entirely by the PLC's own type metadata
//! ([`AdsTypeInfo`](tcads_core::AdsTypeInfo)). Give it any `#[derive(serde::Serialize, serde::Deserialize)]` type and the
//! type information for a PLC symbol, and it reads and writes the exact byte layout TwinCAT
//! expects.
//!
//! For situations where you don't have, or don't want, a static Rust type, the crate also
//! provides `Value`: a dynamically typed value that mirrors whatever PLC data you read
//! without losing width or shape information.
//!
//! ## Features
//!
//! - Read and write any `#[derive(serde::Serialize, serde::Deserialize)]` type directly
//!   against raw ADS bytes: primitives, structs, tuples, tuple structs, arrays, `Option`
//!   (with a caveat, see below), and unit-only enums.
//! - Struct fields match the PLC type by declared position, not by name.
//! - Arrays of any dimension are supported, including nested (`ARRAY[*] OF ARRAY[*]`) and
//!   multidimensional (`ARRAY[*, *]`) declarations, of primitives, strings, structs, or
//!   enums.
//! - `STRING`/`WSTRING` are decoded and encoded automatically (Windows-1252 and UTF-16LE
//!   respectively), with zero-copy reads for ASCII `STRING` values.
//! - Aliases resolve automatically. A `TYPE Temperature : LREAL; END_TYPE` reads and writes
//!   exactly like a plain `LREAL`.
//! - `Value`'s numeric types remember whether they came from a `BYTE` or a `DWORD`, so
//!   writing one back validates against the same field width it was read from.
//! - Type metadata comes from the `TypeProvider` trait, so you can plug in your own source.
//!   `AdsTypeCache` is included as a ready-made in-memory implementation.
//!
//! ## Getting Around
//!
//! This crate is organized into modules dealing with specific serialization concerns:
//!
//! - **The Serialization Engine ([`de`] and [`ser`]):** Contains the core `AdsDeserializer`
//!   and `AdsSerializer` implementations.
//! - **Dynamic Typing ([`value`]):** The [`Value`] enum and high-precision `Number`
//!   representations for untyped PLC memory exploration.
//! - **Type Resolution ([`TypeProvider`] & [`AdsTypeCache`]):** The bridge for injecting PLC
//!   metadata during the parse phase without blocking the runtime.
//!
//! ## Getting Started
//!
//! ### Reading a symbol into a typed struct
//!
//! Map your PLC structures to native Rust structs. You don't need to worry about `#[repr(C)]`
//! or manually padding the fields. Function block headers are automatically skipped.
//!
//! ```rust, no_run
//! use tcads_core::AdsTypeInfo;
//! use tcads_serde::AdsTypeCache;
//!
//! #[derive(Debug, serde::Deserialize)]
//! struct MotorState {
//!     velocity: f64,
//!     is_active: bool,
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let raw_plc_bytes = [0u8; 9];
//! # let type_info = AdsTypeInfo::try_from_slice(&[])?.0;
//! # let provider = AdsTypeCache::new(8);
//! // In a real application, type_info and raw_plc_bytes are fetched via tcads-client
//! let state: MotorState = tcads_serde::from_bytes(
//!     &raw_plc_bytes,
//!     &type_info,
//!     &provider
//! )?;
//!
//! println!("Motor velocity: {:#?}", state.velocity);
//! # Ok(())
//! # }
//! ```
//!
//! ### Writing a value back
//!
//! Use `to_vec` or `to_bytes` to pack a Rust structure into TwinCAT memory boundaries. As with
//! reading, you don't need to worry about `#[repr(C)]` or manually padding the fields.
//!
//! ```rust,no_run
//! use serde::Serialize;
//! # use tcads_core::AdsTypeInfo;
//! # use tcads_serde::AdsTypeCache;
//!
//! #[derive(Serialize)]
//! struct PositionCommand {
//!     target: f32,
//!     execute: bool,
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let type_info = AdsTypeInfo::try_from_slice(&[])?.0;
//! # let provider = AdsTypeCache::new(8);
//! let cmd = PositionCommand { target: 150.5, execute: true };
//!
//! // Calculates padding and packs into the layout expected by the PLC
//! let write_buf = tcads_serde::to_vec(&cmd, &type_info, &provider)?;
//!
//! // Write `write_buf` back to the PLC...
//! # Ok(())
//! # }
//! ```
//!
//! ### Reading into a dynamic `Value`
//!
//! If you do not know the layout of the PLC memory at compile time, you can parse it
//! dynamically. This is excellent for building generic web dashboards with frameworks like
//! [`Dioxus`](https://dioxuslabs.com/), [`Leptos`](https://www.leptos.dev/) or [`Yew`](https://yew.rs/).
//!
//! ```rust,no_run
//! use tcads_serde::Value;
//! # use tcads_core::AdsTypeInfo;
//! # use tcads_serde::AdsTypeCache;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let raw_plc_bytes = [0u8; 9];
//! # let type_info = AdsTypeInfo::try_from_slice(&[])?.0;
//! # let provider = AdsTypeCache::new(8);
//! // Parse an unknown PLC structure dynamically
//! let dynamic_state: Value = tcads_serde::from_bytes(
//!     &raw_plc_bytes,
//!     &type_info,
//!     &provider
//! )?;
//!
//! // Because Value preserves the PLC field names, you can turn it into JSON
//! // `let json_payload = serde_json::to_string_pretty(&dynamic_state).unwrap();`
//! # Ok(())
//! # }
//! ```
//!
//! ### Arrays
//!
//! TwinCAT arrays (e.g. `ARRAY [1..3] OF LREAL`) map to both fixed-size Rust arrays (`[f64; 3]`)
//! and dynamic vectors (`Vec<f64>`). Multidimensional arrays are flattened based on their memory
//! stride.
//!
//! ### Enums
//!
//! TwinCAT `ENUM` types map strictly to Serde unit variants (variants without payloads).
//! Because PLC enums are integer-backed constants with string metadata, `tcads-serde` uses
//! the string name from the `AdsTypeInfo` metadata to match your Rust enum variants.
//!
//! ## Renaming and skipping fields
//!
//! A couple of serde's field and variant attributes are worth knowing about, since how they
//! behave here depends on the position-based matching described above.
//!
//! `#[serde(rename = "...")]` on an enum variant works exactly as you'd expect, since enum
//! variants are already matched by name:
//!
//! ```rust, no_run
//! #[derive(Debug, serde::Serialize, serde::Deserialize)]
//! enum State {
//!     #[serde(rename = "eIdle")]
//!     Idle,
//!     #[serde(rename = "eRunning")]
//!     Running,
//! }
//! ```
//!
//! Struct fields work differently. Since matching is positional, `#[serde(rename = "...")]`
//! on a struct field has no effect; matching depends only on position and type, never on a
//! name. `#[serde(skip)]` still behaves as you'd hope, though: a skipped field is removed
//! from the sequence entirely rather than leaving a gap behind, so you can add a Rust-only
//! field anywhere in the struct without disturbing the PLC fields around it:
//!
//! ```rust, no_run
//! # use serde::*;
//! #[derive(Debug, serde::Serialize, serde::Deserialize)]
//! struct Motor {
//!     speed: f32,
//!     #[serde(skip)]
//!     last_seen: Option<std::time::Instant>,
//!     running: bool,
//! }
//! ```
//!
//! If you want to address struct fields by their actual PLC name instead, for example
//! because your Rust and PLC field orders have drifted apart, deserialize into `Value`
//! first. `Value::Struct` keeps the PLC's real field names as map keys, and from there you
//! can bridge into a renamed, name-matched struct through any ordinary serde format:
//!
//! ```rust, no_run
//! # use tcads_serde::*;
//! # use tcads_core::*;
//! #[derive(Debug, serde::Serialize, serde::Deserialize)]
//! struct LibVersion {
//!     #[serde(rename = "iMajor")]
//!     major: u16,
//!     #[serde(rename = "iMinor")]
//!     minor: u16,
//!     #[serde(rename = "sVersion")]
//!     version_string: String,
//! }
//! # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//! # let bytes = [0u8; 1];
//! # let type_info = tcads_core::AdsTypeInfo::try_from_slice(&[])?.0;
//! # let provider = AdsTypeCache::new(8);
//! let value: tcads_serde::Value = tcads_serde::from_bytes(&bytes, &type_info, &provider)?;
//! let version: LibVersion = serde_json::from_value(serde_json::to_value(&value)?)?;
//! # Ok(())
//! # }
//! ```
//!
//! This costs an extra allocation and a JSON round trip (add `serde_json` as a dependency to
//! use it), so it's worth reaching for only when position-based matching genuinely doesn't
//! fit.

/// The deserialization engine. Maps raw PLC bytes into native Rust types.
pub mod de;

/// Error types specific to memory-mapping, padding, and dynamic type resolution.
pub mod error;

/// Resolves IEC 61131-3 `ALIAS` types recursively down to their base memory footprints.
pub mod resolvers;

/// The serialization engine. Packs native Rust types perfectly into TwinCAT memory layouts.
pub mod ser;

/// A ready-to-use, in-memory implementation of the `TypeProvider` trait.
pub mod type_cache;

/// Defines the `TypeProvider` trait required to synchronously supply PLC metadata during a parse.
pub mod type_provider;

/// Memory boundary checks and type validation to guarantee safe slicing and prevent out-of-bounds panics.
pub mod validators;

/// Dynamic typing and high-precision numeric enums for parsing unknown PLC memory layouts.
pub mod value;

pub use de::{AdsDeserializer, from_bytes};
pub use error::{Error, Result};
pub use ser::{AdsSerializer, to_bytes, to_vec};
pub use type_cache::AdsTypeCache;
pub use type_provider::TypeProvider;
pub use value::{Float, Integer, Number, SignedInteger, UnsignedInteger, Value};
