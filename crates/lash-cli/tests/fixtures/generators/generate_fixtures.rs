//! Executable for generating test fixtures
//!
//! Run this to regenerate the medium-project-realistic fixture.

use std::path::Path;

#[path = "mod.rs"]
mod generator;

fn main() -> std::io::Result<()> {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("repos");

    println!("Generating medium-project-realistic...");
    let output_dir = fixtures_dir.join("medium-project-realistic");

    // Remove existing directory if it exists
    if output_dir.exists() {
        std::fs::remove_dir_all(&output_dir)?;
    }

    generator::generate_ecommerce_project(&output_dir)?;

    println!("✓ Generated medium-project-realistic at {}", output_dir.display());
    println!("  Run 'lash lint {}' to verify", output_dir.display());

    Ok(())
}
