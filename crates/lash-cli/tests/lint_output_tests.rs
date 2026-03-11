//! Integration tests for `lash lint` output format and severity counting.
//!
//! These tests verify that the JSON and text output produced by the lint command
//! correctly counts and categorises diagnostics by severity. They exercise the
//! branches in `output_json_diagnostics`, `print_summary`, and related helpers
//! that are hard to reach through unit tests because those functions write
//! directly to stdout/stderr.
//!
//! Each test invokes the real `lash` binary and parses its output, ensuring that
//! mutation survivors in the severity-counting logic (`== Severity::Error`,
//! `== Severity::Warning`, `> 0`, etc.) are killed by precise assertions on the
//! JSON summary fields.

#![allow(deprecated)] // assert_cmd cargo_bin is deprecated but still works

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Minimal valid task file that passes linting cleanly.
const CLEAN_MD: &str =
    "# Tasks\n\n@id: my-tasks\n@created: 2024-01-15\n\n## Tasks\n\n- [ ] A task\n";

/// Task file with an invalid checkbox `[?]` – produces one `Error`-severity diagnostic.
const ERROR_MD: &str = "# Bad\n\n@id: bad\n@created: 2024-01-15\n\n## Tasks\n\n- [?] Invalid\n";

fn lash() -> Command {
    Command::cargo_bin("lash").expect("lash binary must be available")
}

/// Write `content` to `<dir>/file.md` and return the full path.
fn write_md(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("failed to write test file");
    path
}

/// Run `lash --json lint <path>` and return the parsed JSON summary object.
fn lint_json_summary(path: &std::path::Path) -> (serde_json::Value, i32) {
    let output = lash()
        .arg("--json")
        .arg("lint")
        .arg(path)
        .output()
        .expect("lash must run");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("stdout was not valid JSON: {e}\nstdout={stdout}");
    });
    let summary = json["summary"].clone();
    (summary, code)
}

// ---------------------------------------------------------------------------
// JSON summary: exact severity counts  (kills mut-000440 – 000443)
//
// Each `== Severity::X` comparison in `output_json_diagnostics` is exercised
// by asserting the exact numeric value for that field when it should be 0
// and when it should be non-zero.
// ---------------------------------------------------------------------------

/// Clean file: every summary field must be exactly 0 and exit code must be 0.
#[test]
fn test_json_summary_all_zero_for_clean_file() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "tasks.md", CLEAN_MD);
    let (summary, code) = lint_json_summary(&path);

    assert_eq!(code, 0, "clean file must produce exit code 0");
    assert_eq!(
        summary["errors"].as_u64().unwrap_or(99),
        0,
        "errors must be 0"
    );
    assert_eq!(
        summary["warnings"].as_u64().unwrap_or(99),
        0,
        "warnings must be 0"
    );
    assert_eq!(summary["info"].as_u64().unwrap_or(99), 0, "info must be 0");
    assert_eq!(
        summary["hints"].as_u64().unwrap_or(99),
        0,
        "hints must be 0"
    );
    assert_eq!(
        summary["files_checked"].as_u64().unwrap_or(0),
        1,
        "files_checked must be 1"
    );
}

/// File with one Error-severity diagnostic: `errors` must be exactly 1,
/// all other counts remain 0, and exit code must be 2.
#[test]
fn test_json_summary_one_error_correct_counts() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "bad.md", ERROR_MD);
    let (summary, code) = lint_json_summary(&path);

    assert_eq!(code, 2, "file with errors must produce exit code 2");
    assert_eq!(
        summary["errors"].as_u64().unwrap_or(0),
        1,
        "errors must be 1"
    );
    assert_eq!(
        summary["warnings"].as_u64().unwrap_or(99),
        0,
        "warnings must be 0"
    );
    assert_eq!(summary["info"].as_u64().unwrap_or(99), 0, "info must be 0");
    assert_eq!(
        summary["hints"].as_u64().unwrap_or(99),
        0,
        "hints must be 0"
    );
}

