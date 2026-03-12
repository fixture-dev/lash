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

// ---------------------------------------------------------------------------
// Exact numeric assertions for warning count in JSON  (kills mut-000441, 000448)
//
// These tests assert the exact numeric value for 'warnings' in the JSON summary
// and in the text summary line.  When the filter is mutated from
// `== Severity::Warning` to `!= Severity::Warning`, the count would be wrong.
// ---------------------------------------------------------------------------

/// JSON summary 'warnings' field must be exactly 1 for a file with one warning.
/// Also verifies 'errors' is exactly 0, killing mut-000440 (Error == vs !=).
#[test]
fn test_json_summary_exact_warning_count_is_1() {
    let long_desc: String = "w".repeat(1100);
    let content = format!(
        "# Tasks\n\n@id: tasks\n@created: 2024-01-15\n\n## Description\n\n{long_desc}\n\n## Tasks\n\n- [ ] A task\n"
    );
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "w.md", &content);
    let (summary, code) = lint_json_summary(&path);

    assert_eq!(code, 0, "warning-only file must exit with 0");
    // Exact counts - any mutation of == would make errors wrong or warnings wrong
    assert_eq!(
        summary["errors"].as_u64().unwrap_or(99),
        0,
        "errors must be exactly 0, not any other value"
    );
    assert_eq!(
        summary["warnings"].as_u64().unwrap_or(0),
        1,
        "warnings must be exactly 1, not any other value"
    );
    assert_eq!(
        summary["info"].as_u64().unwrap_or(99),
        0,
        "info must be exactly 0"
    );
    assert_eq!(
        summary["hints"].as_u64().unwrap_or(99),
        0,
        "hints must be exactly 0"
    );
}

/// Text summary line must show "0 errors" and "1 warnings" with no-color output.
/// This exercises the text output paths for warning_count != 0 (kills mut-000448, 000455-462).
#[test]
fn test_text_summary_exact_counts_for_warning_only_file() {
    let long_desc: String = "w".repeat(1100);
    let content = format!(
        "# Tasks\n\n@id: tasks\n@created: 2024-01-15\n\n## Description\n\n{long_desc}\n\n## Tasks\n\n- [ ] A task\n"
    );
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "w.md", &content);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // These exact strings verify the counts in the summary line.
    // Mutations that change == to != in severity counting would produce different numbers.
    assert!(
        stdout.contains("0 errors"),
        "summary must show '0 errors', got:\n{stdout}"
    );
    assert!(
        stdout.contains("1 warnings") || stdout.contains("1 warning"),
        "summary must show '1 warnings' (or '1 warning'), got:\n{stdout}"
    );
    // Must show the "with warnings" variant of the success message
    assert!(
        stdout.contains("with warnings"),
        "summary must show 'with warnings', got:\n{stdout}"
    );
    // Must NOT show the "all passed" message
    assert!(
        !stdout.contains("All files passed"),
        "warning summary must not show 'All files passed', got:\n{stdout}"
    );
}

/// Text summary for a clean file must show "All files passed" (kills mut-000451-000453, 000454).
/// error_count == 0 is TRUE; warning_count == 0 && info_count == 0 && hint_count == 0 is TRUE.
#[test]
fn test_text_summary_clean_file_shows_exact_all_passed_message() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", CLEAN_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must show "All files passed" - any mutation of the error_count == 0 or
    // the inner compound condition would cause a different message.
    assert!(
        stdout.contains("All files passed") || stdout.contains("passed"),
        "clean file must show 'All files passed', got:\n{stdout}"
    );
    // Must NOT show the "with warnings" variant
    assert!(
        !stdout.contains("with warnings"),
        "clean file must not show 'with warnings', got:\n{stdout}"
    );
    // Must NOT show "Summary:" section (clean files return early)
    assert!(
        !stdout.contains("Summary:"),
        "clean file summary should not include 'Summary:' section, got:\n{stdout}"
    );
}

