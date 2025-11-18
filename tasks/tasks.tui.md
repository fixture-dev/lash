# Terminal UI (TUI) Tasks

**Module:** `lash-tui`
**Dependencies:** tasks.cli-framework.md, tasks.sqlite-schema.md, tasks.dependency-resolution.md, tasks.fuzzy-search.md
**Effort:** 10-14 days
**Priority:** HIGH

## Overview

Implement an interactive Terminal UI for browsing, filtering, and editing tasks. The TUI provides a more ergonomic interface than CLI commands for exploring large task trees and understanding dependencies visually.

## Core Requirements

From design-doc.md section 8:
- Two-pane layout: navigation tree + task detail (section 8.1)
- Keyboard-centric interactions (section 8.2)
- Fuzzy search, filtering, dependency viewing (section 8.2)
- Open files in external `$EDITOR` (section 8.2)
- Agent awareness mode (section 8.3)

---

## Task 1: TUI Framework Setup

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.cli-framework.md#1

### Description

Set up the TUI framework using `ratatui` (or similar) with basic rendering and event handling.

### Subtasks

- [ ] Add TUI dependencies
  - [ ] `ratatui` for rendering
  - [ ] `crossterm` for terminal control
  - [ ] `tui-textarea` for text input (optional)
- [ ] Implement `TuiApp` struct
  - [ ] Application state (selected file, pane focus, etc.)
  - [ ] Database connection
  - [ ] Configuration
- [ ] Implement terminal setup/teardown
  - [ ] Enter alternate screen
  - [ ] Enable raw mode
  - [ ] Hide cursor
  - [ ] Restore terminal on exit (even on panic)
- [ ] Implement event loop
  - [ ] Poll for keyboard events
  - [ ] Dispatch to handlers
  - [ ] Render on state change
  - [ ] Handle quit command (q, Ctrl-C)
- [ ] Implement basic rendering
  - [ ] Clear screen
  - [ ] Render placeholder layout
  - [ ] Handle terminal resize events
- [ ] Add error handling
  - [ ] Gracefully handle terminal errors
  - [ ] Show error screen before exiting
  - [ ] Log errors for debugging

### Success Criteria

- TUI launches and renders basic UI
- Terminal properly restored on exit
- Keyboard events are captured
- No panics or crashes

### Tests

- Integration: Launch TUI and quit immediately
- Integration: Test terminal restoration
- Manual: Interactive testing of basic controls

---

## Task 2: Navigation Pane

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** Task 1

### Description

Implement the left navigation pane showing directory tree, files, and label filters.

### Subtasks

- [ ] Define `NavPane` struct
  - [ ] Current view mode (files, labels, search results)
  - [ ] Tree state (expanded/collapsed nodes)
  - [ ] Selection state (current index)
- [ ] Implement file tree view
  - [ ] Load directory structure from DB
  - [ ] Render as collapsible tree
  - [ ] Show file names with status indicators
  - [ ] Indent by directory depth
  - [ ] Use tree characters (├──, └──, etc.)
- [ ] Implement tree navigation
  - [ ] `j/k` or arrow keys: move selection up/down
  - [ ] `Enter` or `l`: expand directory or open file
  - [ ] `h`: collapse directory or go to parent
  - [ ] `gg/G`: go to top/bottom
- [ ] Implement label view
  - [ ] List all labels with task counts
  - [ ] Navigate with `j/k`
  - [ ] Select label to filter
- [ ] Add visual styling
  - [ ] Highlight selected item
  - [ ] Color-code by status (done=green, blocked=red)
  - [ ] Show task counts per file
  - [ ] Scrolling indicator for long lists
- [ ] Implement scrolling
  - [ ] Track viewport (visible range)
  - [ ] Scroll to keep selection visible
  - [ ] Smooth scrolling (optional)

### Success Criteria

- File tree displays correctly
- Navigation is smooth and intuitive
- Tree expand/collapse works
- Label view shows all labels

### Tests

- Unit: Test tree rendering logic
- Unit: Test navigation key handling
- Integration: Load fixture project and navigate
- Manual: Interactive testing of tree navigation

---

## Task 3: Detail Pane

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 1, Task 2

### Description

Implement the right detail pane showing task list and metadata for the selected file or filter.

### Subtasks

