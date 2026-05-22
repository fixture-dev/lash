// LashError is intentionally rich; size-of-Err warning is not relevant here.
#![allow(clippy::result_large_err)]

//! Filesystem watcher that emits debounced, filtered `PathBuf` events for
//! Markdown files inside a project root.
//!
//! The watcher runs on its own thread and forwards events to a caller-owned
//! `mpsc::Sender<PathBuf>`. Returning a handle that owns the underlying
//! `notify::RecommendedWatcher` makes shutdown trivial: drop the handle and
//! the watcher (and its thread) tear down.
//!
//! Filtering & debouncing happen inline in the watcher thread:
//! - non-`.md` paths are dropped
//! - any path inside `.git/`, `target/`, `.lash/`, or `node_modules/` is dropped
//! - identical path events arriving within `DEBOUNCE` are coalesced into one
//!
//! See `docs/live-tui-updates.md` Phase C for the broader context.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use lash_types::error::codes;
use lash_types::{LashError, Result};

/// Default debounce window for coalescing watcher events per-path.
pub const DEBOUNCE: Duration = Duration::from_millis(150);

/// Directory components we never recurse into. Cheap substring check — these
/// are well-known directories so false positives are not a concern.
const IGNORED_DIRS: &[&str] = &[".git", "target", ".lash", "node_modules"];

/// Handle to a running watcher. Drop this to stop the watcher thread.
///
/// We hold both the `notify` watcher (whose drop stops backend events) and
/// the debouncer thread's join handle (which the watcher's death will cause
/// to exit cleanly as its input channel is hung up).
pub struct FileWatcherHandle {
    _watcher: notify::RecommendedWatcher,
    _debouncer_thread: thread::JoinHandle<()>,
}

/// Start a watcher rooted at `root`. Each Markdown file change (after
/// filtering and debouncing) is sent to `tx` as an absolute `PathBuf`.
///
/// The returned handle must be kept alive — dropping it stops the watcher.
///
/// # Errors
///
/// Returns an error if the underlying `notify` watcher fails to start or
/// fails to register `root`.
#[allow(clippy::needless_pass_by_value)]
pub fn start(root: PathBuf, tx: Sender<PathBuf>) -> Result<FileWatcherHandle> {
    start_with_debounce(root, tx, DEBOUNCE)
}

/// Variant of `start` that exposes the debounce window. Used by tests to
/// keep them fast.
///
/// # Errors
///
/// Same as `start`.
#[allow(clippy::needless_pass_by_value)]
pub fn start_with_debounce(
    root: PathBuf,
    out: Sender<PathBuf>,
    debounce: Duration,
) -> Result<FileWatcherHandle> {
    let (raw_tx, raw_rx) = mpsc::channel::<Event>();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(event) = res {
            let _ = raw_tx.send(event);
        }
    })
    .map_err(|e| LashError::IO {
        code: codes::E_IO_READ_ERROR,
        message: format!("failed to create file watcher: {e}"),
        path: Some(root.clone()),
        io_error: Some(e.to_string()),
    })?;

    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| LashError::IO {
            code: codes::E_IO_READ_ERROR,
            message: format!("failed to watch {}: {e}", root.display()),
            path: Some(root.clone()),
            io_error: Some(e.to_string()),
        })?;

    let debouncer_thread = thread::Builder::new()
        .name("lash-file-watcher-debouncer".into())
        .spawn(move || debouncer_loop(raw_rx, out, debounce))
        .map_err(|e| LashError::IO {
            code: codes::E_IO_READ_ERROR,
            message: format!("failed to spawn watcher debouncer thread: {e}"),
            path: None,
            io_error: Some(e.to_string()),
        })?;

    Ok(FileWatcherHandle {
        _watcher: watcher,
        _debouncer_thread: debouncer_thread,
    })
}

/// Returns true if the path is a Markdown file we care about (i.e. has `.md`
/// extension and no ignored component).
fn is_relevant(path: &Path) -> bool {
    if path.extension().map_or(true, |e| e != "md") {
        return false;
    }
    !path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        IGNORED_DIRS.iter().any(|d| s.as_ref() == *d)
    })
}

