//! Integration tests for `lash check-index` output and behaviour.
//!
//! These tests invoke the real `lash` binary to verify output content and exit
//! codes for scenarios that cannot be distinguished through unit tests alone
//! (because the relevant functions write directly to stdout/stderr).
//!
//! Targeted surviving mutants:
//! - mut-000218: `args.json` negation on the no-DB path (JSON vs text error)
//! - mut-000219: `!args.paths.is_empty()` negation (path filter applied vs skipped)
//! - mut-000220: `p.is_absolute()` negation (absolute vs relative path resolution)
//! - mut-000221: `args.json` negation on the post-verify output routing
//! - mut-000224: `Ok(1)` → `Ok(0)` when issues are found (exit code assertion)
//! - mut-000225: `report.is_clean()` negation in `output_text_report`
//! - mut-000228: `show_diff` negation (diff section appears only with --diff)
//! - mut-000232-235: `count > 0` boundary in `print_issue_count_if_any`

#![allow(deprecated)] // assert_cmd cargo_bin is deprecated but still works

use assert_cmd::Command;
use lash_db::{init_database, open_database, FileRepository};
use lash_types::{FileMetadata, TaskFile, TaskTree};
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn lash() -> Command {
    Command::cargo_bin("lash").expect("lash binary must be available")
}

/// Create a temp dir with an initialized (empty and clean) lash database.
fn make_clean_project() -> TempDir {
    let temp = TempDir::new().unwrap();
    let lash_dir = temp.path().join(".lash");
    fs::create_dir_all(&lash_dir).unwrap();
    init_database(&lash_dir.join("lash.db")).unwrap();
    temp
}

/// Create a temp dir with a database that contains one stale-file record.
///
/// The record points to a path that does not exist on disk, so the verifier
/// reports it as a stale-file issue and the index is dirty.
fn make_dirty_project() -> TempDir {
    let temp = TempDir::new().unwrap();
    let lash_dir = temp.path().join(".lash");
    fs::create_dir_all(&lash_dir).unwrap();
    let db_path = lash_dir.join("lash.db");
    init_database(&db_path).unwrap();

    let conn = open_database(&db_path).unwrap();
    let repo = FileRepository::new(&conn);
    let stale = TaskFile {
        path: PathBuf::from("tasks/ghost.md"),
        title: "Ghost".to_string(),
        id: "tasks.ghost".to_string(),
        metadata: FileMetadata::default(),
        description: None,
        description_agent_notes: Vec::new(),
        tasks: TaskTree::new(),
        hash: "aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000".to_string(),
        mtime: SystemTime::UNIX_EPOCH,
    };
    repo.insert(&stale).unwrap();
    temp
}

// ---------------------------------------------------------------------------
// mut-000224: exit code 1 when issues found (Ok(1) vs Ok(0))
//
// A dirty index must exit with code 1, not 0. Asserting the exact code kills
// the mutation that replaces Ok(1) with Ok(0).
// ---------------------------------------------------------------------------

#[test]
fn test_check_index_exits_1_for_dirty_index() {
    let project = make_dirty_project();

    lash()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .code(1);
}

#[test]
fn test_check_index_exits_0_for_clean_index() {
    let project = make_clean_project();

    lash()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .code(0);
}

// ---------------------------------------------------------------------------
// mut-000225: `report.is_clean()` negation in `output_text_report`
//
// Clean index → "in sync" message; dirty index → "issue(s)" message.
// With the negation mutation the messages would be swapped.
// ---------------------------------------------------------------------------

#[test]
fn test_check_index_clean_shows_sync_message() {
    let project = make_clean_project();

    lash()
        .arg("--no-color")
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .success()
        .stdout(predicate::str::contains("sync").or(predicate::str::contains("✓")));
}

#[test]
fn test_check_index_dirty_shows_issues_message() {
    let project = make_dirty_project();

    lash()
        .arg("--no-color")
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("issue").or(predicate::str::contains("Found")));
}

// ---------------------------------------------------------------------------
// mut-000228: `show_diff` negation
//
// The "Detailed issues:" section only appears when --diff is passed.
// With the negation mutation, --diff would suppress the section and absence
// of --diff would show it. Both must be asserted.
// ---------------------------------------------------------------------------

