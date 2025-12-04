//! Test to validate database test helper functions work correctly

mod common;

use common::{assert_file_count, assert_has_file, DbInspector, TestDatabase};
use lash_db::repository::FileRepository;
use lash_types::{FileMetadata, TaskFile, TaskTree};
use std::path::PathBuf;
use std::time::SystemTime;

#[test]
fn test_in_memory_database() {
    let db = TestDatabase::in_memory();
    let conn = db.connection();

    // Verify database is initialized
    let inspector = DbInspector::new(&conn);
    assert_eq!(inspector.count_files(), 0);
    assert_eq!(inspector.count_tasks(), 0);
}

#[test]
fn test_file_based_database() {
    let db = TestDatabase::file_based();
    let conn = db.connection();

    // Verify database is initialized
    let inspector = DbInspector::new(&conn);
    assert_eq!(inspector.count_files(), 0);
    assert_eq!(inspector.count_tasks(), 0);

    // Verify path exists (in-memory or file-based)
    assert!(!db.path().as_os_str().is_empty());
}

#[test]
fn test_db_inspector_file_operations() {
    let db = TestDatabase::in_memory();
    let conn = db.connection();

    // Insert test files
    let file_repo = FileRepository::new(&conn);

    file_repo
        .insert(&TaskFile {
            path: PathBuf::from("test1.md"),
            title: "Test 1".to_string(),
            id: "test1".to_string(),
            hash: "hash1".to_string(),
            mtime: SystemTime::now(),
            metadata: FileMetadata::default(),
            tasks: TaskTree::default(),
            description: None,
            description_agent_notes: Vec::new(),
        })
        .unwrap();

    file_repo
        .insert(&TaskFile {
            path: PathBuf::from("test2.md"),
            title: "Test 2".to_string(),
            id: "test2".to_string(),
            hash: "hash2".to_string(),
            mtime: SystemTime::now(),
            metadata: FileMetadata::default(),
            tasks: TaskTree::default(),
            description: None,
            description_agent_notes: Vec::new(),
        })
        .unwrap();

    // Test inspector
    let inspector = DbInspector::new(&conn);
    assert_eq!(inspector.count_files(), 2);

    assert!(inspector.has_file("test1.md"));
    assert!(inspector.has_file("test2.md"));
    assert!(!inspector.has_file("nonexistent.md"));

    // Test file paths
    let file_paths = inspector.get_file_paths();
    assert_eq!(file_paths.len(), 2);
    assert!(file_paths.contains(&"test1.md".to_string()));
    assert!(file_paths.contains(&"test2.md".to_string()));
}

#[test]
fn test_assert_file_helpers() {
    let db = TestDatabase::in_memory();
    let conn = db.connection();

    // Insert test file
    let file_repo = FileRepository::new(&conn);

    file_repo
        .insert(&TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            hash: "hash1".to_string(),
            mtime: SystemTime::now(),
            metadata: FileMetadata::default(),
            tasks: TaskTree::default(),
            description: None,
            description_agent_notes: Vec::new(),
        })
        .unwrap();

    // Test assert helpers
    assert_file_count(&conn, 1);
    assert_has_file(&conn, "test.md");
}

#[test]
fn test_db_path_helpers() {
    let db = TestDatabase::in_memory();

    // Verify we can get a path
    assert_eq!(db.path().to_str(), Some(":memory:"));
}

#[test]
fn test_db_inspector_print_stats() {
    let db = TestDatabase::in_memory();
    let conn = db.connection();

    let inspector = DbInspector::new(&conn);

    // This should not panic
    inspector.print_stats();
}
