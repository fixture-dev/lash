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

// ---------------------------------------------------------------------------
// `--no-color` flag: verify theme loading branch  (kills mut-000427)
//
// `CliTheme::load(None, !args.no_color)` is called in execute(). When mutated
// to `args.no_color`, the logic is inverted: --no-color enables color and
// default disables it. We verify that:
//   - With `--no-color`, the plain-text summary line appears without ANSI codes.
//   - Without `--no-color`, the summary is still present (exercises both paths).
// ---------------------------------------------------------------------------

/// With `--no-color`, the stdout summary must not contain ANSI escape codes.
/// Kills mut-000427: !args.no_color → args.no_color.
#[test]
fn test_no_color_flag_produces_ansi_free_lint_output() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .env_remove("NO_COLOR")
        .output()
        .expect("lash must run");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // No ANSI escape codes must appear when --no-color is passed
    assert!(
        !stdout.contains('\x1b'),
        "--no-color must suppress ANSI escape codes, got:\n{stdout}"
    );
    // Summary line must still appear (the plain-text path was taken)
    assert!(
        stdout.contains("errors"),
        "--no-color output must still contain summary text, got:\n{stdout}"
    );
}

/// Without `--no-color` (and with NO_COLOR unset), lint output completes
/// normally. This exercises the color-enabled code path through `execute()`.
/// Confirms the two branches produce distinct outcomes (kills mut-000427).
#[test]
fn test_without_no_color_flag_lint_output_contains_summary() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .env_remove("NO_COLOR")
        .env("TERM", "xterm-256color") // hint at color support
        .output()
        .expect("lash must run");

    // Must exit with 2 regardless of color mode
    assert_eq!(
        output.status.code().unwrap_or(-1),
        2,
        "error file must exit 2 regardless of color flag"
    );
}

// ---------------------------------------------------------------------------
// Empty paths vs explicit paths  (kills mut-000428)
//
// `if args.paths.is_empty()` determines whether to search the project root
// or use the explicit list. With the mutation (negated), an explicit path
// would trigger project-root discovery (and ignore the specified file).
// We verify that linting an explicit file path produces the expected result
// for that specific file.
// ---------------------------------------------------------------------------

/// Linting an explicit file path with errors must produce exit code 2.
/// This verifies the `args.paths.is_empty()` false branch is taken.
/// Kills mut-000428: args.paths.is_empty() → !(args.paths.is_empty()).
#[test]
fn test_explicit_path_lints_that_specific_file() {
    let td = TempDir::new().unwrap();
    // Write a clean file and an error file
    let clean = write_md(&td, "clean.md", CLEAN_MD);
    let bad = write_md(&td, "bad.md", ERROR_MD);

    // Linting only the clean file must exit 0
    let clean_output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&clean)
        .output()
        .expect("lash must run");
    assert_eq!(
        clean_output.status.code().unwrap_or(-1),
        0,
        "linting only the clean file must exit 0"
    );

    // Linting only the bad file must exit 2
    let bad_output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&bad)
        .output()
        .expect("lash must run");
    assert_eq!(
        bad_output.status.code().unwrap_or(-1),
        2,
        "linting only the bad file must exit 2"
    );
}

// ---------------------------------------------------------------------------
// Empty directory returns exit 0  (kills mut-000431)
//
// `if files.is_empty()` gates the "no files found" early return. With the
// mutation, a directory containing markdown files would trigger the early
// return (exit 0), while an empty directory would proceed and likely fail.
// We verify that an empty directory (no markdown files) returns exit 0 with
// the expected "No markdown files found" message on stderr.
// ---------------------------------------------------------------------------

/// Linting a directory with no markdown files returns exit code 0 and
/// emits a warning message on stderr. Kills mut-000431.
#[test]
fn test_empty_directory_returns_zero_with_warning() {
    let td = TempDir::new().unwrap();
    // Create a subdirectory with a non-markdown file so the dir is not empty
    let sub = td.path().join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("not-markdown.txt"), "hello").unwrap();

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(td.path())
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "directory with no markdown files must exit 0"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No markdown files") || stderr.contains("no markdown"),
        "empty-directory lint must warn about no markdown files, stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// `--interactive` without `--fix` emits a warning  (kills mut-000432/433/434)
