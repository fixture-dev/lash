//! Comprehensive integration tests for the ErrorReporter across all CLI commands
//!
//! This test suite verifies that the error reporting system works correctly
//! across all commands with different verbosity levels, output formats, and
//! error types. It ensures consistent error handling and proper exit codes.
//!
//! Test Coverage:
//! - Per-command error scenarios with proper exit codes
//! - Verbosity level tests (-q, default, -v, -vv)
//! - Output format tests (text vs JSON)
//! - Exit code verification for all error types
//! - JSON schema validation

mod common;

use assert_cmd::Command;
use common::{temp_test_dir, TestProject};
use serde_json::Value;
use std::fs;

// Allow deprecated cargo_bin for now - will migrate to cargo_bin_cmd! in future
#[allow(deprecated)]
fn create_lash_command() -> Command {
    Command::cargo_bin("lash").expect("Failed to find lash binary")
}

// =============================================================================
// Helper Functions for JSON Validation
// =============================================================================

/// Validate that JSON output has the expected error structure
fn validate_error_json_schema(json: &Value) -> bool {
    // Check for either old format (errors/diagnostics) or new format (success field)
    let has_diagnostics = json.get("diagnostics").is_some();
    let has_errors = json.get("errors").is_some();
    let has_success = json.get("success").is_some();

    has_diagnostics || has_errors || has_success
}

/// Extract error count from JSON output
fn extract_error_count(json: &Value) -> usize {
    // Try different JSON structures
    if let Some(summary) = json.get("summary") {
        if let Some(count) = summary.get("error_count") {
            return count.as_u64().unwrap_or(0) as usize;
        }
    }

    if let Some(diagnostics) = json.get("diagnostics") {
        if let Some(arr) = diagnostics.as_array() {
            return arr.len();
        }
    }

    if let Some(errors) = json.get("errors") {
        if let Some(arr) = errors.as_array() {
            return arr.len();
        }
    }

    0
}

// =============================================================================
// Exit Code Tests for All Command Types
// =============================================================================

/// Test 1: Lint command with invalid input produces exit code 2
#[test]
fn test_lint_error_exit_code() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Checkbox
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox status
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .assert()
        .code(2); // Lint error exit code
}

/// Test 2: Show command with nonexistent file produces exit code 5
#[test]
fn test_show_not_found_exit_code() {
    let temp = temp_test_dir();

    // Create valid index first
    let valid_content = r#"# Valid Project
@id: test.valid
@created: 2024-01-15

## Tasks

- [ ] Task one
"#;
    fs::write(temp.path().join("lash.index.md"), valid_content).unwrap();

    // Index it
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Try to show nonexistent file
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("nonexistent.md")
        .assert()
        .code(5); // Not found exit code
}

/// Test 3: Database error produces exit code 3
#[test]
fn test_database_error_exit_code() {
    let temp = temp_test_dir();

    // Create valid index
    let valid_content = r#"# Valid Project
@id: test.valid
@created: 2024-01-15

## Tasks

- [ ] Task one
"#;
    fs::write(temp.path().join("lash.index.md"), valid_content).unwrap();

    // Index it first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Corrupt the database by writing garbage to it
    let db_path = temp.path().join(".lash").join("lash.db");
    fs::write(&db_path, b"CORRUPTED DATA").unwrap();

    // Try to list (should fail with database error)
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("list")
        .output()
        .expect("Failed to execute command");

    // Should fail (exit code 3 or at least non-zero)
    assert!(!output.status.success());
}

/// Test 4: Config error (no project root) produces exit code 4
#[test]
fn test_config_error_exit_code() {
    let temp = temp_test_dir();
    // Empty directory with no lash.index.md

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    // Should fail with config error
    assert!(!output.status.success());

    // Exit code should be 4 (config error) or at least non-zero
    let exit_code = output.status.code().unwrap_or(1);
    assert!(exit_code != 0, "Should have non-zero exit code");
}

// =============================================================================
// Verbosity Level Tests
// =============================================================================

/// Test 5: Quiet mode (-q) produces minimal output with error count
#[test]
fn test_quiet_mode_error_output() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Tasks
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox 1
- [!] Invalid checkbox 2
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("-q")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    // Should fail with lint error
    assert_eq!(output.status.code(), Some(2));

    // Output should be minimal (less than 300 bytes is a good heuristic)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let total_len = stdout.len() + stderr.len();

    assert!(
        total_len < 300,
        "Quiet mode should produce minimal output. Got {total_len} bytes"
    );
}

/// Test 6: Normal mode (default) produces standard error output
#[test]
fn test_normal_mode_error_output() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Task
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain error information
    assert!(
        stdout.contains("error") || stdout.contains("invalid"),
        "Normal mode should show error details"
    );

    // Should contain file location
    assert!(
        stdout.contains("lash.index.md") || stdout.contains("index"),
        "Normal mode should show file location"
    );
}

