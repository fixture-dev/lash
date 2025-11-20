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

    /// Full unique identifier (file_id#local_id)
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
                task.order_index as i64,
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

        let rows = self.conn.execute(
            "UPDATE tasks
             SET title = ?1, status = ?2, depth = ?3, parent_id = ?4, order_index = ?5, owner = ?6, estimate = ?7, body = ?8, metadata = ?9
             WHERE full_id = ?10",
            (
                &task.title,
                task.status.as_str(),
                task.depth,
                parent_db_id,
                task.order_index as i64,
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
                |row| self.row_to_task_record(row),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get the database ID for a task by its full ID
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    fn get_db_id_by_full_id(&self, full_id: &str) -> DbResult<Option<i64>> {
        self.conn
            .query_row(
                "SELECT id FROM tasks WHERE full_id = ?1",
                [full_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get all tasks in a file, ordered by order_index
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
            .query_map([file_db_id], |row| self.row_to_task_record(row))?
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
            .query_map([status.as_str()], |row| self.row_to_task_record(row))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Find tasks by label
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn find_by_label(&self, label: &str) -> DbResult<Vec<TaskRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.file_id, t.local_id, t.full_id, t.title, t.status, t.depth, t.parent_id, t.order_index, t.owner, t.estimate, t.body, t.metadata
             FROM tasks t
             JOIN task_labels tl ON t.id = tl.task_id
             JOIN labels l ON tl.label_id = l.id
             WHERE l.name = ?1
             ORDER BY t.full_id",
        )?;

        let tasks = stmt
            .query_map([label], |row| self.row_to_task_record(row))?
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
            .query_map([task_db_id], |row| self.row_to_task_record(row))?
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
            .query_map([task_db_id], |row| self.row_to_task_record(row))?
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
            .query_map([task_db_id], |row| self.row_to_task_record(row))?
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
        if !filter.labels.is_empty() {
            joins.push(
                "JOIN task_labels tl ON t.id = tl.task_id
                 JOIN labels l ON tl.label_id = l.id"
                    .to_string(),
            );
        }

        // Add joins for file path if needed
        if filter.file_path.is_some() {
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
            .query_map(params.as_slice(), |row| self.row_to_task_record(row))?
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
                    task.order_index as i64,
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

    /// Helper to convert a row to TaskRecord
    fn row_to_task_record(&self, row: &rusqlite::Row) -> rusqlite::Result<TaskRecord> {
        let metadata_json: String = row.get(12)?;
        let metadata: TaskMetadata = serde_json::from_str(&metadata_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(12, rusqlite::types::Type::Text, Box::new(e))
        })?;

        let status_str: String = row.get(5)?;
        let status = TaskStatus::from_str_lossy(&status_str);

        Ok(TaskRecord {
            id: row.get(0)?,
            file_id: row.get(1)?,
            local_id: row.get(2)?,
            full_id: row.get(3)?,
            title: row.get(4)?,
            status,
            depth: row.get(6)?,
            parent_id: row.get(7)?,
            order_index: row.get::<_, i64>(8)? as usize,
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
}
