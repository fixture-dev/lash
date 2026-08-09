//! Dependency update module for incremental dependency re-resolution
//!
//! This module handles the incremental updating of dependency edges when files change.
//! It ensures that dependency relationships remain consistent with the current state
//! of task files without requiring full graph recomputation.
//!
//! # Key Capabilities
//!
//! - Insert hierarchy dependencies (parent-child relationships)
//! - Detect which tasks are affected by file changes
//! - Selectively delete stale dependency edges
//! - Re-resolve dependencies for modified tasks
//! - Batch database operations for efficiency
//! - Handle broken dependency references gracefully
//!
//! # Example
//!
//! ```no_run
//! use lash_db::dependency_updater::DependencyUpdater;
//! use lash_db::connection::init_database;
//! use std::path::PathBuf;
//!
//! let conn = init_database(&PathBuf::from("/tmp/lash.db"))?;
//! let updater = DependencyUpdater::new(&conn);
//!
//! // Insert hierarchy dependencies for a newly indexed file
//! let file_db_id = 1;
//! updater.insert_hierarchy_dependencies(file_db_id)?;
//! # Ok::<(), lash_db::DbError>(())
//! ```

use crate::error::DbResult;
use crate::repository::{DependencyRepository, TaskRepository};
use lash_types::DependencyKind;
use rusqlite::Connection;

/// Dependency updater for incremental dependency re-resolution
///
/// Manages the updating of dependency edges when files are modified, ensuring
/// that the dependency graph remains consistent without full recomputation.
pub struct DependencyUpdater<'conn> {
    conn: &'conn Connection,
}

impl<'conn> DependencyUpdater<'conn> {
    /// Create a new dependency updater
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::dependency_updater::DependencyUpdater;
    /// use lash_db::connection::init_database;
    /// use std::path::PathBuf;
    ///
    /// let conn = init_database(&PathBuf::from("/tmp/lash.db"))?;
    /// let updater = DependencyUpdater::new(&conn);
    /// # Ok::<(), lash_db::DbError>(())
    /// ```
    #[must_use]
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Insert hierarchy dependencies for all tasks in a file
    ///
    /// This creates `hierarchy` dependency edges for parent-child task relationships.
    /// It should be called after tasks are inserted/updated for a file.
    ///
    /// # Algorithm
    ///
    /// 1. Query all tasks in the file
    /// 2. For each task with a `parent_id`:
    ///    - Insert a hierarchy dependency edge from child to parent
    /// 3. Batch insertions for efficiency
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Database query fails
    /// - Dependency insertion fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::dependency_updater::DependencyUpdater;
    /// use lash_db::connection::init_database;
    /// use std::path::PathBuf;
    ///
    /// let conn = init_database(&PathBuf::from("/tmp/lash.db"))?;
    /// let updater = DependencyUpdater::new(&conn);
    ///
    /// // After indexing a file with database ID 1
    /// let file_db_id = 1;
    /// updater.insert_hierarchy_dependencies(file_db_id)?;
    /// # Ok::<(), lash_db::DbError>(())
    /// ```
    pub fn insert_hierarchy_dependencies(&self, file_db_id: i64) -> DbResult<()> {
        let task_repo = TaskRepository::new(self.conn);
        let dep_repo = DependencyRepository::new(self.conn);

        // Get all tasks in this file
        let tasks = task_repo.get_by_file(file_db_id)?;

        // Insert hierarchy dependency for each task with a parent
        for task in &tasks {
            if let Some(parent_id) = task.parent_id {
                // Insert hierarchy dependency: child depends on parent
                // This creates an edge from child -> parent
                dep_repo.insert(
                    task.id,         // from_task_id (child)
                    Some(parent_id), // to_task_id (parent)
                    &DependencyKind::Hierarchy,
                    None, // no raw_ref for hierarchy deps
                )?;
            }
        }

        Ok(())
    }

