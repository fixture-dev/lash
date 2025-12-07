//! Utility functions for TUI navigation and link detection.

use lash_core::display::extract_link_path;
use lash_db::repository::{DependencyRepository, FileRepository, TaskRepository};
use lash_types::file::synthesize_file_id;
use lash_types::DependencyKind;
use rusqlite::Connection;
use std::path::Path;

/// Information about a resolved cross-file link target.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkTarget {
    /// The file ID of the target file
    pub file_id: i64,
    /// The task ID within the target file (if specified)
    pub task_id: Option<i64>,
    /// The full ID string (e.g., "file-id#task-id")
    pub full_id: String,
}

/// Checks if a task is a cross-file link based on its dependencies or markdown link syntax.
///
/// Returns `true` if:
/// - The task has `ExplicitPath` or `ExplicitId` dependencies, OR
/// - The task title contains a markdown link `[text](path.md)`
///
/// # Examples
///
/// ```no_run
/// use lash_db::init_database;
/// use lash_tui::utils::is_cross_file_link;
/// use std::path::Path;
///
/// let conn = init_database(Path::new("/tmp/test.db")).unwrap();
/// let task_id = 1;
/// let is_link = is_cross_file_link(&conn, task_id);
/// ```
pub fn is_cross_file_link(conn: &Connection, task_id: i64) -> bool {
    // First check explicit dependencies
    let dep_repo = DependencyRepository::new(conn);
    if let Ok(deps) = dep_repo.get_dependencies(task_id) {
        if deps.iter().any(|dep| {
            matches!(
                dep.kind,
                DependencyKind::ExplicitPath | DependencyKind::ExplicitId
            )
        }) {
            return true;
        }
    }

    // Also check task title for markdown link syntax
    let task_repo = TaskRepository::new(conn);
    if let Ok(Some(task)) = task_repo.get_by_db_id(task_id) {
        if extract_link_path(&task.title).is_some() {
            return true;
        }
    }

    false
}

/// Gets the target of a cross-file link.
///
/// Returns `Some(LinkTarget)` if the task has a resolved cross-file dependency
/// or a markdown link in its title that resolves to an existing file.
/// Returns `None` if there is no cross-file link or it can't be resolved.
///
/// # Examples
///
/// ```no_run
/// use lash_db::init_database;
/// use lash_tui::utils::get_link_target;
/// use std::path::Path;
///
/// let conn = init_database(Path::new("/tmp/test.db")).unwrap();
/// let task_id = 1;
/// if let Some(target) = get_link_target(&conn, task_id) {
///     println!("Links to file: {}, task: {:?}", target.file_id, target.task_id);
/// }
/// ```
pub fn get_link_target(conn: &Connection, task_id: i64) -> Option<LinkTarget> {
    let dep_repo = DependencyRepository::new(conn);
    let task_repo = TaskRepository::new(conn);
    let file_repo = FileRepository::new(conn);

    // First, try to find from explicit dependencies
    if let Ok(deps) = dep_repo.get_dependencies(task_id) {
        if let Some(cross_file_dep) = deps.iter().find(|dep| {
            matches!(
                dep.kind,
                DependencyKind::ExplicitPath | DependencyKind::ExplicitId
            )
        }) {
            // Check if dependency is resolved (has to_task_id)
            if let Some(to_task_id) = cross_file_dep.to_task_id {
                if let Ok(Some(target_task)) = task_repo.get_by_db_id(to_task_id) {
                    let full_id = cross_file_dep
                        .to_full_id
                        .clone()
                        .unwrap_or_else(|| target_task.full_id.clone());

                    return Some(LinkTarget {
                        file_id: target_task.file_id,
                        task_id: Some(to_task_id),
                        full_id,
                    });
                }
            }
        }
    }

    // If no explicit dependency, try to resolve from markdown link in task title
    let task = task_repo.get_by_db_id(task_id).ok()??;
    let link_path = extract_link_path(&task.title)?;

    // Parse the link path - it may be "path/to/file.md" or "path/to/file.md#task-id"
    let (file_path_str, task_id_part) = if let Some(hash_idx) = link_path.find('#') {
        (&link_path[..hash_idx], Some(&link_path[hash_idx + 1..]))
    } else {
        (link_path.as_str(), None)
    };

    // Try to find the file in the database
    // The link path is relative to the index file, so we need to resolve it
    // First, get the current file's path to determine the base directory
    let current_file = file_repo.get_by_db_id(task.file_id).ok()??;
    let current_dir = Path::new(&current_file.path)
        .parent()
        .unwrap_or(Path::new(""));

    // Resolve the link path relative to the current file
    let resolved_path = current_dir.join(file_path_str);

    // Try to find the target file by path or file_id
    // The database stores paths relative to project root, so try:
    // 1. Resolved path (relative to current file's directory)
    // 2. Raw link path
    // 3. Synthesized file_id from the link path (e.g., "systems/physics.md" -> "systems.physics")
    let target_file = file_repo
        .get_by_path(&resolved_path)
        .ok()
        .flatten()
        .or_else(|| {
            file_repo
                .get_by_path(Path::new(file_path_str))
                .ok()
                .flatten()
        })
        .or_else(|| {
            // Fallback: try to find by synthesized file_id from the path
            let synthesized_id = synthesize_file_id(Path::new(file_path_str));
            file_repo.get_by_file_id(&synthesized_id).ok().flatten()
        })?;

    // If there's a task ID part, try to resolve it to a specific task
    let target_task_id = if let Some(task_local_id) = task_id_part {
        // Try to find task by local_id within the target file
        if let Ok(tasks) = task_repo.get_by_file(target_file.id) {
            tasks
                .into_iter()
                .find(|t| t.local_id == task_local_id)
                .map(|t| t.id)
        } else {
            None
        }
    } else {
        None
    };

    // Build the full_id
    let full_id = if let Some(tid) = task_id_part {
        format!("{}#{}", target_file.file_id, tid)
    } else {
        target_file.file_id.clone()
    };

    Some(LinkTarget {
        file_id: target_file.id,
        task_id: target_task_id,
        full_id,
    })
}

