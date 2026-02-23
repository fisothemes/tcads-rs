# TwinCAT ADS Core

The fundamental building blocks for the **TwinCAT ADS** protocol in Rust.

This library is for handling AMS/ADS frame construction, parsing, and serialization. Designed to be transport-agnostic as in, it produces and consumes bytes and you provide the socket.

## Features

- **Full AMS/ADS command set** - `Read`, `Write`, `ReadWrite`, `ReadState`,
  `WriteControl`, `ReadDeviceInfo`, and the complete notification lifecycle
  (Add, Delete, Device Notification stream)
- **Bidirectional** - every command type supports both directions; request
  types parse *and* construct, response types construct *and* parse
- **Zero-copy parsing** - borrowed types (`AdsReadResponse<'a>`,
  `AdsDeviceNotification<'a>`, etc.) slice directly into the frame buffer;
  owned types (`AdsReadResponseOwned`, etc.) are available when you need
  to store or send across threads
- **Blocking and async I/O** - `blocking::AmsStream` for synchronous use;
  async equivalents share the same protocol types
- **Type-safe primitives** - `AmsNetId`, `AmsAddr`, `AdsState`,
  `AdsTransMode`, `NotificationHandle`, `WindowsFileTime`, `AdsString<N>`

## Documentation

Generate documentation with `cargo doc --open` and explore the API reference.

## Crate Layout

```text
tcads-core/
  ├── ads/        # ADS primitives (commands, states, error codes, strings, ...)
  ├── ams/        # AMS primitives (addresses, net IDs, router commands, ...)
  ├── io/         # Frame I/O (AmsFrame, AmsReader, AmsWriter, AmsStream)
  └── protocol/   # Request/response types for every ADS command
```

## Quick Start

### Frame

At the lowest level, `AmsStream` sends and receives `AmsFrame`s over TCP.
You can work directly with raw frames if you need full control:

#### Blocking I/O

```rust
use tcads_core::ams::AmsCommand;
use tcads_core::io::{AmsFrame, blocking::AmsStream};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let stream = AmsStream::connect("127.0.0.1:48898")?;
  let (reader, mut writer) = stream.try_split()?;

  // Send a raw frame
  let frame = AmsFrame::new(AmsCommand::PortConnect, [0x00, 0x00]);
  writer.write_frame(&frame)?;

  // Read the response
  let frame = reader.read_frame()?;
  println!("Received: {:?}", frame.header().command());
  
  Ok(())
}
```

#### Async I/O (Tokio)

The async API is identical in shape just swap the import and add `.await`:

```rust
use tcads_core::ams::AmsCommand;
use tcads_core::io::{AmsFrame, tokio::AmsStream};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let stream = AmsStream::connect("127.0.0.1:48898").await?;
  let (reader, mut writer) = stream.into_split();

  let frame = AmsFrame::new(AmsCommand::PortConnect, [0x00, 0x00]);
  writer.write_frame(&frame).await?;

  let frame = reader.read_frame().await?;
  println!("Received: {:?}", frame.header().command());

  Ok(())
}
```

> [!NOTE]
> Support for other async runtimes (e.g. `async-std`, `smol`) is available
> upon request.

### Using the protocol layer

Building frames by hand means managing byte layouts yourself, much pain such work. The protocol module has you covered. Every ADS command has a typed request and response
that serializes to and from `AmsFrame`:

