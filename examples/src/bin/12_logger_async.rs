//! Example 12: Async TwinCAT System Logger
//! Run with: `cargo run --bin 12_logger_async`
//!
//! Demonstrates how to share a single AMS connection between an async `Logger`
//! (for subscribing and writing) and a background listener task, using Tokio.
//!
//! This sits between examples 9 and 10 in complexity: like example 9 it uses
//! only the Logger API without touching PLC symbol handles, but like example 10
//! it shares a connection and runs concurrent tasks.

use std::time::Duration;
use tcads::client::devices::tokio::AdsLogger;
use tcads::core::{AmsNetId, LogMessageType};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Connecting to the TwinCAT System Logger...");

    // 1. Connect directly to the logger.
    // This handles the PortConnect handshake and targets Port 100 automatically.
    let logger = AdsLogger::connect(AmsNetId::local(), Duration::from_secs(5)).await?;

    // 2. Subscribe to the logger notification stream.
    // Clone the logger so the listener task and the main task share the same
    // connection without opening a second socket.
    let (mut rx, handle) = logger.subscribe().await?;
    let logger_clone = logger.clone();
    println!("Subscribed to Logger Notifications!");

    // 3. Spawn a background task to print incoming log entries.
    // The task owns `rx` and stops itself after receiving our marker message.
    let listener = tokio::spawn(async move {
        println!("\nWaiting for logs to arrive...");
        println!("---------------------------------------------------");

        while let Ok(entry) = rx.recv().await {
            println!(
                "{}\t{} | '{}' ({}): {}",
                entry.message_type(),
                entry.timestamp(),
                entry.sender(),
                entry.sender_port(),
                entry.message()
            );

            if entry.message().contains("Goodbye") {
                println!("---------------------------------------------------");
                println!("Goodbye message received. Listener exiting.");
                logger_clone.unsubscribe(handle).await.unwrap();
            }
        }
    });

    // 4. Write a few messages while the listener runs concurrently.
    // We can combine MessageTypes using the standard bitwise OR operator.
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("Writing messages to the logger...");

    logger
        .write_log(
            LogMessageType::HINT | LogMessageType::LOG,
            "RustClient",
            "Hint from tcads-rs.",
        )
        .await?;

    logger
        .write_log(
            LogMessageType::WARNING | LogMessageType::LOG,
            "RustClient",
            "Warning from tcads-rs.",
        )
        .await?;

    // The listener stops when it sees this message.
    logger
        .write_log(
            LogMessageType::ERROR | LogMessageType::LOG,
            "RustClient",
            "Goodbye from tcads-rs!",
        )
        .await?;

    // 5. Wait for the listener to finish, then clean up.
    listener.await?;

    println!("Done.");
    Ok(())
}