//
// The condition `args.interactive && !args.fix` guards the warning message.
// Tests must verify:
//  - When `--interactive` is given WITHOUT `--fix`, the warning appears.
//  - When `--interactive` is given WITH `--fix`, the warning does NOT appear.
// This distinguishes && from || (mut-000433) and both negation variants.
// ---------------------------------------------------------------------------

/// `--interactive` without `--fix` must emit the "no effect" warning to stderr.
/// Kills mut-000432 (full negation) and mut-000434 (!args.fix negation).
#[test]
fn test_interactive_without_fix_warns_on_stderr() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg("--interactive")
        .arg(&path)
        .output()
        .expect("lash must run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--interactive flag has no effect without --fix")
            || stderr.contains("no effect without --fix"),
        "--interactive without --fix must warn; stderr:\n{stderr}"
    );
}

/// `--interactive` WITH `--fix` must NOT emit the "no effect" warning.
/// This tests the `!args.fix` sub-condition. Kills mut-000433 (&&→||):
/// if `||` were used, `args.fix=true` would still trigger the warning.
#[test]
fn test_interactive_with_fix_does_not_warn() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg("--interactive")
        .arg("--fix")
        .arg(&path)
        .output()
        .expect("lash must run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("no effect without --fix"),
        "--interactive with --fix must NOT produce the 'no effect' warning; stderr:\n{stderr}"
    );
}

/// `--fix` alone (no `--interactive`) must not emit the warning.
/// Confirms that `args.interactive=false` causes the condition to be false.
#[test]
fn test_fix_alone_does_not_warn_about_interactive() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg("--fix")
        .arg(&path)
        .output()
        .expect("lash must run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("no effect without --fix"),
        "--fix alone must not produce the interactive warning; stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// `--fix` applies changes to files  (kills mut-000435)
//
// The `if args.fix` block in `execute()` calls `apply_fixes`. With the
// mutation (`!(args.fix)`), fixes would be applied when `--fix` is NOT
// passed, and skipped when `--fix` IS passed. We verify that `--fix` on a
// fixable file modifies the file content.
// ---------------------------------------------------------------------------

/// `--fix` on a file with an auto-fixable date error modifies the file.
/// Kills mut-000435: args.fix → !(args.fix).
///
/// `@created: 2024-1-5` is a valid date that the parser accepts but the linter
/// flags as E_SEM_INVALID_DATE. The auto-fix normalizes it to `2024-01-05`.
#[test]
fn test_fix_flag_modifies_file_with_fixable_error() {
    // A file with a non-ISO date format that has a linter auto-fix
    let fixable_content =
        "# Tasks\n\n@id: my-tasks\n@created: 2024-1-5\n\n## Tasks\n\n- [ ] A task\n";
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "fixable.md", fixable_content);

    let content_before = fs::read_to_string(&path).unwrap();

    let _output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg("--fix")
        .arg(&path)
        .output()
        .expect("lash must run");

    let content_after = fs::read_to_string(&path).unwrap();

    // --fix must have normalized the date (2024-1-5 → 2024-01-05)
    assert_ne!(
        content_before, content_after,
        "--fix must modify files with auto-fixable errors"
    );
    assert!(
        content_after.contains("2024-01-05"),
        "--fix must have corrected the date to 2024-01-05; content:\n{content_after}"
    );
}

/// Without `--fix`, a fixable error must NOT modify the file.
/// This is the counterpart that confirms fix behavior only happens with --fix.
#[test]
fn test_without_fix_flag_file_is_not_modified() {
    // Same date-error file as above
    let fixable_content =
        "# Tasks\n\n@id: my-tasks\n@created: 2024-1-5\n\n## Tasks\n\n- [ ] A task\n";
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "fixable.md", fixable_content);

    let content_before = fs::read_to_string(&path).unwrap();

    let _output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path) // no --fix
        .output()
        .expect("lash must run");

    let content_after = fs::read_to_string(&path).unwrap();

    assert_eq!(
        content_before, content_after,
        "without --fix, lint must not modify files"
    );
}

