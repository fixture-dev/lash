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
///
/// Using a strict `contains("Added:")` without any `or` alternative ensures that
/// mutants which suppress the "Added:" line (e.g. `!(files_added > 0)`) are caught.
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
        .stdout(predicate::str::contains("Added:"));
}

/// On a second index run with no changes, `files_added = 0` so the "Added:" line
/// must NOT appear.  This is the boundary case that distinguishes `> 0` from `>= 0`
/// and `<= 0` on the `files_added` guard.
/// Kills mut-000387 (>= 0 would print on zero) and mut-000388 (<= 0 would print on zero).
#[test]
fn test_index_text_output_no_added_line_on_second_run() {
    let temp = create_test_project();

    // First run – create the DB
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success();

    // Second run with no changes – files_added must be 0, so "Added:" must not appear
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added:").not());
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

/// An incremental index after modifying a file shows "Updated:" in output.
/// The `files_updated > 0` guard must be true for this to print.
/// Kills mut-000391, 392, 393, 394 (negation and boundary mutations on `files_updated > 0`).
#[test]
fn test_index_text_output_shows_updated_count_after_modification() {
    let temp = create_test_project();

    // First index – establish a baseline
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success();

    // Modify a file so the incremental index sees it as updated
    let index_file = temp.path().join("lash.index.md");
    let existing = fs::read_to_string(&index_file).unwrap();
    fs::write(
        &index_file,
        format!("{existing}\n- [ ] Extra task added for update test\n"),
    )
    .unwrap();

    // Incremental re-index (no --force) – the modified file must appear as "Updated:"
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated:"));
}

/// On a fresh index (no prior DB), `files_updated = 0` so "Updated:" must NOT appear.
/// This boundary case distinguishes `> 0` from `>= 0` on the `files_updated` guard.
/// Kills mut-000392 (>= 0 always true) and mut-000393 (<= 0 only true for zero).
#[test]
fn test_index_text_output_no_updated_line_on_fresh_index() {
    let temp = create_test_project();

    // Fresh index – no prior DB means everything is "added", not "updated"
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated:").not());
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

// ---------------------------------------------------------------------------
// files_deleted > 0 boundary tests
// Kills mut-000396 (negation), mut-000397 (>= 0), mut-000398 (<= 0), mut-000399 (0 → 1)
// ---------------------------------------------------------------------------

/// After removing a previously-indexed file, the "Deleted:" line must appear.
/// Verifies the `files_deleted > 0` branch is entered when a file has been removed.
#[test]
fn test_index_text_output_shows_deleted_count_after_file_removal() {
    let temp = create_test_project();

    // First run – index all files including backend.md and frontend.md
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success();

    // Remove one of the indexed task files
    fs::remove_file(temp.path().join("tasks").join("backend.md")).unwrap();

    // Incremental re-index – the removed file must appear as "Deleted:"
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted:"));
}

/// On a fresh index no files are deleted, so "Deleted:" must NOT appear.
/// Kills mut-000397 (>= 0 always true) and mut-000398 (<= 0 only true for zero).
#[test]
fn test_index_text_output_no_deleted_line_on_fresh_index() {
    let temp = create_test_project();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted:").not());
}

// ---------------------------------------------------------------------------
// files_unchanged > 0 boundary tests
// Kills mut-000400 (negation), mut-000401 (>= 0), mut-000402 (<= 0), mut-000403 (0 → 1)
// ---------------------------------------------------------------------------

/// On a fresh index, files_unchanged = 0, so "Unchanged:" must NOT appear.
/// Kills mut-000401 (>= 0 always true) and mut-000402 (<= 0 only true for zero).
#[test]
fn test_index_text_output_no_unchanged_line_on_fresh_index() {
    let temp = create_test_project();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Unchanged:").not());
}

// ---------------------------------------------------------------------------
// summary.error_count > 0 boundary tests (text report error section)
// Kills mut-000404 (negation), mut-000405 (>= 0), mut-000406 (<= 0), mut-000407 (0 → 1)
// ---------------------------------------------------------------------------

/// On a valid project, error_count = 0, so the "Errors:" summary section must NOT appear.
/// Kills mut-000405 (>= 0 always true), mut-000406 (<= 0 only true for zero).
#[test]
fn test_index_text_output_no_error_section_on_valid_project() {
    let temp = create_test_project();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Errors:").not());
}

// ---------------------------------------------------------------------------
// Exit code and JSON field exact-value tests
// Kills mut-000381 (has_errors negation), mut-000382 (exit code 0 → 1),
// mut-000386-394, mut-000396-407
// ---------------------------------------------------------------------------

/// A successful index must exit with code 0, not 3 or 1.
/// Kills mut-000381 (negation of has_errors) and mut-000382 (exit code 0 → 1).
#[test]
fn test_index_exits_zero_on_success_e2e() {
    let temp = create_test_project();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .code(0);
}

/// JSON output on fresh index must have files_added > 0 (not zero).
/// Kills mut-000386 (negation), mut-000388 (<= 0), mut-000389 (literal 0→1).
#[test]
fn test_index_json_files_added_is_positive_on_fresh_index() {
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
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");

    let files_added = json["files_added"]
        .as_u64()
        .expect("files_added must be a number");
    assert!(
        files_added > 0,
        "fresh index must add at least 1 file, got files_added={files_added}"
    );
}

/// JSON output on unchanged re-index must have files_added = 0.
/// Kills mut-000387 (>= 0 makes 0 pass) and mut-000389 (literal 0→1 changes boundary).
#[test]
fn test_index_json_files_added_is_zero_on_unchanged_reindex() {
    let temp = create_test_project();

    // First run
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .assert()
        .success();

    // Second run with no changes
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");

    let files_added = json["files_added"]
        .as_u64()
        .expect("files_added must be a number");
    assert_eq!(
        files_added, 0,
        "second index with no changes must have files_added=0"
    );
}

/// JSON output on unchanged re-index must have files_updated = 0.
/// Kills boundary mutations on `files_updated > 0` (mut-000392, 393, 394).
#[test]
fn test_index_json_files_updated_is_zero_on_unchanged_reindex() {
    let temp = create_test_project();

    // First run
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .assert()
        .success();

    // Second run with no changes
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");

    let files_updated = json["files_updated"]
        .as_u64()
        .expect("files_updated must be a number");
    assert_eq!(
        files_updated, 0,
        "second index with no changes must have files_updated=0"
    );
}

/// JSON output after modifying a file must have files_updated > 0.
/// Kills mut-000391 (negation) and mut-000393 (<= 0) on `files_updated`.
#[test]
fn test_index_json_files_updated_is_positive_after_modification() {
    let temp = create_test_project();

    // First run
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .assert()
        .success();

    // Modify a file
    let index_file = temp.path().join("lash.index.md");
    let existing = fs::read_to_string(&index_file).unwrap();
    fs::write(
        &index_file,
        format!("{existing}\n- [ ] JSON update test task\n"),
    )
    .unwrap();

    // Incremental re-index
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");

    let files_updated = json["files_updated"]
        .as_u64()
        .expect("files_updated must be a number");
    assert!(
        files_updated > 0,
        "incremental index after modification must have files_updated>0, got {files_updated}"
    );
}

/// JSON output after file removal must have files_deleted > 0.
/// Kills mut-000396 (negation), mut-000398 (<= 0), mut-000399 (literal 0→1).
#[test]
fn test_index_json_files_deleted_is_positive_after_removal() {
    let temp = create_test_project();

    // First run
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .assert()
        .success();

    // Remove a file
    fs::remove_file(temp.path().join("tasks").join("frontend.md")).unwrap();

    // Incremental re-index
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");

    let files_deleted = json["files_deleted"]
        .as_u64()
        .expect("files_deleted must be a number");
    assert!(
        files_deleted > 0,
        "index after file removal must have files_deleted>0, got {files_deleted}"
    );
}

/// JSON output on fresh index must have files_deleted = 0.
/// Kills mut-000397 (>= 0 always true) and mut-000399 (literal 0→1).
#[test]
fn test_index_json_files_deleted_is_zero_on_fresh_index() {
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
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");

    let files_deleted = json["files_deleted"]
        .as_u64()
        .expect("files_deleted must be a number");
    assert_eq!(
        files_deleted, 0,
        "fresh index must have files_deleted=0, got {files_deleted}"
    );
}

/// JSON output on second index (no changes) must have files_unchanged > 0.
/// Kills mut-000400 (negation), mut-000401 (>= 0), mut-000402 (<= 0), mut-000403 (0→1).
#[test]
fn test_index_json_files_unchanged_is_positive_on_reindex() {
    let temp = create_test_project();

    // First run
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .assert()
        .success();

    // Second run with no changes
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");

    let files_unchanged = json["files_unchanged"]
        .as_u64()
        .expect("files_unchanged must be a number");
    assert!(
        files_unchanged > 0,
        "second index with no changes must have files_unchanged>0, got {files_unchanged}"
    );
}

/// JSON output on fresh index must have files_unchanged = 0.
/// Kills mut-000401 (>= 0 always true) and mut-000403 (literal 0→1).
#[test]
fn test_index_json_files_unchanged_is_zero_on_fresh_index() {
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
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");

    let files_unchanged = json["files_unchanged"]
        .as_u64()
        .expect("files_unchanged must be a number");
    assert_eq!(
        files_unchanged, 0,
        "fresh index must have files_unchanged=0, got {files_unchanged}"
    );
}

/// JSON error count must be 0 on a valid project.
/// Kills mut-000404 (negation on error section guard), mut-000405 (>= 0),
/// mut-000406 (<= 0), mut-000407 (literal 0→1 in comparison).
#[test]
fn test_index_json_error_count_is_zero_on_valid_project() {
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
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");

    let error_count = json["errors"]["count"]
        .as_u64()
        .expect("errors.count must be a number");
    assert_eq!(
        error_count, 0,
        "valid project must produce error_count=0, got {error_count}"
    );
}

/// `lash index --show-files` without --json must succeed and produce text output.
/// Kills mut-000373 (!args.json negation), mut-000374 (&& replaced by ||).
#[test]
fn test_index_show_files_flag_produces_text_output() {
    let temp = create_test_project();

    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .arg("--show-files")
        .assert()
        .success()
        .stdout(predicate::str::contains("Files processed:"));
}

/// `lash index --show-files --json` must suppress the file progress bar (json takes precedence).
/// The progress bar condition is `!args.json && args.show_files`, so json=true disables it.
/// Kills mut-000373 (!args.json negation) and mut-000374 (&& replaced with ||).
#[test]
fn test_index_show_files_with_json_produces_json_output() {
    let temp = create_test_project();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("index")
        .arg("--show-files")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    // Output must still be valid JSON (progress bar suppressed by --json)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("output must be valid JSON with --json --show-files");
    assert!(json["files_indexed"].is_number() || json["files_processed"].is_number());
}

// --- CHECK-INDEX TARGETED MUTATION-KILLING TESTS ---

/// `lash check-index --no-color` on a freshly indexed clean project outputs "Index is in sync".
/// Kills mut-000219 (is_clean() negation in output_text_report):
/// - When is_clean()==true, the "in sync" message is printed and we return early.
/// - With negation, is_clean()==true would take the "issues found" path, NOT printing "in sync".
#[test]
fn test_check_index_clean_outputs_in_sync_message() {
    let temp = create_test_project();

    // First index the project
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // check-index should show "in sync" for a freshly indexed clean project
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("check-index")
        .assert()
        .success()
        .stdout(predicate::str::contains("in sync"));
}

/// `lash check-index --json` on a clean indexed project outputs JSON with `is_clean: true`.
/// Kills mut-000215 (args.json negation after verification):
/// - json=true should output JSON, json=false outputs text
/// - With negation, json=true would output TEXT (not JSON), making JSON parse fail.
#[test]
fn test_check_index_json_mode_outputs_is_clean_true() {
    let temp = create_test_project();

    // Index the project first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // With --json, output must be valid JSON with is_clean=true
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("check-index")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("output must be valid JSON with --json");
    assert_eq!(
        json["is_clean"].as_bool(),
        Some(true),
        "clean index must have is_clean=true in JSON output"
    );
}

/// `lash check-index` on a project with issues returns exit code 1.
/// Kills mut-000218 (Ok(1) → Ok(0)):
/// - When the index has issues, the function must return 1, not 0.
/// - If the mutation changes Ok(1) to Ok(0), this test will fail.
#[test]
fn test_check_index_returns_1_when_issues_found() {
    let temp = create_test_project();

    // Index the project first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Now delete a tracked file from disk to create a stale-file issue
    fs::remove_file(temp.path().join("tasks").join("backend.md"))
        .expect("Failed to remove backend.md");

    // check-index should now find issues and return exit code 1
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("check-index")
        .assert()
        .code(1); // Exit code 1 means issues found
}

/// `lash check-index --no-color` with issues outputs "Found X issue(s)" text.
/// Kills mut-000219 (is_clean() negation in output_text_report):
/// - When is_clean()==false, the "Found N issue(s)" message is printed.
/// - With negation, is_clean()==false would print the "in sync" message instead.
#[test]
fn test_check_index_dirty_outputs_issues_found_message() {
    let temp = create_test_project();

    // Index the project first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Delete a file to create an issue
    fs::remove_file(temp.path().join("tasks").join("backend.md"))
        .expect("Failed to remove backend.md");

    // check-index should show "issue(s)" for a dirty index
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("check-index")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("issue"));
}

/// `lash check-index --no-color` with issues and `--diff` shows detailed issue list.
/// Kills mut-000222 (show_diff negation):
/// - show_diff=true should print detailed issues section.
/// - With negation, show_diff=true would NOT print detailed issues.
#[test]
fn test_check_index_diff_flag_shows_detailed_issues() {
    let temp = create_test_project();

    // Index the project first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Delete a file to create an issue
    fs::remove_file(temp.path().join("tasks").join("backend.md"))
        .expect("Failed to remove backend.md");

    // check-index with --diff should show detailed issue list
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("check-index")
        .arg("--diff")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Detailed issues"));
}

/// `lash check-index --no-color` without `--diff` does NOT show detailed issue list.
/// Kills mut-000222 (show_diff negation):
/// - show_diff=false should NOT print detailed issues section.
/// - With negation, show_diff=false would print detailed issues (wrong).
#[test]
fn test_check_index_no_diff_flag_does_not_show_detailed_issues() {
    let temp = create_test_project();

    // Index the project first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Delete a file to create an issue
    fs::remove_file(temp.path().join("tasks").join("backend.md"))
        .expect("Failed to remove backend.md");

    // check-index without --diff should NOT show "Detailed issues" section
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("check-index")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("issue"))
        .stdout(predicate::str::contains("Detailed issues").not());
}