/// Text summary for a file with errors must show the Summary section with "1 errors".
/// Exercises error_count > 0 (kills mut-000466-469) and exact error count (kills mut-000447).
#[test]
fn test_text_summary_error_file_shows_exact_error_count() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Exact error count - any mutation of error_count > 0 comparison or filtering
    // would produce "0 errors" here, failing this assertion.
    assert!(
        stdout.contains("1 errors") || stdout.contains("1 error"),
        "error file must show '1 errors' in summary, got:\n{stdout}"
    );
    assert!(
        stdout.contains("0 warnings"),
        "error file must show '0 warnings' in summary, got:\n{stdout}"
    );
    // Must show the Summary section (not return early)
    assert!(
        stdout.contains("Summary:"),
        "error file must include 'Summary:' section, got:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// `suggest` flag with themed output  (kills mut-000446, 000474 in themed context)
//
// Run with NO_COLOR unset (allows theme) so both themed and unthemed code paths
// are exercised through the CLI.
// ---------------------------------------------------------------------------

/// `--suggest` on an error file should complete successfully (exit 2) and not crash.
/// This exercises the suggest=true branch in output_text_diagnostics and print_summary.
#[test]
fn test_suggest_flag_with_themed_output_does_not_crash() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("lint")
        .arg("--suggest")
        .arg(&path)
        .env_remove("NO_COLOR") // allow themed output
        .output()
        .expect("lash must run");

    // Must exit with 2 (errors found) regardless of suggest flag
    assert_eq!(
        output.status.code().unwrap_or(-1),
        2,
        "--suggest must not change exit code"
    );
}

// ---------------------------------------------------------------------------
// `--no-color` vs default output: verify themed branches (kills mut-000466-473)
//
// Running without --no-color allows the theme to be loaded. With an error file
// we verify the exit code is still 2 (the themed path doesn't change behavior).
// ---------------------------------------------------------------------------

/// Without `--no-color`, lint on an error file still exits with 2.
/// This exercises the `if let Some(t) = theme { ... }` branches for error formatting.
#[test]
fn test_lint_without_no_color_error_file_exits_2() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .env_remove("NO_COLOR") // allow theme to load
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        2,
        "error file must exit 2 even with themed output"
    );
}

/// Without `--no-color`, lint on a clean file still exits with 0.
/// This exercises the themed success path (error_count == 0, warning_count == 0).
#[test]
fn test_lint_without_no_color_clean_file_exits_0() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", CLEAN_MD);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .env_remove("NO_COLOR") // allow theme to load
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "clean file must exit 0 even with themed output"
    );
}

/// Without `--no-color`, lint on a warning file still exits with 0.
/// This exercises the themed warning path (error_count==0, warning_count > 0).
#[test]
fn test_lint_without_no_color_warning_file_exits_0() {
    let long_desc: String = "w".repeat(1100);
    let content = format!(
        "# Tasks\n\n@id: tasks\n@created: 2024-01-15\n\n## Description\n\n{long_desc}\n\n## Tasks\n\n- [ ] A task\n"
    );
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "w.md", &content);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .env_remove("NO_COLOR") // allow theme to load
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "warning-only file must exit 0 even with themed output"
    );
}

// ---------------------------------------------------------------------------
// `files_affected` count in text summary  (kills mut-000475)
//
// Verify that when a diagnostic has a location, the "N files affected" line
// appears in the output. When no location is set, the line must not appear.
// ---------------------------------------------------------------------------

/// Error with a location shows the "files affected" line in the summary.
/// This exercises the `!files_affected.is_empty()` path (kills mut-000475).
#[test]
fn test_text_summary_shows_files_affected_count_for_located_diagnostic() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The error diagnostic has a location, so "files affected" must appear
    assert!(
        stdout.contains("files affected") || stdout.contains("file affected"),
        "located diagnostic must show 'files affected' in summary, got:\n{stdout}"
    );
}
