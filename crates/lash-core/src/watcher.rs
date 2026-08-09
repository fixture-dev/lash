// LashError is intentionally rich; size-of-Err warning is not relevant here.
#![allow(clippy::result_large_err)]

//! Filesystem watcher that emits debounced, filtered `PathBuf` events for
//! Markdown files inside a project root.
//!
//! The watcher runs on its own thread and forwards events to a caller-owned
//! `mpsc::Sender<PathBuf>`. Returning a handle that owns the underlying
//! `notify::RecommendedWatcher` makes shutdown trivial: drop the handle and
//! the watcher (and its thread) tear down. The handle's `Drop` joins the
//! debouncer thread, so shutdown is complete — not merely started — by the
//! time `drop` returns.
//!
//! Filtering & debouncing happen inline in the watcher thread:
//! - non-`.md` paths are dropped
//! - any path inside `.git/`, `target/`, `.lash/`, or `node_modules/` is dropped
//! - identical path events arriving within `DEBOUNCE` are coalesced into one
//!
//! The outbound channel is bounded. A burst large enough to fill it (a
//! `git checkout` across a branch that touches thousands of task files, say)
//! would otherwise queue thousands of individual reindexes for a consumer that
//! drains on a ~100ms tick. Instead the watcher drops the paths it cannot
//! enqueue and raises an overflow flag, and the consumer reloads everything
//! once. Losing which files changed is fine when the answer is "too many to
//! be worth tracking"; what must not happen is losing the fact that something
//! changed.
//!
//! See `docs/live-tui-updates.md` Phase C for the broader context, and Phase D
//! for the backpressure above.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use lash_types::error::codes;
use lash_types::{LashError, Result};

/// Default debounce window for coalescing watcher events per-path.
pub const DEBOUNCE: Duration = Duration::from_millis(150);

/// Capacity of the watcher's outbound channel.
///
/// Comfortably above what a normal editing session produces, and far below
/// what a branch switch does. Past it, per-path delivery stops being useful
/// and [`WatcherEvents::drain`] reports an overflow instead.
pub const CHANNEL_CAPACITY: usize = 256;

/// Directory components we never recurse into. Cheap substring check — these
/// are well-known directories so false positives are not a concern.
const IGNORED_DIRS: &[&str] = &[".git", "target", ".lash", "node_modules"];

/// What a drain of the watcher channel found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatcherDrain {
    /// Paths that changed, debounced and filtered.
    pub paths: Vec<PathBuf>,

    /// True if the watcher had to drop paths because the channel was full.
    ///
    /// `paths` is then an arbitrary subset of what changed, and the consumer
    /// should reload everything rather than trust it.
    pub overflowed: bool,
}

/// Receive end of the watcher's bounded channel.
///
/// Wraps the channel together with the overflow flag so a consumer cannot read
/// one without the other: the paths are only complete if `overflowed` is false.
pub struct WatcherEvents {
    rx: mpsc::Receiver<PathBuf>,
    overflowed: Arc<AtomicBool>,
}

impl WatcherEvents {
    /// Take everything currently queued, without blocking.
    ///
    /// Reading the overflow flag clears it, so an overflow is reported to
    /// exactly one drain.
    #[must_use]
    pub fn drain(&self) -> WatcherDrain {
        // Paths first: a path dropped between here and the flag read is
        // covered by the flag, whereas clearing the flag first could discard
        // an overflow whose paths were never delivered.
        let paths: Vec<PathBuf> = self.rx.try_iter().collect();
        let overflowed = self.overflowed.swap(false, Ordering::AcqRel);
        WatcherDrain { paths, overflowed }
    }
}