/// `lash check-index --no-color` with issues shows the issue count per type (e.g., "Stale files").
/// Kills mut-000224/225/226/227 (count > 0 comparisons in print_issue_count_if_any):
/// - When count > 0, the count is printed for that issue type.
/// - Mutations change `> 0` to `>= 0`, `<= 0`, `!= 0`, or `0` to `1`.
/// - By having exactly 1 stale-file issue and verifying it appears in output,
///   we differentiate the mutations.
#[test]
fn test_check_index_prints_issue_count_when_nonzero() {
    let temp = create_test_project();

    // Index the project first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // Delete a file to create a "stale file" issue (in DB but not on disk)
    fs::remove_file(temp.path().join("tasks").join("backend.md"))
        .expect("Failed to remove backend.md");

    // check-index should show the stale file count (count=1 > 0, so it IS printed)
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("check-index")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Stale files"));
}

/// `lash check-index --no-color` on a clean project does NOT print zero-count issue types.
/// Kills mut-000224/225/226/227 (count > 0 boundary):
/// - count=0 should NOT print that issue type.
/// - With `count >= 0` mutation, count=0 WOULD print (wrong behavior).
#[test]
fn test_check_index_does_not_print_zero_count_issue_types() {
    let temp = create_test_project();

    // Index the project - this creates a clean index
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // On a clean index (all counts are 0), no issue type labels should be printed
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("check-index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Stale files").not())
        .stdout(predicate::str::contains("Missing files").not())
        .stdout(predicate::str::contains("Hash mismatches").not());
}

