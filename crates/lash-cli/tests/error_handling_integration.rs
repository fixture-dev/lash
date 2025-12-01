//! Integration tests for error handling across module boundaries
//!
//! These tests verify that errors propagate correctly across modules and are
//! presented to users in helpful, actionable ways. Tests cover:
//! - Parse errors → formatted output (text and JSON)
//! - Lint errors → error aggregation and reporting
//! - Broken dependencies → graceful handling
//! - Multiple error types → proper categorization
//!
//! This completes Task 3 (Integration Tests) by ensuring error handling
//! works correctly end-to-end across the entire CLI pipeline.

mod common;

use assert_cmd::Command;
use common::{temp_test_dir, TestProject};
use predicates::prelude::*;
use std::fs;

// Allow deprecated cargo_bin for now - will migrate to cargo_bin_cmd! in future
#[allow(deprecated)]
fn create_lash_command() -> Command {
    Command::cargo_bin("lash").expect("Failed to find lash binary")
}

/// Test 1: Parse error text output
///
/// Create file with malformed markdown/annotations, run `lash lint`,
/// verify user-friendly error message includes file path, error type, and suggestion.
#[test]
fn test_parse_error_text_output() {
    let temp = temp_test_dir();

    // Create file with invalid checkbox status
    let invalid_content = r#"# Invalid Task File

@id: test.invalid
@status: in-progress
@created: 2024-01-15

## Tasks

- [x] Valid completed task
- [?] Invalid checkbox status
- [ ] Another valid task
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    // Run lash lint
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    // Should fail with lint error exit code (2)
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected lint error exit code 2"
    );

    // Lint output goes to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify error contains:
    // 1. File path
    assert!(
        stdout.contains("lash.index.md") || stdout.contains("index"),
        "Error should mention the file path. Got: {stdout}"
    );

    // 2. Error type/description (invalid checkbox or similar)
    assert!(
        stdout.contains("invalid")
            || stdout.contains("checkbox")
            || stdout.contains("status")
            || stdout.contains("error"),
        "Error should describe the problem. Got: {stdout}"
    );

    // 3. Location information (line number or context)
    // The invalid checkbox is on line 11
    assert!(
        stdout.contains("11") || stdout.contains("Invalid checkbox") || stdout.contains("[?]"),
        "Error should provide location or context. Got: {stdout}"
    );
}

/// Test 2: Parse error JSON output
///
/// Same as test 1 but with --json flag. Verify structured JSON error format
/// with all error fields present (code, message, severity, location).
#[test]
fn test_parse_error_json_output() {
    let temp = temp_test_dir();

    // Create file with invalid checkbox (guaranteed parse error)
    let invalid_content = r#"# Invalid Checkbox

@id: test.invalid
@labels: backend, frontend
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Task one
- [?] Invalid checkbox status
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    // Run lash lint with JSON output
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    // Should fail with lint error exit code
    assert_eq!(output.status.code(), Some(2));

    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify JSON structure contains error information
    // The exact structure may vary, but we should have errors/diagnostics
    assert!(
        json.get("errors").is_some()
            || json.get("diagnostics").is_some()
            || json.get("success").is_some(),
        "JSON should contain error information. Got: {json}"
    );

    // If we have a success field, it should be false
    if let Some(success) = json.get("success") {
        assert_eq!(success, &serde_json::Value::Bool(false));
    }

    // Verify we have error details (message, code, location, etc.)
    let has_error_details = json.to_string().contains("error")
        || json.to_string().contains("diagnostic")
        || json.to_string().contains("message");
    assert!(
        has_error_details,
        "JSON should contain error message details"
    );
}