- [ ] Define `DetailPane` struct
  - [ ] Current task list
  - [ ] Selection state
  - [ ] Scroll position
- [ ] Implement task list rendering
  - [ ] Show hierarchical task tree (indented by depth)
  - [ ] Display checkboxes: `[ ]`, `[x]`, `[-]`, `[!]`
  - [ ] Show task titles
  - [ ] Show inline labels (colored tags)
  - [ ] Color-code by status
  - [ ] Highlight selected task
- [ ] Implement task navigation
  - [ ] `j/k` or arrows: move selection
  - [ ] `Enter`: show task details or expand/collapse
  - [ ] `gg/G`: go to top/bottom
  - [ ] `{/}`: jump to previous/next top-level task
- [ ] Implement task detail view
  - [ ] Show full task metadata:
    - [ ] Title, status, labels
    - [ ] Owner, estimate, created date
    - [ ] File path and line number
    - [ ] Dependencies list
    - [ ] Blockers (if any)
  - [ ] Toggle between list and detail view
- [ ] Add file metadata header
  - [ ] Show file path
  - [ ] Show overall progress (X/Y tasks done)
  - [ ] Show file status
- [ ] Implement scrolling
  - [ ] Track viewport
  - [ ] Scroll to keep selection visible

### Success Criteria

- Task list displays correctly with hierarchy
- Navigation is smooth
- Task details show comprehensive information
- Scrolling works for long lists

### Tests

- Unit: Test task list rendering
- Unit: Test navigation logic
- Integration: Display fixture file in detail pane
- Manual: Interactive testing

---

## Task 4: Keyboard Commands

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 2, Task 3

### Description

Implement all keyboard commands for TUI interaction as specified in design doc section 8.2.

### Subtasks

- [ ] Implement pane switching
  - [ ] `Tab` or `Ctrl-h/l`: switch between nav and detail panes
  - [ ] Visual indicator of focused pane
- [ ] Implement task status toggle
  - [ ] `Space`: cycle task status (open -> done -> waived -> open)
  - [ ] Update DB immediately
  - [ ] Refresh display
  - [ ] Handle hierarchy constraints (warn if children open)
- [ ] Implement editor integration
  - [ ] `e`: open current file in `$EDITOR`
  - [ ] Suspend TUI (exit alternate screen)
  - [ ] Run `$EDITOR` with file path
  - [ ] Resume TUI after editor exits
  - [ ] Reload file if modified
- [ ] Implement search
  - [ ] `/`: open search input
  - [ ] Type query
  - [ ] `Enter`: execute search
  - [ ] Display results in nav pane
  - [ ] `Esc`: cancel search, return to file view
- [ ] Implement filtering
  - [ ] `l`: open label filter input
  - [ ] Type or select labels
  - [ ] Filter task list in detail pane
  - [ ] Show active filters in status bar
  - [ ] `c`: clear filters
- [ ] Implement dependency graph view
  - [ ] `g`: show dependency graph for selected task
  - [ ] Display as text tree or overlay
  - [ ] Highlight blockers
- [ ] Implement help overlay
  - [ ] `?`: show help screen with all commands
  - [ ] `Esc` or `?` again: close help
- [ ] Implement quit
  - [ ] `q` or `Ctrl-C`: quit TUI
  - [ ] Confirm if unsaved changes (future)

### Success Criteria

- All keyboard commands work as specified
- Commands are intuitive and discoverable
- Help overlay is comprehensive
- Editor integration works with common editors (vim, emacs, nano, VSCode)

### Tests

- Integration: Test each keyboard command
- Integration: Test pane switching
- Integration: Test status toggle (verify DB update)
- Manual: Test editor integration with different `$EDITOR` values

---

## Task 5: Agent View Mode

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 3

### Description

Implement an "Agent view" mode that filters and highlights tasks relevant to AI agents.

### Subtasks

- [ ] Implement view mode toggle
  - [ ] `a`: toggle agent view mode
  - [ ] Visual indicator in status bar
- [ ] Implement agent task filtering
  - [ ] Filter by `#agent` label
  - [ ] Filter by owner (if `--for-owner` specified)
  - [ ] Show only incomplete tasks
- [ ] Implement token-aware display
  - [ ] Show estimated token count for visible tasks
  - [ ] Warn if over token budget (configurable)
  - [ ] Prioritize display: blocked > open > done
