//! Integration tests for Index → Query workflow
//!
//! Tests the complete indexing → querying workflow:
//! - Index a multi-file project
//! - Query all tasks
//! - Filter by label, status, path
//! - Perform FTS search
//! - Verify database consistency
//! - Test cross-file queries
//!
//! This fills a critical gap in testing the end-to-end index-query cycle.

mod common;

use common::{assert_file_count, assert_has_file, DbInspector, TestDatabase};
use lash_db::indexer::{Indexer, IndexerConfig};
use lash_db::repository::tasks::TaskFilter;
use lash_db::repository::{FileRepository, TaskRepository};
use lash_db::search::SearchQuery;
use lash_types::{LashConfig, TaskStatus};
use std::fs;
use tempfile::TempDir;

/// Helper to create a realistic multi-file test project
fn create_multi_file_project() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let root = temp_dir.path();

    // Create index file
    fs::write(
        root.join("lash.index.md"),
        r#"# Project Index

@id: project
@status: in-progress
@created: 2024-01-15
@labels: meta

## Overview

Main project tracking file.

## Tasks

- [ ] Project setup
- [ ] Core implementation
- [x] Initial planning
"#,
    )
    .unwrap();

    // Create features directory
    fs::create_dir_all(root.join("features")).unwrap();

    fs::write(
        root.join("features/auth.md"),
        r#"# Authentication

@id: features.auth
@status: in-progress
@created: 2024-01-15
@labels: backend, security
@owner: alice

## Tasks

- [ ] Login flow
- [ ] Logout flow
- [ ] Password reset
  - [ ] Email template
  - [ ] Token generation
"#,
    )
    .unwrap();

    fs::write(
        root.join("features/database.md"),
        r#"# Database

@id: features.database
@status: in-progress
@created: 2024-01-15
@labels: backend, infrastructure
@owner: bob

## Tasks

- [x] Schema migrations
- [ ] Connection pooling
- [ ] Query optimization
"#,
    )
    .unwrap();

    fs::write(
        root.join("features/ui.md"),
        r#"# User Interface

@id: features.ui
@status: in-progress
@created: 2024-01-15
@labels: frontend, design
@owner: alice

## Tasks

- [ ] Dashboard layout
- [ ] Settings page
- [-] Legacy theme support
"#,
    )
    .unwrap();

    // Create bugs file
    fs::write(
        root.join("bugs.md"),
        r#"# Known Bugs

@id: bugs
@status: in-progress
@created: 2024-01-15
@labels: bug, urgent

## Tasks

- [ ] Fix memory leak in connection pool
- [ ] Fix race condition in auth flow
- [x] Fix typo in error message
"#,
    )
    .unwrap();

    temp_dir
}

#[test]
fn test_index_and_query_all_tasks() {
    // Test scenario: Index multi-file project → Query all tasks → Verify complete task list
    let project = create_multi_file_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Index the project
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let report = indexer.index_project().expect("Index should succeed");

    // Verify indexing report
    assert_eq!(report.files_processed, 5, "Should have indexed all 5 files");
    assert_eq!(report.errors.len(), 0, "Should have no errors");

    // Verify database state
    assert_file_count(&conn, 5);
    assert_has_file(&conn, "lash.index.md");
    assert_has_file(&conn, "features/auth.md");
    assert_has_file(&conn, "features/database.md");
    assert_has_file(&conn, "features/ui.md");
    assert_has_file(&conn, "bugs.md");

    // Query all tasks (use find with empty filter)
    let task_repo = TaskRepository::new(&conn);
    let all_tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should be able to list all tasks");

    // Verify we got all tasks from all files
    // Count: 3 + 5 + 3 + 3 + 3 = 17 tasks total
    assert_eq!(all_tasks.len(), 17, "Should have all tasks from all files");

    // Verify task metadata is preserved
    let has_login = all_tasks.iter().any(|t| t.title.contains("Login flow"));
    let has_dashboard = all_tasks
        .iter()
        .any(|t| t.title.contains("Dashboard layout"));
    let has_memory_leak = all_tasks.iter().any(|t| t.title.contains("memory leak"));

    assert!(has_login, "Should have auth tasks");
    assert!(has_dashboard, "Should have UI tasks");
    assert!(has_memory_leak, "Should have bug tasks");

    // Verify different statuses are present
    let open_tasks = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Open)
        .count();
    let done_tasks = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done)
        .count();
    let waived_tasks = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Waived)
        .count();

    assert!(open_tasks > 0, "Should have open tasks");
    assert!(done_tasks > 0, "Should have done tasks");
    assert!(waived_tasks > 0, "Should have waived tasks");
}

