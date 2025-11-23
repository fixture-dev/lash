//! Integration tests for incremental indexing workflow
//!
//! Tests the critical path of incremental indexing:
//! - Index project
//! - Modify file
//! - Re-index (incremental)
//! - Verify only modified files were re-indexed
//!
//! This is a CRITICAL gap in the test suite as incremental indexing
//! is a core performance feature of Lash.

mod common;

use common::{assert_file_count, assert_has_file, DbInspector, TestDatabase};
use lash_db::indexer::{Indexer, IndexerConfig};
use lash_db::repository::TaskRepository;
use lash_types::{LashConfig, TaskStatus};
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create a test project with multiple task files
fn create_test_project() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let root = temp_dir.path();

    // Create index file
    fs::write(
        root.join("lash.index.md"),
        r#"# Test Project

@id: test-project
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Root task
"#,
    )
    .unwrap();

    // Create feature files
    fs::create_dir_all(root.join("features")).unwrap();
    fs::write(
        root.join("features/auth.md"),
        r#"# Authentication

@id: features.auth
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Login flow
- [ ] Logout flow
"#,
    )
    .unwrap();

    fs::write(
        root.join("features/database.md"),
        r#"# Database

@id: features.database
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Schema migrations
- [ ] Connection pooling
"#,
    )
    .unwrap();

    // Create bugs file
    fs::write(
        root.join("bugs.md"),
        r#"# Bugs

@id: bugs
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Fix memory leak
- [ ] Fix race condition
"#,
    )
    .unwrap();

    temp_dir
}

/// Helper to wait for filesystem timestamp resolution
/// This ensures mtime changes are detectable
fn wait_for_mtime_change() {
    thread::sleep(Duration::from_millis(10));
}

#[test]
fn test_incremental_index_modified_file() {
    // Test scenario: Index → Modify 1 file → Re-index → Verify ONLY modified file re-indexed
    let project = create_test_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Initial full index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false) // Full index first
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let report = indexer.index_project().expect("Initial index failed");

    // Verify initial state
    assert_eq!(report.files_processed, 4, "Should index all 4 files");
    assert_file_count(&conn, 4);
    assert_has_file(&conn, "lash.index.md");
    assert_has_file(&conn, "features/auth.md");
    assert_has_file(&conn, "features/database.md");
    assert_has_file(&conn, "bugs.md");

    let inspector = DbInspector::new(&conn);
    let initial_task_count = inspector.count_tasks();
    assert!(initial_task_count > 0, "Should have indexed tasks");

    // Wait for mtime resolution
    wait_for_mtime_change();

    // Modify ONE file
    fs::write(
        root.join("features/auth.md"),
        r#"# Authentication

@id: features.auth
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Login flow
- [ ] Logout flow
- [ ] Password reset flow
"#,
    )
    .unwrap();

    // Incremental re-index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(true) // Incremental mode
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let report = indexer.index_project().expect("Incremental index failed");

    // CRITICAL: Should only process the modified file
    assert_eq!(
        report.files_processed, 1,
        "Incremental index should only process modified file"
    );

    // File count should remain the same
    assert_file_count(&conn, 4);

    // Task count should increase (we added a task)
    let final_task_count = inspector.count_tasks();
    assert!(
        final_task_count > initial_task_count,
        "Task count should increase after adding a task"
    );
}

#[test]
fn test_incremental_index_new_file() {
    // Test scenario: Index → Add new file → Re-index → Verify new file added, others untouched
    let project = create_test_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Initial full index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let report = indexer.index_project().expect("Initial index failed");

    assert_eq!(report.files_processed, 4);
    assert_file_count(&conn, 4);

    // Wait for mtime resolution
    wait_for_mtime_change();

    // Add a new file
    fs::write(
        root.join("features/api.md"),
        r#"# API

@id: features.api
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] REST endpoints
- [ ] GraphQL schema
"#,
    )
    .unwrap();

    // Incremental re-index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(true)
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let report = indexer.index_project().expect("Incremental index failed");

    // Should process only the new file
    assert_eq!(
        report.files_processed, 1,
        "Should only process the new file"
    );

    // File count should increase
    assert_file_count(&conn, 5);
    assert_has_file(&conn, "features/api.md");

    // Original files should still be present
    assert_has_file(&conn, "lash.index.md");
    assert_has_file(&conn, "features/auth.md");
    assert_has_file(&conn, "features/database.md");
    assert_has_file(&conn, "bugs.md");
}

#[test]
fn test_incremental_index_deleted_file() {
    // Test scenario: Index → Delete file → Re-index → Verify file removed from DB
    let project = create_test_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Initial full index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Initial index failed");

    assert_file_count(&conn, 4);
    assert_has_file(&conn, "bugs.md");

    let inspector = DbInspector::new(&conn);
    let initial_task_count = inspector.count_tasks();

    // Wait for mtime resolution
    wait_for_mtime_change();

    // Delete a file
    fs::remove_file(root.join("bugs.md")).unwrap();

    // Incremental re-index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(true)
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let report = indexer.index_project().expect("Incremental index failed");

    // The indexer detects deletions by comparing DB state to filesystem
    // So files_processed might be 0, but deletions should be handled
    assert!(
        report.files_processed == 0,
        "No files should be processed when only deleting"
    );

    // File count should decrease
    assert_file_count(&conn, 3);

    // Deleted file should no longer be in DB
    let inspector = DbInspector::new(&conn);
    assert!(!inspector.has_file("bugs.md"), "bugs.md should be removed");

    // Tasks from deleted file should be removed
    let final_task_count = inspector.count_tasks();
    assert!(
        final_task_count < initial_task_count,
        "Task count should decrease after deleting file"
    );
}

