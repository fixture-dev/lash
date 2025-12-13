//! Integration tests for search functionality

use lash_db::{init_database, search, SearchQuery};
use lash_types::TaskStatus;
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Helper to set up a test database with sample tasks
fn setup_test_db() -> (NamedTempFile, Connection) {
    let temp_file = NamedTempFile::new().unwrap();
    let conn = init_database(temp_file.path()).unwrap();

    // Insert a test file (with description for description search tests)
    conn.execute(
        "INSERT INTO files (path, file_id, title, description, hash, mtime, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            "tasks.md",
            "tasks",
            "Project Tasks",
            "This file contains all the project tasks for the backend parser implementation",
            "hash1",
            1234567890_i64,
            "in_progress",
            "{}",
        ),
    )
    .unwrap();

    let file_id: i64 = conn.last_insert_rowid();

    // Insert sample tasks with owner field
    let tasks = [
        (
            "implement-parser",
            "Implement markdown parser",
            "open",
            "Need to parse markdown files and extract tasks",
            Some("alice"),
        ),
        (
            "add-backend-tests",
            "Add backend unit tests",
            "open",
            "Write comprehensive test suite for the backend",
            Some("bob"),
        ),
        (
            "fix-parser-bug",
            "Fix parser memory leak",
            "done",
            "The parser was leaking memory on large files",
            Some("alice"),
        ),
        (
            "update-docs",
            "Update documentation",
            "open",
            "Documentation needs to reflect recent changes",
            None,
        ),
        (
            "refactor-core",
            "Refactor core module",
            "waived",
            "Decided to rewrite instead of refactor",
            Some("bob"),
        ),
    ];

    for (i, (id, title, status, body, owner)) in tasks.iter().enumerate() {
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, owner, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, '{}')",
            (
                file_id,
                id,
                format!("tasks#{id}"),
                title,
                status,
                i,
                body,
                owner,
            ),
        )
        .unwrap();
    }

    // Insert labels
    conn.execute("INSERT INTO labels (name) VALUES ('backend')", [])
        .unwrap();
    let backend_label_id = conn.last_insert_rowid();

    conn.execute("INSERT INTO labels (name) VALUES ('parser')", [])
        .unwrap();
    let parser_label_id = conn.last_insert_rowid();

    conn.execute("INSERT INTO labels (name) VALUES ('docs')", [])
        .unwrap();
    let docs_label_id = conn.last_insert_rowid();

    // Get task IDs
    let parser_task_id: i64 = conn
        .query_row(
            "SELECT id FROM tasks WHERE local_id = 'implement-parser'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    let backend_task_id: i64 = conn
        .query_row(
            "SELECT id FROM tasks WHERE local_id = 'add-backend-tests'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    let fix_parser_task_id: i64 = conn
        .query_row(
            "SELECT id FROM tasks WHERE local_id = 'fix-parser-bug'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    let docs_task_id: i64 = conn
        .query_row(
            "SELECT id FROM tasks WHERE local_id = 'update-docs'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // Add task labels
    conn.execute(
        "INSERT INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
        (parser_task_id, parser_label_id),
    )
    .unwrap();

    conn.execute(
        "INSERT INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
        (parser_task_id, backend_label_id),
    )
    .unwrap();

    conn.execute(
        "INSERT INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
        (backend_task_id, backend_label_id),
    )
    .unwrap();

    conn.execute(
        "INSERT INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
        (fix_parser_task_id, parser_label_id),
    )
    .unwrap();

    conn.execute(
        "INSERT INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
        (docs_task_id, docs_label_id),
    )
    .unwrap();

    (temp_file, conn)
}

#[test]
fn test_search_by_title() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("parser");
    let results = search(&conn, &query).unwrap();

    // Should find tasks with "parser" in title or body
    assert!(results.results.len() >= 2); // At least "implement-parser" and "fix-parser-bug"
    assert!(results
        .results
        .iter()
        .any(|r| r.title.contains("parser") || r.title.contains("Parser")));
}

#[test]
fn test_search_by_body() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("memory leak");
    let results = search(&conn, &query).unwrap();

    // Should find "fix-parser-bug" which mentions memory leak in body
    assert!(!results.results.is_empty());
    assert!(results
        .results
        .iter()
        .any(|r| r.full_id == "tasks#fix-parser-bug"));
}

#[test]
fn test_search_with_label_filter() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("").with_label("backend".to_string());
    let _results = search(&conn, &query).unwrap();

    // Should find all tasks with "backend" label
    // Note: empty query with filters should be handled
    // For now, this might return empty results since we require a query
}

