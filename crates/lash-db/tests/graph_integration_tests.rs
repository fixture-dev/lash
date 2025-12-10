//! Integration tests for dependency graph construction and querying

use lash_db::repository::{DependencyRepository, FileRepository, TaskRepository};
use lash_db::{init_database, GraphBuilder};
use lash_types::{
    DependencyKind, FileMetadata, Task, TaskFile, TaskMetadata, TaskStatus, TaskTree,
};
use std::path::PathBuf;
use std::time::SystemTime;
use tempfile::NamedTempFile;

fn create_test_file(path: &str, file_id: &str) -> TaskFile {
    TaskFile {
        path: PathBuf::from(path),
        title: "Test File".to_string(),
        id: file_id.to_string(),
        metadata: FileMetadata::default(),
        tasks: TaskTree::new(),
        hash: "test_hash".to_string(),
        mtime: SystemTime::now(),
        description: None,
        description_agent_notes: Vec::new(),
    }
}

fn create_test_task(
    id: &str,
    title: &str,
    depth: u8,
    parent_id: Option<String>,
    order: usize,
    status: TaskStatus,
) -> Task {
    Task {
        id: id.to_string(),
        title: title.to_string(),
        status,
        depth,
        parent_id,
        order_index: order,
        line_number: 0,
        metadata: TaskMetadata::default(),
        body: None,
    }
}

#[test]
fn test_build_and_query_simple_graph() {
    let temp_db = NamedTempFile::new().unwrap();
    let conn = init_database(temp_db.path()).unwrap();

    // Set up database
    let file = create_test_file("test.md", "test");
    let file_repo = FileRepository::new(&conn);
    let file_db_id = file_repo.insert(&file).unwrap();

    let task_repo = TaskRepository::new(&conn);
    task_repo
        .insert(
            &create_test_task("task1", "Task 1", 0, None, 0, TaskStatus::Open),
            file_db_id,
            "test",
        )
        .unwrap();
    task_repo
        .insert(
            &create_test_task("task2", "Task 2", 0, None, 1, TaskStatus::Done),
            file_db_id,
            "test",
        )
        .unwrap();

    // Build graph
    let graph = GraphBuilder::new(&conn).build().unwrap();

    assert_eq!(graph.node_count(), 2);
    assert!(graph.contains_node("test#task1"));
    assert!(graph.contains_node("test#task2"));

    // Query node metadata
    let node = graph.get_node("test#task1").unwrap();
    assert_eq!(node.title, "Task 1");
    assert_eq!(node.status, TaskStatus::Open);

    let node = graph.get_node("test#task2").unwrap();
    assert_eq!(node.title, "Task 2");
    assert_eq!(node.status, TaskStatus::Done);
}

#[test]
fn test_hierarchy_dependencies() {
    let temp_db = NamedTempFile::new().unwrap();
    let conn = init_database(temp_db.path()).unwrap();

    // Set up database with hierarchy
    let file = create_test_file("test.md", "test");
    let file_repo = FileRepository::new(&conn);
    let file_db_id = file_repo.insert(&file).unwrap();

    let task_repo = TaskRepository::new(&conn);
    task_repo
        .insert(
            &create_test_task("parent", "Parent Task", 0, None, 0, TaskStatus::Open),
            file_db_id,
            "test",
        )
        .unwrap();
    task_repo
        .insert(
            &create_test_task(
                "child1",
                "Child 1",
                1,
                Some("parent".to_string()),
                1,
                TaskStatus::Done,
            ),
            file_db_id,
            "test",
        )
        .unwrap();
    task_repo
        .insert(
            &create_test_task(
                "child2",
                "Child 2",
                1,
                Some("parent".to_string()),
                2,
                TaskStatus::Open,
            ),
            file_db_id,
            "test",
        )
        .unwrap();

    // Build graph
    let graph = GraphBuilder::new(&conn).build().unwrap();

    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 2); // parent → child1, parent → child2

    // Test dependency queries
    let deps = graph.get_dependency_ids("test#parent");
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&"test#child1".to_string()));
    assert!(deps.contains(&"test#child2".to_string()));

    // Test filter by kind
    let hierarchy_deps = graph.get_dependencies_by_kind("test#parent", &DependencyKind::Hierarchy);
    assert_eq!(hierarchy_deps.len(), 2);
}

#[test]
fn test_explicit_dependencies() {
    let temp_db = NamedTempFile::new().unwrap();
    let conn = init_database(temp_db.path()).unwrap();

    // Set up database
    let file = create_test_file("test.md", "test");
    let file_repo = FileRepository::new(&conn);
    let file_db_id = file_repo.insert(&file).unwrap();

    let task_repo = TaskRepository::new(&conn);
    let task1_id = task_repo
        .insert(
            &create_test_task("task1", "Task 1", 0, None, 0, TaskStatus::Open),
            file_db_id,
            "test",
        )
        .unwrap();
    let task2_id = task_repo
        .insert(
            &create_test_task("task2", "Task 2", 0, None, 1, TaskStatus::Open),
            file_db_id,
            "test",
        )
        .unwrap();
    let task3_id = task_repo
        .insert(
            &create_test_task("task3", "Task 3", 0, None, 2, TaskStatus::Done),
            file_db_id,
            "test",
        )
        .unwrap();

    // Add explicit dependencies: task1 → task2 → task3
    let dep_repo = DependencyRepository::new(&conn);
    dep_repo
        .insert(
            task1_id,
            Some(task2_id),
            &DependencyKind::ExplicitId,
            Some("test#task2"),
        )
        .unwrap();
    dep_repo
        .insert(
            task2_id,
            Some(task3_id),
            &DependencyKind::ExplicitId,
            Some("test#task3"),
        )
        .unwrap();

    // Build graph
    let graph = GraphBuilder::new(&conn).build().unwrap();

    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 2);

    // Test direct dependencies
    let deps = graph.get_dependency_ids("test#task1");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0], "test#task2");

    // Test transitive dependencies
    let descendants = graph.get_descendants("test#task1").unwrap();
    assert_eq!(descendants.len(), 2);
    assert!(descendants.contains(&"test#task2".to_string()));
    assert!(descendants.contains(&"test#task3".to_string()));

    // Test reverse transitive (ancestors)
    let ancestors = graph.get_ancestors("test#task3").unwrap();
    assert_eq!(ancestors.len(), 2);
    assert!(ancestors.contains(&"test#task2".to_string()));
    assert!(ancestors.contains(&"test#task1".to_string()));
}

