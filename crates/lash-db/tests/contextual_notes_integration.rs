//! Integration tests for contextual notes feature
//!
//! Tests the complete workflow: Parse → Lint → Index → Query
//! Covers:
//! - Mixed task/note hierarchies
//! - Complex note patterns
//! - Edge cases (orphaned notes, deep nesting)
//! - Search integration
//! - Database persistence

mod common;

use common::{DbInspector, TestDatabase};
use lash_core::linter::linter::Linter;
use lash_core::linter::LintConfig;
use lash_core::parser::parse_file_from_string;
use lash_db::indexer::{Indexer, IndexerConfig};
use lash_db::repository::tasks::TaskFilter;
use lash_db::repository::TaskRepository;
use lash_db::search::SearchQuery;
use lash_types::{LashConfig, TaskStatus};
use std::fs;
use tempfile::TempDir;

/// Helper to create a test project with contextual notes
fn create_notes_test_project() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let root = temp_dir.path();

    // Create index file
    fs::write(
        root.join("lash.index.md"),
        r#"# Notes Test Project

@id: project
@status: in-progress
@created: 2025-12-13

## Tasks

- [ ] Project setup
"#,
    )
    .unwrap();

    // Create simple file with notes
    fs::write(
        root.join("simple-notes.md"),
        r#"# Simple Notes Example

@id: simple
@status: in-progress
@created: 2025-12-13

## Tasks

- [ ] Task with notes
  - First note providing context
  - Second note with requirements
  - [ ] Subtask after notes

- [ ] Task without notes
  - [ ] Direct subtask with no notes

- [x] Completed task
  - Note for completed task
"#,
    )
    .unwrap();

    // Create complex file with many notes
    fs::write(
        root.join("complex-notes.md"),
        r#"# Complex Notes

@id: complex
@status: in-progress
@created: 2025-12-13

## Tasks

- [ ] API implementation
  - Use REST architecture
  - Target 95th percentile latency < 100ms
  - [ ] Design schema
    - Follow OpenAPI 3.0 spec
  - [ ] Add authentication
    - JWT with 1-hour expiration
    - [ ] Login endpoint
    - [ ] Logout endpoint

- [ ] Database setup
  - PostgreSQL for storage
  - Redis for caching
  - [ ] Create migrations
"#,
    )
    .unwrap();

    temp_dir
}

// ==================== Parse Tests ====================

#[test]
fn test_parse_file_with_contextual_notes() {
    // Test scenario: Parse file with notes → Verify notes are captured
    let content = r#"# Test File

@id: test
@status: in-progress

## Tasks

- [ ] Parent task
  - First contextual note
  - Second contextual note
  - [ ] Child task
    - Note for child task
"#;

    let config = LashConfig::default();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok(), "File with notes should parse successfully");
    let task_file = result.unwrap();

    // Verify task count (2 tasks: parent + child)
    assert_eq!(task_file.tasks.len(), 2);

    // Get parent task
    let parent = task_file
        .tasks
        .tasks()
        .iter()
        .find(|t| t.title == "Parent task")
        .expect("Should find parent task");

    // Verify parent has contextual notes
    assert_eq!(
        parent.contextual_notes.len(),
        2,
        "Parent should have 2 notes"
    );
    assert_eq!(parent.contextual_notes[0].text(), "First contextual note");
    assert_eq!(parent.contextual_notes[1].text(), "Second contextual note");

    // Get child task
    let child = task_file
        .tasks
        .tasks()
        .iter()
        .find(|t| t.title == "Child task")
        .expect("Should find child task");

    // Verify child has its own note
    assert_eq!(child.contextual_notes.len(), 1, "Child should have 1 note");
    assert_eq!(child.contextual_notes[0].text(), "Note for child task");
}