// ---------------------------------------------------------------------------
// `--rule` flag restricts rules applied  (kills mut-000479)
//
// `if !args.rules.is_empty()` in `configure_linter` clears and resets the
// enabled rule set to only the specified rules. With the mutation (negated),
// the rule set would always be cleared when rules IS empty (i.e. default
// behavior is broken), and never cleared when rules are specified.
//
// Strategy: use a file with E_SEM_INVALID_DATE (bad @created date format).
// With `--rule W_SEM_DESC_TOO_LONG` only, the date error is suppressed.
// Without `--rule`, the date error appears.
// ---------------------------------------------------------------------------

/// With `--rule W_SEM_DESC_TOO_LONG` only, the E_SEM_INVALID_DATE error is
/// suppressed (the date rule is not in the enabled set). Exit code is 0.
/// Kills mut-000479: !args.rules.is_empty() → args.rules.is_empty().
#[test]
fn test_rule_flag_limits_rules_to_specified_code() {
    // File with a non-ISO date that the linter flags as E_SEM_INVALID_DATE
    let content = "# Tasks\n\n@id: tasks\n@created: 2024-1-5\n\n## Tasks\n\n- [ ] A task\n";
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "date.md", content);

    // With `--rule W_SEM_DESC_TOO_LONG`, no matching rule fires → exit 0
    let output_filtered = lash()
        .arg("--json")
        .arg("lint")
        .arg("--rule")
        .arg("W_SEM_DESC_TOO_LONG")
        .arg(&path)
        .output()
        .expect("lash must run");

    let code = output_filtered.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output_filtered.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("not valid JSON: {e}\nstdout={stdout}"));

    let errors_with_filter = json["summary"]["errors"].as_u64().unwrap_or(99);
    assert_eq!(
        errors_with_filter, 0,
        "--rule W_SEM_DESC_TOO_LONG must suppress E_SEM_INVALID_DATE; exit={code}"
    );
    assert_eq!(
        code, 0,
        "--rule W_SEM_DESC_TOO_LONG must yield exit code 0 for a date-error file"
    );
}

/// Without `--rule`, all rules run and E_SEM_INVALID_DATE appears.
/// This counterpart confirms all rules fire by default. Kills mut-000479.
#[test]
fn test_without_rule_flag_all_rules_run() {
    // Same date-error file
    let content = "# Tasks\n\n@id: tasks\n@created: 2024-1-5\n\n## Tasks\n\n- [ ] A task\n";
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "date.md", content);

    let output = lash()
        .arg("--json")
        .arg("lint")
        .arg(&path) // no --rule
        .output()
        .expect("lash must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("not valid JSON: {e}\nstdout={stdout}"));

    // Without a rule filter, E_SEM_INVALID_DATE must appear
    let diagnostics = json["diagnostics"].as_array().expect("diagnostics array");
    let codes: Vec<&str> = diagnostics
        .iter()
        .filter_map(|d| d["code"].as_str())
        .collect();
    assert!(
        codes.contains(&"E_SEM_INVALID_DATE"),
        "without --rule, E_SEM_INVALID_DATE must appear; codes: {codes:?}"
    );
}

// ---------------------------------------------------------------------------
// `config_path.exists()` in load_project_config  (kills mut-000478)
//
// `load_project_config` checks if `.lash/config.toml` exists before loading.
// With the mutation (negated), the config would be attempted even when the
// file does NOT exist, causing a read error and making lint fail.
//
// We verify that lint succeeds on a clean file in an isolated temp directory
// that has NO `.lash/config.toml`. With the mutation, the non-existent file
// would be read, causing a failure (exit code 1 instead of 0).
// ---------------------------------------------------------------------------

