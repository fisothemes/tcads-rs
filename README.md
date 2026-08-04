# TwinCAT ADS for Rust

A native rust implementation of the TwinCAT ADS protocol.

This library aims to provide a way to communicate with TwinCAT ADS devices (PLCs, NC, etc.), without relying on the official Beckhoff `TcAdsDll.dll` or requiring a local TwinCAT installation on the client machine.

## Showcase

```rust
use std::thread;
use std::time::Duration;
use tcads::client::devices::blocking::{AdsDevice, AdsLogger, AdsRuntime, AdsSystemService};
use tcads::core::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    // 1. Establish a single, multiplexed connection to the local TwinCAT AMS router
    let device = AdsDevice::connect(Duration::from_secs(10))?;
    
    
    // 2. ADS Logger (Port 100): Read and write TwinCAT system logs
    let logger = AdsLogger::new(device.clone(), AmsNetId::local());
    let (rx, logger_handle) = logger.subscribe()?;

    // Spawn a background thread to listen to TwinCAT logs
    let thread_handle = thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            println!("[TcLog {:?}] {}", msg.message_type(), msg.message());
        }

        println!("[TcLog] Logger thread exited.");
    });

    // Write a custom message to the TwinCAT ADS logger
    logger.write_log(LogMessageType::WARNING, "RustClient", "Starting up...")?;
    
    
    // 3. PLC Runtime (Port 851): Read, Write, and RPC
    let rt = AdsRuntime::new(device.clone(), AmsAddr::from_local(851));

    // Read a variable directly by its symbol name
    let temperature: f32 = rt.read_value("MAIN.fMachineTemperature")?;
    println!("Current Machine Temperature: {}°C", temperature);

    // Invoke a PLC Method (RPC) by passing a Rust tuple, returning a tuple
    let (quotient, remainder): (i32, i32) = rt.rpc("MAIN.fbMath", "Divide", &(100i32, 3i32))?;
    println!("RPC Result: 100 / 3 = {} (remainder {})", quotient, remainder);


    // 4. System Service (Port 10,000): Host OS Control
    let system = AdsSystemService::new(device.clone(), AmsNetId::local());

    // Fetch the target IPC's TwinCAT version
    let version = system.get_product_version()?;
    println!("Target TwinCAT Version: {}", version);

    // Set working directory
    let folder_path = r#"C:\TcAdsRust"#;

    // Create a new directory on the target device
    system.create_dir(folder_path, AdsFilePathType::Generic)?;

    // Spawn a background process directly on the host Windows/CE/TcBSD/Linux operating system
    system.start_process_on_host(
        r#"C:\Windows\System32\cmd.exe"#,
        folder_path,
        "/C echo Hello from tcads-rs > hello.txt",
        true, // Run hidden from the user
    )?;


    // 5. Clean up
    logger.write_log(LogMessageType::WARNING, "RustClient", "Exiting...")?;

    // Unsubscribing drops the sender channel, gracefully exiting the receiver thread
    logger.unsubscribe(logger_handle)?;

    // Wait for the background thread to finish printing its exit message
    thread_handle.join().unwrap();

    Ok(())
}
```

## Workspace Structure

The project is organized as a Cargo workspace with the following crates:

- **[`tcads-core`](packages/tcads-core)**: The foundational protocol crate. Provides protocol primitives, serialization, and zero-copy parsing without any network dependencies.
- **[`tcads-io`](packages/tcads-io)**: The network transport crate. Takes `tcads-core` frames and routes them over TCP (or custom transports) using `blocking` (`std::net`) or async (`tokio`) streams.
- **[`tcads-client`](packages/tcads-client)**: The high-level API. Provides thread-safe, async-ready clients (like `AdsDevice`) for managing requests, symbols, and notifications.
- **[`tcads-server`](packages/tcads-server)**: Framework for building custom ADS servers/devices in Rust.
- **[`tcads-serde`](packages/tcads-serde)**: The serialization crate. Offers Serde-based serialization/deserialization of PLC data types, including dynamic type resolution, alias handling, and a `Value` enum for generic inspection.
- **[`tcads`](packages/tcads)**: The top-level facade crate that bundles everything together for easy consumption.
- **[`examples`](examples)**: A comprehensive, step-by-step learning progression demonstrating how to use the library from raw bytes up to high-level ADS clients/servers.

## Getting Started: Examples

The best way to learn how to use this library is by exploring the [`examples`](examples) directory. The examples are numbered to provide a gentle learning curve, for example:

1. **[`01_basic_frame_sync`](examples/src/bin/01_basic_frame_sync.rs)**: Sending raw byte payloads over a blocking TCP socket.
2. **[`02_basic_frame_async`](examples/src/bin/02_basic_frame_async.rs)**: Mirroring Example 1 using the `tokio` async engine.
3. **[`03_protocol_structs`](examples/src/bin/03_protocol_structs.rs)**: Using the strongly-typed `protocol` builders instead of manual byte-packing.
4. **[`04_chaining_protocols`](examples/src/bin/04_chaining_protocols.rs)**: Chaining requests to perform a router handshake and read device info.
5. **[`05_rtime_cpu_settings`](examples/src/bin/05_rtime_cpu_settings.rs)**: Querying the TwinCAT OS Real-Time system (Port 200) and parsing little-endian bytes.
6. **[`06_ads_device_basic`](examples/src/bin/06_ads_device_basic.rs)**: Introducing the high-level `AdsDevice` to abstract away sockets, headers, and routing.
7. **[`07_ads_device_async`](examples/src/bin/07_ads_device_async.rs)**: Mirroring Example 6 using the `tokio` async engine.

and [more](examples/src/bin).

## Status

> [!WARNING]
> This project is currently under active development. APIs are subject to change.

[`tcads-core`](packages/tcads-core), [`tcads-io`](packages/tcads-io) and [`tcads-serde`](packages/tcads-serde) are the most mature crates. [`tcads-client`](packages/tcads-client) and [`tcads-server`](packages/tcads-server) are work-in-progress.

## Acknowledgments & Prior Art

Building a native protocol implementation from scratch requires standing on the shoulders of giants. This project was made possible by, and draws heavy inspiration from, the following projects and resources:

- **[Beckhoff/ADS](https://github.com/Beckhoff/ADS)**: The official open-source C++ ADS library provided by Beckhoff. It served as the primary reference for the AMS/ADS protocol routing, device states, and C++ header translations.
- **[jisotalo/ads-client](https://github.com/jisotalo/ads-client)**: An incredible, full-featured Node.js ADS client. Jussi Isotalo's reverse-engineering efforts, specifically documented in his blog post [Subscribing to TwinCAT logger in Node.js](https://jisotalo.fi/subscribing-to-twincat-logger-in-nodejs/), were instrumental in building the `AdsLogger` device and deciphering the undocumented `ADSLOGSTR` wire format.
- **[birkenfeld/ads-rs](https://github.com/birkenfeld/ads-rs)**: An earlier Rust implementation of the ADS protocol that provided excellent prior art and inspiration for modeling ADS concepts in idiomatic Rust.
- **[Beckhoff Information System (InfoSys)](https://infosys.beckhoff.com/content/1033/tc3_ads_intro/index.html)**: The official TwinCAT 3 documentation and ADS specification portal.

---

## Disclaimer

This is an independent project, not affiliated with or endorsed by Beckhoff
Automation GmbH & Co. KG. "TwinCAT" and "ADS" are trademarks of Beckhoff Automation.