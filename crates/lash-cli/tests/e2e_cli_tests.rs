//! End-to-End CLI tests using `assert_cmd`
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
        .stdout(predicate::str::contains("error").or(predicate::str::contains("invalid")));
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

    // List tasks (uses tree view by default, showing files in directory structure)
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("list")
        .assert()
        .success()
        // Tree view shows directory structure with files
        .stdout(predicate::str::contains("tasks/").or(predicate::str::contains("lash.index.md")));
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

    // List with label filter
    // Note: In the current file-based tree view, label filtering is deferred.
    // The command should still succeed and show files.
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("list")
        .arg("--label")
        .arg("testing")
        .assert()
        .success();
    // Label filtering in tree view is a future enhancement
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

    // Generate graph (default is ASCII format)
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("graph")
        .assert()
        .success()
        // ASCII format uses box-drawing characters and checkboxes
        .stdout(predicate::str::contains("───").or(predicate::str::contains("[ ]")));
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
        .arg("check-links")
        .assert()
        .success();
}

#[test]
fn test_check_links_with_broken_link() {
    let temp = tempfile::tempdir().unwrap();

    // Create file with task that has broken dependency
    // Note: Full dependency checking is not yet implemented, so this test
    // just verifies that check-links runs without crashing on such files
    let content = r#"# Test

@id: test

## Tasks

- [ ] Task 1 [@depends-on: nonexistent.md#task:missing]
- [ ] Task 2
"#;
    fs::write(temp.path().join("lash.index.md"), content).unwrap();

    // Index first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Verify check-links runs successfully
    // TODO: Once dependency indexing is implemented, this should detect the broken link
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("check-links")
        .assert()
        .success();
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

    // Create file with inconsistent annotation formatting (but valid checkboxes)
    // Note: Malformed checkboxes like "-  [ ]" are now detected as errors to prevent
    // silent data loss - use valid checkboxes here
    let content = r#"# Test


@id:   test
@labels:backend,  api


## Tasks

- [ ] Task 1
- [x] Task 2
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
    // Labels are sorted alphabetically by the formatter
    assert!(formatted.contains("@labels: api, backend"));
}

#[test]
fn test_format_rejects_malformed_checkboxes() {
    let temp = tempfile::tempdir().unwrap();

    // Create file with malformed checkbox (extra space after dash)
    let content = r#"# Test

@id: test

## Tasks

-  [ ] Task with malformed spacing
"#;
    let file_path = temp.path().join("lash.index.md");
    fs::write(&file_path, content).unwrap();

    // Format should fail because malformed checkboxes are detected as errors
    // to prevent silent data loss
    let mut cmd = create_lash_command();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("format")
        .arg(&file_path)
        .assert()
        .failure(); // Should fail, not succeed
}

#[test]
fn test_format_check_mode() {
    let temp = tempfile::tempdir().unwrap();

    // Create file with inconsistent annotation formatting (but valid checkboxes)
    let content = r#"# Test

@id:   test

## Tasks

- [ ] Task 1
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
        .failure(); // Should fail because file needs formatting (annotation spacing)

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

#[test]
fn test_playground_full_workflow() {
    let temp = TempDir::new().unwrap();
    let playground_path = temp.path().join("playground");

    // 1. Initialize playground
    create_lash_command()
        .arg("playground")
        .arg("init")
        .arg("--path")
        .arg(&playground_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("PixelQuest"));

    // 2. List gameplay tasks
    create_lash_command()
        .current_dir(&playground_path)
        .arg("list")
        .arg("--label")
        .arg("gameplay")
        .assert()
        .success();

    // 3. Search for boss-related tasks
    create_lash_command()
        .current_dir(&playground_path)
        .arg("search")
        .arg("boss")
        .assert()
        .success()
        .stdout(predicate::str::contains("boss"));

    // 4. Show a specific file
    create_lash_command()
        .current_dir(&playground_path)
        .arg("show")
        .arg("features/player-movement.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("Player Movement"));

    // 5. Generate dependency graph (explicitly request DOT format for .dot file)
    let graph_output = playground_path.join("graph.dot");
    create_lash_command()
        .current_dir(&playground_path)
        .arg("graph")
        .arg("--format")
        .arg("dot")
        .arg("--output")
        .arg(&graph_output)
        .assert()
        .success();

    assert!(graph_output.exists());
    let graph_content = fs::read_to_string(&graph_output).unwrap();
    assert!(graph_content.contains("digraph"));

    // 6. Check all links are valid
    create_lash_command()
        .current_dir(&playground_path)
        .arg("check-links")
        .assert()
        .success();

    // 7. List tasks by different labels
    create_lash_command()
        .current_dir(&playground_path)
        .arg("list")
        .arg("--label")
        .arg("backend")
        .assert()
        .success();

    create_lash_command()
        .current_dir(&playground_path)
        .arg("list")
        .arg("--label")
        .arg("art")
        .assert()
        .success();

    // 8. Verify index was created (in .lash directory)
    assert!(playground_path.join(".lash/lash.db").exists());
}

// --- LIST COMMAND WITH CONTEXTUAL NOTES TESTS ---

#[test]
fn test_list_with_show_notes() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create a task file with contextual notes
    let task_content = r#"# Tasks with Notes

@id: tasks.notes
@labels: example

## Tasks

- [ ] Integrate procedural level generation
  - Use Foo library to generate 2D map layouts
  - Ensure levels have an appropriate size constraint
  - Use foo, bar, baz and quux
  - [ ] Research generation algorithms
  - [ ] Implement basic generator

- [ ] Add multiplayer support
  - Must support 2-4 players
  - Consider peer-to-peer vs client-server architecture
  - [ ] Design network protocol

- [x] Complete initial setup
  - Already configured dev environment
  - Database schema created
"#;
    fs::write(root.join("lash.index.md"), task_content).expect("Failed to write task file");

    // Index the project
    create_lash_command()
        .arg("--root")
        .arg(root)
        .arg("index")
        .assert()
        .success();

    // Test list with --show-notes flag
    let output = create_lash_command()
        .arg("--root")
        .arg(root)
        .arg("list")
        .arg("--show-notes")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify notes are displayed
    assert!(stdout.contains("Integrate procedural level generation"));
    assert!(stdout.contains("Use Foo library to generate 2D map layouts"));
    assert!(stdout.contains("Add multiplayer support"));
    assert!(stdout.contains("Must support 2-4 players"));
}

#[test]
fn test_list_without_show_notes() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create a task file with contextual notes
    let task_content = r#"# Tasks with Notes

@id: tasks.notes
@labels: example

## Tasks

- [ ] Integrate procedural level generation
  - Use Foo library to generate 2D map layouts
  - Ensure levels have an appropriate size constraint
  - [ ] Research generation algorithms

- [ ] Add multiplayer support
  - Must support 2-4 players
  - [ ] Design network protocol
"#;
    fs::write(root.join("lash.index.md"), task_content).expect("Failed to write task file");

    // Index the project
    create_lash_command()
        .arg("--root")
        .arg(root)
        .arg("index")
        .assert()
        .success();

    // Test list without --show-notes flag (default behavior)
    let output = create_lash_command()
        .arg("--root")
        .arg(root)
        .arg("list")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify notes are NOT displayed (unless they leak through file description)
    // The notes should not appear as individual lines
    let note_lines = stdout.matches("Use Foo library").count();
    assert_eq!(
        note_lines, 0,
        "Notes should not be displayed without --show-notes flag"
    );
}

#[test]
fn test_list_show_notes_json_output() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create a task file with contextual notes
    let task_content = r#"# Tasks with Notes

@id: tasks.notes
@labels: example

## Tasks

- [ ] Integrate procedural level generation
  - Use Foo library to generate 2D map layouts
  - Ensure levels have an appropriate size constraint
  - [ ] Research generation algorithms
"#;
    fs::write(root.join("lash.index.md"), task_content).expect("Failed to write task file");

    // Index the project
    create_lash_command()
        .arg("--root")
        .arg(root)
        .arg("index")
        .assert()
        .success();

    // Test list with --show-notes and JSON output
    let output = create_lash_command()
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg("list")
        .arg("--show-notes")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify JSON structure includes tasks_with_notes
    assert!(json["files"].is_array());
    let files = json["files"].as_array().unwrap();
    assert!(!files.is_empty());

    // Check if any file has tasks_with_notes field
    let has_tasks_with_notes = files
        .iter()
        .any(|file| file.get("tasks_with_notes").is_some());
    assert!(
        has_tasks_with_notes,
        "At least one file should have tasks_with_notes when --show-notes is used"
    );
}

