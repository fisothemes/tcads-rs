//! Example 28: Reaching Past a Client's API with `AdsDeviceExt`
//! Run with: `cargo run --bin 28_ads_device_ext`
//!
//! Every subsystem client (`AdsRuntime`, `AdsLogger`, `AdsSystemService`, `AdsLicense`, etc.)
//! implements `AdsDeviceExt`. Alongside its own typed methods, each client therefore also
//! exposes the raw ADS commands (`read_device_info`, `read_state`, `read`, `write`,
//! `read_write`, `add_notification`, ...), already aimed at that client's port.
//!
//! This is the escape hatch when a client doesn't wrap something you need. At the time this
//! example was created, `AdsSystemService` had no method for retrieving the host OS's name, but
//! the System Service device has that capability on index group 702. Through the trait, we
//! can issue that read ourselves and decode the reply as a C string.
//!
//! ## PREREQUISITE
//!
//! The TwinCAT System Service must be running on the local machine (any mode).

use tcads::client::Result;
use tcads::client::devices::blocking::{AdsDeviceExt, AdsSystemService};
use tcads::core::{AmsNetId, IndexGroup, IndexOffset};

fn main() -> Result<()> {
    let sys_svc = AdsSystemService::connect(AmsNetId::local(), None)?;

    // Plain ADS commands every client gets from the trait.
    let (version, name) = sys_svc.read_device_info()?;
    let (ads_state, _) = sys_svc.read_state()?;
    println!("{name} v{version} on {}, {ads_state:?}", sys_svc.target());

    // A raw read for something the client doesn't wrap. The reply is a null-terminated
    // string, and 128 bytes is room to spare: the device sends back only what it has.
    let bytes = sys_svc.read(IndexGroup::new(702), IndexOffset::ZERO, 128)?;
    let host_name = c_string(&bytes);
    println!("Host name: {host_name}");

    Ok(())
}

/// Reads a C string: everything up to the first null byte.
///
/// Uses a lossy conversion because Windows hosts may send non-UTF-8 (Windows-1252)
/// characters, which shouldn't crash the example.
fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