    /// Delete all dependencies involving tasks from specific files
    ///
    /// This removes all dependency edges where either the source or target task
    /// belongs to one of the specified files. Used before re-resolving dependencies
    /// for modified files.
    ///
    /// # Errors
    ///
    /// Returns error if database operations fail
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::dependency_updater::DependencyUpdater;
    /// use lash_db::connection::init_database;
    /// use std::path::PathBuf;
    ///
    /// let conn = init_database(&PathBuf::from("/tmp/lash.db"))?;
    /// let updater = DependencyUpdater::new(&conn);
    ///
    /// // Delete dependencies for files with IDs 1 and 2
    /// updater.delete_dependencies_for_files(&[1, 2])?;
    /// # Ok::<(), lash_db::DbError>(())
    /// ```
    pub fn delete_dependencies_for_files(&self, file_db_ids: &[i64]) -> DbResult<usize> {
        if file_db_ids.is_empty() {
            return Ok(0);
        }

        // Build placeholders for SQL IN clause
        let placeholders = file_db_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        // Delete dependencies where from_task is in one of these files
        let sql = format!(
            "DELETE FROM dependencies
             WHERE from_task_id IN (
                 SELECT id FROM tasks WHERE file_id IN ({placeholders})
             )"
        );

        let params: Vec<&dyn rusqlite::ToSql> = file_db_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let deleted = self.conn.execute(&sql, params.as_slice())?;

        Ok(deleted)
    }

    /// Delete dependencies involving specific tasks
    ///
    /// Removes all dependency edges where the specified tasks are either
    /// the source (`from_task_id`) or target (`to_task_id`).
    ///
    /// # Errors
    ///
    /// Returns error if database operations fail
    pub fn delete_dependencies_for_tasks(&self, task_db_ids: &[i64]) -> DbResult<usize> {
        if task_db_ids.is_empty() {
            return Ok(0);
        }

        let placeholders = task_db_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        // Delete dependencies where task is either source or target
        let sql = format!(
            "DELETE FROM dependencies
             WHERE from_task_id IN ({placeholders}) OR to_task_id IN ({placeholders})"
        );

        // Duplicate the parameters for both IN clauses
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(task_db_ids.len() * 2);
        for id in task_db_ids {
            params.push(id as &dyn rusqlite::ToSql);
        }
        for id in task_db_ids {
            params.push(id as &dyn rusqlite::ToSql);
        }

        let deleted = self.conn.execute(&sql, params.as_slice())?;

        Ok(deleted)
    }

    /// Update dependencies for a set of modified files
    ///
    /// This is the main orchestration method that:
    /// 1. Identifies affected tasks
    /// 2. Deletes stale dependency edges
    /// 3. Re-inserts hierarchy dependencies
    /// 4. Rebuilds the transitive closure
    ///
    /// # Errors
    ///
    /// Returns error if any database operation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::dependency_updater::DependencyUpdater;
    /// use lash_db::connection::init_database;
    /// use std::path::PathBuf;
    ///
    /// let conn = init_database(&PathBuf::from("/tmp/lash.db"))?;
    /// let updater = DependencyUpdater::new(&conn);
    ///
    /// // Update dependencies for modified files
    /// let modified_file_ids = vec![1, 2, 3];
    /// updater.update_dependencies_for_files(&modified_file_ids)?;
    /// # Ok::<(), lash_db::DbError>(())
    /// ```
    pub fn update_dependencies_for_files(&self, file_db_ids: &[i64]) -> DbResult<()> {
        if file_db_ids.is_empty() {
            return Ok(());
        }

        // Use a transaction for atomicity
        let tx = self.conn.unchecked_transaction()?;

        // Phase 1: Delete stale dependencies for these files
        let placeholders = file_db_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let delete_sql = format!(
            "DELETE FROM dependencies
             WHERE from_task_id IN (
                 SELECT id FROM tasks WHERE file_id IN ({placeholders})
             )"
        );

        let params: Vec<&dyn rusqlite::ToSql> = file_db_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        tx.execute(&delete_sql, params.as_slice())?;

        // Phase 2: Re-insert hierarchy dependencies for each file
        let task_repo = TaskRepository::new(&tx);
        let dep_repo = DependencyRepository::new(&tx);

        for &file_db_id in file_db_ids {
            let tasks = task_repo.get_by_file(file_db_id)?;

            for task in &tasks {
                if let Some(parent_id) = task.parent_id {
                    dep_repo.insert(task.id, Some(parent_id), &DependencyKind::Hierarchy, None)?;
                }
            }
        }

        // Phase 3: Rebuild transitive closure
        // Do this inline to avoid nested transaction issues
        tx.execute("DELETE FROM dependency_closure", [])?;

        // Insert direct dependencies (depth = 1)
        tx.execute(
            "INSERT INTO dependency_closure (ancestor_id, descendant_id, depth)
             SELECT to_task_id, from_task_id, 1
             FROM dependencies
             WHERE to_task_id IS NOT NULL",
            [],
        )?;

        // Build transitive closure using recursive CTE
        tx.execute(
            "INSERT INTO dependency_closure (ancestor_id, descendant_id, depth)
             WITH RECURSIVE transitive(ancestor_id, descendant_id, depth) AS (
                 SELECT ancestor_id, descendant_id, depth FROM dependency_closure
                 UNION
                 SELECT t.ancestor_id, dc.descendant_id, t.depth + 1
                 FROM transitive t
                 JOIN dependency_closure dc ON t.descendant_id = dc.ancestor_id
                 WHERE t.depth < 100  -- Prevent infinite loops
             )
             SELECT DISTINCT ancestor_id, descendant_id, MIN(depth)
             FROM transitive
             WHERE (ancestor_id, descendant_id) NOT IN (SELECT ancestor_id, descendant_id FROM dependency_closure)
             GROUP BY ancestor_id, descendant_id",
            [],
        )?;

        tx.commit()?;

        Ok(())
    }

