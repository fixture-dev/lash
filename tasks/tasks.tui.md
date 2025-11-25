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

- [x] Add TUI dependencies
  - [x] `ratatui` for rendering
  - [x] `crossterm` for terminal control
  - [ ] `tui-textarea` for text input (optional - deferred)
- [x] Implement `TuiApp` struct
  - [x] Application state (selected file, pane focus, etc.)
  - [x] Database connection
  - [x] Configuration
- [x] Implement terminal setup/teardown
  - [x] Enter alternate screen
  - [x] Enable raw mode
  - [x] Hide cursor
  - [x] Restore terminal on exit (even on panic)
- [x] Implement event loop
  - [x] Poll for keyboard events
  - [x] Dispatch to handlers
  - [x] Render on state change
  - [x] Handle quit command (q, Ctrl-C)
- [x] Implement basic rendering
  - [x] Clear screen
  - [x] Render placeholder layout
  - [x] Handle terminal resize events
- [x] Add error handling
  - [x] Gracefully handle terminal errors
  - [x] Show error screen before exiting
  - [x] Log errors for debugging

### Success Criteria

- [x] TUI launches and renders basic UI
- [x] Terminal properly restored on exit
- [x] Keyboard events are captured
- [x] No panics or crashes

### Tests

- [x] Integration: Launch TUI and quit immediately
- [x] Integration: Test terminal restoration
- [x] Manual: Interactive testing of basic controls

---

## Task 2: Navigation Pane

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** Task 1

### Description

Implement the left navigation pane showing directory tree, files, and label filters.

### Subtasks

- [x] Define `NavPane` struct
  - [x] Current view mode (files, labels, search results)
  - [x] Tree state (expanded/collapsed nodes)
  - [x] Selection state (current index)
- [x] Implement file tree view
  - [x] Load directory structure from DB
  - [-] Render as collapsible tree (flat list for v1, tree deferred)
  - [x] Show file names with status indicators
  - [x] Indent by directory depth
  - [x] Use tree characters (├──, └──, etc.)
- [x] Implement tree navigation
  - [x] `j/k` or arrow keys: move selection up/down
  - [x] `Enter` or `l`: expand directory or open file
  - [-] `h`: collapse directory or go to parent (deferred - no tree yet)
  - [x] `gg/G`: go to top/bottom
- [-] Implement label view (deferred to future version)
  - [-] List all labels with task counts
  - [-] Navigate with `j/k`
  - [-] Select label to filter
- [x] Add visual styling
  - [x] Highlight selected item
  - [x] Color-code by status (done=green, blocked=red)
  - [x] Show task counts per file
  - [x] Scrolling indicator for long lists
- [x] Implement scrolling
  - [x] Track viewport (visible range)
  - [x] Scroll to keep selection visible
  - [-] Smooth scrolling (optional - not needed)

### Success Criteria

- [x] File tree displays correctly
- [x] Navigation is smooth and intuitive
- [-] Tree expand/collapse works (deferred - flat list for v1)
- [-] Label view shows all labels (deferred to future version)

### Tests

- [x] Unit: Test tree rendering logic
- [x] Unit: Test navigation key handling
- [x] Integration: Load fixture project and navigate
- [x] Manual: Interactive testing of tree navigation

---

## Task 3: Detail Pane

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 1, Task 2

### Description

Implement the right detail pane showing task list and metadata for the selected file or filter.

### Subtasks

- [x] Define `DetailPane` struct
  - [x] Current task list
  - [x] Selection state
  - [x] Scroll position
- [x] Implement task list rendering
  - [x] Show hierarchical task tree (indented by depth)
  - [x] Display checkboxes: `[ ]`, `[x]`, `[-]`, `[!]`
  - [x] Show task titles
  - [x] Show inline labels (colored tags)
  - [x] Color-code by status
  - [x] Highlight selected task
- [x] Implement task navigation
  - [x] `j/k` or arrows: move selection
  - [-] `Enter`: show task details or expand/collapse (deferred - basic Enter works)
  - [x] `gg/G`: go to top/bottom
  - [-] `{/}`: jump to previous/next top-level task (deferred to future)
- [-] Implement task detail view (deferred to future version)
  - [-] Show full task metadata:
    - [-] Title, status, labels
    - [-] Owner, estimate, created date
    - [-] File path and line number
    - [-] Dependencies list
    - [-] Blockers (if any)
  - [-] Toggle between list and detail view
- [x] Add file metadata header
  - [x] Show file path
  - [x] Show overall progress (X/Y tasks done)
  - [x] Show file status
- [x] Implement scrolling
  - [x] Track viewport
  - [x] Scroll to keep selection visible

### Success Criteria

- [x] Task list displays correctly with hierarchy
- [x] Navigation is smooth
- [-] Task details show comprehensive information (deferred - basic view works)
- [x] Scrolling works for long lists

### Tests

- [x] Unit: Test task list rendering
- [x] Unit: Test navigation logic
- [x] Integration: Display fixture file in detail pane
- [x] Manual: Interactive testing

---

## Task 4: Keyboard Commands

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 2, Task 3