/// Handle to a running watcher. Drop this to stop the watcher thread.
///
/// Shutdown is synchronous: once `drop` returns, the debouncer thread has
/// exited and no further paths can be sent to the caller's channel. Callers
/// can therefore drop the handle and immediately assume the channel is inert.
///
/// The watcher and thread fields are `Option` only so `Drop` can take them
/// and enforce the order below; they are always `Some` while the handle is
/// alive.
pub struct FileWatcherHandle {
    watcher: Option<notify::RecommendedWatcher>,
    debouncer_thread: Option<thread::JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl Drop for FileWatcherHandle {
    fn drop(&mut self) {
        // Signal before anything else. Dropping the watcher is not enough on
        // its own: the backend can outlive its own drop briefly (FSEvents on
        // macOS does), during which the debouncer may reach the flush deadline
        // for an already-pending path and emit it — after the caller believed
        // the watcher was gone. The flag closes that window; the debouncer
        // checks it before every emit.
        self.shutdown.store(true, Ordering::Release);

        // Then drop the watcher, which destroys the `notify` closure holding
        // the debouncer's input sender. That disconnect is what lets the loop
        // return, so this must happen before the join or it would deadlock.
        drop(self.watcher.take());

        if let Some(thread) = self.debouncer_thread.take() {
            // A panicking debouncer must not turn into a panic-in-drop.
            let _ = thread.join();
        }
    }
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
pub fn start(root: PathBuf) -> Result<(FileWatcherHandle, WatcherEvents)> {
    start_with_debounce(root, DEBOUNCE)
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
    debounce: Duration,
) -> Result<(FileWatcherHandle, WatcherEvents)> {
    let (out, rx) = mpsc::sync_channel::<PathBuf>(CHANNEL_CAPACITY);
    let overflowed = Arc::new(AtomicBool::new(false));
    let thread_overflowed = Arc::clone(&overflowed);

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

    let shutdown = Arc::new(AtomicBool::new(false));
    let thread_shutdown = Arc::clone(&shutdown);

    let debouncer_thread = thread::Builder::new()
        .name("lash-file-watcher-debouncer".into())
        .spawn(move || {
            debouncer_loop(raw_rx, out, debounce, &thread_shutdown, &thread_overflowed);
        })
        .map_err(|e| LashError::IO {
            code: codes::E_IO_READ_ERROR,
            message: format!("failed to spawn watcher debouncer thread: {e}"),
            path: None,
            io_error: Some(e.to_string()),
        })?;

    Ok((
        FileWatcherHandle {
            watcher: Some(watcher),
            debouncer_thread: Some(debouncer_thread),
            shutdown,
        },
        WatcherEvents { rx, overflowed },
    ))
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
fn debouncer_loop(
    raw_rx: mpsc::Receiver<Event>,
    out: SyncSender<PathBuf>,
    debounce: Duration,
    shutdown: &AtomicBool,
    overflowed: &AtomicBool,
) {
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

        // The handle has been dropped: abandon anything still pending rather
        // than racing the teardown to emit it.
        if shutdown.load(Ordering::Acquire) {
            return;
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
            // Never block here. The debouncer thread is also what notices
            // shutdown, so parking it on a full channel would make `drop` wait
            // for a consumer that may itself be waiting on us.
            match out.try_send(path) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) => overflowed.store(true, Ordering::Release),
                Err(TrySendError::Disconnected(_)) => return,
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

    fn drain_events(events: &WatcherEvents, deadline: Duration) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let end = Instant::now() + deadline;
        loop {
            let remaining = end.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match events.rx.recv_timeout(remaining) {
                Ok(p) => out.push(p),
                Err(_) => break,
            }
        }
        out
    }

    fn md_event(path: &Path) -> Event {
        Event::new(EventKind::Modify(notify::event::ModifyKind::Any)).add_path(path.to_path_buf())
    }

    /// The shutdown flag must win over a path that is already due to flush.
    /// A zero debounce makes the path due the instant it is recorded, which is
    /// the race the flag exists to close — the backend can keep feeding events
    /// for a short while after the handle is dropped, and without the flag the
    /// debouncer would emit them.
    #[test]
    fn shutdown_flag_suppresses_due_emissions() {
        let path = PathBuf::from("/tmp/proj/tasks.md");

        // Control: no shutdown signalled, so the due path is emitted.
        let (raw_tx, raw_rx) = mpsc::channel();
        let (out_tx, out_rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        raw_tx.send(md_event(&path)).unwrap();
        drop(raw_tx);
        debouncer_loop(
            raw_rx,
            out_tx,
            Duration::ZERO,
            &AtomicBool::new(false),
            &AtomicBool::new(false),
        );
        assert_eq!(
            out_rx.iter().collect::<Vec<_>>(),
            vec![path.clone()],
            "control: a due path should be emitted when not shutting down"
        );

        // Same input, shutdown already signalled: the path is abandoned.
        let (raw_tx, raw_rx) = mpsc::channel();
        let (out_tx, out_rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        raw_tx.send(md_event(&path)).unwrap();
        drop(raw_tx);
        debouncer_loop(
            raw_rx,
            out_tx,
            Duration::ZERO,
            &AtomicBool::new(true),
            &AtomicBool::new(false),
        );
        assert!(
            out_rx.iter().next().is_none(),
            "no path may be emitted once shutdown is signalled"
        );
    }

    /// Run the debouncer over `count` distinct paths with a zero debounce (so
    /// everything is due immediately) against a channel of size `capacity`.
    fn run_debouncer_over(count: usize, capacity: usize) -> (Vec<PathBuf>, bool) {
        let (raw_tx, raw_rx) = mpsc::channel();
        let (out_tx, out_rx) = mpsc::sync_channel(capacity);
        for i in 0..count {
            raw_tx
                .send(md_event(&PathBuf::from(format!("/tmp/proj/task-{i}.md"))))
                .unwrap();
        }
        drop(raw_tx);

        let overflowed = Arc::new(AtomicBool::new(false));
        debouncer_loop(
            raw_rx,
            out_tx,
            Duration::ZERO,
            &AtomicBool::new(false),
            &overflowed,
        );

        let events = WatcherEvents {
            rx: out_rx,
            overflowed,
        };
        let drained = events.drain();
        (drained.paths, drained.overflowed)
    }

    #[test]
    fn a_burst_within_capacity_does_not_overflow() {
        let (paths, overflowed) = run_debouncer_over(4, 8);

        assert_eq!(paths.len(), 4);
        assert!(!overflowed, "capacity was never reached");
    }

    #[test]
    fn a_burst_past_capacity_reports_overflow_and_keeps_running() {
        // A branch switch touching far more files than the channel holds. The
        // paths that do fit are still delivered, but the flag says they are
        // only a subset.
        let (paths, overflowed) = run_debouncer_over(40, 8);

        assert!(overflowed, "dropped paths must be reported");
        assert_eq!(
            paths.len(),
            8,
            "the channel should be full, not empty or unbounded"
        );
    }

    #[test]
    fn overflow_is_reported_to_exactly_one_drain() {
        let events = WatcherEvents {
            rx: mpsc::sync_channel::<PathBuf>(1).1,
            overflowed: Arc::new(AtomicBool::new(true)),
        };

        assert!(events.drain().overflowed);
        assert!(
            !events.drain().overflowed,
            "a second drain must not re-report the same overflow"
        );
    }

    #[test]
    fn a_full_channel_does_not_block_shutdown() {
        // The debouncer thread is also what observes the shutdown flag, so
        // blocking it on a full channel would make `drop` wait for a consumer
        // that may never drain. Capacity 1 with 200 due paths would deadlock
        // if the send blocked; that it returns at all is the assertion.
        let (paths, overflowed) = run_debouncer_over(200, 1);

        assert!(overflowed);
        assert_eq!(paths.len(), 1);
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

        let (_handle, rx) = start_with_debounce(root, Duration::from_millis(50)).unwrap();

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

        let (_handle, rx) = start_with_debounce(root.clone(), Duration::from_millis(50)).unwrap();

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

        let (handle, rx) = start_with_debounce(root, Duration::from_millis(50)).unwrap();
        std::thread::sleep(Duration::from_millis(80));

        // Clear anything the watcher queued before the drop. macOS FSEvents
        // reports changes from shortly before the stream opened, so the
        // `initial` write above can still be sitting in the channel here, and
        // the assertion below is about events caused *after* the drop. Without
        // this the test failed intermittently on macOS runners with a single
        // stray `tasks.md`.
        drain_events(&rx, Duration::from_millis(150));

        // No settling sleep after this point on purpose: `drop` joins the
        // debouncer thread, so the channel must already be inert when it
        // returns. A sleep here would hide a regression back to detached
        // shutdown, which failed intermittently on loaded CI runners.
        drop(handle);

        for i in 0..10 {
            std::fs::write(&path, format!("after drop {i}\n")).unwrap();
        }
        let events = drain_events(&rx, Duration::from_millis(200));
        assert!(
            events.is_empty(),
            "no events expected after handle is dropped; got {events:?}"
        );
    }
}