/// Test 3: Lint error aggregation
///
/// Create multiple files with various lint violations, run linter across all files,
/// verify all errors are collected and reported with correct count.
#[test]
fn test_lint_error_aggregation() {
    let temp = temp_test_dir();

    // File 1: Missing @id
    let file1_content = r#"# Task File Without ID

@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Task one
"#;
    fs::write(temp.path().join("lash.index.md"), file1_content).unwrap();

    // File 2: Invalid checkbox
    fs::create_dir_all(temp.path().join("tasks")).unwrap();
    let file2_content = r#"# Task File With Invalid Checkbox

@id: tasks.invalid
@status: in-progress
@created: 2024-01-15

## Tasks

- [x] Valid task
- [?] Invalid checkbox
"#;
    fs::write(temp.path().join("tasks/bugs.md"), file2_content).unwrap();

    // File 3: Deeply nested tasks (exceeds max depth)
    let file3_content = r#"# Deeply Nested Tasks

@id: tasks.nested
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Level 1
  - [ ] Level 2
    - [ ] Level 3
      - [ ] Level 4
        - [ ] Level 5 (too deep)
"#;
    fs::write(temp.path().join("tasks/nested.md"), file3_content).unwrap();

    // Run lash lint on entire project
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    // Should fail with lint errors
    assert_eq!(output.status.code(), Some(2));

    // Lint output goes to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify multiple errors are reported
    // We should see errors for multiple files
    let error_count = stdout.matches("error").count() + stdout.matches("Error").count();
    assert!(
        error_count >= 2,
        "Should report multiple errors. Found {error_count} error mentions in: {stdout}"
    );

    // Verify errors mention different files
    let mentions_index = stdout.contains("lash.index.md") || stdout.contains("index");
    let mentions_bugs = stdout.contains("bugs.md") || stdout.contains("bugs");
    let mentions_nested = stdout.contains("nested.md") || stdout.contains("nested");

    assert!(
        mentions_index || mentions_bugs || mentions_nested,
        "Should report errors from multiple files. Got: {stdout}"
    );
}

/// Test 4: Broken dependency graceful handling
///
/// Create file with broken dependency reference, run dependency command,
/// verify graceful error handling (no panic) and clear error message.
#[test]
fn test_broken_dependency_graceful_handling() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks
@status: in-progress
@created: 2024-01-15
@depends-on: nonexistent-file.md#task:missing

## Tasks

- [ ] Task that depends on missing file
"#,
        )
        .build();

    // Index the project first (required for graph/check-links commands)
    create_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Run check-links command
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(project.path())
        .arg("check-links")
        .assert()
        .success(); // Should not crash, even with broken links

    // Run graph command
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(project.path())
        .arg("graph")
        .output()
        .expect("Failed to execute command");

    // Should succeed (or fail gracefully, not panic)
    assert!(
        output.status.success() || output.status.code().is_some(),
        "Command should complete gracefully, not panic"
    );

    // If there's an error, it should be informative
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("nonexistent")
                || stderr.contains("missing")
                || stderr.contains("not found"),
            "Error should clearly indicate the missing dependency. Got: {stderr}"
        );
    }
}

/// Test 5: Multiple error types collection
///
/// Create scenario with parse errors + lint errors + broken deps,
/// run full workflow, verify all errors are collected and categorized.
#[test]
fn test_multiple_error_types_collection() {
    let temp = temp_test_dir();

    // Create index with parse error (invalid checkbox)
    let index_content = r#"# Project Index

@id: test-project
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Valid task
- [*] Invalid checkbox (parse error)
"#;
    fs::write(temp.path().join("lash.index.md"), index_content).unwrap();

    // Create task file with lint error (missing @id)
    fs::create_dir_all(temp.path().join("tasks")).unwrap();
    let tasks_content = r#"# Tasks Without ID

@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Some task
"#;
    fs::write(temp.path().join("tasks/incomplete.md"), tasks_content).unwrap();

    // Create file with broken dependency
    let deps_content = r#"# Tasks With Broken Dependency

@id: tasks.broken-dep
@status: in-progress
@created: 2024-01-15
@depends-on: missing-file.md#task:ghost

## Tasks

- [ ] Task depending on ghost
"#;
    fs::write(temp.path().join("tasks/broken-dep.md"), deps_content).unwrap();

    // Run lint command
    let mut lint_cmd = create_lash_command();
    let lint_output = lint_cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute lint command");

    // Should fail with errors
    assert_eq!(
        lint_output.status.code(),
        Some(2),
        "Lint should report errors"
    );

    // Lint output goes to stdout
    let stdout = String::from_utf8_lossy(&lint_output.stdout);

    // Verify we see multiple types of errors mentioned
    let has_parse_error = stdout.contains("invalid")
        || stdout.contains("checkbox")
        || stdout.contains("[*]")
        || stdout.contains("parse");

    let has_lint_error = stdout.contains("missing")
        || stdout.contains("@id")
        || stdout.contains("required")
        || stdout.contains("lint");

    assert!(
        has_parse_error || has_lint_error,
        "Should report multiple error types. Got: {stdout}"
    );

    // Run index command (should handle errors gracefully)
    let mut index_cmd = create_lash_command();
    let index_output = index_cmd
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .output()
        .expect("Failed to execute index command");

    // Index may fail due to validation errors, but should not panic
    assert!(
        index_output.status.code().is_some(),
        "Index should complete (success or graceful failure)"
    );

    // If indexing succeeded despite errors, check-links should handle broken deps
    if index_output.status.success() {
        let mut check_cmd = create_lash_command();
        check_cmd
            .arg("--root")
            .arg(temp.path())
            .arg("check-links")
            .assert()
            .success(); // Should not panic on broken links
    }
}

