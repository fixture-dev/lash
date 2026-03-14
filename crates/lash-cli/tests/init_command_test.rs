//! Integration tests targeting surviving mutants in `commands/init.rs`.
//!
//! Each test is annotated with the mutant ID(s) it is designed to kill.
//! Tests capture stdout/stderr via `assert_cmd` and assert on content.
//!
//! Mutant coverage:
//! - mut-000451 / mut-000482 : `args.json → !(args.json)` in theme-loading guard
//! - mut-000452 / mut-000483 : `!args.no_color → args.no_color` in CliTheme::load call
//! - mut-000457 : `args.json → !(args.json)` in project-exists error branch
//! - mut-000459 : `index_file.exists() → !(index_file.exists())` in "Found: lash.index.md" diagnostic
//! - mut-000460 / mut-000491 : `lash_dir.exists() → !(lash_dir.exists())` in "Found: .lash/" diagnostic
//! - mut-000465 : `!args.no_index → args.no_index` in indexing guard
//! - mut-000468 : `args.json → !(args.json)` in print_success_message
//! - mut-000470 : `force: true → force: false` in run_index IndexArgs
//! - mut-000471 : `show_files: false → show_files: true` in run_index IndexArgs
//! - mut-000472 : `exit_code != 0 → !(exit_code != 0)` in run_index guard
//! - mut-000473 : `!= → ==` in run_index guard
//! - mut-000474 : literal `0 → 1` in run_index guard

#![allow(deprecated)] // for Command::cargo_bin

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn lash_cmd() -> Command {
    let mut cmd = Command::cargo_bin("lash").unwrap();
    // Remove environment variables that could interfere with color detection.
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("FORCE_COLOR");
    cmd
}

// ---------------------------------------------------------------------------
// mut-000483: `!args.no_color` → `args.no_color` in CliTheme::load
//
// `CliTheme::load(None, !args.no_color)` is called in `execute()`.  When
// mutated to `args.no_color`, the logic inverts:
//   - With `--no-color`: original passes `false` (no theme); mutation passes `true` (loads theme)
//   - Without `--no-color`: original passes `true` (loads theme); mutation passes `false` (no theme)
//
// We distinguish the two by setting FORCE_COLOR=1 (forces owo-colors to emit
// ANSI escape codes even to piped stdout) and checking whether ANSI codes
// appear:
//   - `--no-color` + FORCE_COLOR=1: original → no ANSI (theme is None)
//                                   mutation → ANSI present (theme is Some)
//   - no `--no-color` + FORCE_COLOR=1: original → ANSI present (theme is Some)
//                                       mutation → no ANSI (theme is None)
// ---------------------------------------------------------------------------

/// With `--no-color` and FORCE_COLOR=1, `lash init` stdout must contain no
/// ANSI escape codes.  Kills mut-000483: if `args.no_color` is used instead
/// of `!args.no_color`, --no-color would pass `true` to CliTheme::load and
/// produce ANSI output even when the user requested no color.
#[test]
fn test_init_no_color_suppresses_ansi_with_force_color() {
    let td = TempDir::new().unwrap();

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--path")
        .arg(td.path())
        .arg("--no-index")
        .env("FORCE_COLOR", "1")
        .output()
        .expect("lash must run");

    assert!(
        output.status.success(),
        "init must succeed in a fresh directory"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains('\x1b'),
        "--no-color init must produce no ANSI codes even with FORCE_COLOR=1:\n{stdout}"
    );
    // Must still contain the success message (we printed something)
    assert!(
        stdout.contains("initialized") || stdout.contains("Lash project"),
        "--no-color init must still print a success message:\n{stdout}"
    );
}

