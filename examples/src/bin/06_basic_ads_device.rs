//! Example 6: Basic AdsDevice Usage
//! Run with: `cargo run --bin 06_basic_ads_device`
//!
//! This example shows the absolute basics of using the high-level `AdsDevice` client.
//! Notice how it eliminates the need to manage TCP sockets,
//! AMS headers, or manual byte packing!
//!
//! PREREQUISITE:
//! Open `twincat/TcAdsExamples/TcAdsExamples.sln` in TwinCAT XAE,
//! activate the configuration on your local machine, and put the PLC into RUN mode.

use tcads::client::AmsAddr;
use tcads::client::devices::blocking::AdsDevice;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to the local router...");

    // 1. Connect to the local AMS router (`127.0.0.1:48898`).
    // This automatically handles the PortConnect handshake and
    // spawns background threads to manage incoming and outgoing network traffic.
    let device = AdsDevice::connect(None)?;

    // 2. Ask the router for its Local Net ID
    let local_net_id = device.get_local_net_id()?;
    println!("Local Net ID: {}", local_net_id);

    // 3. Define our target (The standard TwinCAT PLC runtime on Port 851)
    let plc_target = AmsAddr::new(local_net_id, 851);

    // 4. Read Device Info
    // No `AdsReadDeviceInfoRequest` or `.into_frame()` needed here.
    let (version, name) = device.read_device_info(plc_target)?;
    println!("--------------------------------");
    println!("Target PLC Name: {}", name);
    println!("Target PLC Version: {}", version);

    // 5. Read the PLC's execution state (e.g., Run, Stop, Config)
    let (ads_state, device_state) = device.read_state(plc_target)?;
    println!("Current PLC State: {:?}", ads_state);
    println!("Internal Device State: {}", device_state);

    Ok(())
}