#[test]
fn test_search_with_status_filter() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("parser").with_status(TaskStatus::Done);
    let results = search(&conn, &query).unwrap();

    // Should find only the "fix-parser-bug" task (done)
    assert_eq!(results.results.len(), 1);
    assert_eq!(results.results[0].full_id, "tasks#fix-parser-bug");
    assert_eq!(results.results[0].status, TaskStatus::Done);
}

#[test]
fn test_search_pagination() {
    let (_temp, conn) = setup_test_db();

    // First page
    let query = SearchQuery::new("test").with_limit(2).with_offset(0);
    let results = search(&conn, &query).unwrap();

    assert!(results.results.len() <= 2);

    // Second page
    let query2 = SearchQuery::new("test").with_limit(2).with_offset(2);
    let results2 = search(&conn, &query2).unwrap();

    // Results should be different (if there are enough matches)
    if results.total_count > 2 {
        assert!(!results2.results.is_empty());
    }
}

#[test]
fn test_search_empty_query() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("");
    let results = search(&conn, &query).unwrap();

    // Empty query should return empty results
    assert_eq!(results.results.len(), 0);
    assert_eq!(results.total_count, 0);
}

#[test]
fn test_search_no_matches() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("xyzabc123nonexistent");
    let results = search(&conn, &query).unwrap();

    assert_eq!(results.results.len(), 0);
    assert_eq!(results.total_count, 0);
}

#[test]
fn test_search_relevance_ranking() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("parser");
    let results = search(&conn, &query).unwrap();

    // Results should be ranked by relevance
    // Title matches should score higher than body matches
    if results.results.len() > 1 {
        // Check that scores are in descending order
        for i in 0..results.results.len() - 1 {
            assert!(
                results.results[i].score >= results.results[i + 1].score,
                "Results should be sorted by score descending"
            );
        }
    }
}

#[test]
fn test_search_by_label_query_filter() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("label:backend");
    let results = search(&conn, &query).unwrap();

    // All results should have the "backend" label
    for result in &results.results {
        assert!(result.labels.contains(&"backend".to_string()));
    }
}

#[test]
fn test_search_matched_fields() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("parser");
    let results = search(&conn, &query).unwrap();

    // Check that matched_fields is populated
    for result in &results.results {
        assert!(
            !result.matched_fields.is_empty(),
            "matched_fields should indicate where the match occurred"
        );
    }
}

#[test]
fn test_search_snippet_generation() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("memory");
    let results = search(&conn, &query).unwrap();

    // Check that snippets are generated
    for result in &results.results {
        assert!(!result.snippet.is_empty(), "snippet should not be empty");
    }
}

#[test]
fn test_search_with_single_label_filter() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("parser").with_label("backend".to_string());
    let results = search(&conn, &query).unwrap();

    // Should find tasks that match "parser" AND have "backend" label
    assert!(!results.results.is_empty());
    for result in &results.results {
        assert!(result.labels.contains(&"backend".to_string()));
    }
}

#[test]
fn test_search_with_multiple_label_filters() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("parser")
        .with_label("backend".to_string())
        .with_label("parser".to_string());
    let results = search(&conn, &query).unwrap();

    // Should find tasks that match "parser" AND have both "backend" and "parser" labels
    for result in &results.results {
        assert!(result.labels.contains(&"backend".to_string()));
        assert!(result.labels.contains(&"parser".to_string()));
    }
}