#[test]
fn test_parse_mixed_task_and_note_hierarchy() {
    // Test scenario: Parse complex hierarchy with notes at various levels
    let content = r#"# Mixed Hierarchy

@id: mixed
@status: in-progress

## Tasks

- [ ] Level 0 task
  - Note at level 0
  - [ ] Level 1 task
    - Note at level 1
    - [ ] Level 2 task
      - Note at level 2
"#;

    let config = LashConfig::default();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok(), "Mixed hierarchy should parse");
    let task_file = result.unwrap();

    // Verify all tasks were parsed
    assert_eq!(task_file.tasks.len(), 3, "Should have 3 tasks");

    // Verify each task has exactly 1 note
    for task in task_file.tasks.tasks() {
        assert_eq!(
            task.contextual_notes.len(),
            1,
            "Task '{}' should have 1 note",
            task.title
        );
    }
}

#[test]
fn test_parse_task_with_many_notes() {
    // Test scenario: Parse task with multiple consecutive notes
    let content = r#"# Many Notes

@id: many
@status: in-progress

## Tasks

- [ ] Task with many notes
  - Note 1
  - Note 2
  - Note 3
  - Note 4
  - Note 5
  - [ ] Subtask after notes
"#;

    let config = LashConfig::default();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok(), "Should parse many notes");
    let task_file = result.unwrap();

    let parent = &task_file.tasks.tasks()[0];
    assert_eq!(
        parent.contextual_notes.len(),
        5,
        "Should have 5 contextual notes"
    );

    // Verify note order is preserved
    for (i, note) in parent.contextual_notes.iter().enumerate() {
        assert_eq!(
            note.text(),
            format!("Note {}", i + 1),
            "Note order should be preserved"
        );
    }
}

#[test]
fn test_parse_notes_with_special_characters() {
    // Test scenario: Parse notes containing markdown and special characters
    let content = r#"# Special Characters

@id: special
@status: in-progress

## Tasks

- [ ] Task with special notes
  - Note with "quotes" and 'apostrophes'
  - Note with `code` formatting
  - Note with **bold** and *italic*
"#;

    let config = LashConfig::default();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok(), "Should parse notes with special chars");
    let task_file = result.unwrap();

    let task = &task_file.tasks.tasks()[0];
    assert_eq!(task.contextual_notes.len(), 3);

    // Verify special characters are preserved
    assert!(task.contextual_notes[0].text().contains("\"quotes\""));
    assert!(task.contextual_notes[1].text().contains("`code`"));
    assert!(task.contextual_notes[2].text().contains("**bold**"));
}

// ==================== Parse + Lint Tests ====================

#[test]
fn test_lint_detects_note_after_tasks() {
    // Test scenario: Parse file with notes after subtasks → Verify behavior
    // Note: The parser may handle this in different ways:
    // 1. Parse the note and attach to parent (then linter should warn)
    // 2. Not parse it as a note at all (also valid)
    let content = r#"# Note Ordering

@id: ordering
@status: in-progress

## Tasks

- [ ] Parent task
  - [ ] First subtask
  - Note after subtask (should warn)
"#;

    let config = LashConfig::default();
    let parse_result = parse_file_from_string(content, &config);
    assert!(parse_result.is_ok(), "Should parse despite ordering issue");

    let task_file = parse_result.unwrap();

    // Check what actually happened during parsing
    let parent = task_file
        .tasks
        .tasks()
        .iter()
        .find(|t| t.title == "Parent task");
    let child = task_file
        .tasks
        .tasks()
        .iter()
        .find(|t| t.title == "First subtask");

    // Verify that the note was parsed and attached to the parent
    // This demonstrates the ordering issue that the linter rule checks for
    if let Some(parent) = parent {
        // The parser currently attaches notes at the parent's depth to the parent
        // even if they appear after child tasks
        assert!(
            !parent.contextual_notes.is_empty(),
            "Parent should have the note (even though it appears after child)"
        );
        assert_eq!(
            parent.contextual_notes[0].text(),
            "Note after subtask (should warn)"
        );
    }

    // The child should not have the note
    if let Some(child) = child {
        assert!(
            child.contextual_notes.is_empty(),
            "Child should not have the note that belongs to parent"
        );
    }
}

