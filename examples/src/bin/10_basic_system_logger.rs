//! Example 9: Basic TwinCAT System Logger
//! Run with: `cargo run --bin 10_basic_system_logger`
//!
//! Demonstrates how to connect to the TwinCAT Event Logger (Port 100),
//! subscribe to system messages, and write our own custom log messages.
//!
//! PREREQUISITE:
//! Open `examples/twincat/TcAdsExamples.sln` and put the PLC into RUN mode.
//! Toggle `MAIN.bIncrement` or `MAIN.bDecrement` manually to generate logs, or
//! start/stop the PLC to see system state logs.

use std::time::Duration;
use tcads::client::devices::blocking::Logger;
use tcads::core::LogMessageType;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to the TwinCAT System Logger...");

    // 1. Connect directly to the logger
    // This handles the port handshake and targets Port 100 automatically.
    let logger = Logger::connect(Duration::from_secs(5))?;

    // 2. Subscribe to the logger notification stream
    let (rx, handle) = logger.subscribe()?;
    println!("Subscribed to Logger Notifications!");

    // 3. Write a custom message to the TwinCAT Logger!
    // We can combine MessageTypes using standard bitwise OR (|)
    println!("Writing a test message to the logger...");
    logger.write_log(
        (LogMessageType::WARNING | LogMessageType::LOG).into(),
        "RustClient",
        "Hello from tcads-rs! This is a custom log message.",
    )?;

    println!("\nWaiting for logs to arrive...");
    println!("---------------------------------------------------");

    // 4. Listen for events using the blocking iterator
    // We will easily catch the message we just sent, plus any other system noise!
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

        // Exit after we receive our message (or 3 total messages)
        if entry.message().contains("tcads-rs") || i == 2 {
            println!("Test message received. Unsubscribing...");
            logger.unsubscribe(handle)?;
        }
    }

    Ok(())
}