#[test]
fn test_check_index_diff_flag_shows_detailed_issues() {
    let project = make_dirty_project();

    lash()
        .arg("--no-color")
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .arg("--diff")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Detailed issues").or(predicate::str::contains("[Stale")));
}

#[test]
fn test_check_index_without_diff_flag_omits_detailed_issues() {
    let project = make_dirty_project();

    // Without --diff: no "Detailed issues" section in output.
    // With the mutation (!(show_diff)), the section WOULD appear without --diff.
    lash()
        .arg("--no-color")
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Detailed issues").not());
}

// ---------------------------------------------------------------------------
// mut-000232-235: `count > 0` boundary in `print_issue_count_if_any`
//
// When a count is non-zero, the label must appear in output.
// When a count is zero, it must be absent.
// These boundary assertions kill mutations that change `>` to `>=`, `<=`, or
// flip the condition entirely.
// ---------------------------------------------------------------------------

#[test]
fn test_check_index_dirty_shows_stale_file_count_in_output() {
    let project = make_dirty_project();

    // The stale-file count is 1 → print_issue_count_if_any must print it.
    // With `count > 0` mutated to `count >= 0`, `count <= 0`, or negated:
    // the output would be wrong or absent. Asserting presence kills those mutations.
    lash()
        .arg("--no-color")
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Stale files").or(predicate::str::contains("stale")));
}

#[test]
fn test_check_index_clean_does_not_show_issue_type_counts() {
    let project = make_clean_project();

    // All counts are 0 → print_issue_count_if_any must NOT print any label.
    // With `count >= 0`, count=0 would satisfy the condition and print spuriously.
    lash()
        .arg("--no-color")
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Stale files").not())
        .stdout(predicate::str::contains("Missing files").not())
        .stdout(predicate::str::contains("Hash mismatches").not())
        .stdout(predicate::str::contains("Orphaned tasks").not());
}

// ---------------------------------------------------------------------------
// mut-000218: `args.json` negation on the no-DB path
//
// When no DB exists:
//   json=true  → stdout contains JSON with "error" key
//   json=false → stderr contains the plain text error message
// With negation mutation these would be swapped.
//
// The temp dir must have a `.lash/` subdirectory (but no `lash.db` inside it)
// so that the `--root` flag accepts it as a valid project root.
// ---------------------------------------------------------------------------

/// Create a temp dir that is recognised as a lash project root (.lash/ exists)
/// but does NOT contain a database file.
fn make_project_without_db() -> TempDir {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join(".lash")).unwrap();
    temp
}

#[test]
fn test_check_index_json_no_db_outputs_json_to_stdout() {
    let project = make_project_without_db();

    let output = lash()
        .arg("--json")
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 3, "exit code must be 3 when no DB found");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON when --json is passed");
    assert!(
        json.get("error").is_some(),
        "JSON error response must have 'error' key; json={json}"
    );
}

#[test]
fn test_check_index_no_json_no_db_outputs_text_to_stderr() {
    let project = make_project_without_db();

    let output = lash()
        .arg("--no-color")
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 3, "exit code must be 3 when no DB found");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Database") || stderr.contains("database") || stderr.contains("not found"),
        "text error mode must print to stderr; stderr={stderr}"
    );

    // Text mode must not produce JSON on stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_err(),
        "text error mode must not produce JSON on stdout; stdout={stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000221: `args.json` negation on the post-verification output routing
//
// After a successful verification:
//   json=true  → stdout is JSON (parseable)
//   json=false → stdout is human-readable text
// With negation mutation these would be swapped. The clean project case is
// easiest to verify because both paths return exit 0.
// ---------------------------------------------------------------------------

#[test]
fn test_check_index_json_clean_outputs_valid_json() {
    let project = make_clean_project();

    let output = lash()
        .arg("--json")
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .output()
        .expect("lash must run");

    assert_eq!(output.status.code().unwrap_or(-1), 0);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON when --json is passed");
    assert!(
        json.get("is_clean").is_some(),
        "JSON output must contain 'is_clean' key; json={json}"
    );
    assert!(
        json["is_clean"].as_bool().unwrap_or(false),
        "is_clean must be true for a clean index"
    );
}

