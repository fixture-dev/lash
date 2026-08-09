// LashError is intentionally rich with context; size-of-Err is not a concern here.
#![allow(clippy::result_large_err)]

//! Single-writer store coordinating Markdown task-file mutations.
//!
//! The `Store` is the only thing in the system that writes to task files on
//! disk. It records a hash of every byte sequence it writes so that, when an
//! external file watcher later reports a change to that file, the store can
//! tell whether the change is its own write echoing back (drop silently) or a
//! genuine external edit (treat as `FileReloaded`).
//!
//! See `docs/live-tui-updates.md` for the broader architecture.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lash_types::creation::TaskCreationRequest;
use lash_types::error::codes;
use lash_types::{LashConfig, LashError, Result, TaskStatus};

/// A mutation request submitted to the store.
#[derive(Debug, Clone)]
pub enum Mutation {
    /// Toggle one task's checkbox from `old_status` to `new_status` in
    /// `absolute_path`. The task is identified by its title within the file.
    SetTaskStatus {
        /// Absolute path to the task file.
        absolute_path: PathBuf,
        /// Title text of the task, used to locate the matching checkbox line.
        task_title: String,
        /// Status the task is transitioning from.
        old_status: TaskStatus,
        /// Status the task is transitioning to.
        new_status: TaskStatus,
    },
    /// Create a new task — either appended to an existing file or in a fresh
    /// file. Delegates to `crate::creation::TaskCreationService` for the
    /// actual emission and then records a hash of the resulting file so the
    /// watcher's echo is dropped.
    ///
    /// Boxed because `TaskCreationRequest` + `LashConfig` is hundreds of bytes
    /// — without the indirection this would bloat the enum.
    CreateTask(Box<CreateTaskMutation>),
}

/// Payload for `Mutation::CreateTask`.
#[derive(Debug, Clone)]
pub struct CreateTaskMutation {
    /// The validated creation request.
    pub request: TaskCreationRequest,
    /// Parser/format config to use for emission.
    pub config: LashConfig,
}

/// Effects emitted by the store as a result of either an `apply` call or an
/// observed external change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateDelta {
    /// A task's status changed (either via `apply` or as a result of an
    /// external reload that observed a status flip).
    TaskStatusChanged {
        /// Absolute path of the file whose task changed.
        absolute_path: PathBuf,
        /// Task title (used in lieu of a parsed AST for now).
        task_title: String,
        /// Previous status.
        old: TaskStatus,
        /// New status.
        new: TaskStatus,
    },
    /// An external process rewrote a file; consumers should re-parse and
    /// re-render. The source of truth here is the new on-disk content.
    FileReloaded {
        /// Absolute path of the file that changed externally.
        absolute_path: PathBuf,
    },
    /// The watcher dropped events because its channel filled, so which files
    /// changed is no longer known. Consumers should reindex the whole project
    /// and reload the current view.
    FullReload,
    /// A new task was created (either in an existing file or by creating a
    /// new file). Consumers should reindex and reload the affected file.
    TaskCreated {
        /// Absolute path of the file containing the new task.
        absolute_path: PathBuf,
        /// Local task id (the bit after `#` in a full id).
        task_id: String,
        /// True if `absolute_path` was created by this mutation; false if the
        /// task was appended to an already-existing file.
        is_new_file: bool,
    },
}

/// The store's single-writer surface.
#[derive(Debug, Default)]
pub struct Store {
    last_written_hash: HashMap<PathBuf, [u8; 32]>,
}

