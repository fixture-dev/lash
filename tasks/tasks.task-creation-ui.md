# Task Creation UI Tasks

**Module:** `lash-cli` (add command), `lash-tui` (form components), `lash-core` (validation)
**Dependencies:** tasks.cli-framework.md, tasks.tui.md, tasks.linter.md, tasks.core-data-model.md
**Effort:** 15-20 days
**Priority:** HIGH

## Overview

Implement task creation capabilities for both CLI (`lash add`) and TUI interfaces with a shared validation layer. The feature enables users and agents to create tasks through structured inputs with real-time validation and helpful error messages.

## Archived Tasks

Tasks 1-10 have been completed and archived to `tasks/archived/archived.tasks.task-creation-ui.md`.

**Completed on:** 2025-12-09
**Commit:** `0a5bb3e Implement task creation feature with TUI modal and CLI support`

Archived tasks:
- Task 1: Core Types for Task Creation
- Task 2: Validation Pipeline
- Task 3: Placement Resolution
- Task 4: Markdown Emitter
- Task 5: Task Creation Service
- Task 6: CLI `lash add` Command
- Task 7: TUI Form Components
- Task 8: TUI Task Creation Modal
- Task 9: Keyboard Navigation & Help
- Task 10: Validation UX & Error Display

---

## Task 11: Autocomplete & Suggestions

**Priority:** MEDIUM
**Effort:** 2-3 days
**Depends on:** Task 7, Task 8
**Status:** Partially Complete

### Description

Implement autocomplete for labels, owners, and task references.

### Subtasks

- [x] Load available options from database
  - [x] Existing labels (with usage counts)
  - [x] Existing owners (from task metadata) - `get_distinct_owners()` added
  - [x] Existing task IDs (for parent selection)
  - [x] Existing file paths (for dependencies)
- [x] Implement label autocomplete
  - [x] Filter as user types
  - [ ] Show usage count next to label
  - [ ] Highlight matching characters
  - [x] Allow creating new labels
- [x] Implement owner autocomplete
  - [x] Show existing owners from project
  - [x] Allow typing new owner
- [x] Implement parent task selector
  - [x] Hierarchical display with indentation
  - [x] Filter by typing
  - [x] Show task status indicators
  - [x] "None (top-level)" option at top
- [x] Implement dependency selector
  - [x] Search across all files and tasks
  - [x] Show file path + task title
  - [x] Multi-select with checkboxes
  - [ ] Prevent selecting self or descendants
- [ ] Performance optimization
  - [ ] Cache loaded options
  - [ ] Limit displayed suggestions (max 10-15)
  - [ ] Fuzzy matching for better results

### Success Criteria

- [x] Autocomplete is fast and responsive
- [x] Suggestions are relevant and helpful
- [x] Users can discover existing labels/owners
- [x] Parent/dependency selection is intuitive

### Tests

- [ ] Unit: Label filtering
- [ ] Unit: Fuzzy matching
- [ ] Integration: Load options from populated database
- [ ] Manual: Autocomplete UX testing

---

## Task 12: Documentation & Help

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 6, Task 8
**Status:** Partially Complete

### Description

Document the task creation feature for users and update help text.

### Subtasks

- [x] Update CLI help text
  - [x] `lash add --help` comprehensive examples
  - [x] Document all flags with examples
  - [x] Add to main `lash --help` command list
- [x] Update TUI help overlay
  - [x] Add task creation section
  - [x] Document `a` or `n` keybinding
  - [x] List form navigation keys
- [ ] Add to user documentation
  - [ ] Create `docs/task-creation.md`
  - [ ] CLI usage examples
  - [ ] TUI walkthrough with screenshots
  - [ ] Common workflows
- [ ] Add to agent documentation
  - [ ] Update `lash agent-prompt` output
  - [ ] Document JSON output format
  - [ ] Add task creation to allowed operations
- [x] Error code documentation
  - [x] List all E_CREATE_* codes
  - [x] Explain causes and fixes
  - [ ] Add to `lash explain` command

### Success Criteria

- [x] Users can learn feature from --help
- [ ] Documentation is complete and accurate
- [ ] Agents understand how to create tasks
- [x] Error codes are explained

### Tests

- [x] Review: Help text accuracy
- [ ] Review: Documentation completeness

---

## File Structure

```
crates/
  lash-types/src/
    creation.rs           # TaskCreationRequest, builders, results
    creation_errors.rs    # Validation error types
    lib.rs               # Export new modules

  lash-core/src/
    creation/
      mod.rs             # Module exports
      validation.rs      # TaskValidator, ValidationContext
      placement.rs       # PlacementResolver
      emitter.rs         # MarkdownEmitter
      service.rs         # TaskCreationService
    lib.rs               # pub mod creation;

  lash-cli/src/
    commands/
      add.rs             # Add command implementation
      mod.rs             # Export add command

  lash-tui/src/
    components/
      mod.rs             # Component module declaration
      text_input.rs      # TextInput component
      chip_input.rs      # ChipInput component (labels)
      radio_select.rs    # RadioSelect component (status)
      tree_select.rs     # TreeSelect component (parent)
      multi_select.rs    # MultiSelect component (dependencies)
      text_area.rs       # TextArea component (agent note)
    ui/
      task_creation_modal.rs  # Task creation modal rendering
```

---

## Non-Goals (for v1)

- Bulk task creation (multiple tasks at once)
- Task templates/presets
- Import from external formats (Jira, GitHub Issues)
- Drag-and-drop reordering in TUI
- Undo/redo for task creation

---

## Open Questions

- **Auto-index after creation:** Should creating a task automatically update the SQLite index, or require manual `lash index`?
- ~~**New file creation:** Should `lash add --file new-file.md` automatically create the file with proper structure?~~ **DECIDED: Yes** - `--file` creates if file doesn't exist, with optional `--file-title` and `--file-description` for customization
- **Default file:** When no file is specified, should it use the currently selected file (TUI) or a configured default?
- **ID generation:** Should auto-generated IDs include a numeric suffix for uniqueness, or just the title slug?

---

## References

- Design doc sections 4 (Markdown File Format), 7.3 (CLI Commands)
- tasks.cli-framework.md for command implementation patterns
- tasks.tui.md for TUI modal patterns
- `/crates/lash-tui/src/ui/search_modal.rs` - Reference implementation for modals
- `/crates/lash-tui/src/ui/filter_modal.rs` - Reference for picker components
- `/crates/lash-types/src/task.rs` - Task and TaskMetadata definitions
- `tasks/archived/archived.tasks.task-creation-ui.md` - Completed tasks archive
