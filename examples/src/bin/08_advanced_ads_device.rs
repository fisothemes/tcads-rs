//! Example 8: Advanced AdsDevice (Multithreading & Notifications)
//! Run with: `cargo run --bin 08_advanced_ads_device`
//!
//! This example demonstrates the `Send + Sync` capabilities of `AdsDevice`.
//! It uses `std::thread` to spawn concurrent reader and writer threads
//! that interact with the PLC simultaneously over the same multiplexed connection.
//!
//! ## PREREQUISITE
//!
//! Open `examples/twincat/TcAdsExamples.sln` in TwinCAT XAE, activate
//! the configuration on your local machine, and put the PLC into RUN mode.

use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use tcads::client::devices::blocking::AdsDevice;
use tcads::core::{
    AdsNotificationAttrib, AdsNotificationSampleOwned, AdsTransMode, AmsAddr, IndexGroup,
    IndexOffset, NotificationHandle,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to the local router...");
    let device = AdsDevice::connect(Duration::from_secs(5))?;

    let plc_target = AmsAddr::new(device.get_local_net_id()?, 851);

    // 1. Resolve Symbol Handles
    println!("Resolving symbol handles...");
    let count_handle = get_handle(&device, plc_target, "MAIN.nCount")?;
    let inc_handle = get_handle(&device, plc_target, "MAIN.bIncrement")?;
    let dec_handle = get_handle(&device, plc_target, "MAIN.bDecrement")?;

    // 2. Subscribe to Notifications on `MAIN.nCount`
    println!("Subscribing to MAIN.nCount changes...");
    let (rx, notif_handle) = add_notification::<u32>(&device, plc_target, count_handle)?;

    // ADS `ServerOnChange` always fires one initial notification upon registration
    let initial_sample = rx.recv_timeout(Duration::from_secs(2))?;
    let initial_count = u32::from_le_bytes(initial_sample.data().try_into()?);
    println!("Initial `MAIN.nCount` value: {}\n", initial_count);

    // Notification Thread - Listens for the 5 changes we are about to trigger
    let notif_thread = thread::spawn(move || {
        for i in 1..=5 {
            let sample = rx
                .recv_timeout(Duration::from_secs(2))
                .expect("Notification timeout!");
            let count = u32::from_le_bytes(sample.data().try_into().unwrap());
            println!(
                "[Reader Thread] Notification {} received! nCount: {}",
                i, count
            );
        }
    });

    // Writer Thread - Writes to the PLC to trigger the logic
    let device_clone = device.clone();
    let writer_thread = thread::spawn(move || {
        for _ in 0..3 {
            println!("[Writer Thread] Writing TRUE to bIncrement");
            device_clone
                .write(
                    plc_target,
                    IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                    inc_handle,
                    [true as u8],
                )
                .expect("Failed to write increment");
            thread::sleep(Duration::from_millis(500));
        }

        for _ in 3..5 {
            println!("[Writer Thread] Writing TRUE to bDecrement");
            device_clone
                .write(
                    plc_target,
                    IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                    dec_handle,
                    [true as u8],
                )
                .expect("Failed to write decrement");
            thread::sleep(Duration::from_millis(500));
        }
    });

    notif_thread.join().expect("Notification thread panicked!");
    writer_thread.join().expect("Writer thread panicked!");

    // 4. Clean-up
    println!("\nCleaning up...");
    device.delete_notification(plc_target, notif_handle)?;
    release_handle(&device, plc_target, count_handle)?;
    release_handle(&device, plc_target, inc_handle)?;
    release_handle(&device, plc_target, dec_handle)?;

    println!("Example completed successfully!");
    Ok(())
}

/// Helper function to fetch a symbol handle by name.
fn get_handle(device: &AdsDevice, target: AmsAddr, symbol_name: &str) -> Result<IndexOffset> {
    let mut name_bytes = symbol_name.as_bytes().to_vec();
    name_bytes.push(0); // Null-terminate the string for good measure

    let handle_bytes = device.read_write(
        target,
        IndexGroup::SYMBOL_HANDLE_BY_NAME,
        IndexOffset::ZERO,
        4,
        name_bytes,
    )?;
    Ok(IndexOffset::from_bytes(handle_bytes.try_into().unwrap()))
}

/// Helper function to release a symbol handle.
fn release_handle(device: &AdsDevice, target: AmsAddr, handle: IndexOffset) -> Result<()> {
    device.write(
        target,
        IndexGroup::SYMBOL_RELEASE_HANDLE,
        IndexOffset::ZERO,
        handle.to_bytes(),
    )?;
    Ok(())
}

/// Helper function to register a notification for a symbol.
fn add_notification<T: Sized>(
    device: &AdsDevice,
    target: AmsAddr,
    handle: IndexOffset,
) -> Result<(Receiver<AdsNotificationSampleOwned>, NotificationHandle)> {
    Ok(device.add_notification(
        target,
        IndexGroup::SYMBOL_VALUE_BY_HANDLE,
        handle,
        AdsNotificationAttrib {
            length: size_of::<T>() as u32,
            trans_mode: AdsTransMode::ServerOnChange,
            max_delay: 0,
            cycle_time: 10_000 * 10, // 10 ms
        },
    )?)
}