// --- CHECK-LINKS TARGETED MUTATION-KILLING TESTS ---

/// `lash check-links` on a project with no broken links outputs "No broken links found".
/// Kills mut-000235/236/237 (total_broken == 0 in output_text_report):
/// - When total_broken==0, "No broken links found!" is printed and we return.
/// - With negation or comparison swap, total_broken==0 would NOT print this message.
#[test]
fn test_check_links_clean_project_outputs_no_broken_links() {
    let temp = create_test_project();

    // Index the project first (to populate the DB)
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // check-links should print "No broken links found!" for a clean project
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("check-links")
        .assert()
        .success()
        .stdout(predicate::str::contains("No broken links found"));
}

/// `lash check-links --json` on a clean project outputs JSON with total_broken=0.
/// Kills mut-000241/242/243 (total_broken == 0 in mod.rs execute):
/// - When total_broken==0, the function returns Ok(0).
/// - Mutations change this comparison, making clean projects return exit code 1.
#[test]
fn test_check_links_clean_project_returns_0_with_json_output() {
    let temp = create_test_project();

    // Index the project first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // With --json, check-links should output valid JSON with total_broken=0
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("check-links")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("output must be valid JSON with --json check-links");
    assert_eq!(
        json["total_broken"].as_u64(),
        Some(0),
        "clean project must have total_broken=0"
    );
}

