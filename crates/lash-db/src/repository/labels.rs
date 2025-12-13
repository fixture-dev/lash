//! Label repository for managing labels and associations

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use lash_types::normalize;

use crate::error::DbResult;

/// A label record from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelRecord {
    /// Database ID
    pub id: i64,

    /// Label name (normalized)
    pub name: String,
}

/// Label statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelStats {
    /// Label name
    pub name: String,

    /// Number of tasks with this label
    pub task_count: i64,

    /// Number of files with this label
    pub file_count: i64,
}

/// Repository for label operations
pub struct LabelRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> LabelRepository<'conn> {
    /// Create a new label repository
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::connection::init_database;
    /// use lash_db::repository::LabelRepository;
    /// use std::path::Path;
    ///
    /// let conn = init_database(Path::new("/tmp/lash.db")).unwrap();
    /// let repo = LabelRepository::new(&conn);
    /// ```
    #[must_use]
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Get or create a label by name
    ///
    /// Returns the label ID. If the label doesn't exist, it is created.
    ///
    /// # Errors
    ///
    /// Returns error if insert fails
    pub fn get_or_create(&self, name: &str) -> DbResult<i64> {
        let normalized_name = normalize(name);

        // Try to get existing label
        if let Some(id) = self.get_label_id(&normalized_name)? {
            return Ok(id);
        }

        // Insert new label
        self.conn
            .execute("INSERT INTO labels (name) VALUES (?1)", [&normalized_name])?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get label ID by name
    fn get_label_id(&self, name: &str) -> DbResult<Option<i64>> {
        self.conn
            .query_row("SELECT id FROM labels WHERE name = ?1", [name], |row| {
                row.get(0)
            })
            .optional()
            .map_err(Into::into)
    }

    /// Associate a task with a label
    ///
    /// # Errors
    ///
    /// Returns error if insert fails
    pub fn add_task_label(&self, task_id: i64, label_id: i64) -> DbResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
            (task_id, label_id),
        )?;
        Ok(())
    }

    /// Associate a file with a label
    ///
    /// # Errors
    ///
    /// Returns error if insert fails
    pub fn add_file_label(&self, file_id: i64, label_id: i64) -> DbResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO file_labels (file_id, label_id) VALUES (?1, ?2)",
            (file_id, label_id),
        )?;
        Ok(())
    }

    /// Remove a task-label association
    ///
    /// # Errors
    ///
    /// Returns error if delete fails
    pub fn remove_task_label(&self, task_id: i64, label_id: i64) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM task_labels WHERE task_id = ?1 AND label_id = ?2",
            (task_id, label_id),
        )?;
        Ok(())
    }

    /// Remove a file-label association
    ///
    /// # Errors
    ///
    /// Returns error if delete fails
    pub fn remove_file_label(&self, file_id: i64, label_id: i64) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM file_labels WHERE file_id = ?1 AND label_id = ?2",
            (file_id, label_id),
        )?;
        Ok(())
    }

    /// Get all labels for a task
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_task_labels(&self, task_id: i64) -> DbResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.name
             FROM labels l
             JOIN task_labels tl ON l.id = tl.label_id
             WHERE tl.task_id = ?1
             ORDER BY l.name",
        )?;

        let labels = stmt
            .query_map([task_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(labels)
    }

    /// Get all labels for a file
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_file_labels(&self, file_id: i64) -> DbResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.name
             FROM labels l
             JOIN file_labels fl ON l.id = fl.label_id
             WHERE fl.file_id = ?1
             ORDER BY l.name",
        )?;

        let labels = stmt
            .query_map([file_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(labels)
    }

    /// List all labels in the system
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn list_all(&self) -> DbResult<Vec<LabelRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM labels ORDER BY name")?;

        let labels = stmt
            .query_map([], |row| {
                Ok(LabelRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(labels)
    }

    /// Set labels for a task (replaces existing labels)
    ///
    /// # Errors
    ///
    /// Returns error if operation fails
    pub fn set_task_labels(&self, task_id: i64, labels: &[String]) -> DbResult<()> {
        let tx = self.conn.unchecked_transaction()?;

        // Delete existing associations
        tx.execute("DELETE FROM task_labels WHERE task_id = ?1", [task_id])?;

        // Insert new associations
        for label_name in labels {
            let normalized = normalize(label_name);

            // Get or create label
            let label_id = match tx.query_row(
                "SELECT id FROM labels WHERE name = ?1",
                [&normalized],
                |row| row.get::<_, i64>(0),
            ) {
                Ok(id) => id,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    tx.execute("INSERT INTO labels (name) VALUES (?1)", [&normalized])?;
                    tx.last_insert_rowid()
                }
                Err(e) => return Err(e.into()),
            };

            tx.execute(
                "INSERT INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
                (task_id, label_id),
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Set labels for a file (replaces existing labels)
    ///
    /// # Errors
    ///
    /// Returns error if operation fails
    pub fn set_file_labels(&self, file_id: i64, labels: &[String]) -> DbResult<()> {
        let tx = self.conn.unchecked_transaction()?;

        // Delete existing associations
        tx.execute("DELETE FROM file_labels WHERE file_id = ?1", [file_id])?;

        // Insert new associations
        for label_name in labels {
            let normalized = normalize(label_name);

            // Get or create label
            let label_id = match tx.query_row(
                "SELECT id FROM labels WHERE name = ?1",
                [&normalized],
                |row| row.get::<_, i64>(0),
            ) {
                Ok(id) => id,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    tx.execute("INSERT INTO labels (name) VALUES (?1)", [&normalized])?;
                    tx.last_insert_rowid()
                }
                Err(e) => return Err(e.into()),
            };

            tx.execute(
                "INSERT INTO file_labels (file_id, label_id) VALUES (?1, ?2)",
                (file_id, label_id),
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get label statistics
    ///
    /// Returns task counts that include both direct task labels AND tasks
    /// that inherit the label from their file (matching `find_by_label` behavior).
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_label_stats(&self) -> DbResult<Vec<LabelStats>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.name,
                    (SELECT COUNT(DISTINCT t.id)
                     FROM tasks t
                     JOIN files f ON t.file_id = f.id
                     LEFT JOIN task_labels tl ON t.id = tl.task_id AND tl.label_id = l.id
                     LEFT JOIN file_labels fl ON f.id = fl.file_id AND fl.label_id = l.id
                     WHERE tl.label_id IS NOT NULL OR fl.label_id IS NOT NULL) as task_count,
                    (SELECT COUNT(*) FROM file_labels fl WHERE fl.label_id = l.id) as file_count
             FROM labels l
             ORDER BY l.name",
        )?;

        let stats = stmt
            .query_map([], |row| {
                Ok(LabelStats {
                    name: row.get(0)?,
                    task_count: row.get(1)?,
                    file_count: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(stats)
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
            description: None,
            description_agent_notes: Vec::new(),
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
            line_number: 0,
            metadata: TaskMetadata::default(),
            body: None,
            contextual_notes: Vec::new(),
        }
    }

    #[test]
    fn test_get_or_create_label() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = LabelRepository::new(&conn);

        // Create new label
        let id1 = repo.get_or_create("backend").unwrap();
        assert!(id1 > 0);

        // Get existing label (should return same ID)
        let id2 = repo.get_or_create("backend").unwrap();
        assert_eq!(id1, id2);

        // Different label
        let id3 = repo.get_or_create("frontend").unwrap();
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_add_task_label() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Test Task");
        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        let label_repo = LabelRepository::new(&conn);
        let label_id = label_repo.get_or_create("backend").unwrap();

        label_repo.add_task_label(task_db_id, label_id).unwrap();

        let labels = label_repo.get_task_labels(task_db_id).unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0], "backend");
    }

    #[test]
    fn test_add_file_label() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let label_repo = LabelRepository::new(&conn);
        let label_id = label_repo.get_or_create("docs").unwrap();

        label_repo.add_file_label(file_db_id, label_id).unwrap();

        let labels = label_repo.get_file_labels(file_db_id).unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0], "docs");
    }

    #[test]
    fn test_remove_task_label() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Test Task");
        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        let label_repo = LabelRepository::new(&conn);
        let label_id = label_repo.get_or_create("backend").unwrap();
        label_repo.add_task_label(task_db_id, label_id).unwrap();

        // Remove the label
        label_repo.remove_task_label(task_db_id, label_id).unwrap();

        let labels = label_repo.get_task_labels(task_db_id).unwrap();
        assert_eq!(labels.len(), 0);
    }

    #[test]
    fn test_set_task_labels() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Test Task");
        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        let label_repo = LabelRepository::new(&conn);

        // Set initial labels
        label_repo
            .set_task_labels(task_db_id, &["backend".to_string(), "rust".to_string()])
            .unwrap();

        let labels = label_repo.get_task_labels(task_db_id).unwrap();
        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&"backend".to_string()));
        assert!(labels.contains(&"rust".to_string()));

        // Replace labels
        label_repo
            .set_task_labels(task_db_id, &["frontend".to_string()])
            .unwrap();

        let labels = label_repo.get_task_labels(task_db_id).unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0], "frontend");
    }

    #[test]
    fn test_list_all_labels() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = LabelRepository::new(&conn);

        repo.get_or_create("backend").unwrap();
        repo.get_or_create("frontend").unwrap();
        repo.get_or_create("rust").unwrap();

        let labels = repo.list_all().unwrap();
        assert_eq!(labels.len(), 3);

        // Should be ordered alphabetically
        assert_eq!(labels[0].name, "backend");
        assert_eq!(labels[1].name, "frontend");
        assert_eq!(labels[2].name, "rust");
    }

    #[test]
    fn test_label_stats() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task1 = create_test_task("task1", "Task 1");
        let task2 = create_test_task("task2", "Task 2");
        let task_repo = TaskRepository::new(&conn);
        let task1_db_id = task_repo.insert(&task1, file_db_id, "test").unwrap();
        let task2_db_id = task_repo.insert(&task2, file_db_id, "test").unwrap();

        let label_repo = LabelRepository::new(&conn);

        // Add labels to tasks
        label_repo
            .set_task_labels(task1_db_id, &["backend".to_string()])
            .unwrap();
        label_repo
            .set_task_labels(task2_db_id, &["backend".to_string()])
            .unwrap();

        // Add label to file
        label_repo
            .set_file_labels(file_db_id, &["docs".to_string()])
            .unwrap();

        let stats = label_repo.get_label_stats().unwrap();

        let backend_stats = stats.iter().find(|s| s.name == "backend").unwrap();
        assert_eq!(backend_stats.task_count, 2);
        assert_eq!(backend_stats.file_count, 0);

        // docs is a file label - tasks in the file inherit it, so task_count = 2
        let docs_stats = stats.iter().find(|s| s.name == "docs").unwrap();
        assert_eq!(docs_stats.task_count, 2);
        assert_eq!(docs_stats.file_count, 1);
    }
}
