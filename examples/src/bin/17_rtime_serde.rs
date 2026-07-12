//! Example 17: Serialization and deserialization with complex nested recipes.
//! Run with: `cargo run --bin 17_rtime_serde`
//!
//! This example demonstrates the power of `tcads-serde` by simulating a Recipe Management system.
//! We define a `Recipe` containing an array of `RecipeStep` structs, which in turn contain
//! an enum (`StepCommand`).
//!
//! Because the high-level `read_value` and `write_value` wrappers are not yet used here,
//! this example shows the raw pipeline: fetching type metadata, explicitly invoking
//! `tcads_serde::to_vec`, writing the bytes, and reading them back for deserialization.
//!
//! ## PREREQUISITE
//!
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE, activate the configuration
//! on your local machine, and put the PLC into RUN mode.

use serde::{Deserialize, Serialize};
use tcads::client::devices::blocking::RuntimeDevice;
use tcads::core::AmsAddr;
use tcads_serde::{AdsTypeCache, TypeProvider};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("Connecting to the PLC runtime...");
    // 1. Connect to the local PLC runtime (Port 851)
    let device = RuntimeDevice::connect(AmsAddr::from_local(851), None)?;

    // 2. Fetch the PLC metadata.
    // For this example, we use a "brute-force" approach by downloading the entire
    // PLC type system into our local cache. In a production scenario, you would
    // lazily fetch only the types you need.
    println!("Fetching PLC metadata (this may take a moment)...");
    let ptr_size = device.get_upload_info()?.platform_ptr_size().unwrap_or(8);
    let mut provider = AdsTypeCache::new(ptr_size);
    provider.insert_all(device.get_all_type_infos()?.filter_map(|res| res.ok()));

    // 3. Get the symbol location and structural type info for the Recipe.
    let symbol_name = "MAIN.fbCurrentRecipe";
    let sym_info = device.get_symbol_info(symbol_name)?;
    let type_info = provider.get_type_info(sym_info.type_name()).unwrap();

    // 4. Create and mutate the recipe steps in Rust.
    let mut recipe = Recipe::default();

    recipe.id = "New Recipe".into();

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
    recipe.steps[2] = RecipeStep {
        command: StepCommand::Pump,
        target: 100.0,
        duration_ms: 100_000,
    };
    recipe.steps[3] = RecipeStep {
        command: StepCommand::Cool,
        target: 40.0,
        duration_ms: 10_000,
    };
    recipe.steps[4] = RecipeStep {
        command: StepCommand::Idle,
        target: 0.0,
        duration_ms: 100_000,
    };

    // 5. Serialize the Rust struct into PLC bytes.
    println!("Serializing Recipe to PLC byte layout...");
    let write_buf = tcads_serde::to_vec(&recipe, type_info, &provider)?;

    // 6. Write the payload to the PLC.
    println!("Writing Recipe to '{}'...", symbol_name);
    device.write_bytes_by_info(&sym_info, write_buf)?;

    // 7. Read the raw bytes back from the PLC to prove it worked
    println!("Reading raw memory back from PLC...");
    let read_buf = device.read_bytes_by_info(&sym_info)?;

    // 8. Deserialize the raw bytes back into a new Rust struct
    println!("Deserializing raw bytes into Rust struct...");
    let updated_recipe: Recipe = tcads_serde::from_bytes(&read_buf, type_info, &provider)?;

    println!("\n--- Success! Downloaded Recipe ---");
    println!("{:#?}", updated_recipe);

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
    target: f64, // Matches LREAL
    duration_ms: u32,
}

/// Maps to a TwinCAT Function Block or STRUCT (e.g. FB_Recipe)
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
struct Recipe {
    id: String,
    steps: [RecipeStep; 5],
}
