//! Example 11: Advanced Shared Logger
//! Run with: `cargo run --bin 11_logger_advanced`
//!
//! Demonstrates multiplexing: sharing a single AMS connection between a
//! standard `AdsDevice` (for PLC control) and a `Logger` (for system monitoring).
//!
//! ## PREREQUISITE
//!
//! Open `examples/twincat/TcAdsExamples.sln` and put the PLC into RUN mode.

use std::time::Duration;
use tcads::client::devices::blocking::{AdsDevice, Logger};
use tcads::core::{AmsAddr, IndexGroup, IndexOffset};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting the base AdsDevice...");
    // 1. Establish our single underlying router connection
    let device = AdsDevice::connect(Duration::from_secs(5))?;
    let local_net_id = device.get_local_net_id()?;
    let plc_target = AmsAddr::new(local_net_id, 851);

    // 2. Wrap the existing connection in our Logger client!
    // This does NOT open a new socket; it reuses the dispatcher.
    let logger = Logger::new(device.clone(), local_net_id);
    let (logger_rx, _handle) = logger.subscribe()?;

    // 3. Resolve the handle for our trigger variable
    let inc_handle = get_handle(&device, plc_target, "MAIN.bIncrement")?;

    println!("Starting automated trigger test...");

    // 4. Multithreading: Run the Logger and PLC Writer concurrently
    std::thread::scope(|s| {
        // --- LOGGER THREAD ---
        s.spawn(move || {
            // Listen for 3 log events
            for (i, result) in logger_rx.iter().take(3).enumerate() {
                if let Ok(entry) = result {
                    println!("[Logger] Event {}: {}", i + 1, entry.message());
                }
            }
            println!("[Logger] Finished listening.");
        });

        // --- WRITER THREAD ---
        s.spawn(|| {
            // Write TRUE to `bIncrement` 3 times to trigger the PLC logic
            // (Assuming the PLC has an ADSLOGSTR or similar triggered by this)
            for i in 1..=3 {
                std::thread::sleep(Duration::from_millis(500));
                println!("[Writer] Triggering PLC increment {}...", i);

                device
                    .write(
                        plc_target,
                        IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                        inc_handle,
                        [true as u8],
                    )
                    .expect("Failed to write to PLC");
            }
        });
    });

    // 5. Cleanup
    println!("Cleaning up symbol handles...");
    release_handle(&device, plc_target, inc_handle)?;

    // Note: The Logger's notification handle is automatically deleted here
    // because `logger_rx` goes out of scope and its `Drop` implementation fires!

    Ok(())
}

/// Helper function to fetch a symbol handle by name
fn get_handle(device: &AdsDevice, target: AmsAddr, symbol_name: &str) -> Result<IndexOffset> {
    let mut name_bytes = symbol_name.as_bytes().to_vec();
    name_bytes.push(0);
    let handle_bytes = device.read_write(
        target,
        IndexGroup::SYMBOL_HANDLE_BY_NAME,
        IndexOffset::ZERO,
        4,
        name_bytes,
    )?;
    Ok(IndexOffset::from_bytes(handle_bytes.try_into().unwrap()))
}

/// Helper function to release a symbol handle
fn release_handle(device: &AdsDevice, target: AmsAddr, handle: IndexOffset) -> Result<()> {
    device.write(
        target,
        IndexGroup::SYMBOL_RELEASE_HANDLE,
        IndexOffset::ZERO,
        handle.to_bytes(),
    )?;
    Ok(())
}
