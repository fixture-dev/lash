//! Regression tests using snapshot testing
//!
//! These tests capture the output of various lash commands on fixture projects
//! and compare against snapshots to detect unintended changes.

#![allow(clippy::uninlined_format_args)]
#![allow(unused_variables)]
#![allow(deprecated)]

mod common;

use assert_cmd::Command;
use common::TestProject;
use insta::assert_snapshot;
use std::path::Path;

/// Helper to run lash command and capture output
fn run_lash(args: &[&str], cwd: &Path) -> (String, String, i32) {
    let output = Command::cargo_bin("lash")
        .unwrap()
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to execute lash");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    (stdout, stderr, code)
}

/// Helper to normalize paths in output for consistent snapshots
fn normalize_output(output: &str, project_path: &Path) -> String {
    let path_str = project_path.to_string_lossy();
    output
        .replace(&*path_str, "<PROJECT_ROOT>")
        .replace('\\', "/") // Normalize Windows paths
}

#[test]
fn test_lint_small_project_snapshot() {
    let project = TestProject::from_fixture("small");
    let (stdout, stderr, code) = run_lash(&["lint", "."], project.path());

    assert_eq!(code, 0, "Linting should succeed");

    let normalized_stdout = normalize_output(&stdout, project.path());
    let normalized_stderr = normalize_output(&stderr, project.path());

    assert_snapshot!("lint_small_project_stdout", normalized_stdout);
    assert_snapshot!("lint_small_project_stderr", normalized_stderr);
}

#[test]
fn test_lint_medium_project_snapshot() {
    let project = TestProject::from_fixture("medium");
    let (stdout, stderr, code) = run_lash(&["lint", "."], project.path());

    assert_eq!(code, 0, "Linting should succeed");

    let normalized_stdout = normalize_output(&stdout, project.path());
    let normalized_stderr = normalize_output(&stderr, project.path());

    assert_snapshot!("lint_medium_project_stdout", normalized_stdout);
    assert_snapshot!("lint_medium_project_stderr", normalized_stderr);
}

#[test]
fn test_lint_flat_project_snapshot() {
    let project = TestProject::from_fixture("flat");
    let (stdout, stderr, code) = run_lash(&["lint", "."], project.path());

    assert_eq!(code, 0, "Linting should succeed");

    let normalized_stdout = normalize_output(&stdout, project.path());
    assert_snapshot!("lint_flat_project_stdout", normalized_stdout);
    assert_snapshot!(
        "lint_flat_project_stderr",
        normalize_output(&stderr, project.path())
    );
}

#[test]
fn test_lint_deeply_nested_snapshot() {
    let project = TestProject::from_fixture("deeply-nested");
    let (stdout, stderr, code) = run_lash(&["lint", "."], project.path());

    assert_eq!(code, 0, "Linting should succeed");

    let normalized_stdout = normalize_output(&stdout, project.path());
    assert_snapshot!("lint_deeply_nested_stdout", normalized_stdout);
    assert_snapshot!(
        "lint_deeply_nested_stderr",
        normalize_output(&stderr, project.path())
    );
}

#[test]
fn test_lint_mixed_structure_snapshot() {
    let project = TestProject::from_fixture("mixed-structure");
    let (stdout, stderr, code) = run_lash(&["lint", "."], project.path());

    assert_eq!(code, 0, "Linting should succeed");

    let normalized_stdout = normalize_output(&stdout, project.path());
    assert_snapshot!("lint_mixed_structure_stdout", normalized_stdout);
    assert_snapshot!(
        "lint_mixed_structure_stderr",
        normalize_output(&stderr, project.path())
    );
}

#[test]
fn test_list_all_tasks_snapshot() {
    let project = TestProject::from_fixture("medium");
    let (_stdout, _stderr, code) = run_lash(&["index"], project.path());
    assert_eq!(code, 0, "Indexing should succeed");

    let (stdout, _stderr, code) = run_lash(&["list"], project.path());
    assert_eq!(code, 0, "List should succeed");

    let normalized = normalize_output(&stdout, project.path());
    assert_snapshot!("list_all_tasks", normalized);
}

