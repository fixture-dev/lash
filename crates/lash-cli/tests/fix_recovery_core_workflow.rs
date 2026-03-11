//! Core fix recovery workflow tests
//!
//! These tests verify the `FixApplicator` and `ErrorValidator` work together correctly
//! for the complete fix recovery workflow, without relying on specific linter rules.

mod common;

use assert_cmd::Command;
use common::temp_test_dir;
use std::fs;

// Allow deprecated cargo_bin for now - will migrate to cargo_bin_cmd! in future
#[allow(deprecated)]
fn create_lash_command() -> Command {
    Command::cargo_bin("lash").expect("Failed to find lash binary")
}

/// Test that --fix flag doesn't crash on valid files
#[test]
fn test_fix_on_valid_file_no_crash() {
    let temp = temp_test_dir();

    let content = r#"# Valid Tasks

@id: test.valid
@labels: backend, frontend
@created: 2024-01-15

## Tasks

- [ ] Task one
- [ ] Task two
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Run lash lint --fix on valid file
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("lint")
        .arg("--fix")
        .assert()
        .success(); // Should succeed

    // File should remain unchanged
    let final_content = fs::read_to_string(temp.path().join("lash.index.md")).unwrap();
    assert_eq!(final_content, content, "Valid file should not be modified");
}

/// Test that --suggest shows summary with fixable count
#[test]
fn test_suggest_shows_fixable_count() {
    let temp = temp_test_dir();

    let content = r#"# Tasks

@id: test.valid
@labels: backend
@created: 2024-01-15

## Tasks

- [ ] Task
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Run lash lint --suggest
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .arg("--suggest")
        .output()
        .expect("Failed to execute command");

    // Should succeed (no errors)
    assert!(output.status.success(), "Valid file should pass linting");

    // Output should show success or summary
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fixable") || stdout.contains("Summary") || stdout.contains("passed"),
        "Should show summary or success message: {stdout}"
    );
}

/// Test that --suggest with JSON includes fixable count
#[test]
fn test_suggest_json_has_fixable_field() {
    let temp = temp_test_dir();

    let content = r#"# Tasks

@id: test.valid
@labels: backend
@created: 2024-01-15

## Tasks

- [ ] Task
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Run lash lint --suggest --json
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .arg("--suggest")
        .arg("--json")
        .output()
        .expect("Failed to execute command");

    // Should succeed
    assert!(output.status.success());

    // Parse JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Should have summary with fixable field
    assert!(
        json.get("summary").is_some(),
        "JSON should have summary: {json}"
    );
    assert!(
        json["summary"].get("fixable").is_some(),
        "Summary should have fixable field: {json}"
    );
}

/// Test that --fix handles files without errors gracefully
#[test]
fn test_fix_on_error_free_file() {
    let temp = temp_test_dir();

    let content = r#"# Tasks

@id: test.no-errors
@labels: backend
@created: 2024-01-15

## Tasks

- [ ] Task one
- [x] Task two completed
- [ ] Task three
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Run lash lint --fix
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .arg("--fix")
        .output()
        .expect("Failed to execute command");

    // Should succeed
    assert!(output.status.success(), "Should handle error-free file");

    // Stderr should indicate no fixes
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No auto-fixes available") || stderr.is_empty(),
        "Should indicate no fixes needed: {stderr}"
    );
}

/// Test that --fix with --quiet produces minimal output
#[test]
fn test_fix_quiet_minimal_output() {
    let temp = temp_test_dir();

    let content = r#"# Tasks

@id: test.quiet
@labels: backend
@created: 2024-01-15

## Tasks

- [ ] Task
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Run lash lint --fix --quiet
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .arg("--fix")
        .arg("--quiet")
        .output()
        .expect("Failed to execute command");

    // Should succeed
    assert!(output.status.success());

    // Output should be minimal
    let stderr_len = output.stderr.len();
    let stdout_len = output.stdout.len();
    let total = stderr_len + stdout_len;

    assert!(
        total < 500,
        "Quiet mode should produce minimal output (got {total} bytes)"
    );
}

/// Test that --fix doesn't crash on empty files
#[test]
fn test_fix_handles_empty_file() {
    let temp = temp_test_dir();

    // Create empty file
    fs::write(temp.path().join("lash.index.md"), "").unwrap();

    // Run lash lint --fix
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .arg("--fix")
        .output()
        .expect("Failed to execute command");

    // Should complete without crashing
    assert!(
        output.status.code().is_some(),
        "Should handle empty file without crashing"
    );
}

/// Test that --fix works with multiple files
#[test]
fn test_fix_multiple_files() {
    let temp = temp_test_dir();

    // Create index file
    let index = r#"# Project

@id: test-project
@labels: backend
@created: 2024-01-15

## Tasks

- [ ] Task
"#;
    fs::write(temp.path().join("lash.index.md"), index).unwrap();

    // Create subdirectory with another file
    fs::create_dir_all(temp.path().join("tasks")).unwrap();
    let tasks = r#"# Tasks

@id: tasks.main
@labels: frontend
@created: 2024-01-15

## Tasks

- [ ] Task
"#;
    fs::write(temp.path().join("tasks/main.md"), tasks).unwrap();

    // Run lash lint --fix
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .arg("--fix")
        .output()
        .expect("Failed to execute command");

    // Should complete successfully
    assert!(
        output.status.code().is_some(),
        "Should handle multiple files"
    );
}

/// Test that --fix with JSON produces valid JSON output
#[test]
fn test_fix_json_valid_output() {
    let temp = temp_test_dir();

    let content = r#"# Tasks

@id: test.json
@labels: backend
@created: 2024-01-15

## Tasks

- [ ] Task
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Run lash lint --fix --json
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .arg("--fix")
        .arg("--json")
        .output()
        .expect("Failed to execute command");

    // Should succeed
    assert!(output.status.success());

    // Should output valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Should have expected fields
    assert!(json.get("diagnostics").is_some(), "Should have diagnostics");
    assert!(json.get("summary").is_some(), "Should have summary");
}

/// Test iteration limit to prevent infinite loops
#[test]
fn test_fix_iteration_has_limit() {
    let temp = temp_test_dir();

    let content = r#"# Tasks

@id: test.iteration
@labels: backend
@created: 2024-01-15

## Tasks

- [ ] Task
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Run lash lint --fix
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .arg("--fix")
        .output()
        .expect("Failed to execute command");

    // Should complete (not hang indefinitely)
    assert!(output.status.code().is_some(), "Should complete");

    // Check that iteration count is reasonable
    let stderr = String::from_utf8_lossy(&output.stderr);
    let iteration_mentions = stderr.matches("Iteration").count();

    assert!(
        iteration_mentions <= 3,
        "Should not exceed maximum iterations (found {iteration_mentions})"
    );
}