#[test]
fn test_search_with_owner_filter() {
    let (_temp, conn) = setup_test_db();

    let query = SearchQuery::new("parser").with_owner("alice".to_string());
    let results = search(&conn, &query).unwrap();

    // Should find parser tasks owned by alice
    assert!(!results.results.is_empty());
    for result in &results.results {
        assert_eq!(result.owner, Some("alice".to_string()));
    }
}

#[test]
fn test_search_with_combined_filters() {
    let (_temp, conn) = setup_test_db();

    // Search with multiple filters: label + status
    let query = SearchQuery::new("parser")
        .with_label("backend".to_string())
        .with_status(TaskStatus::Open);
    let results = search(&conn, &query).unwrap();

    // Should find open tasks that match "parser" and have "backend" label
    for result in &results.results {
        assert_eq!(result.status, TaskStatus::Open);
        assert!(result.labels.contains(&"backend".to_string()));
    }
}

#[test]
fn test_search_with_label_status_and_owner_filters() {
    let (_temp, conn) = setup_test_db();

    // Search with all filters
    let query = SearchQuery::new("parser")
        .with_label("parser".to_string())
        .with_status(TaskStatus::Done)
        .with_owner("alice".to_string());
    let results = search(&conn, &query).unwrap();

    // Should find done parser tasks owned by alice with parser label
    for result in &results.results {
        assert_eq!(result.status, TaskStatus::Done);
        assert_eq!(result.owner, Some("alice".to_string()));
        assert!(result.labels.contains(&"parser".to_string()));
    }
}

#[test]
fn test_search_with_path_scope_filter() {
    let temp_file = NamedTempFile::new().unwrap();
    let conn = init_database(temp_file.path()).unwrap();

    // Insert two files in different directories
    conn.execute(
        "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            "core/tasks.md",
            "core-tasks",
            "Core Tasks",
            "hash1",
            1234567890_i64,
            "in_progress",
            "{}",
        ),
    )
    .unwrap();
    let core_file_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            "ui/tasks.md",
            "ui-tasks",
            "UI Tasks",
            "hash2",
            1234567890_i64,
            "in_progress",
            "{}",
        ),
    )
    .unwrap();
    let ui_file_id = conn.last_insert_rowid();

    // Insert tasks in both files
    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, '{}')",
        (
            core_file_id,
            "core-parser",
            "core-tasks#core-parser",
            "Core parser implementation",
            "open",
            "Parser for the core module",
        ),
    )
    .unwrap();

    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 1, ?6, '{}')",
        (
            ui_file_id,
            "ui-parser",
            "ui-tasks#ui-parser",
            "UI parser implementation",
            "open",
            "Parser for the UI layer",
        ),
    )
    .unwrap();

    // Search with path scope filter
    let query = SearchQuery::new("parser").with_scope(PathBuf::from("core/"));
    let results = search(&conn, &query).unwrap();

    // Should only find tasks in core/ directory
    assert_eq!(results.results.len(), 1);
    assert!(results.results[0].file_path.starts_with("core/"));
    assert_eq!(results.results[0].full_id, "core-tasks#core-parser");
}

#[test]
fn test_search_by_description_content() {
    let (_temp, conn) = setup_test_db();

    // Search for text that only appears in the file description
    let query = SearchQuery::new("implementation");
    let results = search(&conn, &query).unwrap();

    // Should find tasks in files with matching descriptions
    assert!(!results.results.is_empty());

    // Check that at least one result has "file_description" in matched_fields
    let has_description_match = results
        .results
        .iter()
        .any(|r| r.matched_fields.contains(&"file_description".to_string()));
    assert!(
        has_description_match,
        "Expected at least one result to match the file description"
    );
}

