//! Example 10: Advanced Shared Logger
//! Run with: `cargo run --bin 10_advanced_system_logger`
//!
//! Demonstrates multiplexing: sharing a single AMS connection between a
//! standard `AdsDevice` (for PLC control) and a `Logger` (for system monitoring).
//!
//! PREREQUISITE:
//! Open `examples/twincat/TcAdsExamples.sln` and put the PLC into RUN mode.

use std::time::Duration;
use tcads::client::AmsAddr;
use tcads::client::devices::blocking::{AdsDevice, Logger};

const GET_SYMHANDLE_BYNAME: u32 = 0xF003;
const READ_WRITE_SYMVAL_BYHANDLE: u32 = 0xF005;
const RELEASE_SYMHANDLE: u32 = 0xF006;

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
                        READ_WRITE_SYMVAL_BYHANDLE,
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
fn get_handle(device: &AdsDevice, target: AmsAddr, symbol_name: &str) -> Result<u32> {
    let mut name_bytes = symbol_name.as_bytes().to_vec();
    name_bytes.push(0);
    let handle_bytes = device.read_write(target, GET_SYMHANDLE_BYNAME, 0, 4, name_bytes)?;
    Ok(u32::from_le_bytes(handle_bytes.try_into().unwrap()))
}

/// Helper function to release a symbol handle
fn release_handle(device: &AdsDevice, target: AmsAddr, handle: u32) -> Result<()> {
    device.write(target, RELEASE_SYMHANDLE, 0, handle.to_le_bytes())?;
    Ok(())
}
