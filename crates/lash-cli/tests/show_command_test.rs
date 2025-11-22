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
