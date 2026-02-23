#![doc = include_str!("../README.md")]

/// ADS protocol primitives - commands, states, error, strings, and
/// wire-format types like [`AdsState`], [`AdsReturnCode`], etc.
pub mod ads;

/// AMS layer - network addressing ([`AmsNetId`], [`AmsAddr`]) and the router
/// command types ([`PortConnect`](AmsCommand::PortConnect),
/// [`GetLocalNetId`](AmsCommand::GetLocalNetId), [`RouterNotification`](AmsCommand::RouterNotification)).
pub mod ams;

/// Frame I/O - [`AmsFrame`] construction and the blocking/async stream types
/// that read and write frames over TCP.
pub mod io;

/// Typed request and response structs for every ADS command. Start here if
/// you are building a client or server.
pub mod protocol;

pub use ads::{AdsCommand, AdsError, AdsHeader, AdsReturnCode, AdsState, AdsTransMode};
pub use ams::{AmsAddr, AmsCommand, AmsNetId, AmsPort, AmsTcpHeader};
pub use io::AmsFrame;
