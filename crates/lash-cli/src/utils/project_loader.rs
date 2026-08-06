//! Load an entire project into the resolver-shaped map used by
//! [`lash_core::dependency::reference::resolve_reference`].
//!
//! Several CLI surfaces need to validate `@depends-on` references against
//! the *current* on-disk Markdown rather than the (possibly stale) `SQLite`
//! index: `lash complete` gates completion on unmet dependencies (GitHub
//! issue #17) and `lash add --depends-on` refuses to write a dangling
//! reference (GitHub issue #27). Both need the same "parse every file, keyed
//! by its root-relative path" map, so it lives here once instead of being
//! duplicated per command.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lash_core::parser::parse_file;
use lash_types::config::LashConfig;
use lash_types::{make_full_id, Task, TaskFile};

use crate::utils::file_discovery::discover_markdown_files;

/// Reparse every task file under `project_root` into a resolver-shaped map.
///
/// Unparseable files are skipped — `lash lint` surfaces those separately, so
/// callers of this helper only need to reason about files that parse cleanly.
#[must_use]
pub fn load_project(project_root: &Path) -> (LashConfig, HashMap<PathBuf, TaskFile>) {
    let config = LashConfig::from_root(project_root).unwrap_or_default();
    let mut files: HashMap<PathBuf, TaskFile> = HashMap::new();
    if let Ok(markdown_files) = discover_markdown_files(&[project_root.to_path_buf()], true) {
        for path in &markdown_files {
            if let Ok(file) = parse_file(path, &config) {
                let relative = path
                    .strip_prefix(project_root)
                    .unwrap_or(path)
                    .to_path_buf();
                files.insert(relative, file);
            }
        }
    }
    (config, files)
}

/// Find the parsed task (and its file path) whose full id equals `full_id`.
///
/// Used by callers that resolve a `@depends-on` target (or a task's own id)
/// against the freshly-parsed project returned by [`load_project`] — e.g.
/// `lash complete`'s unmet-dependency gate and `lash show`'s dependency
/// status display — rather than the possibly-stale `SQLite` index.
#[must_use]
pub fn find_task_by_full_id<'a>(
    project: &'a HashMap<PathBuf, TaskFile>,
    full_id: &str,
) -> Option<(&'a PathBuf, &'a TaskFile, &'a Task)> {
    for (path, file) in project {
        for task in file.tasks.tasks() {
            if make_full_id(&file.id, &task.id) == full_id {
                return Some((path, file, task));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_project_parses_markdown_files_keyed_by_relative_path() {
        let temp = tempfile::TempDir::new().unwrap();
        fs::write(
            temp.path().join("tasks.md"),
            "# Tasks\n\n@id: tasks\n\n## Tasks\n\n- [ ] Task A\n  @id: task-a\n",
        )
        .unwrap();

        let (_, files) = load_project(temp.path());
        assert_eq!(files.len(), 1);
        let file = files.get(Path::new("tasks.md")).expect("file indexed");
        assert_eq!(file.id, "tasks");
        assert_eq!(file.tasks.tasks().len(), 1);
    }

    #[test]
    fn load_project_skips_unparseable_files() {
        let temp = tempfile::TempDir::new().unwrap();
        fs::write(temp.path().join("bad.md"), "- [*] not a valid checkbox\n").unwrap();

        let (_, files) = load_project(temp.path());
        assert!(files.is_empty());
    }

    #[test]
    fn find_task_by_full_id_locates_task_across_files() {
        let temp = tempfile::TempDir::new().unwrap();
        fs::write(
            temp.path().join("tasks.md"),
            "# Tasks\n\n@id: tasks\n\n## Tasks\n\n- [ ] Task A\n  @id: task-a\n",
        )
        .unwrap();

        let (_, files) = load_project(temp.path());
        let found = find_task_by_full_id(&files, "tasks#task-a").expect("task found");
        assert_eq!(found.2.title, "Task A");

        assert!(find_task_by_full_id(&files, "tasks#ghost").is_none());
    }
}
