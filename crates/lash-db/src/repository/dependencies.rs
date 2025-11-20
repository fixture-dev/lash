//! Dependency repository for managing task dependencies

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use lash_types::DependencyKind;

use crate::error::DbResult;

/// A dependency record from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRecord {
    /// Database ID
    pub id: i64,

    /// Task that has the dependency (depends ON `to_task_id`)
    pub from_task_id: i64,

    /// Task that is depended upon (can be None for unresolved refs)
    pub to_task_id: Option<i64>,

    /// Kind of dependency
    pub kind: DependencyKind,

    /// Original reference string
    pub raw_ref: Option<String>,
}

/// Repository for dependency operations
pub struct DependencyRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> DependencyRepository<'conn> {
    /// Create a new dependency repository
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::connection::init_database;
    /// use lash_db::repository::DependencyRepository;
    /// use std::path::Path;
    ///
    /// let conn = init_database(Path::new("/tmp/lash.db")).unwrap();
    /// let repo = DependencyRepository::new(&conn);
    /// ```
    #[must_use]
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Insert a new dependency
    ///
    /// # Errors
    ///
    /// Returns error if insert fails
    pub fn insert(
        &self,
        from_task_id: i64,
        to_task_id: Option<i64>,
        kind: &DependencyKind,
        raw_ref: Option<&str>,
    ) -> DbResult<i64> {
        self.conn.execute(
            "INSERT INTO dependencies (from_task_id, to_task_id, kind, raw_ref)
             VALUES (?1, ?2, ?3, ?4)",
            (from_task_id, to_task_id, kind.as_str(), raw_ref),
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Delete a dependency
    ///
    /// # Errors
    ///
    /// Returns error if delete fails
    pub fn delete(&self, from_task_id: i64, to_task_id: i64) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM dependencies WHERE from_task_id = ?1 AND to_task_id = ?2",
            (from_task_id, to_task_id),
        )?;
        Ok(())
    }

    /// Get all dependencies for a task (outgoing - what this task depends ON)
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_dependencies(&self, task_id: i64) -> DbResult<Vec<DependencyRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, from_task_id, to_task_id, kind, raw_ref
             FROM dependencies WHERE from_task_id = ?1",
        )?;

        let deps = stmt
            .query_map([task_id], |row| {
                let kind_str: String = row.get(3)?;
                Ok(DependencyRecord {
                    id: row.get(0)?,
                    from_task_id: row.get(1)?,
                    to_task_id: row.get(2)?,
                    kind: DependencyKind::from_str_lossy(&kind_str),
                    raw_ref: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(deps)
    }

    /// Get all dependents for a task (incoming - tasks that depend on this one)
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_dependents(&self, task_id: i64) -> DbResult<Vec<DependencyRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, from_task_id, to_task_id, kind, raw_ref
             FROM dependencies WHERE to_task_id = ?1",
        )?;

        let deps = stmt
            .query_map([task_id], |row| {
                let kind_str: String = row.get(3)?;
                Ok(DependencyRecord {
                    id: row.get(0)?,
                    from_task_id: row.get(1)?,
                    to_task_id: row.get(2)?,
                    kind: DependencyKind::from_str_lossy(&kind_str),
                    raw_ref: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(deps)
    }

    /// Check if a dependency would create a cycle
    ///
    /// Returns true if adding edge (from → to) would create a cycle.
    /// Uses recursive query to check if `from` is already a descendant of `to`.
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn would_create_cycle(&self, from_task_id: i64, to_task_id: i64) -> DbResult<bool> {
        // Check if from_task_id is already a descendant of to_task_id
        // (i.e., there's already a path from to_task_id to from_task_id)
        let cycle_exists: bool = self.conn.query_row(
            "WITH RECURSIVE descendants(id) AS (
                 SELECT to_task_id FROM dependencies WHERE from_task_id = ?1 AND to_task_id IS NOT NULL
                 UNION ALL
                 SELECT d.to_task_id FROM dependencies d
                 JOIN descendants desc ON d.from_task_id = desc.id
                 WHERE d.to_task_id IS NOT NULL
             )
             SELECT COUNT(*) > 0 FROM descendants WHERE id = ?2",
            (to_task_id, from_task_id),
            |row| row.get(0),
        )?;

        Ok(cycle_exists)
    }

    /// Rebuild the transitive closure table
    ///
    /// This should be called after bulk dependency changes.
    /// For now, this is a placeholder - full implementation would rebuild
    /// the `dependency_closure` table for O(1) reachability queries.
    ///
    /// # Errors
    ///
    /// Returns error if rebuild fails
    pub fn rebuild_closure(&self) -> DbResult<()> {
        let tx = self.conn.unchecked_transaction()?;

        // Clear existing closure
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
        // This finds all paths and inserts them
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

    /// Get all transitive dependencies (all tasks this task depends on, directly or indirectly)
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_all_dependencies(&self, task_id: i64) -> DbResult<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT ancestor_id FROM dependency_closure WHERE descendant_id = ?1 ORDER BY depth",
        )?;

        let deps = stmt
            .query_map([task_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(deps)
    }

    /// Get all transitive dependents (all tasks that depend on this one, directly or indirectly)
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_all_dependents(&self, task_id: i64) -> DbResult<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT descendant_id FROM dependency_closure WHERE ancestor_id = ?1 ORDER BY depth",
        )?;

        let deps = stmt
            .query_map([task_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(deps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use crate::repository::{FileRepository, TaskRepository};
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
            tasks: TaskTree::new(),
            hash: "test_hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    fn create_test_task(id: &str, title: &str) -> Task {
        Task {
            id: id.to_string(),
            title: title.to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            metadata: TaskMetadata::default(),
            body: None,
        }
    }

    #[test]
    fn test_insert_dependency() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task1 = create_test_task("task1", "Task 1");
        let task2 = create_test_task("task2", "Task 2");
        let task_repo = TaskRepository::new(&conn);
        let task1_id = task_repo.insert(&task1, file_db_id, "test").unwrap();
        let task2_id = task_repo.insert(&task2, file_db_id, "test").unwrap();

        let dep_repo = DependencyRepository::new(&conn);
        let dep_id = dep_repo
            .insert(task1_id, Some(task2_id), &DependencyKind::ExplicitId, None)
            .unwrap();

        assert!(dep_id > 0);
    }

    #[test]
    fn test_get_dependencies() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task1 = create_test_task("task1", "Task 1");
        let task2 = create_test_task("task2", "Task 2");
        let task_repo = TaskRepository::new(&conn);
        let task1_id = task_repo.insert(&task1, file_db_id, "test").unwrap();
        let task2_id = task_repo.insert(&task2, file_db_id, "test").unwrap();

        let dep_repo = DependencyRepository::new(&conn);
        dep_repo
            .insert(task1_id, Some(task2_id), &DependencyKind::ExplicitId, None)
            .unwrap();

        let deps = dep_repo.get_dependencies(task1_id).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].to_task_id, Some(task2_id));
    }

    #[test]
    fn test_get_dependents() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task1 = create_test_task("task1", "Task 1");
        let task2 = create_test_task("task2", "Task 2");
        let task_repo = TaskRepository::new(&conn);
        let task1_id = task_repo.insert(&task1, file_db_id, "test").unwrap();
        let task2_id = task_repo.insert(&task2, file_db_id, "test").unwrap();

        let dep_repo = DependencyRepository::new(&conn);
        dep_repo
            .insert(task1_id, Some(task2_id), &DependencyKind::ExplicitId, None)
            .unwrap();

        let deps = dep_repo.get_dependents(task2_id).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].from_task_id, task1_id);
    }

    #[test]
    fn test_cycle_detection() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task1 = create_test_task("task1", "Task 1");
        let task2 = create_test_task("task2", "Task 2");
        let task3 = create_test_task("task3", "Task 3");
        let task_repo = TaskRepository::new(&conn);
        let task1_id = task_repo.insert(&task1, file_db_id, "test").unwrap();
        let task2_id = task_repo.insert(&task2, file_db_id, "test").unwrap();
        let task3_id = task_repo.insert(&task3, file_db_id, "test").unwrap();

        let dep_repo = DependencyRepository::new(&conn);

        // Create chain: task1 → task2 → task3
        dep_repo
            .insert(task1_id, Some(task2_id), &DependencyKind::ExplicitId, None)
            .unwrap();
        dep_repo
            .insert(task2_id, Some(task3_id), &DependencyKind::ExplicitId, None)
            .unwrap();

        // Adding task3 → task1 would create a cycle
        let would_cycle = dep_repo.would_create_cycle(task3_id, task1_id).unwrap();
        assert!(would_cycle);

        // Adding task3 → task1 should not create a cycle (different direction)
        let would_cycle = dep_repo.would_create_cycle(task1_id, task3_id).unwrap();
        assert!(!would_cycle);
    }

    #[test]
    fn test_rebuild_closure() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task1 = create_test_task("task1", "Task 1");
        let task2 = create_test_task("task2", "Task 2");
        let task3 = create_test_task("task3", "Task 3");
        let task_repo = TaskRepository::new(&conn);
        let task1_id = task_repo.insert(&task1, file_db_id, "test").unwrap();
        let task2_id = task_repo.insert(&task2, file_db_id, "test").unwrap();
        let task3_id = task_repo.insert(&task3, file_db_id, "test").unwrap();

        let dep_repo = DependencyRepository::new(&conn);

        // Create chain: task1 → task2 → task3
        dep_repo
            .insert(task1_id, Some(task2_id), &DependencyKind::ExplicitId, None)
            .unwrap();
        dep_repo
            .insert(task2_id, Some(task3_id), &DependencyKind::ExplicitId, None)
            .unwrap();

        // Rebuild closure
        dep_repo.rebuild_closure().unwrap();

        // task1 should have transitive dependencies on task2 and task3
        let all_deps = dep_repo.get_all_dependencies(task1_id).unwrap();
        assert_eq!(all_deps.len(), 2);
        assert!(all_deps.contains(&task2_id));
        assert!(all_deps.contains(&task3_id));
    }
}
