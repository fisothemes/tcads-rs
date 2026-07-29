//! Example 15: Reading and writing raw bytes using dynamic Handles.
//! Run with: `cargo run --bin 15_rtime_raw_handle`
//!
//! This example demonstrates how to ask the TwinCAT runtime device server for a dynamic handle
//! to a variable, read its raw bytes, parse them manually using `from_le_bytes`, and write them
//! back.
//!
//! ## Note
//!
//!  - Handles are fast but must always be released to prevent PLC memory leaks!
//!  - Handles automatically resolve PLC `REFERENCE TO` and `VAR_IN_OUT` variables under the hood.
//!
//! ## PREREQUISITE
//!
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE,
//! activate the configuration on your local machine, and put the PLC into RUN mode.

use tcads::client::devices::blocking::AdsRuntime;
use tcads::core::AmsAddr;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to the PLC runtime...");
    // 1. Connect to the local PLC runtime (Port 851)
    let device = AdsRuntime::connect(AmsAddr::from_local(851), None)?;

    // 2. Ask the server for a symbol handle.
    let symbol_name = "MAIN.nCount";
    let handle = device.get_handle_by_name(symbol_name)?;
    println!("Acquired handle {handle:?} for '{symbol_name}'");

    // 3. Read the raw bytes using the handle
    // (We must know that MAIN.nCounter is a 4-byte UDINT/DINT)
    let buf = device.read_bytes_by_handle(handle, 4)?;

    // 4. Manually parse the little-endian bytes
    let mut value = u32::from_le_bytes(buf.try_into().unwrap());
    println!("Current value: {}", value);

    // 5. Mutate and pack the bytes back to Little Endian
    value += 1;
    device.write_bytes_by_handle(handle, value.to_le_bytes())?;
    println!("Wrote new value: {}", value);

    // 6. ALWAYS release handles when done to prevent PLC memory leaks!
    device.release_handle(handle)?;
    println!("Handle released safely.");

    Ok(())
}
