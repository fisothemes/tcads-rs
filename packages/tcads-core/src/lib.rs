//! # TwinCAT ADS Core
//!
//! This crate contains the core building blocks for the **TwinCAT AMS/ADS** protocol.
//!
//! It provides a transport-agnostic implementation of the Beckhoff ADS protocol. It is designed to
//! handle the complexities of byte-level frame construction and parsing without dictating the
//! underlying network or async runtime.
//!
//! ## Getting Around
//!
//! This crate is organised into layers that mirror the protocol stack:
//!
//! - **The AMS Layer ([`ams`]):** Handles network addressing ([`AmsNetId`]),
//!   routing logic, and AMS-specific commands like port connection.
//! - **The ADS Layer ([`ads`]):** Contains the protocol primitives, including
//!   command IDs, device states, and error codes.
//! - **The Protocol Layer ([`protocol`]):** Provides strongly typed Request and
//!   Response structures for every ADS command (e.g. `Read`, `Write`, `AddNotification`).
//! - **The Frame Container ([`frame`]):** Defines the [`AmsFrame`] which is the primary container
//!   for wire communication.
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
//! This crate is purely protocol logic and contains **no I/O primitives** (no `TcpStream`,
//! no `tokio`). To send and receive [`AmsFrame`]s over a network, you should pair this
//! crate with **`tcads-io`**, or implement your own transport layer.
//!
//! ## Getting Started
//!
//! ### Low-level Frame Construction
//!
//! For direct control, you can construct raw [`AmsFrame`] objects and serialize
//! them to a byte vector to send over any transport layer of your choosing.
//!
//! ```rust
//! use tcads_core::ams::AmsCommand;
//! use tcads_core::frame::AmsFrame;
//!
//! // Construct a raw "Port Connect" command
//! let frame = AmsFrame::new(AmsCommand::PortConnect, [0x00, 0x00]);
//!
//! // Serialize the frame to a contiguous byte vector
//! let network_bytes: Vec<u8> = frame.to_vec();
//! assert_eq!(network_bytes.len(), 8); // 6 byte header + 2 byte payload
//! ```
//!
//! ### High-level Protocol Logic
//!
//! Use the [`protocol`] module for type-safe interactions. Every request type
//! can be directly serialized into an [`AmsFrame`].
//!
//! ```rust
//! use tcads_core::protocol::PortConnectRequest;
//! use tcads_core::frame::AmsFrame;
//!
//! // Construct a typed request
//! let request = PortConnectRequest::default();
//!
//! // Convert it to a wire-ready frame
//! let frame: AmsFrame = request.into_frame();
//! ```
//!
//! ### Parsing Responses
//!
//! Borrowed response types slice directly into the frame buffer, performing
//! zero copies of the underlying data payload.
//!
//! ```rust
//! use tcads_core::protocol::AdsReadResponse;
//! # use tcads_core::frame::AmsFrame;
//! # fn parse_example(frame: &AmsFrame) -> Result<(), Box<dyn std::error::Error>> {
//!
//! // Parsed response borrows from `frame`, there is no copy of the data bytes
//! let response = AdsReadResponse::try_from(frame)?;
//! let value = i32::from_le_bytes(response.data().try_into()?);
//!
//! // Need to store it across threads? Convert explicitly
//! let owned = response.into_owned();
//! # Ok(())
//! # }
//! ```

/// ADS protocol primitives and wire-format types.
///
/// This module contains the fundamental enums and constants defined by the Beckhoff
/// specification, including [`AdsState`], [`AdsReturnCode`], and [`AdsTransMode`].
/// It also provides helper types for ADS-specific data like [`AdsString`](ads::AdsString) and
/// [`WindowsFileTime`].
pub mod ads;

/// AMS layer addressing and router management.
///
/// Handles the outer layer of the protocol: network addressing via [`AmsNetId`]
/// and [`AmsAddr`], as well as the specialized commands used to communicate
/// with the AMS Router itself (e.g. [`PortConnect`](AmsCommand::PortConnect)).
pub mod ams;

/// I/O Frame I/O for wire communication.
///
/// Defines the [`AmsFrame`] for all wire communication. This is transportation layer agnostic.
pub mod frame;

/// High-level, type-safe Request and Response definitions.
///
/// This is the primary entry point for building clients or servers. Every ADS
/// command has a corresponding pair of structs here that handle the byte-level
/// math of the protocol. Most types here follow the "Borrowed vs Owned" pattern
/// to allow for zero-copy parsing directly from the wire.
pub mod protocol;

pub use ads::{
    AdsArrayInfo, AdsAttribute, AdsCommand, AdsDeviceVersion, AdsEnumInfo, AdsError, AdsFieldInfo,
    AdsFileAttributes, AdsFileFlags, AdsFileHandle, AdsFilePathType, AdsFileSeekOrigin,
    AdsFileStatus, AdsHeader, AdsMethodFlags, AdsMethodInfo, AdsMethodParamFlags,
    AdsMethodParamInfo, AdsMethodReturnTypeInfo, AdsNotificationAttrib, AdsOsType, AdsPlatform,
    AdsProductVersion, AdsRefactorInfo, AdsReturnCode, AdsState, AdsSymbol2Flags, AdsSymbolFlags,
    AdsSymbolInfo, AdsSymbolInfoIterator, AdsSymbolInfoIteratorOwned, AdsSymbolUploadFlags,
    AdsSymbolUploadInfo, AdsSymbolUploadInfoV1, AdsSymbolUploadInfoV2, AdsSymbolUploadInfoV3,
    AdsSystemState, AdsSystemStateFlags, AdsTargetType, AdsTransMode, AdsTypeCategory,
    AdsTypeFlags, AdsTypeId, AdsTypeInfo, AdsTypeInfoIterator, AdsTypeInfoIteratorOwned,
    DeviceState, Guid, IndexGroup, IndexOffset, InvokeId, LogEntry, LogMessageType,
    NotificationHandle, SumAddNotificationIter, SumAddNotificationRequest,
    SumAddNotificationResponse, SumDeleteNotificationIter, SumDeleteNotificationResponse,
    SumReadRequest, SumReadResponse, SumReadResponseIter, SumReadResponseOwned,
    SumReadWriteRequest, SumReadWriteRequestOwned, SumReadWriteResponse, SumReadWriteResponseIter,
    SumReadWriteResponseOwned, SumWriteIter, SumWriteRequest, SumWriteResponse, SymbolHandle,
    WinRegistryValueType, WindowsFileTime,
};
pub use ams::{
    AmsAddr, AmsCommand, AmsError, AmsNetId, AmsPort, AmsTcpHeader, RouterState, RuntimeType,
};
pub use frame::{AMS_FRAME_MAX_LEN, AmsFrame};
pub use protocol::{AdsNotificationSampleOwned, ProtocolError};
