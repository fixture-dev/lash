//! End-to-End CLI tests using assert_cmd
//!
//! These tests verify the actual `lash` binary behavior by invoking it as a user would.
//! They test all commands with various flags, error scenarios, and output formats.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// Allow deprecated cargo_bin for now - will migrate to cargo_bin_cmd! in future
#[allow(deprecated)]
fn create_lash_command() -> Command {
    Command::cargo_bin("lash").expect("Failed to find lash binary")
}

/// Helper to create a test project with an index file
fn create_test_project() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create index file
    let index_content = r#"# Test Project

@id: test-project
@labels: test
@status: in-progress

A test project for E2E testing.

## Structure

- `tasks/backend.md` - Backend tasks
- `tasks/frontend.md` - Frontend tasks

## Tasks

- [x] Set up project
- [ ] Complete testing
  - [x] Unit tests
  - [ ] E2E tests #testing
- [ ] Deploy to production #ops
"#;
    fs::write(root.join("lash.index.md"), index_content).expect("Failed to write index");

    // Create tasks directory
    let tasks_dir = root.join("tasks");
    fs::create_dir(&tasks_dir).expect("Failed to create tasks dir");

    // Create backend.md
    let backend_content = r#"# Backend Tasks

@id: backend
@owner: alice
@labels: backend, api
@status: in-progress
@created: 2025-01-15

Backend development tasks.

## Tasks

- [x] Set up database
  - [x] Design schema
  - [x] Add migrations
- [ ] Implement API endpoints #api
  - [x] Authentication endpoint
  - [ ] User CRUD endpoints #important
  - [ ] Task CRUD endpoints
- [ ] Add tests #testing
  - [x] Unit tests
  - [ ] Integration tests
"#;
    fs::write(tasks_dir.join("backend.md"), backend_content).expect("Failed to write backend.md");

    // Create frontend.md
    let frontend_content = r#"# Frontend Tasks

@id: frontend
@owner: bob
@labels: frontend, ui
@status: open
@created: 2025-01-16
@depends-on: tasks/backend.md#task:api-endpoints

Frontend development tasks.

## Tasks

- [ ] Design UI components #design
  - [ ] Login form
  - [ ] Dashboard
  - [ ] Task list view
- [ ] Implement components #implementation
  - [ ] Login form component
  - [ ] Dashboard component
- [ ] Add tests #testing
  - [ ] Component tests
  - [ ] E2E tests
"#;
    fs::write(tasks_dir.join("frontend.md"), frontend_content)
        .expect("Failed to write frontend.md");

    temp_dir
}

/// Helper to create an invalid project (bad task file)
fn create_invalid_project() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let index_content = r#"# Invalid Project

@id: invalid-project

## Tasks

- [x] Valid task
- [?] Invalid checkbox status
"#;
    fs::write(root.join("lash.index.md"), index_content).expect("Failed to write index");

    temp_dir
}

#[test]
fn test_help_command() {
    let mut cmd = create_lash_command();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Lash is an ultra-fast"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("lint"))
        .stdout(predicate::str::contains("format"));
}

#[test]
fn test_version_command() {
    let mut cmd = create_lash_command();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("lash"));
}

// --- LINT COMMAND TESTS ---

#[test]
fn test_lint_valid_project() {
    let temp = create_test_project();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("lint")
        .assert()
        .success()
        .stdout(predicate::str::contains("passed").or(predicate::str::contains("✓")));
}

#[test]
fn test_lint_invalid_project() {
    let temp = create_invalid_project();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("lint")
        .assert()
        .code(2) // Lint error exit code
        .stderr(predicate::str::contains("error").or(predicate::str::contains("invalid")));
}

#[test]
fn test_lint_specific_file() {
    let temp = create_test_project();
    let backend_file = temp.path().join("tasks").join("backend.md");

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("lint")
        .arg(&backend_file)
        .assert()
        .success();
}

#[test]
fn test_lint_json_output() {
    let temp = create_test_project();

    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Verify output is valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _: serde_json::Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
}

#[test]
fn test_lint_non_project_dir() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("lint")
        .assert()
        .failure()
        .stderr(predicate::str::contains("index").or(predicate::str::contains("project")));
}

// --- INDEX COMMAND TESTS ---

#[test]
fn test_index_command() {
    let temp = create_test_project();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Indexed").or(predicate::str::contains("index")));

    // Verify database was created
    assert!(temp.path().join(".lash").join("lash.db").exists());
}

#[test]
fn test_index_json_output() {
    let temp = create_test_project();

    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Verify output is valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Should have indexed some files
    assert!(json.get("files_indexed").is_some() || json.get("indexed").is_some());
}

// --- CHECK-INDEX COMMAND TESTS ---

#[test]
fn test_check_index_after_indexing() {
    let temp = create_test_project();

    // First, index the project
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Then check index
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("check-index")
        .assert()
        .success()
        .stdout(predicate::str::contains("sync").or(predicate::str::contains("✓")));
}

#[test]
fn test_check_index_without_indexing() {
    let temp = create_test_project();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("check-index")
        .assert()
        .failure()
        .stderr(predicate::str::contains("index").or(predicate::str::contains("database")));
}

// --- LIST COMMAND TESTS ---

#[test]
fn test_list_all_tasks() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // List tasks
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("backend").or(predicate::str::contains("frontend")));
}

#[test]
fn test_list_with_label_filter() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // List tasks with #testing label
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("list")
        .arg("--label")
        .arg("testing")
        .assert()
        .success()
        .stdout(predicate::str::contains("testing"));
}

#[test]
fn test_list_with_status_filter() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // List only open tasks
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("list")
        .arg("--status")
        .arg("open")
        .assert()
        .success();
}

