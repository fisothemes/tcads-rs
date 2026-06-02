//! Example 14: Symbol Information Exploration
//! Run with: `cargo run --bin 14_symbol_info_exploration`
//!
//! Demonstrates how to query symbol metadata from a TwinCAT PLC runtime.
//! It shows how to fetch upload statistics, look up a single symbol by name,
//! and perform a full scan of every symbol in the PLC's symbol table.
//!
//! PREREQUISITE:
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE,
//! activate the configuration on your local machine, and put the PLC into RUN mode.

use std::time::Duration;
use tcads::client::devices::blocking::SymbolInfoDevice;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to the PLC runtime...");
    // 1. Establish a connection to the first PLC runtime (Port 851, local AMS router)
    let device = SymbolInfoDevice::connect(851, Duration::from_secs(5))?;

    // 2. Retrieve Symbol Table Metadata
    // The upload info describes how many symbols exist and how large the raw blob is.
    let upload_info = device.get_upload_info()?;

    println!("Symbol Count:      {}", upload_info.symbol_count());
    println!(
        "Symbol Blob Size:  {} bytes",
        upload_info.symbol_blob_size()
    );
    println!("{:-<76}\n", "");

    // 3. Targeted Symbol Lookup
    // Fetch the full metadata record for a single symbol by its instance path.
    let lookup_name = "MAIN.nCount";
    if let Ok(sym) = device.get_symbol_info(lookup_name) {
        println!("Targeted Lookup [{lookup_name}]:");
        println!("  Type:    {}", sym.type_name());
        println!("  Size:    {} bytes", sym.size());
        println!("  Comment: {}", sym.comment());
        println!(
            "  Index:   {:#010X}:{:#010X}",
            sym.index_group(),
            sym.index_offset()
        );
        println!();
    }

    // 4. Full Symbol Table Scan
    // Download the entire symbol blob in one round-trip and iterate lazily.
    println!("{:<45} | {:<20} | {:<6}", "Name", "Type", "Size");
    println!("{:-<45}-+-{:-<20}-+-{:-<6}", "", "", "");

    device
        .get_all_symbol_info()?
        .filter_map(|res| res.ok()) // Skip any corrupted entries
        .for_each(|sym| {
            println!(
                "{:<45} | {:<20} | {:<6}",
                truncate(sym.name(), 45),
                truncate(sym.type_name(), 20),
                sym.size(),
            )
        });

    Ok(())
}

/// Truncates a string to `max` characters, appending `…` if needed.
fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max - 3])
    } else {
        s.into()
    }
}
