//! Canonical `@depends-on` reference resolution
//!
//! This module is the **single source of truth** for turning a `@depends-on`
//! reference string into concrete target task IDs. Historically the linter,
//! the graph resolver, and the CLI each resolved references differently, so
//! the same reference could lint clean in one surface and fail in another
//! (GitHub issues #15 and #19). Routing every surface through
//! [`resolve_reference`] keeps them in agreement.
//!
//! # Supported forms
//!
//! Given a project of parsed [`TaskFile`]s, a reference resolves in this order:
//!
//! | Form | Example | Meaning |
//! |------|---------|---------|
//! | bare `@id` | `base-task` | file with that id (file-level), else task with that `@id` |
//! | same-file task | `#task:base-task` or `#base-task` | task in the current file |
//! | file-id + task | `repro-file#task:base-task` or `repro-file#base-task` | task in another file (by id) |
//! | path + task | `core/api.md#task:setup` | task in another file (by path) |
//! | path (file-level) | `core/api.md` | every top-level task in that file |
//!
//! The `task:` prefix on a fragment is optional everywhere — both the
//! documented `file.md#task:id` form and the natural `file.md#id` form work.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lash_types::dependency::make_full_id;
use lash_types::TaskFile;

/// A successfully resolved reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefResolution {
    /// Reference points at a single task; the string is its `file-id#task-id`.
    Task(String),
    /// File-level reference; every top-level task's `file-id#task-id`.
    File(Vec<String>),
}

impl RefResolution {
    /// Flatten to the list of resolved target full IDs.
    #[must_use]
    pub fn full_ids(self) -> Vec<String> {
        match self {
            Self::Task(id) => vec![id],
            Self::File(ids) => ids,
        }
    }
}

/// Why a reference could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefError {
    /// The file part (id or path) did not match any file in the project.
    FileNotFound {
        /// The file id or path as written in the reference.
        reference: String,
    },
    /// The file was found but does not contain the referenced task.
    TaskNotFound {
        /// Human-readable label for the file that was searched.
        file_label: String,
        /// The task id that was looked for.
        task: String,
        /// A few available task ids in that file, for a helpful message.
        available: Vec<String>,
    },
}

/// Collect the `file-id#task-id` of every top-level (depth 0) task in a file.
fn file_level_targets(file: &TaskFile) -> Vec<String> {
    file.tasks
        .tasks()
        .iter()
        .filter(|t| t.depth == 0)
        .map(|t| make_full_id(&file.id, &t.id))
        .collect()
}

/// First few task ids in a file, for error messages.
fn available_task_ids(file: &TaskFile) -> Vec<String> {
    file.tasks
        .tasks()
        .iter()
        .map(|t| t.id.clone())
        .take(5)
        .collect()
}

/// Resolve a task inside a known target file, or produce a `TaskNotFound`.
fn resolve_task_in_file(file: &TaskFile, task_key: &str) -> Result<RefResolution, RefError> {
    // Accept both `task:id` and `id`.
    let key = task_key.strip_prefix("task:").unwrap_or(task_key);
    if file.tasks.get_task(key).is_some() {
        Ok(RefResolution::Task(make_full_id(&file.id, key)))
    } else {
        Err(RefError::TaskNotFound {
            file_label: file.id.clone(),
            task: key.to_string(),
            available: available_task_ids(file),
        })
    }
}

/// Does this reference's file part look like a path rather than a file id?
fn looks_like_path(s: &str) -> bool {
    s.contains('/') || s.contains('\\') || s.to_lowercase().ends_with(".md")
}

