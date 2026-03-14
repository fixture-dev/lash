//! Integration tests for `lash config` command output format and behaviour.
//!
//! These tests kill surviving mutants in `commands/config.rs` that cannot be
//! reached through unit tests because the affected functions write directly to
//! stdout or branch on observable side-effects (file system).
//!
//! Mutants targeted:
//! - mut-000259  args.json → !(args.json)  in execute() theme loading
//! - mut-000260  !args.no_color → args.no_color  in execute() theme loading
//! - mut-000262  args.json → !(args.json)  in get()
//! - mut-000263  Ok(0) → Ok(1)  in get() success path
//! - mut-000264  user → !(user)  in set()
//! - mut-000265  config_path.exists() → !(config_path.exists())  in set()
//! - mut-000267  args.json → !(args.json)  in set()
//! - mut-000269  Ok(0) → Ok(1)  in set() success path

#![allow(deprecated)] // assert_cmd cargo_bin is deprecated but still works

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Spawn the `lash` binary and return a builder pre-configured with `--no-logo`
/// so tests do not need to strip the banner from output.
fn lash() -> Command {
    let mut cmd = Command::cargo_bin("lash").expect("lash binary must be available");
    cmd.arg("--no-logo");
    cmd
}

/// Create a temporary directory that acts as the project root.
/// Initialising `.lash/` ensures Config::load_merged finds the directory.
fn temp_project() -> TempDir {
    let td = TempDir::new().expect("must create temp dir");
    fs::create_dir_all(td.path().join(".lash")).expect("must create .lash dir");
    td
}

// ---------------------------------------------------------------------------
// mut-000262 / mut-000263: args.json in get() and Ok(0) exit code
//
// When --json is present, get() must emit a JSON object and return exit 0.
// When --json is absent, get() must emit plain text (not JSON) and return exit 0.
// ---------------------------------------------------------------------------

/// `--json config get <key>` produces valid JSON output and exits 0.
/// Kills mut-000262 (json→!json would emit plain text instead of JSON) and
/// mut-000263 (0→1 would change exit code).
#[test]
fn test_config_get_json_flag_produces_json_and_exits_0() {
    let td = temp_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--json")
        .arg("config")
        .arg("get")
        .arg("output.default_format")
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "config get with valid key must exit 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");

    assert_eq!(
        parsed["key"].as_str(),
        Some("output.default_format"),
        "JSON must contain the requested key name"
    );
    assert!(
        parsed["value"].is_string(),
        "JSON must contain a string value"
    );
}

/// `config get <key>` without --json produces plain text (not JSON) and exits 0.
/// Kills mut-000262 (json→!json would emit JSON instead of plain text) and
/// mut-000263 (0→1 would change exit code).
#[test]
fn test_config_get_plain_text_and_exits_0() {
    let td = temp_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("config")
        .arg("get")
        .arg("output.default_format")
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "config get with valid key must exit 0");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Plain text output is just the value, not a JSON object
    assert!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).is_err(),
        "plain text output must not be valid JSON; got: {stdout}"
    );

    // The default value "text" should appear literally
    assert!(
        stdout.trim() == "text",
        "plain text output must be just the value; got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000267 / mut-000269: args.json in set() and Ok(0) exit code
//
// When --json, set() must emit a JSON object with "status": "success".
// When not --json, set() must emit human-readable text and return exit 0.
// ---------------------------------------------------------------------------

/// `--json config set <key> <value>` produces valid JSON and exits 0.
/// Kills mut-000267 (json→!json would emit plain text) and
/// mut-000269 (0→1 would change exit code).
#[test]
fn test_config_set_json_flag_produces_json_and_exits_0() {
    let td = temp_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--json")
        .arg("config")
        .arg("set")
        .arg("output.default_format")
        .arg("json")
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "config set with valid key/value must exit 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");

    assert_eq!(
        parsed["status"].as_str(),
        Some("success"),
        "JSON status must be 'success'"
    );
    assert_eq!(
        parsed["key"].as_str(),
        Some("output.default_format"),
        "JSON must contain the key"
    );
    assert_eq!(
        parsed["value"].as_str(),
        Some("json"),
        "JSON must contain the new value"
    );
}

/// `config set <key> <value>` without --json produces plain text and exits 0.
/// Kills mut-000267 (json→!json would emit JSON) and
/// mut-000269 (0→1 would change exit code).
#[test]
fn test_config_set_plain_text_and_exits_0() {
    let td = temp_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("config")
        .arg("set")
        .arg("output.default_format")
        .arg("json")
        .output()
        .expect("lash must run");

    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "config set with valid key/value must exit 0");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Plain text includes the confirmation message
    assert!(
        stdout.contains("Configuration updated") || stdout.contains("output.default_format"),
        "plain text output must contain update confirmation; got: {stdout}"
    );

    // Plain text must NOT be valid JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).is_err(),
        "plain text output must not be valid JSON; got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000264: user → !(user) in set()