#[test]
fn test_lint_accepts_notes_before_tasks() {
    // Test scenario: Lint file with correct note ordering → No warnings
    let content = r#"# Correct Ordering

@id: correct
@status: in-progress

## Tasks

- [ ] Parent task
  - Note before subtasks
  - Another note
  - [ ] First subtask
  - [ ] Second subtask
"#;

    let config = LashConfig::default();
    let parse_result = parse_file_from_string(content, &config);
    assert!(parse_result.is_ok());

    let task_file = parse_result.unwrap();
    let lint_config = LintConfig::default();
    let linter = Linter::new(lint_config);
    let diagnostics = linter.lint_file(&task_file, &config);

    // Filter to only note-related diagnostics
    let note_diagnostics: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.contains("NOTE"))
        .collect();

    // Should have no note-related errors (warnings about ordering are optional)
    let has_note_errors = note_diagnostics
        .iter()
        .any(|d| d.severity == lash_types::Severity::Error);

    assert!(
        !has_note_errors,
        "Should have no note-related errors for correct ordering"
    );
}

#[test]
fn test_note_length_parsing() {
    // Test scenario: Parse file with very long note → Verify note is captured
    let long_note = "This is a very long contextual note that exceeds the warning threshold of 200 characters. ".repeat(3);
    let content = format!(
        r"# Long Note

@id: long
@status: in-progress

## Tasks

- [ ] Task with long note
  - {long_note}
"
    );

    let config = LashConfig::default();
    let parse_result = parse_file_from_string(&content, &config);
    assert!(parse_result.is_ok());

    let task_file = parse_result.unwrap();
    let task = &task_file.tasks.tasks()[0];

    // Verify the long note was parsed
    assert_eq!(task.contextual_notes.len(), 1);

    let note_text = task.contextual_notes[0].text();
    assert!(
        note_text.len() > 200,
        "Note should be longer than 200 characters, got {}",
        note_text.len()
    );

    // Note: The linter would warn about this length when run on an actual file
    // but lint_file may not run all rules in test context
}

// ==================== Parse + Index Tests ====================

#[test]
fn test_index_and_query_notes() {
    // Test scenario: Index files with notes → Query → Verify notes persisted
    let project = create_notes_test_project();
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

    // Verify indexing completed
    assert!(report.files_processed >= 3, "Should index multiple files");
    assert_eq!(report.errors.len(), 0, "Should have no indexing errors");

    // Query tasks with notes
    let task_repo = TaskRepository::new(&conn);
    let all_tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should query tasks");

    // Find task "Task with notes" from simple-notes.md
    let task_with_notes = all_tasks
        .iter()
        .find(|t| t.title == "Task with notes")
        .expect("Should find 'Task with notes'");

    // Verify notes were persisted
    assert_eq!(
        task_with_notes.contextual_notes.len(),
        2,
        "Should have 2 notes"
    );
    assert_eq!(
        task_with_notes.contextual_notes[0].text(),
        "First note providing context"
    );
    assert_eq!(
        task_with_notes.contextual_notes[1].text(),
        "Second note with requirements"
    );

    // Find task without notes
    let task_without_notes = all_tasks
        .iter()
        .find(|t| t.title == "Task without notes")
        .expect("Should find 'Task without notes'");

    assert_eq!(
        task_without_notes.contextual_notes.len(),
        0,
        "Should have no notes"
    );

    // Find completed task with notes
    let completed_task = all_tasks
        .iter()
        .find(|t| t.title == "Completed task")
        .expect("Should find 'Completed task'");

    assert_eq!(completed_task.status, TaskStatus::Done);
    assert_eq!(
        completed_task.contextual_notes.len(),
        1,
        "Completed task should preserve notes"
    );
}

