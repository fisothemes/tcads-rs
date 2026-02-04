//! Binary wire format definitions for ADS/AMS communication.
//!
//! This module provides the low-level structs and enums that map directly to the byte streams
//! exchanged with ADS devices. It covers the main layers of an AMS message:
//!
//! * **Framing**: The `packet` and `header` modules handle the AMS Packet structure and Routing Header.
//! * **Payloads**: The `commands` module defines the specific data layouts for operations like Read, Write, or Device Info.
//! * **Metadata**: Helper types like `state_flags` and `index_groups` provide constants and bitmasks required for valid communication.
//!
//! These types are transport-agnostic; they describe *what* is sent, not *how* it is sent (TCP vs UDP).
//!
//! # Example
//!
//! Constructing a raw AMS packet to read 4 bytes from a device:
//!
//! ```rust
//! use tcads_core::protocol::{
//!     //! //!     ads::{CommandId, AdsReadRequest},
//!     //! };
//! use tcads_core::types::{AmsAddr, AmsNetId};
//! use tcads_core::errors::AdsReturnCode;
//! use std::io::Write;
//!
//! // 1. Prepare the Payload (Read 4 bytes from IndexGroup 0x4020, Offset 0)
//! let request = AdsReadRequest::new(0x4020, 0, 4);
//! let mut payload = Vec::new();
//! request.write_to(&mut payload).unwrap();
//!
//! // 2. Prepare the Header
//! let header = AmsHeader::new(
//!     AmsAddr::new(AmsNetId([5, 1, 2, 3, 1, 1]), 851),        // Target
//!     AmsAddr::new(AmsNetId([192, 168, 0, 10, 1, 1]), 30000), // Source
//!     CommandId::AdsRead,
//!     StateFlag::tcp_ads_request(),
//!     payload.len() as u32,
//!     AdsReturnCode::Ok,
//!     0,
//! );
//!
//! // 3. Assemble the Packet
//! let packet = AmsAdsPacket::new(header, payload);
//! ```

pub mod router;
pub mod tcp;
