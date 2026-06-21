//! Example 12: Advanced Runtime Type System Exploration
//! Run with: `cargo run --bin 13_advanced_rtime_type_exploration`
//!
//! Demonstrates how to perform a deep-scan of a TwinCAT PLC's data type system.
//! It shows how to classify types (Structs, FBs, Enums) dynamically based on the target
//! platform's architecture.
//!
//! PREREQUISITE:
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE,
//! activate the configuration on your local machine, and put the PLC into RUN mode.

use std::time::Duration;
use tcads::client::devices::blocking::RuntimeDevice;
use tcads::core::{AdsTypeCategory, AmsAddr};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to the PLC runtime...");
    // 1. Establish a connection a PLC runtime (Port 851 and local AMS router)
    let device = RuntimeDevice::connect(AmsAddr::from_local(851), Duration::from_secs(5))?;

    // 2. Retrieve Architectural Metadata
    // We need the platform pointer size (4 or 8 bytes) to accurately
    // distinguish between Interfaces and Function Blocks.
    let upload_info = device.get_upload_info()?;
    let ptr_size = upload_info.platform_ptr_size().unwrap();

    println!("Target Architecture: {}-bit", ptr_size * 8);
    println!("Total Symbols:       {}", upload_info.symbol_count());
    println!(
        "Total Data Types:    {}",
        upload_info.data_type_count().unwrap()
    );
    println!("{:-<76}\n", "");

    // 3. Targeted Type Lookup
    // Demonstrates fetching the blueprint of a specific type by its name.
    let lookup_type_name = "ST_LibVersion";
    if let Ok(dt_info) = device.get_type_info(lookup_type_name) {
        println!("Targeted Lookup [{lookup_type_name}]:");
        println!("  Size:    {} bytes", dt_info.size());
        println!("  Fields:  {}", dt_info.field_infos().len());
        println!("  Comment: {}", dt_info.comment());
        println!();
    }

    // 4. Comprehensive Type System Scan
    // We fetch the entire type blob and use our category helper to classify them.
    println!(
        "{:<20} | {:<35} | {:<6} | {:<5}",
        "Category", "Name", "Size", "Fields"
    );
    println!("{:-<20}-+-{:-<35}-+-{:-<6}-+-{:-<6}", "", "", "", "");

    device
        .get_all_type_infos()?
        .filter_map(|res| res.ok()) // Skip any corrupted entries
        .filter(|info| !(AdsTypeCategory::is_primitive(info) | AdsTypeCategory::is_alias(info))) // Skip all primitives and aliases
        .for_each(|info| {
            let category = AdsTypeCategory::determine(&info, ptr_size);
            println!(
                "{:<20} | {:<35} | {:<6} | {:<5}",
                format!("{:?}", category),
                truncate_name(info.name(), 35),
                info.size(),
                info.field_infos().len()
            )
        });

    Ok(())
}

/// Helper to keep the console output tidy.
fn truncate_name(name: &str, max: usize) -> String {
    if name.len() > max {
        format!("{}...", &name[..max - 3])
    } else {
        name.into()
    }
}
