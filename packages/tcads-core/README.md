# TwinCAT ADS Core

This crate contains the core building blocks for the **TwinCAT AMS/ADS** protocol.

It handles the heavy lifting of AMS/ADS frame construction, parsing, and serialization. It is strictly **transport-agnostic** and contains **no networking dependencies** (like `tokio` or `std::net`). It doesn't care how you move bytes—whether you're using standard TCP, a custom serial bridge, or an async runtime, `tcads-core` provides the pure protocol logic.

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
- **Type-safe primitives** - `AmsNetId`, `AmsAddr`, `AdsState`,
  `AdsTransMode`, `NotificationHandle`, `WindowsFileTime`, `AdsString<N>`

## Documentation

Generate documentation with `cargo doc --open` and explore the API reference.

## Crate Layout

```text
tcads-core/
  ├── ads/        # ADS primitives (commands, states, error codes, strings, ...)
  ├── ams/        # AMS primitives (addresses, net IDs, router commands, ...)
  ├── protocol/   # Request/response types for every ADS command
  └── frame.rs    # The AmsFrame byte-buffer container

```

## Quick Start

### Low-level Frame Construction

At the lowest level, the protocol is wrapped in an `AmsFrame`. You can construct raw frames and serialize them into byte vectors to send over your chosen transport layer.

```rust
use tcads_core::ams::AmsCommand;
use tcads_core::AmsFrame;

fn main() {
    // Construct a raw Port Connect frame
    let port_connect_frame = AmsFrame::new(AmsCommand::PortConnect, [0x00, 0x00]);

    // Extract the wire-ready bytes to send over your socket
    let network_bytes: Vec<u8> = port_connect_frame.to_vec();
    
    assert_eq!(network_bytes.len(), 8); // 6-byte header + 2-byte payload
}

```

### Using the protocol layer

Building frames by hand means managing byte layouts yourself. This is best described as `"much pain, such work"`. Luckily, the `protocol` module has you covered. Every AMS and ADS command has a typed request and response that serializes to and from the `AmsFrame`:

```rust
use tcads_core::protocol::PortConnectRequest;
use tcads_core::AmsFrame;

fn main() {
    // Construct a typed request
    let request = PortConnectRequest::default();

    // Convert it to a wire-ready frame
    let frame: AmsFrame = request.into_frame();
    
    // Serialize to bytes for transmission
    let bytes_to_send = frame.to_vec();
}

```

### Zero-copy response parsing

When you receive bytes from your network layer, you can parse them into typed responses. Borrowed types slice directly into the frame buffer, performing zero allocations for the data payload:

```rust
use tcads_core::protocol::AdsReadResponse;
use tcads_core::AmsFrame;

fn parse_example(frame: &AmsFrame) -> Result<(), Box<dyn std::error::Error>> {
    // Parsed response borrows from `frame`, there is no copy of the data bytes
    let response = AdsReadResponse::try_from(frame)?;
    let value = i32::from_le_bytes(response.data().try_into()?);

    // Need to store it across threads? Convert explicitly
    let owned = response.into_owned();
    
    Ok(())
}

```

### Symbol handle lookup (AdsReadWrite)

Constructing complex requests is simple and strongly typed:

```rust
use tcads_core::protocol::AdsReadWriteRequestOwned;
use tcads_core::ams::AmsAddr;
use tcads_core::ads::{IndexGroup, IndexOffset};

fn build_handle_request(target: AmsAddr, source: AmsAddr, invoke_id: u32) {
    let request = AdsReadWriteRequestOwned::new(
        target, source, invoke_id,
        IndexGroup::new(0xF003), // Symbol handle by name
        IndexOffset::ZERO,
        4, // Handle is 4 bytes
        b"MAIN.nCount\0",
    );

    let frame = request.into_frame();
    // Pass `frame.to_vec()` to your socket...
}
```

### Subscribing to variable changes

```rust
use tcads_core::ads::{AdsTransMode, AdsNotificationAttrib, NotificationHandle, IndexGroup, IndexOffset};
use tcads_core::protocol::{AdsAddDeviceNotificationRequest, AdsDeviceNotification};
use tcads_core::ams::AmsAddr;
use tcads_core::AmsFrame;

fn build_subscription(target: AmsAddr, source: AmsAddr, invoke_id: u32, handle: NotificationHandle) {
    // Subscribe
    let request = AdsAddDeviceNotificationRequest::new(
        target, 
        source, 
        invoke_id,
        IndexGroup::new(0xF005), // Value by handle
        IndexOffset::new(handle.as_u32()),
        AdsNotificationAttrib {
            length: 4, // variable size in bytes
            trans_mode: AdsTransMode::ServerOnChange, 
            max_delay: 0, // (100ns steps)
            cycle_time: 10_000 * 100, // 100 ms (100ns steps)
        }
    );
    
    let frame = request.into_frame();
    // Pass `frame.to_vec()` to your socket...
}

fn handle_notification(frame: &AmsFrame, my_handle: NotificationHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Receive sample data as zero-copy from the frame
    let notif = AdsDeviceNotification::try_from(frame)?;
    
    for (timestamp, sample) in notif.iter_samples() {
        if sample.handle() == my_handle {
            let value = i32::from_le_bytes(sample.data().try_into()?);
            println!("nCount = {value} at {timestamp}");
        }
    }
    
    Ok(())
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