    /// Get statistics about dependency updates
    ///
    /// Returns a tuple of (`total_dependencies`, `hierarchy_dependencies`, `explicit_dependencies`)
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_dependency_stats(&self) -> DbResult<(usize, usize, usize)> {
        let total: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM dependencies", [], |row| row.get(0))?;

        let hierarchy: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM dependencies WHERE kind = 'hierarchy'",
            [],
            |row| row.get(0),
        )?;

        let explicit = total - hierarchy;

        Ok((total, hierarchy, explicit))
    }

    /// Verify that all hierarchy dependencies are present
    ///
    /// Checks that every task with a `parent_id` has a corresponding hierarchy dependency.
    /// Returns the number of missing hierarchy dependencies found.
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn verify_hierarchy_dependencies(&self) -> DbResult<usize> {
        let missing: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks t
             WHERE t.parent_id IS NOT NULL
             AND NOT EXISTS (
                 SELECT 1 FROM dependencies d
                 WHERE d.from_task_id = t.id
                 AND d.to_task_id = t.parent_id
                 AND d.kind = 'hierarchy'
             )",
            [],
            |row| row.get(0),
        )?;

        Ok(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use crate::repository::FileRepository;
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

    fn create_test_task(id: &str, title: &str, depth: u8, parent_id: Option<&str>) -> Task {
        Task {
            id: id.to_string(),
            has_explicit_id: false,
            title: title.to_string(),
            status: TaskStatus::Open,
            depth,
            parent_id: parent_id.map(String::from),
            order_index: 0,
            line_number: 0,
            annotation_line_count: 0,
            metadata: TaskMetadata::default(),
            body: None,
            contextual_notes: Vec::new(),
        }
    }

    #[test]
    fn test_insert_hierarchy_dependencies_no_parents() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Create file
        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Insert flat tasks (no parents)
        let task1 = create_test_task("task1", "Task 1", 0, None);
        let task2 = create_test_task("task2", "Task 2", 0, None);

        let task_repo = TaskRepository::new(&conn);
        task_repo.insert(&task1, file_db_id, "test").unwrap();
        task_repo.insert(&task2, file_db_id, "test").unwrap();

        // Insert hierarchy dependencies
        let updater = DependencyUpdater::new(&conn);
        updater.insert_hierarchy_dependencies(file_db_id).unwrap();

        // Verify no dependencies created (no parent-child relationships)
        let (total, hierarchy, _) = updater.get_dependency_stats().unwrap();
        assert_eq!(total, 0);
        assert_eq!(hierarchy, 0);
    }

    #[test]
    fn test_insert_hierarchy_dependencies_with_parents() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Create file
        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Insert hierarchical tasks
        let parent = create_test_task("parent", "Parent Task", 0, None);
        let child1 = create_test_task("child1", "Child 1", 1, Some("parent"));
        let child2 = create_test_task("child2", "Child 2", 1, Some("parent"));

        let task_repo = TaskRepository::new(&conn);
        let parent_db_id = task_repo.insert(&parent, file_db_id, "test").unwrap();
        let child1_db_id = task_repo.insert(&child1, file_db_id, "test").unwrap();
        let child2_db_id = task_repo.insert(&child2, file_db_id, "test").unwrap();

        // Insert hierarchy dependencies
        let updater = DependencyUpdater::new(&conn);
        updater.insert_hierarchy_dependencies(file_db_id).unwrap();

        // Verify dependencies created
        let (total, hierarchy, explicit) = updater.get_dependency_stats().unwrap();
        assert_eq!(total, 2, "Should have 2 total dependencies");
        assert_eq!(hierarchy, 2, "Should have 2 hierarchy dependencies");
        assert_eq!(explicit, 0, "Should have 0 explicit dependencies");

        // Verify specific dependencies
        let dep_repo = DependencyRepository::new(&conn);
        let child1_deps = dep_repo.get_dependencies(child1_db_id).unwrap();
        assert_eq!(child1_deps.len(), 1);
        assert_eq!(child1_deps[0].to_task_id, Some(parent_db_id));
        assert_eq!(child1_deps[0].kind, DependencyKind::Hierarchy);

        let child2_deps = dep_repo.get_dependencies(child2_db_id).unwrap();
        assert_eq!(child2_deps.len(), 1);
        assert_eq!(child2_deps[0].to_task_id, Some(parent_db_id));
    }

    #[test]
    fn test_insert_hierarchy_dependencies_nested() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Create file
        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Insert nested hierarchy: grandparent -> parent -> child
        let grandparent = create_test_task("gp", "Grandparent", 0, None);
        let parent = create_test_task("parent", "Parent", 1, Some("gp"));
        let child = create_test_task("child", "Child", 2, Some("parent"));

        let task_repo = TaskRepository::new(&conn);
        let gp_db_id = task_repo.insert(&grandparent, file_db_id, "test").unwrap();
        let parent_db_id = task_repo.insert(&parent, file_db_id, "test").unwrap();
        let child_db_id = task_repo.insert(&child, file_db_id, "test").unwrap();

        // Insert hierarchy dependencies
        let updater = DependencyUpdater::new(&conn);
        updater.insert_hierarchy_dependencies(file_db_id).unwrap();

        // Verify dependencies
        let dep_repo = DependencyRepository::new(&conn);

        // Parent depends on grandparent
        let parent_deps = dep_repo.get_dependencies(parent_db_id).unwrap();
        assert_eq!(parent_deps.len(), 1);
        assert_eq!(parent_deps[0].to_task_id, Some(gp_db_id));

        // Child depends on parent
        let child_deps = dep_repo.get_dependencies(child_db_id).unwrap();
        assert_eq!(child_deps.len(), 1);
        assert_eq!(child_deps[0].to_task_id, Some(parent_db_id));
    }

    #[test]
    fn test_delete_dependencies_for_files() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Create two files
        let file1 = create_test_file("file1.md", "file1");
        let file2 = create_test_file("file2.md", "file2");
        let file_repo = FileRepository::new(&conn);
        let file1_db_id = file_repo.insert(&file1).unwrap();
        let file2_db_id = file_repo.insert(&file2).unwrap();

        // Add tasks with hierarchy to each file
        let task_repo = TaskRepository::new(&conn);

        let p1 = create_test_task("p1", "Parent 1", 0, None);
        let c1 = create_test_task("c1", "Child 1", 1, Some("p1"));
        task_repo.insert(&p1, file1_db_id, "file1").unwrap();
        task_repo.insert(&c1, file1_db_id, "file1").unwrap();

        let p2 = create_test_task("p2", "Parent 2", 0, None);
        let c2 = create_test_task("c2", "Child 2", 1, Some("p2"));
        task_repo.insert(&p2, file2_db_id, "file2").unwrap();
        task_repo.insert(&c2, file2_db_id, "file2").unwrap();

        // Insert hierarchy dependencies for both files
        let updater = DependencyUpdater::new(&conn);
        updater.insert_hierarchy_dependencies(file1_db_id).unwrap();
        updater.insert_hierarchy_dependencies(file2_db_id).unwrap();

        // Verify we have 2 dependencies
        let (total, _, _) = updater.get_dependency_stats().unwrap();
        assert_eq!(total, 2);

        // Delete dependencies for file1
        let deleted = updater
            .delete_dependencies_for_files(&[file1_db_id])
            .unwrap();
        assert_eq!(deleted, 1, "Should delete 1 dependency from file1");

        // Verify only 1 dependency remains
        let (total, _, _) = updater.get_dependency_stats().unwrap();
        assert_eq!(total, 1);
    }

    #[test]
    fn test_update_dependencies_for_files() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Create file
        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Insert initial hierarchy
        let parent = create_test_task("parent", "Parent", 0, None);
        let child = create_test_task("child", "Child", 1, Some("parent"));

        let task_repo = TaskRepository::new(&conn);
        task_repo.insert(&parent, file_db_id, "test").unwrap();
        task_repo.insert(&child, file_db_id, "test").unwrap();

        // Insert hierarchy dependencies
        let updater = DependencyUpdater::new(&conn);
        updater.insert_hierarchy_dependencies(file_db_id).unwrap();

        // Verify initial state
        let (total, _, _) = updater.get_dependency_stats().unwrap();
        assert_eq!(total, 1);

        // Simulate file update: delete old tasks, insert new structure
        // Note: This CASCADE deletes dependencies automatically
        conn.execute("DELETE FROM tasks WHERE file_id = ?1", [file_db_id])
            .unwrap();

        // Also clear the closure table since we deleted tasks
        conn.execute("DELETE FROM dependency_closure WHERE ancestor_id IN (SELECT id FROM tasks WHERE file_id = ?1) OR descendant_id IN (SELECT id FROM tasks WHERE file_id = ?1)", [file_db_id])
            .unwrap();

        // New structure: parent with two children
        let parent = create_test_task("parent", "Parent", 0, None);
        let child1 = create_test_task("child1", "Child 1", 1, Some("parent"));
        let child2 = create_test_task("child2", "Child 2", 1, Some("parent"));

        task_repo.insert(&parent, file_db_id, "test").unwrap();
        task_repo.insert(&child1, file_db_id, "test").unwrap();
        task_repo.insert(&child2, file_db_id, "test").unwrap();

        // Update dependencies
        updater
            .update_dependencies_for_files(&[file_db_id])
            .unwrap();

        // Verify new state: should have 2 dependencies now
        let (total, hierarchy, _) = updater.get_dependency_stats().unwrap();
        assert_eq!(total, 2, "Should have 2 dependencies after update");
        assert_eq!(hierarchy, 2, "Should have 2 hierarchy dependencies");
    }

    #[test]
    fn test_verify_hierarchy_dependencies() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Create file
        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Insert tasks with parent-child relationship
        let parent = create_test_task("parent", "Parent", 0, None);
        let child = create_test_task("child", "Child", 1, Some("parent"));

        let task_repo = TaskRepository::new(&conn);
        task_repo.insert(&parent, file_db_id, "test").unwrap();
        task_repo.insert(&child, file_db_id, "test").unwrap();

        // Before inserting dependencies, should find missing ones
        let updater = DependencyUpdater::new(&conn);
        let missing = updater.verify_hierarchy_dependencies().unwrap();
        assert_eq!(missing, 1, "Should find 1 missing hierarchy dependency");

        // Insert hierarchy dependencies
        updater.insert_hierarchy_dependencies(file_db_id).unwrap();

        // After inserting, should find none missing
        let missing = updater.verify_hierarchy_dependencies().unwrap();
        assert_eq!(missing, 0, "Should find 0 missing hierarchy dependencies");
    }

    #[test]
    fn test_delete_dependencies_for_files_empty() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let updater = DependencyUpdater::new(&conn);

        // Should handle empty input gracefully
        let deleted = updater.delete_dependencies_for_files(&[]).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_update_dependencies_for_files_empty() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let updater = DependencyUpdater::new(&conn);

        // Should handle empty input gracefully
        updater.update_dependencies_for_files(&[]).unwrap();
    }
}