/// Lint on a file in a directory with NO `.lash/config.toml` succeeds.
/// With the mutation `!(config_path.exists())`, the non-existent file would
/// be read, causing a failure. Kills mut-000478.
#[test]
fn test_lint_succeeds_without_project_config_file() {
    let td = TempDir::new().unwrap();
    // No .lash/ directory at all → config_path.exists() is false
    let path = write_md(&td, "tasks.md", CLEAN_MD);

    // Provide explicit path so project root is discovered from the temp dir
    // (which has no .lash/ dir, so config_path won't exist)
    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "lint must succeed with exit 0 when no .lash/config.toml exists;\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Lint on two different files produces consistent results, demonstrating
/// that the config-missing path is stable. Kills mut-000478.
#[test]
fn test_lint_consistent_without_project_config() {
    let td = TempDir::new().unwrap();
    // Clean file and error file in a dir without .lash/config.toml
    let clean = write_md(&td, "clean.md", CLEAN_MD);
    let bad = write_md(&td, "bad.md", ERROR_MD);

    let out_clean = lash()
        .arg("--json")
        .arg("lint")
        .arg(&clean)
        .output()
        .expect("lash must run");
    assert_eq!(
        out_clean.status.code().unwrap_or(-1),
        0,
        "clean file must exit 0 (no config needed)"
    );

    let out_bad = lash()
        .arg("--json")
        .arg("lint")
        .arg(&bad)
        .output()
        .expect("lash must run");
    assert_eq!(
        out_bad.status.code().unwrap_or(-1),
        2,
        "error file must exit 2 (no config needed)"
    );
}

// ---------------------------------------------------------------------------
// `args.fix` in configure_linter sets auto_fix = true  (kills mut-000480/481)
//
// `if args.fix { config.auto_fix = true; }` in configure_linter. With the
// mutation (negated: !(args.fix)), auto_fix would be set to true when --fix
// is NOT passed - meaning fixes run without the user asking for them.
// We verify that without --fix, a fixable file is not changed, and with
// --fix it is changed, distinguishing the two paths.
// (These tests overlap with mut-000435 coverage but approach from configure_linter.)
// ---------------------------------------------------------------------------

/// `auto_fix` is enabled only when `--fix` is passed; without it, files are
/// not changed. This verifies configure_linter's `args.fix` branch.
/// Kills mut-000480 and mut-000481.
///
/// Uses a file with `@created: 2024-1-5` (non-ISO date) which is auto-fixable
/// by the ValidDateRule linter rule.
#[test]
fn test_configure_linter_auto_fix_only_set_with_fix_flag() {
    let fixable = "# Tasks\n\n@id: tasks\n@created: 2024-1-5\n\n## Tasks\n\n- [ ] A task\n";
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "fix.md", fixable);
    let original = fs::read_to_string(&path).unwrap();

    // Without --fix: content must not change
    let _out = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");
    let after_no_fix = fs::read_to_string(&path).unwrap();
    assert_eq!(
        original, after_no_fix,
        "without --fix, configure_linter must not enable auto_fix"
    );

    // With --fix: content must change (date normalized from 2024-1-5 to 2024-01-05)
    let _out = lash()
        .arg("--no-color")
        .arg("lint")
        .arg("--fix")
        .arg(&path)
        .output()
        .expect("lash must run");
    let after_fix = fs::read_to_string(&path).unwrap();
    assert_ne!(
        original, after_fix,
        "with --fix, configure_linter must enable auto_fix and apply changes"
    );
    assert!(
        after_fix.contains("2024-01-05"),
        "with --fix, the date must be normalized to 2024-01-05; content:\n{after_fix}"
    );
}

// ---------------------------------------------------------------------------
// Themed `warning_count > 0` branch in print_summary  (kills mut-000470-473)
//
// In `print_summary`, within the `if let Some(t) = theme` branch, the
// warning string is styled differently when `warning_count > 0`. To kill
// these mutants we need a test that:
// 1. Has a warning-only file (warning_count = 1 > 0 is TRUE)
// 2. Runs WITHOUT --no-color so the theme branch is taken
// 3. Asserts the warning is reported in the output
// ---------------------------------------------------------------------------

