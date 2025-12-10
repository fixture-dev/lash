//! Build dependency graphs from the database
//!
//! This module provides functionality to construct in-memory dependency graphs
//! from the `SQLite` database. It queries all tasks and dependencies, then builds
//! an efficient graph representation for dependency analysis.
//!
//! # Example
//!
//! ```no_run
//! use lash_db::connection::init_database;
//! use lash_db::graph_builder::GraphBuilder;
//! use std::path::Path;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let conn = init_database(Path::new("lash.db"))?;
//! let graph = GraphBuilder::new(&conn).build()?;
//!
//! println!("Graph has {} nodes and {} edges", graph.node_count(), graph.edge_count());
//! # Ok(())
//! # }
//! ```

use lash_core::dependency::{DependencyGraph, EdgeData, NodeData};
use lash_types::DependencyKind;
use rusqlite::Connection;
use std::collections::HashMap;

use crate::error::DbResult;
use crate::repository::{DependencyRepository, FileRepository, TaskRepository};

/// Builder for constructing dependency graphs from the database
///
/// The `GraphBuilder` queries the database for all tasks and dependencies,
/// then constructs an efficient in-memory graph representation. The process is:
///
/// 1. Query all files to build file ID mappings
/// 2. Query all tasks and create graph nodes
/// 3. Query all dependencies and create graph edges
/// 4. Handle hierarchical dependencies (parent-child relationships)
///
/// # Performance
///
/// Construction is O(V + E) where V is the number of tasks and E is the number
/// of dependencies. The builder performs batched queries to minimize database round-trips.
pub struct GraphBuilder<'conn> {
    conn: &'conn Connection,
}

impl<'conn> GraphBuilder<'conn> {
    /// Create a new graph builder
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::connection::init_database;
    /// use lash_db::graph_builder::GraphBuilder;
    /// use std::path::Path;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let conn = init_database(Path::new("lash.db"))?;
    /// let builder = GraphBuilder::new(&conn);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Build the dependency graph from the database
    ///
    /// Queries all tasks and dependencies from the database and constructs
    /// a complete in-memory dependency graph.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Database queries fail
    /// - Task or file data is inconsistent
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::connection::init_database;
    /// use lash_db::graph_builder::GraphBuilder;
    /// use std::path::Path;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let conn = init_database(Path::new("lash.db"))?;
    /// let graph = GraphBuilder::new(&conn).build()?;
    ///
    /// // Use the graph for dependency analysis
    /// println!("Built graph with {} tasks", graph.node_count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(&self) -> DbResult<DependencyGraph> {
        let mut graph = DependencyGraph::new();

        // Step 1: Build file ID and path mappings (db_id -> file_id, db_id -> path)
        let file_repo = FileRepository::new(self.conn);
        let files = file_repo.list_all()?;

        let mut db_id_to_file_id: HashMap<i64, String> = HashMap::new();
        let mut db_id_to_file_path: HashMap<i64, String> = HashMap::new();
        for file in &files {
            db_id_to_file_id.insert(file.id, file.file_id.clone());
            db_id_to_file_path.insert(file.id, file.path.to_string_lossy().to_string());
        }

        // Step 2: Query all tasks and create nodes
        let task_repo = TaskRepository::new(self.conn);
        let mut db_id_to_full_id: HashMap<i64, String> = HashMap::new();
        let mut parent_relationships: Vec<(String, String)> = Vec::new();

        // We need to get tasks from all files
        for (file_db_id, file_id) in &db_id_to_file_id {
            let tasks = task_repo.get_by_file(*file_db_id)?;
            let file_path = db_id_to_file_path.get(file_db_id).cloned();

            for task in tasks {
                let full_id = task.full_id.clone();

                // Store mapping for dependency resolution
                db_id_to_full_id.insert(task.id, full_id.clone());

                // Create node with source path
                let mut node =
                    NodeData::new(task.title.clone(), task.status, file_id.clone(), task.depth);
                if let Some(ref path) = file_path {
                    node = node.with_source_path(path.clone());
                }
                graph.add_node(full_id.clone(), node);

                // Track parent relationship for hierarchy edges
                if let Some(parent_db_id) = task.parent_id {
                    if let Some(parent_full_id) = db_id_to_full_id.get(&parent_db_id) {
                        parent_relationships.push((full_id.clone(), parent_full_id.clone()));
                    }
                }
            }
        }

        // Step 3: Add hierarchy edges (parent → child)
        for (child_id, parent_id) in parent_relationships {
            // Parent depends on child in our dependency model
            let edge = EdgeData::new(DependencyKind::Hierarchy, None);
            graph.add_edge(parent_id, child_id, edge);
        }

        // Step 4: Query explicit dependencies and create edges
        let dep_repo = DependencyRepository::new(self.conn);

