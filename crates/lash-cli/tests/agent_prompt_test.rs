//! Targeted mutation-killing tests for the agent-prompt command.
//!
//! Each test is designed to distinguish the original code from a specific
//! mutation. The comments above each test explain which mutant it kills and
//! why the original code passes while the mutated code would fail.

#![allow(deprecated)] // assert_cmd::Command::cargo_bin

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn lash_cmd() -> Command {
    Command::cargo_bin("lash").expect("Failed to find lash binary")
}

/// Create a minimal valid lash project in `dir`.
fn create_minimal_project(dir: &std::path::Path) {
    fs::write(
        dir.join("lash.index.md"),
        "# Test Project\n\n@id: test-project\n@created: 2024-01-15\n\n## Tasks\n\n- [ ] Task\n",
    )
    .expect("Failed to write index file");
}

/// Create a fake HOME directory whose `.lash/config.toml` specifies a color
/// scheme name that does not exist in the built-in registry.
///
/// Returns the `TempDir` that must be kept alive for the duration of the test.
fn create_bad_scheme_home() -> TempDir {
    let temp_home = tempfile::tempdir().expect("Failed to create temp home");
    let lash_config_dir = temp_home.path().join(".lash");
    fs::create_dir_all(&lash_config_dir).expect("Failed to create .lash dir in temp home");
    fs::write(
        lash_config_dir.join("config.toml"),
        // This scheme name does not exist in the REGISTRY; CliTheme::load will
        // propagate Err("Color scheme … not found") when colors_enabled=true.
        "color_scheme = \"NonExistentSchemeForMutationTest\"\n",
    )
    .expect("Failed to write bad scheme config");
    temp_home
}

// ---------------------------------------------------------------------------
// mut-000147: `args.json` replaced with `!(args.json)` in theme-loading branch
//
// With the mutation the code becomes:
//   if !(args.json) { None } else { CliTheme::load(None, !args.no_color)? }
//
// When `--json` is passed (args.json = true) the mutation incorrectly enters
// the *else* branch and calls CliTheme::load.  If that call fails (bad scheme
// in user config), the command exits with an error.
//
// Original code: `--json` → None (CliTheme::load is never called) → success.
// Mutated code:  `--json` → CliTheme::load("NonExistentScheme") → Err → failure.
// ---------------------------------------------------------------------------

/// Passing `--json` must succeed even when the user config specifies a color
/// scheme that does not exist.  The json branch must short-circuit to None
/// without ever calling CliTheme::load.
///
/// Kills mut-000147.
#[test]
fn test_agent_prompt_json_flag_skips_theme_loading() {
    let project_dir = tempfile::tempdir().expect("Failed to create project dir");
    create_minimal_project(project_dir.path());

    let bad_home = create_bad_scheme_home();

    lash_cmd()
        .env("HOME", bad_home.path())
        // --json sets args.json=true; with original code theme=None (no load)
        .arg("--json")
        .arg("--root")
        .arg(project_dir.path())
        .arg("agent-prompt")
        .assert()
        .success();

    // Keep alive
    drop(bad_home);
}

// ---------------------------------------------------------------------------
// mut-000148: `!args.no_color` replaced with `args.no_color` inside the else
// branch of theme-loading.
//
// Original:  CliTheme::load(None, !args.no_color)
//            → with --no-color: load(None, false) → Ok(None) (fast path, no
//              user config read and no registry lookup)
//
// Mutated:   CliTheme::load(None, args.no_color)
//            → with --no-color: load(None, true) → reads user config →
//              looks up bad scheme in registry → Err → command fails.
// ---------------------------------------------------------------------------

/// `--no-color` must disable color loading entirely (pass `false` to
/// `CliTheme::load`), bypassing the scheme registry lookup.  A user config
/// with an invalid scheme name must not cause a failure when colors are
/// disabled.
///
/// Kills mut-000148.
#[test]
fn test_agent_prompt_no_color_disables_theme_loading() {
    let project_dir = tempfile::tempdir().expect("Failed to create project dir");
    create_minimal_project(project_dir.path());

    let bad_home = create_bad_scheme_home();

    lash_cmd()
        .env("HOME", bad_home.path())
        // --no-color sets args.no_color=true; original passes !true=false to
        // CliTheme::load, which returns Ok(None) without touching the registry.
        .arg("--no-color")
        .arg("--root")
        .arg(project_dir.path())
        .arg("agent-prompt")
        .assert()
        .success();

    drop(bad_home);
}

// ---------------------------------------------------------------------------
// mut-000151: the entire condition `prompt.truncated && !args.json` is negated
// to `!(prompt.truncated && !args.json)`.
//
// When truncated=true and json=false:
//   Original:  true  && true  = true  → truncation warning IS printed to stderr
//   Mutated:  !(true && true) = false → truncation warning is NOT printed
//
// mut-000153: `!args.json` is replaced by `args.json`.
//
// When truncated=true and json=false:
//   Original:  true && !false = true  → warning printed
//   Mutated:   true &&  false = false → warning NOT printed
//
// Both mutants are killed by the same test: force truncation (--max-tokens 1,
// plain format) without --json and verify the truncation note appears in stderr.
// ---------------------------------------------------------------------------

/// When the prompt is truncated and JSON mode is off, a "Content was
/// truncated" note must appear on stderr.
///
/// Kills mut-000151 and mut-000153.
#[test]
fn test_agent_prompt_truncation_warning_printed_when_truncated_and_not_json() {
    let project_dir = tempfile::tempdir().expect("Failed to create project dir");
    create_minimal_project(project_dir.path());

    // --max-tokens 1 forces the prompt to be truncated (budget is far below
    // the minimum content size).  --format plain is required because only the
    // plain builder sets truncated=true.  --no-color keeps the warning as
    // plain text so the assertion is not confused by ANSI escape codes.
    let output = lash_cmd()
        .arg("--no-color")
        .arg("--root")
        .arg(project_dir.path())
        .arg("agent-prompt")
        .arg("--format")
        .arg("plain")
        .arg("--max-tokens")
        .arg("1")
        .output()
        .expect("Failed to run command");

    assert!(output.status.success(), "Command must succeed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("truncated"),
        "Truncation warning must appear on stderr when content is truncated and \
         json=false; got stderr: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// mut-000152: `&&` replaced with `||` in `prompt.truncated && !args.json`.
//
// To distinguish && from || we need exactly one sub-condition to be true.
// The cleanest scenario: truncated=false (no --max-tokens), json=false
// (no --json).
//
//   Original: false && true = false → NO warning printed
//   Mutated:  false || true = true  → warning IS printed (false positive)
//
// Test: no --max-tokens → truncated=false; no --json → json=false.
// Assert that stderr does NOT contain the truncation warning.
// ---------------------------------------------------------------------------

/// When the prompt is NOT truncated (no token budget set), the truncation
/// warning must not appear in stderr regardless of the json flag.
///
/// Kills mut-000152.
#[test]
fn test_agent_prompt_no_truncation_warning_when_not_truncated() {
    let project_dir = tempfile::tempdir().expect("Failed to create project dir");
    create_minimal_project(project_dir.path());

    // No --max-tokens means truncated=false; no --json means json=false.
    // The && condition evaluates false && true = false → no warning.
    // The || mutation would evaluate false || true = true → spurious warning.
    let output = lash_cmd()
        .arg("--no-color")
        .arg("--root")
        .arg(project_dir.path())
        .arg("agent-prompt")
        .output()
        .expect("Failed to run command");

    assert!(output.status.success(), "Command must succeed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("truncated"),
        "No truncation warning should appear when content is not truncated; \
         got stderr: {stderr}"
    );
}
