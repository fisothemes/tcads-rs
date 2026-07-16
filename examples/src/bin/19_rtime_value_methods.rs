//! Example 19: High-level Reading and Writing with Serde.
//! Run with: `cargo run --bin 19_rtime_value_methods`
//!
//! This example demonstrates the `read_value` and `write_value` methods
//! on the blocking `RuntimeDevice`. These methods automatically handle
//! type resolution, handle caching, memory padding, and safe Read-Modify-Write
//! cycles for complex nested types like Function Blocks.
//!
//! ## PREREQUISITE
//!
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE, activate the configuration
//! on your local machine, and put the PLC into RUN mode.

use serde::{Deserialize, Serialize};
use tcads::client::devices::blocking::RuntimeDevice;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to the PLC runtime...");
    // 1. Connect to the local PLC runtime (Port 851)
    let device = RuntimeDevice::connect_local(851, None)?;
    let symbol_name = "MAIN.fbCurrentRecipe";

    // 2. Read the value directly from the PLC
    let mut recipe: Recipe = device.read_value(symbol_name)?;
    println!("Current Recipe:\n{:#?}", recipe);

    // 3. Mutate the native Rust struct
    recipe.id = "Updated Recipe".into();
    recipe.steps[0] = RecipeStep {
        command: StepCommand::Heat,
        target: 80.0,
        duration_ms: 100_000,
    };
    recipe.steps[1] = RecipeStep {
        command: StepCommand::Mix,
        target: 25.0,
        duration_ms: 10_000,
    };

    // 4. Write the value safely back to the PLC
    device.write_value(symbol_name, &recipe)?;
    println!("Successfully wrote updated Recipe.");

    Ok(())
}

// Maps to an IEC 61131-3 ENUM (e.g. E_RecipeStepCommand)
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Default)]
enum StepCommand {
    #[default]
    Idle,
    Heat,
    Cool,
    Mix,
    Pump,
}

/// Maps to a TwinCAT STRUCT (e.g. ST_RecipeStep)
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
struct RecipeStep {
    command: StepCommand,
    target: f64, // Matches LREAL
    duration_ms: u32,
}

/// Maps to a TwinCAT Function Block or STRUCT (e.g. FB_Recipe)
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
struct Recipe {
    id: String,
    steps: [RecipeStep; 5],
}
