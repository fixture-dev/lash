# Live TUI Updates

Status: Design (proposed)
Owner: TUI
Related tasks: `tasks/tasks.live-updates.md`, `tasks/tasks.status-bar-activity.md`

## Goal

While `lash tui` is running, its view (file tree, task tree, checkboxes, progress
bar, status bar) should reflect the current on-disk state in near-real-time, even
when an outside process — another human in `$EDITOR`, a coding agent calling
`lash` from a script, a bulk format — mutates task files or the SQLite index.

The TUI must also remain the source of truth for its own writes: when the user
toggles a checkbox or adds a task, that write must not feed back through the
watcher and cause a redundant reload or clobber an in-flight modal.

A new bottom-status-bar feature, "activity", is part of this work: two
live-updated sections showing the title of the currently in-progress task (if
any) and a rolling list of recently-completed task titles, truncated to fit.

## Non-goals

- Real-time multi-user merge / OT / CRDT. v1 prefers "refuse to silently
  overwrite, prompt the user to reload" if a file changed under an in-flight
  modal.
- Streaming partial-file updates. We re-parse whole files on change; the
  parser is already fast enough (~70µs for typical files).
- Cross-machine sync. Out of scope.

## Architecture

```
   crossterm input ──┐                          ┌── notify watcher
                     ▼                          ▼   (debounced ~150ms)
              ┌──────────────────────────────────────┐
              │           Store (actor)              │
              │   - canonical in-mem TaskFile map    │
              │   - last_written_hash[path]          │
              │   - applies mutations                │
              │   - emits StateDelta                 │
              └──────────────┬───────────────────────┘
                 atomic write│           ▲ external change
                 (tmp+rename)│           │ (hash-checked)
                             ▼           │
                         Markdown ──► inotify
                             │
                             └──► SQLite reindex (driven by Store)
```

### Store: single writer, single owner of canonical state

A `Store` actor in `lash-core` owns the in-memory `TaskFile` map and is the
**only** writer to disk. The rest of the system talks to it via a typed
`Mutation` enum and a `StateDelta` event stream.

```rust
pub enum Mutation {
    SetTaskStatus { full_id: String, status: TaskStatus },
    CreateTask    { request: TaskCreationRequest },
    // ...
}

pub enum StateDelta {
    TaskStatusChanged { full_id: String, old: TaskStatus, new: TaskStatus, title: String },
    FileReloaded     { path: PathBuf },
    FileDisappeared  { path: PathBuf },
    // ...
}

impl Store {
    pub fn apply(&mut self, mutation: Mutation) -> Result<Vec<StateDelta>>;
    pub fn handle_external_change(&mut self, path: &Path) -> Result<Vec<StateDelta>>;
}
```

The Store also drives incremental SQLite reindex: after `apply` or
`handle_external_change`, it patches the DB so callers never see drift between
Markdown and the index.

### Self-write handling: hash dedupe, not time windows

A naive "ignore watcher events for 100ms after our own write" approach is
fragile — a real external write inside the window is dropped silently. We use
a deterministic content hash instead.

1. `Store::apply` builds the new bytes for the affected file.
2. Before writing, it computes `h = blake3(bytes)` and records
   `last_written_hash[path] = h`.
3. It writes via `tmp + rename` (atomic) so the watcher never sees a partial
   file.
4. When the watcher fires for `path`, the Store re-reads the file, hashes it,
   and compares to `last_written_hash[path]`. If equal, **drop silently**.
   Otherwise, treat as external change.

The hash table is bounded to one entry per path; an entry is cleared the
moment its hash is matched (preventing later stale matches if the file is
re-written externally to the same bytes).

### Atomic writes

All Store writes go through one helper:

```rust
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("md.lash-tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
```

Rename is atomic on POSIX. On Windows the rename is also atomic on the same
volume (which we always satisfy here).

### File watcher

A background thread driven by the `notify` crate watches the project root
(recursive). It debounces ~150ms — coalescing the burst of events that
`$EDITOR` save flurries produce — and forwards a single `ExternalChange { path }`
to the Store via an `mpsc::Sender`.

Files outside the lash project (e.g. `.git/`, `target/`, `.lash/`) are
ignored at the watcher layer.

### Watcher → Store → TUI plumbing

The original design had `EventSource::poll_event` broadened to deliver an
`AppInputEvent { Term, External(StateDelta), Tick }` enum, with a
`MergedEventSource` muxing crossterm and watcher channels. In Phase C
implementation we deliberately deviated: the watcher's `mpsc::Receiver<PathBuf>`
is held as a sidecar field on `TuiAppCore` and **drained at the top of
`tick()`** before rendering. Each path is fed through
`Store::handle_external_change`, which produces zero or more `StateDelta`s
that the TUI dispatches via `apply_delta`.

Why the change: keyboard/mouse events and filesystem events have very
different ownership, threading, and test-injection stories. Muxing them in
one EventSource forced either a cascade of test-suite rewrites or a
`MergedEventSource` whose two halves still behaved very differently.
The sidecar channel is simpler, keeps `EventSource` focused on input
devices, and lets tests synthesize external edits by calling
`app.process_external_change(path)` directly — no fake watcher required.

```text
notify watcher ──debounce──> mpsc::Sender<PathBuf>
                                      │
                            (held by TuiAppCore)
                                      │
                  tick(): drain_external_changes()
                                      │
                         Store::handle_external_change
                                      │
                              Vec<StateDelta>
                                      │
                                apply_delta
                                      │
                  reindex + refetch + cursor-preserve
```

If a future need demands true muxing (e.g. RPC subagent events that should
preempt watcher events), promoting back to the EventSource design is local
to `tick()` and a single sidecar field — no API churn required.