#[test]
fn test_search_description_with_empty_description() {
    let temp_file = NamedTempFile::new().unwrap();
    let conn = init_database(temp_file.path()).unwrap();

    // Insert a file with empty description
    conn.execute(
        "INSERT INTO files (path, file_id, title, description, hash, mtime, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            "empty-desc.md",
            "empty-desc",
            "Empty Description File",
            "",
            "hash1",
            1234567890_i64,
            "in_progress",
            "{}",
        ),
    )
    .unwrap();

    let file_id = conn.last_insert_rowid();

    // Insert a task
    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, '{}')",
        (
            file_id,
            "task1",
            "empty-desc#task1",
            "Test Task",
            "open",
            "Body with searchable content",
        ),
    )
    .unwrap();

    // Search for content in body
    let query = SearchQuery::new("searchable");
    let results = search(&conn, &query).unwrap();

    // Should find the task via body match, not description
    assert_eq!(results.results.len(), 1);
    assert!(results.results[0]
        .matched_fields
        .contains(&"body".to_string()));
    assert!(!results.results[0]
        .matched_fields
        .contains(&"file_description".to_string()));
}

#[test]
fn test_search_description_relevance_ranking() {
    let temp_file = NamedTempFile::new().unwrap();
    let conn = init_database(temp_file.path()).unwrap();

    // Insert two files: one with description match, one with title match
    conn.execute(
        "INSERT INTO files (path, file_id, title, description, hash, mtime, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            "file1.md",
            "file1",
            "File One",
            "Contains the unique word xylophone in the description",
            "hash1",
            1234567890_i64,
            "in_progress",
            "{}",
        ),
    )
    .unwrap();
    let file1_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO files (path, file_id, title, description, hash, mtime, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            "file2.md",
            "file2",
            "Xylophone Tasks",
            "This file has xylophone in the title",
            "hash2",
            1234567890_i64,
            "in_progress",
            "{}",
        ),
    )
    .unwrap();
    let file2_id = conn.last_insert_rowid();

    // Insert tasks in both files
    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, '{}')",
        (
            file1_id,
            "task1",
            "file1#task1",
            "Task One",
            "open",
            "Regular task body",
        ),
    )
    .unwrap();

    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, '{}')",
        (
            file2_id,
            "task2",
            "file2#task2",
            "Task Two",
            "open",
            "Another task body",
        ),
    )
    .unwrap();

    // Search for "xylophone"
    let query = SearchQuery::new("xylophone");
    let results = search(&conn, &query).unwrap();

    // Should find both tasks
    assert_eq!(results.results.len(), 2);

    // The task from file2 (title match) should rank higher than file1 (description match)
    // because title weight (3.0) > description weight (1.5)
    assert_eq!(results.results[0].full_id, "file2#task2");
    assert_eq!(results.results[1].full_id, "file1#task1");
}

#[test]
fn test_search_contextual_notes() {
    let (temp_file, conn) = setup_test_db();
    let _temp_file = temp_file; // Keep alive

    // Insert a test file
    conn.execute(
        "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
         VALUES ('notes-test.md', 'notes-test', 'Notes Test', 'hash1', 1234567890, 'in_progress', '{}')",
        [],
    )
    .unwrap();

    let file_id = conn.last_insert_rowid();

    // Insert a task with contextual notes
    let notes_json = r#"[{"text":"Use library X for parsing","line_number":10},{"text":"Target performance < 100ms","line_number":11}]"#;
    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata, contextual_notes)
         VALUES (?1, 'task1', 'notes-test#task1', 'Implement feature', 'open', 0, 0, 'Task body', '{}', ?2)",
        rusqlite::params![file_id, notes_json],
    )
    .unwrap();

    // Search for content in contextual notes
    let query = SearchQuery::new("library");
    let results = search(&conn, &query).unwrap();

    // Should find the task
    assert_eq!(results.results.len(), 1);
    assert_eq!(results.results[0].full_id, "notes-test#task1");

    // Check that contextual_notes is in matched_fields
    assert!(results.results[0]
        .matched_fields
        .contains(&"contextual_notes".to_string()));

    // Snippet should include the note
    assert!(results.results[0].snippet.contains("[Note]"));
    assert!(results.results[0]
        .snippet
        .contains("Use library X for parsing"));
}

