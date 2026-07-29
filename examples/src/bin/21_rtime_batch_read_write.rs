//! Example 21: Batch Reading and Writing.
//! Run with: `cargo run --bin 21_rtime_batch_read_write`
//!
//! This example demonstrates `read_multi_values` and `write_multi_values`, which
//! bundle several symbol reads or writes into a single network round trip instead
//! of one round trip per symbol.
//!
//! ## Note
//!
//! If any single symbol in the batch fails to resolve, read, or write, the whole call returns that
//! error.
//!
//! ## PREREQUISITE
//!
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE, activate the configuration
//! on your local machine, and put the PLC into RUN mode.

use serde::{Deserialize, Serialize};
use tcads::client::Result;
use tcads::client::devices::blocking::AdsRuntime;
use tcads::core::AmsAddr;

fn main() -> Result<()> {
    println!("Connecting to the PLC runtime...");
    // 1. Connect to the local PLC runtime (Port 851)
    let device = AdsRuntime::connect(AmsAddr::from_local(851), None)?;

    // 2. Build the recipe steps to be written.
    let partial_steps: Vec<(String, RecipeStep)> = (0..3)
        .map(|i| {
            let path = format!("MAIN.fbCurrentRecipe.arSteps[{i}]");
            let step = RecipeStep {
                command: match i {
                    0 => StepCommand::Pump,
                    1 => StepCommand::Heat,
                    _ => StepCommand::Mix,
                },
                target: 40.0 + (i as f64 * 15.0),
                duration_ms: 5000 * (i + 1),
            };
            (path, step)
        })
        .collect();

    let steps_refs = partial_steps
        .iter()
        .map(|(path, step)| (path.as_str(), step));

    // 3. Write to symbols in a single batched write.
    println!("Writing batch...");
    device
        .write_multi_values()
        .push("MAIN.bIncrement", &true)
        .push("MAIN.fbCurrentRecipe.Id", "BATCH_ID_01")
        .push_all(steps_refs)
        .execute()?;

    // 4. Read the symbols in a single batched read.
    let mut read_batch = device.read_multi_values([
        "MAIN.nCount",
        "MAIN.fbCurrentRecipe.Id",
        "MAIN.fbCurrentRecipe.arSteps[0]",
        "MAIN.fbCurrentRecipe.arSteps[1]",
    ])?;

    let count: u32 = read_batch.pop_front().unwrap()?;
    let recipe_id: String = read_batch.pop_front().unwrap()?;
    let steps: Vec<RecipeStep> = read_batch.into_iter_as().collect::<Result<_>>()?;

    println!("Count: {}", count);
    println!("Recipe ID: {:?}", recipe_id);
    for step in steps {
        println!("Step: {:?}", step);
    }

    Ok(())
}

/// Maps to an IEC 61131-3 ENUM (e.g. E_RecipeStepCommand)
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
    target: f64,
    duration_ms: u32,
}