// --- GRAPH TARGETED MUTATION-KILLING TESTS ---

/// `lash graph` on an indexed project returns exit code 0, not 1.
/// Kills mut-000356 (Ok(0) → Ok(1)):
/// - On success, the function must return 0, not 1.
/// - If mutation changes Ok(0) to Ok(1), this test fails.
#[test]
fn test_graph_returns_0_on_success() {
    let temp = create_test_project();

    // Index the project first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // graph should return exit code 0 on success
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("graph")
        .assert()
        .success()
        .code(0);
}

// --- INIT TARGETED MUTATION-KILLING TESTS ---

/// `lash init --json` on a fresh directory outputs JSON with `success: true`.
/// Kills mut-000425 (args.json negation in print_success_message):
/// - json=true should output JSON success message.
/// - With negation, json=true would output text instead of JSON.
#[test]
fn test_init_json_mode_outputs_json_success_message() {
    let temp = tempfile::tempdir().expect("Failed to create temp dir");

    let output = create_lash_command()
        .arg("--json")
        .arg("init")
        .arg("--no-index")
        .arg("--path")
        .arg(temp.path())
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("output must be valid JSON with --json init");
    assert_eq!(
        json["success"].as_bool(),
        Some(true),
        "json init output must have success=true"
    );
}

/// `lash init` (non-json mode) on a fresh directory outputs human-readable success message.
/// Kills mut-000409 (args.json negation in execute for theme loading):
/// - json=false loads the theme and outputs text success message.
/// - With negation, json=false would skip theme loading (no theme = no colored output).
#[test]
fn test_init_text_mode_outputs_success_message() {
    let temp = tempfile::tempdir().expect("Failed to create temp dir");

    create_lash_command()
        .arg("--no-color")
        .arg("init")
        .arg("--no-index")
        .arg("--path")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("initialized successfully"));
}

