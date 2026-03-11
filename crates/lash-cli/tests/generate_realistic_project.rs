//! Integration test that generates the medium-project-realistic fixture
//!
//! Run with: cargo test --test `generate_realistic_project` -- --ignored

#![allow(clippy::uninlined_format_args)]

#[path = "fixtures/generators/mod.rs"]
mod generator;

use std::path::PathBuf;

#[test]
#[ignore] // Run explicitly to regenerate fixtures
fn generate_medium_project_realistic() {
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("repos")
        .join("medium-project-realistic");

    // Remove existing if present
    if output_dir.exists() {
        std::fs::remove_dir_all(&output_dir).expect("Failed to remove existing directory");
    }

    // Generate the project
    generator::generate_ecommerce_project(&output_dir).expect("Failed to generate project");

    println!("✓ Generated medium-project-realistic");
    println!("  Location: {}", output_dir.display());

    // Count files
    fn count_md_files(dir: &std::path::Path) -> usize {
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    count += count_md_files(&path);
                } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    count += 1;
                }
            }
        }
        count
    }

    let file_count = count_md_files(&output_dir);
    println!("  Files: {}", file_count);
    assert!(file_count >= 20, "Should have at least 20 markdown files");
}