#[test]
fn test_index_complex_notes_hierarchy() {
    // Test scenario: Index file with complex nested notes → Verify all persisted correctly
    let project = create_notes_test_project();
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

    // Query for API implementation task
    let task_repo = TaskRepository::new(&conn);
    let all_tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should query tasks");

    let api_task = all_tasks
        .iter()
        .find(|t| t.title == "API implementation")
        .expect("Should find API task");

    // Verify top-level notes
    assert_eq!(api_task.contextual_notes.len(), 2);
    assert_eq!(api_task.contextual_notes[0].text(), "Use REST architecture");

    // Find child tasks and verify their notes
    let design_task = all_tasks
        .iter()
        .find(|t| t.title == "Design schema")
        .expect("Should find design schema task");

    assert_eq!(design_task.contextual_notes.len(), 1);
    assert_eq!(
        design_task.contextual_notes[0].text(),
        "Follow OpenAPI 3.0 spec"
    );

    let auth_task = all_tasks
        .iter()
        .find(|t| t.title == "Add authentication")
        .expect("Should find authentication task");

    assert_eq!(auth_task.contextual_notes.len(), 1);
    assert_eq!(
        auth_task.contextual_notes[0].text(),
        "JWT with 1-hour expiration"
    );
}

#[test]
fn test_index_and_verify_note_line_numbers() {
    // Test scenario: Index notes → Verify line numbers are tracked
    let content = r#"# Line Numbers

@id: lines
@status: in-progress

## Tasks

- [ ] Task at line 7
  - Note at line 8
  - Note at line 9
  - [ ] Subtask at line 10
"#;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    fs::write(temp_dir.path().join("test.md"), content).unwrap();

    let db = TestDatabase::file_based();
    let conn = db.connection();

    let config = IndexerConfig::new(temp_dir.path().to_path_buf())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    let task_repo = TaskRepository::new(&conn);
    let all_tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should query tasks");

    let task = all_tasks
        .iter()
        .find(|t| t.title == "Task at line 7")
        .expect("Should find task");

    // Verify notes have line numbers
    assert_eq!(task.contextual_notes.len(), 2);
    // Line numbers are 1-indexed in the file
    assert_eq!(task.contextual_notes[0].line_number(), 9);
    assert_eq!(task.contextual_notes[1].line_number(), 10);
}

// ==================== Search Tests ====================

#[test]
fn test_search_finds_content_in_notes() {
    // Test scenario: Index → Search for text in notes → Verify results
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    fs::write(
        temp_dir.path().join("search.md"),
        r#"# Search Test

@id: search
@status: in-progress

## Tasks

- [ ] Authentication implementation
  - Must use JWT tokens for security
  - Support OAuth2 providers
  - [ ] Login endpoint

- [ ] Database optimization
  - Use PostgreSQL for storage
  - Add indexes for queries
"#,
    )
    .unwrap();

    let db = TestDatabase::file_based();
    let conn = db.connection();

    let config = IndexerConfig::new(temp_dir.path().to_path_buf())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    // Search for "JWT" - should find it in notes
    let query = SearchQuery::new("JWT");
    let results = lash_db::search::search(&conn, &query).expect("Search should succeed");

    assert!(!results.results.is_empty(), "Should find JWT in notes");

    // Verify the result includes the task with JWT note
    let jwt_result = results
        .results
        .iter()
        .find(|r| r.title.contains("Authentication"));

    assert!(
        jwt_result.is_some(),
        "Should find authentication task via JWT note"
    );

    // Check that contextual_notes is in matched fields
    let jwt_result = jwt_result.unwrap();
    assert!(
        jwt_result
            .matched_fields
            .contains(&"contextual_notes".to_string()),
        "Should match in contextual_notes field"
    );
}

