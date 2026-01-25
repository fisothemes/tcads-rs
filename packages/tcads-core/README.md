# TwinCAT ADS Core

The fundamental building blocks for the **TwinCAT ADS** protocol in Rust.

This library provides the **Data Structures**, **Serialization**, and **Deserialization** logic required to communicate with Beckhoff ADS Devices. It is designed to be **transport-agnostic**, meaning it handles the *bytes*, while you (or `tcads-client`) handle the *sockets*.

## Features

* **Protocol Implementation**: Full implementation of AMS/TCP Header, AMS Header, and ADS Payload framing.
* **Type Safety**: Strongly-typed wrappers for ADS primitives:
    * `AmsNetId` & `AmsAddr` (with parsing support).
    * `AdsState` & `AdsTransMode` enums.
    * `NotificationHandle` new-types.
    * `WindowsFiletime` with `std::time::SystemTime` conversion.
* **String Handling**: `AdsString<N>` handles legacy Windows-1252 (CP1252) encoding and null-termination automatically.
* **Zero-Copy Friendly**: The `AmsPacket` struct supports borrowed content (`&[u8]`) for high-performance parsing.
* **Command Support**: Ready-to-use Request and Response structures for:
    * Read / Write / ReadWrite
    * Device Info
    * Device Status (ReadState / WriteControl)
    * Device Notifications (Add / Delete / Stream)

## Modules

### `protocol`
Contains the wire-format structures.
* **`packet::AmsPacket`**: The main container (Header + Payload).
* **`header::AmsHeader`**: The 32-byte routing header (Target NetId, Source NetId, Command ID, etc.).
* **`codec::AmsCodec`**: A stateless helper to encode/decode packets to/from `std::io::Read` and `std::io::Write` streams (handles TCP framing).

### `commands`
Payload definitions for standard ADS commands.
* `AdsReadRequest` / `AdsReadResponse`
* `AdsWriteRequest` / `AdsWriteResponse`
* `AdsAddDeviceNotificationRequest` / `AdsDeviceNotificationStreamHeader`
* ...and more.

### `types`
Rust types mapping to ADS common data types.
* **`AdsString<N>`**: Maps to `STRING(N-1)` in PLC. Handles CP1252 <-> UTF-8.
* **`AmsNetId`**: The 6-byte identifier (e.g., `172.16.17.20.1.1`).
* **`WindowsFiletime`**: 64-bit timestamp (100ns ticks since 1601).

## Usage Examples

### 1. Creating a Packet (Client Side)

```rust
use tcads_core::protocol::packet::AmsPacket;
use tcads_core::protocol::header::AmsHeader;
use tcads_core::protocol::commands::{CommandId, AdsReadRequest};
use tcads_core::protocol::state_flags::StateFlag;
use tcads_core::types::{AmsAddr, AmsNetId};
use tcads_core::errors::AdsReturnCode;

fn main() {
    // 1. Define Routing
    let target = AmsAddr::new(AmsNetId([5, 1, 2, 3, 1, 1]), 851);
    let source = AmsAddr::new(AmsNetId([192, 168, 0, 10, 1, 1]), 30000);

    // 2. Create Payload (Read 4 bytes from IndexGroup 0x4020, Offset 0)
    let request = AdsReadRequest::new(0x4020, 0, 4);
    let mut payload = Vec::new();
    request.write_to(&mut payload).unwrap();

    // 3. Construct Header
    let header = AmsHeader::new(
        target,
        source,
        CommandId::AdsRead,
        StateFlag::tcp_ads_request(),
        payload.len() as u32,
        AdsReturnCode::Ok,
        1, // Invoke ID
    );

    // 4. Create Packet
    let packet = AmsPacket::new(header, payload);
    
    // 5. Serialize to bytes (e.g., for TCP)
    // Use AmsCodec::write(&mut stream, &packet) in real apps
}
```

### 2. Parsing a Notification (Server Side / Client Receive)

```rust
use tcads_core::protocol::commands::response::{
    AdsDeviceNotificationStreamHeader, AdsStampHeader, AdsNotificationSampleHeader
};
use tcads_core::types::WindowsFiletime;
use std::io::Cursor;
use std::time::SystemTime;

fn parse_notification(data: &[u8]) {
    let mut reader = Cursor::new(data);

    // 1. Read Stream Header
    let stream_header = AdsDeviceNotificationStreamHeader::read_from(&mut reader).unwrap();
    println!("Notification contains {} stamps", stream_header.stamps);

    for _ in 0..stream_header.stamps {
        // 2. Read Stamp (Timestamp)
        let stamp = AdsStampHeader::read_from(&mut reader).unwrap();
        let time: SystemTime = stamp.timestamp.into();
        println!("  Time: {:?}", time);

        for _ in 0..stamp.samples {
            // 3. Read Sample (Handle + Data Size)
            let sample = AdsNotificationSampleHeader::read_from(&mut reader).unwrap();
            
            // 4. Read Data
            let mut value = vec![0u8; sample.sample_size as usize];
            std::io::Read::read_exact(&mut reader, &mut value).unwrap();
            
            println!("    Handle: {}, Data: {:?}", sample.handle, value);
        }
    }
}
```