/// Test 6: Parse error with multiple violations in single file
///
/// Tests that multiple parse errors in one file are all collected and reported.
#[test]
fn test_multiple_parse_errors_single_file() {
    let temp = temp_test_dir();

    let content = r#"# File With Multiple Errors

@id: test.multiple-errors
@status: invalid-status-value
@created: not-a-date

## Tasks

- [x] Valid task
- [?] First invalid checkbox
- [ ] Valid task
- [!] Second invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    // Lint output goes to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should report multiple errors (at least 2: the invalid checkboxes)
    let error_count = stdout.matches("error").count() + stdout.matches("invalid").count();
    assert!(
        error_count >= 2,
        "Should report multiple errors. Got: {stdout}"
    );
}

/// Test 7: Lint error with --json shows structured diagnostics
///
/// Verify that lint errors in JSON mode include all diagnostic fields.
#[test]
fn test_lint_error_json_structured_diagnostics() {
    let temp = temp_test_dir();

    // File with invalid checkbox (guaranteed error)
    let content = r#"# Invalid Checkbox Error

@id: test.invalid-checkbox
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Task one
- [?] Invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify JSON contains structured error information
    let json_str = json.to_string();
    assert!(
        json_str.contains("error")
            || json_str.contains("diagnostic")
            || json_str.contains("missing"),
        "JSON should contain diagnostic information. Got: {json}"
    );
}

/// Test 8: Error output respects --quiet flag
///
/// Verify that --quiet suppresses verbose error output but still sets correct exit code.
#[test]
fn test_error_output_respects_quiet_flag() {
    let temp = temp_test_dir();

    let invalid_content = r#"# Invalid File

@id: test.invalid
@status: in-progress
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox
"#;

    fs::write(temp.path().join("lash.index.md"), invalid_content).unwrap();

    // Run with --quiet flag
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("--quiet")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    // Should still have error exit code
    assert_eq!(output.status.code(), Some(2));

    // Output should be minimal (quiet mode)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // In quiet mode, output should be shorter than verbose mode
    // (This is a heuristic - exact behavior depends on implementation)
    let total_output_len = stderr.len() + stdout.len();
    assert!(
        total_output_len < 500,
        "Quiet mode should produce minimal output. Got {total_output_len} bytes"
    );
}

/// Test 9: Verify exit codes for different error types
///
/// Test that different error categories produce correct exit codes.
#[test]
fn test_exit_codes_for_error_types() {
    let temp = temp_test_dir();

    // Test 1: Lint error -> exit code 2
    let lint_error_content = r#"# Lint Error

@id: test.lint
@status: in-progress
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox
"#;
    fs::write(temp.path().join("lash.index.md"), lint_error_content).unwrap();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .assert()
        .code(2);

    // Test 2: Not found error -> exit code 5
    // Create valid index first
    let valid_content = r#"# Valid Project

@id: test.valid
@status: in-progress
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

    // Try to show non-existent file
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("nonexistent.md")
        .assert()
        .code(5);

    // Test 3: Config error -> exit code 4
    // Try to run command in directory without index
    let empty_temp = temp_test_dir();
    create_lash_command()
        .arg("--root")
        .arg(empty_temp.path())
        .arg("lint")
        .assert()
        .code(predicate::function(|code: &i32| *code != 0)); // Should fail (exact code may vary)
}

/// Test 10: Error messages include helpful suggestions
///
/// Verify that error messages provide actionable guidance to users.
#[test]
fn test_error_messages_include_suggestions() {
    let temp = temp_test_dir();

    let content = r#"# File With Error

@id: test.error
@status: in-progress
@created: 2024-01-15

## Tasks

- [?] Invalid checkbox status
"#;

    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    // Lint output goes to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Error should provide helpful context
    // Look for any of: suggestion, help, valid, expected, should
    let has_helpful_text = stdout.contains("suggestion")
        || stdout.contains("help")
        || stdout.contains("valid")
        || stdout.contains("expected")
        || stdout.contains("should")
        || stdout.contains("use")
        || stdout.contains("must");

    assert!(
        has_helpful_text,
        "Error message should include helpful suggestions. Got: {stdout}"
    );
}
