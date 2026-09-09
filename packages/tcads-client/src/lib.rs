//! # TwinCAT ADS Client
//!
//! The high-level client layer for the **TwinCAT AMS/ADS** protocol.
//!
//! This crate takes the typed frames from `tcads-core`, pushes them over a `tcads-io` stream, and
//! gives you back device clients you can call from anywhere in your program: read a PLC variable
//! by name, subscribe to value changes, call a function block method, tail the TwinCAT log, or
//! drive the system service on the target IPC.
//!
//! ## Getting Around
//!
//! - **[`devices`]** - the device clients, split into a `blocking` and a `tokio` module that
//!   mirror each other:
//!   - `AdsDevice` owns the connection and exposes the raw ADS command set: read, write,
//!     read/write, read state, write control, device info, notifications, and the Sum (batch)
//!     variants of each.
//!   - `AdsRuntime` talks to a PLC runtime (ports 851, 801-899, 301-399). Symbols by name, values
//!     through `serde`, subscriptions, batch reads and writes, and RPC.
//!   - `AdsLogger` reads and writes the TwinCAT system log on port 100.
//!   - `AdsSystemService` drives the host machine on port 10000: TwinCAT mode, file I/O, host
//!     processes, the Windows registry, and the system clock.
//!   - `AdsSubsystem` is the trait the three subsystem clients share: `read_state`,
//!     `read_device_info`, `subscribe_state`, and `write_control` against their own target.
//! - **[`notif_guard`]** - RAII guards that delete a notification on the router when dropped.
//! - **[`tasks`]** - the reader, the writer, and the three dispatchers underneath `AdsDevice`.
//!   Public so you can build your own device client on an existing connection.
//! - **[`error`]** - [`Error`] and [`Result`], returned by every fallible call in the crate.
//!
//! ## Feature Flags
//!
//! | Flag       | Default | What it enables                                                   |
//! |------------|---------|-------------------------------------------------------------------|
//! | `blocking` | Yes     | [`std::net`] sockets and background threads (`devices::blocking`) |
//! | `tokio`    | No      | Tokio sockets and background tasks (`devices::tokio`)             |
//!
//! Both can be enabled at the same time. Moving a blocking program to async is mostly a matter of
//! adding `.await`.
//!
//! ## Connecting
//!
//! Every device client offers three ways in. `connect` and `connect_to` go through the local AMS
//! router. Installing XAE or XAR installs that router; it listens on port 48898 and carries every
//! ADS message, whether the target is on the same machine or across the network. `connect_local`
//! does the same but asks the router for the local Net ID first, which is what you want when the
//! TwinCAT target system is set to `<Local>`.
//!
//! `connect_remote` skips the local router and connects straight to the router on the target. Use
//! it on machines that cannot run XAE or XAR: a Linux or macOS development machine, a container,
//! or a bare Windows box. Your source Net ID has to be registered on the target as a static route
//! first. See [Connecting without a local AMS router][routes] for how to add the route and what to
//! do when the connection drops.
//!
//! [routes]: https://github.com/fisothemes/tcads-rs/blob/master/docs/static-routes.md
//!
//! ## Getting Started
//!
//! ### Reading and writing PLC symbols
//!
//! ```rust,no_run
//! use tcads_client::devices::blocking::AdsRuntime;
//! use tcads_core::AmsAddr;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to the first PLC runtime through the local AMS router
//!     let runtime = AdsRuntime::connect(AmsAddr::from_local(851), None)?;
//!
//!     // Symbols are resolved by instance path and cached after the first access
//!     let count: u32 = runtime.read_value("MAIN.nCount")?;
//!     runtime.write_value("MAIN.bIncrement", true)?;
//!
//!     // Several symbols in one round trip
//!     let mut batch = runtime.read_multi_values(["MAIN.nCount", "MAIN.fbCurrentRecipe.Id"])?;
//!     let count: u32 = batch.pop_front().unwrap()?;
//!     let recipe_id: String = batch.pop_front().unwrap()?;
//!
//!     // Call a method on a function block. Inputs and outputs follow a 0, 1, N rule:
//!     // `()` for none, the bare type for one, a tuple for several.
//!     let (quotient, remainder): (i32, i32) =
//!         runtime.rpc("MAIN.fbMath", "DivideValues", (100i32, 3i32))?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Subscribing to value changes
//!
//! ```rust,no_run
//! use std::time::Duration;
//! use tcads_client::Error;
//! use tcads_client::devices::blocking::AdsRuntime;
//! use tcads_core::{AdsTransMode, AmsAddr};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let runtime = AdsRuntime::connect(AmsAddr::from_local(851), None)?;
//!
//!     let (rx, handle) = runtime.subscribe_value::<u32>(
//!         "MAIN.nCount",
//!         AdsTransMode::ServerOnChange,
//!         Duration::ZERO,
//!         Duration::ZERO,
//!     )?;
//!
//!     for result in rx.iter().take(5) {
//!         match result {
//!             Ok(value) => println!("MAIN.nCount = {value}"),
//!             // An online change invalidated the handle behind the subscription
//!             Err(Error::HandleInvalidated(path)) => break,
//!             Err(err) => return Err(err.into()),
//!         }
//!     }
//!
//!     rx.unsubscribe()?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Sharing one connection
//!
//! Connect once, then wrap the same `AdsDevice` in as many subsystem clients as you need. They
//! share the socket, the reader, and the writer.
//!
//! ```rust,no_run
//! use std::time::Duration;
//! use tcads_client::devices::blocking::{AdsDevice, AdsLogger, AdsRuntime, AdsSystemService};
//! use tcads_core::{AmsAddr, AmsNetId, LogMessageType};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let device = AdsDevice::connect(Duration::from_secs(5))?;
//!
//!     let runtime = AdsRuntime::new(device.clone(), AmsAddr::from_local(851));
//!     let logger = AdsLogger::new(device.clone(), AmsNetId::local());
//!     let system = AdsSystemService::new(device.clone(), AmsNetId::local());
//!
//!     logger.write_log(LogMessageType::WARNING, "RustClient", "Starting up...")?;
//!     println!("TwinCAT version: {}", system.get_product_version()?);
//!
//!     // Dropping the last clone tears the connection down, or do it explicitly
//!     device.shutdown()?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Async
//!
//! ```rust,no_run
//! use tcads_client::devices::tokio::AdsRuntime;
//! use tcads_core::AmsAddr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let runtime = AdsRuntime::connect(AmsAddr::from_local(851), None).await?;
//!
//!     let count: u32 = runtime.read_value("MAIN.nCount").await?;
//!     runtime.write_value("MAIN.bIncrement", true).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! Async receivers take `&mut self` in `recv` and have no `recv_timeout` or `iter`. Wrap `recv` in
//! `tokio::time::timeout` and drive it from a loop instead.
//!
//! ## The Connection Model
//!
//! Connecting splits the socket into a reader half and a writer half, then spawns one background
//! worker for each: threads under `blocking`, tasks under `tokio`.
//!
//! The writer takes frames from a channel and writes them in FIFO order, so callers never hold a
//! lock on the socket. The reader parses each incoming frame and hands it to one of three
//! dispatchers in [`tasks`], keyed by AMS command: `AmsRequestDispatcher` for command responses
//! and the `PortConnect` and `GetLocalNetId` handshakes, `AdsNotificationDispatcher` for device
//! notification samples, and `RouterNotificationDispatcher` for router state changes. Invoke IDs
//! match responses to their callers, so any number of threads or tasks can have requests in flight
//! at once.
//!
//! A notification subscription registers before its request goes out, keyed by invoke ID, then
//! moves to the notification handle when the response arrives. A PLC that fires a sample before
//! answering the add request does not lose it.
//!
//! Dropping the last `AdsDevice` clone drops the writer's channel, which ends the writer, which
//! closes the socket, which ends the reader on its next read. Pending callers get
//! [`Error::Disconnected`] and notification receivers get [`Err`] on their next `recv`.
//!
//! ## The Symbol Cache
//!
//! `AdsRuntime` resolves a symbol path once and caches its type metadata, its size, whether writes
//! need a read-modify-write cycle, and its handle. Type descriptions are shared, so a thousand
//! instances of one function block hold one type description between them. `preload` fetches the
//! whole type dictionary and symbol table in two bulk transfers, if you would rather pay that cost
//! up front than have each first access pay its own.
//!
//! An online change bumps the PLC symbol version and invalidates every handle. A `read_value` or
//! `write_value` on a stale handle returns [`Error::HandleInvalidated`] and flushes the cache; call
//! again and the symbol resolves fresh. A subscription has no failing call of its own, so the PLC
//! pushes a zero-length sample instead, which surfaces as the same error. Subscriptions are not
//! renewed automatically, because the type or the size may have changed. Decide that it is safe,
//! then subscribe again.
//!
//! Stale handles need no release call: the PLC discarded them when it reloaded the symbol table.

pub mod devices;
pub mod error;
pub mod notif_guard;
pub mod tasks;

pub use error::{Error, Result};
