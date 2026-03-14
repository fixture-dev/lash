//! Integration tests for `lash check-links` output format.
//!
//! These tests kill surviving mutants in `commands/check_links/core.rs` and
//! `commands/check_links/mod.rs` that require subprocess-level output inspection.
//!
//! Mutants targeted:
//! - mut-000243  total_broken == 0 → !(total_broken == 0)  in output_text_report
//! - mut-000244  == → !=  in the same check
//! - mut-000245  0 → 1  in the same check
//! - mut-000247  show_summary: false → true  in ErrorReporterConfig
//! - mut-000248  0 → 1 (line number) in dep_not_found call
//! - mut-000249  0 → 1 (column number) in dep_not_found call
//! - mut-000253  args.json → !(args.json)  in no-DB branch of execute
//! - mut-000257  args.json → !(args.json)  in zero-broken branch of execute

#![allow(deprecated)]

use assert_cmd::Command;
use lash_db::{init_database, run_migrations, Indexer, IndexerConfig};
use lash_types::LashConfig;
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

/// Create a minimal temp project directory with `.lash/` initialised but
/// WITHOUT a database – simulates a project before `lash index` is run.
fn temp_project_no_db() -> TempDir {
    let td = TempDir::new().expect("must create temp dir");
    fs::create_dir_all(td.path().join(".lash")).expect("must create .lash dir");
    td
}

/// Create a minimal temp project with one valid task file, then index it so
/// the database exists and contains no broken links.
fn temp_indexed_clean_project() -> TempDir {
    let td = TempDir::new().expect("must create temp dir");
    fs::create_dir_all(td.path().join(".lash")).expect("must create .lash dir");

    fs::write(
        td.path().join("tasks.md"),
        "# Tasks\n\n@id: tasks\n@created: 2024-01-15\n\n## Tasks\n\n- [ ] A task\n",
    )
    .expect("must write task file");

    let db_path = td.path().join(".lash").join("lash.db");
    let conn = init_database(&db_path).expect("must create database");
    run_migrations(&conn).expect("must run migrations");

    let config = IndexerConfig::new(td.path().to_path_buf());
    let lash_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &lash_config);
    indexer.index_project().expect("must index project");

    td
}

// ---------------------------------------------------------------------------
// mut-000253: args.json → !(args.json) in no-DB branch
//
// When --json is given and the DB doesn't exist, the output must be JSON.
// With the mutation, the non-JSON (plain stderr) path runs instead.
// ---------------------------------------------------------------------------

/// `--json check-links` with no DB must produce valid JSON error output.
/// Kills mut-000253: json→!json would emit plain stderr text instead of JSON.
#[test]
fn test_check_links_no_db_json_flag_produces_json_error() {
    let td = temp_project_no_db();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--json")
        .arg("check-links")
        .output()
        .expect("lash must run");

    // Exit code must be 3 (DB error)
    assert_eq!(
        output.status.code().unwrap_or(-1),
        3,
        "no-DB check-links must exit with code 3"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json must produce valid JSON for no-DB case");

    assert!(
        parsed["error"].is_string(),
        "JSON error response must have 'error' field; got: {parsed}"
    );
}

/// `check-links` without --json and with no DB must NOT produce JSON on stdout.
/// Kills mut-000253: with the negation, plain text would go to stdout when
/// json=false, making the two branches hard to distinguish otherwise.
#[test]
fn test_check_links_no_db_plain_text_on_stderr() {
    let td = temp_project_no_db();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("check-links")
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        3,
        "no-DB check-links must exit with code 3"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // stdout must NOT contain JSON when --json is not given
    assert!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).is_err(),
        "plain mode must not put JSON on stdout; stdout: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000243 / mut-000244 / mut-000245: total_broken == 0 in output_text_report
// mut-000257: args.json → !(args.json) in zero-broken branch of execute
//
// When the database exists but has no broken links:
// - Plain text: must print "No broken links found!" (tests 243/244/245)
// - JSON: must print a JSON object with total_broken=0 (tests 257)
// ---------------------------------------------------------------------------

/// `check-links` on a clean project (text mode) must print the success message.
/// Kills mut-000243 (negation), mut-000244 (==→!=), mut-000245 (0→1):
/// with any of those mutations, `total_broken == 0` evaluates false for a
/// project with 0 broken links, so the success message would be skipped.
#[test]
fn test_check_links_clean_project_text_shows_no_broken_links() {
    let td = temp_indexed_clean_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--no-color")
        .arg("check-links")
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "clean project must exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No broken links found"),
        "text output must contain success message for zero broken links; got: {stdout}"
    );
}

/// `--json check-links` on a clean project must produce a JSON object.
/// Kills mut-000257: json→!json would emit plain text instead of JSON.
#[test]
fn test_check_links_clean_project_json_produces_json_report() {
    let td = temp_indexed_clean_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--json")
        .arg("check-links")
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "clean project with --json must exit 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("--json check-links on clean project must produce valid JSON");

    assert_eq!(
        parsed["total_broken"].as_u64().unwrap_or(99),
        0,
        "total_broken must be 0 for clean project; got: {parsed}"
    );
}

/// `check-links` without --json on a clean project must NOT produce JSON.
/// Kills mut-000257 (negation would produce JSON when json=false).
#[test]
fn test_check_links_clean_project_text_not_json() {
    let td = temp_indexed_clean_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("check-links")
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "clean project must exit 0"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).is_err(),
        "plain text mode must not produce JSON; stdout: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000247: show_summary: false → true in ErrorReporterConfig