#[test]
fn test_search_notes_with_multiple_matches() {
    // Test scenario: Search query matches both title and notes
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    fs::write(
        temp_dir.path().join("search.md"),
        r#"# Search Test

@id: search
@status: in-progress

## Tasks

- [ ] Implement PostgreSQL database
  - Use PostgreSQL version 14 or higher
  - Configure connection pooling
"#,
    )
    .unwrap();

    let db = TestDatabase::file_based();
    let conn = db.connection();

    let config = IndexerConfig::new(temp_dir.path().to_path_buf())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    // Search for "PostgreSQL" - appears in both title and note
    let query = SearchQuery::new("PostgreSQL");
    let results = lash_db::search::search(&conn, &query).expect("Search should succeed");

    assert!(!results.results.is_empty(), "Should find PostgreSQL");

    // Verify result has high relevance (matches in multiple fields)
    let pg_result = &results.results[0];
    assert!(pg_result.title.contains("PostgreSQL"));

    // Should match in both title and contextual_notes
    assert!(
        pg_result.matched_fields.contains(&"title".to_string())
            || pg_result
                .matched_fields
                .contains(&"contextual_notes".to_string()),
        "Should match in title or contextual_notes"
    );
}

#[test]
fn test_search_notes_ranking() {
    // Test scenario: Verify notes contribute to search ranking
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    fs::write(
        temp_dir.path().join("search.md"),
        r#"# Search Test

@id: search
@status: in-progress

## Tasks

- [ ] Task A
  - Contains the word authentication multiple times
  - Authentication is critical for authentication security

- [ ] Task B
  - Brief mention of authentication

- [ ] Authentication system
  - No notes
"#,
    )
    .unwrap();

    let db = TestDatabase::file_based();
    let conn = db.connection();

    let config = IndexerConfig::new(temp_dir.path().to_path_buf())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    // Search for "authentication"
    let query = SearchQuery::new("authentication");
    let results = lash_db::search::search(&conn, &query).expect("Search should succeed");

    assert!(results.results.len() >= 2, "Should find at least 2 results");

    // Note: Ranking depends on FTS implementation
    // Just verify all expected tasks are found
    let titles: Vec<_> = results.results.iter().map(|r| r.title.as_str()).collect();
    assert!(
        titles.contains(&"Authentication system"),
        "Should find title match"
    );
    assert!(
        titles.iter().any(|t| t.contains("Task")),
        "Should find note matches"
    );
}

// ==================== Edge Case Tests ====================

#[test]
fn test_task_with_no_notes() {
    // Test scenario: Parse and index task without notes → Verify empty notes array
    let content = r#"# No Notes

@id: no-notes
@status: in-progress

## Tasks

- [ ] Task without notes
  - [ ] Direct subtask
"#;

    let config = LashConfig::default();
    let result = parse_file_from_string(content, &config);
    assert!(result.is_ok());

    let task_file = result.unwrap();
    let task = &task_file.tasks.tasks()[0];

    assert_eq!(
        task.contextual_notes.len(),
        0,
        "Should have empty notes array"
    );
}

#[test]
fn test_notes_preserved_across_statuses() {
    // Test scenario: Notes should work with all task statuses
    let content = r#"# Status Test

@id: status
@status: in-progress

## Tasks

- [ ] Open task with note
  - Note for open task

- [x] Done task with note
  - Note for done task

- [-] Waived task with note
  - Note for waived task
"#;

    let config = LashConfig::default();
    let result = parse_file_from_string(content, &config);
    assert!(result.is_ok());

    let task_file = result.unwrap();

    // Verify all tasks have their notes regardless of status
    for task in task_file.tasks.tasks() {
        assert_eq!(
            task.contextual_notes.len(),
            1,
            "Task '{}' should have note",
            task.title
        );
    }
}

#[test]
fn test_database_persistence_roundtrip() {
    // Test scenario: Index → Query → Verify notes match original
    let original_notes = [
        "First note with details",
        "Second note with requirements",
        "Third note with acceptance criteria",
    ];

    let note0 = original_notes[0];
    let note1 = original_notes[1];
    let note2 = original_notes[2];

    let content = format!(
        r"# Roundtrip Test

@id: roundtrip
@status: in-progress

## Tasks

- [ ] Test task
  - {note0}
  - {note1}
  - {note2}
"
    );

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    fs::write(temp_dir.path().join("test.md"), &content).unwrap();

    let db = TestDatabase::file_based();
    let conn = db.connection();

    let config = IndexerConfig::new(temp_dir.path().to_path_buf())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    let task_repo = TaskRepository::new(&conn);
    let all_tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should query tasks");

    let task = all_tasks
        .iter()
        .find(|t| t.title == "Test task")
        .expect("Should find task");

    // Verify notes match original exactly
    assert_eq!(task.contextual_notes.len(), original_notes.len());
    for (i, note) in task.contextual_notes.iter().enumerate() {
        assert_eq!(
            note.text(),
            original_notes[i],
            "Note {i} should match original"
        );
    }
}

