# TwinCAT ADS Client

The high-level client layer for the **TwinCAT AMS/ADS** protocol.

This crate takes the typed frames from `tcads-core`, pushes them over a `tcads-io` stream,
and gives you back device clients you can call from anywhere in your program:
read a PLC variable by name, subscribe to value changes, call a function block method, tail 
the TwinCAT log, or drive the system service on the target IPC.

## Features

- **Symbol access by name** - `read_value("MAIN.nCount")` and `write_value` resolve the symbol,
  cache its handle and type metadata, and use `serde` to map PLC memory onto your Rust types.
- **Batch (Sum) commands** - read or write many symbols in one round trip with
  `read_multi_values` and `write_multi_values`, or use the raw `read_multi`, `write_multi`,
  `read_write_multi`, and `add_multi_notifications` on `AdsDevice`.
- **Notifications** - `subscribe_value` returns a typed receiver. The subscription is deleted on
  the router when the receiver drops, or explicitly through `unsubscribe`.
- **RPC** - call methods on function blocks and interfaces with Rust tuples in and tuples out.
- **Blocking and Tokio** - two implementations with the same shape, chosen with feature flags.
- **Subsystem devices** - `AdsRuntime` (PLC ports 851, 801-899, 301-399), `AdsLogger` (port 100),
  `AdsSystemService` (port 10000), and `AdsDevice` for any port you want to drive by hand.

## Feature Flags

| Flag       | Default | What it enables                                                 |
|------------|---------|-----------------------------------------------------------------|
| `blocking` | Yes     | `std::net` sockets and background threads (`devices::blocking`) |
| `tokio`    | No      | Tokio sockets and background tasks (`devices::tokio`)           |

Both can be enabled at the same time. The two module trees mirror each other, so moving a blocking
program to async is mostly a matter of adding `.await`.

## Crate Layout

```text
tcads-client/
  ├── devices/
  │     ├── ads_device/      # AdsDevice: the connection and the raw ADS command set
  │     ├── runtime/         # AdsRuntime: symbols, values, subscriptions, RPC
  │     ├── logger/          # AdsLogger: TwinCAT system log (port 100)
  │     └── system_service/  # AdsSystemService: host OS, files, processes (port 10000)
  ├── tasks/                 # Reader, writer, and the dispatchers that route frames
  ├── notif_guard.rs         # RAII guards that delete notifications on drop
  └── error.rs
```

## Quick Start

### Connecting

There are three ways in, and every device client offers all three.

```rust
use std::time::Duration;
use tcads_client::devices::blocking::AdsRuntime;
use tcads_core::AmsAddr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Through the local AMS router at 127.0.0.1:48898.
    //    The router assigns the source address during the PortConnect handshake.
    let runtime = AdsRuntime::connect(AmsAddr::from_local(851), Duration::from_secs(5))?;

    // 2. Through the local router, but ask it for the local Net ID first.
    //    Use this when the TwinCAT target system is set to <Local>.
    let runtime = AdsRuntime::connect_local(851u16, None)?;

    // 3. Straight to a remote router, with no local router on this machine.
    //    `source` must already exist as a static route on the target.
    let source: AmsAddr = "192.168.1.10.1.1:32750".parse()?;
    let target: AmsAddr = "192.168.1.120.1.1:851".parse()?;
    let runtime = AdsRuntime::connect_remote("192.168.1.120:48898", source, target, None)?;

    Ok(())
}
```

The first two go through the local AMS router. Installing XAE or XAR installs that router; it
listens on port `48898` and every ADS message passes through it, whether the target is on the same
machine or across the network.

On Windows, `127.0.0.1` needs the `EnableAmsTcpLoopback` registry key. It is set by default from
TwinCAT 4024.5 onwards.

The third form is for machines that cannot run XAE or XAR, and so have no local router: a Linux or
MacOS development machine, a container, or a bare Windows box. You connect to the target's router
directly, and your source Net ID has to be registered there first. See
[Connecting without a local AMS router](../../docs/static-routes.md) for how to add the route and
what to do when the connection drops.

### Sharing one connection

Connect once, then wrap the same `AdsDevice` in as many subsystem clients as you need. They share
the socket, the reader, and the writer.

```rust
use std::time::Duration;
use tcads_client::devices::blocking::{AdsDevice, AdsLogger, AdsRuntime, AdsSystemService};
use tcads_core::{AmsAddr, AmsNetId};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = AdsDevice::connect(Duration::from_secs(5))?;

    let runtime = AdsRuntime::new(device.clone(), AmsAddr::from_local(851));
    let logger = AdsLogger::new(device.clone(), AmsNetId::local());
    let system = AdsSystemService::new(device.clone(), AmsNetId::local());

    Ok(())
}
```

### Reading and writing values

