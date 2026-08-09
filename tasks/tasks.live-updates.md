# Live TUI Updates Tasks

**Module:** `lash-core`, `lash-tui`
**Dependencies:** tasks.tui.md, tasks.markdown-parser.md, tasks.indexing.md
**Effort:** 6-9 days
**Priority:** MEDIUM
**Design doc:** [docs/live-tui-updates.md](../docs/live-tui-updates.md)

## Overview

While `lash tui` is running, its view must reflect external changes to task
files in near-real-time, while not feeding TUI-initiated writes back through
the watcher as redundant reloads.

This is delivered in phases. Phase A (activity status bar) ships standalone
value and is tracked in [tasks.status-bar-activity.md](tasks.status-bar-activity.md).
This file tracks Phases B–D.

---

## Task 1: Atomic write helper + last-written-hash table

**Priority:** HIGH ✅ done
**Effort:** 0.5 day
**Depends on:** —

### Description

Add a `write_atomic(path, bytes)` helper in `lash-core` that writes to a
sibling temp path and renames into place. The hash table is embedded inside
`Store` rather than being a separate struct — the only legitimate consumer
is the Store itself.

### Subtasks

- [x] `write_atomic` helper with tests (clean write, no leaked temp file)
- [x] Per-path `last_written_hash: HashMap<PathBuf, [u8; 32]>` embedded in `Store`
- [x] `handle_external_change` deduplicates by hash and clears the entry on first match

---

## Task 2: Store actor with `Mutation` / `StateDelta` API

**Priority:** HIGH ✅ done
**Effort:** 1.5 days
**Depends on:** Task 1

### Description

`lash_core::store::Store` is the single writer. Exposes
`apply(Mutation) -> Vec<StateDelta>` and
`handle_external_change(&Path) -> Vec<StateDelta>`. Internal calls go
through `write_atomic` and record the hash.

The in-memory `TaskFile` map (originally described in the design doc) was
not needed in this slice — the SQLite index stays the canonical
intermediate cache for TUI reads, and the Store's job is restricted to
write-side coordination and external-change hash dedupe. If a later phase
finds it needs the in-memory cache, it can be added without changing the
API surface here.

### Subtasks

- [x] Define `Mutation::SetTaskStatus { absolute_path, task_title, old_status, new_status }`
- [x] Define `StateDelta::TaskStatusChanged` and `StateDelta::FileReloaded`
- [x] Implement `Store::apply` for `SetTaskStatus` — read file, rewrite with new status (regex-based, ported from the TUI), atomic write, record hash, emit delta
- [x] Implement `Store::handle_external_change` — re-read, hash-check; if match, drop and clear cache; if differ (or no cache entry), emit `FileReloaded`
- [x] Unit tests (8): apply emits expected delta; missing task → `E_INTERNAL`; self-write echo dropped; external edit emits reload; no-prior-write external read emits reload; second identical event after first-match is treated as external; missing file is quiet

---

## Task 3: Route TUI write sites through Store::apply

**Priority:** HIGH ✅ partially done (status-toggle paths)
**Effort:** 1 day
**Depends on:** Task 2

### Description

Replace the direct `fs::write` in `crates/lash-tui/src/app.rs` (inside
`update_markdown_task_status`) with a call to `Store::apply`. All five
toggle/cascade call sites now flow through this single helper, so they
get atomic writes and hash recording for free.

The task-creation write path and `lash_core::formatter::format_file_in_place`
still write directly to disk; those are tracked as follow-ups but aren't
on the critical path for the watcher → reload loop (creating a new task is
rare, and `lash format` from the CLI never races with a running TUI).

### Subtasks

- [x] Add `store: lash_core::store::Store` field to `TuiAppCore`
- [x] `update_markdown_task_status` → `store.apply(SetTaskStatus { ... })`
- [x] All five callers (`handle_toggle_status` + 3 cascade handlers + linked-file complete) go through the new helper unchanged
- [x] Existing TUI tests (160 lib tests) still pass
- [x] `handle_submit_task_creation` write path → `store.apply(CreateTask { ... })`
- [x] `lash_core::formatter::format_file_in_place` uses `write_atomic`

### Acceptance

- ✅ Production `std::fs::write` for status-toggle writes no longer exists in `lash-tui`
- ✅ All existing TUI tests pass unchanged

---

## Task 4: `notify` file watcher with debounce

**Priority:** HIGH ✅ done
**Effort:** 1 day
**Depends on:** Task 2

