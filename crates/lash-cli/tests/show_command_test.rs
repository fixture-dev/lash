//! Integration tests for the `lash show` command

mod common;

use clap::Parser;
use common::temp_test_dir;
use lash_core::parser::parse_file;
use lash_db::{open_database, DependencyRepository, FileRepository, TaskRepository};
use lash_types::{DependencyKind, LashConfig};
use std::fs;
use std::path::PathBuf;

/// Create a test project with files containing dependencies
fn setup_test_project_with_deps() -> (tempfile::TempDir, PathBuf) {
    let temp_dir = temp_test_dir();
    let project_root = temp_dir.path().to_path_buf();

    // Create .lash directory
    let lash_dir = project_root.join(".lash");
    fs::create_dir_all(&lash_dir).unwrap();

    // Create test files with tasks
    let file1_content = r#"# File 1

@id: file1

## Tasks

- [ ] Task A
  @id: task-a
  @labels: backend

- [ ] Task B
  @id: task-b
  @depends-on: file1#task-a
  @labels: backend
"#;

    let file2_content = r#"# File 2

@id: file2

## Tasks

- [ ] Task C
  @id: task-c
  @depends-on: file1#task-b
  @labels: frontend

- [ ] Task D
  @id: task-d
  @labels: frontend
"#;

    let file1_path = project_root.join("file1.md");
    let file2_path = project_root.join("file2.md");

    fs::write(&file1_path, file1_content).unwrap();
    fs::write(&file2_path, file2_content).unwrap();

    // Initialize database with schema
    let db_path = lash_dir.join("db.sqlite");
    let conn = lash_db::init_database(&db_path).unwrap();

    let file_repo = FileRepository::new(&conn);
    let task_repo = TaskRepository::new(&conn);
    let dep_repo = DependencyRepository::new(&conn);

    // Create a minimal config
    let config = LashConfig::default();

    // Parse file1
    let file1 = parse_file(&file1_path, &config).unwrap();
    let file1_db_id = file_repo.insert(&file1).unwrap();

    // Insert tasks from file1
    for task in file1.tasks.tasks() {
        task_repo.insert(task, file1_db_id, &file1.id).unwrap();
    }

    // Parse file2
    let file2 = parse_file(&file2_path, &config).unwrap();
    let file2_db_id = file_repo.insert(&file2).unwrap();

    // Insert tasks from file2
    for task in file2.tasks.tasks() {
        task_repo.insert(task, file2_db_id, &file2.id).unwrap();
    }

    // Manually insert dependencies
    // Task B depends on Task A
    let task_a = task_repo.get_by_full_id("file1#task-a").unwrap().unwrap();
    let task_b = task_repo.get_by_full_id("file1#task-b").unwrap().unwrap();
    dep_repo
        .insert(
            task_b.id,
            Some(task_a.id),
            &DependencyKind::ExplicitId,
            None,
        )
        .unwrap();

    // Task C depends on Task B
    let task_c = task_repo.get_by_full_id("file2#task-c").unwrap().unwrap();
    dep_repo
        .insert(
            task_c.id,
            Some(task_b.id),
            &DependencyKind::ExplicitId,
            None,
        )
        .unwrap();

    (temp_dir, project_root)
}

#[test]
fn test_show_command_exists() {
    // Verify the show command is registered in the CLI
    use clap::CommandFactory;
    use lash_cli::cli::LashCli;

    let cli = LashCli::command();
    let show_cmd = cli.find_subcommand("show");
    assert!(show_cmd.is_some(), "Show command should be registered");
}

#[test]
fn test_show_accepts_target() {
    use lash_cli::cli::LashCli;

    // Should parse successfully with just a target
    let result = LashCli::try_parse_from(["lash", "show", "file#task"]);
    assert!(result.is_ok(), "Should parse show command with target");
}

#[test]
fn test_show_accepts_deps_flag() {
    use lash_cli::cli::LashCli;

    let result = LashCli::try_parse_from(["lash", "show", "file#task", "--deps"]);
    assert!(result.is_ok(), "Should parse show command with --deps");
}

#[test]
fn test_show_accepts_rdeps_flag() {
    use lash_cli::cli::LashCli;

    let result = LashCli::try_parse_from(["lash", "show", "file#task", "--rdeps"]);
    assert!(result.is_ok(), "Should parse show command with --rdeps");
}