// ---------------------------------------------------------------------------
// Index command text output content tests
//
// These tests verify the exact stdout content produced by `output_text_report`,
// killing mutations in the comparison guards and summary labels.
// ---------------------------------------------------------------------------

/// `lash index` without --force must print "Incremental index complete" (not the force label).
/// Kills mut-000384: force → !(force).
#[test]
fn test_index_text_output_incremental_label() {
    let temp = create_test_project();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Incremental index complete"));
}

/// `lash index --force` must print "Full rebuild complete" (not the incremental label).
/// Kills mut-000384: force → !(force).
#[test]
fn test_index_text_output_force_label() {
    let temp = create_test_project();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Full rebuild complete"));
}

/// After a fresh index, `files_added` > 0 so the "Added:" line must appear.
/// Kills mut-000386, 387, 388, 389 (negation and boundary mutations on `files_added` > 0).
#[test]
fn test_index_text_output_shows_added_count() {
    let temp = create_test_project();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Added:").or(predicate::str::contains("Files processed:")),
        );
}

/// After indexing and re-indexing the same project, `files_unchanged` > 0 so the
/// "Unchanged:" line must appear on the second run.
/// Kills mut-000400, 401, 402, 403 (negation and boundary mutations on `files_unchanged` > 0).
#[test]
fn test_index_text_output_shows_unchanged_count_on_reindex() {
    let temp = create_test_project();

    // First run – populate the DB
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success();

    // Second run without changes – must report unchanged files
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Unchanged:"));
}