### Description

Background thread using `notify` + hand-rolled debouncer watches the project
root. Ignores `.git/`, `target/`, `.lash/`, `node_modules/`, and non-`.md`
paths. Forwards collapsed events as `PathBuf` on an `mpsc::Sender`.

### Subtasks

- [x] Add `notify` dependency to `lash-core` and workspace
- [x] `watcher::start(project_root, tx)` spawns thread, returns handle
- [x] Debounce window 150ms (configurable via `start_with_debounce` for tests); coalesce per-path
- [x] Ignore-list: `.git/`, `target/`, `.lash/`, `node_modules/`, and any path not ending in `.md`
- [x] Graceful shutdown on handle drop: `Drop` signals a shutdown flag, drops
      the watcher, then joins the debouncer, so no path can be emitted once
      `drop` returns (verified by `shutdown_flag_suppresses_due_emissions`;
      the original sleep-based test was an unreliable guard and flaked on CI)
- [x] Test: edit a fixture file, assert a small number of events surface after debounce
- [x] Test: non-`.md` writes produce 0 events

---

## Task 5: Watcher → TUI wiring

**Priority:** HIGH ✅ done (via sidecar channel, not EventSource broadening)
**Effort:** 1 day
**Depends on:** Task 2, Task 4

### Description

The original plan was to broaden `EventSource::poll_event` to deliver an
`AppInputEvent` enum. In implementation we deviated: the watcher's
`mpsc::Receiver<PathBuf>` is held as a sidecar field on `TuiAppCore` and
drained at the top of `tick()`. See `docs/live-tui-updates.md` for the
rationale.

### Subtasks

- [x] Watcher receiver field on `TuiAppCore`; watcher handle kept alive for app lifetime
- [x] `TuiApp::new_with_scheme` starts the watcher rooted at the project root; failures degrade gracefully (live updates disabled, app still runs)
- [x] `tick()` drains the receiver before render
- [x] Each `PathBuf` is fed through `Store::handle_external_change`; the resulting `StateDelta`s are dispatched via `apply_delta`
- [x] Public `process_external_change(path)` method lets tests bypass the OS watcher
- [x] All existing TUI tests pass unchanged

### Acceptance

- ✅ No regression in any TUI test
- ✅ Calling `process_external_change(path)` after an out-of-band write causes the TUI to reindex and reload the file

---

## Task 6: Stable-id cursor preservation on external reload

**Priority:** MEDIUM ✅ done
**Effort:** 0.5 day
**Depends on:** Task 5

### Description

When a `FileReloaded` delta arrives for the currently-viewed file, recompute
the row index of the selected task by `full_id` instead of trusting the
previous index. If the task is gone, fall back to the closest valid index.

### Subtasks

- [x] `AppState::selected_task_full_id()` captures the cursor's stable id
- [x] `AppState::restore_task_selection_by_full_id(full_id)` walks the flattened tree, sets the index if found, clamps to valid range otherwise
- [x] `handle_file_reloaded` captures the id pre-reload, rebuilds the tree, restores expansion state, then restores selection
- [x] Integration test: edit a fixture to insert a task above the cursor, assert cursor still anchors to the original task

### Acceptance

- ✅ Cursor stays anchored to the same task through external inserts above it

---

## Task 7: Stale-modal conflict policy

**Priority:** MEDIUM
**Effort:** 0.5 day
**Depends on:** Task 5

### Description

If an in-flight task-creation or edit modal targets a file that is
`FileReloaded`'d under it, mark the modal stale and surface a status banner
("file changed on disk — press R to reload"). Refuse to commit a stale write.

### Subtasks

- [x] Add `stale: bool` to relevant modal states
- [x] On `FileReloaded { path }`, set `stale = true` if any open modal targets `path`
- [x] On submit, refuse if stale; show banner
- [-] `R` key reloads the modal's baseline from the Store and clears the stale flag — Esc-and-retry covers it; a reload-in-place key is not worth the extra state

### Acceptance

- Submitting a stale modal does not overwrite external changes
- Reload-then-resubmit works

---

## Task 8: Phase D polish

**Priority:** LOW
**Effort:** 0.5–1 day
**Depends on:** Task 7

### Description

- [x] Bounded watcher channel; on overflow, emit a `FullReload` delta instead.
- [-] Optional: directory-scoped watch for very large projects — deferred; no perf complaint has surfaced, and the bounded channel now caps the damage from a large burst.