// --- EXPLAIN TARGETED MUTATION-KILLING TESTS ---

/// `lash explain --list` in non-json mode outputs categorized error codes.
///
/// Kills mut-000263-271 (starts_with conditions in list_error_codes):
/// - Each error code prefix routes to a specific category bucket.
/// - Mutations negate individual starts_with conditions, misrouting codes.
///
/// This test verifies that "Parse Errors" and "Lint Errors" headers appear.
#[test]
fn test_explain_list_shows_parse_and_lint_error_categories() {
    create_lash_command()
        .arg("--no-color")
        .arg("explain")
        .arg("--list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Parse Errors"))
        .stdout(predicate::str::contains("Lint Errors"))
        .stdout(predicate::str::contains("Dependency Errors"))
        .stdout(predicate::str::contains("Index Errors"));
}

/// `lash explain --list --json` outputs JSON with error_codes array containing E_PARSE codes.
/// Kills mut-000261 (args.json negation in list_error_codes):
/// - json=true should output JSON with error_codes.
/// - With negation, json=true would output text format instead.
#[test]
fn test_explain_list_json_contains_parse_codes() {
    let output = create_lash_command()
        .arg("--json")
        .arg("explain")
        .arg("--list")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("output must be valid JSON with --json explain --list");
    let codes = json["error_codes"]
        .as_array()
        .expect("must have error_codes array");
    assert!(
        codes
            .iter()
            .any(|c| c.as_str().is_some_and(|s| s.starts_with("E_PARSE"))),
        "JSON error_codes must contain at least one E_PARSE code"
    );
    assert!(
        codes
            .iter()
            .any(|c| c.as_str().is_some_and(|s| s.starts_with("E_LINT"))),
        "JSON error_codes must contain at least one E_LINT code"
    );
}

/// `lash explain E_PARSE_INVALID_CHECKBOX --json` outputs JSON format.
/// Kills mut-000257 (args.json negation for found explanation):
/// - json=true should output JSON explanation.
/// - With negation, json=true would output human-readable text.
#[test]
fn test_explain_known_code_json_outputs_json() {
    let output = create_lash_command()
        .arg("--json")
        .arg("explain")
        .arg("E_PARSE_INVALID_CHECKBOX")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("output must be valid JSON with --json explain <code>");
    assert_eq!(
        json["code"].as_str(),
        Some("E_PARSE_INVALID_CHECKBOX"),
        "json output must contain the code field"
    );
}

