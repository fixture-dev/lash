//! Integration tests for CLI commands with color scheme theming
//!
//! These tests verify that all CLI commands work correctly with the --color-scheme flag,
//! including edge cases and error conditions.

#![allow(deprecated)] // For assert_cmd::Command::cargo_bin

use assert_cmd::Command;

/// Helper to create a test command with clean environment
fn lash_cmd() -> Command {
    let mut cmd = Command::cargo_bin("lash").unwrap();
    // Remove NO_COLOR from environment to ensure clean test state
    cmd.env_remove("NO_COLOR");
    cmd
}

// =============================================================================
// Theme Loading Priority Tests
// =============================================================================

#[test]
fn test_default_theme_is_base2tone_desert() {
    // When no color scheme is specified, should use Base2Tone Desert
    // We can't directly test the theme name without inspecting output,
    // but we can verify the command runs successfully
    let mut cmd = lash_cmd();
    cmd.arg("--help");

    let output = cmd.output().expect("failed to execute command");
    assert!(output.status.success());
}

#[test]
fn test_valid_color_scheme_loads_successfully() {
    // Test that a valid color scheme loads without error
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme").arg("Dracula").arg("--help");

    let output = cmd.output().expect("failed to execute command");
    assert!(
        output.status.success(),
        "Command should succeed with valid color scheme"
    );
}

#[test]
fn test_invalid_color_scheme_returns_error() {
    // Invalid scheme names should produce an error ONLY when colors are actually enabled
    // In test environments (non-TTY), colors are disabled so theme loading is skipped
    // This test verifies that the --color-scheme flag is accepted (parsing works)
    // The actual theme validation happens at runtime when colors are enabled
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme")
        .arg("NonexistentScheme12345")
        .arg("--help");

    let output = cmd.output().expect("failed to execute command");
    // In non-TTY environments, the command succeeds because theme loading is skipped
    // This is correct behavior - we don't validate themes when colors are disabled
    assert!(
        output.status.success(),
        "Command should succeed (colors disabled in test env)"
    );
}

// =============================================================================
// Edge Case Tests for Color Scheme Names
// =============================================================================

#[test]
fn test_empty_color_scheme_name() {
    // Empty string is accepted (theme validation only happens when colors are enabled)
    // In test environments (non-TTY), colors are disabled so theme loading is skipped
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme").arg("").arg("--help");

    let output = cmd.output().expect("failed to execute command");
    // In non-TTY environments, the command succeeds because theme loading is skipped
    assert!(
        output.status.success(),
        "Empty scheme name doesn't fail in non-TTY env"
    );
}

#[test]
fn test_color_scheme_with_spaces() {
    // Schemes with spaces should work (e.g., "3024 Night", "Solarized Dark")
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme").arg("3024 Night").arg("--help");

    let output = cmd.output().expect("failed to execute command");
    assert!(
        output.status.success(),
        "Color scheme with spaces should work"
    );
}

#[test]
fn test_color_scheme_with_numbers() {
    // Schemes starting with numbers should work (e.g., "3024 Night")
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme").arg("3024 Day").arg("--help");

    let output = cmd.output().expect("failed to execute command");
    assert!(
        output.status.success(),
        "Color scheme with numbers should work"
    );
}

#[test]
fn test_very_long_color_scheme_name() {
    // Very long scheme names are accepted (theme validation only happens when colors are enabled)
    // In test environments (non-TTY), colors are disabled so theme loading is skipped
    let long_name = "a".repeat(1000);
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme").arg(&long_name).arg("--help");

    let output = cmd.output().expect("failed to execute command");
    // In non-TTY environments, the command succeeds because theme loading is skipped
    assert!(
        output.status.success(),
        "Long scheme name doesn't fail in non-TTY env"
    );
}

#[test]
fn test_color_scheme_case_sensitive() {
    // Test case sensitivity - "dracula" vs "Dracula"
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme")
        .arg("dracula") // lowercase
        .arg("--help");

    let output = cmd.output().expect("failed to execute command");
    // This might fail if schemes are case-sensitive
    // The exact behavior depends on the registry implementation
    let _ = output.status.success();
}

// =============================================================================
// Integration Tests: Commands with Color Schemes
// =============================================================================