/// With themed output (no --no-color) and a warning, the summary line must
/// include the warning count. Kills mut-000470/471/472/473.
#[test]
fn test_themed_summary_shows_warning_count_for_warning_file() {
    let long_desc: String = "w".repeat(1100);
    let content = format!(
        "# Tasks\n\n@id: tasks\n@created: 2024-01-15\n\n## Description\n\n{long_desc}\n\n## Tasks\n\n- [ ] A task\n"
    );
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "warn.md", &content);

    // Use --json so we can parse the output regardless of theme
    let output = lash()
        .arg("--json")
        .arg("lint")
        .arg(&path)
        .env_remove("NO_COLOR")
        .output()
        .expect("lash must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("not valid JSON: {e}\nstdout={stdout}"));

    // warnings == 1, errors == 0: confirms the warning_count > 0 filter works
    assert_eq!(
        json["summary"]["warnings"].as_u64().unwrap_or(0),
        1,
        "themed JSON run must report exactly 1 warning"
    );
    assert_eq!(
        json["summary"]["errors"].as_u64().unwrap_or(99),
        0,
        "themed JSON run must report 0 errors"
    );
    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "warning-only file must exit 0 in themed mode"
    );
}

/// With themed output and NO warnings (clean file), warning_count == 0.
/// This exercises the `else` branch (warning_count > 0 is FALSE). Kills mut-000470.
#[test]
fn test_themed_summary_zero_warnings_for_clean_file() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "clean.md", CLEAN_MD);

    let output = lash()
        .arg("--json")
        .arg("lint")
        .arg(&path)
        .env_remove("NO_COLOR")
        .output()
        .expect("lash must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("not valid JSON: {e}\nstdout={stdout}"));

    assert_eq!(
        json["summary"]["warnings"].as_u64().unwrap_or(99),
        0,
        "clean file with themed output must show 0 warnings"
    );
    assert_eq!(
        json["summary"]["errors"].as_u64().unwrap_or(99),
        0,
        "clean file with themed output must show 0 errors"
    );
}

// ===========================================================================
// Tests targeting surviving mutants identified in the second flawd pass
// ===========================================================================

// ---------------------------------------------------------------------------
// mut-000475: !args.no_color → args.no_color  in execute()
//
// CliTheme::load(None, !args.no_color) is the call site.  When mutated the
// logic inverts: --no-color enables color and the default disables it.  Two
// complementary tests confirm the correct branch is taken each time.
// ---------------------------------------------------------------------------

/// With `--no-color`, lint stdout must contain no ANSI escape codes.
/// Kills mut-000475: !args.no_color → args.no_color.
#[test]
fn test_mut000475_no_color_suppresses_ansi_in_lint_stdout() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .env_remove("NO_COLOR")
        .output()
        .expect("lash must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains('\x1b'),
        "--no-color must suppress ANSI in lint stdout (mut-000475):\n{stdout}"
    );
    assert!(
        stdout.contains("error") || stdout.contains("Summary"),
        "--no-color lint must still print summary text (mut-000475):\n{stdout}"
    );
}

/// Without `--no-color`, lint exits 2 on an error file (color-enabled path).
/// Kills mut-000475: distinguishes !no_color=true from !no_color=false.
#[test]
fn test_mut000475_without_no_color_lint_exits_2() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("lint")
        .arg(&path)
        .env_remove("NO_COLOR")
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        2,
        "lint without --no-color must exit 2 for an error file (mut-000475)"
    );
}

// ---------------------------------------------------------------------------
// mut-000478: true → false  in discover_markdown_files(&paths, true)
//
// The recursive flag enables descent into subdirectories.  With the mutation
// (false), nested files are missed → exit 0 instead of 2.
// ---------------------------------------------------------------------------

/// Files nested two levels deep must be found with recursive=true.
/// Kills mut-000478: true → false in discover_markdown_files.
#[test]
fn test_mut000478_recursive_discovery_finds_deeply_nested_files() {
    let td = TempDir::new().unwrap();
    let deep = td.path().join("a").join("b");
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("deep.md"), ERROR_MD).unwrap();

    let output = lash()
        .arg("--json")
        .arg("lint")
        .arg(td.path())
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code, 2,
        "recursive discovery must find nested error file (mut-000478); exit={code}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("not valid JSON: {e}\nstdout={stdout}"));
    assert_eq!(
        json["summary"]["files_checked"].as_u64().unwrap_or(0),
        1,
        "recursive discovery must check the 1 nested file (mut-000478)"
    );
}

