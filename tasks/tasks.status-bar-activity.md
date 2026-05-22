# Status Bar Activity Sections Tasks

**Module:** `lash-tui`
**Dependencies:** tasks.tui.md
**Effort:** 1–2 days
**Priority:** MEDIUM
**Design doc:** [docs/live-tui-updates.md](../docs/live-tui-updates.md) §"Status bar: activity sections"

## Overview

Add two live-updated sections to the bottom status bar:

- **In-progress**: title of the currently in-progress task (icon `▶`), if any.
- **Recently completed**: rolling list of titles for tasks that transitioned
  to Done/Waived within the last 5 minutes (icon `✓`), newest first, up to 3.

Titles are truncated with `…` to fit terminal width. Updates flow from
`StateDelta::TaskStatusChanged` events — the same plumbing that will later
carry external changes once the Store and watcher land.

In Phase A (this file), updates are wired only to TUI-initiated transitions
via `handle_toggle_status`. The hook surface is designed so Phase C of the
live-updates work can drop external-change updates into the same `ActivityState`
without further refactor.

---

## Task 1: ActivityState data structure

**Priority:** HIGH
**Effort:** 0.5 day

### Description

Add `ActivityState` to `crates/lash-tui/src/state.rs`. Pure data, no IO,
fully unit-testable.

### Subtasks

- [x] `ActivityEntry { full_id, title, at }`
- [x] `ActivityState { in_progress: Option<ActivityEntry>, recently_completed: VecDeque<ActivityEntry> }`
- [x] `ActivityState::record_transition(full_id, title, old: TaskStatus, new: TaskStatus, now: Instant)`
  - Open|Blocked|Waived → InProgress: set `in_progress`
  - InProgress → Done|Waived: clear `in_progress` if matches; push to `recently_completed`
  - Open|InProgress → Done|Waived: push to `recently_completed`
  - Done|Waived → Open|InProgress: remove from `recently_completed` (re-opened a recently-completed task should not still show as "recent")
- [x] `ActivityState::prune(now)` — drop `recently_completed` entries older than 5 minutes; cap to 3 newest
- [x] Add `pub activity: ActivityState` field to `AppState`, initialize in `with_theme`
- [x] Unit tests: each transition path, prune by age, cap by count, reopen removes from recent

### Acceptance

- 100% branch coverage on `record_transition` and `prune`

---

## Task 2: Initial scan on startup

**Priority:** MEDIUM
**Effort:** 0.25 day
**Depends on:** Task 1

### Description

On TUI startup, query the DB for any tasks with `status = InProgress` and
seed `activity.in_progress` with the first one (deterministic ordering by
`full_id`). Recently-completed stays empty — we don't backfill historical
completion times.

### Subtasks

- [x] Seed via existing `TaskRepository::find_by_status(InProgress)` (already ordered by `full_id` — no new query needed)
- [x] Seed `state.activity.in_progress` in `TuiApp::new_with_scheme`
- [ ] Test: DB with two in-progress tasks → smaller full_id wins *(deferred — relies on TUI integration harness; ordering itself is covered by existing repository tests)*

---

## Task 3: Hook handle_toggle_status

**Priority:** HIGH
**Effort:** 0.25 day
**Depends on:** Task 1

### Description

After a successful status transition in `app.rs::handle_toggle_status`,
call `state.activity.record_transition(...)`. The existing
`set_success_message` path is preserved — it overlays the activity bar
for its transient duration.

### Subtasks

- [x] Insert `record_transition` call after `update_markdown_task_status` succeeds (primary toggle path)
- [x] Hook the three cascading handlers (`handle_confirm_cascading_complete`, `handle_confirm_linked_file_complete`, `handle_confirm_cascading_incomplete`) — initiating task only, not cascade results
- [x] Existing tests for `handle_toggle_status` still pass
- [ ] New end-to-end test: toggling Open→InProgress sets `activity.in_progress`; InProgress→Done clears it and pushes to recent *(deferred to Phase C when a more general TUI integration harness exists; covered indirectly by unit tests on `ActivityState::record_transition`)*

---

## Task 4: Periodic prune in tick()

**Priority:** MEDIUM
**Effort:** 0.1 day
**Depends on:** Task 1

### Description

`tick()` already runs at ~10Hz. Call `state.activity.prune(Instant::now())`
alongside `check_status_expiry()`. Cheap (~O(3)).

### Subtasks

- [x] One-line addition in `tick()`
- [x] No test needed beyond what Task 1 already covers (`prune` is unit-tested)

---

## Task 5: Render activity sections in status_bar.rs

**Priority:** HIGH
**Effort:** 0.75 day
**Depends on:** Task 1

### Description

Extend `render_default` in `crates/lash-tui/src/ui/status_bar.rs` to render:

```
 Tasks   Files: 12  Tasks: 184  ▶ Implementing Store actor   ✓ Add lash task files  ✓ Survey TUI writes
```

### Subtasks

- [x] Compute available width after the existing pane/counts/filter prefix and the right-side "Press ? for help" suffix
- [x] Allocate ~40% to in-progress section, remainder split evenly among up-to-3 recent entries
- [x] Width-aware ellipsis: minimum 12 visible chars per entry; drop entries from the right if they don't fit
- [x] Icons: `▶` (in-progress) and `✓` (recent) styled with theme colors (`task_in_progress` / `task_done`)
- [x] When `status_message` is active, the existing message overlay continues to take over the whole bar
- [x] Unit tests using `ratatui::backend::TestBackend`: assert the buffer contents

### Acceptance

- Bar fits any terminal width ≥ 60 cols without panicking or wrapping
- Reasonable truncation behavior verified by buffer snapshot tests
