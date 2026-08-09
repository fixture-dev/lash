//! Resolve a user-supplied task handle to a task record.
//!
//! Commands that take a "Task ID" (`show`, `start`, `complete`) historically
//! only accepted the `file-id#task-id` full id (the truncated heading slug),
//! never the `@id` the tool displays most prominently. This helper accepts
//! either, so the natural loop — read an `@id` from `lash status`, run
//! `lash show <that-id>` — works (GitHub issue #14).
//!
//! Resolution order for a target string:
//! 1. Exact `full_id` (`file-id#task-id`) match.
//! 2. Exact `@id` (`local_id`) match — unique across the project.
//! 3. Otherwise not found (ambiguous `@id`s are reported as candidates).

use lash_db::repository::tasks::TaskRecord;
use lash_db::TaskRepository;

/// Outcome of resolving a task target that did not uniquely identify a task.
#[derive(Debug)]
pub enum TargetError {
    /// No task matched by full id or `@id`.
    NotFound,
    /// The `@id` matched tasks in more than one file; the caller must
    /// disambiguate using one of the listed full ids.
    Ambiguous(Vec<String>),
    /// The database query failed.
    Db(String),
}

/// Resolve `target` to a single [`TaskRecord`].
///
/// # Errors
///
/// Returns [`TargetError`] when the target does not resolve to exactly one task.
pub fn resolve_task_target(
    task_repo: &TaskRepository,
    target: &str,
) -> Result<TaskRecord, TargetError> {
    // 1. Full id (file-id#task-id).
    match task_repo.get_by_full_id(target) {
        Ok(Some(task)) => return Ok(task),
        Ok(None) => {}
        Err(e) => return Err(TargetError::Db(e.to_string())),
    }

    // 2. Bare @id (local_id).
    match task_repo.get_by_local_id(target) {
        Ok(mut matches) if matches.len() == 1 => Ok(matches.remove(0)),
        Ok(matches) if matches.is_empty() => Err(TargetError::NotFound),
        Ok(matches) => Err(TargetError::Ambiguous(
            matches.into_iter().map(|t| t.full_id).collect(),
        )),
        Err(e) => Err(TargetError::Db(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_db::{init_database, FileRepository};
    use lash_types::{FileMetadata, Task, TaskFile, TaskMetadata, TaskStatus, TaskTree};
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tempfile::NamedTempFile;

    fn test_file(id: &str) -> TaskFile {
        TaskFile {
            path: PathBuf::from(format!("{id}.md")),
            title: "T".to_string(),
            id: id.to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "h".to_string(),
            mtime: SystemTime::now(),
        }
    }

    fn test_task(id: &str, order: usize) -> Task {
        Task {
            id: id.to_string(),
            has_explicit_id: true,
            title: id.to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: order,
            line_number: 0,
            annotation_line_count: 0,
            metadata: TaskMetadata::default(),
            body: None,
            contextual_notes: Vec::new(),
        }
    }

    #[test]
    fn resolves_full_id_and_bare_id_reports_ambiguity_and_missing() {
        let db = NamedTempFile::new().unwrap();
        let conn = init_database(db.path()).unwrap();
        let files = FileRepository::new(&conn);
        let tasks = TaskRepository::new(&conn);

        let fa = files.insert(&test_file("a")).unwrap();
        let fb = files.insert(&test_file("b")).unwrap();
        tasks.insert(&test_task("unique", 0), fa, "a").unwrap();
        tasks.insert(&test_task("shared", 1), fa, "a").unwrap();
        tasks.insert(&test_task("shared", 0), fb, "b").unwrap();

        // Full id resolves directly.
        assert_eq!(
            resolve_task_target(&tasks, "a#unique").unwrap().full_id,
            "a#unique"
        );
        // Bare unique @id resolves.
        assert_eq!(
            resolve_task_target(&tasks, "unique").unwrap().full_id,
            "a#unique"
        );
        // Ambiguous bare @id lists candidates.
        match resolve_task_target(&tasks, "shared") {
            Err(TargetError::Ambiguous(c)) => assert_eq!(c.len(), 2),
            _ => panic!("expected Ambiguous"),
        }
        // Missing target is NotFound.
        assert!(matches!(
            resolve_task_target(&tasks, "ghost"),
            Err(TargetError::NotFound)
        ));
    }
}