#[test]
fn test_list_with_label_filter_snapshot() {
    let project = TestProject::from_fixture("medium");
    let (stdout, _stderr, code) = run_lash(&["index"], project.path());
    assert_eq!(code, 0, "Indexing should succeed");

    let (stdout, _stderr, code) = run_lash(&["list", "--label", "backend"], project.path());
    assert_eq!(code, 0, "List with label should succeed");

    let normalized = normalize_output(&stdout, project.path());
    assert_snapshot!("list_backend_label", normalized);
}

#[test]
fn test_list_with_status_filter_snapshot() {
    let project = TestProject::from_fixture("medium");
    let (stdout, _stderr, code) = run_lash(&["index"], project.path());
    assert_eq!(code, 0, "Indexing should succeed");

    let (stdout, _stderr, code) = run_lash(&["list", "--status", "open"], project.path());
    assert_eq!(code, 0, "List with status should succeed");

    let normalized = normalize_output(&stdout, project.path());
    assert_snapshot!("list_open_status", normalized);
}

#[test]
fn test_show_file_snapshot() {
    let project = TestProject::from_fixture("small");
    let (stdout, _stderr, code) = run_lash(&["index"], project.path());
    assert_eq!(code, 0, "Indexing should succeed");

    let (stdout, _stderr, code) = run_lash(&["show", "tasks.md"], project.path());
    assert_eq!(code, 0, "Show file should succeed");

    let normalized = normalize_output(&stdout, project.path());
    assert_snapshot!("show_tasks_file", normalized);
}

#[test]
fn test_graph_output_snapshot() {
    let project = TestProject::from_fixture("medium");
    let (stdout, _stderr, code) = run_lash(&["index"], project.path());
    assert_eq!(code, 0, "Indexing should succeed");

    let (stdout, _stderr, code) = run_lash(&["graph"], project.path());
    assert_eq!(code, 0, "Graph should succeed");

    let normalized = normalize_output(&stdout, project.path());
    assert_snapshot!("graph_medium_project", normalized);
}

#[test]
fn test_error_invalid_file_snapshot() {
    let project = TestProject::builder()
        .with_index("test", "Test")
        .with_file(
            "invalid.md",
            "# Invalid\n@id: invalid\n## Tasks\n- [?] Invalid status\n",
        )
        .build();

    let (stdout, stderr, code) = run_lash(&["lint", "."], project.path());

    assert_ne!(code, 0, "Linting invalid file should fail");

    let normalized_stdout = normalize_output(&stdout, project.path());
    let normalized_stderr = normalize_output(&stderr, project.path());

    assert_snapshot!("error_invalid_status_stdout", normalized_stdout);
    assert_snapshot!("error_invalid_status_stderr", normalized_stderr);
}

#[test]
fn test_error_missing_id_snapshot() {
    let project = TestProject::builder()
        .with_index("test", "Test")
        .with_file("no-id.md", "# No ID\n## Tasks\n- [ ] Task\n")
        .build();

    let (stdout, stderr, code) = run_lash(&["lint", "."], project.path());

    // Files without @id get a synthesized ID, but produce orphan warning
    // Exit code should be 0 (warnings don't cause failure)
    assert_eq!(code, 0, "Linting file without ID should pass with warnings");

    let normalized_stdout = normalize_output(&stdout, project.path());
    let normalized_stderr = normalize_output(&stderr, project.path());

    assert_snapshot!("error_missing_id_stdout", normalized_stdout);
    assert_snapshot!("error_missing_id_stderr", normalized_stderr);
}

