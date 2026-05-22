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
- [ ] `handle_submit_task_creation` write path → `store.apply(CreateTask { ... })` — deferred; needs `Mutation::CreateTask`
- [ ] `lash_core::formatter::format_file_in_place` uses `write_atomic` — deferred; nice-to-have for safety, no behavioral impact

### Acceptance

- ✅ Production `std::fs::write` for status-toggle writes no longer exists in `lash-tui`
- ✅ All existing TUI tests pass unchanged

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