        for (task_db_id, from_full_id) in &db_id_to_full_id {
            let deps = dep_repo.get_dependencies(*task_db_id)?;

            for dep in deps {
                // Only process resolved dependencies
                if let Some(to_task_db_id) = dep.to_task_id {
                    if let Some(to_full_id) = db_id_to_full_id.get(&to_task_db_id) {
                        let edge = EdgeData::new(dep.kind, dep.raw_ref);
                        graph.add_edge(from_full_id.clone(), to_full_id.clone(), edge);
                    }
                }
            }
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use lash_types::{FileMetadata, Task, TaskFile, TaskMetadata, TaskStatus, TaskTree};
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tempfile::NamedTempFile;

    fn create_test_file(path: &str, file_id: &str) -> TaskFile {
        TaskFile {
            path: PathBuf::from(path),
            title: "Test File".to_string(),
            id: file_id.to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "test_hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    fn create_test_task(
        id: &str,
        title: &str,
        depth: u8,
        parent_id: Option<String>,
        order: usize,
    ) -> Task {
        Task {
            id: id.to_string(),
            title: title.to_string(),
            status: TaskStatus::Open,
            depth,
            parent_id,
            order_index: order,
            line_number: 0,
            metadata: TaskMetadata::default(),
            body: None,
        }
    }

    #[test]
    fn test_build_empty_graph() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let graph = GraphBuilder::new(&conn).build().unwrap();

        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_build_graph_with_tasks() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Insert a file
        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Insert tasks
        let task_repo = TaskRepository::new(&conn);
        task_repo
            .insert(
                &create_test_task("task1", "Task 1", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();
        task_repo
            .insert(
                &create_test_task("task2", "Task 2", 0, None, 1),
                file_db_id,
                "test",
            )
            .unwrap();

        // Build graph
        let graph = GraphBuilder::new(&conn).build().unwrap();

        assert_eq!(graph.node_count(), 2);
        assert!(graph.contains_node("test#task1"));
        assert!(graph.contains_node("test#task2"));
    }

    #[test]
    fn test_build_graph_with_hierarchy() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Insert file
        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Insert parent and child tasks
        let task_repo = TaskRepository::new(&conn);
        task_repo
            .insert(
                &create_test_task("parent", "Parent Task", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();
        task_repo
            .insert(
                &create_test_task("child", "Child Task", 1, Some("parent".to_string()), 1),
                file_db_id,
                "test",
            )
            .unwrap();

        // Build graph
        let graph = GraphBuilder::new(&conn).build().unwrap();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        // Verify edge exists (parent → child)
        let edge = graph.get_edge("test#parent", "test#child");
        assert!(edge.is_some());
        assert!(matches!(edge.unwrap().kind, DependencyKind::Hierarchy));
    }

    #[test]
    fn test_build_graph_with_explicit_dependency() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Insert file
        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Insert tasks
        let task_repo = TaskRepository::new(&conn);
        let task1_id = task_repo
            .insert(
                &create_test_task("task1", "Task 1", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();
        let task2_id = task_repo
            .insert(
                &create_test_task("task2", "Task 2", 0, None, 1),
                file_db_id,
                "test",
            )
            .unwrap();

        // Add explicit dependency: task1 depends on task2
        let dep_repo = DependencyRepository::new(&conn);
        dep_repo
            .insert(
                task1_id,
                Some(task2_id),
                &DependencyKind::ExplicitId,
                Some("test#task2"),
            )
            .unwrap();

        // Build graph
        let graph = GraphBuilder::new(&conn).build().unwrap();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        // Verify edge exists
        let edge = graph.get_edge("test#task1", "test#task2");
        assert!(edge.is_some());
        assert!(matches!(edge.unwrap().kind, DependencyKind::ExplicitId));
    }

    #[test]
    fn test_build_graph_complex() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Insert file
        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Build hierarchy:
        // parent
        //   ├── child1
        //   └── child2
        // plus explicit: child1 → child2

        let task_repo = TaskRepository::new(&conn);
        task_repo
            .insert(
                &create_test_task("parent", "Parent", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();
        let child1_id = task_repo
            .insert(
                &create_test_task("child1", "Child 1", 1, Some("parent".to_string()), 1),
                file_db_id,
                "test",
            )
            .unwrap();
        let child2_id = task_repo
            .insert(
                &create_test_task("child2", "Child 2", 1, Some("parent".to_string()), 2),
                file_db_id,
                "test",
            )
            .unwrap();

        // Add explicit dependency
        let dep_repo = DependencyRepository::new(&conn);
        dep_repo
            .insert(
                child1_id,
                Some(child2_id),
                &DependencyKind::ExplicitId,
                Some("test#child2"),
            )
            .unwrap();

        // Build graph
        let graph = GraphBuilder::new(&conn).build().unwrap();

        assert_eq!(graph.node_count(), 3);
        // 2 hierarchy edges (parent → child1, parent → child2) + 1 explicit (child1 → child2)
        assert_eq!(graph.edge_count(), 3);

        // Verify all edges
        assert!(graph.get_edge("test#parent", "test#child1").is_some());
        assert!(graph.get_edge("test#parent", "test#child2").is_some());
        assert!(graph.get_edge("test#child1", "test#child2").is_some());
    }
}
