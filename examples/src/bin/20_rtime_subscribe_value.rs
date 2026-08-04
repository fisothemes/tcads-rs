//! Example 20: Subscribing to Symbol Value Changes with Tokio.
//! Run with: `cargo run --bin 20_rtime_subscribe_value`
//!
//! This example demonstrates the asynchronous `subscribe_value` method on the
//! `RuntimeDevice`. This method automatically handles symbol resolution, registers
//! handles under the hood, and streams value updates via an async channel receiver.
//! It also demonstrates what happens when a symbol version changes by surfacing a
//! `HandleInvalidated` error and how to gracefully unsubscribe from a subscription.
//!
//! ## PREREQUISITE
//!
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE, activate the configuration
//! on your local machine, and put the PLC into RUN mode.

use std::time::Duration;
use tcads::client::devices::tokio::AdsRuntime;
use tcads::client::{Error, Result};
use tcads::core::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Connecting to the PLC runtime...");
    // 1. Connect to the local PLC runtime (Port 851)
    let device = AdsRuntime::connect(AmsAddr::from_local(851), None).await?;

    let sym_name = "MAIN.nCount";

    // 2. Subscribe to a variable's value changes
    let (mut rx, notif_handle) = device
        .subscribe_value::<u32>(
            sym_name,
            AdsTransMode::ServerOnChange,
            Duration::ZERO,
            Duration::ZERO,
        )
        .await?;

    println!("Subscribed to {}. Spawning background task...", sym_name);

    // 3. Move the receiver into a background Tokio task
    let task_handle = tokio::spawn(async move {
        for i in 1..=5 {
            match rx.recv().await {
                Ok(value) => {
                    println!("[Background] Event {i}: {sym_name} = {value:?}");
                }
                Err(Error::HandleInvalidated(sym)) => {
                    eprintln!(
                        "[Background] WARNING: Handle for '{sym}' was invalidated by symbol version change!"
                    );
                    break;
                }
                Err(e) => {
                    eprintln!("[Background] Subscription disconnected or failed: {e}");
                    break;
                }
            }
        }
    });

    // 4. Wait for 30 seconds or until the task is completed
    if tokio::time::timeout(Duration::from_secs(30), task_handle)
        .await
        .is_err()
    {
        println!("Timeout reached before 5 events! Unsubscribing...");
        device.unsubscribe_notification(notif_handle).await?;
    }

    Ok(())
}
