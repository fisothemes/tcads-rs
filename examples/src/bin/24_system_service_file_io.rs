//! Example 24: System Service Remote File Operations
//! Run with: `cargo run --bin 24_system_service_file_io`
//!
//! This example demonstrates remote file I/O operations against the TwinCAT host OS
//! via the System Service (Port 10000). It covers creating, writing, seeking, reading,
//! retrieving status metadata, and deleting files.
//!
//! ## PREREQUISITE
//!
//! Ensure TwinCAT System Service is running on your local machine or target IPC.

use tcads::client::devices::blocking::AdsSystemService;
use tcads::core::{AdsFileFlags, AdsFilePathType, AdsFileSeekOrigin};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to local System Service (Port 10000)...");
    let device = AdsSystemService::connect_local(None)?;

    let test_path = r#"C:\TwinCAT\3.1\Boot\tcads_example_test.txt"#;

    // 1. Open a new file for writing
    println!("1. Creating remote file: {}", test_path);
    let flags = AdsFileFlags::APPEND | AdsFileFlags::PLUS; // Append to existing file, create if not exists
    let handle = device.open_file(test_path, AdsFilePathType::Generic, flags)?;

    // 2. Write data to the remote file
    let message = b"Hello from tcads-rs System Service client!";
    println!("2. Writing {} bytes to remote file...", message.len());
    let written = device.write_file(handle, message)?;
    println!("   Successfully wrote {} bytes.", written);

    // 3. Seek back to the start of the file
    println!("3. Seeking to origin of file...");
    device.seek_file(handle, 0, AdsFileSeekOrigin::Set)?;

    // 4. Read back the written data
    println!("4. Reading file contents back...");
    let mut buffer = [0u8; 64];
    let read_bytes = device.read_file(handle, &mut buffer)?;
    let content = std::str::from_utf8(&buffer[..read_bytes])?;
    println!("   Read {} bytes: \"{}\"", read_bytes, content);

    // 5. Close the remote file handle
    println!("5. Closing file handle...");
    device.close_file(handle)?;

    // 6. Inspect file status and attributes
    println!("6. Reading file status metadata...");
    let status = device.get_file_status(test_path, AdsFilePathType::Generic)?;
    println!("   File Size: {} bytes", status.size());
    println!("   Attributes: {:#X}", status.attributes());

    // 7. Delete the test file
    println!("7. Cleaning up remote test file...");
    device.delete_file(test_path, AdsFilePathType::Generic)?;
    println!("   File deleted successfully.");

    Ok(())
}