### Cursor preservation by stable ID

When an external reload changes the shape of the task tree, the cursor's row
index becomes meaningless. We track selection by `(file_id, task_id)` (and
focused pane), and re-resolve the row index after every reload. If the task
the cursor was on disappears, we fall back to the nearest surviving sibling.

### Conflict policy

When `StateDelta::FileReloaded` arrives for a file that has an in-flight modal
open (task creation, edit, confirm-complete), v1 takes the conservative path:

- mark the modal stale,
- show a small status-bar banner: "file changed on disk — press R to reload",
- refuse to commit a stale write.

Three-way merge by `@id` is a v2 idea, not v1.

## Status bar: activity sections

The bottom status bar gains two sections, rendered alongside the existing pane
indicator / file-count / task-count:

```
 Tasks   Files: 12  Tasks: 184  ▶ Implementing Store actor    ✓ Add lash task files  ✓ Survey TUI writes …
 ^pane   ^counts                ^in-progress (icon ▶)         ^recently completed (icon ✓), oldest right
```

### Rules

- **In-progress section**: shows up to **one** task currently in
  `InProgress` status, picked deterministically (the most recently transitioned
  one; ties broken by `full_id`). If zero in-progress tasks exist, the section
  is omitted (no empty placeholder).
- **Recently-completed section**: shows up to **3** tasks that transitioned
  to `Done` (or `Waived`) within the last **5 minutes**, newest-first. Entries
  older than 5 minutes age out and are dropped from the rolling buffer.
- **Truncation**: each title is truncated with `…` so the whole bar fits the
  terminal width. We give the in-progress section ~40% of the available
  budget after counts/pane, then the recently-completed section gets the
  rest, split evenly. If even one entry can't fit at minimum width
  (~12 chars + ellipsis), it is dropped from the right.
- **Live updates**: both sections update from the same `StateDelta`
  stream the Store emits — so they react to TUI writes *and* external writes
  identically.
- **Coexistence with transient status messages**: when a transient
  status_message (`set_success_message`, etc.) is active, it overlays as
  today. Activity sections re-appear when the message expires.

### State shape

```rust
pub struct ActivityState {
    pub in_progress: Option<ActivityEntry>,   // current InProgress task
    pub recently_completed: VecDeque<ActivityEntry>, // newest-first, bounded
}

pub struct ActivityEntry {
    pub full_id: String,
    pub title:   String,
    pub at:      Instant,
}
```

`ActivityState` lives on `AppState`. It is initialized on TUI startup by
scanning current DB state for any InProgress task (no completion history is
backfilled — recently-completed starts empty), and is updated by feeding
`StateDelta::TaskStatusChanged` into a single `ActivityState::apply` method.

The expiry of recently-completed entries is checked in the existing periodic
`tick()` — already runs every ~100ms — so we don't need a separate timer.

## Phasing

This design is sequenced so that each phase ships value on its own.

### Phase A — Activity status bar, internal events only (this PR)

- Add `ActivityState` to `AppState`.
- Hook `handle_toggle_status` to push `ActivityEntry` into `ActivityState`
  after every successful status change.
- Render the two new sections in `ui::status_bar` with width-aware
  truncation.
- Tests: unit tests for `ActivityState` (push, age-out, ordering, bounded
  size), rendering tests for truncation.

This is end-to-end functional for TUI-initiated changes. External-process
changes are not reflected yet — that lights up in Phase C.

### Phase B — Store actor + atomic writes + hash dedupe

- New crate-internal module `lash_core::store`.
- Move `update_markdown_task_status` (currently in `lash-tui::app`) and
  the formatter/creation write paths through `Store::apply`.
- All writes go via `write_atomic`.
- `last_written_hash` table maintained per-path.
- Tests: round-trip a mutation, verify atomic temp file is gone, verify
  hash table records the write.

### Phase C — Watcher + broadened EventSource + external reload

- Add `notify` dep, write `FileWatcher` thread.
- Extend `EventSource` to `AppInputEvent`.
- Wire watcher → Store → `mpsc::Sender<StateDelta>` → TUI tick.
- Implement reload-on-external-change with stable-id cursor preservation.
- Conflict policy: stale-modal banner.
- Tests: write externally to a fixture file, assert the TUI's
  `TestEventSource` receives the `External(StateDelta::FileReloaded)`,
  assert cursor preservation.

### Phase D — Polish

- Bounded backpressure on the watcher channel (drop-coalesce on overflow,
  with a "reload all" fallback if we ever overflow).
- Optional: directory-scoped watch (perf at very large project sizes).

## Open questions

- **Where does the SQLite index update sit relative to disk write?** Current
  proposal: write Markdown atomically first, then update SQLite from the
  parsed result. If SQLite update fails post-write, the next external-change
  pass will re-reconcile. (Markdown is the source of truth — this is
  intentional.)
- **Recently-completed window — 5 minutes or "last N regardless of age"?**
  Proposed: 5 minutes *and* max 3 entries. Either condition aging out an
  entry.
- **Activity persistence across TUI restarts.** Out of scope for v1: the
  recently-completed list is in-memory only.

## Test plan

Phase A is testable with no new infrastructure:

- `ActivityState::apply` unit tests cover push, age-out, ordering,
  capacity, status-change semantics (Done→Open shouldn't add to
  recently-completed; Open→InProgress should set in_progress).
- Status-bar rendering tests use `ratatui::backend::TestBackend` to
  snapshot the rendered line at multiple terminal widths.
- Existing `handle_toggle_status` integration tests gain assertions that
  `ActivityState` is updated.

Phase B & C tests are sketched in their respective task files.
