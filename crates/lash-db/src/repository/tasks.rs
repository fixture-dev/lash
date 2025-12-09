//! Task repository for CRUD operations on tasks

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use lash_types::{make_full_id, Task, TaskMetadata, TaskStatus};

use crate::error::{DbError, DbResult};

/// A task record from the database
///
/// Represents a row from the tasks table, including all metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    /// Database ID
    pub id: i64,

    /// Reference to parent file
    pub file_id: i64,

    /// Task ID within the file
    pub local_id: String,

    /// Full unique identifier (`file_id#local_id`)
    pub full_id: String,

    /// Task title
    pub title: String,

    /// Current status
    pub status: TaskStatus,

    /// Nesting level
    pub depth: u8,

    /// Parent task database ID (if nested)
    pub parent_id: Option<i64>,

    /// Position among siblings
    pub order_index: usize,

    /// Owner
    pub owner: Option<String>,

    /// Estimate
    pub estimate: Option<String>,

    /// Extended description
    pub body: Option<String>,

    /// Task metadata
    pub metadata: TaskMetadata,
}

/// Filter criteria for querying tasks
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    /// Filter by status
    pub status: Option<TaskStatus>,

    /// Filter by labels (must have all of these labels)
    pub labels: Vec<String>,

    /// Filter by owner
    pub owner: Option<String>,

    /// Filter by file path
    pub file_path: Option<String>,

    /// Filter by blocked status
    pub blocked: Option<bool>,
}