impl Store {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a mutation: rewrite the affected file atomically, record the hash
    /// of what was written, and emit a delta describing the change.
    ///
    /// # Errors
    ///
    /// Returns an error if the target file cannot be read, the expected task
    /// title cannot be found in the file, or the atomic write fails.
    pub fn apply(&mut self, mutation: Mutation) -> Result<Vec<StateDelta>> {
        match mutation {
            Mutation::SetTaskStatus {
                absolute_path,
                task_title,
                old_status,
                new_status,
            } => {
                let original =
                    std::fs::read_to_string(&absolute_path).map_err(|e| LashError::IO {
                        code: codes::E_IO_READ_ERROR,
                        message: format!("failed to read {}", absolute_path.display()),
                        path: Some(absolute_path.clone()),
                        io_error: Some(e.to_string()),
                    })?;

                let updated = rewrite_checkbox(&original, &task_title, old_status, new_status)
                    .ok_or_else(|| LashError::Internal {
                        code: codes::E_INTERNAL,
                        message: format!(
                            "task '{task_title}' with status {old_status} not found in {}",
                            absolute_path.display()
                        ),
                        context: None,
                    })?;

                let bytes = updated.as_bytes();
                let hash = blake3::hash(bytes);
                self.last_written_hash
                    .insert(absolute_path.clone(), *hash.as_bytes());

                write_atomic(&absolute_path, bytes)?;

                Ok(vec![StateDelta::TaskStatusChanged {
                    absolute_path,
                    task_title,
                    old: old_status,
                    new: new_status,
                }])
            }
            Mutation::CreateTask(payload) => {
                let CreateTaskMutation { request, config } = *payload;
                let service = crate::creation::TaskCreationService::new(config);
                let result = service.create_task(&request).map_err(|errors| {
                    let summary = errors
                        .iter()
                        .map(lash_types::TaskCreationError::message)
                        .collect::<Vec<_>>()
                        .join("; ");
                    LashError::Internal {
                        code: codes::E_INTERNAL,
                        message: format!("task creation failed: {summary}"),
                        context: None,
                    }
                })?;

                // The service writes atomically itself, but doesn't run through
                // `write_atomic` here, so we re-read the resulting file and
                // hash it. That hash is what `handle_external_change` will
                // compare against when the watcher echoes our own write back.
                if let Ok(bytes) = std::fs::read(&result.file_path) {
                    self.last_written_hash
                        .insert(result.file_path.clone(), *blake3::hash(&bytes).as_bytes());
                }

                Ok(vec![StateDelta::TaskCreated {
                    absolute_path: result.file_path,
                    task_id: result.task_id,
                    is_new_file: result.is_new_file,
                }])
            }
        }
    }

    /// Handle the watcher having dropped events because its channel filled.
    ///
    /// Which files changed is unknowable at that point, so the only honest
    /// answer is to reload everything. The write-hash table is cleared as part
    /// of that: its entries exist to recognize the watcher echoing one of our
    /// own writes, and an echo we never received is one we can no longer match.
    /// Keeping a stale entry would let a genuine later edit that happens to
    /// reproduce those bytes be mistaken for our own write and dropped.
    pub fn handle_watcher_overflow(&mut self) -> Vec<StateDelta> {
        self.last_written_hash.clear();
        vec![StateDelta::FullReload]
    }

    /// Process an external-change notification for `path`. Returns an empty
    /// vec if the on-disk content matches our most recent write to this path
    /// (i.e. the watcher is echoing our own write) — and otherwise returns a
    /// `FileReloaded` delta.
    ///
    /// The cached hash is cleared on a match, so a second identical watcher
    /// event would not be silently dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn handle_external_change(&mut self, absolute_path: &Path) -> Result<Vec<StateDelta>> {
        let bytes = match std::fs::read(absolute_path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.last_written_hash.remove(absolute_path);
                return Ok(Vec::new());
            }
            Err(e) => {
                return Err(LashError::IO {
                    code: codes::E_IO_READ_ERROR,
                    message: format!("failed to read {}", absolute_path.display()),
                    path: Some(absolute_path.to_path_buf()),
                    io_error: Some(e.to_string()),
                });
            }
        };

        let hash = *blake3::hash(&bytes).as_bytes();
        if self.last_written_hash.get(absolute_path) == Some(&hash) {
            self.last_written_hash.remove(absolute_path);
            return Ok(Vec::new());
        }

        // External change wins — clear any stale hash so we don't accidentally
        // suppress a future event.
        self.last_written_hash.remove(absolute_path);

        Ok(vec![StateDelta::FileReloaded {
            absolute_path: absolute_path.to_path_buf(),
        }])
    }

    /// Test-only access to the hash table.
    #[cfg(test)]
    fn has_recorded_hash(&self, path: &Path) -> bool {
        self.last_written_hash.contains_key(path)
    }
}

/// Write `bytes` to `path` atomically: write to a sibling temp file and
/// rename it into place. The rename is atomic on POSIX and on Windows
/// (same-volume), which is always true for in-project task files.
///
/// On error after the temp file is created, the temp file is removed so it
/// doesn't leak.
///
/// # Errors
///
/// Returns an `IO` error if the temp write or the rename fails.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    // If the target already exists, verify we can open it for writing
    // before doing the tmp-write/rename dance. Without this check, atomic
    // rename would silently overwrite a read-only file (rename only needs
    // write permission on the *parent directory*, not on the destination
    // file itself), which would be a surprising regression vs the older
    // direct-`fs::write` semantics callers rely on.
    if path.exists() {
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(|e| LashError::IO {
                code: codes::E_IO_WRITE_ERROR,
                message: format!("failed to write file: {}", path.display()),
                path: Some(path.to_path_buf()),
                io_error: Some(e.to_string()),
            })?;
    }

    let tmp = tmp_path_for(path);

    std::fs::write(&tmp, bytes).map_err(|e| LashError::IO {
        code: codes::E_IO_WRITE_ERROR,
        message: format!("failed to write file: {}", path.display()),
        path: Some(path.to_path_buf()),
        io_error: Some(e.to_string()),
    })?;

    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(LashError::IO {
            code: codes::E_IO_WRITE_ERROR,
            message: format!("failed to write file: {}", path.display()),
            path: Some(path.to_path_buf()),
            io_error: Some(e.to_string()),
        });
    }

    Ok(())
}