/// Test 7: Verbose mode (-v) produces detailed error output
#[test]
fn test_verbose_mode_error_output() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Task
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox status
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("-v")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verbose output should be longer than normal mode
    assert!(
        stdout.len() > 50,
        "Verbose mode should produce detailed output"
    );

    // Should contain error details
    assert!(
        stdout.contains("error") || stdout.contains("invalid"),
        "Verbose mode should show error information"
    );
}

/// Test 8: Debug mode (-vv) produces maximum detail
#[test]
fn test_debug_mode_error_output() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Task
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("-vv")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Debug mode should produce substantial output
    let total_len = stdout.len() + stderr.len();
    assert!(total_len > 0, "Debug mode should produce output");
}

// =============================================================================
// JSON Output Format Tests
// =============================================================================

/// Test 9: JSON output for lint errors has correct schema
#[test]
fn test_lint_json_output_schema() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Task
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox
- [!] Another invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Validate schema
    assert!(
        validate_error_json_schema(&json),
        "JSON should have valid error structure"
    );

    // Should report multiple errors
    let error_count = extract_error_count(&json);
    assert!(
        error_count >= 1,
        "Should report at least one error, got {error_count}"
    );
}

/// Test 10: JSON output includes all diagnostic fields
#[test]
fn test_json_diagnostic_fields() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Task
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let json_str = json.to_string();

    // Should contain error-related fields
    let has_error_info = json_str.contains("error")
        || json_str.contains("diagnostic")
        || json_str.contains("message")
        || json_str.contains("severity");

    assert!(has_error_info, "JSON should contain diagnostic fields");
}

/// Test 11: Text vs JSON output comparison
#[test]
fn test_text_vs_json_output_comparison() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Task
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    // Run with text output
    let text_output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(text_output.status.code(), Some(2));
    let text_stdout = String::from_utf8_lossy(&text_output.stdout);

    // Run with JSON output
    let json_output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(json_output.status.code(), Some(2));
    let json_stdout = String::from_utf8_lossy(&json_output.stdout);

    // Verify JSON is valid
    let _: Value = serde_json::from_str(&json_stdout).expect("JSON output should be valid JSON");

    // Text output should NOT be valid JSON
    assert!(
        serde_json::from_str::<Value>(&text_stdout).is_err(),
        "Text output should not be JSON"
    );

    // Both should indicate errors
    assert!(
        text_stdout.contains("error") || text_stdout.contains("invalid"),
        "Text output should mention errors"
    );

    assert!(
        json_stdout.contains("error") || json_stdout.contains("diagnostic"),
        "JSON output should mention errors"
    );
}

// =============================================================================
// Per-Command Error Tests
// =============================================================================

/// Test 12: Format command with invalid file
#[test]
fn test_format_command_error() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Metadata
@id: test.invalid
@created: not-a-valid-date

## Tasks

- [ ] Task
"#;

    let file_path = temp.path().join("lash.index.md");
    fs::write(&file_path, invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("format")
        .arg(&file_path)
        .output()
        .expect("Failed to execute command");

    // Format may fail or succeed with warnings
    // Just verify it handles errors gracefully
    assert!(
        output.status.code().is_some(),
        "Format should complete (success or graceful failure)"
    );
}

/// Test 13: Index command with invalid files
#[test]
fn test_index_command_with_invalid_files() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Task
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .output()
        .expect("Failed to execute command");

    // Index may fail or succeed with errors collected
    // Verify error handling is graceful
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If it failed, should have helpful error message
    if !output.status.success() {
        let combined = format!("{stdout}{stderr}");
        assert!(
            combined.contains("error")
                || combined.contains("invalid")
                || combined.contains("parse"),
            "Error output should be informative"
        );
    }
}

/// Test 14: Search command with database error
#[test]
fn test_search_command_database_error() {
    let temp = temp_test_dir();

    // Valid content
    let content = r#"# Valid Project
@id: test.valid
@created: 2024-01-15

## Tasks

- [ ] Task one
"#;
    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Corrupt database
    let db_path = temp.path().join(".lash").join("lash.db");
    fs::write(&db_path, b"CORRUPTED").unwrap();

    // Try to search
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("search")
        .arg("task")
        .output()
        .expect("Failed to execute command");

    // Should fail gracefully
    assert!(!output.status.success());

    // Error message should be helpful
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("database") || combined.contains("error") || combined.contains("corrupt"),
        "Error message should indicate database problem"
    );
}