#[test]
fn test_check_index_no_json_clean_outputs_text() {
    let project = make_clean_project();

    let output = lash()
        .arg("--no-color")
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .output()
        .expect("lash must run");

    assert_eq!(output.status.code().unwrap_or(-1), 0);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Human-readable text must not be valid JSON at the top level
    let is_json = serde_json::from_str::<serde_json::Value>(&stdout).is_ok();
    assert!(
        !is_json,
        "text output mode must not produce top-level JSON; stdout={stdout}"
    );
    assert!(
        stdout.contains("sync") || stdout.contains("✓") || stdout.contains("Checked"),
        "text output must contain recognisable sync message; stdout={stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000219: `!args.paths.is_empty()` negation
//
// When a non-empty paths list is provided the filter must be applied. With the
// negation mutation a non-empty list would SKIP the filter (same as empty).
//
// Observability: pass a path that is under the project root but non-existent
// on disk. With the filter applied, the verifier errors (path doesn't exist).
// With mutation (filter skipped), the verifier walks the project root (empty
// temp dir) and returns Ok(0).
// ---------------------------------------------------------------------------

#[test]
fn test_check_index_nonexistent_path_filter_causes_error() {
    let project = make_clean_project();
    // A path that definitely does not exist under the project root.
    let ghost = project.path().join("tasks").join("does_not_exist.md");
    assert!(!ghost.exists());

    // When the filter is applied (correct behaviour), the walker errors on the
    // missing path → command fails with non-zero code.
    // When mutation skips the filter, the walker succeeds → code 0.
    let output = lash()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .arg(ghost)
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    assert_ne!(
        code, 0,
        "a non-existent filter path must cause a non-zero exit code, got {code}"
    );
}

// ---------------------------------------------------------------------------
// mut-000220: `p.is_absolute()` negation in path resolution
//
// When a relative path is given in the paths list:
//   original:  `!p.is_absolute()` is true → `cwd.join(p)` (absolute result)
//   mutation:  `p.is_absolute()` is false → `p.clone()` (stays relative)
//
// The observable difference: the relative path stays relative with mutation,
// so the verifier may not find the path if cwd differs from the project root.
// We can detect this by providing a relative path that resolves correctly only
// after joining with the project root as cwd.
//
// Strategy: use the lash binary from within the project root directory so that
// the relative path resolves correctly. The relative path points to a valid
// directory. Both code paths should succeed in this case, but with an
// absolute path we can also confirm that the absolute branch works correctly.
// ---------------------------------------------------------------------------

#[test]
fn test_check_index_absolute_path_filter_succeeds_on_clean_db() {
    let project = make_clean_project();
    // The project root itself is an absolute path that exists.
    let abs_path = project.path().to_path_buf();
    assert!(abs_path.is_absolute());

    lash()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .arg(&abs_path)
        .assert()
        .code(0);
}

// ---------------------------------------------------------------------------
// mut-000215: `!args.no_color` negation in theme loading
//
// With --no-color: theme = None → output_text_report uses plain text
// Without --no-color: theme = Some(CliTheme) → output_text_report uses styled text
// Both must produce the same logical content (is_clean / issues).
// ---------------------------------------------------------------------------

#[test]
fn test_check_index_no_color_and_color_both_show_sync_on_clean() {
    for no_color_flag in [true, false] {
        let project = make_clean_project();
        let mut cmd = lash();
        cmd.arg("--root").arg(project.path());
        if no_color_flag {
            cmd.arg("--no-color");
        }
        cmd.arg("check-index");
        cmd.assert()
            .code(0)
            .stdout(predicate::str::contains("sync").or(predicate::str::contains("✓")));
    }
}

// ---------------------------------------------------------------------------
// mut-000214: `args.json` negation in theme loading
//
// With --json: theme = None, output is JSON
// Without --json: theme = Some or None depending on no_color, output is text
// Both must complete without error on a clean index.
// ---------------------------------------------------------------------------

#[test]
fn test_check_index_json_and_text_both_succeed_on_clean() {
    let project1 = make_clean_project();
    lash()
        .arg("--json")
        .arg("--root")
        .arg(project1.path())
        .arg("check-index")
        .assert()
        .code(0);

    let project2 = make_clean_project();
    lash()
        .arg("--no-color")
        .arg("--root")
        .arg(project2.path())
        .arg("check-index")
        .assert()
        .code(0);
}