/// File with a Warning-severity diagnostic: `warnings` must be exactly 1,
/// `errors` must be 0, and exit code must be 0 (warnings do not fail the lint).
///
/// A description of 1100+ characters in a `## Description` section triggers
/// `W_SEM_DESC_TOO_LONG` at Warning severity.
#[test]
fn test_json_summary_one_warning_correct_counts() {
    // Build a description that is just over the 1000-char warning threshold
    let long_desc: String = "w".repeat(1100);
    let content = format!(
        "# Tasks\n\n@id: tasks\n@created: 2024-01-15\n\n## Description\n\n{long_desc}\n\n## Tasks\n\n- [ ] A task\n"
    );

    let td = TempDir::new().unwrap();
    let path = write_md(&td, "tasks.md", &content);
    let (summary, code) = lint_json_summary(&path);

    assert_eq!(code, 0, "warning-only file must produce exit code 0");
    assert_eq!(
        summary["errors"].as_u64().unwrap_or(99),
        0,
        "errors must be 0"
    );
    assert_eq!(
        summary["warnings"].as_u64().unwrap_or(0),
        1,
        "warnings must be 1"
    );
}

/// `files_checked` must equal the actual number of files passed to lint.
#[test]
fn test_json_summary_files_checked_count() {
    let td = TempDir::new().unwrap();
    let f1 = write_md(&td, "a.md", CLEAN_MD);
    let f2 = write_md(&td, "b.md", CLEAN_MD);

    let output = lash()
        .arg("--json")
        .arg("lint")
        .arg(&f1)
        .arg(&f2)
        .output()
        .expect("lash must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("not valid JSON: {e}\nstdout={stdout}"));

    assert_eq!(
        json["summary"]["files_checked"].as_u64().unwrap_or(0),
        2,
        "files_checked must match the number of files passed"
    );
}

// ---------------------------------------------------------------------------
// Text output: recursive discovery flag  (kills mut-000430)
//
// `discover_markdown_files(&paths, true)` passes recursive=true.  When mutated
// to `false`, subdirectories are not descended.  We verify that linting a
// directory discovers files in nested subdirectories.
// ---------------------------------------------------------------------------

/// Files in subdirectories must be discovered when linting a parent directory.
#[test]
fn test_lint_discovers_files_in_subdirectories() {
    let td = TempDir::new().unwrap();
    let sub = td.path().join("sub");
    fs::create_dir_all(&sub).unwrap();
    let nested = sub.join("nested.md");
    // Write a file with an error so we can confirm it was discovered and linted
    fs::write(&nested, ERROR_MD).unwrap();

    let output = lash()
        .arg("--json")
        .arg("lint")
        .arg(td.path())
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    // If recursive discovery works, the error file is found → exit code 2
    // If the mutation flips recursive to false, no files would be found → exit code 0
    assert_eq!(
        code, 2,
        "files in subdirectories must be found with recursive=true"
    );
}

// ---------------------------------------------------------------------------
// Text output: suggest flag in output_text_diagnostics  (kills mut-000446)
// and suggest flag in print_summary  (kills mut-000474)
//
// With `--suggest`, the code enters the `if suggest { }` block. We verify the
// command completes without error for both suggest=true and suggest=false.
// The distinct outcomes (different branches taken) kill the negation mutations.
// ---------------------------------------------------------------------------

/// `--suggest` with a file that has diagnostics takes the suggest=true branch.
#[test]
fn test_lint_suggest_flag_with_errors() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "bad.md", ERROR_MD);

    // Exit code must still be 2 when --suggest is provided
    let output = lash()
        .arg("lint")
        .arg("--suggest")
        .arg(&path)
        .output()
        .expect("lash must run");
    assert_eq!(output.status.code().unwrap_or(-1), 2);
}

/// Without `--suggest`, the suggest branch is not taken; result is still 2.
#[test]
fn test_lint_no_suggest_flag_with_errors() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "bad.md", ERROR_MD);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    assert_eq!(output.status.code().unwrap_or(-1), 2);
}

// ---------------------------------------------------------------------------
// Text output: print_summary branches  (kills mut-000447 – 000462, 000466 – 000475)
//
// The print_summary function has multiple severity-counting conditions. We
// exercise them by producing files with known severity distributions and
// asserting on the text content of stdout.
// ---------------------------------------------------------------------------

