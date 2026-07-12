# TwinCAT ADS for Rust

A rust-native implementation of the TwinCAT ADS protocol.

This library aims to provide a robust way to communicate with TwinCAT devices (PLCs, NC, etc.), without relying on the official Beckhoff `TcAdsDll.dll` or requiring a local TwinCAT installation on the client machine.

The project is organized as a Cargo workspace with the following crates:

- **[`tcads-core`](packages/tcads-core)**: The foundational crate. Provides protocol primitives, serialization, and raw TCP framing.
- **[`tcads-client`](packages/tcads-client)**: The high-level API. Provides thread-safe, async-ready clients (like `AdsDevice`) for managing requests, symbols, and notifications.
- **[`tcads-server`](packages/tcads-server)**: Framework for building custom ADS servers/devices in Rust.
- **[`tcads-serde`](packages/tcads-serde)**: The serialization engine. Offers Serde-based serialization/deserialization of PLC data types, including dynamic type resolution, alias handling, and a `Value` enum for generic inspection.
- **[`tcads`](packages/tcads)**: The top-level facade crate that bundles everything together for easy consumption.
- **[`examples`](examples)**: A comprehensive, step-by-step learning progression demonstrating how to use the library from raw bytes up to high-level Actor clients.

## Getting Started: Examples

The best way to learn how to use this library is by exploring the [`examples`](examples) directory. The examples are numbered to provide a gentle learning curve, for example:

1. **[`01_basic_frame_sync`](examples/src/bin/01_basic_frame_sync.rs)**: Sending raw byte payloads over a blocking TCP socket.
2. **[`02_basic_frame_async`](examples/src/bin/02_basic_frame_async.rs)**: Mirroring Example 1 using the `tokio` async engine.
3. **[`03_protocol_structs`](examples/src/bin/03_protocol_structs.rs)**: Using the strongly-typed `protocol` builders instead of manual byte-packing.
4. **[`04_chaining_protocols`](examples/src/bin/04_chaining_protocols.rs)**: Chaining requests to perform a router handshake and read device info.
5. **[`05_rtime_cpu_settings`](examples/src/bin/05_rtime_cpu_settings.rs)**: Querying the TwinCAT OS Real-Time system (Port 200) and parsing little-endian bytes
6. **[`06_basic_ads_device`](examples/src/bin/06_basic_ads_device.rs)**: Introducing the high-level `AdsDevice` to abstract away sockets, headers, and routing.
7. **[`07_basic_ads_device_async`](examples/src/bin/07_basic_ads_device_async.rs)**: Mirroring Example 6 using the `tokio` async engine.

and [more](examples/src/bin/).

## Status

> [!WARNING]
> This project is currently under active development. APIs are subject to change.

[`tcads-core`](packages/tcads-core) is the most mature component and covers the full AMS/ADS
command set. [`tcads-client`](packages/tcads-client) and [`tcads-server`](packages/tcads-server) are in progress.

## Acknowledgments & Prior Art

Building a native protocol implementation from scratch requires standing on the shoulders of giants. This project was made possible by, and draws heavy inspiration from, the following projects and resources:

- **[Beckhoff/ADS](https://github.com/Beckhoff/ADS)**: The official open-source C++ ADS library provided by Beckhoff. It served as the primary reference for the AMS/ADS protocol routing, device states, and C++ header translations.
- **[jisotalo/ads-client](https://github.com/jisotalo/ads-client)**: An incredible, full-featured Node.js ADS client. Jussi Isotalo's reverse-engineering efforts, specifically documented in his blog post [Subscribing to TwinCAT logger in Node.js](https://jisotalo.fi/subscribing-to-twincat-logger-in-nodejs/), were instrumental in building the `Logger` device and deciphering the undocumented `ADSLOGSTR` wire format.
- **[birkenfeld/ads-rs](https://github.com/birkenfeld/ads-rs)**: An earlier Rust implementation of the ADS protocol that provided excellent prior art and inspiration for modeling ADS concepts in idiomatic Rust.
- **[Beckhoff Information System (InfoSys)](https://infosys.beckhoff.com/content/1033/tc3_ads_intro/index.html)**: The official TwinCAT 3 documentation and ADS specification portal.

---

## Disclaimer

This is an independent project, not affiliated with or endorsed by Beckhoff
Automation GmbH & Co. KG. "TwinCAT" and "ADS" are trademarks of Beckhoff
Automation.