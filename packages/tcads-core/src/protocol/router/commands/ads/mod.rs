//! Definition of ADS Command IDs and their Payload structures.

pub mod enums;
pub mod handles;
/// Definitions for the AMS/TCP framing and routing headers.
pub mod header;
pub mod id;
/// Reserved Index Groups for accessing system services and PLC memory areas.
pub mod index_groups;
/// The [`AmsAdsPacket`](packet::AmsAdsPacket) container, combining headers with the command payload.
pub mod packet;
pub mod request;
pub mod response;
/// Bitflags indicating the nature of the message (Request/Response) and transport attributes.
pub mod state_flags;

pub use enums::*;
pub use handles::*;
pub use id::*;
pub use request::*;
pub use response::*;
