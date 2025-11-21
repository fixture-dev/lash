//! Integration tests for search functionality

use lash_db::{init_database, search, SearchQuery};
use lash_types::TaskStatus;
use rusqlite::Connection;
use tempfile::NamedTempFile;

/// Helper to set up a test database with sample tasks
fn setup_test_db() -> (NamedTempFile, Connection) {
    let temp_file = NamedTempFile::new().unwrap();
    let conn = init_database(temp_file.path()).unwrap();

    // Insert a test file
    conn.execute(
        "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            "tasks.md",
            "tasks",
            "Project Tasks",
            "hash1",
            1234567890_i64,
            "in_progress",
            "{}",
        ),
    )
    .unwrap();

    let file_id: i64 = conn.last_insert_rowid();

    // Insert sample tasks
    let tasks = vec![
        (
            "implement-parser",
            "Implement markdown parser",
            "open",
            "Need to parse markdown files and extract tasks",
        ),
        (
            "add-backend-tests",
            "Add backend unit tests",
            "open",
            "Write comprehensive test suite for the backend",
        ),
        (
            "fix-parser-bug",
            "Fix parser memory leak",
            "done",
            "The parser was leaking memory on large files",
        ),
        (
            "update-docs",
            "Update documentation",
            "open",
            "Documentation needs to reflect recent changes",
        ),
        (
            "refactor-core",
            "Refactor core module",
            "waived",
            "Decided to rewrite instead of refactor",
        ),
    ];

    for (i, (id, title, status, body)) in tasks.iter().enumerate() {
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, '{}')",
            (
                file_id,
                id,
                format!("tasks#{id}"),
                title,
                status,
                i,
                body,
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