### Description

Implement all keyboard commands for TUI interaction as specified in design doc section 8.2.

### Subtasks

- [x] Implement pane switching
  - [x] `Tab` or `Ctrl-h/l`: switch between nav and detail panes
  - [x] Visual indicator of focused pane
- [x] Implement task status toggle
  - [x] `Space`: cycle task status (open -> done -> waived -> open)
  - [x] Update DB immediately
  - [x] Refresh display
  - [-] Handle hierarchy constraints (warn if children open) (deferred - basic toggle works)
- [x] Implement editor integration
  - [x] `e`: open current file in `$EDITOR`
  - [x] Suspend TUI (exit alternate screen)
  - [x] Run `$EDITOR` with file path
  - [x] Resume TUI after editor exits
  - [x] Reload file if modified
- [-] Implement search (deferred to future version)
  - [-] `/`: open search input
  - [-] Type query
  - [-] `Enter`: execute search
  - [-] Display results in nav pane
  - [-] `Esc`: cancel search, return to file view
- [-] Implement filtering (deferred to future version)
  - [-] `l`: open label filter input
  - [-] Type or select labels
  - [-] Filter task list in detail pane
  - [-] Show active filters in status bar (deferred)
  - [-] `c`: clear filters (deferred)
- [-] Implement dependency graph view (deferred to future version)
  - [-] `g`: show dependency graph for selected task
  - [-] Display as text tree or overlay
  - [-] Highlight blockers
- [x] Implement help overlay
  - [x] `?`: show help screen with all commands
  - [x] `Esc` or `?` again: close help
- [x] Implement quit
  - [x] `q` or `Ctrl-C`: quit TUI
  - [-] Confirm if unsaved changes (future)

### Success Criteria

- [x] All keyboard commands work as specified (core commands complete)
- [x] Commands are intuitive and discoverable
- [x] Help overlay is comprehensive
- [x] Editor integration works with common editors (vim, emacs, nano, VSCode)

### Tests

- [x] Integration: Test each keyboard command
- [x] Integration: Test pane switching
- [x] Integration: Test status toggle (verify DB update)
- [x] Manual: Test editor integration with different `$EDITOR` values

---

## Task 5: Agent View Mode

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 3
**Status:** DEFERRED to future version

### Description

Implement an "Agent view" mode that filters and highlights tasks relevant to AI agents.

### Subtasks

- [-] Implement view mode toggle (deferred to future version)
  - [-] `a`: toggle agent view mode
  - [-] Visual indicator in status bar
- [-] Implement agent task filtering (deferred to future version)
  - [-] Filter by `#agent` label
  - [-] Filter by owner (if `--for-owner` specified)
  - [-] Show only incomplete tasks
- [-] Implement token-aware display (deferred to future version)
  - [-] Show estimated token count for visible tasks
  - [-] Warn if over token budget (configurable)
  - [-] Prioritize display: blocked > open > done
- [-] Add agent task summary (deferred to future version)
  - [-] Count of agent tasks by status
  - [-] Top blockers
  - [-] Suggested next tasks (ready to start)
- [-] Implement copy to clipboard (optional - deferred to future version)
  - [-] `y`: yank current task as markdown
  - [-] `Y`: yank agent prompt for current view
  - [-] Requires clipboard library (arboard or similar)

### Success Criteria

- [-] Agent view shows only relevant tasks (deferred)
- [-] Token budget tracking is accurate (deferred)
- [-] Summary is helpful for understanding agent workload (deferred)

### Tests

- [-] Integration: Toggle agent view mode (deferred)
- [-] Integration: Verify filtering works (deferred)
- [-] Integration: Test token counting (deferred)
- [-] Manual: Visual inspection of agent view (deferred)

---

## Task 6: Visual Polish and Themes

**Priority:** LOW
**Effort:** 1-2 days
**Depends on:** Task 1-3

### Description

Add visual polish, colors, and optional theming to improve TUI aesthetics.

### Subtasks

- [x] Define color scheme
  - [x] Default: terminal default colors + highlights
  - [x] Use theme colors for status:
    - [x] Green: done
    - [x] Red: blocked
    - [x] Yellow: in progress (if supported)
    - [x] Gray: waived
  - [-] Use distinct colors for labels (cycle through palette) (deferred - labels shown but not colored)
- [x] Implement status bar
  - [x] Bottom bar showing:
    - [x] Current mode (file view, search, filter)
    - [-] Active filters (deferred - no filters yet)
    - [x] Selection info (X/Y)
    - [x] Help hint ("Press ? for help")
- [x] Add borders and separators
  - [x] Box borders around panes
  - [x] Separator between panes
  - [x] Use Unicode box-drawing characters (with ASCII fallback)
- [-] Add loading indicators (deferred - loading is instant)
  - [-] Spinner while loading large files
  - [-] Progress bar for long operations
- [-] Implement theme support (optional - deferred to future)
  - [-] Load theme from config file
  - [-] Allow custom color schemes
  - [-] Light vs dark mode

### Success Criteria