/// Gets the target file ID for navigation (simpler version for quick checks).
///
/// Returns the file ID of the target if the task has a resolved cross-file link,
/// or `None` otherwise.
///
/// # Examples
///
/// ```no_run
/// use lash_db::init_database;
/// use lash_tui::utils::get_target_file_id;
/// use std::path::Path;
///
/// let conn = init_database(Path::new("/tmp/test.db")).unwrap();
/// let task_id = 1;
/// if let Some(file_id) = get_target_file_id(&conn, task_id) {
///     println!("Target file ID: {}", file_id);
/// }
/// ```
pub fn get_target_file_id(conn: &Connection, task_id: i64) -> Option<i64> {
    get_link_target(conn, task_id).map(|target| target.file_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_db::{init_database, repository::DependencyRepository, Indexer, IndexerConfig};
    use lash_types::LashConfig;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper to set up a test database with some tasks
    fn setup_test_db_with_tasks() -> (TempDir, PathBuf, Connection) {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let db_path = project_root.join(".lash").join("db.sqlite");

        // Create .lash directory
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        // Initialize database
        let conn = init_database(&db_path).unwrap();

        // Create a simple task file
        let task_file = project_root.join("lash.index.md");
        std::fs::write(
            &task_file,
            r"# Test File

@id: test-file

## Tasks

- [ ] Local task
- [ ] Another task
- [ ] Parent task
  - [ ] Child task
",
        )
        .unwrap();

        // Index the project
        let indexer_config = IndexerConfig::new(project_root)
            .with_incremental(false)
            .with_progress(false);
        let parser_config = LashConfig::default();
        let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
        indexer.index_project().unwrap();

        (temp_dir, db_path, conn)
    }

    #[test]
    fn test_is_cross_file_link_with_explicit_path() {
        let (_temp_dir, _db_path, conn) = setup_test_db_with_tasks();
        let task_repo = TaskRepository::new(&conn);
        let dep_repo = DependencyRepository::new(&conn);

        // Get a task to add a cross-file dependency to
        let local_task = task_repo
            .get_by_full_id("test-file#local-task")
            .unwrap()
            .expect("local-task should exist");

        // Manually insert an ExplicitPath dependency (simulating what the parser will do
        // when markdown link parsing is implemented)
        dep_repo
            .insert(
                local_task.id,
                None, // Unresolved
                &DependencyKind::ExplicitPath,
                Some("target-file.md"),
            )
            .unwrap();

        assert!(
            is_cross_file_link(&conn, local_task.id),
            "task with ExplicitPath dependency should be detected as cross-file link"
        );
    }

    #[test]
    fn test_is_cross_file_link_false_for_local_task() {
        let (_temp_dir, _db_path, conn) = setup_test_db_with_tasks();
        let task_repo = TaskRepository::new(&conn);

        // Get a task which has no cross-file dependencies
        let local_task = task_repo
            .get_by_full_id("test-file#local-task")
            .unwrap()
            .expect("local-task should exist");

        assert!(
            !is_cross_file_link(&conn, local_task.id),
            "local-task should not be detected as cross-file link"
        );
    }

    #[test]
    fn test_is_cross_file_link_false_for_hierarchy() {
        let (_temp_dir, _db_path, conn) = setup_test_db_with_tasks();
        let task_repo = TaskRepository::new(&conn);

        // Get the child-task which only has Hierarchy dependency
        let child_task = task_repo
            .get_by_full_id("test-file#child-task")
            .unwrap()
            .expect("child-task should exist");

        assert!(
            !is_cross_file_link(&conn, child_task.id),
            "child-task should not be detected as cross-file link (hierarchy only)"
        );
    }

    #[test]
    fn test_get_link_target_none_for_unresolved() {
        let (_temp_dir, _db_path, conn) = setup_test_db_with_tasks();
        let task_repo = TaskRepository::new(&conn);
        let dep_repo = DependencyRepository::new(&conn);

        // Get a task and add an unresolved cross-file dependency
        let local_task = task_repo
            .get_by_full_id("test-file#local-task")
            .unwrap()
            .expect("local-task should exist");

        dep_repo
            .insert(
                local_task.id,
                None, // Unresolved
                &DependencyKind::ExplicitPath,
                Some("missing-file.md"),
            )
            .unwrap();

        // Should return None for unresolved link
        assert!(
            get_link_target(&conn, local_task.id).is_none(),
            "unresolved link should return None"
        );
    }

    #[test]
    fn test_get_link_target_none_for_local_task() {
        let (_temp_dir, _db_path, conn) = setup_test_db_with_tasks();
        let task_repo = TaskRepository::new(&conn);

        // Get a task with no dependencies
        let another_task = task_repo
            .get_by_full_id("test-file#another-task")
            .unwrap()
            .expect("another-task should exist");

        // Should return None for local task
        assert!(
            get_link_target(&conn, another_task.id).is_none(),
            "task without dependencies should not have a link target"
        );
    }

    #[test]
    fn test_get_target_file_id_none_for_unresolved() {
        let (_temp_dir, _db_path, conn) = setup_test_db_with_tasks();
        let task_repo = TaskRepository::new(&conn);
        let dep_repo = DependencyRepository::new(&conn);

        // Get a task and add an unresolved dependency
        let local_task = task_repo
            .get_by_full_id("test-file#local-task")
            .unwrap()
            .expect("local-task should exist");

        dep_repo
            .insert(
                local_task.id,
                None, // Unresolved
                &DependencyKind::ExplicitPath,
                Some("missing-file.md"),
            )
            .unwrap();

        // Should return None for unresolved link
        assert!(
            get_target_file_id(&conn, local_task.id).is_none(),
            "unresolved link should not have a target file ID"
        );
    }

    #[test]
    fn test_get_target_file_id_none_for_local() {
        let (_temp_dir, _db_path, conn) = setup_test_db_with_tasks();
        let task_repo = TaskRepository::new(&conn);

        // Get a task with no dependencies
        let another_task = task_repo
            .get_by_full_id("test-file#another-task")
            .unwrap()
            .expect("another-task should exist");

        assert!(
            get_target_file_id(&conn, another_task.id).is_none(),
            "task without dependencies should not have a target file ID"
        );
    }
}
