# TwinCAT ADS for Rust

A rust-native implementation of the TwinCAT ADS protocol.

This library aims to provide a robust way to communicate with TwinCAT devices (PLCs, NC, etc.) directly over TCP/IP, without relying on the official Beckhoff `TcAdsDll.dll` or requiring a local TwinCAT installation on the client machine.

The project is organised as a Cargo workspace with the following crates:

* **[`tcads-core`](packages/tcads-core)**:
    * Contains the low-level ADS/AMS protocol definitions.
    * Handles serialization/deserialization of headers, commands, and payloads.
    * Provides type-safe wrappers for ADS primitives (`AmsNetId`, `AdsState`, `AdsString`, etc.).

* **[`tcads-client`](packages/tcads-client)**:
    * An asynchronous ADS client.
    * Manages the TCP connection, AMS routing, and request/response matching.

* **[`tcads-server`](packages/tcads-server)**:
    * Framework for building custom ADS servers/devices in Rust.

* **[`tcads`](packages/tcads)**:
    * The top-level crate that bundles the ecosystem together for easy usage.

## Disclaimer

This is an independent project and is not affiliated with, endorsed by, or associated with Beckhoff Automation GmbH & Co. KG. "TwinCAT" and "ADS" are trademarks of Beckhoff Automation.

> [!WARNING]
> This project is currently under active development. APIs are subject to change.