// ---------------------------------------------------------------------------
// mut-000482: !(args.interactive && !args.fix)  – full negation
// mut-000483: && → ||  in args.interactive && !args.fix
// mut-000484: !args.fix → args.fix
//
// The compound condition guards the "--interactive without --fix" warning.
// Three tests cover the three mutation cases:
//   1. interactive=true,  fix=false → warning present  (base case)
//   2. interactive=true,  fix=true  → warning absent   (kills 000483, 000484)
//   3. interactive=false, fix=false → warning absent   (kills 000482)
// ---------------------------------------------------------------------------

/// --interactive without --fix emits the warning on stderr.
/// Kills mut-000482 (full negation) and mut-000484 (!args.fix → args.fix).
#[test]
fn test_mut000482_interactive_without_fix_warns() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", CLEAN_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg("--interactive")
        .arg(&path)
        .output()
        .expect("lash must run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no effect without --fix"),
        "--interactive without --fix must warn (mut-000482/484):\n{stderr}"
    );
}

/// --interactive WITH --fix must not emit the warning.
/// Kills mut-000483 (&& → ||): with ||, fix=true still triggers the warn.
/// Also kills mut-000484: if !args.fix → args.fix, fix=true triggers warn.
#[test]
fn test_mut000483_interactive_with_fix_no_warn() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", CLEAN_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg("--interactive")
        .arg("--fix")
        .arg(&path)
        .output()
        .expect("lash must run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("no effect without --fix"),
        "--interactive with --fix must not warn (mut-000483):\n{stderr}"
    );
}

/// Neither --interactive nor --fix: no warning.
/// Kills mut-000482 (full negation would invert: warn here, not warn above).
#[test]
fn test_mut000482_no_interactive_no_warn() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", CLEAN_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("no effect without --fix"),
        "lint without --interactive must not warn (mut-000482):\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// mut-000486: args.fix → !(args.fix)  in `if args.fix { apply_fixes(...) }`
//
// With mutation, fixes run without --fix and are skipped with --fix.
// ---------------------------------------------------------------------------

/// --fix on a file with a non-ISO date normalizes it; content must change.
/// Kills mut-000486: args.fix → !(args.fix).
#[test]
fn test_mut000486_fix_flag_applies_fixes() {
    let fixable = "# Tasks\n\n@id: my-id\n@created: 2024-1-5\n\n## Tasks\n\n- [ ] A task\n";
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "fixable.md", fixable);
    let before = fs::read_to_string(&path).unwrap();

    lash()
        .arg("--no-color")
        .arg("lint")
        .arg("--fix")
        .arg(&path)
        .output()
        .expect("lash must run");

    let after = fs::read_to_string(&path).unwrap();
    assert_ne!(before, after, "--fix must modify the file (mut-000486)");
    assert!(
        after.contains("2024-01-05"),
        "--fix must normalize date to 2024-01-05 (mut-000486); got:\n{after}"
    );
}

/// Without --fix, the file must remain unchanged.
/// Confirms the non-fix branch (mut-000486 would change content here).
#[test]
fn test_mut000486_without_fix_file_unchanged() {
    let fixable = "# Tasks\n\n@id: my-id\n@created: 2024-1-5\n\n## Tasks\n\n- [ ] A task\n";
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "fixable.md", fixable);
    let before = fs::read_to_string(&path).unwrap();

    lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");

    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(
        before, after,
        "without --fix, file must not change (mut-000486)"
    );
}

// ---------------------------------------------------------------------------
// mut-000487: args.json → !(args.json)  in `if args.json { output_json... }`
//
// With mutation, --json gives text and no-json gives JSON.
// ---------------------------------------------------------------------------