#[test]
fn test_json_output_format_snapshot() {
    let project = TestProject::from_fixture("small");
    let (stdout, _stderr, code) = run_lash(&["index"], project.path());
    assert_eq!(code, 0, "Indexing should succeed");

    let (stdout, _stderr, code) = run_lash(&["list", "--json"], project.path());
    assert_eq!(code, 0, "List JSON should succeed");

    // Parse JSON to ensure it's valid
    let _json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let normalized = normalize_output(&stdout, project.path());
    assert_snapshot!("list_json_output", normalized);
}

#[test]
fn test_search_query_snapshot() {
    let project = TestProject::from_fixture("medium");
    let (stdout, _stderr, code) = run_lash(&["index"], project.path());
    assert_eq!(code, 0, "Indexing should succeed");

    let (stdout, _stderr, code) = run_lash(&["search", "api"], project.path());
    assert_eq!(code, 0, "Search should succeed");

    let normalized = normalize_output(&stdout, project.path());
    assert_snapshot!("search_api_query", normalized);
}

#[test]
fn test_check_links_snapshot() {
    let project = TestProject::from_fixture("medium");
    let (stdout, _stderr, code) = run_lash(&["index"], project.path());
    assert_eq!(code, 0, "Indexing should succeed");

    let (stdout, stderr, code) = run_lash(&["check-links"], project.path());

    let normalized_stdout = normalize_output(&stdout, project.path());
    let normalized_stderr = normalize_output(&stderr, project.path());

    assert_snapshot!("check_links_stdout", normalized_stdout);
    assert_snapshot!("check_links_stderr", normalized_stderr);
    assert_snapshot!("check_links_exit_code", code.to_string());
}

#[test]
fn test_agent_prompt_snapshot() {
    let project = TestProject::from_fixture("small");
    let (stdout, _stderr, code) = run_lash(&["index"], project.path());
    assert_eq!(code, 0, "Indexing should succeed");

    let (stdout, _stderr, code) = run_lash(&["agent-prompt"], project.path());
    assert_eq!(code, 0, "Agent prompt should succeed");

    let normalized = normalize_output(&stdout, project.path());
    assert_snapshot!("agent_prompt_output", normalized);
}

#[test]
fn test_lint_unicode_filenames() {
    // Test that Unicode filenames are handled correctly
    let project = TestProject::builder()
        .with_index("unicode-test", "Unicode Test")
        .with_file(
            "日本語.md",
            "# 日本語\n@id: japanese\n@created: 2024-01-10\n## Tasks\n- [ ] テスト\n",
        )
        .build();

    let (stdout, stderr, code) = run_lash(&["lint", "."], project.path());

    let normalized_stdout = normalize_output(&stdout, project.path());
    let normalized_stderr = normalize_output(&stderr, project.path());

    assert_snapshot!("lint_unicode_filename_stdout", normalized_stdout);
    assert_snapshot!("lint_unicode_filename_stderr", normalized_stderr);
    assert_snapshot!("lint_unicode_filename_exit_code", code.to_string());
}

#[test]
fn test_very_long_list_performance() {
    // Ensure we can handle files with many tasks
    use std::fs;
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let project_path = temp.path();

    // Create index
    fs::write(
        project_path.join("lash.index.md"),
        "# Test\n@id: test\n## Tasks\n- [ ] Init\n",
    )
    .unwrap();

    // Create a file with 200 tasks
    let mut content = "# Many Tasks\n@id: many\n@created: 2024-01-10\n## Tasks\n\n".to_string();
    for i in 1..=200 {
        content.push_str(&format!("- [ ] Task {i}\n"));
    }
    fs::write(project_path.join("many.md"), content).unwrap();

    let (stdout, stderr, code) = run_lash(&["lint", "."], project_path);

    assert_eq!(code, 0, "Should handle 200 tasks without issue");

    let normalized_stdout = normalize_output(&stdout, project_path);
    let normalized_stderr = normalize_output(&stderr, project_path);

    assert_snapshot!("lint_200_tasks_stdout", normalized_stdout);
    assert_snapshot!("lint_200_tasks_stderr", normalized_stderr);
}