#[test]
fn test_mixed_dependencies() {
    let temp_db = NamedTempFile::new().unwrap();
    let conn = init_database(temp_db.path()).unwrap();

    // Set up database with both hierarchy and explicit dependencies
    let file = create_test_file("test.md", "test");
    let file_repo = FileRepository::new(&conn);
    let file_db_id = file_repo.insert(&file).unwrap();

    let task_repo = TaskRepository::new(&conn);
    task_repo
        .insert(
            &create_test_task("parent", "Parent", 0, None, 0, TaskStatus::Open),
            file_db_id,
            "test",
        )
        .unwrap();
    let child_id = task_repo
        .insert(
            &create_test_task(
                "child",
                "Child",
                1,
                Some("parent".to_string()),
                1,
                TaskStatus::Open,
            ),
            file_db_id,
            "test",
        )
        .unwrap();
    let other_id = task_repo
        .insert(
            &create_test_task("other", "Other", 0, None, 2, TaskStatus::Open),
            file_db_id,
            "test",
        )
        .unwrap();

    // Add explicit dependency: child → other
    let dep_repo = DependencyRepository::new(&conn);
    dep_repo
        .insert(
            child_id,
            Some(other_id),
            &DependencyKind::ExplicitId,
            Some("test#other"),
        )
        .unwrap();

    // Build graph
    let graph = GraphBuilder::new(&conn).build().unwrap();

    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 2); // parent → child (hierarchy), child → other (explicit)

    // parent transitively depends on both child and other
    let descendants = graph.get_descendants("test#parent").unwrap();
    assert_eq!(descendants.len(), 2);
    assert!(descendants.contains(&"test#child".to_string()));
    assert!(descendants.contains(&"test#other".to_string()));

    // Filter by kind
    let hierarchy = graph.get_dependencies_by_kind("test#parent", &DependencyKind::Hierarchy);
    assert_eq!(hierarchy.len(), 1);
    assert_eq!(hierarchy[0], "test#child");

    let explicit = graph.get_dependencies_by_kind("test#child", &DependencyKind::ExplicitId);
    assert_eq!(explicit.len(), 1);
    assert_eq!(explicit[0], "test#other");
}

#[test]
fn test_depth_limited_query() {
    let temp_db = NamedTempFile::new().unwrap();
    let conn = init_database(temp_db.path()).unwrap();

    // Set up long chain: task1 → task2 → task3 → task4
    let file = create_test_file("test.md", "test");
    let file_repo = FileRepository::new(&conn);
    let file_db_id = file_repo.insert(&file).unwrap();

    let task_repo = TaskRepository::new(&conn);
    let task1_id = task_repo
        .insert(
            &create_test_task("task1", "Task 1", 0, None, 0, TaskStatus::Open),
            file_db_id,
            "test",
        )
        .unwrap();
    let task2_id = task_repo
        .insert(
            &create_test_task("task2", "Task 2", 0, None, 1, TaskStatus::Open),
            file_db_id,
            "test",
        )
        .unwrap();
    let task3_id = task_repo
        .insert(
            &create_test_task("task3", "Task 3", 0, None, 2, TaskStatus::Open),
            file_db_id,
            "test",
        )
        .unwrap();
    let task4_id = task_repo
        .insert(
            &create_test_task("task4", "Task 4", 0, None, 3, TaskStatus::Open),
            file_db_id,
            "test",
        )
        .unwrap();

    let dep_repo = DependencyRepository::new(&conn);
    dep_repo
        .insert(
            task1_id,
            Some(task2_id),
            &DependencyKind::ExplicitId,
            Some("test#task2"),
        )
        .unwrap();
    dep_repo
        .insert(
            task2_id,
            Some(task3_id),
            &DependencyKind::ExplicitId,
            Some("test#task3"),
        )
        .unwrap();
    dep_repo
        .insert(
            task3_id,
            Some(task4_id),
            &DependencyKind::ExplicitId,
            Some("test#task4"),
        )
        .unwrap();

    // Build graph
    let graph = GraphBuilder::new(&conn).build().unwrap();

    // Test depth-limited queries
    let deps_depth1 = graph.get_descendants_with_depth("test#task1", 1).unwrap();
    assert_eq!(deps_depth1.len(), 1);
    assert_eq!(deps_depth1[0], "test#task2");

    let deps_depth2 = graph.get_descendants_with_depth("test#task1", 2).unwrap();
    assert_eq!(deps_depth2.len(), 2);
    assert!(deps_depth2.contains(&"test#task2".to_string()));
    assert!(deps_depth2.contains(&"test#task3".to_string()));

    let deps_depth3 = graph.get_descendants_with_depth("test#task1", 3).unwrap();
    assert_eq!(deps_depth3.len(), 3);

    // Full transitive should get all 3
    let deps_full = graph.get_descendants("test#task1").unwrap();
    assert_eq!(deps_full.len(), 3);
}