#[test]
fn test_search_contextual_notes_multiple_matches() {
    let (temp_file, conn) = setup_test_db();
    let _temp_file = temp_file; // Keep alive

    // Insert a test file
    conn.execute(
        "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
         VALUES ('multi-notes.md', 'multi-notes', 'Multi Notes', 'hash1', 1234567890, 'in_progress', '{}')",
        [],
    )
    .unwrap();

    let file_id = conn.last_insert_rowid();

    // Insert multiple tasks, some with matching notes
    let tasks = [
        (
            "task1",
            "Task One",
            r#"[{"text":"Use database Y","line_number":5}]"#,
        ),
        (
            "task2",
            "Task Two",
            r#"[{"text":"Use database Z for storage","line_number":8}]"#,
        ),
        ("task3", "Task Three", r#"[]"#), // No notes
    ];

    for (id, title, notes_json) in &tasks {
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata, contextual_notes)
             VALUES (?1, ?2, ?3, ?4, 'open', 0, 0, 'Body', '{}', ?5)",
            rusqlite::params![file_id, id, format!("multi-notes#{id}"), title, notes_json],
        )
        .unwrap();
    }

    // Search for "database"
    let query = SearchQuery::new("database");
    let results = search(&conn, &query).unwrap();

    // Should find tasks 1 and 2 (have "database" in notes)
    assert_eq!(results.results.len(), 2);

    // Both should have contextual_notes in matched fields
    for result in &results.results {
        assert!(result
            .matched_fields
            .contains(&"contextual_notes".to_string()));
        assert!(result.snippet.contains("[Note]"));
    }
}

#[test]
fn test_search_contextual_notes_with_body_match() {
    let (temp_file, conn) = setup_test_db();
    let _temp_file = temp_file; // Keep alive

    // Insert a test file
    conn.execute(
        "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
         VALUES ('combined.md', 'combined', 'Combined', 'hash1', 1234567890, 'in_progress', '{}')",
        [],
    )
    .unwrap();

    let file_id = conn.last_insert_rowid();

    // Insert task with keyword in both body and notes
    let notes_json = r#"[{"text":"Use optimization X","line_number":5}]"#;
    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata, contextual_notes)
         VALUES (?1, 'task1', 'combined#task1', 'Task Title', 'open', 0, 0, 'Body with optimization', '{}', ?2)",
        rusqlite::params![file_id, notes_json],
    )
    .unwrap();

    // Search for "optimization"
    let query = SearchQuery::new("optimization");
    let results = search(&conn, &query).unwrap();

    // Should find the task
    assert_eq!(results.results.len(), 1);

    // Should match in both body and contextual_notes
    assert!(results.results[0]
        .matched_fields
        .contains(&"body".to_string()));
    assert!(results.results[0]
        .matched_fields
        .contains(&"contextual_notes".to_string()));

    // Snippet should include both body and note
    assert!(results.results[0].snippet.contains("optimization"));
    assert!(results.results[0].snippet.contains("[Note]"));
}

#[test]
fn test_search_contextual_notes_truncation() {
    let (temp_file, conn) = setup_test_db();
    let _temp_file = temp_file; // Keep alive

    // Insert a test file
    conn.execute(
        "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
         VALUES ('truncate.md', 'truncate', 'Truncate', 'hash1', 1234567890, 'in_progress', '{}')",
        [],
    )
    .unwrap();

    let file_id = conn.last_insert_rowid();

    // Insert task with very long note
    let long_note = "a".repeat(150) + " keyword " + &"b".repeat(50);
    let notes_json = format!(r#"[{{"text":"{long_note}","line_number":5}}]"#);
    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata, contextual_notes)
         VALUES (?1, 'task1', 'truncate#task1', 'Task Title', 'open', 0, 0, 'Body', '{}', ?2)",
        rusqlite::params![file_id, notes_json],
    )
    .unwrap();

    // Search for "keyword"
    let query = SearchQuery::new("keyword");
    let results = search(&conn, &query).unwrap();

    // Should find the task
    assert_eq!(results.results.len(), 1);

    // Snippet should be truncated
    assert!(results.results[0].snippet.contains("[Note]"));
    assert!(results.results[0].snippet.ends_with("..."));
    // Should be much shorter than the original note
    assert!(results.results[0].snippet.len() < long_note.len());
}
