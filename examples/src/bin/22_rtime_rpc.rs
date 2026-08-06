//! Example 22: Remote Procedure Calls (RPC).
//! Run with: `cargo run --bin 22_rtime_rpc`
//!
//! This example demonstrates how to invoke methods on a TwinCAT Function Block
//! using the `rpc` method. It showcases the "0, 1, N" arity rule for inputs
//! and outputs:
//! - 0 parameters: Use the unit type `()`
//! - 1 parameter: Use the bare type (e.g., `i32`)
//! - N parameters: Use a tightly packed tuple (e.g., `(i32, i32)`)
//!
//! ## PREREQUISITE
//!
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE, activate the configuration
//! on your local machine, and put the PLC into RUN mode.

use tcads::client::Result;
use tcads::client::devices::blocking::AdsRuntime;
use tcads::core::AmsAddr;

fn main() -> Result<()> {
    println!("Connecting to the PLC runtime...");
    let device = AdsRuntime::connect(AmsAddr::from_local(851), None)?;

    let fb_path = "MAIN.fbMath";

    // 1. Zero Inputs, Zero Outputs
    // Both sides use the unit type `()`
    println!("1. Calling Reset()...");
    device.rpc::<()>(fb_path, "Reset", ())?;

    // 2. One Input, Zero Outputs
    // Pass in `100` directly instead of a tuple `(100,)`
    println!("2. Calling SetValue(100)...");
    device.rpc::<()>(fb_path, "SetValue", 100i32)?;

    // 3. Zero Inputs, One Output
    // Parses the return value directly into a `i32`
    let value: i32 = device.rpc(fb_path, "GetValue", ())?;
    println!("3. GetValue() returned: {}", value);

    // 4. Multiple Inputs (N), One Output
    // Pass in a tuple `(50, 25)` and gets a `i32` back
    let sum: i32 = device.rpc(fb_path, "SumValues", (50i32, 25i32))?;
    println!("4. SumValues(50, 25) returned: {}", sum);

    // 5. Multiple Inputs (N), Multiple Outputs (N)
    // Passes a tuple and returns a tuple.
    // The output tuple ALWAYS starts with the RETURN value, followed by VAR_OUTPUT/IN_OUT.
    let (quotient, remainder): (i32, i32) = device.rpc(fb_path, "DivideValues", (100i32, 3i32))?;
    println!(
        "5. DivideValues(100, 3) returned: Quotient = {}, Remainder = {}",
        quotient, remainder
    );

    Ok(())
}
