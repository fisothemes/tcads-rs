//! Example 23: System Service Target Information
//! Run with: `cargo run --bin 23_system_service_target_info`
//!
//! This example demonstrates how to interact with the TwinCAT System Service (Port 10000)
//! to query hardware, OS platform, TwinCAT product version, and active project metadata.
//!
//! ## PREREQUISITE
//!
//! Ensure TwinCAT System Service and a TwinCAT Project is running on your local machine
//! or target IPC.

use tcads::client::Result;
use tcads::client::devices::blocking::AdsSystemService;

fn main() -> Result<()> {
    println!("Connecting to local System Service (Port 10000)...");

    // 1. Connect to the local System Service via local AMS router
    let device = AdsSystemService::connect_local(None)?;

    // 2. Read TwinCAT Product Version
    let version = device.get_product_version()?;
    println!("--------------------------------");
    println!("TwinCAT Product Version: {}", version);

    // 3. Read Target Device Type and Platform
    let target_type = device.get_target_type()?;
    let platform = device.get_target_platform()?;
    println!("Target Category: {:?}", target_type);
    println!("Target Platform: {}", platform);

    // 4. Read Active Project Name (if loaded)
    if let Ok(project_name) = device.get_target_project_name() {
        println!("Active Project Name: {}", project_name);
    }

    // 5. Read the complete raw XML target description manifest
    println!("--------------------------------");
    println!("Fetching raw target info XML manifest...");
    let xml_manifest = device.get_target_info()?;
    println!("{}", xml_manifest);

    Ok(())
}