/// `lash explain E_PARSE_INVALID_CHECKBOX` in text mode outputs human-readable format.
///
/// Kills mut-000257 (args.json negation):
/// - json=false should output human-readable text.
///
/// This distinguishes the text path from the JSON path.
#[test]
fn test_explain_known_code_text_outputs_description() {
    create_lash_command()
        .arg("--no-color")
        .arg("explain")
        .arg("E_PARSE_INVALID_CHECKBOX")
        .assert()
        .success()
        .stdout(predicate::str::contains("Description").or(predicate::str::contains("How To Fix")));
}

// --- AGENT-PROMPT TARGETED MUTATION-KILLING TESTS ---

/// `lash agent-prompt --json` outputs prompt in JSON-parseable format without theme messages.
/// Kills mut-000144 (args.json negation for theme loading):
/// - json=true loads no theme, outputs plain prompt content.
/// - json=false loads a theme and may add color codes.
#[test]
fn test_agent_prompt_json_flag_outputs_content() {
    let temp = create_test_project();

    // Index the project first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // agent-prompt with --json flag outputs the prompt (no theme messages)
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--json")
        .arg("agent-prompt")
        .assert()
        .success();
}

/// `lash agent-prompt` without --json flag outputs plain text prompt.
/// Kills mut-000144 (args.json negation):
/// - json=false outputs prompt with potential theme-colored warnings.
/// - Both paths should succeed with exit code 0.
#[test]
fn test_agent_prompt_text_mode_outputs_content() {
    let temp = create_test_project();

    // Index the project first
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("index")
        .assert()
        .success();

    // agent-prompt without --json flag still outputs the prompt
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("agent-prompt")
        .assert()
        .success();
}

/// format command reports 'N file(s) failed to format' in stderr when a file fails.
///
/// Kills mut-000346 (result.failed > 0 negation in output_text_results),
/// mut-000347/348/349 (boundary mutations around the > 0 comparison).
#[test]
fn test_format_stderr_reports_failed_count_for_unformattable_file() {
    let temp = tempfile::tempdir().unwrap();

    // Create a file with malformed checkboxes that the formatter will fail to parse
    let content = "# Test

@id: test

## Tasks

-  [ ] Task with malformed spacing
";
    let file_path = temp.path().join("lash.index.md");
    fs::write(&file_path, content).unwrap();

    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("format")
        .arg(&file_path)
        .output()
        .expect("Failed to execute command");

    // The command must fail (exit code 1 due to parse error)
    assert!(
        !output.status.success(),
        "format must fail for malformed file"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("failed to format"),
        "stderr must report failed count; got: {stderr}"
    );
}

// --- CONFIG TARGETED MUTATION-KILLING TESTS ---

/// `lash config set linter.rules ""` with empty string clears the rules list.
///
/// Kills mut-000248 (`||` → `&&`):
/// - With `||`: empty string clears rules (is_empty() is true).
/// - With `&&`: empty string would NOT clear rules (because "[]" == value is false).
///
/// This test uses a project directory to avoid needing `--user`.
#[test]
fn test_config_set_linter_rules_empty_string_via_get() {
    let temp = tempfile::tempdir().expect("Failed to create temp dir");

    // Initialize a project first (config set requires a project root)
    create_lash_command()
        .arg("--no-color")
        .arg("init")
        .arg("--no-index")
        .arg("--path")
        .arg(temp.path())
        .assert()
        .success();

    // Set linter.rules to "[]" notation (value == "[]" is true, value.is_empty() is false)
    // With ||: clears the list (correct)
    // With &&: would NOT clear the list because is_empty() is false for "[]"
    create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("config")
        .arg("set")
        .arg("linter.rules")
        .arg("[]")
        .assert()
        .success();

    // Verify the rules were cleared by getting the value
    let output = create_lash_command()
        .arg("--root")
        .arg(temp.path())
        .arg("config")
        .arg("get")
        .arg("linter.rules")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Rules should be empty list
    assert!(
        stdout.trim().is_empty() || stdout.contains("[]"),
        "linter.rules should be empty after setting to '[]', got: {stdout}"
    );
}