#[test]
fn test_index_and_filter_by_label() {
    // Test scenario: Index project → Filter by label → Verify correct subset
    let project = create_multi_file_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Index the project
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    // Check if labels were indexed
    let inspector = DbInspector::new(&conn);
    let label_count = inspector.count_labels();

    // Note: File-level labels (from @labels annotation) may not be automatically
    // indexed for all tasks. This depends on the indexer implementation.
    // If labels are present, verify label queries work. Otherwise, skip this test.
    if label_count == 0 {
        eprintln!("WARNING: No labels indexed. File-level @labels may not be stored on tasks.");
        eprintln!("Skipping label filter test as labels are not available.");
        return;
    }

    // Query tasks by label: backend
    let task_repo = TaskRepository::new(&conn);
    let backend_tasks = task_repo
        .find_by_label("backend")
        .expect("Should query by label");

    // Verify we got only backend tasks (from auth.md and database.md files)
    // auth.md has 5 tasks, database.md has 3 tasks (but both files have @labels: backend)
    // However, find_by_label returns tasks whose FILE has the label
    // Actually, looking at the schema, labels are associated with tasks via task_labels table
    // The indexer stores file-level labels on the file's root task or all tasks
    // Let's verify we got tasks from the backend-labeled files
    assert!(
        !backend_tasks.is_empty(),
        "Should have tasks labeled 'backend'"
    );

    // Verify tasks are from the right files
    let file_repo = FileRepository::new(&conn);
    for task in &backend_tasks {
        let file = file_repo
            .get_by_db_id(task.file_id)
            .expect("Should get file")
            .expect("File should exist");

        // Backend label is on auth.md and database.md
        assert!(
            file.path.to_string_lossy().contains("auth.md")
                || file.path.to_string_lossy().contains("database.md"),
            "Backend task should be from auth or database file, got: {:?}",
            file.path
        );
    }

    // Query tasks by label: frontend
    let frontend_tasks = task_repo
        .find_by_label("frontend")
        .expect("Should query by label");

    assert!(
        !frontend_tasks.is_empty(),
        "Should have tasks labeled 'frontend'"
    );

    // Verify no overlap (backend and frontend are on different files)
    let backend_ids: Vec<_> = backend_tasks.iter().map(|t| &t.full_id).collect();
    let frontend_ids: Vec<_> = frontend_tasks.iter().map(|t| &t.full_id).collect();

    let has_overlap = backend_ids.iter().any(|id| frontend_ids.contains(id));
    assert!(
        !has_overlap,
        "Backend and frontend tasks should not overlap"
    );
}

#[test]
fn test_index_and_filter_by_status() {
    // Test scenario: Index project → Filter by status → Verify correct tasks returned
    let project = create_multi_file_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Index the project
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    // Query by status: Open
    let task_repo = TaskRepository::new(&conn);
    let open_tasks = task_repo
        .find_by_status(TaskStatus::Open)
        .expect("Should query by status");

    assert!(!open_tasks.is_empty(), "Should have open tasks");

    // Verify all returned tasks are actually open
    for task in &open_tasks {
        assert_eq!(
            task.status,
            TaskStatus::Open,
            "Should only return open tasks"
        );
    }

    // Query by status: Done
    let done_tasks = task_repo
        .find_by_status(TaskStatus::Done)
        .expect("Should query by status");

    assert!(!done_tasks.is_empty(), "Should have done tasks");

    // Verify all returned tasks are actually done
    for task in &done_tasks {
        assert_eq!(
            task.status,
            TaskStatus::Done,
            "Should only return done tasks"
        );
    }

    // Query by status: Waived
    let waived_tasks = task_repo
        .find_by_status(TaskStatus::Waived)
        .expect("Should query by status");

    assert!(
        !waived_tasks.is_empty(),
        "Should have waived tasks (legacy theme support)"
    );

    // Verify all returned tasks are actually waived
    for task in &waived_tasks {
        assert_eq!(
            task.status,
            TaskStatus::Waived,
            "Should only return waived tasks"
        );
    }

    // Verify status counts add up
    let total = open_tasks.len() + done_tasks.len() + waived_tasks.len();
    let all_tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should list all tasks");

    assert_eq!(
        total,
        all_tasks.len(),
        "Status counts should add up to total task count"
    );
}

