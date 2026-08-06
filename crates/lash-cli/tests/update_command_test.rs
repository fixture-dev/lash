//! Integration tests for the `lash update` command (GitHub issue #25)

mod common;

use common::{run_lash_command, TestProject};
use predicates::prelude::*;
use std::fs;

/// The key scenario from issue #25: retitling a task whose id is derived
/// from its title must pin the old derived slug as an explicit `@id:` so
/// every `@depends-on` reference elsewhere in the project that pointed at
/// it keeps resolving.
#[test]
fn test_update_title_pins_id_and_cross_file_ref_still_resolves() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "auth.md",
            r#"# Auth Features

@id: auth

## Tasks

- [ ] Legacy OAuth Flow
"#,
        )
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Build payment processor
  @id: payment-processor
  @depends-on: auth#legacy-oauth-flow
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
        .arg("update")
        .arg("auth#legacy-oauth-flow")
        .arg("--title")
        .arg("New OAuth2 Flow")
        .assert()
        .success()
        .stdout(predicate::str::contains("pinned @id: legacy-oauth-flow"));

    let auth_content = fs::read_to_string(project.file_path("auth.md")).unwrap();
    assert!(auth_content.contains("- [ ] New OAuth2 Flow"));
    assert!(auth_content.contains("@id: legacy-oauth-flow"));
    // @id must come before the title line's position in the file relative
    // ordering doesn't matter, but it must be the task's own annotation.
    let lines: Vec<&str> = auth_content.lines().collect();
    let title_idx = lines
        .iter()
        .position(|l| l.contains("New OAuth2 Flow"))
        .unwrap();
    let id_idx = lines
        .iter()
        .position(|l| l.contains("@id: legacy-oauth-flow"))
        .unwrap();
    assert!(id_idx > title_idx, "@id must be nested under the task line");

    // Round-trip: must pass lint (no dangling references).
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();

    // The dependent task's @depends-on must still resolve to the retitled
    // task, not go dangling.
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("show")
        .arg("tasks#payment-processor")
        .assert()
        .success()
        .stdout(predicate::str::contains("auth#legacy-oauth-flow"))
        .stdout(predicate::str::contains("New OAuth2 Flow"))
        .stdout(predicate::str::contains("unresolved").not());
}

#[test]
fn test_update_title_on_already_explicit_id_task_does_not_pin() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Old title
  @id: fixed-id
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
        .arg("update")
        .arg("tasks#fixed-id")
        .arg("--title")
        .arg("Brand new title")
        .assert()
        .success()
        .stdout(predicate::str::contains("pinned @id").not());

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("- [ ] Brand new title"));
    assert!(content.contains("@id: fixed-id"));
    // Only one @id line for the task - no duplicate pin (the file header's
    // own @id: tasks is a separate, expected annotation).
    assert_eq!(content.matches("@id: fixed-id").count(), 1);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();
}

#[test]
fn test_update_retitle_preserves_inline_label() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Fix bug #backend
  @id: fix-bug
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
        .arg("update")
        .arg("tasks#fix-bug")
        .arg("--title")
        .arg("Fix the bug properly")
        .assert()
        .success();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("- [ ] Fix the bug properly #backend"));

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();
}

#[test]
fn test_update_add_and_remove_label() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Plain task
  @id: plain-task
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
        .arg("update")
        .arg("tasks#plain-task")
        .arg("--add-label")
        .arg("urgent")
        .assert()
        .success()
        .stdout(predicate::str::contains("added label #urgent"));

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("- [ ] Plain task #urgent"));

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("update")
        .arg("tasks#plain-task")
        .arg("--remove-label")
        .arg("urgent")
        .assert()
        .success()
        .stdout(predicate::str::contains("removed label #urgent"));

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("- [ ] Plain task"));
    assert!(!content.contains("#urgent"));

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();
}

#[test]
fn test_update_remove_label_not_present_errors() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Plain task
  @id: plain-task
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
        .arg("update")
        .arg("tasks#plain-task")
        .arg("--remove-label")
        .arg("nonexistent")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("E_LABEL_NOT_FOUND"));

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("- [ ] Plain task\n"));
}

#[test]
fn test_update_owner_and_estimate_set_and_clear() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task
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
        .arg("update")
        .arg("tasks#task-1")
        .arg("--owner")
        .arg("alice")
        .arg("--estimate")
        .arg("2h")
        .assert()
        .success();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("@owner: alice"));
    assert!(content.contains("@estimate: 2h"));

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("update")
        .arg("tasks#task-1")
        .arg("--owner")
        .arg("")
        .arg("--estimate")
        .arg("")
        .assert()
        .success();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(!content.contains("@owner"));
    assert!(!content.contains("@estimate"));

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();
}

