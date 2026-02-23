# TwinCAT ADS for Rust

A rust-native implementation of the TwinCAT ADS protocol.

This library aims to provide a robust way to communicate with TwinCAT devices (PLCs, NC, etc.), without relying on the official Beckhoff `TcAdsDll.dll` or requiring a local TwinCAT installation on the client machine.

The project is organised as a Cargo workspace with the following crates:

- **[`tcads-core`](packages/tcads-core)**: Protocol primitives, serialization, and frame I/O
- **[`tcads-client`](packages/tcads-client)**: High-level connection and request management for ADS devices.
- **[`tcads-server`](packages/tcads-server)**: Framework for building custom ADS servers/devices in Rust.
- **[`tcads`](packages/tcads)**: The top-level crate that bundles everything together.

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