/// Without `--no-color` and with FORCE_COLOR=1, `lash init` stdout must
/// contain ANSI escape codes (the theme was loaded and applied).  Kills
/// mut-000483: if `args.no_color` replaces `!args.no_color`, the theme would
/// not be loaded when no_color=false, producing plain text instead.
#[test]
fn test_init_without_no_color_produces_ansi_with_force_color() {
    let td = TempDir::new().unwrap();

    let output = lash_cmd()
        // no --no-color → !args.no_color = true → theme loaded
        .arg("init")
        .arg("--path")
        .arg(td.path())
        .arg("--no-index")
        .env("FORCE_COLOR", "1")
        .output()
        .expect("lash must run");

    assert!(
        output.status.success(),
        "init must succeed in a fresh directory"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('\x1b'),
        "init without --no-color must produce ANSI codes when FORCE_COLOR=1:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000482: `args.json → !(args.json)` in theme-loading guard
//
// The theme-loading guard in `execute()`:
//   let theme = if args.json { None } else { CliTheme::load(None, !args.no_color)? };
//
// With negation (`!(args.json)`):
//   - json=true  → goes to else → CliTheme::load(...) called (theme may be Some)
//   - json=false → goes to if  → theme is always None
//
// The directly observable effect: with json=false + FORCE_COLOR=1,
//   original → theme=Some → ANSI in stdout's print_success_message
//   mutation → theme=None → plain text in stdout's print_success_message
//
// This is the complement of the mut-000483 tests above but exercises the
// json=false branch explicitly.
// ---------------------------------------------------------------------------

/// With `--json`, init must produce valid JSON output (not themed text).
/// Kills mut-000482: with negation, json=true would load the theme and the
/// output branch selection logic would still emit JSON (since print_success_message
/// checks args.json independently), but this test confirms the JSON path is taken.
#[test]
fn test_init_json_flag_produces_json_stdout() {
    let td = TempDir::new().unwrap();

    let output = lash_cmd()
        .arg("--json")
        .arg("init")
        .arg("--path")
        .arg(td.path())
        .arg("--no-index")
        .output()
        .expect("lash must run");

    assert!(output.status.success(), "init --json must succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json init must produce valid JSON");
    assert!(
        parsed.get("success").is_some() || parsed.get("path").is_some(),
        "--json init JSON must contain 'success' or 'path' key:\n{stdout}"
    );
    // Must not contain ANSI codes (JSON output must be clean)
    assert!(
        !stdout.contains('\x1b'),
        "--json init must produce no ANSI codes:\n{stdout}"
    );
}

/// Without `--json` and with FORCE_COLOR=1, init must produce themed text
/// (not JSON).  Kills mut-000482: with negation, json=false would make
/// theme=None, which triggers the plain-text branch of print_success_message
/// (no ANSI codes even with FORCE_COLOR=1).
#[test]
fn test_init_no_json_flag_produces_ansi_text_with_force_color() {
    let td = TempDir::new().unwrap();

    let output = lash_cmd()
        // no --json and no --no-color → theme loaded
        .arg("init")
        .arg("--path")
        .arg(td.path())
        .arg("--no-index")
        .env("FORCE_COLOR", "1")
        .output()
        .expect("lash must run");

    assert!(output.status.success(), "init must succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Without --json, output must not be parseable as JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_err(),
        "init without --json must not produce JSON:\n{stdout}"
    );
    // With FORCE_COLOR=1 and no --no-color, themed text must include ANSI codes
    assert!(
        stdout.contains('\x1b'),
        "init without --json and --no-color must produce ANSI with FORCE_COLOR=1:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000491: `lash_dir.exists() → !(lash_dir.exists())`
//
// In the "project already exists" error branch:
//   if lash_dir.exists() {
//       eprintln!("  Found: .lash/");
//   }
//
// With the mutation `!(lash_dir.exists())`:
//   - .lash/ exists → does NOT print "  Found: .lash/" (condition is false)
//   - .lash/ absent → DOES print "  Found: .lash/" (condition is true)
//
// We kill this by:
//   1. A project where ONLY .lash/ exists → stderr MUST contain "Found: .lash/"
//   2. A project where ONLY lash.index.md exists → stderr must NOT contain "Found: .lash/"
// ---------------------------------------------------------------------------

/// When only the `.lash/` directory exists (no `lash.index.md`), stderr must
/// report "Found: .lash/".  Kills mut-000491: with the negation, .lash/ exists
/// → condition is false → "Found: .lash/" would NOT be printed.
#[test]
fn test_init_existing_lash_dir_reported_in_stderr() {
    let td = TempDir::new().unwrap();
    // Create only the .lash/ directory, not lash.index.md
    fs::create_dir_all(td.path().join(".lash")).unwrap();
    assert!(!td.path().join("lash.index.md").exists());

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--path")
        .arg(td.path())
        .arg("--no-index")
        .output()
        .expect("lash must run");

    // Exit code 1: project already exists
    assert_eq!(
        output.status.code().unwrap_or(-1),
        1,
        "init with existing .lash/ must exit 1"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Found: .lash/"),
        "stderr must report 'Found: .lash/' when .lash/ exists (mut-000491):\n{stderr}"
    );
    // Must NOT report lash.index.md since it doesn't exist
    assert!(
        !stderr.contains("Found: lash.index.md"),
        "stderr must not report 'Found: lash.index.md' when it doesn't exist:\n{stderr}"
    );
}

/// When only `lash.index.md` exists (no `.lash/` directory), stderr must NOT
/// contain "Found: .lash/".  Kills mut-000491: with the negation, .lash/
/// absent → condition `!(lash_dir.exists())` is true → "Found: .lash/" IS
/// printed incorrectly.
#[test]
fn test_init_absent_lash_dir_not_reported_in_stderr() {
    let td = TempDir::new().unwrap();
    // Create only lash.index.md, not .lash/
    fs::write(td.path().join("lash.index.md"), "# Existing").unwrap();
    assert!(!td.path().join(".lash").exists());

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--path")
        .arg(td.path())
        .arg("--no-index")
        .output()
        .expect("lash must run");

    // Exit code 1: project already exists
    assert_eq!(
        output.status.code().unwrap_or(-1),
        1,
        "init with existing lash.index.md must exit 1"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Must report lash.index.md (it exists)
    assert!(
        stderr.contains("Found: lash.index.md"),
        "stderr must report 'Found: lash.index.md' when it exists:\n{stderr}"
    );
    // Must NOT report .lash/ since it doesn't exist
    assert!(
        !stderr.contains("Found: .lash/"),
        "stderr must NOT report 'Found: .lash/' when it doesn't exist (mut-000491):\n{stderr}"
    );
}

/// When both `lash.index.md` and `.lash/` exist, stderr must report both.
/// This is a complementary test confirming the correct lash_dir.exists() path.
#[test]
fn test_init_both_existing_files_reported_in_stderr() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("lash.index.md"), "# Existing").unwrap();
    fs::create_dir_all(td.path().join(".lash")).unwrap();

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--path")
        .arg(td.path())
        .arg("--no-index")
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        1,
        "init with existing project must exit 1"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Found: lash.index.md"),
        "stderr must report lash.index.md:\n{stderr}"
    );
    assert!(
        stderr.contains("Found: .lash/"),
        "stderr must report .lash/ when both exist:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// mut-000457 : `args.json → !(args.json)` in the project-already-exists
// error branch (line 66 of init.rs).
//
// Original : when project exists and --json is given, a JSON error object is
//            printed to stdout and exit code 1 is returned.
// Mutation : branches are inverted – --json gets plain-text error (not JSON)
//            and text mode gets a JSON error object instead.
//
// Kill: assert that --json + existing project → stdout is valid JSON with
//       an "error" key, and that text mode → stdout is NOT JSON.
// ---------------------------------------------------------------------------

/// Kill mut-000457: `--json` + existing project must emit a JSON error object.
#[test]
fn test_init_json_existing_project_emits_json_error() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("lash.index.md"), "# Existing").unwrap();

    let output = lash_cmd()
        .arg("--json")
        .arg("init")
        .arg("--no-index")
        .arg("--path")
        .arg(td.path())
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        1,
        "--json init on existing project must exit 1"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json error output must be valid JSON, got stdout");
    assert!(
        v.get("error").is_some(),
        "JSON error must contain 'error' key, got: {stdout}"
    );
    assert!(
        v["error"]
            .as_str()
            .is_some_and(|s| s.contains("already exists")),
        "JSON 'error' must say 'already exists', got: {stdout}"
    );
}

/// Kill mut-000457 (complement): text mode + existing project must NOT emit JSON
/// on stdout; the error goes to stderr.
#[test]
fn test_init_text_existing_project_error_on_stderr_not_stdout() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("lash.index.md"), "# Existing").unwrap();

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--no-index")
        .arg("--path")
        .arg(td.path())
        .output()
        .expect("lash must run");

    assert_eq!(
        output.status.code().unwrap_or(-1),
        1,
        "text-mode init on existing project must exit 1"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // stdout must NOT be JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_err() || stdout.trim().is_empty(),
        "text-mode error must not produce JSON on stdout, got: {stdout}"
    );
    // stderr must mention the error
    assert!(
        stderr.contains("already exists"),
        "text-mode error must mention 'already exists' in stderr, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// mut-000459 : `index_file.exists()` → `!(index_file.exists())` at line 80
//
// In the project-already-exists error branch (text mode only):
//   if index_file.exists() { eprintln!("  Found: lash.index.md"); }
//
// With mutation `!(index_file.exists())`:
//   - index file exists  → condition is false → NOT printed
//   - index file absent  → condition is true  → wrongly printed
//
// Kill: assert that stderr contains "Found: lash.index.md" when the file
//       exists and does NOT contain it when the file is absent.
// ---------------------------------------------------------------------------

/// Kill mut-000459: when only `lash.index.md` exists, stderr must report
/// "Found: lash.index.md".
#[test]
fn test_init_found_index_file_message_when_index_file_exists() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("lash.index.md"), "# Existing").unwrap();
    assert!(!td.path().join(".lash").exists());

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--no-index")
        .arg("--path")
        .arg(td.path())
        .output()
        .expect("lash must run");

    assert_eq!(output.status.code().unwrap_or(-1), 1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Found: lash.index.md"),
        "stderr must report 'Found: lash.index.md' when it exists (mut-000459): {stderr}"
    );
    assert!(
        !stderr.contains("Found: .lash/"),
        "stderr must NOT report 'Found: .lash/' when .lash is absent: {stderr}"
    );
}

