//! Example 28: Reading Installed Licenses.
//! Run with: `cargo run --bin 26_license_info`
//!
//! This example demonstrates how to extract license information from the license server
//! (port 30) with `AdsLicense`:
//! - Reading the target's system ID, platform ID, and volume number.
//! - Listing every installed license with its order number, instance count,
//!   status, expiry, and name.
//!
//! The license list is printed in the same layout as Beckhoff's
//! `adstool <netid> license onlineinfo`, so the two can be compared directly.
//!
//! ## PREREQUISITE
//!
//! TwinCAT must be running on the target, in either Config or Run mode. No PLC
//! project is needed. On a development machine without purchased licenses,
//! expect the list to contain 7-day trial licenses.

use tcads::client::Result;
use tcads::client::devices::blocking::AdsLicense;

fn main() -> Result<()> {
    println!("Connecting to the license server...");
    let license = AdsLicense::connect_local(None)?;

    // System identity. The system ID is what Beckhoff licenses are bound to.
    println!("System ID:   {}", license.get_system_id()?);
    println!("Platform ID: {}", license.get_platform_id()?);
    println!("Volume No.:  {}", license.get_volume_no()?);
    println!();

    let licenses = license.get_online_info()?;

    if licenses.is_empty() {
        println!("No licenses installed.");
        return Ok(());
    }

    println!("{} license(s) installed:\n", licenses.len());
    println!(
        "{:<14} {:<10} {:<16} {:<26} License",
        "OrderNo.", "Instances", "Status", "Expires"
    );

    for info in &licenses {
        // The order number and name aren't part of the entry itself. Each one is a
        // separate request, keyed by the license ID.
        let order_no = license.get_order_number(info.license_id())?;
        let name = license.get_license_name(info.license_id())?;

        // A count of 0 means the license is per-CPU rather than a fixed number of
        // instances, which is how adstool prints it too.
        let instances = match info.count() {
            0 => "CPU".to_string(),
            n => n.to_string(),
        };

        let expires = match info.expire_time() {
            Some(t) => t.to_string(),
            None => "never".to_string(),
        };

        println!(
            "{:<14} {:<10} {:<16} {:<26} {}",
            or_na(&order_no),
            instances,
            info.status(),
            expires,
            or_na(&name),
        );
    }

    Ok(())
}

/// Prints "N/A" for an empty string, matching how adstool handles a license with no
/// name or order number.
fn or_na(s: &str) -> &str {
    if s.is_empty() { "N/A" } else { s }
}