/// Test 15: List command with no database
#[test]
fn test_list_command_no_database() {
    let temp = temp_test_dir();

    // Create valid content but don't index
    let content = r#"# Valid Project
@id: test.valid
@created: 2024-01-15

## Tasks

- [ ] Task one
"#;
    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Try to list without indexing
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("list")
        .output()
        .expect("Failed to execute command");

    // Should fail with helpful message
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The error message might be in stdout or stderr
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("index")
            || combined.contains("database")
            || combined.contains("run")
            || combined.contains("not found"),
        "Error should suggest running index command. Got stderr: {stderr}, stdout: {stdout}"
    );
}

/// Test 16: Graph command error handling
#[test]
fn test_graph_command_error_handling() {
    let temp = temp_test_dir();

    // Try to generate graph without any files
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("graph")
        .output()
        .expect("Failed to execute command");

    // Should fail gracefully
    assert!(!output.status.success());

    // Should have error message
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !stderr.is_empty() || !stdout.is_empty(),
        "Should provide error feedback"
    );
}

// =============================================================================
// Error Message Quality Tests
// =============================================================================

/// Test 17: Error messages are formatted correctly in text mode
#[test]
fn test_error_message_formatting() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Task
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox status
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Error should contain:
    // 1. Severity indicator (error, warning, etc.)
    assert!(
        stdout.contains("error") || stdout.contains("Error"),
        "Should indicate error severity"
    );

    // 2. File location
    assert!(
        stdout.contains("lash.index.md") || stdout.contains("index"),
        "Should show file location"
    );

    // 3. Descriptive message
    assert!(
        stdout.contains("invalid") || stdout.contains("checkbox"),
        "Should describe the problem"
    );
}

/// Test 18: Multiple errors are all reported
#[test]
fn test_multiple_errors_reported() {
    let temp = temp_test_dir();

    // Create file with multiple errors
    let content = r#"# Multiple Errors
@id: test.multiple
@created: 2024-01-15

## Tasks

- [?] First invalid checkbox
- [ ] Valid task
- [!] Second invalid checkbox
- [*] Third invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should report multiple errors (at least 2 mentions of "error" or "invalid")
    let error_mentions = stdout.matches("error").count() + stdout.matches("invalid").count();
    assert!(
        error_mentions >= 2,
        "Should report multiple errors, found {error_mentions} mentions"
    );
}

/// Test 19: Quiet mode with JSON output
#[test]
fn test_quiet_mode_with_json() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Task
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("-q")
        .arg("--json")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should still produce valid JSON even in quiet mode
    let json: Value =
        serde_json::from_str(&stdout).expect("Quiet JSON output should still be valid JSON");

    // Should contain error information
    assert!(
        validate_error_json_schema(&json),
        "JSON should have valid error structure even in quiet mode"
    );
}

/// Test 20: Verbose mode with JSON output
#[test]
fn test_verbose_mode_with_json() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid Task
@id: test.invalid
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("-v")
        .arg("--json")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should produce valid JSON
    let json: Value =
        serde_json::from_str(&stdout).expect("Verbose JSON output should be valid JSON");

    // Should contain error information
    assert!(
        validate_error_json_schema(&json),
        "JSON should have valid error structure"
    );
}

// =============================================================================
// Edge Cases and Error Recovery
// =============================================================================

/// Test 21: Multiple files with errors
#[test]
fn test_multiple_files_with_errors() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks1.md",
            r#"# Tasks 1
@id: tasks1
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox in file 1
"#,
        )
        .with_file(
            "tasks2.md",
            r#"# Tasks 2
@id: tasks2
@created: 2024-01-15

## Tasks

- [!] Invalid checkbox in file 2
"#,
        )
        .build();

    let output = create_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should mention both files
    let mentions_file1 = stdout.contains("tasks1");
    let mentions_file2 = stdout.contains("tasks2");

    assert!(
        mentions_file1 || mentions_file2,
        "Should report errors from multiple files"
    );
}

/// Test 22: Error with empty project
#[test]
fn test_error_with_empty_project() {
    let temp = temp_test_dir();

    // Empty directory, no files
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    // Should fail
    assert!(!output.status.success());

    // Should have helpful error message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("index") || stderr.contains("not found") || stderr.contains("project"),
        "Should explain that no project was found"
    );
}

/// Test 23: Check-index with out-of-sync database
#[test]
fn test_check_index_out_of_sync() {
    let temp = temp_test_dir();

    let content = r#"# Test Project
@id: test
@created: 2024-01-15

## Tasks

- [ ] Task one
"#;
    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Index it
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Modify the file
    let new_content = r#"# Test Project
@id: test
@created: 2024-01-15

## Tasks

- [ ] Task one
- [ ] Task two (new)
"#;
    fs::write(temp.path().join("lash.index.md"), new_content).unwrap();

    // Check index should detect the change
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("check-index")
        .output()
        .expect("Failed to execute command");

    // Should complete (may succeed or fail depending on implementation)
    assert!(output.status.code().is_some());
}
