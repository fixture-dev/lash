//! Integration tests for the `lash waive` command

mod common;

use common::{run_lash_command, TestProject};
use predicates::prelude::*;
use std::fs;

#[test]
fn test_waive_command_basic() {
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

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#task-1")
        .assert()
        .success()
        .stdout(predicate::str::contains("[-]"))
        .stdout(predicate::str::contains("tasks#task-1"));

    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [-] Open task"));
}

#[test]
fn test_waive_command_dry_run() {
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

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("--dry-run")
        .arg("tasks#task-1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would waive"));

    // Nothing on disk should have changed.
    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [ ] Open task"));
    assert!(!tasks_content.contains("- [-] Open task"));
}

#[test]
fn test_waive_command_multiple_tasks() {
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

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#task-1")
        .arg("tasks#task-2")
        .assert()
        .success()
        .stdout(predicate::str::contains("tasks#task-1"))
        .stdout(predicate::str::contains("tasks#task-2"));

    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [-] Task one"));
    assert!(tasks_content.contains("- [-] Task two"));
    assert!(tasks_content.contains("- [ ] Task three"));
}

#[test]
fn test_waive_command_already_waived() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [-] Already waived task
  @id: task-1
"#,
        )
        .build();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_ALREADY_WAIVED"))
        .stderr(predicate::str::contains("already waived"));
}

#[test]
fn test_waive_command_done_task_rejected() {
    // Completed work shouldn't be silently waived (issue #23 requirement).
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

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_DONE"))
        .stderr(predicate::str::contains("Hand-edit"));

    // File must be untouched.
    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [x] Completed task"));
}

#[test]
fn test_waive_command_open_in_progress_and_blocked_allowed() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Open task
  @id: task-1
- [>] In progress task
  @id: task-2
- [!] Blocked task
  @id: task-3
"#,
        )
        .build();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#task-1")
        .arg("tasks#task-2")
        .arg("tasks#task-3")
        .assert()
        .success();

    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [-] Open task"));
    assert!(tasks_content.contains("- [-] In progress task"));
    assert!(tasks_content.contains("- [-] Blocked task"));
}

#[test]
fn test_waive_command_task_not_found() {
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

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_NOT_FOUND"))
        .stderr(predicate::str::contains("Task not found"));
}

#[test]
fn test_waive_command_fuzzy_matching() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task one
  @id: task-1
"#,
        )
        .build();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#taks-1") // typo
        .assert()
        .failure()
        .stderr(predicate::str::contains("Did you mean"));
}

#[test]
fn test_waive_command_cascade_flips_plain_children() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Parent task
  @id: parent
  - Some contextual note
  - [ ] Plain step one
  - [ ] Plain step two
  - [ ] Tracked child
    @id: tracked
"#,
        )
        .build();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("--cascade")
        .arg("tasks#parent")
        .assert()
        .success()
        .stdout(predicate::str::contains("cascaded"));

    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [-] Parent task"));
    assert!(tasks_content.contains("- [-] Plain step one"));
    assert!(tasks_content.contains("- [-] Plain step two"));
    // Tracked child has its own @id and must not be cascaded.
    assert!(tasks_content.contains("- [ ] Tracked child"));
}

#[test]
fn test_waive_command_warns_on_unchecked_children_without_cascade() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Parent task
  @id: parent
  - [ ] Plain step one
"#,
        )
        .build();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#parent")
        .assert()
        .success()
        .stderr(predicate::str::contains("remain unchecked"))
        .stderr(predicate::str::contains("--cascade"));

    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [-] Parent task"));
    // Left untouched without --cascade.
    assert!(tasks_content.contains("- [ ] Plain step one"));
}

#[test]
fn test_waive_command_json_output() {
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

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    let output = run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("--json")
        .arg("waive")
        .arg("tasks#task-1")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["waived"][0]["task_id"], "tasks#task-1");
    assert_eq!(json["waived"][0]["previous_status"], "open");
}

#[test]
fn test_waive_command_json_error() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [-] Already waived task
  @id: task-1
"#,
        )
        .build();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    let output = run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("--json")
        .arg("waive")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json["success"], false);
    assert_eq!(json["errors"][0]["code"], "E_ALREADY_WAIVED");
    assert_eq!(json["errors"][0]["task_id"], "tasks#task-1");
}

#[test]
fn test_waive_command_no_database() {
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

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .code(3)
        .stderr(
            predicate::str::contains("E_IO_FILE_NOT_FOUND").or(predicate::str::contains("lash.db")),
        );
}

#[test]
fn test_waive_command_no_task_id() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .build();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .assert()
        .failure();
}

#[test]
fn test_waive_command_mixed_results() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Open task
  @id: task-1
- [-] Already waived task
  @id: task-2
- [ ] Another open task
  @id: task-3
"#,
        )
        .build();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#task-1")
        .arg("tasks#task-2") // already waived, should fail
        .arg("tasks#task-3")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("tasks#task-1"))
        .stdout(predicate::str::contains("tasks#task-3"))
        .stdout(predicate::str::contains("Summary"))
        .stderr(predicate::str::contains("E_ALREADY_WAIVED"));

    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [-] Open task"));
    assert!(tasks_content.contains("- [-] Already waived task")); // unchanged
    assert!(tasks_content.contains("- [-] Another open task"));
}

#[test]
fn test_waive_command_reindexes() {
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

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("tasks#task-1")
        .assert()
        .success();

    // The DB should already reflect the waive with no separate `lash index`
    // step (this is the whole point of issue #23).
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("list")
        .arg("--status")
        .arg("waived")
        .assert()
        .success()
        .stdout(predicate::str::contains("tasks#task-1"));
}

#[test]
fn test_waive_command_reason_recorded_and_passes_lint() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Deprecated task
  @id: task-1
  @depends-on: tasks#task-2
- [ ] Other task
  @id: task-2
"#,
        )
        .build();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("--reason")
        .arg("Superseded by the new auth flow")
        .arg("tasks#task-1")
        .assert()
        .success()
        .stdout(predicate::str::contains("reason:"))
        .stdout(predicate::str::contains("Superseded by the new auth flow"));

    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(tasks_content.contains("- [-] Deprecated task"));
    // The reason must appear as a contextual note, after the annotations so
    // @id/@depends-on still parse as one block.
    let lines: Vec<&str> = tasks_content.lines().collect();
    let id_idx = lines
        .iter()
        .position(|l| l.contains("@id: task-1"))
        .unwrap();
    let dep_idx = lines
        .iter()
        .position(|l| l.contains("@depends-on: tasks#task-2"))
        .unwrap();
    let note_idx = lines
        .iter()
        .position(|l| l.contains("Superseded by the new auth flow"))
        .unwrap();
    assert!(id_idx < dep_idx);
    assert!(dep_idx < note_idx);
    assert_eq!(lines[note_idx], "  - Superseded by the new auth flow");

    // Must round-trip through the parser and linter cleanly.
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();

    // The index must reflect the new status without a separate `lash
    // index` step.
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .success();
}

#[test]
fn test_waive_command_dry_run_does_not_write_reason() {
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

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("waive")
        .arg("--dry-run")
        .arg("--reason")
        .arg("Not needed")
        .arg("tasks#task-1")
        .assert()
        .success();

    let tasks_content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(!tasks_content.contains("Not needed"));
    assert!(tasks_content.contains("- [ ] Open task"));
}