#[test]
fn test_update_agent_note_replace_and_append() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task
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
        .arg("update")
        .arg("tasks#task-1")
        .arg("--agent-note")
        .arg("First note")
        .assert()
        .success();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("@agent-note: First note"));

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("update")
        .arg("tasks#task-1")
        .arg("--append-agent-note")
        .arg("Second note")
        .assert()
        .success();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("@agent-note: First note"));
    assert!(content.contains("Second note"));

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();

    // The appended line should show up as part of the note when shown.
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("show")
        .arg("tasks#task-1")
        .assert()
        .success()
        .stdout(predicate::str::contains("First note"))
        .stdout(predicate::str::contains("Second note"));

    // Replacing should drop the old (multi-line) note entirely.
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("update")
        .arg("tasks#task-1")
        .arg("--agent-note")
        .arg("Replacement note")
        .assert()
        .success();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("@agent-note: Replacement note"));
    assert!(!content.contains("First note"));
    assert!(!content.contains("Second note"));
}

#[test]
fn test_update_add_depends_on_dangling_is_hard_error_file_untouched() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task
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

    let before = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("update")
        .arg("tasks#task-1")
        .arg("--add-depends-on")
        .arg("does-not-exist")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("E_CREATE_DEPENDENCY_NOT_FOUND"));

    let after = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert_eq!(
        before, after,
        "file must be untouched on validation failure"
    );
}

#[test]
fn test_update_add_depends_on_valid_is_written_and_resolvable() {
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

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("update")
        .arg("tasks#task-2")
        .arg("--add-depends-on")
        .arg("task-1")
        .assert()
        .success()
        .stdout(predicate::str::contains("added dependency: task-1"));

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("@depends-on: task-1"));

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("show")
        .arg("tasks#task-2")
        .assert()
        .success()
        .stdout(predicate::str::contains("tasks#task-1"))
        .stdout(predicate::str::contains("unresolved").not());
}

#[test]
fn test_update_add_depends_on_allow_forward_ref_writes_warning() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task
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
        .arg("update")
        .arg("tasks#task-1")
        .arg("--add-depends-on")
        .arg("not-yet-created")
        .arg("--allow-forward-ref")
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning"));

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("@depends-on: not-yet-created"));
}

#[test]
fn test_update_remove_depends_on() {
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
  @depends-on: task-1
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
        .arg("update")
        .arg("tasks#task-2")
        .arg("--remove-depends-on")
        .arg("task-1")
        .assert()
        .success()
        .stdout(predicate::str::contains("removed dependency: task-1"));

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(!content.contains("@depends-on"));

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();
}

#[test]
fn test_update_remove_depends_on_not_found_errors() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task
  @id: task-1
  @depends-on: other-ref
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
        .arg("update")
        .arg("tasks#task-1")
        .arg("--remove-depends-on")
        .arg("does-not-match")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("E_DEPENDS_ON_NOT_FOUND"));

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("@depends-on: other-ref"));
}

#[test]
fn test_update_dry_run_changes_nothing() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Original title
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

    let before = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("update")
        .arg("tasks#task-1")
        .arg("--title")
        .arg("New title")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would update"))
        .stdout(predicate::str::contains("New title"));

    let after = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert_eq!(before, after, "dry-run must not modify the file");
}

#[test]
fn test_update_no_mutation_flags_errors() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task
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
        .arg("update")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("E_NO_MUTATION"));
}

#[test]
fn test_update_not_found_reports_suggestions() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task
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
        .arg("update")
        .arg("tasks#taks-1") // typo
        .arg("--title")
        .arg("New title")
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("E_NOT_FOUND"))
        .stderr(predicate::str::contains("Did you mean"));
}

#[test]
fn test_update_json_output() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task
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
        .arg("update")
        .arg("tasks#task-1")
        .arg("--owner")
        .arg("alice")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["task_id"], "tasks#task-1");
    assert_eq!(json["dry_run"], false);
    let changes = json["changes"].as_array().unwrap();
    assert!(changes
        .iter()
        .any(|c| c.as_str().unwrap().contains("owner")));
}

#[test]
fn test_update_json_error() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task
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
        .arg("update")
        .arg("tasks#task-1")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json["success"], false);
    assert_eq!(json["error"]["code"], "E_NO_MUTATION");
}

#[test]
fn test_update_reindexes_without_separate_index_step() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Task
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
        .arg("update")
        .arg("tasks#task-1")
        .arg("--owner")
        .arg("alice")
        .assert()
        .success();

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
        .arg("--owner")
        .arg("alice")
        .assert()
        .success()
        .stdout(predicate::str::contains("tasks#task-1"));
}

#[test]
fn test_update_multiple_mutations_round_trip_lint() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Original title
- [ ] Other task
  @id: other-task
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
        .arg("update")
        .arg("tasks#original-title")
        .arg("--title")
        .arg("Fully updated title")
        .arg("--add-label")
        .arg("backend")
        .arg("--owner")
        .arg("bob")
        .arg("--estimate")
        .arg("3d")
        .arg("--agent-note")
        .arg("Initial note")
        .arg("--add-depends-on")
        .arg("other-task")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("- [ ] Fully updated title #backend"));
    assert!(content.contains("@id: original-title"));
    assert!(content.contains("@owner: bob"));
    assert!(content.contains("@estimate: 3d"));
    assert!(content.contains("@agent-note: Initial note"));
    assert!(content.contains("@depends-on: other-task"));
}
