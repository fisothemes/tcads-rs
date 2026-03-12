# TwinCAT ADS for Rust

A rust-native implementation of the TwinCAT ADS protocol.

This library aims to provide a robust way to communicate with TwinCAT devices (PLCs, NC, etc.), without relying on the official Beckhoff `TcAdsDll.dll` or requiring a local TwinCAT installation on the client machine.

The project is organized as a Cargo workspace with the following crates:

- **[`tcads-core`](packages/tcads-core)**: The foundational crate. Provides protocol primitives, serialization, and raw TCP framing.
- **[`tcads-client`](packages/tcads-client)**: The high-level API. Provides thread-safe, async-ready clients (like `AdsDevice`) for managing requests, symbols, and notifications.
- **[`tcads-server`](packages/tcads-server)**: Framework for building custom ADS servers/devices in Rust.
- **[`tcads`](packages/tcads)**: The top-level facade crate that bundles everything together for easy consumption.
- **[`examples`](examples)**: A comprehensive, step-by-step learning progression demonstrating how to use the library from raw bytes up to high-level Actor clients.

## Getting Started: Examples

The best way to learn how to use this library is by exploring the [`examples`](examples) directory. The examples are numbered to provide a gentle learning curve, for example:

1. **[`01_basic_frame_sync`](examples/src/bin/01_basic_frame_sync.rs)**: Sending raw byte payloads over a blocking TCP socket.
2. **[`02_basic_frame_async`](examples/src/bin/02_basic_frame_async.rs)**: Mirroring Example 1 using the `tokio` async engine.
3. **[`03_protocol_structs`](examples/src/bin/03_protocol_structs.rs)**: Using the strongly-typed `protocol` builders instead of manual byte-packing.
4. **[`04_chaining_protocols`](examples/src/bin/04_chaining_protocols.rs)**: Chaining requests to perform a router handshake and read device info.
5. **[`05_rtime_cpu_settings`](examples/src/bin/05_rtime_cpu_settings.rs)**: Querying the TwinCAT OS Real-Time system (Port 200) and parsing little-endian bytes.

and [more](examples/src/bin/).

## Status

> [!WARNING]
> This project is currently under active development. APIs are subject to change.

[`tcads-core`](packages/tcads-core) is the most mature component and covers the full AMS/ADS
command set. [`tcads-client`](packages/tcads-client) and [`tcads-server`](packages/tcads-server) are in progress.

---

## Disclaimer

This is an independent project, not affiliated with or endorsed by Beckhoff
Automation GmbH & Co. KG. "TwinCAT" and "ADS" are trademarks of Beckhoff
Automation.