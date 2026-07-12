//! Example 16: Reading and writing raw bytes using Symbol Info (Async).
//! Run with: `cargo run --bin 16_rtime_raw_symbol`
//!
//! This example demonstrates how to bypass handles and use absolute memory offsets
//! (`IndexGroup` and `IndexOffset`) via `AdsSymbolInfo` using the `tokio` async client.
//!
//! This is safer than handles as there are no memory leaks.
//!
//! ## PREREQUISITE
//!
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE,
//! activate the configuration on your local machine, and put the PLC into RUN mode.

use tcads::client::devices::tokio::RuntimeDevice;
use tcads::core::AmsAddr;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Connecting to the PLC runtime...");
    let device = RuntimeDevice::connect(AmsAddr::from_local(851), None).await?;

    // 2. Ask the server for a symbol's information.
    let symbol_name = "MAIN.nCount";
    let sym_info = device.get_symbol_info(symbol_name).await?;

    println!(
        "Found '{}' at Index Group: {}, Index Offset: {}",
        symbol_name,
        sym_info.index_group(),
        sym_info.index_offset()
    );

    // 2. Read raw bytes using symbol information.
    let buf = device.read_bytes_by_info(&sym_info).await?;

    // 3. Manually parse the little-endian bytes
    let mut value = u32::from_le_bytes(buf.try_into().unwrap());
    println!("Current value: {}", value);

    // 4. Mutate and pack the bytes back to Little Endian
    value += 1;
    device
        .write_bytes_by_info(&sym_info, value.to_le_bytes())
        .await?;
    println!("Wrote new value: {}", value);

    Ok(())
}