- [ ] Add agent task summary
  - [ ] Count of agent tasks by status
  - [ ] Top blockers
  - [ ] Suggested next tasks (ready to start)
- [ ] Implement copy to clipboard (optional)
  - [ ] `y`: yank current task as markdown
  - [ ] `Y`: yank agent prompt for current view
  - [ ] Requires clipboard library (arboard or similar)

### Success Criteria

- Agent view shows only relevant tasks
- Token budget tracking is accurate
- Summary is helpful for understanding agent workload

### Tests

- Integration: Toggle agent view mode
- Integration: Verify filtering works
- Integration: Test token counting
- Manual: Visual inspection of agent view

---

## Task 6: Visual Polish and Themes

**Priority:** LOW
**Effort:** 1-2 days
**Depends on:** Task 1-3

### Description

Add visual polish, colors, and optional theming to improve TUI aesthetics.

### Subtasks

- [ ] Define color scheme
  - [ ] Default: terminal default colors + highlights
  - [ ] Use theme colors for status:
    - [ ] Green: done
    - [ ] Red: blocked
    - [ ] Yellow: in progress (if supported)
    - [ ] Gray: waived
  - [ ] Use distinct colors for labels (cycle through palette)
- [ ] Implement status bar
  - [ ] Bottom bar showing:
    - [ ] Current mode (file view, search, filter)
    - [ ] Active filters
    - [ ] Selection info (X/Y)
    - [ ] Help hint ("Press ? for help")
- [ ] Add borders and separators
  - [ ] Box borders around panes
  - [ ] Separator between panes
  - [ ] Use Unicode box-drawing characters (with ASCII fallback)
- [ ] Add loading indicators
  - [ ] Spinner while loading large files
  - [ ] Progress bar for long operations
- [ ] Implement theme support (optional)
  - [ ] Load theme from config file
  - [ ] Allow custom color schemes
  - [ ] Light vs dark mode

### Success Criteria

- TUI looks polished and professional
- Colors improve usability
- Status bar is informative
- Borders and separators are clean

### Tests

- Manual: Visual inspection of TUI
- Manual: Test with different terminal emulators
- Manual: Test with light and dark terminal backgrounds

---

## Task 7: Performance Optimization

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 1-5

### Description

Optimize TUI rendering and responsiveness for large projects.

### Subtasks

- [ ] Add performance instrumentation
  - [ ] Measure rendering time
  - [ ] Measure event handling time
  - [ ] Track frame rate
- [ ] Optimize rendering
  - [ ] Only re-render changed regions (if supported by ratatui)
  - [ ] Throttle rendering (60 FPS max)
  - [ ] Use double buffering
- [ ] Optimize data loading
  - [ ] Lazy-load file contents (only when selected)
  - [ ] Cache loaded data
  - [ ] Invalidate cache on changes
- [ ] Handle large task lists
  - [ ] Virtual scrolling (render only visible rows)
  - [ ] Pagination for very large files
- [ ] Reduce DB queries
  - [ ] Batch queries where possible
  - [ ] Cache frequently accessed data
- [ ] Benchmark and document
  - [ ] Smooth rendering (>30 FPS) for large projects
  - [ ] Responsive input (<50ms latency)

### Success Criteria

- TUI is smooth and responsive
- No lag with large task files (100+ tasks)
- Rendering stays performant on resize

### Tests

- Benchmark: Measure rendering time with large fixtures
- Benchmark: Test input latency
- Manual: Use TUI with large project, assess smoothness

---

## Non-Goals (for v1)

- Mouse support (keyboard-only is sufficient)
- Inline editing (use `$EDITOR` instead)
- Multi-file editing (edit one file at a time)
- Split pane resizing (fixed layout is fine)
- Custom key bindings (use sensible defaults)

---

## Open Questions

- **Editor suspension:** How to handle editors that don't exit cleanly?
- **Status toggle:** Should toggling parent status update children?
- **Theme config:** TOML, JSON, or hardcoded themes?
- **Virtual scrolling:** Worth complexity for v1?

---

## References

- Design doc section 8 (TUI Design)
- `ratatui` documentation: https://docs.rs/ratatui/
- `crossterm` documentation: https://docs.rs/crossterm/
- TUI examples: https://github.com/ratatui-org/ratatui/tree/main/examples
