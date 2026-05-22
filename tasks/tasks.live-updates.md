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

**Priority:** HIGH
**Effort:** 0.5 day
**Depends on:** —

### Description

Add a `write_atomic(path, bytes)` helper in `lash-core` that writes to a
sibling temp path and renames into place. Add a `LastWrittenHashes` struct
that records `path -> blake3` for the most recent intentional write.

### Subtasks

- [ ] `write_atomic` helper with tests for partial-write resilience
- [ ] `LastWrittenHashes::record(path, hash)` and `matches_and_clear(path, hash) -> bool`
- [ ] Unit tests covering: record then match clears, double-match returns false the second time, distinct paths don't interfere

### Acceptance

- An interrupted write never leaves a partially-written `.md` on disk
- `matches_and_clear` is idempotent and returns false after first match

---

## Task 2: Store actor with `Mutation` / `StateDelta` API

**Priority:** HIGH
**Effort:** 1.5 days
**Depends on:** Task 1

### Description

Introduce `lash_core::store::Store`: single-owner of in-memory `TaskFile`
map. Exposes `apply(Mutation) -> Vec<StateDelta>` and
`handle_external_change(&Path) -> Vec<StateDelta>`. Internal calls go
through `write_atomic` and record the hash.

### Subtasks

- [ ] Define `Mutation` enum (start with `SetTaskStatus`; add `CreateTask` later)
- [ ] Define `StateDelta` enum (`TaskStatusChanged`, `FileReloaded`, `FileDisappeared`)
- [ ] Implement `Store::apply` for `SetTaskStatus` — read file, rewrite with new status, atomic write, record hash, emit delta
- [ ] Implement `Store::handle_external_change` — re-read, hash-check against last-written; if match, drop; if differ, re-parse and emit `FileReloaded`
- [ ] Unit tests: apply emits expected delta; external-change after self-write is silently dropped; external-change after a real external edit emits `FileReloaded`

### Acceptance

- Round-trip: `apply(SetTaskStatus)` then simulate watcher event → no delta emitted
- External edit then watcher event → `FileReloaded` emitted

---

## Task 3: Route TUI write sites through Store::apply

**Priority:** HIGH
**Effort:** 1 day
**Depends on:** Task 2

### Description

Replace the direct `fs::write` in `crates/lash-tui/src/app.rs:1392` (inside
`update_markdown_task_status`) and the task-creation write path with a
call to `Store::apply`. Also route `lash-core::formatter::format_file`'s
write through `write_atomic` (it doesn't need the Store, but it should be
atomic).

### Subtasks

- [ ] Add `store: Store` field to `TuiAppCore`
- [ ] `handle_toggle_status` → `store.apply(SetTaskStatus { ... })`
- [ ] `handle_submit_task_creation` write path → `store.apply(CreateTask { ... })`
- [ ] `lash_core::formatter::format_file` uses `write_atomic`
- [ ] Existing toggle-status integration tests still pass

### Acceptance

- No production `std::fs::write` for task files remains outside `lash-core::store` / `formatter`
- All TUI integration tests pass unchanged

---

## Task 4: `notify` file watcher with debounce

**Priority:** HIGH
**Effort:** 1 day
**Depends on:** Task 2

### Description

Background thread using `notify` (and `notify-debouncer-mini` or
hand-rolled ~150ms debouncer) watches the project root. Ignores `.git/`,
`target/`, `.lash/`. Forwards collapsed events to the Store via
`mpsc::Sender<PathBuf>`.

### Subtasks

- [ ] Add `notify` dependency to `lash-core`
- [ ] `FileWatcher::start(project_root, tx)` spawns thread, returns handle
- [ ] Debounce window 150ms; coalesce per-path
- [ ] Ignore-list: `.git/`, `target/`, `.lash/`, and any path not matching `**/*.md`
- [ ] Graceful shutdown on handle drop
- [ ] Test: edit a fixture file, assert exactly one event surfaces after debounce

### Acceptance

- A burst of 10 saves in <50ms produces 1 event per file
- A non-`.md` change produces 0 events

---

## Task 5: Broaden EventSource to deliver AppInputEvent

**Priority:** HIGH
**Effort:** 1 day
**Depends on:** Task 2, Task 4

### Description

Replace `EventSource::poll_event -> Option<crossterm::Event>` with
`Option<AppInputEvent>` (Term / External / Tick). Implement
`MergedEventSource` that draws from both crossterm polling and the
watcher channel. Update `TestEventSource` to inject `External(...)`
events for tests.

### Subtasks

- [ ] Add `AppInputEvent` enum in `crates/lash-tui/src/event.rs`
- [ ] Change trait signature; update `TerminalEventSource` and `TestEventSource`
- [ ] New `MergedEventSource { term, store_rx }`
- [ ] TUI `tick()` matches on `AppInputEvent`, routes `External(delta)` to a new `handle_external_delta` method
- [ ] Existing TUI tests rewired to use `AppInputEvent::Term(...)`

### Acceptance

- No regression in any TUI test
- A synthesized `AppInputEvent::External(FileReloaded { ... })` causes the TUI to re-fetch tasks for that file

---

## Task 6: Stable-id cursor preservation on external reload

**Priority:** MEDIUM
**Effort:** 0.5 day
**Depends on:** Task 5

### Description

When a `FileReloaded` delta arrives for the currently-viewed file, recompute
the row index of the selected task by `@id` instead of trusting the previous
index. If the task is gone, snap to nearest surviving sibling.

### Subtasks

- [ ] Capture `selected_task_full_id` before reload
- [ ] After `build_task_tree`, find the row index whose task matches that id
- [ ] Fallback path: previous sibling, then next sibling, then index 0
- [ ] Test: edit a fixture file to insert a task above the cursor, assert cursor still points to original task

### Acceptance

- Cursor stays "on" the same task through arbitrary external edits

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

- [ ] Add `stale: bool` to relevant modal states
- [ ] On `FileReloaded { path }`, set `stale = true` if any open modal targets `path`
- [ ] On submit, refuse if stale; show banner
- [ ] `R` key reloads the modal's baseline from the Store and clears the stale flag

### Acceptance

- Submitting a stale modal does not overwrite external changes
- Reload-then-resubmit works

---

## Task 8: Phase D polish

**Priority:** LOW
**Effort:** 0.5–1 day
**Depends on:** Task 7

### Description

- Bounded watcher channel; on overflow, emit a `FullReload` delta instead.
- Optional: directory-scoped watch for very large projects (deferred unless a real perf complaint surfaces).