/// Repository for task operations
pub struct TaskRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> TaskRepository<'conn> {
    /// Create a new task repository
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::connection::init_database;
    /// use lash_db::repository::TaskRepository;
    /// use std::path::Path;
    ///
    /// let conn = init_database(Path::new("/tmp/lash.db")).unwrap();
    /// let repo = TaskRepository::new(&conn);
    /// ```
    #[must_use]
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Insert a new task into the database
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Insert fails
    /// - Metadata serialization fails
    /// - Full ID already exists (unique constraint)
    pub fn insert(&self, task: &Task, file_db_id: i64, file_id: &str) -> DbResult<i64> {
        let metadata_json = serde_json::to_string(&task.metadata)?;
        let full_id = make_full_id(file_id, &task.id);

        // Convert parent_id (local) to database parent_id if present
        let parent_db_id = if let Some(ref parent_local_id) = task.parent_id {
            let parent_full_id = make_full_id(file_id, parent_local_id);
            self.get_db_id_by_full_id(&parent_full_id)?
        } else {
            None
        };

        // SQLite uses i64 for integers. order_index is usize but limited to task hierarchy depth,
        // so this cast is safe in practice (we'd never have 2^63 tasks at the same level).
        #[allow(clippy::cast_possible_wrap)]
        let order_index_i64 = task.order_index as i64;

        self.conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, parent_id, order_index, owner, estimate, body, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            (
                file_db_id,
                &task.id,
                &full_id,
                &task.title,
                task.status.as_str(),
                task.depth,
                parent_db_id,
                order_index_i64,
                &task.metadata.owner,
                &task.metadata.estimate,
                &task.body,
                metadata_json,
            ),
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Update an existing task
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Task not found
    /// - Metadata serialization fails
    pub fn update(&self, task: &Task, file_id: &str) -> DbResult<()> {
        let metadata_json = serde_json::to_string(&task.metadata)?;
        let full_id = make_full_id(file_id, &task.id);

        // Convert parent_id (local) to database parent_id if present
        let parent_db_id = if let Some(ref parent_local_id) = task.parent_id {
            let parent_full_id = make_full_id(file_id, parent_local_id);
            self.get_db_id_by_full_id(&parent_full_id)?
        } else {
            None
        };

        // SQLite uses i64 for integers. order_index is usize but limited to task hierarchy depth,
        // so this cast is safe in practice (we'd never have 2^63 tasks at the same level).
        #[allow(clippy::cast_possible_wrap)]
        let order_index_i64 = task.order_index as i64;

        let rows = self.conn.execute(
            "UPDATE tasks
             SET title = ?1, status = ?2, depth = ?3, parent_id = ?4, order_index = ?5, owner = ?6, estimate = ?7, body = ?8, metadata = ?9
             WHERE full_id = ?10",
            (
                &task.title,
                task.status.as_str(),
                task.depth,
                parent_db_id,
                order_index_i64,
                &task.metadata.owner,
                &task.metadata.estimate,
                &task.body,
                metadata_json,
                &full_id,
            ),
        )?;

        if rows == 0 {
            return Err(DbError::TaskNotFound(full_id));
        }

        Ok(())
    }

    /// Delete a task by full ID
    ///
    /// # Errors
    ///
    /// Returns error if deletion fails
    pub fn delete(&self, full_id: &str) -> DbResult<()> {
        self.conn
            .execute("DELETE FROM tasks WHERE full_id = ?1", [full_id])?;
        Ok(())
    }

    /// Get a task by its full ID
    ///
    /// # Errors
    ///
    /// Returns error if query fails or metadata deserialization fails
    pub fn get_by_full_id(&self, full_id: &str) -> DbResult<Option<TaskRecord>> {
        self.conn
            .query_row(
                "SELECT id, file_id, local_id, full_id, title, status, depth, parent_id, order_index, owner, estimate, body, metadata
                 FROM tasks WHERE full_id = ?1",
                [full_id],
                Self::row_to_task_record,
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get a task by its database ID
    ///
    /// # Errors
    ///
    /// Returns error if query fails or metadata deserialization fails
    pub fn get_by_db_id(&self, id: i64) -> DbResult<Option<TaskRecord>> {
        self.conn
            .query_row(
                "SELECT id, file_id, local_id, full_id, title, status, depth, parent_id, order_index, owner, estimate, body, metadata
                 FROM tasks WHERE id = ?1",
                [id],
                Self::row_to_task_record,
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get the database ID for a task by its full ID
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_db_id_by_full_id(&self, full_id: &str) -> DbResult<Option<i64>> {
        self.conn
            .query_row(
                "SELECT id FROM tasks WHERE full_id = ?1",
                [full_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get all tasks in a file, ordered by `order_index`
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_by_file(&self, file_db_id: i64) -> DbResult<Vec<TaskRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_id, local_id, full_id, title, status, depth, parent_id, order_index, owner, estimate, body, metadata
             FROM tasks WHERE file_id = ?1 ORDER BY order_index",
        )?;

        let tasks = stmt
            .query_map([file_db_id], Self::row_to_task_record)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Find tasks by status
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn find_by_status(&self, status: TaskStatus) -> DbResult<Vec<TaskRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_id, local_id, full_id, title, status, depth, parent_id, order_index, owner, estimate, body, metadata
             FROM tasks WHERE status = ?1 ORDER BY full_id",
        )?;

        let tasks = stmt
            .query_map([status.as_str()], Self::row_to_task_record)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Find tasks by label
    ///
    /// Finds tasks that have the label directly (via `task_labels`) OR
    /// are in a file with the label (via `file_labels`).
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn find_by_label(&self, label: &str) -> DbResult<Vec<TaskRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT t.id, t.file_id, t.local_id, t.full_id, t.title, t.status, t.depth, t.parent_id, t.order_index, t.owner, t.estimate, t.body, t.metadata
             FROM tasks t
             JOIN files f ON t.file_id = f.id
             LEFT JOIN task_labels tl ON t.id = tl.task_id
             LEFT JOIN file_labels fl ON f.id = fl.file_id
             JOIN labels l ON (l.id = tl.label_id OR l.id = fl.label_id)
             WHERE l.name = ?1
             ORDER BY t.full_id",
        )?;

        let tasks = stmt
            .query_map([label], Self::row_to_task_record)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Get direct children of a task
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_children(&self, task_db_id: i64) -> DbResult<Vec<TaskRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_id, local_id, full_id, title, status, depth, parent_id, order_index, owner, estimate, body, metadata
             FROM tasks WHERE parent_id = ?1 ORDER BY order_index",
        )?;

        let tasks = stmt
            .query_map([task_db_id], Self::row_to_task_record)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Get all descendants of a task (recursive)
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_descendants(&self, task_db_id: i64) -> DbResult<Vec<TaskRecord>> {
        // Use recursive CTE to get all descendants
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE descendants(id) AS (
                 SELECT id FROM tasks WHERE parent_id = ?1
                 UNION ALL
                 SELECT t.id FROM tasks t
                 JOIN descendants d ON t.parent_id = d.id
             )
             SELECT t.id, t.file_id, t.local_id, t.full_id, t.title, t.status, t.depth, t.parent_id, t.order_index, t.owner, t.estimate, t.body, t.metadata
             FROM tasks t
             JOIN descendants d ON t.id = d.id
             ORDER BY t.depth, t.order_index",
        )?;

        let tasks = stmt
            .query_map([task_db_id], Self::row_to_task_record)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Get all ancestors of a task (walking up the parent chain)
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_ancestors(&self, task_db_id: i64) -> DbResult<Vec<TaskRecord>> {
        // Use recursive CTE to walk up parent chain
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE ancestors(id) AS (
                 SELECT parent_id as id FROM tasks WHERE id = ?1 AND parent_id IS NOT NULL
                 UNION ALL
                 SELECT t.parent_id FROM tasks t
                 JOIN ancestors a ON t.id = a.id
                 WHERE t.parent_id IS NOT NULL
             )
             SELECT t.id, t.file_id, t.local_id, t.full_id, t.title, t.status, t.depth, t.parent_id, t.order_index, t.owner, t.estimate, t.body, t.metadata
             FROM tasks t
             JOIN ancestors a ON t.id = a.id
             ORDER BY t.depth",
        )?;

        let tasks = stmt
            .query_map([task_db_id], Self::row_to_task_record)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Find tasks matching the given filter criteria
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn find(&self, filter: &TaskFilter) -> DbResult<Vec<TaskRecord>> {
        // Build dynamic query based on filter
        let mut query = String::from(
            "SELECT DISTINCT t.id, t.file_id, t.local_id, t.full_id, t.title, t.status, t.depth, t.parent_id, t.order_index, t.owner, t.estimate, t.body, t.metadata
             FROM tasks t",
        );

        let mut joins = Vec::new();
        let mut wheres = Vec::new();

        // Add joins for labels if needed
        // Match tasks that have the label directly OR are in a file with the label
        if !filter.labels.is_empty() {
            // Always join files table when filtering by labels (needed for file_labels lookup)
            if filter.file_path.is_none() {
                joins.push("JOIN files f ON t.file_id = f.id".to_string());
            }
            joins.push(
                "LEFT JOIN task_labels tl ON t.id = tl.task_id
                 LEFT JOIN file_labels fl ON f.id = fl.file_id
                 JOIN labels l ON (l.id = tl.label_id OR l.id = fl.label_id)"
                    .to_string(),
            );
        }

        // Add joins for file path if needed
        if filter.file_path.is_some() && filter.labels.is_empty() {
            joins.push("JOIN files f ON t.file_id = f.id".to_string());
        }

        // Build WHERE clauses
        if filter.status.is_some() {
            wheres.push("t.status = ?".to_string());
        }

        if filter.owner.is_some() {
            wheres.push("t.owner = ?".to_string());
        }

        if filter.file_path.is_some() {
            wheres.push("f.path = ?".to_string());
        }

        if !filter.labels.is_empty() {
            wheres.push(format!(
                "l.name IN ({})",
                vec!["?"; filter.labels.len()].join(", ")
            ));
        }

        // Combine query parts
        if !joins.is_empty() {
            query.push(' ');
            query.push_str(&joins.join(" "));
        }

        if !wheres.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&wheres.join(" AND "));
        }

        query.push_str(" ORDER BY t.full_id");

        // Prepare statement and bind parameters
        let mut stmt = self.conn.prepare(&query)?;

        // Convert status to owned string to avoid lifetime issues
        let status_str = filter.status.map(|s| s.as_str().to_string());

        // Build parameter list
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();

        if let Some(ref status) = status_str {
            params.push(status);
        }
        if let Some(ref owner) = filter.owner {
            params.push(owner);
        }
        if let Some(ref path) = filter.file_path {
            params.push(path);
        }
        for label in &filter.labels {
            params.push(label);
        }

        let tasks = stmt
            .query_map(params.as_slice(), Self::row_to_task_record)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Insert multiple tasks in a single transaction
    ///
    /// # Errors
    ///
    /// Returns error if any insert fails
    pub fn insert_batch(&self, tasks: &[(Task, i64, String)]) -> DbResult<()> {
        let tx = self.conn.unchecked_transaction()?;

        for (task, file_db_id, file_id) in tasks {
            let metadata_json = serde_json::to_string(&task.metadata)?;
            let full_id = make_full_id(file_id, &task.id);

            // Convert parent_id (local) to database parent_id if present
            let parent_db_id: Option<i64> = if let Some(ref parent_local_id) = task.parent_id {
                let parent_full_id = make_full_id(file_id, parent_local_id);
                // Query within transaction
                tx.query_row(
                    "SELECT id FROM tasks WHERE full_id = ?1",
                    [&parent_full_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
            } else {
                None
            };

            // SQLite uses i64 for integers. order_index is usize but limited to task hierarchy depth,
            // so this cast is safe in practice (we'd never have 2^63 tasks at the same level).
            #[allow(clippy::cast_possible_wrap)]
            let order_index_i64 = task.order_index as i64;

            tx.execute(
                "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, parent_id, order_index, owner, estimate, body, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                (
                    file_db_id,
                    &task.id,
                    &full_id,
                    &task.title,
                    task.status.as_str(),
                    task.depth,
                    parent_db_id,
                    order_index_i64,
                    &task.metadata.owner,
                    &task.metadata.estimate,
                    &task.body,
                    metadata_json,
                ),
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get total task count and completed task count for the entire project
    ///
    /// Returns a tuple of `(total_tasks, completed_tasks)` where completed
    /// includes both "done" and "waived" statuses.
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_project_counts(&self) -> DbResult<(usize, usize)> {
        let (total, completed): (i64, i64) = self.conn.query_row(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status IN ('done', 'waived') THEN 1 ELSE 0 END) as completed
             FROM tasks",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        #[allow(clippy::cast_sign_loss)]
        Ok((total as usize, completed as usize))
    }

    /// Get distinct owners from all tasks
    ///
    /// Returns a sorted list of unique owner names (excluding NULL values).
    /// Useful for autocomplete suggestions when creating tasks.
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_distinct_owners(&self) -> DbResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT owner FROM tasks WHERE owner IS NOT NULL ORDER BY owner")?;

        let owners = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(owners)
    }

    /// Helper to convert a row to `TaskRecord`
    fn row_to_task_record(row: &rusqlite::Row) -> rusqlite::Result<TaskRecord> {
        let metadata_json: String = row.get(12)?;
        let metadata: TaskMetadata = serde_json::from_str(&metadata_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(12, rusqlite::types::Type::Text, Box::new(e))
        })?;

        let status_str: String = row.get(5)?;
        let status = TaskStatus::from_str_lossy(&status_str);

        // SQLite stores order_index as i64, convert back to usize.
        // This is safe because we only store valid usize values (task positions).
        // On 32-bit platforms, this could theoretically truncate, but task counts
        // would never approach 2^32 in practice.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let order_index = row.get::<_, i64>(8)? as usize;

        Ok(TaskRecord {
            id: row.get(0)?,
            file_id: row.get(1)?,
            local_id: row.get(2)?,
            full_id: row.get(3)?,
            title: row.get(4)?,
            status,
            depth: row.get(6)?,
            parent_id: row.get(7)?,
            order_index,
            owner: row.get(9)?,
            estimate: row.get(10)?,
            body: row.get(11)?,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use crate::repository::FileRepository;
    use lash_types::{FileMetadata, TaskFile, TaskTree};
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
            metadata: TaskMetadata::default(),
            body: None,
        }
    }

    #[test]
    fn test_insert_task() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        // Insert a file first
        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Insert a task
        let task = create_test_task("task1", "Test Task", 0, None, 0);
        let task_repo = TaskRepository::new(&conn);
        let task_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        assert!(task_id > 0);
    }

    #[test]
    fn test_get_by_full_id() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Test Task", 0, None, 0);
        let task_repo = TaskRepository::new(&conn);
        task_repo.insert(&task, file_db_id, "test").unwrap();

        let retrieved = task_repo.get_by_full_id("test#task1").unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.full_id, "test#task1");
        assert_eq!(retrieved.title, "Test Task");
    }

    #[test]
    fn test_get_by_db_id() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Test Task", 0, None, 0);
        let task_repo = TaskRepository::new(&conn);
        let db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        // Test retrieving by database ID
        let retrieved = task_repo.get_by_db_id(db_id).unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, db_id);
        assert_eq!(retrieved.full_id, "test#task1");
        assert_eq!(retrieved.title, "Test Task");

        // Test non-existent ID
        let not_found = task_repo.get_by_db_id(99999).unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_by_file() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

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
        task_repo
            .insert(
                &create_test_task("task3", "Task 3", 0, None, 2),
                file_db_id,
                "test",
            )
            .unwrap();

        let tasks = task_repo.get_by_file(file_db_id).unwrap();
        assert_eq!(tasks.len(), 3);

        // Should be ordered by order_index
        assert_eq!(tasks[0].local_id, "task1");
        assert_eq!(tasks[1].local_id, "task2");
        assert_eq!(tasks[2].local_id, "task3");
    }

    #[test]
    fn test_find_by_status() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);
        let mut task1 = create_test_task("task1", "Task 1", 0, None, 0);
        task1.status = TaskStatus::Open;
        task_repo.insert(&task1, file_db_id, "test").unwrap();

        let mut task2 = create_test_task("task2", "Task 2", 0, None, 1);
        task2.status = TaskStatus::Done;
        task_repo.insert(&task2, file_db_id, "test").unwrap();

        let open_tasks = task_repo.find_by_status(TaskStatus::Open).unwrap();
        assert_eq!(open_tasks.len(), 1);
        assert_eq!(open_tasks[0].local_id, "task1");

        let done_tasks = task_repo.find_by_status(TaskStatus::Done).unwrap();
        assert_eq!(done_tasks.len(), 1);
        assert_eq!(done_tasks[0].local_id, "task2");
    }

    #[test]
    fn test_get_children() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // Insert parent task
        let parent = create_test_task("parent", "Parent Task", 0, None, 0);
        let parent_db_id = task_repo.insert(&parent, file_db_id, "test").unwrap();

        // Insert child tasks
        task_repo
            .insert(
                &create_test_task("child1", "Child 1", 1, Some("parent".to_string()), 0),
                file_db_id,
                "test",
            )
            .unwrap();
        task_repo
            .insert(
                &create_test_task("child2", "Child 2", 1, Some("parent".to_string()), 1),
                file_db_id,
                "test",
            )
            .unwrap();

        let children = task_repo.get_children(parent_db_id).unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].local_id, "child1");
        assert_eq!(children[1].local_id, "child2");
    }

    #[test]
    fn test_update_task() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);
        let mut task = create_test_task("task1", "Original Title", 0, None, 0);
        task_repo.insert(&task, file_db_id, "test").unwrap();

        // Update the task
        task.title = "Updated Title".to_string();
        task.status = TaskStatus::Done;
        task_repo.update(&task, "test").unwrap();

        let retrieved = task_repo.get_by_full_id("test#task1").unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated Title");
        assert_eq!(retrieved.status, TaskStatus::Done);
    }

    #[test]
    fn test_delete_task() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);
        let task = create_test_task("task1", "Test Task", 0, None, 0);
        task_repo.insert(&task, file_db_id, "test").unwrap();

        task_repo.delete("test#task1").unwrap();

        let retrieved = task_repo.get_by_full_id("test#task1").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_insert_batch() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let tasks = vec![
            (
                create_test_task("task1", "Task 1", 0, None, 0),
                file_db_id,
                "test".to_string(),
            ),
            (
                create_test_task("task2", "Task 2", 0, None, 1),
                file_db_id,
                "test".to_string(),
            ),
            (
                create_test_task("task3", "Task 3", 0, None, 2),
                file_db_id,
                "test".to_string(),
            ),
        ];

        let task_repo = TaskRepository::new(&conn);
        task_repo.insert_batch(&tasks).unwrap();

        let all_tasks = task_repo.get_by_file(file_db_id).unwrap();
        assert_eq!(all_tasks.len(), 3);
    }

    #[test]
    fn test_get_db_id_by_full_id() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Test Task", 0, None, 0);
        let task_repo = TaskRepository::new(&conn);
        let inserted_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        // Test successful lookup
        let db_id = task_repo.get_db_id_by_full_id("test#task1").unwrap();
        assert_eq!(db_id, Some(inserted_id));

        // Test non-existent task
        let not_found = task_repo.get_db_id_by_full_id("test#nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_find_by_label() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // Create tasks
        let task1 = create_test_task("task1", "Task 1", 0, None, 0);
        let task1_db_id = task_repo.insert(&task1, file_db_id, "test").unwrap();

        let task2 = create_test_task("task2", "Task 2", 0, None, 1);
        let task2_db_id = task_repo.insert(&task2, file_db_id, "test").unwrap();

        let task3 = create_test_task("task3", "Task 3", 0, None, 2);
        task_repo.insert(&task3, file_db_id, "test").unwrap();

        // Add labels using raw SQL (would normally use LabelRepository)
        conn.execute(
            "INSERT INTO labels (name) VALUES (?1), (?2)",
            ["backend", "frontend"],
        )
        .unwrap();

        let backend_label_id: i64 = conn
            .query_row(
                "SELECT id FROM labels WHERE name = ?1",
                ["backend"],
                |row| row.get(0),
            )
            .unwrap();

        let frontend_label_id: i64 = conn
            .query_row(
                "SELECT id FROM labels WHERE name = ?1",
                ["frontend"],
                |row| row.get(0),
            )
            .unwrap();

        // Associate labels with tasks
        conn.execute(
            "INSERT INTO task_labels (task_id, label_id) VALUES (?1, ?2), (?3, ?4), (?5, ?6)",
            (
                task1_db_id,
                backend_label_id,
                task2_db_id,
                backend_label_id,
                task2_db_id,
                frontend_label_id,
            ),
        )
        .unwrap();

        // Find tasks by label
        let backend_tasks = task_repo.find_by_label("backend").unwrap();
        assert_eq!(backend_tasks.len(), 2);
        assert!(backend_tasks.iter().any(|t| t.local_id == "task1"));
        assert!(backend_tasks.iter().any(|t| t.local_id == "task2"));

        let frontend_tasks = task_repo.find_by_label("frontend").unwrap();
        assert_eq!(frontend_tasks.len(), 1);
        assert_eq!(frontend_tasks[0].local_id, "task2");

        // Non-existent label should return empty list
        let not_found = task_repo.find_by_label("nonexistent").unwrap();
        assert!(not_found.is_empty());
    }

    #[test]
    fn test_get_descendants() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // Create hierarchy: parent -> child1 -> grandchild1
        //                           -> child2
        let parent = create_test_task("parent", "Parent", 0, None, 0);
        let parent_db_id = task_repo.insert(&parent, file_db_id, "test").unwrap();

        task_repo
            .insert(
                &create_test_task("child1", "Child 1", 1, Some("parent".to_string()), 0),
                file_db_id,
                "test",
            )
            .unwrap();

        task_repo
            .insert(
                &create_test_task("child2", "Child 2", 1, Some("parent".to_string()), 1),
                file_db_id,
                "test",
            )
            .unwrap();

        task_repo
            .insert(
                &create_test_task(
                    "grandchild1",
                    "Grandchild 1",
                    2,
                    Some("child1".to_string()),
                    0,
                ),
                file_db_id,
                "test",
            )
            .unwrap();

        // Get all descendants of parent
        let descendants = task_repo.get_descendants(parent_db_id).unwrap();
        assert_eq!(descendants.len(), 3);

        // Verify all descendants are present
        let ids: Vec<&str> = descendants.iter().map(|t| t.local_id.as_str()).collect();
        assert!(ids.contains(&"child1"));
        assert!(ids.contains(&"child2"));
        assert!(ids.contains(&"grandchild1"));

        // Verify ordering by depth then order_index
        assert!(descendants[0].depth <= descendants[1].depth);
        assert!(descendants[1].depth <= descendants[2].depth);
    }

    #[test]
    fn test_get_ancestors() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // Create hierarchy: grandparent -> parent -> child
        task_repo
            .insert(
                &create_test_task("grandparent", "Grandparent", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();

        task_repo
            .insert(
                &create_test_task("parent", "Parent", 1, Some("grandparent".to_string()), 0),
                file_db_id,
                "test",
            )
            .unwrap();

        let child = create_test_task("child", "Child", 2, Some("parent".to_string()), 0);
        let child_db_id = task_repo.insert(&child, file_db_id, "test").unwrap();

        // Get ancestors of child
        let ancestors = task_repo.get_ancestors(child_db_id).unwrap();
        assert_eq!(ancestors.len(), 2);

        // Verify ancestors are in order by depth (ascending - grandparent first)
        assert_eq!(ancestors[0].local_id, "grandparent");
        assert_eq!(ancestors[1].local_id, "parent");

        // Root task should have no ancestors
        let root_ancestors = task_repo
            .get_ancestors(
                task_repo
                    .get_db_id_by_full_id("test#grandparent")
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
        assert!(root_ancestors.is_empty());
    }

    #[test]
    fn test_find_with_filter() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // Create tasks with different properties
        let mut task1 = create_test_task("task1", "Task 1", 0, None, 0);
        task1.status = TaskStatus::Open;
        task1.metadata.owner = Some("alice".to_string());
        task_repo.insert(&task1, file_db_id, "test").unwrap();

        let mut task2 = create_test_task("task2", "Task 2", 0, None, 1);
        task2.status = TaskStatus::Done;
        task2.metadata.owner = Some("bob".to_string());
        task_repo.insert(&task2, file_db_id, "test").unwrap();

        let mut task3 = create_test_task("task3", "Task 3", 0, None, 2);
        task3.status = TaskStatus::Open;
        task3.metadata.owner = Some("alice".to_string());
        task_repo.insert(&task3, file_db_id, "test").unwrap();

        // Test filter by status
        let filter = TaskFilter {
            status: Some(TaskStatus::Open),
            ..Default::default()
        };
        let results = task_repo.find(&filter).unwrap();
        assert_eq!(results.len(), 2);

        // Test filter by owner
        let filter = TaskFilter {
            owner: Some("alice".to_string()),
            ..Default::default()
        };
        let results = task_repo.find(&filter).unwrap();
        assert_eq!(results.len(), 2);

        // Test filter by status AND owner
        let filter = TaskFilter {
            status: Some(TaskStatus::Open),
            owner: Some("alice".to_string()),
            ..Default::default()
        };
        let results = task_repo.find(&filter).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|t| t.status == TaskStatus::Open));
        assert!(results.iter().all(|t| t.owner.as_deref() == Some("alice")));

        // Test empty filter (should return all)
        let filter = TaskFilter::default();
        let results = task_repo.find(&filter).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_find_with_file_path_filter() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file_repo = FileRepository::new(&conn);
        let task_repo = TaskRepository::new(&conn);

        // Create multiple files with tasks
        let file1 = create_test_file("tasks/file1.md", "file1");
        let file1_db_id = file_repo.insert(&file1).unwrap();
        task_repo
            .insert(
                &create_test_task("task1", "Task 1", 0, None, 0),
                file1_db_id,
                "file1",
            )
            .unwrap();

        let file2 = create_test_file("tasks/file2.md", "file2");
        let file2_db_id = file_repo.insert(&file2).unwrap();
        task_repo
            .insert(
                &create_test_task("task2", "Task 2", 0, None, 0),
                file2_db_id,
                "file2",
            )
            .unwrap();

        // Filter by file path
        let filter = TaskFilter {
            file_path: Some("tasks/file1.md".to_string()),
            ..Default::default()
        };
        let results = task_repo.find(&filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].local_id, "task1");
    }

    #[test]
    fn test_find_with_labels_filter() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);
        let task1_db_id = task_repo
            .insert(
                &create_test_task("task1", "Task 1", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();

        let task2_db_id = task_repo
            .insert(
                &create_test_task("task2", "Task 2", 0, None, 1),
                file_db_id,
                "test",
            )
            .unwrap();

        // Create labels
        conn.execute(
            "INSERT INTO labels (name) VALUES (?1), (?2), (?3)",
            ["urgent", "backend", "frontend"],
        )
        .unwrap();

        let urgent_id: i64 = conn
            .query_row("SELECT id FROM labels WHERE name = ?1", ["urgent"], |row| {
                row.get(0)
            })
            .unwrap();
        let backend_id: i64 = conn
            .query_row(
                "SELECT id FROM labels WHERE name = ?1",
                ["backend"],
                |row| row.get(0),
            )
            .unwrap();

        // task1 has both urgent and backend labels
        conn.execute(
            "INSERT INTO task_labels (task_id, label_id) VALUES (?1, ?2), (?3, ?4)",
            (task1_db_id, urgent_id, task1_db_id, backend_id),
        )
        .unwrap();

        // task2 has only urgent label
        conn.execute(
            "INSERT INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
            [task2_db_id, urgent_id],
        )
        .unwrap();

        // Filter by single label
        let filter = TaskFilter {
            labels: vec!["urgent".to_string()],
            ..Default::default()
        };
        let results = task_repo.find(&filter).unwrap();
        assert_eq!(results.len(), 2);

        // Filter by multiple labels
        let filter = TaskFilter {
            labels: vec!["urgent".to_string(), "backend".to_string()],
            ..Default::default()
        };
        let results = task_repo.find(&filter).unwrap();
        // Should return tasks that have ANY of the specified labels
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_update_nonexistent_task() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);
        let task = create_test_task("nonexistent", "Task", 0, None, 0);

        let result = task_repo.update(&task, "test");
        assert!(result.is_err());
        match result {
            Err(DbError::TaskNotFound(full_id)) => {
                assert_eq!(full_id, "test#nonexistent");
            }
            _ => panic!("Expected TaskNotFound error"),
        }
    }

    #[test]
    fn test_task_with_all_optional_fields() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);
        let mut task = create_test_task("task1", "Task with fields", 0, None, 0);
        task.metadata.owner = Some("alice".to_string());
        task.metadata.estimate = Some("2h".to_string());
        task.metadata.agent_note = Some("Important task".to_string());
        task.body = Some("Detailed description".to_string());
        task.metadata
            .custom
            .insert("created".to_string(), "2024-01-01".to_string());

        task_repo.insert(&task, file_db_id, "test").unwrap();

        let retrieved = task_repo.get_by_full_id("test#task1").unwrap().unwrap();
        assert_eq!(retrieved.owner, Some("alice".to_string()));
        assert_eq!(retrieved.estimate, Some("2h".to_string()));
        assert_eq!(
            retrieved.metadata.agent_note,
            Some("Important task".to_string())
        );
        assert_eq!(retrieved.body, Some("Detailed description".to_string()));
        assert_eq!(
            retrieved.metadata.custom.get("created"),
            Some(&"2024-01-01".to_string())
        );
    }

    #[test]
    fn test_task_with_no_optional_fields() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);
        let task = create_test_task("task1", "Minimal task", 0, None, 0);

        task_repo.insert(&task, file_db_id, "test").unwrap();

        let retrieved = task_repo.get_by_full_id("test#task1").unwrap().unwrap();
        assert_eq!(retrieved.owner, None);
        assert_eq!(retrieved.estimate, None);
        assert_eq!(retrieved.body, None);
    }

    #[test]
    fn test_all_task_statuses() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        let statuses = vec![
            TaskStatus::Open,
            TaskStatus::Done,
            TaskStatus::Waived,
            TaskStatus::Blocked,
        ];

        for (idx, status) in statuses.iter().enumerate() {
            let mut task =
                create_test_task(&format!("task{idx}"), &format!("Task {idx}"), 0, None, idx);
            task.status = *status;
            task_repo.insert(&task, file_db_id, "test").unwrap();
        }

        // Verify each status
        for (idx, status) in statuses.iter().enumerate() {
            let retrieved = task_repo
                .get_by_full_id(&format!("test#task{idx}"))
                .unwrap()
                .unwrap();
            assert_eq!(retrieved.status, *status);
        }

        // Test finding by each status
        for status in statuses {
            let found = task_repo.find_by_status(status).unwrap();
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].status, status);
        }
    }

    #[test]
    fn test_deep_task_hierarchy() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // Create a hierarchy of depth 5
        task_repo
            .insert(
                &create_test_task("level0", "Level 0", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();

        task_repo
            .insert(
                &create_test_task("level1", "Level 1", 1, Some("level0".to_string()), 0),
                file_db_id,
                "test",
            )
            .unwrap();

        task_repo
            .insert(
                &create_test_task("level2", "Level 2", 2, Some("level1".to_string()), 0),
                file_db_id,
                "test",
            )
            .unwrap();

        task_repo
            .insert(
                &create_test_task("level3", "Level 3", 3, Some("level2".to_string()), 0),
                file_db_id,
                "test",
            )
            .unwrap();

        let deepest_task = create_test_task("level4", "Level 4", 4, Some("level3".to_string()), 0);
        let deepest_id = task_repo.insert(&deepest_task, file_db_id, "test").unwrap();

        // Verify the deepest task can be retrieved
        let retrieved = task_repo.get_by_db_id(deepest_id).unwrap().unwrap();
        assert_eq!(retrieved.depth, 4);
        assert_eq!(retrieved.local_id, "level4");

        // Verify ancestors
        let ancestors = task_repo.get_ancestors(deepest_id).unwrap();
        assert_eq!(ancestors.len(), 4);
    }

    #[test]
    fn test_delete_nonexistent_task() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // Deleting non-existent task should succeed (no error)
        let result = task_repo.delete("test#nonexistent");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_children_empty() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo
            .insert(
                &create_test_task("task1", "Task 1", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();

        // Task with no children
        let children = task_repo.get_children(task_db_id).unwrap();
        assert!(children.is_empty());
    }

    #[test]
    fn test_get_by_file_empty() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // File with no tasks
        let tasks = task_repo.get_by_file(file_db_id).unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_find_by_status_empty() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // Insert only open tasks
        task_repo
            .insert(
                &create_test_task("task1", "Task 1", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();

        // Search for done tasks should return empty
        let done_tasks = task_repo.find_by_status(TaskStatus::Done).unwrap();
        assert!(done_tasks.is_empty());
    }

    #[test]
    fn test_get_full_id_not_found() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        let result = task_repo.get_by_full_id("test#nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_insert_batch_with_parent_child() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        // Create parent and child tasks
        let parent = create_test_task("parent", "Parent", 0, None, 0);
        let child = create_test_task("child", "Child", 1, Some("parent".to_string()), 0);

        let tasks = vec![
            (parent, file_db_id, "test".to_string()),
            (child, file_db_id, "test".to_string()),
        ];

        let task_repo = TaskRepository::new(&conn);
        task_repo.insert_batch(&tasks).unwrap();

        // Verify parent-child relationship
        let parent_record = task_repo.get_by_full_id("test#parent").unwrap().unwrap();
        let child_record = task_repo.get_by_full_id("test#child").unwrap().unwrap();

        assert_eq!(child_record.parent_id, Some(parent_record.id));
    }

    #[test]
    fn test_update_with_parent_change() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // Create two potential parents and a child
        task_repo
            .insert(
                &create_test_task("parent1", "Parent 1", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();

        task_repo
            .insert(
                &create_test_task("parent2", "Parent 2", 0, None, 1),
                file_db_id,
                "test",
            )
            .unwrap();

        let mut child = create_test_task("child", "Child", 1, Some("parent1".to_string()), 0);
        task_repo.insert(&child, file_db_id, "test").unwrap();

        // Update child to have different parent
        child.parent_id = Some("parent2".to_string());
        task_repo.update(&child, "test").unwrap();

        let parent2_record = task_repo.get_by_full_id("test#parent2").unwrap().unwrap();
        let child_record = task_repo.get_by_full_id("test#child").unwrap().unwrap();

        assert_eq!(child_record.parent_id, Some(parent2_record.id));
    }

    #[test]
    fn test_update_to_remove_parent() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        task_repo
            .insert(
                &create_test_task("parent", "Parent", 0, None, 0),
                file_db_id,
                "test",
            )
            .unwrap();

        let mut child = create_test_task("child", "Child", 1, Some("parent".to_string()), 0);
        task_repo.insert(&child, file_db_id, "test").unwrap();

        // Update to remove parent
        child.parent_id = None;
        child.depth = 0;
        task_repo.update(&child, "test").unwrap();

        let child_record = task_repo.get_by_full_id("test#child").unwrap().unwrap();
        assert_eq!(child_record.parent_id, None);
        assert_eq!(child_record.depth, 0);
    }

    #[test]
    fn test_large_order_index() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task_repo = TaskRepository::new(&conn);

        // Test with a large order index
        let task = create_test_task("task1", "Task 1", 0, None, 9999);
        task_repo.insert(&task, file_db_id, "test").unwrap();

        let retrieved = task_repo.get_by_full_id("test#task1").unwrap().unwrap();
        assert_eq!(retrieved.order_index, 9999);
    }
}