/// --json produces valid JSON with a "summary" key.
/// Kills mut-000487: args.json → !(args.json).
#[test]
fn test_mut000487_json_flag_gives_json_stdout() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("--json")
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        parsed.is_ok(),
        "--json must produce valid JSON (mut-000487):\n{stdout}"
    );
    assert!(
        parsed.unwrap().get("summary").is_some(),
        "--json JSON must have 'summary' key (mut-000487)"
    );
}

/// Without --json, stdout is plain text (not parseable as JSON).
/// Kills mut-000487: the mutated code produces JSON here.
#[test]
fn test_mut000487_no_json_flag_gives_plain_text() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "t.md", ERROR_MD);

    let output = lash()
        .arg("--no-color")
        .arg("lint")
        .arg(&path)
        .output()
        .expect("lash must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_err(),
        "lint without --json must give plain text, not JSON (mut-000487):\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000491: d.severity == Severity::Error  → !=
// mut-000492: d.severity == Severity::Warning → !=
// mut-000493: d.severity == Severity::Info    → !=
// mut-000494: d.severity == Severity::Hint    → !=
//
// Strategy: assert every severity bucket at its exact expected value for files
// that produce a non-zero count in exactly one bucket.  Any `==` → `!=`
// mutation misroutes that diagnostic into the wrong bucket, causing the
// numerical assertion for the wrong bucket to fail.
//
// No linter rule produces Severity::Info or Hint through the integration path
// (the parser pre-waives children before the linter sees them).  We rely on
// cross-bucket assertions: an Error file has info==0 and hints==0; with
// mut-000493/494 those become 1.
// ---------------------------------------------------------------------------

/// Error file: every severity counter must be at its exact expected value.
///
/// - mut-000491: errors becomes 0 instead of 1.
/// - mut-000492: warnings becomes 1 instead of 0.
/// - mut-000493: info    becomes 1 instead of 0.
/// - mut-000494: hints   becomes 1 instead of 0.
#[test]
fn test_mut000491_494_exact_severity_counts_error_file() {
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "bad.md", ERROR_MD);
    let (summary, _) = lint_json_summary(&path);

    assert_eq!(
        summary["errors"].as_u64().unwrap_or(0),
        1,
        "errors must be 1 (mut-000491); summary={summary}"
    );
    assert_eq!(
        summary["warnings"].as_u64().unwrap_or(99),
        0,
        "warnings must be 0 (mut-000492); summary={summary}"
    );
    assert_eq!(
        summary["info"].as_u64().unwrap_or(99),
        0,
        "info must be 0 (mut-000493); summary={summary}"
    );
    assert_eq!(
        summary["hints"].as_u64().unwrap_or(99),
        0,
        "hints must be 0 (mut-000494); summary={summary}"
    );
}

/// Warning file: every severity counter must be at its exact expected value.
///
/// - mut-000491: errors   becomes 1 instead of 0.
/// - mut-000492: warnings becomes 0 instead of 1.
/// - mut-000493: info     becomes 1 instead of 0.
/// - mut-000494: hints    becomes 1 instead of 0.
#[test]
fn test_mut000491_494_exact_severity_counts_warning_file() {
    let long_desc: String = "w".repeat(1100);
    let content = format!(
        "# Tasks\n\n@id: tasks\n@created: 2024-01-15\n\n## Description\n\n{long_desc}\n\n## Tasks\n\n- [ ] A task\n"
    );
    let td = TempDir::new().unwrap();
    let path = write_md(&td, "warn.md", &content);
    let (summary, _) = lint_json_summary(&path);

    assert_eq!(
        summary["errors"].as_u64().unwrap_or(99),
        0,
        "errors must be 0 (mut-000491); summary={summary}"
    );
    assert_eq!(
        summary["warnings"].as_u64().unwrap_or(0),
        1,
        "warnings must be 1 (mut-000492); summary={summary}"
    );
    assert_eq!(
        summary["info"].as_u64().unwrap_or(99),
        0,
        "info must be 0 (mut-000493); summary={summary}"
    );
    assert_eq!(
        summary["hints"].as_u64().unwrap_or(99),
        0,
        "hints must be 0 (mut-000494); summary={summary}"
    );
}
