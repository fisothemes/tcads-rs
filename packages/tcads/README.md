# TwinCAT ADS for Rust

A native rust implementation of the TwinCAT ADS protocol.

This crate is the front door to the `tcads-rs` workspace. It pulls in every sub-crate and
re-exports them under one name, so you depend on `tcads` and reach the rest through it.

```rust
use std::time::Duration;
use tcads::client::devices::blocking::AdsRuntime;
use tcads::core::AmsAddr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = AdsRuntime::connect(AmsAddr::from_local(851), Duration::from_secs(5))?;

    let count: u32 = runtime.read_value("MAIN.nCount")?;
    runtime.write_value("MAIN.bIncrement", true)?;

    Ok(())
}
```

## Sub-crates

| Path            | Crate                    | What it does                                      |
|-----------------|--------------------------|---------------------------------------------------|
| `tcads::core`   | [`tcads-core`][core]     | Protocol primitives, frames, and serialization    |
| `tcads::client` | [`tcads-client`][client] | Connection and device clients                     |
| `tcads::io`     | [`tcads-io`][io]         | Network transport                                 |
| `tcads::serde`  | [`tcads-serde`][serde]   | Serde data format for the ADS wire representation |
| `tcads::server` | [`tcads-server`][server] | Framework for building ADS servers                |

Most of what you want is in `tcads::client`. Start with its
[README][client] for connecting, reading and writing symbols, subscriptions, and RPC.

## Feature Flags

| Flag       | Default | What it enables                             |
|------------|---------|---------------------------------------------|
| `blocking` | Yes     | `std::net` sockets and background threads   |
| `tokio`    | No      | Tokio sockets and background tasks          |

Both can be enabled at the same time.

> This crate is a work in progress and its API will change.

[core]: https://github.com/fisothemes/tcads-rs/tree/master/packages/tcads-core
[client]: https://github.com/fisothemes/tcads-rs/tree/master/packages/tcads-client
[io]: https://github.com/fisothemes/tcads-rs/tree/master/packages/tcads-io
[serde]: https://github.com/fisothemes/tcads-rs/tree/master/packages/tcads-serde
[server]: https://github.com/fisothemes/tcads-rs/tree/master/packages/tcads-server
