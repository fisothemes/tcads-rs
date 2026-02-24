//! # TwinCAT ADS Core
//!
//! This crate contains the core building blocks for the **TwinCAT AMS/ADS** protocol.
//!
//! It provides a high-performance, transport-agnostic implementation of the
//! Beckhoff ADS protocol. It is designed to handle the complexities of
//! byte-level frame construction and parsing while remaining flexible enough to
//! integrate with any I/O layer.
//!
//! ## Getting Around
//!
//! This crate is organised into layers that mirror the protocol stack:
//!
//! - **The AMS Layer ([`ams`]):** Handles network addressing ([`AmsNetId`]),
//!     routing logic, and AMS-specific commands like port connection.
//! - **The ADS Layer ([`ads`]):** Contains the protocol primitives, including
//!     command IDs, device states, and error codes.
//! - **The Protocol Layer ([`protocol`]):** Provides strongly typed Request and
//!     Response structures for every ADS command (e.g., `Read`, `Write`, `AddNotification`).
//! - **The I/O Layer ([`io`]):** Defines the [`AmsFrame`]—the primary container
//!     for wire communication—and provides concrete streams for both blocking
//!     and async (Tokio) runtimes.
//!
//! ## Memory Efficiency: Borrowed vs. Owned
//!
//! A key feature of this crate is its focus on zero-copy parsing. Most data-heavy
//! protocol types come in two variants:
//!
//! 1.  **Borrowed (`'a`):** Types like [`AdsReadResponse<'a>`](protocol::AdsReadResponse)
//!     slice directly into the read buffer, performing no allocations for the payload.
//! 2.  **Owned:** Types like [`AdsReadResponseOwned`](protocol::AdsReadResponseOwned)
//!     take ownership of their data, making them suitable for long-term storage or passing
//!     across thread boundaries.
//!
//! You can move between these representations using `.into_owned()` or `.as_view()`
//! where applicable.
//!
//! ## Transport Agnosticism
//!
//! While this crate includes TCP-based stream implementations in the [`io`] module,
//! the core logic only requires types that implement standard traits. You can
//! introduce any blocking or async transportation layer required by your
//! specific hardware environment.
//!
//! ## Getting Started
//!
//! ### Low-level Frame Communication
//!
//! For direct control, you can work with raw [`AmsFrame`] objects over a stream.
//!
//! ```rust,no_run
//! use tcads_core::ams::AmsCommand;
//! use tcads_core::io::{AmsFrame, blocking::AmsStream};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let stream = AmsStream::connect("127.0.0.1:48898")?;
//! let (reader, mut writer) = stream.try_split()?;
//!
//! // Send a raw "Port Connect" command
//! let frame = AmsFrame::new(AmsCommand::PortConnect, [0x00, 0x00]);
//! writer.write_frame(&frame)?;
//!
//! // Read the response
//! let frame = reader.read_frame()?;
//! println!("Received: {:?}", frame.header().command());
//! # Ok(())
//! # }
//! ```
//!
//! The async API is identical in shape, just swap the import and add `.await`:
//!
//! ```rust,no_run
//! use tcads_core::ams::AmsCommand;
//! use tcads_core::io::{AmsFrame, tokio::AmsStream};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let stream = AmsStream::connect("127.0.0.1:48898")?;
//! let (reader, mut writer) = stream.into_split()?;
//!
//! let frame = AmsFrame::new(AmsCommand::PortConnect, [0x00, 0x00]);
//! writer.write_frame(&frame).await?;
//!
//! let frame = reader.read_frame().await?;
//! println!("Received: {:?}", frame.header().command());
//! # Ok(())
//! # }
//! ```
//!
//! ### High-level Protocol Logic
//!
//! Use the [`protocol`] module for type-safe interactions without manual byte-shuffling.
//!
//! ```rust,no_run
//! use tcads_core::io::blocking::AmsStream;
//! use tcads_core::protocol::PortConnectRequest;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let stream = AmsStream::connect("127.0.0.1:48898")?;
//! let (_, mut writer) = stream.try_split()?;
//!
//! // Construct a typed request and convert it to a wire-ready frame
//! let request = PortConnectRequest::default();
//! writer.write_frame(&request.into_frame())?;
//! # Ok(())
//! # }
//! ```

/// ADS protocol primitives and wire-format types.
///
/// This module contains the fundamental enums and constants defined by the Beckhoff
/// specification, including [`AdsState`], [`AdsReturnCode`], and [`AdsTransMode`].
/// It also provides helper types for ADS-specific data like [`AdsString`] and
/// [`WindowsFileTime`].
pub mod ads;

/// AMS layer addressing and router management.
///
/// Handles the outer layer of the protocol: network addressing via [`AmsNetId`]
/// and [`AmsAddr`], as well as the specialised commands used to communicate
/// with the AMS Router itself (e.g. [`PortConnect`](AmsCommand::PortConnect)).
pub mod ams;

/// Frame I/O and transport implementations.
///
/// Defines the [`AmsFrame`] for all wire communication and provides concrete `AmsStream` implementations
/// for both [blocking](io::blocking::AmsStream) and [tokio-based](io::tokio::AmsStream) async I/O.
pub mod io;

/// High-level, type-safe Request and Response definitions.
///
/// This is the primary entry point for building clients or servers. Every ADS
/// command has a corresponding pair of structs here that handle the byte-level
/// math of the protocol. Most types here follow the "Borrowed vs Owned" pattern
/// to allow for zero-copy parsing directly from the wire.
pub mod protocol;

pub use ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, AdsState, AdsTransMode, IndexGroup,
    IndexOffset, WindowsFileTime,
};
pub use ams::{AmsAddr, AmsCommand, AmsNetId, AmsPort, AmsTcpHeader};
pub use io::AmsFrame;