#[test]
fn test_show_accepts_both_flags() {
    use lash_cli::cli::LashCli;

    let result = LashCli::try_parse_from(["lash", "show", "file#task", "--deps", "--rdeps"]);
    assert!(
        result.is_ok(),
        "Should parse show command with both --deps and --rdeps"
    );
}

#[test]
fn test_show_verifies_dependency_resolution() {
    let (_temp_dir, project_root) = setup_test_project_with_deps();
    let db_path = project_root.join(".lash/db.sqlite");
    let conn = open_database(&db_path).unwrap();

    let task_repo = TaskRepository::new(&conn);
    let dep_repo = DependencyRepository::new(&conn);

    // Get task B
    let task_b = task_repo.get_by_full_id("file1#task-b").unwrap().unwrap();

    // Verify it has one dependency
    let deps = dep_repo.get_dependencies(task_b.id).unwrap();
    assert_eq!(deps.len(), 1);

    // Verify we can resolve that dependency
    let dep_task_id = deps[0].to_task_id.unwrap();
    let dep_task = task_repo.get_by_db_id(dep_task_id).unwrap();
    assert!(dep_task.is_some());
    assert_eq!(dep_task.unwrap().full_id, "file1#task-a");
}

#[test]
fn test_show_verifies_reverse_dependency_resolution() {
    let (_temp_dir, project_root) = setup_test_project_with_deps();
    let db_path = project_root.join(".lash/db.sqlite");
    let conn = open_database(&db_path).unwrap();

    let task_repo = TaskRepository::new(&conn);
    let dep_repo = DependencyRepository::new(&conn);

    // Get task B
    let task_b = task_repo.get_by_full_id("file1#task-b").unwrap().unwrap();

    // Verify it has one dependent (task C)
    let rdeps = dep_repo.get_dependents(task_b.id).unwrap();
    assert_eq!(rdeps.len(), 1);

    // Verify we can resolve that dependent
    let rdep_task = task_repo.get_by_db_id(rdeps[0].from_task_id).unwrap();
    assert!(rdep_task.is_some());
    assert_eq!(rdep_task.unwrap().full_id, "file2#task-c");
}

/// Create a test project with tasks containing contextual notes
fn setup_test_project_with_notes() -> (tempfile::TempDir, PathBuf) {
    let temp_dir = temp_test_dir();
    let project_root = temp_dir.path().to_path_buf();

    // Create .lash directory
    let lash_dir = project_root.join(".lash");
    fs::create_dir_all(&lash_dir).unwrap();

    // Create test file with tasks and contextual notes
    let file_content = r#"# Test File

@id: test-file

## Tasks

- [ ] Task with notes
  @id: task-with-notes
  - Use library X for parsing
  - Target < 100ms latency
  - Must handle edge cases

- [ ] Task without notes
  @id: task-without-notes

- [ ] Parent task
  @id: parent-task
  - [ ] Child task with notes
    @id: child-task
    - This is a child note
    - Another child note
"#;

    let file_path = project_root.join("test.md");
    fs::write(&file_path, file_content).unwrap();

    // Initialize database with schema
    let db_path = lash_dir.join("db.sqlite");
    let conn = lash_db::init_database(&db_path).unwrap();

    let file_repo = FileRepository::new(&conn);
    let task_repo = TaskRepository::new(&conn);

    // Create a minimal config
    let config = LashConfig::default();

    // Parse file
    let file = parse_file(&file_path, &config).unwrap();
    let file_db_id = file_repo.insert(&file).unwrap();

    // Insert tasks
    for task in file.tasks.tasks() {
        task_repo.insert(task, file_db_id, &file.id).unwrap();
    }

    (temp_dir, project_root)
}

#[test]
fn test_show_task_with_contextual_notes() {
    let (_temp_dir, project_root) = setup_test_project_with_notes();
    let db_path = project_root.join(".lash/db.sqlite");
    let conn = open_database(&db_path).unwrap();

    let task_repo = TaskRepository::new(&conn);

    // Get task with notes
    let task = task_repo
        .get_by_full_id("test-file#task-with-notes")
        .unwrap()
        .unwrap();

    // Verify contextual notes are present
    assert_eq!(task.contextual_notes.len(), 3);
    assert_eq!(task.contextual_notes[0].text(), "Use library X for parsing");
    assert_eq!(task.contextual_notes[1].text(), "Target < 100ms latency");
    assert_eq!(task.contextual_notes[2].text(), "Must handle edge cases");
}

