//! # TwinCAT ADS for Rust
//!
//! A rust-native implementation of the TwinCAT ADS protocol.
//!
//! This crate aims to provide a robust way to communicate with TwinCAT devices (PLCs, NC, etc.),
//! without relying on the official Beckhoff `TcAdsDll.dll` or requiring a local TwinCAT
//! installation on the client machine.
//!
//! This crate is composed of the following sub-crates:
//!
//! - [`core`] - Protocol primitives, serialization, and frame I/O
//! - [`client`] - High-level connection and request management for ADS devices.
//! - [`server`] - Framework for building custom ADS servers/devices in Rust.

pub use tcads_client as client;
pub use tcads_core as core;
pub use tcads_server as server;
