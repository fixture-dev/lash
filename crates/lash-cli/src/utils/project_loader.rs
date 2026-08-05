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
use lash_types::TaskFile;

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
}