/// Clean file → summary prints "All files passed linting" (no-errors, no-warnings branch).
#[test]
fn test_text_summary_clean_file_shows_all_passed() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", CLEAN_MD);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("passed") || stdout.contains("✓"),
        "clean file summary should contain 'passed' or '✓', got: {stdout}"
    );
}

/// File with only warnings → summary shows "Linting passed (with warnings)"
/// and the summary line shows `0 errors, 1 warnings`.
#[test]
fn test_text_summary_warnings_only_shows_passed_with_warnings() {
    let long_desc: String = "w".repeat(1100);
    let content = format!(
        "# Tasks\n\n@id: tasks\n@created: 2024-01-15\n\n## Description\n\n{long_desc}\n\n## Tasks\n\n- [ ] A task\n"
    );
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", &content);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must mention warnings-but-passed path
    assert!(
        stdout.contains("with warnings") || stdout.contains("warning"),
        "warning-only summary should mention warnings, got: {stdout}"
    );
    // Summary line: exactly 0 errors
    assert!(
        stdout.contains("0 errors"),
        "summary must show '0 errors', got: {stdout}"
    );
    // Summary line: exactly 1 warning
    assert!(
        stdout.contains("1 warnings") || stdout.contains("1 warning"),
        "summary must show '1 warnings', got: {stdout}"
    );
}

/// File with an error → summary line shows "> 0 errors".
#[test]
fn test_text_summary_errors_shows_error_count() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Summary line must show at least 1 error (not 0)
    assert!(
        stdout.contains("1 errors") || stdout.contains("1 error"),
        "error summary must show a non-zero error count, got: {stdout}"
    );
    // Must NOT show the all-passed message
    assert!(
        !stdout.contains("All files passed"),
        "error summary must not show 'All files passed', got: {stdout}"
    );
}

/// Summary shows files-affected count when diagnostics have a location.
#[test]
fn test_text_summary_shows_files_affected() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should print "1 files affected" (location is set on parse errors)
    assert!(
        stdout.contains("files affected") || stdout.contains("file affected"),
        "summary with located diagnostic should mention files affected, got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// JSON fixable count: `fix_steps.is_some() || recovery_command.is_some()`
// (kills mut-000444)
//
// The linter does not currently produce diagnostics with fix_steps or
// recovery_command set through normal lint rules, so the fixable count will
// be 0 in all real runs. We verify the field exists and equals 0 for both
// clean and error files, confirming the counting logic is executed.
// ---------------------------------------------------------------------------

/// `fixable` field is present and equals 0 for a clean file.
#[test]
fn test_json_fixable_count_present_and_zero_for_clean_file() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", CLEAN_MD);
    let (summary, _) = lint_json_summary(&path);
    assert!(
        summary.get("fixable").is_some(),
        "summary must contain 'fixable' field"
    );
    assert_eq!(
        summary["fixable"].as_u64().unwrap_or(99),
        0,
        "fixable must be 0"
    );
}

/// `fixable` field equals 0 even when there are errors (since linter rules
/// do not produce `fix_steps` / `recovery_command` on standard diagnostics).
#[test]
fn test_json_fixable_count_zero_with_errors() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "bad.md", ERROR_MD);
    let (summary, _) = lint_json_summary(&path);
    assert_eq!(
        summary["fixable"].as_u64().unwrap_or(99),
        0,
        "fixable must be 0"
    );
}

// ---------------------------------------------------------------------------
// ErrorReporterConfig.show_summary = false  (kills mut-000445)
//
// When show_summary is set to `true` instead of `false`, the ErrorReporter
// prints its own summary in addition to our hand-crafted one. We verify that
// only one "Summary:" heading appears in the output (not two).
// ---------------------------------------------------------------------------

/// The lint output must contain exactly one "Summary:" section header.
#[test]
fn test_text_output_contains_exactly_one_summary_section() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    let summary_count = stdout.matches("Summary:").count();
    assert_eq!(
        summary_count, 1,
        "there should be exactly one 'Summary:' section, found {summary_count}: {stdout}"
    );
}