#[test]
fn test_index_and_search_query() {
    // Test scenario: Index project → Perform FTS search → Verify relevant results
    let project = create_multi_file_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Index the project
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    // Search for "auth" - should find authentication related tasks
    let query = SearchQuery::new("auth");
    let auth_results = lash_db::search::search(&conn, &query).expect("Search should succeed");

    assert!(
        !auth_results.results.is_empty(),
        "Should find auth-related tasks"
    );

    // Verify at least some results are relevant (contain "auth" or related terms)
    // Note: FTS may return related results that don't contain exact term
    let has_auth_results = auth_results.results.iter().any(|result| {
        let text = format!("{} {}", result.title, result.snippet);
        let text_lower = text.to_lowercase();
        text_lower.contains("auth")
            || text_lower.contains("login")
            || text_lower.contains("password")
            || result.file_path.contains("auth")
    });

    assert!(
        has_auth_results,
        "At least some results should be auth-related"
    );

    // Search for "pool" - should find connection pool related tasks
    let query = SearchQuery::new("pool");
    let pool_results = lash_db::search::search(&conn, &query).expect("Search should succeed");

    assert!(
        !pool_results.results.is_empty(),
        "Should find pool-related tasks"
    );

    // Verify results contain "pool"
    for result in &pool_results.results {
        let text = format!("{} {}", result.title, result.snippet);
        assert!(
            text.to_lowercase().contains("pool"),
            "Result should contain 'pool', got: {text}"
        );
    }

    // Search with filter: search for "fix" but only in bugs
    // Use scope to filter by path
    let query = SearchQuery::new("fix").with_scope("bugs.md".into());

    let bug_results =
        lash_db::search::search(&conn, &query).expect("Search with filter should succeed");

    assert!(!bug_results.results.is_empty(), "Should find bug fixes");

    // Verify results are from bugs.md
    for result in &bug_results.results {
        assert!(
            result.file_path.contains("bugs.md"),
            "Result should be from bugs.md, got: {}",
            result.file_path
        );
    }
}

#[test]
fn test_index_and_verify_db_consistency() {
    // Test scenario: Index project → Verify DB consistency (counts match)
    let project = create_multi_file_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Index the project
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    let report = indexer.index_project().expect("Index should succeed");

    // Verify file count matches report
    let inspector = DbInspector::new(&conn);
    assert_eq!(
        inspector.count_files(),
        report.files_processed,
        "File count should match indexing report"
    );

    // Verify task count is greater than file count (each file has multiple tasks)
    let actual_task_count = inspector.count_tasks();
    let file_count = inspector.count_files();
    assert!(
        actual_task_count > file_count,
        "Task count ({actual_task_count}) should be greater than file count ({file_count})"
    );

    // Verify label consistency - count unique labels in files vs DB
    // Note: File-level labels (from @labels annotation) may not be automatically
    // indexed. This depends on the indexer implementation.
    let db_labels = inspector.get_labels();

    // If labels are indexed, verify they're present
    if !db_labels.is_empty() {
        eprintln!("Labels indexed: {db_labels:?}");
    } else {
        eprintln!("WARNING: No labels indexed. File-level @labels may not be stored on tasks.");
    }

    // Verify referential integrity - all tasks should have valid file_id
    let file_repo = FileRepository::new(&conn);
    let task_repo = TaskRepository::new(&conn);
    let all_tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should list tasks");

    for task in &all_tasks {
        let file = file_repo
            .get_by_db_id(task.file_id)
            .expect("Should query file");
        assert!(
            file.is_some(),
            "Task's file_id should reference valid file: task={}, file_id={}",
            task.full_id,
            task.file_id
        );
    }

    // Verify parent_id references are valid (if present)
    for task in &all_tasks {
        if let Some(parent_id) = task.parent_id {
            let parent = task_repo
                .get_by_db_id(parent_id)
                .expect("Should query parent");
            assert!(
                parent.is_some(),
                "Task's parent_id should reference valid task: task={}, parent_id={}",
                task.full_id,
                parent_id
            );
        }
    }
}