/// Pull events from the raw `notify` channel, filter them, debounce per-path,
/// and forward the survivors to `out`.
///
/// We use `recv_timeout` so that a "pending" path can fire after `debounce`
/// even if no further events arrive — without that, the very last event in a
/// burst would sit in the pending map forever.
#[allow(clippy::needless_pass_by_value)]
fn debouncer_loop(raw_rx: mpsc::Receiver<Event>, out: Sender<PathBuf>, debounce: Duration) {
    let mut pending: HashMap<PathBuf, Instant> = HashMap::new();

    loop {
        let next_flush_in = next_flush_delay(&pending, debounce);
        let recv = if let Some(d) = next_flush_in {
            raw_rx.recv_timeout(d)
        } else {
            raw_rx
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        };

        match recv {
            Ok(event) => {
                if matters(event.kind) {
                    for path in event.paths {
                        if is_relevant(&path) {
                            pending.insert(path, Instant::now());
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }

        let now = Instant::now();
        let mut to_emit: Vec<PathBuf> = Vec::new();
        pending.retain(|path, &mut deadline| {
            if now.duration_since(deadline) >= debounce {
                to_emit.push(path.clone());
                false
            } else {
                true
            }
        });
        for path in to_emit {
            if out.send(path).is_err() {
                return;
            }
        }
    }
}

/// Compute how long until the oldest pending path is due to flush, or `None`
/// if nothing is pending (so the loop can block indefinitely on `recv`).
fn next_flush_delay(pending: &HashMap<PathBuf, Instant>, debounce: Duration) -> Option<Duration> {
    let earliest = pending.values().min()?;
    let elapsed = earliest.elapsed();
    Some(debounce.saturating_sub(elapsed))
}

fn matters(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn drain_events(rx: &mpsc::Receiver<PathBuf>, deadline: Duration) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let end = Instant::now() + deadline;
        loop {
            let remaining = end.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match rx.recv_timeout(remaining) {
                Ok(p) => out.push(p),
                Err(_) => break,
            }
        }
        out
    }

    #[test]
    fn is_relevant_accepts_md_outside_ignored() {
        assert!(is_relevant(Path::new("/tmp/foo/tasks.md")));
    }

    #[test]
    fn is_relevant_rejects_non_md() {
        assert!(!is_relevant(Path::new("/tmp/foo/tasks.rs")));
        assert!(!is_relevant(Path::new("/tmp/foo/no_ext")));
    }

    #[test]
    fn is_relevant_rejects_ignored_dirs() {
        assert!(!is_relevant(Path::new("/tmp/proj/.git/config.md")));
        assert!(!is_relevant(Path::new("/tmp/proj/target/foo.md")));
        assert!(!is_relevant(Path::new("/tmp/proj/.lash/cache.md")));
        assert!(!is_relevant(Path::new("/tmp/proj/node_modules/x/y.md")));
    }

    #[test]
    fn writing_md_file_yields_one_debounced_event() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let path = root.join("tasks.md");
        std::fs::write(&path, "initial\n").unwrap();

        let (tx, rx) = mpsc::channel();
        let _handle = start_with_debounce(root, tx, Duration::from_millis(50)).unwrap();

        // notify needs a moment to wire up after watch() returns.
        std::thread::sleep(Duration::from_millis(80));

        // Burst of writes — should debounce to a single event.
        for i in 0..5 {
            std::fs::write(&path, format!("step {i}\n")).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }

        let events = drain_events(&rx, Duration::from_millis(400));
        let matching: Vec<_> = events
            .iter()
            .filter(|p| p.file_name().and_then(|n| n.to_str()) == Some("tasks.md"))
            .collect();
        assert!(
            !matching.is_empty(),
            "expected at least one event for tasks.md, got {events:?}"
        );
        // We don't pin the exact count because notify backends vary; what we
        // care about is that one burst doesn't become many events. Three is
        // a generous upper bound on a 50ms debounce.
        assert!(
            matching.len() <= 3,
            "burst should debounce to a small number of events; got {} ({:?})",
            matching.len(),
            matching
        );
    }

    #[test]
    fn non_md_writes_produce_no_events() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();

        let (tx, rx) = mpsc::channel();
        let _handle = start_with_debounce(root.clone(), tx, Duration::from_millis(50)).unwrap();

        std::thread::sleep(Duration::from_millis(80));
        std::fs::write(root.join("ignored.txt"), "hi\n").unwrap();
        std::fs::write(root.join("also.rs"), "hi\n").unwrap();

        let events = drain_events(&rx, Duration::from_millis(250));
        let matching: Vec<_> = events
            .iter()
            .filter(|p| {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                name == "ignored.txt" || name == "also.rs"
            })
            .collect();
        assert!(
            matching.is_empty(),
            "non-md events should be filtered; got {matching:?}"
        );
    }

    #[test]
    fn dropping_handle_stops_events() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let path = root.join("tasks.md");
        std::fs::write(&path, "initial\n").unwrap();

        let (tx, rx) = mpsc::channel();
        let handle = start_with_debounce(root, tx, Duration::from_millis(50)).unwrap();
        std::thread::sleep(Duration::from_millis(80));
        drop(handle);
        // Allow watcher to fully tear down.
        std::thread::sleep(Duration::from_millis(50));

        std::fs::write(&path, "after drop\n").unwrap();
        let events = drain_events(&rx, Duration::from_millis(200));
        assert!(
            events.is_empty(),
            "no events expected after handle is dropped; got {events:?}"
        );
    }
}