```rust
use serde::{Deserialize, Serialize};
use tcads_client::devices::blocking::AdsRuntime;
use tcads_core::AmsAddr;

#[derive(Debug, Serialize, Deserialize)]
struct RecipeStep {
    command: u16,
    target: f64,
    duration_ms: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = AdsRuntime::connect(AmsAddr::from_local(851), None)?;

    let count: u32 = runtime.read_value("MAIN.nCount")?;
    let recipe_id: String = runtime.read_value("MAIN.fbCurrentRecipe.Id")?;
    let step: RecipeStep = runtime.read_value("MAIN.fbCurrentRecipe.arSteps[0]")?;

    runtime.write_value("MAIN.bIncrement", true)?;
    runtime.write_value("MAIN.fbCurrentRecipe.Id", "BATCH_ID_01")?;

    Ok(())
}
```

Struct fields match the PLC type by position, not by name. See [`tcads-serde`](../tcads-serde) for
the mapping rules, including strings, arrays, enums, and aliases.

### Batching

Each `read_value` call costs a round trip. `read_multi_values` and `write_multi_values` bundle
several into one. If any symbol in the batch fails, the whole call returns that error.

```rust
use tcads_client::devices::blocking::AdsRuntime;
use tcads_core::AmsAddr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = AdsRuntime::connect(AmsAddr::from_local(851), None)?;

    runtime
        .write_multi_values()
        .push("MAIN.bIncrement", &true)
        .push("MAIN.fbCurrentRecipe.Id", "BATCH_ID_01")
        .execute()?;

    let mut batch = runtime.read_multi_values(["MAIN.nCount", "MAIN.fbCurrentRecipe.Id"])?;

    let count: u32 = batch.pop_front().unwrap()?;
    let recipe_id: String = batch.pop_front().unwrap()?;

    Ok(())
}
```

Results come back in the order the paths went in. `pop_front`, `pop_back`, `get`, and
`into_iter_as` each pick the type at the point of use.

### Subscribing to value changes

```rust
use std::time::Duration;
use tcads_client::Error;
use tcads_client::devices::blocking::AdsRuntime;
use tcads_core::{AdsTransMode, AmsAddr};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = AdsRuntime::connect(AmsAddr::from_local(851), None)?;

    let (rx, handle) = runtime.subscribe_value::<u32>(
        "MAIN.nCount",
        AdsTransMode::ServerOnChange,
        Duration::ZERO,
        Duration::ZERO,
    )?;

    for result in rx.iter().take(5) {
        match result {
            Ok(value) => println!("MAIN.nCount = {value}"),
            Err(Error::HandleInvalidated(path)) => {
                println!("'{path}' changed under us, resolve it again");
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }

    rx.unsubscribe()?;

    Ok(())
}
```

Use `AdsTransMode::ServerCycle` with a `cycle_time` for values that change too fast to push on
every edit.

### Calling PLC methods

Inputs and outputs follow a 0, 1, N rule: `()` for none, the bare type for one, a tuple for
several. The output tuple starts with the return value, then `VAR_OUTPUT` and `VAR_IN_OUT`.

```rust
use tcads_client::devices::blocking::AdsRuntime;
use tcads_core::AmsAddr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = AdsRuntime::connect(AmsAddr::from_local(851), None)?;
    let fb = "MAIN.fbMath";

    runtime.rpc::<()>(fb, "Reset", ())?;
    runtime.rpc::<()>(fb, "SetValue", 100i32)?;

    let value: i32 = runtime.rpc(fb, "GetValue", ())?;
    let (quotient, remainder): (i32, i32) = runtime.rpc(fb, "DivideValues", (100i32, 3i32))?;

    Ok(())
}
```

Method handles are cached per instance, so repeated calls on the same function block skip the
lookup.

### Reading and writing the TwinCAT log

```rust
use std::thread;
use tcads_client::devices::blocking::AdsLogger;
use tcads_core::LogMessageType;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = AdsLogger::connect_local(None)?;

    let (rx, handle) = logger.subscribe()?;

    let printer = thread::spawn(move || {
        for entry in rx.iter().take(10).flatten() {
            println!("[{:?}] {}", entry.message_type(), entry.message());
        }
    });

    logger.write_log(LogMessageType::WARNING, "RustClient", "Starting up...")?;

    logger.unsubscribe(handle)?;
    printer.join().unwrap();

    Ok(())
}
```

`write_log` is the equivalent of `ADSLOGSTR` in Structured Text. Use `write_entry` when you need to
set the timestamp yourself.

### Driving the host system

`AdsSystemService` reaches past the PLC to the machine it runs on: TwinCAT mode, file I/O, host
processes, the Windows registry, and the system clock.

```rust
use tcads_client::devices::blocking::AdsSystemService;
use tcads_core::AdsFilePathType;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let system = AdsSystemService::connect_local(None)?;

    println!("TwinCAT version: {}", system.get_product_version()?);
    println!("Target: {}", system.get_target_info()?);

    let dir = r#"C:\TcAdsRust"#;
    system.create_dir(dir, AdsFilePathType::Generic)?;

    system.start_process_on_host(
        r#"C:\Windows\System32\cmd.exe"#,
        dir,
        "/C echo Hello from tcads-rs > hello.txt",
        true, // hidden from the user
    )?;

    Ok(())
}
```

### Raw ADS commands