/// Resolve a single `@depends-on` reference against the project's files.
///
/// * `target` — the reference string (already comma-split and trimmed).
/// * `source_path` — the current file's path (the map key), for same-file refs.
/// * `_source_id` — the current file's `@id` (reserved for future diagnostics).
/// * `files` — every parsed file in the project, keyed by their map path.
/// * `resolve_path` — resolves a path reference (relative to the source file)
///   to the key used in `files` (root-relative, normalized).
///
/// Directory references (trailing `/`) are **not** handled here; callers detect
/// and skip them before calling.
///
/// # Errors
///
/// Returns [`RefError`] when the file or task part cannot be found.
#[allow(clippy::implicit_hasher)]
pub fn resolve_reference(
    target: &str,
    source_path: &Path,
    _source_id: &str,
    files: &HashMap<PathBuf, TaskFile>,
    resolve_path: impl Fn(&str) -> PathBuf,
) -> Result<RefResolution, RefError> {
    let target = target.trim();

    // Split into a file part and an optional task fragment.
    let (before, fragment) = match target.split_once('#') {
        Some((b, f)) => (b, Some(f)),
        None => (target, None),
    };

    if let Some(fragment) = fragment {
        // --- Task reference: `<file>#<task>` (file part may be empty) ---
        if before.is_empty() {
            // Same-file reference (`#task:id` / `#id`).
            let file = files
                .get(source_path)
                .ok_or_else(|| RefError::FileNotFound {
                    reference: source_path.display().to_string(),
                })?;
            return resolve_task_in_file(file, fragment);
        }

        // Cross-file: resolve the file part by id, then by path.
        if let Some(file) = files.values().find(|f| f.id == before) {
            return resolve_task_in_file(file, fragment);
        }
        if looks_like_path(before) {
            let resolved = resolve_path(before);
            if let Some(file) = files.get(&resolved) {
                return resolve_task_in_file(file, fragment);
            }
        }
        return Err(RefError::FileNotFound {
            reference: before.to_string(),
        });
    }

    // --- Bare reference (no `#`): file-level, or a task by @id ---

    // 1. A file id → depend on the whole file.
    if let Some(file) = files.values().find(|f| f.id == target) {
        return Ok(RefResolution::File(file_level_targets(file)));
    }

    // 2. A path to a file → depend on the whole file.
    if looks_like_path(target) {
        let resolved = resolve_path(target);
        if let Some(file) = files.get(&resolved) {
            return Ok(RefResolution::File(file_level_targets(file)));
        }
        return Err(RefError::FileNotFound {
            reference: target.to_string(),
        });
    }

    // 3. A task `@id` — prefer the same file, then a unique match project-wide.
    if let Some(file) = files.get(source_path) {
        if file.tasks.get_task(target).is_some() {
            return Ok(RefResolution::Task(make_full_id(&file.id, target)));
        }
    }
    let mut matches = files
        .values()
        .filter(|f| f.tasks.get_task(target).is_some())
        .collect::<Vec<_>>();
    matches.sort_by(|a, b| a.id.cmp(&b.id));
    match matches.as_slice() {
        [file] => Ok(RefResolution::Task(make_full_id(&file.id, target))),
        [] => Err(RefError::FileNotFound {
            reference: target.to_string(),
        }),
        // Ambiguous bare id across files: pick deterministically (first by
        // file id). Ambiguity is separately lintable; resolving avoids a
        // spurious broken-link error for a reference that does exist.
        [first, ..] => Ok(RefResolution::Task(make_full_id(&first.id, target))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::file::FileMetadata;
    use lash_types::task::{TaskBuilder, TaskTree};
    use std::time::SystemTime;

    fn file_with_tasks(path: &str, id: &str, task_ids: &[&str]) -> (PathBuf, TaskFile) {
        let mut tree = TaskTree::new();
        for tid in task_ids {
            tree.add_task(
                TaskBuilder::new(format!("Task {tid}"))
                    .id(*tid)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        }
        let pb = PathBuf::from(path);
        let file = TaskFile {
            path: pb.clone(),
            title: "T".to_string(),
            id: id.to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: tree,
            hash: "h".to_string(),
            mtime: SystemTime::now(),
        };
        (pb, file)
    }

    fn project(files: Vec<(PathBuf, TaskFile)>) -> HashMap<PathBuf, TaskFile> {
        files.into_iter().collect()
    }

    /// Identity path resolver (paths are already root-relative in tests).
    fn id_resolver(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn resolves_bare_task_id_same_file() {
        let (p, f) = file_with_tasks("tasks.md", "repro", &["base-task", "dep"]);
        let files = project(vec![(p.clone(), f)]);
        let r = resolve_reference("base-task", &p, "repro", &files, id_resolver).unwrap();
        assert_eq!(r, RefResolution::Task("repro#base-task".to_string()));
    }

    #[test]
    fn resolves_samefile_task_form() {
        let (p, f) = file_with_tasks("tasks.md", "repro", &["base-task"]);
        let files = project(vec![(p.clone(), f)]);
        let r = resolve_reference("#task:base-task", &p, "repro", &files, id_resolver).unwrap();
        assert_eq!(r, RefResolution::Task("repro#base-task".to_string()));
        // Also without the task: prefix.
        let r2 = resolve_reference("#base-task", &p, "repro", &files, id_resolver).unwrap();
        assert_eq!(r2, RefResolution::Task("repro#base-task".to_string()));
    }

    #[test]
    fn resolves_file_id_task_forms() {
        let (p, f) = file_with_tasks("tasks.md", "repro", &["base-task"]);
        let files = project(vec![(p.clone(), f)]);
        // Documented `file-id#task:id` form.
        let r =
            resolve_reference("repro#task:base-task", &p, "repro", &files, id_resolver).unwrap();
        assert_eq!(r, RefResolution::Task("repro#base-task".to_string()));
        // Natural `file-id#id` form.
        let r2 = resolve_reference("repro#base-task", &p, "repro", &files, id_resolver).unwrap();
        assert_eq!(r2, RefResolution::Task("repro#base-task".to_string()));
    }

    #[test]
    fn resolves_path_task_form() {
        let (p1, f1) = file_with_tasks("core/api.md", "core.api", &["setup"]);
        let (p2, f2) = file_with_tasks("feat.md", "feat", &["x"]);
        let files = project(vec![(p1, f1), (p2.clone(), f2)]);
        let r =
            resolve_reference("core/api.md#task:setup", &p2, "feat", &files, id_resolver).unwrap();
        assert_eq!(r, RefResolution::Task("core.api#setup".to_string()));
    }

    #[test]
    fn resolves_cross_file_bare_task_id() {
        let (p1, f1) = file_with_tasks("a.md", "a", &["only-here"]);
        let (p2, f2) = file_with_tasks("b.md", "b", &["other"]);
        let files = project(vec![(p1, f1), (p2.clone(), f2)]);
        // `only-here` lives in file a, referenced from b.
        let r = resolve_reference("only-here", &p2, "b", &files, id_resolver).unwrap();
        assert_eq!(r, RefResolution::Task("a#only-here".to_string()));
    }

    #[test]
    fn resolves_bare_file_id_to_file_level() {
        let (p1, f1) = file_with_tasks("a.md", "a", &["t1", "t2"]);
        let (p2, f2) = file_with_tasks("b.md", "b", &["s"]);
        let files = project(vec![(p1, f1), (p2.clone(), f2)]);
        let r = resolve_reference("a", &p2, "b", &files, id_resolver).unwrap();
        match r {
            RefResolution::File(ids) => {
                assert!(ids.contains(&"a#t1".to_string()));
                assert!(ids.contains(&"a#t2".to_string()));
            }
            RefResolution::Task(_) => panic!("expected file-level"),
        }
    }

    #[test]
    fn missing_file_errors() {
        let (p, f) = file_with_tasks("tasks.md", "repro", &["base"]);
        let files = project(vec![(p.clone(), f)]);
        let err = resolve_reference("nope#task:x", &p, "repro", &files, id_resolver).unwrap_err();
        assert!(matches!(err, RefError::FileNotFound { .. }));
    }

    #[test]
    fn missing_task_errors_with_available() {
        let (p, f) = file_with_tasks("tasks.md", "repro", &["base"]);
        let files = project(vec![(p.clone(), f)]);
        let err =
            resolve_reference("repro#task:ghost", &p, "repro", &files, id_resolver).unwrap_err();
        match err {
            RefError::TaskNotFound {
                task, available, ..
            } => {
                assert_eq!(task, "ghost");
                assert!(available.contains(&"base".to_string()));
            }
            RefError::FileNotFound { .. } => panic!("expected TaskNotFound"),
        }
    }
}