#[test]
fn test_list_json_output() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // List with JSON output
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("list")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Verify output is valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _: serde_json::Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
}

// --- SEARCH COMMAND TESTS ---

#[test]
fn test_search_tasks() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Search for "API"
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("search")
        .arg("API")
        .assert()
        .success()
        .stdout(predicate::str::contains("API").or(predicate::str::contains("api")));
}

#[test]
fn test_search_no_results() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Search for something that doesn't exist
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("search")
        .arg("nonexistent-query-xyz")
        .assert()
        .success()
        .stdout(predicate::str::contains("No results").or(predicate::str::is_empty()));
}

// --- SHOW COMMAND TESTS ---

#[test]
fn test_show_file() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Show backend.md
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("tasks/backend.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("Backend"));
}

#[test]
fn test_show_nonexistent_file() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Show nonexistent file
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("nonexistent.md")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("error")));
}

// --- GRAPH COMMAND TESTS ---

#[test]
fn test_graph_output() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Generate graph
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("graph")
        .assert()
        .success()
        .stdout(predicate::str::contains("digraph").or(predicate::str::contains("graph")));
}

#[test]
fn test_graph_json_output() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Generate graph with JSON output
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("graph")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Verify output is valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _: serde_json::Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
}

// --- CHECK-LINKS COMMAND TESTS ---

#[test]
fn test_check_links_valid() {
    let temp = create_test_project();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("check-links")
        .assert()
        .success();
}

#[test]
fn test_check_links_with_broken_link() {
    let temp = tempfile::tempdir().unwrap();

    // Create file with broken link
    let content = r#"# Test

@id: test
@depends-on: nonexistent.md#task:missing

## Tasks

- [ ] Task 1
"#;
    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("check-links")
        .assert()
        .failure()
        .stdout(predicate::str::contains("broken").or(predicate::str::contains("error")));
}

// --- AGENT-PROMPT COMMAND TESTS ---

#[test]
fn test_agent_prompt_generation() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Generate agent prompt
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("agent-prompt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task").or(predicate::str::contains("#")));
}

#[test]
fn test_agent_prompt_json_output() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Generate agent prompt with JSON
    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("agent-prompt")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Verify output is valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _: serde_json::Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
}

// --- FORMAT COMMAND TESTS ---

#[test]
fn test_format_command() {
    let temp = tempfile::tempdir().unwrap();

    // Create file with inconsistent formatting
    let content = r#"# Test


@id:   test
@labels:backend,  api


## Tasks

-  [ ]   Task 1
- [x]Task 2
"#;
    let file_path = temp.path().join("lash.index.md");
    fs::write(&file_path, content).unwrap();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("format")
        .arg(&file_path)
        .assert()
        .success();

    // Read formatted content
    let formatted = fs::read_to_string(&file_path).unwrap();

    // Should have normalized spacing
    assert!(formatted.contains("@id: test"));
    assert!(formatted.contains("@labels: backend, api"));
}

#[test]
fn test_format_check_mode() {
    let temp = tempfile::tempdir().unwrap();

    // Create file with inconsistent formatting
    let content = r#"# Test

@id:   test

## Tasks

-  [ ]   Task 1
"#;
    let file_path = temp.path().join("lash.index.md");
    fs::write(&file_path, content).unwrap();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("format")
        .arg("--check")
        .arg(&file_path)
        .assert()
        .failure(); // Should fail because file needs formatting

    // File should not be modified
    let unchanged = fs::read_to_string(&file_path).unwrap();
    assert_eq!(unchanged, content);
}

// --- GLOBAL FLAGS TESTS ---

#[test]
fn test_verbose_flag() {
    let temp = create_test_project();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("-vv")
        .arg("lint")
        .assert()
        .success();
    // With verbose, we should see more output (though exact format may vary)
}

#[test]
fn test_quiet_flag() {
    let temp = create_test_project();

    let mut cmd = create_lash_command();
    let output = cmd
        .arg("--root")
        .arg(temp.path())
        .arg("--quiet")
        .arg("lint")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    // Quiet mode should produce minimal or no output
}

// --- ERROR EXIT CODES TESTS ---

#[test]
fn test_exit_code_lint_error() {
    let temp = create_invalid_project();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("lint")
        .assert()
        .code(2); // Lint error code
}

#[test]
fn test_exit_code_not_found() {
    let temp = create_test_project();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("nonexistent.md")
        .assert()
        .code(5); // Resource not found code
}

// --- WORKFLOW TESTS ---

#[test]
fn test_complete_workflow() {
    let temp = create_test_project();

    // 1. Lint the project
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("lint")
        .assert()
        .success();

    // 2. Index the project
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // 3. Check index is in sync
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("check-index")
        .assert()
        .success();

    // 4. List tasks
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("list")
        .assert()
        .success();

    // 5. Search for tasks
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("search")
        .arg("backend")
        .assert()
        .success();

    // 6. Show a file
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("show")
        .arg("tasks/backend.md")
        .assert()
        .success();

    // 7. Generate graph
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("graph")
        .assert()
        .success();

    // 8. Check links
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("check-links")
        .assert()
        .success();
}

#[test]
fn test_modify_and_reindex_workflow() {
    let temp = create_test_project();

    // Index the project
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Modify a file
    let backend_file = temp.path().join("tasks").join("backend.md");
    let mut content = fs::read_to_string(&backend_file).unwrap();
    content.push_str("\n- [ ] New task added #new\n");
    fs::write(&backend_file, content).unwrap();

    // Re-index (should be incremental)
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Search for the new task
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("search")
        .arg("New task added")
        .assert()
        .success()
        .stdout(predicate::str::contains("New task"));
}
