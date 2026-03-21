//! Example 9: Basic TwinCAT System Logger
//! Run with: `cargo run --bin 09_basic_system_logger`
//!
//! Demonstrates how to quickly connect to the TwinCAT Event Logger
//! (Port 100) and listen for system messages using the standalone
//! `Logger::connect()` method.
//!
//! PREREQUISITE:
//! Open `examples/twincat/TcAdsExamples.sln` and put the PLC into RUN mode.
//! Toggle `MAIN.bIncrement` or `MAIN.bDecrement` manually to generate logs, or
//! start/stop the PLC to see system state logs.

use std::time::Duration;
use tcads::client::devices::blocking::Logger;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to the TwinCAT System Logger...");

    // 1. Connect directly to the logger
    // This handles the port handshake and targets Port 100 automatically.
    let logger = Logger::connect(Duration::from_secs(5))?;

    // 2. Subscribe to the logger notification stream
    println!("Subscribed! Waiting for logs...");
    println!(
        "(Try toggling `MAIN.bIncrement` or `MAIN.bDecrement` in TwinCAT or restarting the PLC)"
    );
    println!("---------------------------------------------------");

    let (rx, handle) = logger.subscribe()?;

    // 3. Listen for events using the blocking iterator
    // We will listen for exactly 3 events before exiting.
    for (i, result) in rx.iter().enumerate() {
        let entry = result?;

        println!(
            "{}\t{} | '{}' ({}): {}",
            entry.message_type(),
            entry.timestamp(),
            entry.sender(),
            entry.sender_port(),
            entry.message()
        );

        if i == 2 {
            // Exit after 3 messages (0, 1, 2)
            println!("Received 3 messages. Unsubscribing...");
            // Explicitly unsubscribe (though dropping `rx` also cleans up automatically!)
            logger.unsubscribe(handle)?;
        }
    }

    Ok(())
}