`AdsDevice` exposes the command set directly when no subsystem client fits, for example reading a
CPU setting on port 200 or talking to a custom ADS server.

```rust
use tcads_client::devices::blocking::AdsDevice;
use tcads_core::{AmsAddr, IndexGroup, IndexOffset};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = AdsDevice::connect(None)?;
    let target = AmsAddr::from_local(851);

    let (version, name) = device.read_device_info(target)?;
    let (ads_state, device_state) = device.read_state(target)?;

    let bytes = device.read(target, IndexGroup::PLC_MEMORY_BYTES, IndexOffset::new(0), 4)?;
    device.write(target, IndexGroup::PLC_MEMORY_BYTES, IndexOffset::new(0), [1, 0, 0, 0])?;

    device.shutdown()?;

    Ok(())
}
```

### Async

The `tokio` module is the same API with `.await` on the I/O calls.

```rust
use std::time::Duration;
use tcads_client::devices::tokio::AdsRuntime;
use tcads_core::{AdsTransMode, AmsAddr};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = AdsRuntime::connect(AmsAddr::from_local(851), None).await?;

    let count: u32 = runtime.read_value("MAIN.nCount").await?;
    runtime.write_value("MAIN.bIncrement", true).await?;

    let (mut rx, handle) = runtime
        .subscribe_value::<u32>(
            "MAIN.nCount",
            AdsTransMode::ServerOnChange,
            Duration::ZERO,
            Duration::ZERO,
        )
        .await?;

    while let Ok(value) = rx.recv().await {
        println!("MAIN.nCount = {value}");
    }

    Ok(())
}
```

Two differences worth knowing: async receivers take `&mut self` in `recv`, and they have no
`recv_timeout` or `iter`. Wrap `recv` in `tokio::time::timeout` and drive it from a loop instead.

## How the connection works

`AdsDevice::connect` splits the socket into a reader half and a writer half, then spawns one
background worker for each: threads under `blocking`, tasks under `tokio`.

- **Writer** takes frames from a channel and writes them in FIFO order. Callers never hold a lock
  on the socket.
- **Reader** parses each incoming frame and hands it to one of three dispatchers, keyed by AMS
  command: `AmsRequestDispatcher` for command responses and the `PortConnect` and `GetLocalNetId`
  handshakes, `AdsNotificationDispatcher` for device notification samples, and
  `RouterNotificationDispatcher` for router state changes.
- **Invoke IDs** match responses to their callers, so any number of threads or tasks can have
  requests in flight at once.

A notification subscription registers before its request goes out, keyed by invoke ID, then moves
to the notification handle when the response arrives. A PLC that fires a sample before answering
the add request does not lose it.

`AdsDeviceInner` and the whole `tasks` module are public. To build your own subsystem client on an
existing connection, reach the dispatchers directly instead of going through `AdsDevice`.

### Shutdown

Call `shutdown` for a clean disconnect, or drop the last clone. Dropping the last `AdsDevice` drops
the writer's channel, which ends the writer, which closes the socket, which ends the reader on its
next read. Pending callers get `Error::Disconnected` and notification receivers get `Err` on their
next `recv`.

## The symbol cache

`AdsRuntime` resolves a symbol path once and caches its type metadata, its size, whether writes
need a read-modify-write cycle, and its handle. Type descriptions are shared, so a thousand
instances of one function block hold one type description between them.

`preload` fetches the entire type dictionary and symbol table in two bulk transfers, if you would
rather pay that cost up front than have each first access pay its own.

An online change bumps the PLC symbol version and invalidates every handle. Two things happen when
that lands:

- A `read_value` or `write_value` on a stale handle returns `Error::HandleInvalidated(path)` and
  flushes the cache. Call again and the symbol resolves fresh.
- A subscription has no failing call of its own, so the PLC pushes a zero-length sample instead.
  `ValueReceiver` turns that into the same `HandleInvalidated` error and flushes the cache. It does
  not resubscribe on your behalf, because the type or the size may have changed. Decide that it is
  safe, then call `subscribe_value` again.

Use `subscribe_symbol_version` to see the change coming, or `invalidate_symbol_cache` to flush by
hand after a reconnect. Stale handles need no release call: the PLC discarded them when it reloaded
the symbol table.

## Errors

Every fallible call returns `tcads_client::Result<T>`, an alias for
`Result<T, tcads_client::Error>`. `Error` is `Clone`, holds `io::Error` behind an `Arc`, and
carries through the protocol errors from `tcads-core` (`AdsReturnCode`, `AmsError`,
`ProtocolError`) and the serialization errors from `tcads-serde`. The variants worth matching on by
hand are `Disconnected`, `Timeout`, `HandleInvalidated`, `MethodNotFound`, and `MethodNotCallable`.

## Examples

The [`examples`](../../examples/src/bin) directory runs from raw frames up to this crate. Examples
06 onwards use `AdsDevice`, 10 to 12 the logger, 13 to 22 the runtime, and 23 to 25 the system
service. They need the TwinCAT project in [`examples/twincat`](../../examples/twincat) activated
and in RUN mode.

> This crate is a work in progress and its API will change.