#[test]
fn test_show_task_without_contextual_notes() {
    let (_temp_dir, project_root) = setup_test_project_with_notes();
    let db_path = project_root.join(".lash/db.sqlite");
    let conn = open_database(&db_path).unwrap();

    let task_repo = TaskRepository::new(&conn);

    // Get task without notes
    let task = task_repo
        .get_by_full_id("test-file#task-without-notes")
        .unwrap()
        .unwrap();

    // Verify no contextual notes
    assert_eq!(task.contextual_notes.len(), 0);
}

#[test]
fn test_show_child_task_with_contextual_notes() {
    let (_temp_dir, project_root) = setup_test_project_with_notes();
    let db_path = project_root.join(".lash/db.sqlite");
    let conn = open_database(&db_path).unwrap();

    let task_repo = TaskRepository::new(&conn);

    // Get child task with notes
    let task = task_repo
        .get_by_full_id("test-file#child-task")
        .unwrap()
        .unwrap();

    // Verify contextual notes are present
    assert_eq!(task.contextual_notes.len(), 2);
    assert_eq!(task.contextual_notes[0].text(), "This is a child note");
    assert_eq!(task.contextual_notes[1].text(), "Another child note");
}

#[test]
fn test_show_contextual_notes_are_serialized_to_json() {
    let (_temp_dir, project_root) = setup_test_project_with_notes();
    let db_path = project_root.join(".lash/db.sqlite");
    let conn = open_database(&db_path).unwrap();

    let task_repo = TaskRepository::new(&conn);

    // Get task with notes
    let task = task_repo
        .get_by_full_id("test-file#task-with-notes")
        .unwrap()
        .unwrap();

    // Serialize task to JSON
    let json = serde_json::to_string(&task).unwrap();

    // Verify JSON contains contextual_notes field
    assert!(json.contains("contextual_notes"));
    assert!(json.contains("Use library X for parsing"));
    assert!(json.contains("Target < 100ms latency"));
    assert!(json.contains("Must handle edge cases"));
}

// ===== GitHub issue #26: `lash show` displays the full task record =====

use common::{run_lash_command, TestProject};
use predicates::prelude::*;

/// A project with:
/// - `launch.md`: a done task (`pay-flow`) and an open task (`email`)
/// - `tasks.md`: `ship-feature`, which has a multi-line `@agent-note`,
///   `@owner`/`@estimate`, three `@depends-on` refs (one satisfied, one
///   unsatisfied, one dangling), and three children (one `@id`-tagged done
///   child, one `@id`-tagged open child with its own nested child, and one
///   plain-bullet waived child) — plus `no-frills`, a task with none of the
///   above, to verify empty fields are suppressed rather than printed blank.
fn project_with_full_task_record() -> TestProject {
    TestProject::builder()
        .with_index("show26", "Show26 Fixture")
        .with_file(
            "launch.md",
            r#"# Launch

@id: launch

## Tasks

- [x] Set up payment provider
  @id: pay-flow

- [ ] Send confirmation email
  @id: email
"#,
        )
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Ship feature
  @id: ship-feature
  @owner: alice
  @estimate: 3d
  @depends-on: launch#pay-flow, launch#email, launch#ghost-task
  @agent-note: First line of the note.
    Second line of the note.
    Third line with more detail.
  - [x] Set up Stripe account
    @id: stripe-setup
  - [ ] Configure webhook
    @id: webhook
    - [ ] Test webhook locally
      @id: test-webhook
  - [-] Old approach

- [ ] No frills task
  @id: no-frills
"#,
        )
        .build()
}

fn index_project(project: &TestProject) {
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();
}