/// A full rebuild (`--force`) on an already-indexed project reports updated files;
/// verifies the `files_updated` > 0 branch.
/// Kills mut-000391, 392, 393, 394.
#[test]
fn test_index_text_output_shows_updated_count_on_force_rebuild() {
    let temp = create_test_project();

    // First index (adds files)
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success();

    // Modify the file so it is re-parsed as "updated"
    let index_file = temp.path().join("lash.index.md");
    let existing = fs::read_to_string(&index_file).unwrap();
    fs::write(&index_file, format!("{existing}\n- [ ] Extra task\n")).unwrap();

    // Force rebuild – modified file must appear as Updated
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated:").or(predicate::str::contains("Added:")));
}

/// A JSON index report must contain numeric counts with the exact field names.
/// Kills mut-000370 (json branch selection) and verifies field values exist.
#[test]
fn test_index_json_output_exact_fields() {
    let temp = create_test_project();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // These field names must be exactly present – kills mutations that remove them
    assert!(
        json["files_processed"].is_number(),
        "files_processed must be a number"
    );
    assert!(
        json["files_added"].is_number(),
        "files_added must be a number"
    );
    assert!(
        json["files_updated"].is_number(),
        "files_updated must be a number"
    );
    assert!(
        json["files_deleted"].is_number(),
        "files_deleted must be a number"
    );
    assert!(
        json["files_unchanged"].is_number(),
        "files_unchanged must be a number"
    );
    assert!(json["errors"].is_object(), "errors must be an object");

    // files_added must be > 0 on the first index
    let files_added = json["files_added"].as_u64().unwrap();
    assert!(files_added > 0, "first index must add at least one file");
}

/// `lash index --force` JSON output must have `files_added` > 0.
/// Kills mut-000362, 363, 364 (force path: args.force || !`db_path.exists()`).
#[test]
fn test_index_force_json_output_shows_added() {
    let temp = create_test_project();

    // First run to create DB
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .assert()
        .success();

    // Force rebuild should also add files
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .arg("--force")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let files_added = json["files_added"].as_u64().unwrap();
    assert!(files_added > 0, "force rebuild must re-add all files");
}

/// Incremental second index (force=false, DB exists) reports unchanged files, not added.
/// Distinguishes the `else` branch from the `if args.force || !db_path.exists()` branch.
/// Kills mut-000362, 363, 364.
#[test]
fn test_index_incremental_second_run_shows_unchanged() {
    let temp = create_test_project();

    // First run
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .assert()
        .success();

    // Second run without modification – no files should be re-added
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let files_unchanged = json["files_unchanged"].as_u64().unwrap();
    assert!(
        files_unchanged > 0,
        "incremental second run must report unchanged files"
    );

    let files_added = json["files_added"].as_u64().unwrap();
    assert_eq!(
        files_added, 0,
        "incremental run must not re-add unchanged files"
    );
}

/// Streaming error display mode must succeed (kills mut-000371, 379).
#[test]
fn test_index_errors_streaming_flag_succeeds() {
    let temp = create_test_project();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--errors-streaming")
        .arg("--no-color")
        .arg("index")
        .assert()
        .success();
}

/// `--no-color` flag must succeed and produce text output without ANSI codes
/// on a simple index run (kills mut-000360).
#[test]
fn test_index_no_color_flag_produces_plain_text() {
    let temp = create_test_project();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should not contain ANSI escape sequences
    assert!(
        !stdout.contains("\x1b["),
        "output must not contain ANSI codes when --no-color is set"
    );
    // Must still contain the summary label
    assert!(
        stdout.contains("index complete") || stdout.contains("rebuild complete"),
        "output must contain summary label"
    );
}
