//! Integration tests for color output handling
//!
//! Tests verify that:
//! 1. `NO_COLOR` environment variable disables all colors
//! 2. --no-color flag disables all colors
//! 3. --no-color flag takes priority over color scheme selection
//! 4. Non-TTY output disables colors automatically
//! 5. JSON output never includes ANSI codes

#![allow(deprecated)] // For assert_cmd::Command::cargo_bin

use assert_cmd::Command;

/// Helper to create a test command with clean environment
fn lash_cmd() -> Command {
    let mut cmd = Command::cargo_bin("lash").unwrap();
    // Remove NO_COLOR from environment to ensure clean test state
    cmd.env_remove("NO_COLOR");
    cmd
}

#[test]
fn test_no_color_flag_disables_colors() {
    // The --no-color flag should prevent ANSI color codes in output
    let mut cmd = lash_cmd();
    cmd.arg("--no-color").arg("--help");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "Output should not contain ANSI escape codes with --no-color:\n{stdout}"
    );
}

#[test]
fn test_no_color_env_var_disables_colors() {
    // The NO_COLOR environment variable should prevent ANSI color codes
    let mut cmd = lash_cmd();
    cmd.env("NO_COLOR", "1").arg("--help");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "Output should not contain ANSI escape codes with NO_COLOR=1:\n{stdout}"
    );
}

#[test]
fn test_no_color_flag_overrides_color_scheme() {
    // Even if a color scheme is specified, --no-color should take priority
    let mut cmd = lash_cmd();
    cmd.arg("--no-color")
        .arg("--color-scheme")
        .arg("Dracula")
        .arg("--help");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "Output should not contain ANSI escape codes even with color scheme:\n{stdout}"
    );
}

#[test]
fn test_json_output_never_has_colors() {
    // JSON output should never contain ANSI codes, regardless of color settings
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    // Test with JSON output - should have no ANSI codes
    let mut cmd = lash_cmd();
    cmd.arg("--json")
        .arg("--root")
        .arg(&playground_path)
        .arg("list");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_ok(),
        "Output should be valid JSON:\n{stdout}"
    );

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "JSON output should not contain ANSI escape codes:\n{stdout}"
    );
}

#[test]
fn test_json_overrides_color_scheme() {
    // JSON output should ignore color scheme settings
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    // Test with JSON output and color scheme
    let mut cmd = lash_cmd();
    cmd.arg("--json")
        .arg("--color-scheme")
        .arg("Nord")
        .arg("--root")
        .arg(&playground_path)
        .arg("list");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_ok(),
        "Output should be valid JSON:\n{stdout}"
    );

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "JSON output should not contain ANSI escape codes:\n{stdout}"
    );
}

#[test]
fn test_list_command_respects_no_color() {
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    // Test list command with --no-color
    let mut cmd = lash_cmd();
    cmd.arg("--no-color")
        .arg("--root")
        .arg(&playground_path)
        .arg("list");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "List output should not contain ANSI escape codes with --no-color:\n{stdout}"
    );
}

#[test]
fn test_search_command_respects_no_color() {
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    // Index the project
    let mut cmd = lash_cmd();
    cmd.arg("--root").arg(&playground_path).arg("index");
    cmd.assert().success();

    // Test search command with --no-color
    let mut cmd = lash_cmd();
    cmd.arg("--no-color")
        .arg("--root")
        .arg(&playground_path)
        .arg("search")
        .arg("task");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "Search output should not contain ANSI escape codes with --no-color:\n{stdout}"
    );
}

#[test]
fn test_index_command_respects_no_color() {
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    // Test index command with --no-color
    let mut cmd = lash_cmd();
    cmd.arg("--no-color")
        .arg("--root")
        .arg(&playground_path)
        .arg("index");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "Index output should not contain ANSI escape codes with --no-color:\n{stdout}"
    );
}

#[test]
fn test_check_index_command_respects_no_color() {
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground and index it
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    let mut cmd = lash_cmd();
    cmd.arg("--root").arg(&playground_path).arg("index");
    cmd.assert().success();

    // Test check-index command with --no-color
    let mut cmd = lash_cmd();
    cmd.arg("--no-color")
        .arg("--root")
        .arg(&playground_path)
        .arg("check-index");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "Check-index output should not contain ANSI escape codes with --no-color:\n{stdout}"
    );
}

#[test]
fn test_show_command_respects_no_color() {
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground and index it
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    let mut cmd = lash_cmd();
    cmd.arg("--root").arg(&playground_path).arg("index");
    cmd.assert().success();

    // Test show command with --no-color (show the index file)
    let mut cmd = lash_cmd();
    cmd.arg("--no-color")
        .arg("--root")
        .arg(&playground_path)
        .arg("show")
        .arg("lash.index.md");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "Show output should not contain ANSI escape codes with --no-color:\n{stdout}"
    );
}

#[test]
fn test_no_color_env_var_priority() {
    // NO_COLOR should take precedence even if color scheme is specified
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    // Test with NO_COLOR set and color scheme specified
    let mut cmd = lash_cmd();
    cmd.env("NO_COLOR", "1")
        .arg("--color-scheme")
        .arg("Dracula")
        .arg("--root")
        .arg(&playground_path)
        .arg("list");

    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "Output should respect NO_COLOR over color scheme:\n{stdout}"
    );
}