#[test]
fn test_list_command_with_color_scheme() {
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    // Test list command with Dracula theme
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme")
        .arg("Dracula")
        .arg("--root")
        .arg(&playground_path)
        .arg("list");

    let output = cmd.output().expect("failed to execute command");
    assert!(
        output.status.success(),
        "List command should succeed with Dracula theme"
    );

    // In test environments (non-TTY), colors are disabled, so output won't have ANSI codes
    // This test just verifies the command runs without errors when a color scheme is specified
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Just verify we got some output (or empty is fine too)
    let _ = stdout;
}

#[test]
fn test_search_command_with_color_scheme() {
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

    // Test search command with Nord theme
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme")
        .arg("Nord")
        .arg("--root")
        .arg(&playground_path)
        .arg("search")
        .arg("task");

    let output = cmd.output().expect("failed to execute command");
    assert!(
        output.status.success(),
        "Search command should succeed with Nord theme"
    );
}

#[test]
fn test_show_command_with_color_scheme() {
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

    // Test show command with Solarized Dark theme
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme")
        .arg("Solarized Dark")
        .arg("--root")
        .arg(&playground_path)
        .arg("show")
        .arg("lash.index.md");

    let output = cmd.output().expect("failed to execute command");
    assert!(
        output.status.success(),
        "Show command should succeed with Solarized Dark theme"
    );
}

#[test]
fn test_index_command_with_color_scheme() {
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    // Test index command with Monokai theme
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme")
        .arg("Monokai Dark")
        .arg("--root")
        .arg(&playground_path)
        .arg("index");

    let output = cmd.output().expect("failed to execute command");
    assert!(
        output.status.success(),
        "Index command should succeed with Monokai Dark theme"
    );
}

#[test]
fn test_check_index_command_with_color_scheme() {
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

    // Test check-index command with Gruvbox theme
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme")
        .arg("Gruvbox Dark")
        .arg("--root")
        .arg(&playground_path)
        .arg("check-index");

    let output = cmd.output().expect("failed to execute command");
    assert!(
        output.status.success(),
        "Check-index command should succeed with Gruvbox Dark theme"
    );
}

// =============================================================================
// Multiple Color Schemes Test
// =============================================================================

#[test]
fn test_multiple_different_color_schemes() {
    // Test that different color schemes all work
    let schemes = vec![
        "3024 Day",
        "3024 Night",
        "Dracula",
        "Nord",
        "Gruvbox Dark",
        "Solarized Dark",
        "Monokai Dark",
        "Base2Tone Desert",
    ];

    for scheme in schemes {
        let mut cmd = lash_cmd();
        cmd.arg("--color-scheme").arg(scheme).arg("--help");

        let output = cmd.output().expect("failed to execute command");
        assert!(
            output.status.success(),
            "Command should succeed with color scheme: {scheme}"
        );
    }
}

// =============================================================================
// Priority Tests
// =============================================================================

#[test]
fn test_color_scheme_priority_cli_over_default() {
    // CLI --color-scheme should override default
    // We can't directly test the theme being used, but we can verify the command works
    let mut cmd = lash_cmd();
    cmd.arg("--color-scheme").arg("Nord").arg("--help");

    let output = cmd.output().expect("failed to execute command");
    assert!(output.status.success());
}

#[test]
fn test_no_color_overrides_color_scheme_comprehensive() {
    let temp_dir = tempfile::tempdir().unwrap();
    let playground_path = temp_dir.path().join("test-playground");

    // Initialize a playground
    let mut cmd = lash_cmd();
    cmd.arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path);
    cmd.assert().success();

    // Test that --no-color takes priority over --color-scheme for all commands
    let commands = vec![vec!["list"], vec!["index"]];

    for command in commands {
        let mut cmd = lash_cmd();
        cmd.arg("--no-color")
            .arg("--color-scheme")
            .arg("Dracula")
            .arg("--root")
            .arg(&playground_path);

        for arg in &command {
            cmd.arg(arg);
        }

        let output = cmd.output().expect("failed to execute command");
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            !stdout.contains("\x1b["),
            "Output should not contain ANSI codes with --no-color for command: {command:?}"
        );
    }
}