/// Kill mut-000459 (complement): when only `.lash/` exists (no index file),
/// stderr must NOT report "Found: lash.index.md".
#[test]
fn test_init_no_found_index_file_message_when_index_file_absent() {
    let td = TempDir::new().unwrap();
    fs::create_dir_all(td.path().join(".lash")).unwrap();
    assert!(!td.path().join("lash.index.md").exists());

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--no-index")
        .arg("--path")
        .arg(td.path())
        .output()
        .expect("lash must run");

    assert_eq!(output.status.code().unwrap_or(-1), 1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Found: lash.index.md"),
        "stderr must NOT report 'Found: lash.index.md' when it is absent (mut-000459): {stderr}"
    );
    assert!(
        stderr.contains("Found: .lash/"),
        "stderr must report 'Found: .lash/' when it exists: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// mut-000465 : `!args.no_index` → `args.no_index` at line 106
//
// Original : `if !args.no_index { run_index(...) }`
//   - --no-index (no_index=true)  → skip indexing, no lash.db created
//   - no --no-index (no_index=false) → run indexing, lash.db created
//
// Mutation : `if args.no_index { run_index(...) }` — semantics are inverted.
//
// Kill: assert lash.db is absent with --no-index and present without --no-index.
// ---------------------------------------------------------------------------

/// Kill mut-000465: `--no-index` must prevent `lash.db` from being created.
#[test]
fn test_init_no_index_flag_skips_database_creation() {
    let td = TempDir::new().unwrap();

    lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--no-index")
        .arg("--path")
        .arg(td.path())
        .assert()
        .success();

    assert!(
        !td.path().join(".lash").join("lash.db").exists(),
        "lash.db must NOT exist when --no-index is passed (mut-000465)"
    );
}

/// Kill mut-000465 (complement): without `--no-index`, `lash.db` must be created.
#[test]
fn test_init_without_no_index_creates_database() {
    let td = TempDir::new().unwrap();

    lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--path")
        .arg(td.path())
        .assert()
        .success();

    assert!(
        td.path().join(".lash").join("lash.db").exists(),
        "lash.db must exist when --no-index is NOT passed (mut-000465)"
    );
}

// ---------------------------------------------------------------------------
// mut-000468 : `args.json → !(args.json)` in `print_success_message` (line 135)
//
// Original : `if args.json { JSON output } else if let Some(t) = theme { ... } else { text }`
// Mutation : branches inverted – --json gets text/theme output, text mode gets JSON
//
// Kill: assert --json success output is a valid JSON object with expected keys,
//       and text mode is NOT JSON.
// ---------------------------------------------------------------------------

/// Kill mut-000468: `--json` init success must emit a JSON object with `success: true`.
#[test]
fn test_init_json_success_message_is_json_object() {
    let td = TempDir::new().unwrap();

    let output = lash_cmd()
        .arg("--json")
        .arg("init")
        .arg("--no-index")
        .arg("--path")
        .arg(td.path())
        .output()
        .expect("lash must run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json success must be valid JSON");
    assert_eq!(
        v["success"].as_bool(),
        Some(true),
        "JSON success must have success=true (mut-000468): {stdout}"
    );
    assert_eq!(
        v["indexed"].as_bool(),
        Some(false),
        "JSON success must have indexed=false with --no-index (mut-000468): {stdout}"
    );
}

/// Kill mut-000468 (complement): text-mode init success must NOT be JSON.
#[test]
fn test_init_text_success_message_is_not_json() {
    let td = TempDir::new().unwrap();

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--no-index")
        .arg("--path")
        .arg(td.path())
        .output()
        .expect("lash must run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_err(),
        "text-mode success must not be JSON (mut-000468): {stdout}"
    );
    assert!(
        stdout.contains("initialized successfully"),
        "text-mode success must contain 'initialized successfully': {stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000470 : `force: true → force: false` in run_index IndexArgs
//
// run_index always passes `force: true` to the index command so that a stale
// or corrupt database is rebuilt.  With force: false (mutation), a corrupt
// database would cause open_database to fail, run_index returns Err (non-fatal
// warning), and the database remains corrupt.
//
// Kill: place a corrupt lash.db, run `init --force`, assert the database is
//       a valid SQLite file afterwards (was rebuilt).
// ---------------------------------------------------------------------------

/// Kill mut-000470: `init --force` must rebuild a corrupt `.lash/lash.db`.
/// This relies on run_index passing `force: true` to the index command.
#[test]
fn test_init_force_rebuilds_corrupt_db() {
    let td = TempDir::new().unwrap();
    let lash_dir = td.path().join(".lash");
    fs::create_dir_all(&lash_dir).unwrap();
    fs::write(lash_dir.join("lash.db"), b"not a sqlite database").unwrap();

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--force")
        .arg("--path")
        .arg(td.path())
        .output()
        .expect("lash must run");

    assert!(
        output.status.success(),
        "init --force with corrupt DB must succeed (mut-000470)"
    );
    let db = lash_dir.join("lash.db");
    let bytes = fs::read(&db).unwrap();
    assert_eq!(
        &bytes[..16],
        b"SQLite format 3\0",
        "lash.db must be valid SQLite after init --force (mut-000470)"
    );
    // No warning about indexing failure must appear
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Warning: Initial indexing failed"),
        "stderr must not warn about indexing failure (mut-000470): {stderr}"
    );
}

// ---------------------------------------------------------------------------
// mut-000471 : `show_files: false → show_files: true` in run_index IndexArgs
//
// When show_files=true and json=false, a progress bar is created in the index
// command.  In non-TTY environments (test subprocess) progress bars are
// suppressed, so the observable output is unchanged.  The strongest assertion
// is that init completes successfully with a valid database.
// ---------------------------------------------------------------------------

/// Kill mut-000471 (best-effort): init without --no-index must complete
/// successfully and produce a valid database regardless of show_files value.
#[test]
fn test_init_index_completes_with_valid_db_show_files_variant() {
    let td = TempDir::new().unwrap();

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--path")
        .arg(td.path())
        .output()
        .expect("lash must run");

    assert!(
        output.status.success(),
        "init without --no-index must succeed (mut-000471)"
    );
    let db = td.path().join(".lash").join("lash.db");
    assert!(db.exists(), "lash.db must exist (mut-000471)");
    let bytes = fs::read(&db).unwrap();
    assert_eq!(
        &bytes[..16],
        b"SQLite format 3\0",
        "lash.db must be valid SQLite (mut-000471)"
    );
}

// ---------------------------------------------------------------------------
// mut-000472 : `exit_code != 0 → !(exit_code != 0)` = `exit_code == 0`
// mut-000473 : `!=` → `==`
// mut-000474 : literal `0 → 1`
//
// All three affect the guard in run_index:
//   `if exit_code != 0 { anyhow::bail!(...) }`
//
// With any mutation, a SUCCESSFUL index (exit_code=0) would trigger bail!,
// making run_index return Err.  In execute(), this becomes a non-fatal warning
// printed to stderr and the database may not be fully created/valid.
//
// Kill: run init without --no-index on a clean project; assert:
//   1. stderr does NOT contain the indexing-failed warning
//   2. lash.db IS a valid SQLite file (index ran to completion)
// ---------------------------------------------------------------------------

/// Kill mut-000472/473/474: a successful index must not trigger the bail! guard.
/// Observable as: no warning in stderr AND a valid SQLite database on disk.
#[test]
fn test_init_successful_index_no_warning_and_valid_db() {
    let td = TempDir::new().unwrap();

    let output = lash_cmd()
        .arg("--no-color")
        .arg("init")
        .arg("--path")
        .arg(td.path())
        .output()
        .expect("lash must run");

    assert!(
        output.status.success(),
        "init must exit with code 0 (mut-000472/473/474)"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Warning: Initial indexing failed"),
        "stderr must NOT contain warning when index succeeds (mut-000472/473/474): {stderr}"
    );
    let db = td.path().join(".lash").join("lash.db");
    assert!(db.exists(), "lash.db must exist (mut-000472/473/474)");
    let bytes = fs::read(&db).unwrap();
    assert_eq!(
        &bytes[..16],
        b"SQLite format 3\0",
        "lash.db must be valid SQLite (mut-000472/473/474)"
    );
}
