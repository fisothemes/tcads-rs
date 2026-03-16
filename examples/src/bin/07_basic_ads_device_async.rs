//! Example 7: Basic Asynchronous AdsDevice (Tokio)
//! Run with: `cargo run --bin 07_basic_ads_device_async`
//!
//! This example mirrors Example 6 exactly but utilizes the `tokio`
//! asynchronous runtime. Notice how the API shape remains identical;
//! you only need to change the import path and await the futures!

use tcads::client::AmsAddr;
use tcads::client::devices::tokio::AdsDevice;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Connecting asynchronously to the local router...");

    // 1. Connect using the Tokio AdsDevice.
    // This spawns non-blocking Tokio tasks for reading and writing!
    let device = AdsDevice::connect(None).await?;

    // 2. Ask the router for its Local Net ID
    let local_net_id = device.get_local_net_id().await?;
    println!("Local Net ID: {}", local_net_id);

    // 3. Define our target (The standard TwinCAT PLC runtime on Port 851)
    let plc_target = AmsAddr::new(local_net_id, 851);

    // 4. Read Device Info asynchronously
    let (version, name) = device.read_device_info(plc_target).await?;
    println!("--------------------------------");
    println!("Target PLC Name: {}", name);
    println!("Target PLC Version: {:?}", version);

    // 5. Read the PLC's execution state
    let (ads_state, device_state) = device.read_state(plc_target).await?;
    println!("Current PLC State: {:?}", ads_state);
    println!("Internal Device State: {}", device_state);

    Ok(())
}