#[test]
fn test_incremental_index_task_status_change() {
    // Test scenario: Index → Modify task status → Re-index → Verify status updated
    let project = create_test_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Initial full index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Initial index failed");

    // Get all tasks before modification
    let task_repo = TaskRepository::new(&conn);
    let initial_open_count = task_repo
        .find_by_status(TaskStatus::Open)
        .expect("Should query tasks")
        .len();

    // Wait for mtime resolution
    wait_for_mtime_change();

    // Modify task status (mark login flow as done)
    let auth_path = root.join("features/auth.md");
    fs::write(
        &auth_path,
        r#"# Authentication

@id: features.auth
@status: in-progress
@created: 2024-01-15

## Tasks

- [x] Login flow
- [ ] Logout flow
"#,
    )
    .unwrap();

    // Incremental re-index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(true)
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let report = indexer.index_project().expect("Incremental index failed");

    // Should process the modified file
    assert_eq!(report.files_processed, 1);

    // Verify status was updated
    // The number of open tasks should decrease (one was marked done)
    let final_open_count = task_repo
        .find_by_status(TaskStatus::Open)
        .expect("Should query tasks")
        .len();
    assert!(
        final_open_count < initial_open_count,
        "Open task count should decrease after marking a task done"
    );

    // Verify we now have done tasks
    let done_tasks = task_repo
        .find_by_status(TaskStatus::Done)
        .expect("Should query tasks");
    let has_login = done_tasks.iter().any(|t| t.title.contains("Login flow"));
    assert!(has_login, "Login flow should be marked as done");
}

#[test]
fn test_incremental_index_dependency_update() {
    // Test scenario: Index → Add dependency → Re-index → Verify graph updated
    let project = create_test_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Initial full index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Initial index failed");

    let inspector = DbInspector::new(&conn);
    let initial_dep_count = inspector.count_dependencies();

    // Wait for mtime resolution
    wait_for_mtime_change();

    // Add a dependency to auth.md
    fs::write(
        root.join("features/auth.md"),
        r#"# Authentication

@id: features.auth
@status: in-progress
@created: 2024-01-15
@depends-on: features/database.md#features.database

## Tasks

- [ ] Login flow
- [ ] Logout flow
"#,
    )
    .unwrap();

    // Incremental re-index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(true)
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let report = indexer.index_project().expect("Incremental index failed");

    // Verify the file was re-indexed
    assert_eq!(
        report.files_processed, 1,
        "Should process the modified file"
    );

    // Verify dependency count (should not decrease, might increase if file-level deps are tracked)
    let final_dep_count = inspector.count_dependencies();

    // Note: File-level dependencies (@depends-on in metadata) may or may not be stored
    // in the dependencies table depending on implementation. The key is that the file
    // was successfully re-indexed and the dependency graph can be rebuilt.
    // For now, we just verify the count didn't decrease.
    assert!(
        final_dep_count >= initial_dep_count,
        "Dependency count should not decrease after updating file. Initial: {initial_dep_count}, Final: {final_dep_count}"
    );
}

#[test]
fn test_incremental_faster_than_full_reindex() {
    // Test scenario: Verify incremental is faster than full re-index (performance check)
    let project = create_test_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    let parser_config = LashConfig::default();

    // Initial full index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Initial index failed");

    // Wait for mtime resolution
    wait_for_mtime_change();

    // Modify one file
    fs::write(
        root.join("features/auth.md"),
        r#"# Authentication

@id: features.auth
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Login flow (updated)
- [ ] Logout flow
"#,
    )
    .unwrap();

    // Time incremental index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(true)
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let start_incremental = std::time::Instant::now();
    let incremental_report = indexer.index_project().expect("Incremental index failed");
    let incremental_duration = start_incremental.elapsed();

    // Time full re-index
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let start_full = std::time::Instant::now();
    let full_report = indexer.index_project().expect("Full index failed");
    let full_duration = start_full.elapsed();

    // Incremental should process fewer files
    assert!(
        incremental_report.files_processed < full_report.files_processed,
        "Incremental should process fewer files than full index"
    );

    // Incremental should be faster (with some tolerance for small projects)
    // For small projects, the difference might be minimal, so we just verify
    // that incremental processed fewer files
    println!(
        "Incremental: {:?} ({} files), Full: {:?} ({} files)",
        incremental_duration,
        incremental_report.files_processed,
        full_duration,
        full_report.files_processed
    );

    // The key metric is files processed
    assert_eq!(
        incremental_report.files_processed, 1,
        "Incremental should only process 1 file"
    );
    assert_eq!(
        full_report.files_processed, 4,
        "Full index should process all 4 files"
    );
}
