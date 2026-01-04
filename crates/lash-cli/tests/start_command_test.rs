//! Integration tests for the `lash start` command

mod common;

use common::{run_lash_command, TestProject};
use predicates::prelude::*;
use std::fs;

#[test]
fn test_start_command_basic() {
    // Create a test project with tasks
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Open task
- [>] In progress task
- [x] Done task
- [-] Waived task
- [!] Blocked task
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Start an open task
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#open-task")
        .assert()
        .success()
        .stdout(predicate::str::contains("[>]"))
        .stdout(predicate::str::contains("tasks#open-task"));

    // Verify the task was updated in the markdown file
    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [>] Open task"));
}

#[test]
fn test_start_command_dry_run() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Open task
  @id: task-1
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Dry run start
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("--dry-run")
        .arg("tasks#task-1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would start"));

    // Verify the task was NOT updated in the markdown file
    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [ ] Open task"));
    assert!(!tasks_content.contains("- [>] Open task"));
}

#[test]
fn test_start_command_multiple_tasks() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task one
  @id: task-1
- [ ] Task two
  @id: task-2
- [ ] Task three
  @id: task-3
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Start multiple tasks
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#task-1")
        .arg("tasks#task-2")
        .assert()
        .success()
        .stdout(predicate::str::contains("tasks#task-1"))
        .stdout(predicate::str::contains("tasks#task-2"));

    // Verify both tasks were updated
    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [>] Task one"));
    assert!(tasks_content.contains("- [>] Task two"));
    assert!(tasks_content.contains("- [ ] Task three"));
}

#[test]
fn test_start_command_already_in_progress() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [>] Already in-progress task
  @id: task-1
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Try to start a task that's already in progress
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_ALREADY_IN_PROGRESS"))
        .stderr(predicate::str::contains("already in progress"));
}

#[test]
fn test_start_command_already_complete() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [x] Completed task
  @id: task-1
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Try to start a completed task
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_ALREADY_COMPLETE"))
        .stderr(predicate::str::contains("already complete"));
}

#[test]
fn test_start_command_waived_task() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [-] Waived task
  @id: task-1
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Try to start a waived task
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_WAIVED"))
        .stderr(predicate::str::contains("waived"));
}

#[test]
fn test_start_command_blocked_task() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [!] Blocked task
  @id: task-1
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Start a blocked task (should unblock and start)
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#task-1")
        .assert()
        .success()
        .stdout(predicate::str::contains("[>]"))
        .stdout(predicate::str::contains("tasks#task-1"));

    // Verify the task was updated from blocked to in-progress
    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [>] Blocked task"));
}

#[test]
fn test_start_command_task_not_found() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Existing task
  @id: task-1
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Try to start a non-existent task
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_NOT_FOUND"))
        .stderr(predicate::str::contains("Task not found"));
}

#[test]
fn test_start_command_fuzzy_matching() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task one
  @id: task-1
- [ ] Task two
  @id: task-2
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Try to start a task with a typo - should suggest similar tasks
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#taks-1") // typo
        .assert()
        .failure()
        .stderr(predicate::str::contains("Did you mean"));
}

#[test]
fn test_start_command_json_output() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Open task
  @id: task-1
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Start with JSON output
    let output = run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("--json")
        .arg("start")
        .arg("tasks#task-1")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["started"][0]["task_id"], "tasks#task-1");
    assert_eq!(json["started"][0]["previous_status"], "open");
}

#[test]
fn test_start_command_json_error() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [x] Completed task
  @id: task-1
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Try to start a completed task with JSON output
    let output = run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("--json")
        .arg("start")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "E_ALREADY_COMPLETE");
    assert_eq!(json["errors"][0]["task_id"], "tasks#task-1");
}

#[test]
fn test_start_command_no_database() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Open task
  @id: task-1
"#,
        )
        .build();

    // Try to start without indexing first
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .code(3) // DB error exit code
        .stderr(
            predicate::str::contains("E_IO_FILE_NOT_FOUND").or(predicate::str::contains("lash.db")),
        );
}

#[test]
fn test_start_command_no_task_id() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .build();

    // Try to run start without providing a task ID
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .assert()
        .failure();
}

#[test]
fn test_start_command_mixed_results() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Open task
  @id: task-1
- [x] Done task
  @id: task-2
- [ ] Another open task
  @id: task-3
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Try to start multiple tasks where some will fail
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#task-1")
        .arg("tasks#task-2") // This should fail (already done)
        .arg("tasks#task-3")
        .assert()
        .failure() // Partial success still returns failure
        .code(1)
        .stdout(predicate::str::contains("tasks#task-1"))
        .stdout(predicate::str::contains("tasks#task-3"))
        .stdout(predicate::str::contains("Summary"))
        .stderr(predicate::str::contains("E_ALREADY_COMPLETE"));

    // Verify the open tasks were started
    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [>] Open task"));
    assert!(tasks_content.contains("- [x] Done task")); // Unchanged
    assert!(tasks_content.contains("- [>] Another open task"));
}

#[test]
fn test_start_command_reindexes() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Open task
  @id: task-1
"#,
        )
        .build();

    // Index the project
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    // Start the task
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("start")
        .arg("tasks#task-1")
        .assert()
        .success();

    // Check that the index is still in sync (start should have re-indexed)
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .success();

    // Verify we can query the task and it shows as in-progress
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("show")
        .arg("tasks#task-1")
        .assert()
        .success()
        .stdout(predicate::str::contains("in-progress"));
}
