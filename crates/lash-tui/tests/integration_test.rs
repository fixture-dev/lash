//! Integration tests for TUI

use lash_db::{init_database, Indexer, IndexerConfig};
use lash_tui::{TuiApp, TuiResult};
use lash_types::LashConfig;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to set up a test database with sample data
fn setup_test_db() -> TuiResult<(TempDir, PathBuf)> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path().to_path_buf();
    let db_path = project_root.join(".lash").join("db.sqlite");

    // Create .lash directory
    std::fs::create_dir_all(db_path.parent().unwrap())?;

    // Initialize database
    let conn = init_database(&db_path)?;

    // Create a minimal test file in temp directory
    let test_file = project_root.join("test.md");
    std::fs::write(
        &test_file,
        r#"# Test File

- [ ] Task 1
  - [ ] Subtask 1.1
  - [x] Subtask 1.2
- [x] Task 2
- [-] Task 3
"#,
    )?;

    // Create index file
    let index_file = project_root.join("lash.index.md");
    std::fs::write(
        &index_file,
        r#"# Project Index

@id: index

## Tasks

- [ ] Root task
"#,
    )?;

    // Index the project
    let indexer_config = IndexerConfig::new(project_root)
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
    indexer.index_project()?;

    Ok((temp_dir, db_path))
}

#[test]
#[ignore] // Ignore by default because it requires a terminal
fn test_tui_app_creation() -> TuiResult<()> {
    let (_temp_dir, db_path) = setup_test_db()?;

    // Create TUI app (don't run it, just verify it initializes)
    // Note: We can't actually run the TUI in tests because it requires a terminal
    let _app = TuiApp::new(&db_path)?;

    Ok(())
}

#[test]
fn test_database_loads_files() -> TuiResult<()> {
    let (_temp_dir, db_path) = setup_test_db()?;
    let conn = lash_db::open_database(&db_path)?;

    // Verify files were indexed
    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;

    assert!(!files.is_empty(), "Database should contain indexed files");

    // Files are sorted by path, so we should have:
    // 1. lash.index.md (Project Index)
    // 2. test.md (Test File)
    assert!(files.len() >= 2, "Should have at least 2 files");

    // Find the test file
    let test_file = files.iter().find(|f| f.title == "Test File");
    assert!(test_file.is_some(), "Should find Test File");

    Ok(())
}

#[test]
fn test_database_loads_tasks() -> TuiResult<()> {
    let (_temp_dir, db_path) = setup_test_db()?;
    let conn = lash_db::open_database(&db_path)?;

    // Find the test file
    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;
    let test_file = files
        .iter()
        .find(|f| f.title == "Test File")
        .expect("Test File should exist");

    // Get tasks for that file
    let task_repo = lash_db::repository::TaskRepository::new(&conn);
    let tasks = task_repo.get_by_file(test_file.id)?;

    assert_eq!(tasks.len(), 5, "Should have 5 tasks (including subtasks)");

    // Verify task hierarchy
    let task1 = &tasks[0];
    assert_eq!(task1.title, "Task 1");
    assert_eq!(task1.status, lash_types::TaskStatus::Open);

    // Verify subtasks exist
    let subtask1 = &tasks[1];
    assert_eq!(subtask1.title, "Subtask 1.1");
    assert_eq!(subtask1.depth, 1);

    Ok(())
}
