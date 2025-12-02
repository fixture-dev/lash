//! Integration tests for playground mode
//!
//! These tests verify that the `lash playground init` command works correctly,
//! generates valid content, and integrates properly with other Lash commands.

#![allow(deprecated)] // For assert_cmd::Command::cargo_bin

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
use tempfile::TempDir;
use walkdir::WalkDir;

/// Count markdown files in a directory recursively
fn count_md_files(dir: &Path) -> usize {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .count()
}

/// Verify a directory exists and is a directory
fn assert_dir_exists(path: &Path) {
    assert!(
        path.exists(),
        "Expected directory to exist: {}",
        path.display()
    );
    assert!(path.is_dir(), "Expected path to be a directory");
}

#[test]
fn test_playground_init_creates_all_files() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "PixelQuest playground initialized",
        ));

    // Verify structure
    assert!(playground_path.join("lash.index.md").exists());
    assert_dir_exists(&playground_path.join("features"));
    assert_dir_exists(&playground_path.join("systems"));
    assert_dir_exists(&playground_path.join("content"));
    assert_dir_exists(&playground_path.join("infrastructure"));
    assert_dir_exists(&playground_path.join("design"));
    assert_dir_exists(&playground_path.join("milestones"));
    assert!(playground_path.join("PLAYGROUND_GUIDE.md").exists());

    // Count files (should be 24 task files + guide = 25)
    let file_count = count_md_files(&playground_path);
    assert_eq!(
        file_count, 25,
        "Expected 25 markdown files (24 task files from fixture + PLAYGROUND_GUIDE.md)"
    );
}

#[test]
fn test_playground_passes_lint() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    // Generate playground
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    // Run lint (expect warnings but no errors)
    Command::cargo_bin("lash")
        .unwrap()
        .current_dir(&playground_path)
        .arg("lint")
        .assert()
        .success()
        .stdout(predicate::str::contains("Linting passed"));
}

#[test]
fn test_playground_indexes_successfully() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    // Generate (auto-indexes)
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    // Verify database exists (in .lash directory)
    assert!(
        playground_path.join(".lash/lash.db").exists(),
        "Database should exist at .lash/lash.db"
    );

    // Verify can query - tree view shows directory structure with file count
    Command::cargo_bin("lash")
        .unwrap()
        .current_dir(&playground_path)
        .arg("list")
        .assert()
        .success()
        // Tree view shows directories and total file count
        .stdout(predicate::str::contains("features/").or(predicate::str::contains("24 file(s)")));
}

#[test]
fn test_playground_reset_regenerates() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    // Initial generation
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    // Modify a file
    let file_path = playground_path.join("features/player-movement.md");
    std::fs::write(&file_path, "# Modified\n").unwrap();

    // Reset
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .arg("--reset")
        .assert()
        .success();

    // Verify file is restored
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        content.contains("Player Movement"),
        "File should be restored to original content"
    );
}

#[test]
fn test_playground_errors_if_exists_without_reset() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    // Initial generation
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    // Try again without reset
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_playground_guide_is_created() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    let guide_path = playground_path.join("PLAYGROUND_GUIDE.md");
    assert!(guide_path.exists());

    let content = std::fs::read_to_string(&guide_path).unwrap();
    assert!(content.contains("PixelQuest"));
    assert!(content.contains("Quick Start"));
    assert!(content.contains("lash list"));
}

#[test]
fn test_playground_json_output() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    let output = Command::cargo_bin("lash")
        .unwrap()
        .arg("--json")
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Parse JSON to verify it's valid
    // Note: output may contain ANSI-colored text from indexing before the JSON,
    // so we need to find the JSON object
    let json_str = String::from_utf8(output).unwrap();

    // Find the JSON object (starts with '{')
    let json_start = json_str.find('{').expect("No JSON object found in output");
    let json_part = &json_str[json_start..];

    let parsed: serde_json::Value =
        serde_json::from_str(json_part).expect("Failed to parse JSON output");

    assert_eq!(parsed["success"], true);
    assert!(parsed["message"].as_str().unwrap().contains("PixelQuest"));
}

#[test]
fn test_playground_contains_expected_features() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    // Verify key feature files exist
    let expected_files = vec![
        "features/player-movement.md",
        "features/enemy-ai.md",
        "features/level-generation.md",
        "features/power-ups.md",
        "features/boss-fights.md",
        "systems/rendering.md",
        "systems/audio.md",
        "systems/physics.md",
        "systems/input.md",
    ];

    for file in expected_files {
        let path = playground_path.join(file);
        assert!(path.exists(), "Expected file to exist: {file}");
    }
}

#[test]
fn test_playground_search_works() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    // Generate playground
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    // Search for "boss" should find boss-related tasks
    Command::cargo_bin("lash")
        .unwrap()
        .current_dir(&playground_path)
        .arg("search")
        .arg("boss")
        .assert()
        .success()
        .stdout(predicate::str::contains("boss"));
}

#[test]
fn test_playground_show_specific_file() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    // Generate playground
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    // Show a specific file
    Command::cargo_bin("lash")
        .unwrap()
        .current_dir(&playground_path)
        .arg("show")
        .arg("features/player-movement.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("Player Movement"));
}

#[test]
fn test_playground_graph_export() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    // Generate playground
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    // Export graph (explicitly request DOT format)
    let graph_output = playground_path.join("graph.dot");
    Command::cargo_bin("lash")
        .unwrap()
        .current_dir(&playground_path)
        .arg("graph")
        .arg("--format")
        .arg("dot")
        .arg("--output")
        .arg(&graph_output)
        .assert()
        .success();

    assert!(graph_output.exists());
    let graph_content = std::fs::read_to_string(&graph_output).unwrap();
    assert!(graph_content.contains("digraph"));
}

#[test]
fn test_playground_list_with_labels() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    // Generate playground
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    // List gameplay tasks
    Command::cargo_bin("lash")
        .unwrap()
        .current_dir(&playground_path)
        .arg("list")
        .arg("--label")
        .arg("gameplay")
        .assert()
        .success();

    // List backend tasks
    Command::cargo_bin("lash")
        .unwrap()
        .current_dir(&playground_path)
        .arg("list")
        .arg("--label")
        .arg("backend")
        .assert()
        .success();
}

#[test]
fn test_playground_check_links() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    // Generate playground
    Command::cargo_bin("lash")
        .unwrap()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success();

    // Check links
    Command::cargo_bin("lash")
        .unwrap()
        .current_dir(&playground_path)
        .arg("check-links")
        .assert()
        .success();
}
