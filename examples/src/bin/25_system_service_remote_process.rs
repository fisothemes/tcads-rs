//! Example 25: System Service Process Execution
//! Run with: `cargo run --bin 25_system_service_remote_process`
//!
//! This example demonstrates how to spawn a process on the host operating system
//! via the TwinCAT System Service (Port 10000).
//!
//! ## PREREQUISITE
//!
//! Ensure TwinCAT System Service is running on your local machine or target IPC.

use tcads::client::Result;
use tcads::client::devices::blocking::AdsSystemService;

fn main() -> Result<()> {
    println!("Connecting to local System Service (Port 10000)...");
    let device = AdsSystemService::connect_local(None)?;

    // 1. Query the target host's local system time
    let local_time = device.get_time_local()?;
    println!("Host local system time: {}", local_time);

    // 2. Start a process on the host operating system
    println!("Spawning background process on target host OS...");
    device.start_process_on_host(
        r#"C:\Windows\System32\cmd.exe"#,
        r#"C:\TwinCAT\3.1\Boot"#,
        r#"/C echo Process started by tcads-rs > tcads_process_test.txt"#,
        true, // hidden process
    )?;

    println!("Process launched successfully on host system.");

    Ok(())
}
