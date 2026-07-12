//! Example 13: Batch requests (Sum Commands).
//! Run with: `cargo run --bin 09_batch_requests`
//!
//! This example demonstrates how to use the `SumDevice` to resolve multiple symbol
//! handles, register them for notifications in a single network round-trip,
//! stream the data in background threads, and gracefully tear everything down.
//!
//! ## PREREQUISITE
//!
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE,
//! activate the configuration on your local machine, and put the PLC into RUN mode.

use std::time::Duration;
use tcads::client::devices::blocking::AdsDevice;
use tcads::core::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    // 1. Connect to the local router
    let device = AdsDevice::connect(Duration::from_secs(5))?;
    let target = AmsAddr::new(device.get_local_net_id()?, 851);
    println!("Connected! Target PLC: {}", target);

    // 2. Resolve symbol handles
    let reqs = [
        SumReadWriteRequest::new(
            IndexGroup::SYMBOL_HANDLE_BY_NAME,
            IndexOffset::ZERO,
            4,
            b"MAIN.bIncrement\0",
        ),
        SumReadWriteRequest::new(
            IndexGroup::SYMBOL_HANDLE_BY_NAME,
            IndexOffset::ZERO,
            4,
            b"MAIN.nCount\0",
        ),
    ];

    println!("Resolving symbol handles...");
    let handles: Vec<u32> = device
        .read_write_multi(target, &reqs)?
        .iter()
        .enumerate()
        .map(|(i, res)| {
            let bytes = res.unwrap_or_else(|e| panic!("Failed to resolve symbol {}: {:?}", i, e));
            u32::from_le_bytes(bytes.try_into().unwrap())
        })
        .collect();

    println!("  MAIN.bIncrement Handle: {}", handles[0]);
    println!("  MAIN.nCount Handle: {}", handles[1]);

    // 3. Register batch notifications
    let reqs = [
        SumAddNotificationRequest::new(
            IndexGroup::SYMBOL_VALUE_BY_HANDLE,
            IndexOffset::from(handles[0]),
            AdsNotificationAttrib::new(1, AdsTransMode::ServerOnChange, 0, 100_000),
        ),
        SumAddNotificationRequest::new(
            IndexGroup::SYMBOL_VALUE_BY_HANDLE,
            IndexOffset::from(handles[1]),
            AdsNotificationAttrib::new(4, AdsTransMode::ServerOnChange, 0, 100_000),
        ),
    ];

    println!("Registering notifications...");
    let results = device.add_multi_notifications(target, &reqs)?;

    let mut notif_handles = Vec::new();
    let mut thread_handles = Vec::new();

    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok((handle, rx)) => {
                notif_handles.push(handle);

                // Spawn a listener thread for each successful registration
                let t = std::thread::spawn(move || {
                    // This loop will automatically break when the channel is dropped!
                    while let Ok(sample) = rx.recv() {
                        println!("[Thread {}] Data Received: {:?}", i, sample.data());
                    }
                    println!("[Thread {}] Channel closed. Thread exiting gracefully.", i);
                });

                thread_handles.push(t);
            }
            Err(e) => eprintln!("Failed to register notification {}: {:?}", i, e),
        }
    }

    // 4. Run the system
    println!("Listening for 5 seconds... (Change values in TwinCAT to see data!)");
    std::thread::sleep(Duration::from_secs(5));

    // 5. Tear everything down gracefully
    println!("Initiating teardown...");
    let del_results = device.delete_multi_notifications(target, &notif_handles)?;

    for (i, res) in del_results.iter().enumerate() {
        match res {
            Ok(_) => println!("  Successfully deleted notification on PLC."),
            Err(e) => eprintln!("  Failed to delete notification {}: {:?}", i, e),
        }
    }

    // Because the notifications were deleted, the internal dispatcher dropped the
    // Senders. This closes the Receivers, causing `rx.recv()` in the threads to fail.
    println!("Waiting for background threads to terminate...");
    for t in thread_handles {
        let _ = t.join();
    }

    println!("All threads cleanly exited. Done!");
    Ok(())
}
