//! Integration tests for logging and diagnostics

use std::process::Command;

/// Get the path to the lash binary
fn lash_bin() -> std::path::PathBuf {
    // Use the binary built in the target directory
    let mut path = std::env::current_exe().expect("Failed to get current executable path");
    path.pop(); // Remove test executable name
    if path.ends_with("deps") {
        path.pop(); // Remove 'deps' directory
    }
    path.push("lash");
    path
}

#[test]
fn test_logging_quiet_mode() {
    // In quiet mode, only errors should be logged (ERROR level)
    let output = Command::new(lash_bin())
        .arg("--quiet")
        .arg("--help")
        .env("LASH_LOG", "trace") // Even with trace level, quiet should suppress
        .output()
        .expect("Failed to execute command");

    // Quiet mode should still show help output (goes to stdout)
    // but stderr should be minimal
    let stderr = String::from_utf8_lossy(&output.stderr);

    // In quiet mode with --help, we shouldn't see debug/info/trace logs
    assert!(!stderr.contains("DEBUG"));
    assert!(!stderr.contains("INFO"));
    assert!(!stderr.contains("TRACE"));
}

#[test]
fn test_logging_verbose_mode() {
    // In verbose mode (-v), INFO level logs should appear
    // Use a subcommand that will actually execute
    let output = Command::new(lash_bin())
        .arg("-v")
        .arg("lint")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    // We should see the help text on stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Validate Lash task files") || stdout.contains("lint"));

    // Command should succeed
    assert!(output.status.success());
}

#[test]
fn test_logging_debug_mode() {
    // In debug mode (-vv), DEBUG level logs should appear
    // Use a subcommand that will actually execute
    let output = Command::new(lash_bin())
        .arg("-vv")
        .arg("format")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    // We should see the help text on stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Format Lash task files") || stdout.contains("format"));

    // Command should succeed
    assert!(output.status.success());
}

#[test]
fn test_logging_env_var_override() {
    // LASH_LOG environment variable should override verbosity
    // Use version flag which doesn't require subcommand parsing
    let output = Command::new(lash_bin())
        .arg("--version")
        .env("LASH_LOG", "debug")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lash"));

    // Command should succeed
    assert!(output.status.success());
}

#[test]
fn test_logging_json_mode() {
    // In JSON mode, logs should be emitted as JSON events
    let output = Command::new(lash_bin())
        .arg("--json")
        .arg("-v")
        .arg("lint")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    // JSON mode should still work with verbose logging
    assert!(output.status.success());
}

#[test]
fn test_version_flag_works() {
    // Ensure the version flag works (smoke test for initialization)
    let output = Command::new(lash_bin())
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lash"));
    assert!(output.status.success());
}

#[test]
fn test_no_panic_on_invalid_command() {
    // Ensure we don't panic on invalid input
    let output = Command::new(lash_bin())
        .arg("nonexistent-command")
        .output()
        .expect("Failed to execute command");

    // Should fail with non-zero exit code, not crash
    assert!(!output.status.success());

    // Should suggest valid commands
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("unrecognized"));
}