```rust
use tcads_core::ads::{AdsCommand, AdsHeader};
use tcads_core::ams::{AmsAddr, AmsCommand};
use tcads_core::io::blocking::AmsStream;
use tcads_core::protocol::{
  GetLocalNetIdRequest, GetLocalNetIdResponse,
  PortConnectRequest, PortConnectResponse,
  AdsReadStateRequest, AdsReadStateResponse,
  RouterNotification,
};

// JetBrains RustRover is using 2 spaces for indentation on my README 😔
fn main() -> Result<(), Box<dyn std::error::Error>> {
  let stream = AmsStream::connect("127.0.0.1:48898")?;

  let (reader, mut writer) = stream.try_split()?;

  writer.write_frame(&PortConnectRequest::default().into_frame())?;

  let mut source = AmsAddr::default();
  let mut target = AmsAddr::default();

  for result in reader.incoming() {
    let frame = result?;
    match frame.header().command() {
      AmsCommand::PortConnect => {
        let resp = PortConnectResponse::try_from(frame)?;
        source = *resp.addr();
        println!("AMS Router has assigned us the address {}!", source);
        writer.write_frame(&GetLocalNetIdRequest::into_frame())?;
      }
      AmsCommand::GetLocalNetId => {
        let resp = GetLocalNetIdResponse::try_from(frame)?;
        println!("Local Net ID is {}", resp.net_id());
        target = AmsAddr::new(resp.net_id(), 851);
        println!("Target address is {}",target);
        writer.write_frame(
          &AdsReadStateRequest::new(target, source, 0x01).into_frame()
        )?;
      }
      AmsCommand::RouterNotification => {
        // Received when changing between config and run mode
        let notif = RouterNotification::try_from(frame)?;
        println!("AMS Router state: {:?}", notif.state());
      }
      AmsCommand::AdsCommand => {
        let (header, payload) = AdsHeader::parse_prefix(frame.payload())?;
        match header.command_id() {
          AdsCommand::AdsReadState => {
            let (_, state, _) = AdsReadStateResponse::parse_payload(payload)?;
            println!("PLC state: {state:?}");
          }
          _ => {}
        }
      }
      _ => {}
    }
  }

  Ok(())
}
```

Result:

```console
AMS Router has assigned us the address 192.168.137.1.1.1:32817!
Local Net ID is 192.168.137.1.1.1
Target address is 192.168.137.1.1.1:851
PLC state: Run
```

### Zero-copy response parsing

Borrowed types slice directly into the frame, no allocation for the data payload:

```rust
use tcads_core::protocol::AdsReadResponse;

// Parsed response borrows from `frame`, there no copy of the data bytes
let response = AdsReadResponse::try_from(&frame)?;
let value = i32::from_le_bytes(response.data().try_into()?);

// Need to store it? Convert explicitly
let owned = response.into_owned();
```

### Symbol handle lookup (AdsReadWrite)

```rust
use tcads_core::protocol::AdsReadWriteRequestOwned;

let request = AdsReadWriteRequestOwned::new(
    target, source, invoke_id,
    0xF003, // ADSIGRP_SYM_HNDBYNAME
    0x0000,
    4,      // handle is 4 bytes
    b"MAIN.nCount\0",
);

writer.write_frame(&request.into_frame())?;
```

### Subscribing to variable changes

```rust
use tcads_core::ads::AdsTransMode;
use tcads_core::protocol::{
    AdsAddDeviceNotificationRequest,
    AdsDeviceNotification,
};

// Subscribe
let request = AdsAddDeviceNotificationRequest::new(
    target, source, invoke_id,
    0xF005, handle,   // index group + offset (value by handle)
    4,                // variable size in bytes
    AdsTransMode::ClientOnChange,
    0,                // max delay (ms)
    100,              // cycle time (ms)
);
writer.write_frame(&request.into_frame())?;

// Receive sample data as zero-copy from the frame
let frame = reader.read_frame()?;
let notif = AdsDeviceNotification::try_from(&frame)?;
for (timestamp, sample) in notif.iter_samples() {
    if sample.handle() == my_handle {
        let value = i32::from_le_bytes(sample.data().try_into()?);
        println!("nCount = {value} at {}", timestamp.as_raw());
    }
}
```

## Borrowed vs Owned

Every type that carries a variable-length data payload comes in two forms:

| Type                   | Use when                                  |
|------------------------|-------------------------------------------|
| `AdsReadResponse<'a>`  | Parsing - borrows from the frame, no copy |
| `AdsReadResponseOwned` | Construction or storage - owns its buffer |

Convert between them with `.into_owned()`, `.to_owned()`, and `.as_view()`.
The same pattern applies to `AdsWriteRequest`, `AdsReadWriteRequest`,
`AdsReadWriteResponse`, `AdsWriteControlRequest`, and all notification types.