#[test]
fn show_full_output_includes_agent_note_dependency_status_and_children() {
    let project = project_with_full_task_record();
    index_project(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("--no-color")
        .arg("show")
        .arg("tasks#ship-feature")
        .assert()
        .success()
        // Agent note: full multi-line content, line breaks preserved.
        .stdout(predicate::str::contains("Agent note:"))
        .stdout(predicate::str::contains("First line of the note."))
        .stdout(predicate::str::contains("Second line of the note."))
        .stdout(predicate::str::contains("Third line with more detail."))
        // Dependencies: satisfied, unsatisfied, and a dangling reference —
        // none of which should crash the command.
        .stdout(predicate::str::contains("Depends on (1/3 satisfied):"))
        .stdout(predicate::str::contains("[done]"))
        .stdout(predicate::str::contains("launch#pay-flow"))
        .stdout(predicate::str::contains("[open]"))
        .stdout(predicate::str::contains("launch#email"))
        .stdout(predicate::str::contains("[unresolved]"))
        .stdout(predicate::str::contains("launch#ghost-task"))
        // Children: both @id-tagged and plain-bullet, with checkbox state
        // and a nested-descendant count for the one with a grandchild.
        .stdout(predicate::str::contains("Children (2/3 done):"))
        .stdout(predicate::str::contains(
            "Set up Stripe account (tasks#stripe-setup)",
        ))
        .stdout(predicate::str::contains("Configure webhook"))
        .stdout(predicate::str::contains("1 nested"))
        .stdout(predicate::str::contains("Old approach"))
        // Other populated metadata already shown by `show`.
        .stdout(predicate::str::contains("Owner:    alice"))
        .stdout(predicate::str::contains("Estimate: 3d"));
}

#[test]
fn show_short_flag_preserves_terse_output() {
    let project = project_with_full_task_record();
    index_project(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("--no-color")
        .arg("show")
        .arg("tasks#ship-feature")
        .arg("--short")
        .assert()
        .success()
        .stdout(predicate::str::contains("ID:       tasks#ship-feature"))
        .stdout(predicate::str::contains("Title:    Ship feature"))
        .stdout(predicate::str::contains("Status:   open"))
        .stdout(predicate::str::contains("File:     tasks.md"))
        // Everything issue #26 added must be absent under --short.
        .stdout(predicate::str::contains("Agent note:").not())
        .stdout(predicate::str::contains("Depends on").not())
        .stdout(predicate::str::contains("Children").not())
        .stdout(predicate::str::contains("Owner:").not())
        .stdout(predicate::str::contains("Estimate:").not());
}

#[test]
fn show_full_output_suppresses_empty_fields() {
    let project = project_with_full_task_record();
    index_project(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("--no-color")
        .arg("show")
        .arg("tasks#no-frills")
        .assert()
        .success()
        .stdout(predicate::str::contains("ID:       tasks#no-frills"))
        // No agent note, deps, children, owner, estimate, or labels for a
        // task that has none of them — nothing should print blank headers.
        .stdout(predicate::str::contains("Agent note:").not())
        .stdout(predicate::str::contains("Depends on").not())
        .stdout(predicate::str::contains("Children").not())
        .stdout(predicate::str::contains("Owner:").not())
        .stdout(predicate::str::contains("Estimate:").not())
        .stdout(predicate::str::contains("Labels:").not());
}

#[test]
fn show_json_includes_agent_note_dependency_status_and_children() {
    let project = project_with_full_task_record();
    index_project(&project);

    let output = run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("--json")
        .arg("show")
        .arg("tasks#ship-feature")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json = common::parse_json_output(&String::from_utf8(output).unwrap());

    assert_eq!(
        json["agent_note"],
        "First line of the note.\nSecond line of the note.\nThird line with more detail."
    );

    assert_eq!(json["depends_on"]["satisfied"], 1);
    assert_eq!(json["depends_on"]["total"], 3);
    let deps = json["depends_on"]["items"].as_array().unwrap();
    assert_eq!(deps.len(), 3);
    assert!(deps
        .iter()
        .any(|d| d["full_id"] == "launch#pay-flow" && d["satisfied"] == true));
    assert!(deps
        .iter()
        .any(|d| d["full_id"] == "launch#email" && d["satisfied"] == false));
    // The dangling reference resolves to no full_id/title/status rather
    // than crashing the command.
    assert!(deps.iter().any(|d| d["reference"] == "launch#ghost-task"
        && d["full_id"].is_null()
        && d["satisfied"] == false));

    assert_eq!(json["children"]["done"], 2);
    assert_eq!(json["children"]["total"], 3);
    let children = json["children"]["items"].as_array().unwrap();
    assert!(children
        .iter()
        .any(|c| c["full_id"] == "tasks#webhook" && c["nested_count"] == 1));
}

#[test]
fn show_json_short_omits_extended_fields() {
    let project = project_with_full_task_record();
    index_project(&project);

    let output = run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("--json")
        .arg("show")
        .arg("tasks#ship-feature")
        .arg("--short")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json = common::parse_json_output(&String::from_utf8(output).unwrap());

    assert_eq!(json["id"], "tasks#ship-feature");
    assert_eq!(json["title"], "Ship feature");
    assert!(json.get("agent_note").is_none());
    assert!(json.get("depends_on").is_none());
    assert!(json.get("children").is_none());
}