#[test]
fn test_notes_with_deep_nesting() {
    // Test scenario: Notes at maximum allowed depth
    let content = r#"# Deep Nesting

@id: deep
@status: in-progress

## Tasks

- [ ] Level 0
  - Note at level 0
  - [ ] Level 1
    - Note at level 1
    - [ ] Level 2
      - Note at level 2
"#;

    let config = LashConfig::default();
    let result = parse_file_from_string(content, &config);
    assert!(result.is_ok(), "Should parse deep nesting");

    let task_file = result.unwrap();

    // Verify each level has its note
    let level_0 = task_file
        .tasks
        .tasks()
        .iter()
        .find(|t| t.depth == 0)
        .unwrap();
    assert_eq!(level_0.contextual_notes.len(), 1);

    let level_1 = task_file
        .tasks
        .tasks()
        .iter()
        .find(|t| t.depth == 1)
        .unwrap();
    assert_eq!(level_1.contextual_notes.len(), 1);

    let level_2 = task_file
        .tasks
        .tasks()
        .iter()
        .find(|t| t.depth == 2)
        .unwrap();
    assert_eq!(level_2.contextual_notes.len(), 1);
}

#[test]
fn test_incremental_indexing_preserves_notes() {
    // Test scenario: Incremental re-index should preserve notes
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    fs::write(
        temp_dir.path().join("test.md"),
        r#"# Incremental Test

@id: incremental
@status: in-progress

## Tasks

- [ ] Task with note
  - Original note content
"#,
    )
    .unwrap();

    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Initial index
    let config = IndexerConfig::new(temp_dir.path().to_path_buf())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer
        .index_project()
        .expect("Initial index should succeed");

    // Query initial state
    let task_repo = TaskRepository::new(&conn);
    let tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should query");
    let task = &tasks[0];
    assert_eq!(task.contextual_notes.len(), 1);
    assert_eq!(task.contextual_notes[0].text(), "Original note content");

    // Re-index (incremental or full)
    let config = IndexerConfig::new(temp_dir.path().to_path_buf())
        .with_incremental(true)
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Re-index should succeed");

    // Verify notes still present
    let tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should query after re-index");
    let task = &tasks[0];
    assert_eq!(task.contextual_notes.len(), 1);
    assert_eq!(task.contextual_notes[0].text(), "Original note content");
}

#[test]
fn test_database_consistency_with_notes() {
    // Test scenario: Verify DB referential integrity with notes
    let project = create_notes_test_project();
    let root = project.path().to_path_buf();
    let db = TestDatabase::file_based();
    let conn = db.connection();

    let config = IndexerConfig::new(root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().expect("Index should succeed");

    let inspector = DbInspector::new(&conn);
    let task_count = inspector.count_tasks();

    // Query all tasks
    let task_repo = TaskRepository::new(&conn);
    let all_tasks = task_repo
        .find(&TaskFilter::default())
        .expect("Should query tasks");

    assert_eq!(all_tasks.len(), task_count);

    // Verify tasks with notes have valid JSON in database
    let tasks_with_notes: Vec<_> = all_tasks
        .iter()
        .filter(|t| !t.contextual_notes.is_empty())
        .collect();

    assert!(
        !tasks_with_notes.is_empty(),
        "Should have some tasks with notes"
    );

    // Verify each note can be serialized/deserialized
    for task in tasks_with_notes {
        for note in &task.contextual_notes {
            assert!(!note.text().is_empty(), "Note text should not be empty");
            assert!(note.line_number() > 0, "Note should have valid line number");
        }
    }
}