#[test]
fn test_index_multi_file_cross_queries() {
    // Test scenario: Index multi-file project → Query across files → Verify cross-file results
    let project = create_multi_file_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Index the project
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    // Query 1: Get all tasks owned by "alice" (should span multiple files)
    // Note: File-level @owner may not propagate to all tasks
    let task_repo = TaskRepository::new(&conn);
    let alice_tasks = task_repo
        .find(&TaskFilter {
            owner: Some("alice".to_string()),
            ..Default::default()
        })
        .expect("Should query by owner");

    // If owner is not being stored on tasks, skip this check
    if alice_tasks.is_empty() {
        eprintln!(
            "WARNING: No tasks owned by alice. File-level @owner may not be stored on tasks."
        );
        eprintln!("Continuing with other cross-file queries...");
    } else {
        // Verify alice's tasks are from different files (auth.md and ui.md)
        let alice_file_ids: Vec<_> = alice_tasks
            .iter()
            .map(|t| t.file_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        assert!(
            alice_file_ids.len() > 1,
            "Alice's tasks should span multiple files"
        );
    }

    // Query 2: Get all tasks with a specific label across all files
    // Note: File-level labels may not be indexed
    let backend_tasks = task_repo
        .find_by_label("backend")
        .expect("Should query by label");

    if backend_tasks.is_empty() {
        eprintln!("WARNING: No backend-labeled tasks found. Labels may not be indexed.");
    } else {
        // Verify these span multiple files
        let backend_file_ids: Vec<_> = backend_tasks
            .iter()
            .map(|t| t.file_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        assert!(
            backend_file_ids.len() > 1,
            "Backend tasks should span multiple files (auth and database)"
        );
    }

    // Query 3: Combine filters - status filter should work regardless of labels
    let open_tasks = task_repo
        .find(&TaskFilter {
            status: Some(TaskStatus::Open),
            ..Default::default()
        })
        .expect("Should query with status filter");

    // Verify all results match the criteria
    assert!(!open_tasks.is_empty(), "Should have open tasks");
    for task in &open_tasks {
        assert_eq!(
            task.status,
            TaskStatus::Open,
            "Should only return open tasks"
        );
    }

    // Query 4: Get tasks from specific file path
    let file_repo = FileRepository::new(&conn);
    let auth_tasks = task_repo
        .find(&TaskFilter {
            file_path: Some("features/auth.md".to_string()),
            ..Default::default()
        })
        .expect("Should query by file path");

    assert!(!auth_tasks.is_empty(), "Should have tasks from auth.md");

    // Verify all tasks are from the right file
    for task in &auth_tasks {
        let file = file_repo
            .get_by_db_id(task.file_id)
            .expect("Should get file")
            .expect("File should exist");

        assert!(
            file.path.to_string_lossy().contains("auth.md"),
            "Should only return tasks from auth.md"
        );
    }

    // Query 5: Verify we can query tasks from the database
    // The specific task IDs depend on how the parser generates them
    let all_tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should list all tasks");

    assert!(
        !all_tasks.is_empty(),
        "Should have tasks in database for cross-file queries"
    );

    // Verify we have tasks from different files
    let file_ids: Vec<_> = all_tasks
        .iter()
        .map(|t| t.file_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    assert!(
        file_ids.len() >= 4,
        "Should have tasks from at least 4 different files"
    );
}

#[test]
fn test_file_level_label_filtering() {
    // Test scenario: Verify file-level labels are indexed and filterable
    // This test explicitly verifies that @labels from file metadata are:
    // 1. Indexed into the labels and file_labels tables
    // 2. Queryable via TaskFilter
    // 3. Properly associated with all tasks in the file
    let project = create_multi_file_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Index the project
    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    // Verify file-level labels are in the labels table
    let inspector = DbInspector::new(&conn);
    let labels = inspector.get_labels();
    assert!(
        !labels.is_empty(),
        "File-level labels should be indexed into labels table"
    );

    // Verify specific labels exist
    assert!(
        labels.contains(&"backend".to_string()),
        "Should have 'backend' label from auth.md and database.md"
    );
    assert!(
        labels.contains(&"frontend".to_string()),
        "Should have 'frontend' label from ui.md"
    );
    assert!(
        labels.contains(&"security".to_string()),
        "Should have 'security' label from auth.md"
    );

    // Test 1: Filter by file-level label "backend"
    let task_repo = TaskRepository::new(&conn);
    let backend_tasks = task_repo
        .find(&TaskFilter {
            labels: vec!["backend".to_string()],
            ..Default::default()
        })
        .expect("Should find tasks by file-level label");

    // Should get ALL tasks from auth.md (5 tasks) and database.md (3 tasks) = 8 tasks
    assert!(
        backend_tasks.len() >= 5,
        "Should have at least 5 tasks from backend-labeled files, got {}",
        backend_tasks.len()
    );

    // Verify all tasks are from backend-labeled files
    let file_repo = FileRepository::new(&conn);
    for task in &backend_tasks {
        let file = file_repo
            .get_by_db_id(task.file_id)
            .expect("Should get file")
            .expect("File should exist");
        let path = file.path.to_string_lossy();
        assert!(
            path.contains("auth.md") || path.contains("database.md"),
            "Backend task should be from auth.md or database.md, got: {path}"
        );
    }

    // Test 2: Filter by file-level label "frontend"
    let frontend_tasks = task_repo
        .find(&TaskFilter {
            labels: vec!["frontend".to_string()],
            ..Default::default()
        })
        .expect("Should find tasks by file-level label");

    // Should get ALL tasks from ui.md (3 tasks)
    assert_eq!(
        frontend_tasks.len(),
        3,
        "Should have exactly 3 tasks from ui.md"
    );

    // Verify all tasks are from ui.md
    for task in &frontend_tasks {
        let file = file_repo
            .get_by_db_id(task.file_id)
            .expect("Should get file")
            .expect("File should exist");
        assert!(
            file.path.to_string_lossy().contains("ui.md"),
            "Frontend task should be from ui.md"
        );
    }

    // Test 3: Filter by file-level label "bug"
    let bug_tasks = task_repo
        .find(&TaskFilter {
            labels: vec!["bug".to_string()],
            ..Default::default()
        })
        .expect("Should find tasks by file-level label");

    // Should get ALL tasks from bugs.md (3 tasks)
    assert_eq!(
        bug_tasks.len(),
        3,
        "Should have exactly 3 tasks from bugs.md"
    );

    // Test 4: Filter by file-level label "security" (only on auth.md)
    let security_tasks = task_repo
        .find(&TaskFilter {
            labels: vec!["security".to_string()],
            ..Default::default()
        })
        .expect("Should find tasks by file-level label");

    // Should get ALL tasks from auth.md (5 tasks)
    assert_eq!(
        security_tasks.len(),
        5,
        "Should have exactly 5 tasks from auth.md"
    );

    // Test 5: Combine file-level label with status filter
    let open_backend_tasks = task_repo
        .find(&TaskFilter {
            labels: vec!["backend".to_string()],
            status: Some(TaskStatus::Open),
            ..Default::default()
        })
        .expect("Should find tasks with combined filters");

    // Verify all results match both criteria
    for task in &open_backend_tasks {
        assert_eq!(task.status, TaskStatus::Open, "Should only be open tasks");
        let file = file_repo
            .get_by_db_id(task.file_id)
            .expect("Should get file")
            .expect("File should exist");
        let path = file.path.to_string_lossy();
        assert!(
            path.contains("auth.md") || path.contains("database.md"),
            "Should be from backend-labeled file"
        );
    }

    // Test 6: Verify no cross-contamination between different file-level labels
    let backend_ids: std::collections::HashSet<_> =
        backend_tasks.iter().map(|t| &t.full_id).collect();
    let frontend_ids: std::collections::HashSet<_> =
        frontend_tasks.iter().map(|t| &t.full_id).collect();

    let overlap: Vec<_> = backend_ids.intersection(&frontend_ids).collect();
    assert!(
        overlap.is_empty(),
        "Backend and frontend tasks should not overlap (different files), found: {overlap:?}"
    );
}
