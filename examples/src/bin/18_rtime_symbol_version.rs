//! Example 18: Monitoring Symbol Version.
//! Run with: `cargo run --bin 18_rtime_symbol_version`
//!
//! This example demonstrates how to monitor the PLC's Symbol Version using
//! the`RuntimeDevice`.
//!
//! The Symbol Version is a 1-byte counter maintained by the TwinCAT Runtime ADS Server. It
//! increments on a **Login with download**. It does *not* increment during a standard
//! **Online Change**.
//!
//! By setting up a device notification, a client can detect these changes in real-time.

use tcads::client::devices::blocking::RuntimeDevice;
use tcads::core::AmsAddr;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to the PLC runtime...");
    // 1. Connect to the local PLC runtime (Port 851)
    let device = RuntimeDevice::connect(AmsAddr::from_local(851), None)?;

    // 2. Fetch the symbol version.
    let version = device.get_symbol_version()?;
    println!("Symbol Version: {}", version);

    // 3. Subscribe to symbol version changes.
    let (recv, _handle) = device.subscribe_symbol_version()?;

    for version in recv.iter() {
        println!("Symbol Version Changed: {}", version?);
    }

    Ok(())
}