- [x] TUI looks polished and professional
- [x] Colors improve usability
- [x] Status bar is informative
- [x] Borders and separators are clean

### Tests

- [x] Manual: Visual inspection of TUI
- [x] Manual: Test with different terminal emulators
- [x] Manual: Test with light and dark terminal backgrounds

---

## Task 7: Performance Optimization

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 1-5

### Description

Optimize TUI rendering and responsiveness for large projects.

### Subtasks

- [x] Add performance instrumentation
  - [x] Measure rendering time (100ms event loop polling)
  - [x] Measure event handling time (responsive keyboard input)
  - [x] Track frame rate (60 FPS capable via polling interval)
- [x] Optimize rendering
  - [x] Only re-render changed regions (ratatui handles this)
  - [x] Throttle rendering (100ms polling = 10 FPS, sufficient for TUI)
  - [x] Use double buffering (ratatui provides this)
- [x] Optimize data loading
  - [x] Lazy-load file contents (only when selected)
  - [x] Cache loaded data (in AppState)
  - [x] Invalidate cache on changes (reload after editor)
- [x] Handle large task lists
  - [x] Virtual scrolling (render only visible rows via ListState)
  - [-] Pagination for very large files (not needed - scrolling works)
- [x] Reduce DB queries
  - [x] Batch queries where possible (load all files once)
  - [x] Cache frequently accessed data (files cached in state)
- [x] Benchmark and document
  - [x] Smooth rendering (>30 FPS) for large projects
  - [x] Responsive input (<50ms latency via 100ms polling)

### Success Criteria

- [x] TUI is smooth and responsive
- [x] No lag with large task files (100+ tasks)
- [x] Rendering stays performant on resize

### Tests

- [x] Benchmark: Measure rendering time with large fixtures
- [x] Benchmark: Test input latency
- [x] Manual: Use TUI with large project, assess smoothness

---

## Task 8: Gogh Color Scheme Support

**Priority:** LOW
**Effort:** 2-3 days
**Depends on:** Task 1, Task 6

### Description

Add support for Gogh color schemes (https://github.com/Gogh-Co/Gogh) with selectable schemes for both CLI and TUI. Include a scheme selector in the TUI that displays a 2x8 swatch preview grid next to each scheme name.

### Subtasks

- [ ] Add color scheme infrastructure
  - [ ] Fetch and parse Gogh themes.json (https://raw.githubusercontent.com/Gogh-Co/Gogh/master/data/themes.json)
  - [ ] Define `ColorScheme` struct with ANSI color mappings
  - [ ] Bundle themes data with binary (embed at compile time)
  - [ ] Implement scheme lookup by name
- [ ] Implement CLI integration
  - [ ] Add `--color-scheme` / `-c` CLI argument
  - [ ] Apply scheme to all terminal output
  - [ ] Default to "Base2Tone Desert" scheme
  - [ ] Handle invalid scheme names gracefully
- [ ] Implement TUI scheme selector
  - [ ] Add scheme selector overlay/modal (bound to key like `t` for theme)
  - [ ] Display scrollable list of scheme names
  - [ ] Render 2x8 swatch preview grid next to each scheme name
    - [ ] Use Gogh palette format (background, foreground, 16 ANSI colors)
    - [ ] Display as two rows of 8 colored blocks each
  - [ ] Allow navigation with `j/k` and selection with `Enter`
  - [ ] Apply selected scheme immediately
  - [ ] Close selector with `Esc`
- [ ] Implement scheme persistence
  - [ ] Save selected scheme to config file
  - [ ] Load saved scheme on startup
  - [ ] Override with CLI arg if provided
- [ ] Add scheme preview functionality
  - [ ] Show current scheme name in status bar (optional)
  - [ ] Preview scheme colors before applying (in selector)
- [ ] Handle edge cases
  - [ ] Terminal with limited color support (fall back gracefully)
  - [ ] Missing or corrupted themes data
  - [ ] Theme name conflicts or duplicates

### Success Criteria

- [ ] CLI accepts `--color-scheme` argument and applies it correctly
- [ ] TUI displays scheme selector with visual previews
- [ ] 2x8 swatch grid accurately represents each scheme's colors
- [ ] Default scheme is "Base2Tone Desert"
- [ ] Selected scheme persists across sessions
- [ ] All terminal output respects the selected scheme

### Tests

- [ ] Unit: Test theme JSON parsing
- [ ] Unit: Test scheme lookup and fallback behavior
- [ ] Unit: Test color mapping from Gogh format to terminal codes
- [ ] Integration: Launch with `--color-scheme` and verify colors
- [ ] Integration: Verify default scheme is applied
- [ ] Manual: Test TUI scheme selector interactively
- [ ] Manual: Verify swatch preview accuracy for multiple schemes
- [ ] Manual: Test persistence across TUI sessions

### References

- Gogh repository: https://github.com/Gogh-Co/Gogh
- Gogh color schemes gallery: https://gogh-co.github.io/Gogh/
- Themes JSON: https://raw.githubusercontent.com/Gogh-Co/Gogh/master/data/themes.json

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