//
// When broken links exist, output_text_report creates an ErrorReporter with
// show_summary: false. With the mutation (true), an extra summary section is
// printed at the end. We cannot easily test this without broken links, but we
// can verify that the clean-project text path does NOT contain a summary section
// prefix (e.g., a line beginning with "Summary:").
//
// The strongest test for mut-000247 requires broken links, which needs a more
// complex setup. For the clean-project path, the reporter is NOT created
// (early return at total_broken == 0). The mutation matters only when broken
// links exist.
// ---------------------------------------------------------------------------

/// Verify the ErrorReporter summary is absent when there are no broken links
/// (tests the early-return path that bypasses reporter creation entirely).
/// Indirectly exercises mut-000247's surrounding code.
#[test]
fn test_check_links_no_broken_links_no_summary_section() {
    let td = temp_indexed_clean_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--no-color")
        .arg("check-links")
        .output()
        .expect("lash must run");

    assert_eq!(output.status.code().unwrap_or(-1), 0);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // There must be no "Summary" section header (only expected when reporter flushes)
    assert!(
        !stdout.contains("Summary:"),
        "clean output must not contain a Summary section; got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// Helper: project with a broken dependency link in the database
//
// The lash indexer only records hierarchy (parent-child) dependencies at
// index time; it does not create dependency rows with NULL to_task_id for
// unresolvable @depends-on refs. To exercise the broken-link output path
// we insert the record directly via SQL after initialising the database.
// ---------------------------------------------------------------------------

/// Create a temp project whose database contains exactly one broken dependency:
/// a dependency row with `to_task_id = NULL` and `raw_ref = 'missing#task'`.
fn temp_project_with_broken_link_in_db() -> TempDir {
    let td = TempDir::new().expect("must create temp dir");
    fs::create_dir_all(td.path().join(".lash")).expect("must create .lash dir");

    let db_path = td.path().join(".lash").join("lash.db");
    let conn = init_database(&db_path).expect("must create database");
    run_migrations(&conn).expect("must run migrations");

    // Insert a minimal file record
    conn.execute(
        "INSERT INTO files (path, file_id, title, hash, mtime, metadata)
         VALUES ('broken.md', 'broken', 'Broken Tasks', 'deadbeef', 1234567890, '{}')",
        [],
    )
    .expect("must insert file");

    let file_id: i64 = conn
        .query_row("SELECT id FROM files WHERE path = 'broken.md'", [], |r| {
            r.get(0)
        })
        .expect("must get file id");

    // Insert a task in that file
    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, metadata)
         VALUES (?1, 'task1', 'broken#task1', 'Task 1', 'open', 0, 0, '{}')",
        [file_id],
    )
    .expect("must insert task");

    let task_id: i64 = conn
        .query_row(
            "SELECT id FROM tasks WHERE full_id = 'broken#task1'",
            [],
            |r| r.get(0),
        )
        .expect("must get task id");

    // Insert a broken dependency (to_task_id IS NULL)
    conn.execute(
        "INSERT INTO dependencies (from_task_id, to_task_id, kind, raw_ref)
         VALUES (?1, NULL, 'explicit_id', 'missing#task')",
        [task_id],
    )
    .expect("must insert broken dependency");

    td
}

// ---------------------------------------------------------------------------
// mut-000247: show_summary: false → true in ErrorReporterConfig
//
// When broken links exist, output_text_report creates an ErrorReporter with
// show_summary: false. The reporter is flushed with flush() (not
// flush_with_summary()), so show_summary has no effect on output. This
// makes the mutation equivalent in terms of observable behaviour.
//
// The test below verifies the strongest observable property we can assert:
// that no "Summary:" section appears when broken links are reported, which
// is the intended behaviour guarded by show_summary: false.
// ---------------------------------------------------------------------------

/// With broken links present, the output must NOT contain a "Summary:" section.
/// Kills mut-000247 as far as observable output allows (show_summary: false
/// is set but output_text_report uses flush() not flush_with_summary(), making
/// a direct kill impossible without modifying production code).
#[test]
fn test_check_links_broken_links_no_summary_section() {
    let td = temp_project_with_broken_link_in_db();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--no-color")
        .arg("check-links")
        .output()
        .expect("lash must run");

    // Broken link exits with code 1
    assert_eq!(
        output.status.code().unwrap_or(-1),
        1,
        "broken-link project must exit with code 1; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Combine stdout and stderr: "Summary:" must not appear in either stream
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Summary:"),
        "broken-link output must not contain a Summary section in stderr; got: {stderr}"
    );
    assert!(
        !stdout.contains("Summary:"),
        "broken-link output must not contain a Summary section in stdout; got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000248: 0 → 1 (line number) and mut-000249: 0 → 1 (column number)
// in the dep_not_found call inside output_text_report.
//
// output_text_report passes literal 0 for both line and column because the
// database does not store source positions. The formatted diagnostic therefore
// contains ":0:0" in the location string.  Changing either literal to 1 would
// produce ":1:0" or ":0:1" respectively.
// ---------------------------------------------------------------------------

/// When broken links are reported in plain-text mode, the error location
/// must use ":0:0" (the placeholder used when no source position is known).
/// Kills mut-000248 (0→1 for line) and mut-000249 (0→1 for column).
#[test]
fn test_check_links_broken_links_error_location_is_zero_zero() {
    let td = temp_project_with_broken_link_in_db();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--no-color")
        .arg("check-links")
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        1,
        "broken-link project must exit 1"
    );

    // The error reporter writes diagnostics to stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(":0:0"),
        "error location must be ':0:0' (no line/column in DB); got stderr: {stderr}"
    );
    assert!(
        !stderr.contains(":1:0"),
        "error location must not be ':1:0' (would indicate 0→1 mutation on line); got stderr: {stderr}"
    );
    assert!(
        !stderr.contains(":0:1"),
        "error location must not be ':0:1' (would indicate 0→1 mutation on column); got stderr: {stderr}"
    );
}
