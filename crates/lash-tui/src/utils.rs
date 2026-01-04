//! Utility functions for TUI navigation, link detection, and text highlighting.

use lash_core::display::extract_link_path;
use lash_db::repository::{DependencyRepository, FileRepository, TaskRepository};
use lash_types::file::synthesize_file_id;
use lash_types::DependencyKind;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
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
#[allow(clippy::too_many_lines)]
pub fn get_link_target(conn: &Connection, task_id: i64) -> Option<LinkTarget> {
    tracing::debug!("get_link_target called for task_id={}", task_id);

    let dep_repo = DependencyRepository::new(conn);
    let task_repo = TaskRepository::new(conn);
    let file_repo = FileRepository::new(conn);

    // First, try to find from explicit dependencies
    if let Ok(deps) = dep_repo.get_dependencies(task_id) {
        tracing::debug!("Found {} dependencies for task_id={}", deps.len(), task_id);
        if let Some(cross_file_dep) = deps.iter().find(|dep| {
            matches!(
                dep.kind,
                DependencyKind::ExplicitPath | DependencyKind::ExplicitId
            )
        }) {
            tracing::debug!(
                "Found cross-file dep: kind={:?}, to_task_id={:?}",
                cross_file_dep.kind,
                cross_file_dep.to_task_id
            );
            // Check if dependency is resolved (has to_task_id)
            if let Some(to_task_id) = cross_file_dep.to_task_id {
                if let Ok(Some(target_task)) = task_repo.get_by_db_id(to_task_id) {
                    let full_id = cross_file_dep
                        .to_full_id
                        .clone()
                        .unwrap_or_else(|| target_task.full_id.clone());

                    tracing::debug!(
                        "Resolved via explicit dep: file_id={}, task_id={}",
                        target_task.file_id,
                        to_task_id
                    );
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
    let task = match task_repo.get_by_db_id(task_id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            tracing::debug!("Task not found for task_id={}", task_id);
            return None;
        }
        Err(e) => {
            tracing::debug!("Error getting task {}: {:?}", task_id, e);
            return None;
        }
    };
    tracing::debug!("Task title: {:?}", task.title);

    let Some(link_path) = extract_link_path(&task.title) else {
        tracing::debug!("No link path extracted from title");
        return None;
    };
    tracing::debug!("Extracted link path: {:?}", link_path);

    // Parse the link path - it may be "path/to/file.md" or "path/to/file.md#task-id"
    let (file_path_str, task_id_part) = if let Some(hash_idx) = link_path.find('#') {
        (&link_path[..hash_idx], Some(&link_path[hash_idx + 1..]))
    } else {
        (link_path.as_str(), None)
    };
    tracing::debug!(
        "Parsed link: file_path={:?}, task_id_part={:?}",
        file_path_str,
        task_id_part
    );

    // Try to find the file in the database
    // The link path is relative to the index file, so we need to resolve it
    // First, get the current file's path to determine the base directory
    let current_file = match file_repo.get_by_db_id(task.file_id) {
        Ok(Some(f)) => f,
        Ok(None) => {
            tracing::debug!("Current file not found for file_id={}", task.file_id);
            return None;
        }
        Err(e) => {
            tracing::debug!("Error getting current file {}: {:?}", task.file_id, e);
            return None;
        }
    };
    tracing::debug!(
        "Current file path: {:?}, file_id: {:?}",
        current_file.path,
        current_file.file_id
    );

    let current_dir = Path::new(&current_file.path)
        .parent()
        .unwrap_or(Path::new(""));
    tracing::debug!("Current dir: {:?}", current_dir);

    // Resolve the link path relative to the current file
    let resolved_path = current_dir.join(file_path_str);
    tracing::debug!("Resolved path: {:?}", resolved_path);

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
            tracing::debug!("get_by_path({:?}) failed, trying raw path", resolved_path);
            file_repo
                .get_by_path(Path::new(file_path_str))
                .ok()
                .flatten()
        })
        .or_else(|| {
            tracing::debug!(
                "get_by_path({:?}) failed, trying synthesized file_id",
                file_path_str
            );
            // Fallback: try to find by synthesized file_id from the path
            let synthesized_id = synthesize_file_id(Path::new(file_path_str));
            tracing::debug!("Synthesized file_id: {:?}", synthesized_id);
            file_repo.get_by_file_id(&synthesized_id).ok().flatten()
        });

    let target_file = if let Some(f) = target_file {
        tracing::debug!(
            "Found target file: id={}, path={:?}, file_id={:?}",
            f.id,
            f.path,
            f.file_id
        );
        f
    } else {
        tracing::debug!("Target file not found for path {:?}", file_path_str);
        return None;
    };

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

/// Highlight matching characters in text for autocomplete suggestions.
///
/// Takes a query string and text, and returns a `Line` with matching portions
/// highlighted using bold styling. Supports both substring and fuzzy matching.
///
/// For substring matches, the entire contiguous match is highlighted.
/// For fuzzy matches, individual matching characters are highlighted.
///
/// # Arguments
///
/// * `query` - The search query to match against
/// * `text` - The text to highlight matches in
/// * `highlight_style` - The style to use for highlighted portions (typically bold)
/// * `normal_style` - The style to use for non-highlighted portions
///
/// # Examples
///
/// ```
/// use lash_tui::utils::highlight_match;
/// use ratatui::style::{Modifier, Style};
///
/// let style = Style::default().add_modifier(Modifier::BOLD);
/// let normal = Style::default();
/// let line = highlight_match("feat", "feature", style, normal);
/// // Returns a Line with "feat" in bold and "ure" in normal style
/// ```
#[must_use]
pub fn highlight_match(
    query: &str,
    text: &str,
    highlight_style: Style,
    normal_style: Style,
) -> Line<'static> {
    if query.is_empty() {
        return Line::from(Span::styled(text.to_string(), normal_style));
    }

    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();

    // Try substring match first (case-insensitive)
    if let Some(start) = text_lower.find(&query_lower) {
        let end = start + query.len();
        let mut spans = Vec::new();

        if start > 0 {
            spans.push(Span::styled(text[..start].to_string(), normal_style));
        }
        spans.push(Span::styled(text[start..end].to_string(), highlight_style));
        if end < text.len() {
            spans.push(Span::styled(text[end..].to_string(), normal_style));
        }

        return Line::from(spans);
    }

    // Fallback to fuzzy character-by-character matching
    let mut spans = Vec::new();
    let mut query_chars = query_lower.chars().peekable();
    let mut current_segment = String::new();
    let mut in_match = false;

    for ch in text.chars() {
        let ch_lower = ch.to_lowercase().next().unwrap_or(ch);
        let should_highlight = if let Some(&next_query_char) = query_chars.peek() {
            if ch_lower == next_query_char {
                query_chars.next();
                true
            } else {
                false
            }
        } else {
            false
        };

        if should_highlight != in_match {
            // Transition: flush current segment
            if !current_segment.is_empty() {
                let style = if in_match {
                    highlight_style
                } else {
                    normal_style
                };
                spans.push(Span::styled(current_segment.clone(), style));
                current_segment.clear();
            }
            in_match = should_highlight;
        }

        current_segment.push(ch);
    }

    // Flush remaining segment
    if !current_segment.is_empty() {
        let style = if in_match {
            highlight_style
        } else {
            normal_style
        };
        spans.push(Span::styled(current_segment, style));
    }

    // If no spans were created, return the whole text unhighlighted
    if spans.is_empty() {
        Line::from(Span::styled(text.to_string(), normal_style))
    } else {
        Line::from(spans)
    }
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

    /// Helper to set up a test database with index file and linked files
    fn setup_test_db_with_markdown_links() -> (TempDir, PathBuf, Connection) {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let db_path = project_root.join(".lash").join("db.sqlite");

        // Create .lash directory
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        // Create target directory
        std::fs::create_dir_all(project_root.join("systems")).unwrap();

        // Initialize database
        let conn = init_database(&db_path).unwrap();

        // Create target file first
        let target_file = project_root.join("systems/physics.md");
        std::fs::write(
            &target_file,
            r"# Physics System

@id: systems.physics

## Tasks

- [ ] Implement collision detection
- [ ] Add gravity
",
        )
        .unwrap();

        // Create index file with markdown link to target
        let index_file = project_root.join("lash.index.md");
        std::fs::write(
            &index_file,
            r"# Project Index

@id: project-index

## Tasks

- [ ] [Physics System](systems/physics.md)
- [ ] Local task
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
    fn test_get_link_target_with_markdown_link() {
        let (_temp_dir, _db_path, conn) = setup_test_db_with_markdown_links();
        let task_repo = TaskRepository::new(&conn);
        let file_repo = FileRepository::new(&conn);

        // Get the index file
        let index_file = file_repo
            .get_by_file_id("project-index")
            .unwrap()
            .expect("index file should exist");

        // Get all tasks in the index file and find the one with the markdown link
        let tasks = task_repo.get_by_file(index_file.id).unwrap();
        let link_task = tasks
            .iter()
            .find(|t| t.title.contains("[Physics System]"))
            .expect("task with markdown link should exist");

        // Verify the link is detected
        assert!(
            is_cross_file_link(&conn, link_task.id),
            "task with markdown link should be detected as cross-file link"
        );

        // Verify the target is resolved
        let target = get_link_target(&conn, link_task.id);
        assert!(
            target.is_some(),
            "get_link_target should resolve markdown link"
        );

        let target = target.unwrap();

        // Verify target file is correct
        let target_file = file_repo.get_by_db_id(target.file_id).unwrap();
        assert!(target_file.is_some(), "target file should exist");
        let target_file = target_file.unwrap();
        assert_eq!(
            target_file.file_id, "systems.physics",
            "target file_id should match"
        );
    }

    #[test]
    fn test_is_cross_file_link_with_markdown_link() {
        let (_temp_dir, _db_path, conn) = setup_test_db_with_markdown_links();
        let task_repo = TaskRepository::new(&conn);
        let file_repo = FileRepository::new(&conn);

        // Get the index file
        let index_file = file_repo
            .get_by_file_id("project-index")
            .unwrap()
            .expect("index file should exist");

        // Get all tasks in the index file
        let tasks = task_repo.get_by_file(index_file.id).unwrap();
        let link_task = tasks
            .iter()
            .find(|t| t.title.contains("[Physics System]"))
            .expect("task with markdown link should exist");

        assert!(
            is_cross_file_link(&conn, link_task.id),
            "task with markdown link should be detected as cross-file link"
        );

        // Verify local task is NOT a cross-file link
        let local_task = tasks
            .iter()
            .find(|t| t.title == "Local task")
            .expect("local task should exist");

        assert!(
            !is_cross_file_link(&conn, local_task.id),
            "local task should not be detected as cross-file link"
        );
    }

    /// Test using the actual pixelquest fixture structure
    #[test]
    fn test_get_link_target_with_pixelquest_style_index() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let db_path = project_root.join(".lash").join("db.sqlite");

        // Create .lash directory and systems directory
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(project_root.join("systems")).unwrap();

        // Initialize database
        let conn = init_database(&db_path).unwrap();

        // Create target file similar to pixelquest fixture
        let target_file = project_root.join("systems/physics.md");
        std::fs::write(
            &target_file,
            r"# Physics & Collision System

@id: systems.physics
@labels: backend, physics, p0

## Tasks

- [x] Implement collision detection
- [ ] Add physics simulation
",
        )
        .unwrap();

        // Create index file similar to pixelquest fixture
        let index_file = project_root.join("lash.index.md");
        std::fs::write(
            &index_file,
            r"# PixelQuest: Retro 2D Platformer

@id: pixelquest

## Tasks

### Core Systems
Engine components and foundational infrastructure.

- [ ] [Physics & Collision](systems/physics.md) @id:`systems.physics` @labels:`backend, physics, p0`
- [ ] [Input Handling](systems/input.md) @id:`systems.input` @labels:`backend, input, p0`

### Gameplay Features
Player mechanics, AI, and game systems.

- [ ] [Player Movement](features/player-movement.md) @id:`features.player-movement`
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

        let task_repo = TaskRepository::new(&conn);
        let file_repo = FileRepository::new(&conn);

        // Get the index file
        let index_file_rec = file_repo
            .get_by_file_id("pixelquest")
            .unwrap()
            .expect("index file should exist");

        // Get all tasks in the index file
        let tasks = task_repo.get_by_file(index_file_rec.id).unwrap();
        eprintln!("Found {} tasks in index file", tasks.len());
        for task in &tasks {
            eprintln!("  Task: id={}, title={:?}", task.id, task.title);
        }

        // Find the physics task
        let physics_task = tasks
            .iter()
            .find(|t| t.title.contains("[Physics & Collision]"))
            .expect("task with Physics & Collision link should exist");

        eprintln!(
            "Physics task: id={}, file_id={}, title={:?}",
            physics_task.id, physics_task.file_id, physics_task.title
        );

        // Verify the link is detected
        assert!(
            is_cross_file_link(&conn, physics_task.id),
            "physics task should be detected as cross-file link"
        );

        // Verify the target is resolved
        let target = get_link_target(&conn, physics_task.id);
        assert!(
            target.is_some(),
            "get_link_target should resolve physics markdown link"
        );

        let target = target.unwrap();
        eprintln!(
            "Target: file_id={}, task_id={:?}, full_id={:?}",
            target.file_id, target.task_id, target.full_id
        );

        // Verify target file is correct
        let target_file_rec = file_repo.get_by_db_id(target.file_id).unwrap();
        assert!(target_file_rec.is_some(), "target file should exist");
        let target_file_rec = target_file_rec.unwrap();
        assert_eq!(
            target_file_rec.file_id, "systems.physics",
            "target file_id should match"
        );
    }
}
