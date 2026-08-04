//! # TwinCAT ADS I/O
//!
//! This crate provides the network transport layer for the TwinCAT AMS/ADS protocol.
//!
//! It bridges the pure-protocol definitions found in `tcads-core` with actual
//! network sockets, providing robust streams, frame-chunking readers, and writers.
//!
//! ## Transport Agnostic
//!
//! While the default transport is TCP (`TcpStream`), the stream wrappers in this
//! crate are generic over the underlying I/O traits (`Read + Write` for blocking,
//! `AsyncRead + AsyncWrite` for Tokio).
//!
//! This means you are not locked into standard TCP. You can route ADS traffic
//! over **TLS**, **Unix Domain Sockets**, **Serial Ports**, or even **in-memory buffers**
//! for testing, simply by passing your custom stream into [`AmsStream::new()`](blocking::AmsStream::new).
//!
//! ## Runtimes
//!
//! The crate is split into two distinct implementations:
//! - **[`blocking`]**: Uses standard library [`std::io`] and blocking threads.
//! - **[`tokio`]**: Uses [`tokio::io`] for asynchronous I/O.
//!
//! Both modules provide an `AmsStream` that can be split into independent reader
//! and writer halves, allowing you to process incoming server notifications
//! concurrently with outgoing client requests.
//!
//! ## Example (Async Tokio)
//!
//! ```rust,no_run
//! use tcads_io::tokio::AmsStream;
//! use tcads_core::ams::AmsCommand;
//! use tcads_core::protocol::{PortConnectRequest, PortConnectResponse};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. Establish a connection to the AMS Router (defaults to TCP)
//!     let stream = AmsStream::connect("127.0.0.1:48898").await?;
//!
//!     // 2. Split the stream into a concurrent reader and writer
//!     let (mut reader, mut writer) = stream.into_split();
//!
//!     // 3. Send a request using a tcads-core protocol builder
//!     let request = PortConnectRequest::default();
//!     writer.write_frame(&request.into_frame()).await?;
//!
//!     // 4. Wait for the specific response frame
//!     let response_frame = reader.read_frame().await?;
//!
//!     if response_frame.header().command() == AmsCommand::PortConnect {
//!         let response = PortConnectResponse::try_from(&response_frame)?;
//!         println!("Router assigned us the address: {}", response.addr());
//!     }
//!
//!     Ok(())
//! }
//! ```

/// Synchronous (blocking) I/O utilizing [`std::net`] and [`std::io`].
pub mod blocking;

/// Asynchronous I/O utilizing [`tokio::net`] and [`tokio::io`].
pub mod tokio;