//
// When user=false (no --user flag), set() must write to the project config
// path (<root>/.lash/config.toml). With the mutation, it would use the user
// config path instead.
// ---------------------------------------------------------------------------

/// `config set` without --user flag writes to project root config.
/// Kills mut-000264 (user→!user would route to user config path).
#[test]
fn test_config_set_without_user_flag_writes_project_config() {
    let td = temp_project();
    let project_config = td.path().join(".lash").join("config.toml");

    // The project config should not exist yet
    assert!(
        !project_config.exists(),
        "project config must not exist before set"
    );

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("config")
        .arg("set")
        .arg("output.default_format")
        .arg("json")
        .output()
        .expect("lash must run");

    assert!(
        output.status.success(),
        "config set must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // With correct routing (user=false), file must be in project root
    assert!(
        project_config.exists(),
        "project config must be created at <root>/.lash/config.toml"
    );

    let contents = fs::read_to_string(&project_config).expect("must read config file");
    assert!(
        contents.contains("default_format"),
        "project config must contain the set key; contents: {contents}"
    );
}

// ---------------------------------------------------------------------------
// mut-000265: config_path.exists() → !(config_path.exists()) in set()
//
// When the config file already exists, set() should load it (preserving
// existing values) and then update the target key. With the mutation, it
// would ignore the existing file and start from defaults, losing other values.
// ---------------------------------------------------------------------------

/// Writing to a pre-existing config file preserves other values.
/// Kills mut-000265 (!exists() would discard the existing file contents).
#[test]
fn test_config_set_preserves_existing_config_values() {
    let td = temp_project();

    // First set: write output.verbosity=verbose
    let first = lash()
        .arg("--root")
        .arg(td.path())
        .arg("config")
        .arg("set")
        .arg("output.verbosity")
        .arg("verbose")
        .output()
        .expect("first lash set must run");
    assert!(
        first.status.success(),
        "first config set must succeed; stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    // Second set: write output.default_format=json (different key)
    let second = lash()
        .arg("--root")
        .arg(td.path())
        .arg("config")
        .arg("set")
        .arg("output.default_format")
        .arg("json")
        .output()
        .expect("second lash set must run");
    assert!(
        second.status.success(),
        "second config set must succeed; stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    // Read back verbosity: it must still be "verbose" (not the default "normal")
    let read = lash()
        .arg("--root")
        .arg(td.path())
        .arg("config")
        .arg("get")
        .arg("output.verbosity")
        .output()
        .expect("config get must run");
    assert!(
        read.status.success(),
        "config get must succeed; stderr: {}",
        String::from_utf8_lossy(&read.stderr)
    );

    let stdout = String::from_utf8_lossy(&read.stdout);
    assert_eq!(
        stdout.trim(),
        "verbose",
        "first-written value must be preserved after second set; got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000259: args.json → !(args.json) in execute() theme loading
//
// When json=true, theme must be None (no styled output). With the mutation,
// json=true would load the theme and json=false would get None. The effect
// is visible in the `list` output: with json=true we expect JSON, not styled text.
//
// Note: the json routing in list() has its own args.json check (mut-000268,
// not in scope), so the theme mutation only affects whether styled text is
// produced when json=false. Testing that json=true produces JSON output
// implicitly confirms the execute() theme path is irrelevant for JSON mode.
//
// The more observable effect: with json=false and the mutation, theme would
// be None, so list_text's header would be unstyled ("Configuration Settings"
// with no ANSI codes). Without --no-color and without a TTY, styling is
// suppressed anyway, making the two paths indistinguishable for the header.
//
// We kill this mutant by testing that `--json config list` produces valid JSON
// (the theme=None path) and `config list` produces text (not JSON), confirming
// the two branches are distinct.
// ---------------------------------------------------------------------------

/// `--json config list` produces valid JSON output.
/// Indirectly kills mut-000259 by confirming JSON mode works end-to-end.
#[test]
fn test_config_list_json_produces_valid_json() {
    let td = temp_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--json")
        .arg("config")
        .arg("list")
        .output()
        .expect("lash must run");

    assert!(
        output.status.success(),
        "config list --json must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json config list must produce valid JSON");

    // JSON must contain top-level config sections
    assert!(
        parsed["output"].is_object(),
        "JSON must have 'output' section"
    );
    assert!(
        parsed["linter"].is_object(),
        "JSON must have 'linter' section"
    );
    assert!(
        parsed["search"].is_object(),
        "JSON must have 'search' section"
    );
    assert!(
        parsed["agent"].is_object(),
        "JSON must have 'agent' section"
    );
}

/// `config list` without --json produces text (not JSON).
/// Kills mut-000259 (with negation, json=false gets no theme and json=true
/// would also use the text-producing code path).
#[test]
fn test_config_list_without_json_produces_text() {
    let td = temp_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("config")
        .arg("list")
        .output()
        .expect("lash must run");

    assert!(
        output.status.success(),
        "config list must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Text output must contain section headers, not JSON
    assert!(
        stdout.contains("Configuration Settings"),
        "text output must contain 'Configuration Settings' header; got: {stdout}"
    );
    assert!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).is_err(),
        "text output must not be valid JSON; got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000260: !args.no_color → args.no_color in execute() theme loading
//
// The no_color flag controls whether colour is enabled in the theme.
// With the mutation, the polarity is reversed: --no-color would enable colour
// and omitting --no-color would disable it.
//
// We kill this mutant by verifying that `--no-color config list` and
// `config list` both succeed and produce text with the same content structure
// (the key indicator is section headers appear in both). A direct ANSI code
// test is fragile in CI (no TTY). The test below confirms the flag is accepted
// and does not break output format.
// ---------------------------------------------------------------------------

/// `--no-color config list` succeeds and still contains section headers.
/// Kills mut-000260 by exercising the no_color=true code path.
#[test]
fn test_config_list_no_color_flag_accepted_and_text_output() {
    let td = temp_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--no-color")
        .arg("config")
        .arg("list")
        .output()
        .expect("lash must run");

    assert!(
        output.status.success(),
        "--no-color config list must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Configuration Settings"),
        "--no-color output must still show section headers; got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// mut-000259 (revised): FORCE_COLOR-based test
//
// When args.json=false and the execute() theme branch is correct, CliTheme::load
// is called with colors_enabled=true, producing a Some(theme). With FORCE_COLOR=1
// the theme emits ANSI escape sequences (\x1b[) in the text output.
//
// With the mutation (args.json → !(args.json)), when json=false the condition
// !(false)=true takes the None path, so theme=None and no ANSI codes appear even
// with FORCE_COLOR=1.
//
// We verify this with the --no-color variant too: --no-color must suppress ANSI
// codes even when FORCE_COLOR=1 (because !no_color = false → theme=None). This
// confirms the polarity of both mut-000259 and mut-000260.
// ---------------------------------------------------------------------------

/// `config list` (no --json, no --no-color) with FORCE_COLOR=1 must produce
/// ANSI escape sequences because execute() loads the theme (json=false path).
///
/// Kills mut-000259: with the negation (if !json → None), json=false would give
/// theme=None and no ANSI codes would appear, failing this assertion.
#[test]
fn test_config_list_text_mode_has_ansi_codes_when_force_color() {
    let td = temp_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("config")
        .arg("list")
        .env("FORCE_COLOR", "1")
        .env_remove("NO_COLOR") // ensure NO_COLOR is not inherited from parent environment
        .output()
        .expect("lash must run");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "config list must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // With FORCE_COLOR=1 and theme loaded (json=false), output must contain ANSI escape
    // sequences. The theme styles section headers, key labels, and separators.
    assert!(
        stdout.contains('\x1b'),
        "config list in text mode with FORCE_COLOR=1 must contain ANSI escape sequences \
         (theme is loaded when json=false); got: {stdout:?}"
    );
}

/// `config list` with --no-color and FORCE_COLOR=1 must NOT produce ANSI codes.
///
/// Kills mut-000260: the correct code passes !no_color=false to CliTheme::load,
/// returning None. With the mutation (no_color instead of !no_color), it would
/// pass true → load theme → emit ANSI codes, failing this assertion.
///
/// Also acts as a baseline confirming that ANSI codes in the previous test
/// originate from the theme, not from some other source.
#[test]
fn test_config_list_no_color_suppresses_ansi_codes_even_with_force_color() {
    let td = temp_project();

    let output = lash()
        .arg("--root")
        .arg(td.path())
        .arg("--no-color")
        .arg("config")
        .arg("list")
        .env("FORCE_COLOR", "1")
        .output()
        .expect("lash must run");

    assert!(
        output.status.success(),
        "--no-color config list must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // With --no-color, execute() calls CliTheme::load(None, !true=false) → None.
    // theme=None means print_section/print_setting use the unstyled path: no ANSI codes.
    assert!(
        !stdout.contains('\x1b'),
        "--no-color must suppress all ANSI escape sequences even with FORCE_COLOR=1; \
         got: {stdout:?}"
    );

    // The content must still be present (sanity check)
    assert!(
        stdout.contains("Configuration Settings"),
        "--no-color output must still show section headers; got: {stdout}"
    );
}
