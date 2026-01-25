//! The prelude module.
//!
//! Import this module to get the most common types and traits:
//!
//! ```rust
//! use tcads_core::prelude::*;
//! ```

// 1. Essential Types (Addressing & Primitives)
pub use crate::types::{AdsString, AmsAddr, AmsNetId, AmsPort, WindowsFiletime};

// 2. Protocol Basics
pub use crate::protocol::commands::{AdsState, AdsTransMode, CommandId, NotificationHandle};
pub use crate::protocol::state_flags::StateFlag;

// 3. Packet & Header (Essential for building messages)
pub use crate::protocol::header::{AmsHeader, AmsTcpHeader};
pub use crate::protocol::packet::AmsPacket;

// 4. Errors (You always need these)
pub use crate::errors::{AdsError, AdsReturnCode};

// 5. Codec (Useful if they are implementing their own transport)
pub use crate::codec::AmsCodec;

// Note: We usually DO NOT re-export specific Request/Response structs here (like AdsReadRequest)
// because there are too many of them. Users should usually import specific commands
// or use `tcads_core::protocol::commands::*` if they really want them all.