fn tmp_path_for(path: &Path) -> PathBuf {
    // Prefer "<name>.lash-tmp" rather than replacing the extension, so we
    // don't collide if two writes happen against the same stem with different
    // extensions.
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut name = path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".lash-tmp");
    parent.join(name)
}

/// Rewrite the first checkbox line for `task_title` with status `old_status`
/// to use `new_status`'s checkbox character.
///
/// Returns `None` if no matching line is found. Preserves trailing newline.
fn rewrite_checkbox(
    content: &str,
    task_title: &str,
    old_status: TaskStatus,
    new_status: TaskStatus,
) -> Option<String> {
    let old_char = old_status.to_checkbox_char();
    let new_char = new_status.to_checkbox_char();
    let escaped_title = regex::escape(task_title);

    let pattern = if matches!(old_status, TaskStatus::Done) {
        format!(r"^(\s*- \[)[xX](\] {escaped_title})")
    } else {
        format!(r"^(\s*- \[){old_char}(\] {escaped_title})")
    };

    let re = regex::Regex::new(&pattern).ok()?;

    let mut found = false;
    let updated: String = content
        .lines()
        .map(|line| {
            if !found && re.is_match(line) {
                found = true;
                re.replace(line, format!("${{1}}{new_char}${{2}}"))
                    .to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !found {
        return None;
    }

    let final_content = if content.ends_with('\n') && !updated.ends_with('\n') {
        format!("{updated}\n")
    } else {
        updated
    };
    Some(final_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixture() -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        std::fs::write(
            &path,
            "# Test File\n\n@id: f\n\n## Tasks\n\n- [ ] First task\n- [ ] Second task\n",
        )
        .unwrap();
        (dir, path)
    }

    #[test]
    fn write_atomic_writes_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.md");
        write_atomic(&path, b"hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn write_atomic_leaves_no_temp_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.md");
        write_atomic(&path, b"hello").unwrap();
        let tmp_left = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .any(|e| e.file_name().to_string_lossy().ends_with(".lash-tmp"));
        assert!(!tmp_left, "temp file leaked");
    }

    #[test]
    fn rewrite_checkbox_finds_and_replaces() {
        let content = "- [ ] First task\n- [ ] Second task\n";
        let out = rewrite_checkbox(
            content,
            "First task",
            TaskStatus::Open,
            TaskStatus::InProgress,
        )
        .unwrap();
        assert_eq!(out, "- [>] First task\n- [ ] Second task\n");
    }

    #[test]
    fn rewrite_checkbox_returns_none_when_missing() {
        assert!(rewrite_checkbox(
            "- [ ] Other task\n",
            "Missing",
            TaskStatus::Open,
            TaskStatus::Done
        )
        .is_none());
    }

    #[test]
    fn apply_records_hash_and_emits_delta() {
        let (_dir, path) = fixture();
        let mut store = Store::new();
        let deltas = store
            .apply(Mutation::SetTaskStatus {
                absolute_path: path.clone(),
                task_title: "First task".into(),
                old_status: TaskStatus::Open,
                new_status: TaskStatus::InProgress,
            })
            .unwrap();
        assert_eq!(deltas.len(), 1);
        assert!(matches!(
            deltas[0],
            StateDelta::TaskStatusChanged { ref task_title, .. } if task_title == "First task"
        ));
        assert!(store.has_recorded_hash(&path));
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("- [>] First task"));
    }

    #[test]
    fn apply_missing_task_errors() {
        let (_dir, path) = fixture();
        let mut store = Store::new();
        let err = store
            .apply(Mutation::SetTaskStatus {
                absolute_path: path,
                task_title: "Nope".into(),
                old_status: TaskStatus::Open,
                new_status: TaskStatus::Done,
            })
            .unwrap_err();
        assert_eq!(err.code(), codes::E_INTERNAL);
    }

    #[test]
    fn handle_external_change_drops_self_write_echo() {
        let (_dir, path) = fixture();
        let mut store = Store::new();
        store
            .apply(Mutation::SetTaskStatus {
                absolute_path: path.clone(),
                task_title: "First task".into(),
                old_status: TaskStatus::Open,
                new_status: TaskStatus::Done,
            })
            .unwrap();
        // Simulate the watcher firing for the file we just wrote.
        let deltas = store.handle_external_change(&path).unwrap();
        assert!(deltas.is_empty(), "self-write echo should be dropped");
        assert!(
            !store.has_recorded_hash(&path),
            "matched hash should be cleared after first observation"
        );
    }

    #[test]
    fn handle_external_change_emits_reload_for_real_edit() {
        let (_dir, path) = fixture();
        let mut store = Store::new();
        // Self-write, then someone else edits the file externally.
        store
            .apply(Mutation::SetTaskStatus {
                absolute_path: path.clone(),
                task_title: "First task".into(),
                old_status: TaskStatus::Open,
                new_status: TaskStatus::Done,
            })
            .unwrap();
        std::fs::write(&path, "- [ ] Brand new external content\n").unwrap();
        let deltas = store.handle_external_change(&path).unwrap();
        assert_eq!(deltas.len(), 1);
        assert!(matches!(
            deltas[0],
            StateDelta::FileReloaded { ref absolute_path } if absolute_path == &path
        ));
    }

    #[test]
    fn handle_watcher_overflow_asks_for_a_full_reload() {
        let mut store = Store::new();

        assert_eq!(
            store.handle_watcher_overflow(),
            vec![StateDelta::FullReload]
        );
    }

    #[test]
    fn handle_watcher_overflow_forgets_pending_self_write_hashes() {
        // The hash exists to recognize the watcher echoing our own write. If
        // the echo was among the dropped events it will never arrive, and a
        // stale entry would let a later genuine edit that reproduces those
        // bytes be mistaken for the echo and dropped.
        let (_dir, path) = fixture();
        let mut store = Store::new();
        store
            .apply(Mutation::SetTaskStatus {
                absolute_path: path.clone(),
                task_title: "First task".into(),
                old_status: TaskStatus::Open,
                new_status: TaskStatus::Done,
            })
            .unwrap();
        assert!(store.has_recorded_hash(&path));

        store.handle_watcher_overflow();

        assert!(!store.has_recorded_hash(&path));
        assert_eq!(
            store.handle_external_change(&path).unwrap().len(),
            1,
            "the same content must now read as an external edit, not an echo"
        );
    }

    #[test]
    fn handle_external_change_emits_reload_when_no_prior_self_write() {
        let (_dir, path) = fixture();
        let mut store = Store::new();
        let deltas = store.handle_external_change(&path).unwrap();
        assert_eq!(deltas.len(), 1);
    }

    #[test]
    fn second_identical_external_change_after_match_emits_reload() {
        let (_dir, path) = fixture();
        let mut store = Store::new();
        store
            .apply(Mutation::SetTaskStatus {
                absolute_path: path.clone(),
                task_title: "First task".into(),
                old_status: TaskStatus::Open,
                new_status: TaskStatus::Done,
            })
            .unwrap();
        // First event: matches our write, dropped.
        let first = store.handle_external_change(&path).unwrap();
        assert!(first.is_empty());
        // Second event with same bytes (no further edit): hash was cleared,
        // so we treat this as a real external change.
        let second = store.handle_external_change(&path).unwrap();
        assert_eq!(second.len(), 1);
    }

    #[test]
    fn handle_external_change_for_missing_file_is_quiet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ghost.md");
        let mut store = Store::new();
        let deltas = store.handle_external_change(&path).unwrap();
        assert!(deltas.is_empty());
    }

    #[test]
    fn create_task_emits_delta_and_records_hash_for_self_write_dedupe() {
        use lash_types::creation::TaskCreationRequestBuilder;

        let (_dir, path) = fixture();
        let mut store = Store::new();

        let request = TaskCreationRequestBuilder::new("Brand new task")
            .file_path(path.clone())
            .build();

        let deltas = store
            .apply(Mutation::CreateTask(Box::new(CreateTaskMutation {
                request,
                config: LashConfig::default(),
            })))
            .unwrap();

        assert_eq!(deltas.len(), 1);
        let StateDelta::TaskCreated {
            ref absolute_path,
            ref task_id,
            is_new_file,
        } = deltas[0]
        else {
            panic!("expected TaskCreated, got {:?}", deltas[0]);
        };
        assert_eq!(*absolute_path, path);
        assert_eq!(task_id, "brand-new-task");
        assert!(!is_new_file);

        // The new task should be on disk.
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            on_disk.contains("Brand new task"),
            "expected new task in file content: {on_disk}"
        );

        // And the watcher's echo of this write should be silently dropped.
        let echo = store.handle_external_change(&path).unwrap();
        assert!(echo.is_empty(), "self-write echo should be dropped");
    }

    #[test]
    fn create_task_propagates_validation_errors_as_internal_error() {
        use lash_types::creation::TaskCreationRequestBuilder;

        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::new();

        // Empty title — should fail validation.
        let request = TaskCreationRequestBuilder::new("")
            .file_path(dir.path().join("missing.md"))
            .build();

        let err = store
            .apply(Mutation::CreateTask(Box::new(CreateTaskMutation {
                request,
                config: LashConfig::default(),
            })))
            .unwrap_err();
        assert_eq!(err.code(), codes::E_INTERNAL);
    }
}
