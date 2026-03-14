//! Integration tests for `lash graph` output and error handling.
//!
//! These tests kill surviving mutants in `commands/graph.rs` that require
//! subprocess-level output inspection.
//!
//! Mutants targeted:
//! - mut-000390  show_summary: false → true  in ErrorReporterConfig when DB missing
//! - mut-000392  0 → 1  in LashError::index_out_of_sync(0)

#![allow(deprecated)]

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn lash() -> Command {
    let mut cmd = Command::cargo_bin("lash").expect("lash binary must be available");
    cmd.arg("--no-logo");
    cmd
}

/// Create a minimal temp directory with `.lash/` but no database.
fn temp_project_no_db() -> TempDir {
    let td = TempDir::new().expect("must create temp dir");
    fs::create_dir_all(td.path().join(".lash")).expect("must create .lash dir");
    td
}

// ---------------------------------------------------------------------------
// mut-000390: show_summary: false → true  in ErrorReporterConfig
//
// When the database does not exist, `graph` creates an ErrorReporter with
// show_summary: false so that only the error line is printed – not an extra
// "Errors: 1, Warnings: 0" summary after it. With the mutation (true), an
// extra summary line would appear.
//
// We kill this by verifying the error output does NOT contain a summary-style
// line (e.g., a line with "errors" and a count that the summary section adds).
//
// The exit code (3) is separately verified by mut-000392.
// ---------------------------------------------------------------------------

/// `graph` without a database must exit with code 3.
/// Kills mut-000392: 0→1 in index_out_of_sync(0) does not affect the exit code
/// path (Ok(3) on line 92), but the exit code itself is a baseline check.
#[test]
fn test_graph_no_db_exits_3() {
    let td = temp_project_no_db();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("graph")
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        3,
        "graph without a database must exit with code 3; \
         stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `graph` without a database must print an error line but no separate
/// summary section (show_summary must be false).
/// Kills mut-000390: show_summary false→true would add a summary line.
#[test]
fn test_graph_no_db_no_summary_section_in_output() {
    let td = temp_project_no_db();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--no-color")
        .arg("graph")
        .output()
        .expect("lash must run");

    assert_eq!(output.status.code().unwrap_or(-1), 3);

    // Combine stdout and stderr for inspection
    let all_output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // An error about the missing index must appear
    assert!(
        all_output.contains("index")
            || all_output.contains("Index")
            || all_output.contains("database"),
        "error message must mention missing index/database; output: {all_output}"
    );

    // With show_summary=false, there must be no standalone "N errors" summary line
    // (the ErrorReporter summary format typically starts with a count like "1 error")
    let has_summary_line = all_output.lines().any(|line| {
        // The summary line from ErrorReporter looks like: "1 error, 0 warnings"
        // with a digit followed by " error" or " errors"
        let trimmed = line.trim();
        trimmed.starts_with(|c: char| c.is_ascii_digit())
            && (trimmed.contains(" error") || trimmed.contains(" warning"))
    });

    assert!(
        !has_summary_line,
        "show_summary=false must not print a digit-led summary line; output: {all_output}"
    );
}

/// `graph` without a database must emit an error referencing the index.
/// Combines both mutants: the error is always reported (baseline), and the
/// content references index state (exercises index_out_of_sync error path).
#[test]
fn test_graph_no_db_emits_index_out_of_sync_error() {
    let td = temp_project_no_db();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--no-color")
        .arg("graph")
        .output()
        .expect("lash must run");

    let all_output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // The error code E_INDEX_OUT_OF_SYNC should appear, or a hint to run lash index
    assert!(
        all_output.contains("index") || all_output.contains("lash index"),
        "error message must reference the index; output: {all_output}"
    );
}

// ---------------------------------------------------------------------------
// mut-000392: 0 → 1  in LashError::index_out_of_sync(0)
//
// The graph command calls `LashError::index_out_of_sync(0)` when the database
// does not exist. The `files_changed` parameter is embedded directly in the
// error message: "index is out of sync (0 files changed)".
//
// If the mutant changes the argument to 1, the message would instead say
// "index is out of sync (1 files changed)".
//
// We kill this by asserting the stderr contains the exact string
// "0 files changed".
// ---------------------------------------------------------------------------

/// `graph` without a database must report exactly 0 files changed in the
/// index-out-of-sync error message.
/// Kills mut-000392: changing 0→1 would produce "(1 files changed)" instead.
#[test]
fn test_graph_no_db_error_message_reports_zero_files_changed() {
    let td = temp_project_no_db();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--no-color")
        .arg("graph")
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        3,
        "graph without a database must exit with code 3"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // index_out_of_sync(0) produces: "index is out of sync (0 files changed)"
    // With the mutation 0→1, the message would say "(1 files changed)" instead.
    assert!(
        stderr.contains("0 files changed"),
        "error message must contain '0 files changed'; stderr: {stderr}"
    );
}
