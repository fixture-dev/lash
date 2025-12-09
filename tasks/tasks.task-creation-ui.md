# Task Creation UI Tasks

**Module:** `lash-cli` (add command), `lash-tui` (form components), `lash-core` (validation)
**Dependencies:** tasks.cli-framework.md, tasks.tui.md, tasks.linter.md, tasks.core-data-model.md
**Effort:** 15-20 days
**Priority:** HIGH

## Overview

Implement task creation capabilities for both CLI (`lash add`) and TUI interfaces with a shared validation layer. The feature enables users and agents to create tasks through structured inputs with real-time validation and helpful error messages.

## Core Requirements

From design-doc.md and user requirements:
- CLI task creation via `lash add` command with rich flags and interactive mode
- TUI task creation via form-based modal with structured inputs (not free-form text editing)
- Shared validation layer in `lash-core` for consistent behavior across interfaces
- Helpful, intuitive error messages for validation failures
- Support for all task annotations: @id, @labels, @owner, @estimate, @depends-on, @agent-note

---

## Task 1: Core Types for Task Creation

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** tasks.core-data-model.md

### Description

Add task creation request types, builders, and error types to `lash-types` crate.

### Subtasks

- [ ] Create `creation.rs` module in lash-types
  - [ ] Define `FileTarget` enum (Current, Path, ContainingTask, NewFile)
  - [ ] Define `ParentRef` enum (None, Id, FullRef, AppendAtDepth)
  - [ ] Define `InsertPosition` enum (Append, AtIndex, Before, After)
  - [ ] Define `TaskCreationRequest` struct with all task fields
  - [ ] Implement `TaskCreationRequestBuilder` with fluent API
  - [ ] Define `TaskCreationResult` struct (created task, file path, line number)
- [ ] Create `creation_errors.rs` module in lash-types
  - [ ] Define `TaskCreationError` enum with all validation error variants:
    - [ ] EmptyTitle, TitleTooLong
    - [ ] FileNotFound, FileNotWritable, FileParseFailed
    - [ ] ParentNotFound, DepthLimitExceeded
    - [ ] DuplicateId, InvalidIdFormat
    - [ ] InvalidLabel, InvalidEstimate
    - [ ] DependencyNotFound, WouldCreateCycle
    - [ ] InvalidPosition
  - [ ] Implement `message()` method for user-friendly messages
  - [ ] Implement `help()` method for fix suggestions
  - [ ] Implement `to_diagnostic()` for consistent error display
  - [ ] Implement `error_code()` for stable error identifiers
- [ ] Export new modules from lash-types lib.rs
- [ ] Add comprehensive unit tests for builders and error formatting

### Success Criteria

- [ ] All creation types are defined with proper documentation
- [ ] Builder pattern provides ergonomic task creation API
- [ ] Error types provide both human and machine-readable output
- [ ] All tests pass

### Tests

- [ ] Unit: TaskCreationRequestBuilder fluent API
- [ ] Unit: TaskCreationError message formatting
- [ ] Unit: Error to diagnostic conversion
- [ ] Unit: Serialization/deserialization of request types

---

## Task 2: Validation Pipeline

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 1, tasks.linter.md

### Description

Implement the shared validation layer in `lash-core` that validates task creation requests for both CLI and TUI interfaces.

### Subtasks

- [ ] Create `creation/` module directory in lash-core
- [ ] Implement `validation.rs`
  - [ ] Define `ValidationContext` struct (config, resolved file, parent task, computed depth)
  - [ ] Implement `TaskValidator` struct
    - [ ] `validate()` - main entry point, collects all errors
    - [ ] `validate_title()` - non-empty, length limits (max 200 chars)
    - [ ] `validate_id()` - format check, uniqueness within file
    - [ ] `validate_label()` - alphanumeric with dashes only
    - [ ] `validate_estimate()` - format check (e.g., 1h, 2d, 3w, 30m)
    - [ ] `validate_owner()` - format validation
    - [ ] `resolve_parent()` - parent existence, depth computation
    - [ ] `resolve_file_target()` - file path resolution
  - [ ] Support collecting multiple errors (don't stop at first)
  - [ ] Return `ValidationResult<ValidationContext>` on success
- [ ] Implement dependency validation
  - [ ] `validate_dependencies()` - check targets exist
  - [ ] Integration with `CycleDetector` from lash-core
  - [ ] `dependency_target_exists()` helper
- [ ] Add validation configuration
  - [ ] Max title length (default: 200)
  - [ ] Max depth (from LashConfig)
  - [ ] ID format regex pattern
- [ ] Write comprehensive tests
  - [ ] Valid requests pass validation
  - [ ] Each error type is triggered correctly
  - [ ] Multiple errors collected in single validation

### Success Criteria

- [ ] Validation catches all invalid inputs
- [ ] Error messages are clear and actionable
- [ ] Validation is fast (<10ms for typical requests)
- [ ] All error types have test coverage

### Tests

- [ ] Unit: Title validation (empty, too long, valid)
- [ ] Unit: ID validation (format, uniqueness)
- [ ] Unit: Label validation (format)
- [ ] Unit: Estimate validation (formats)
- [ ] Unit: Parent resolution (exists, not found, depth)
- [ ] Unit: File target resolution
- [ ] Unit: Dependency cycle detection
- [ ] Integration: Full validation pipeline

---

## Task 3: Placement Resolution

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 1, Task 2

### Description

Implement logic to determine where to insert new tasks within existing Markdown files.

### Subtasks

- [ ] Implement `placement.rs` in lash-core/creation/
  - [ ] Define `PlacementInfo` struct (line_number, order_index, indent_level)
  - [ ] Implement `PlacementResolver::resolve()` entry point
  - [ ] Implement `resolve_append()` for parent/no-parent cases
  - [ ] Implement `resolve_at_index()` for specific position
  - [ ] Implement `resolve_before()` for inserting before task
  - [ ] Implement `resolve_after()` for inserting after task (including descendants)
- [ ] Implement helper methods
  - [ ] `find_end_of_tasks_section()` - locate ## Tasks section end
  - [ ] `find_insertion_after_parent()` - after parent's last child
  - [ ] `find_task_line()` - locate task's line number
  - [ ] `find_end_of_task_subtree()` - after all descendants
- [ ] Handle edge cases
  - [ ] Empty file (create new ## Tasks section)
  - [ ] No ## Tasks section exists
  - [ ] Parent has no existing children
  - [ ] Parent is last task in file
- [ ] Add source location tracking during parsing (if not present)
- [ ] Write tests for all placement scenarios

### Success Criteria

- [ ] Tasks inserted at correct positions in all cases
- [ ] Indentation matches parent/sibling levels
- [ ] Order indices are correct among siblings
- [ ] Edge cases handled gracefully

### Tests

- [ ] Unit: Append to empty file
- [ ] Unit: Append as top-level task
- [ ] Unit: Append as child of parent
- [ ] Unit: Insert before specific task
- [ ] Unit: Insert after task with children
- [ ] Integration: Complex file with multiple levels

---

## Task 4: Markdown Emitter

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 3

### Description

Implement Markdown generation and file writing for new tasks.

### Subtasks

- [ ] Implement `emitter.rs` in lash-core/creation/
  - [ ] Implement `MarkdownEmitter::emit()` entry point
  - [ ] Implement `insert_into_existing()` for existing files
  - [ ] Implement `create_new_file()` for new task files
  - [ ] Implement `format_task_line()` for proper markdown output
- [ ] Task line formatting
  - [ ] Correct indentation based on depth
  - [ ] Status checkbox ([ ], [x], [-], [!])
  - [ ] Task title
  - [ ] Inline labels (#label1 #label2)
  - [ ] Inline metadata block [@id: x, @owner: y]
- [ ] Multi-line annotation support
  - [ ] @depends-on on separate indented line
  - [ ] @agent-note on separate indented line
  - [ ] @estimate, @owner inline or separate
- [ ] File creation for new files
  - [ ] Generate proper header (# Title from filename or user input)
  - [ ] Generate file-level metadata block (@id, @status, @labels, @owner)
  - [ ] Create optional ## Description section
  - [ ] Create ## Tasks section
  - [ ] Insert the new task as first task
  - [ ] Ensure parent directories exist (create recursively)
  - [ ] Validate file path (no special characters, .md extension)
- [ ] Safe file writing
  - [ ] Create backup before modification (optional)
  - [ ] Atomic write where possible
  - [ ] Handle write errors gracefully

### Success Criteria

- [ ] Generated markdown passes `lash lint`
- [ ] Existing file formatting is preserved
- [ ] New files follow project conventions
- [ ] No data loss on write errors

### Tests

- [ ] Unit: Task line formatting (all status types)
- [ ] Unit: Inline labels formatting
- [ ] Unit: Metadata block formatting
- [ ] Unit: Multi-line annotations
- [ ] Integration: Insert into existing file
- [ ] Integration: Create new file
- [ ] Integration: Round-trip (create, parse, verify)

---

## Task 5: Task Creation Service

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 2, Task 3, Task 4

### Description

Implement the orchestration service that ties together validation, placement, and emission.

### Subtasks

- [ ] Implement `service.rs` in lash-core/creation/
  - [ ] Define `TaskCreationService` struct
  - [ ] Implement `create_task()` main entry point
    - [ ] Step 1: Load target file (if exists)
    - [ ] Step 2: Validate request
    - [ ] Step 3: Resolve placement
    - [ ] Step 4: Build task from request
    - [ ] Step 5: Emit to markdown
    - [ ] Step 6: Return result
  - [ ] Implement `load_target_file()` helper
  - [ ] Implement `build_task()` from request and context
  - [ ] Implement `generate_id()` for auto-generated IDs (from title slug)
  - [ ] Implement `identify_new_labels()` for reporting
- [ ] Add database update support (optional)
  - [ ] Insert task record after file write
  - [ ] Insert label records for new labels
  - [ ] Update file record (hash, mtime)
- [ ] Create module exports
  - [ ] `mod.rs` with pub use statements
  - [ ] Export from lash-core lib.rs
- [ ] Integration with existing parser
  - [ ] Reuse `parse_file()` for loading
  - [ ] Ensure consistency with parser output

### Success Criteria

- [ ] End-to-end task creation works
- [ ] Database stays in sync (if enabled)
- [ ] Clear error reporting at each step
- [ ] Service is reusable by CLI and TUI

### Tests

- [ ] Unit: Task building from request
- [ ] Unit: ID generation from title
- [ ] Integration: Create task in existing file
- [ ] Integration: Create task in new file
- [ ] Integration: Create nested task
- [ ] Integration: Verify database update

---

## Task 6: CLI `lash add` Command

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 5, tasks.cli-framework.md

### Description

Implement the `lash add` CLI command for creating tasks from the command line.

### Subtasks

- [ ] Define `AddArgs` struct with clap
  - [ ] `title` - required positional argument
  - [ ] `--file / -f` - target file path (creates new file if doesn't exist)
  - [ ] `--file-title` - title for new file header (defaults to filename, only used when creating)
  - [ ] `--file-description` - description for new file's ## Description section (only used when creating)
  - [ ] `--parent / -p` - parent task ID
  - [ ] `--after` - insert after task ID
  - [ ] `--before` - insert before task ID
  - [ ] `--label / -l` - labels (comma-separated, repeatable)
  - [ ] `--owner / -o` - task owner
  - [ ] `--estimate / -e` - time estimate
  - [ ] `--status` - initial status (enum: open, done, waived, blocked)
  - [ ] `--id` - explicit task ID
  - [ ] `--depends-on / -d` - dependencies (comma-separated, repeatable)
  - [ ] `--agent-note` - agent note text
  - [ ] `--edit` - open editor for extended description
  - [ ] `--format` - output format (text, json)
  - [ ] `--dry-run` - validate without creating
  - [ ] `--interactive / -i` - interactive mode
- [ ] Implement `execute()` function
  - [ ] Find project root
  - [ ] Load config
  - [ ] Handle interactive mode
  - [ ] Build TaskCreationRequest
  - [ ] Call TaskCreationService
  - [ ] Format and display result
- [ ] Implement interactive mode (using `dialoguer` crate)
  - [ ] `prompt_for_missing_fields()` function
  - [ ] File selection from discovered files
  - [ ] Parent task selection from file tasks
  - [ ] Label multi-select from existing labels
  - [ ] Owner input with suggestions
- [ ] Implement output formatting
  - [ ] `output_success()` - text and JSON formats
  - [ ] `output_errors()` - clear error display with help text
- [ ] Implement dry-run mode
  - [ ] Validate without writing
  - [ ] Show what would be created
- [ ] Register command in CLI
  - [ ] Add to `Commands` enum in cli.rs
  - [ ] Route in main.rs

### Success Criteria

- [ ] All flags work as documented
- [ ] Interactive mode guides users through creation
- [ ] Dry-run mode is useful for validation
- [ ] JSON output is parseable
- [ ] Exit codes follow convention (0=success, 1=validation error)

### Tests

- [ ] Unit: Argument parsing
- [ ] Unit: Request building from args
- [ ] Integration: Create simple task
- [ ] Integration: Create task with all options
- [ ] Integration: Create task in new file (file doesn't exist)
- [ ] Integration: New file with custom title and description
- [ ] Integration: Interactive mode (manual)
- [ ] Integration: Dry-run mode
- [ ] Integration: JSON output format
- [ ] Integration: Error cases

### CLI Usage Examples

```bash
# Basic task creation (adds to default/current file)
lash add "Implement user authentication"

# Create task in specific file (creates file if it doesn't exist)
lash add "Add login form" --file tasks/frontend.md

# Create task in new file with custom title and description
lash add "Initial setup" \
    --file tasks/auth-system.md \
    --file-title "Authentication System" \
    --file-description "Tasks for implementing OAuth2 and session management"

# Create subtask under parent
lash add "Write unit tests" --parent implement-auth --label testing

# Full example with all options
lash add "Implement OAuth2 flow" \
    --file tasks/auth.md \
    --parent auth-system \
    --label backend,security \
    --owner alice \
    --estimate 4h \
    --depends-on "tasks/core.md#task:session-manager" \
    --id oauth2-impl

# Interactive mode (prompts for file selection including "Create new file")
lash add "New feature" --interactive

# Dry run validation
lash add "Test task" --dry-run

# JSON output for scripting/agents
lash add "CI task" --format json
```

---

## Task 7: TUI Form Components

**Priority:** HIGH
**Effort:** 4-5 days
**Depends on:** tasks.tui.md

### Description

Implement reusable form components for the TUI task creation modal.

### Subtasks

- [ ] Create `components/` module in lash-tui
- [ ] Implement `TextInputState` component
  - [ ] `value: String` - current input
  - [ ] `cursor_position: usize` - cursor location
  - [ ] `placeholder: String` - hint text
  - [ ] `required: bool` - validation flag
  - [ ] `max_length: usize` - limit (0 = unlimited)
  - [ ] `suggestions: Vec<String>` - autocomplete options
  - [ ] `selected_suggestion: Option<usize>`
  - [ ] `show_suggestions: bool`
  - [ ] Input methods: `input_char()`, `backspace()`, `delete()`
  - [ ] Navigation: `cursor_left()`, `cursor_right()`, `home()`, `end()`
  - [ ] Autocomplete: `next_suggestion()`, `prev_suggestion()`, `accept_suggestion()`
- [ ] Implement `ChipInputState` component (for labels)
  - [ ] `chips: Vec<String>` - committed values
  - [ ] `input: String` - current partial input
  - [ ] `focused_chip: Option<usize>` - for deletion
  - [ ] `suggestions: Vec<String>` - autocomplete
  - [ ] Methods: `add_chip()`, `remove_chip()`, `focus_chip()`
  - [ ] Input handling for comma/enter to commit
- [ ] Implement `RadioSelectState<T>` component (for status)
  - [ ] `options: Vec<RadioOption<T>>` - available choices
  - [ ] `selected_index: usize` - current selection
  - [ ] Methods: `select_next()`, `select_prev()`, `select_by_key()`
- [ ] Implement `TreeSelectState` component (for parent task)
  - [ ] `input: String` - filter text
  - [ ] `filtered_tasks: Vec<TreeSelectItem>` - filtered list
  - [ ] `selected_index: usize` - highlighted item
  - [ ] `selected_parent: Option<TaskRecord>` - committed selection
  - [ ] `is_expanded: bool` - dropdown state
  - [ ] Methods: `filter()`, `select()`, `expand()`, `collapse()`
- [ ] Implement `MultiSelectState` component (for dependencies)
  - [ ] `input: String` - search filter
  - [ ] `all_options: Vec<DependencyOption>` - all available
  - [ ] `filtered_indices: Vec<usize>` - visible options
  - [ ] `selected_indices: HashSet<usize>` - checked items
  - [ ] Methods: `toggle_selection()`, `filter()`, `get_selected()`
- [ ] Implement `TextAreaState` component (for agent note)
  - [ ] `lines: Vec<String>` - multi-line content
  - [ ] `cursor_row: usize`, `cursor_col: usize`
  - [ ] `scroll_offset: usize` - for long content
  - [ ] Methods: `input_char()`, `newline()`, `backspace()`
- [ ] Write rendering functions for each component
  - [ ] `render_text_input()` - with focus highlight, error display
  - [ ] `render_chip_input()` - chips as tags, input area
  - [ ] `render_radio_select()` - horizontal options
  - [ ] `render_tree_select()` - dropdown with hierarchy
  - [ ] `render_multi_select()` - checkboxes in list
  - [ ] `render_text_area()` - multi-line with scroll

### Success Criteria

- [ ] All components handle keyboard input correctly
- [ ] Components are visually clear and consistent
- [ ] Focus states are obvious
- [ ] Components are reusable for other forms

### Tests

- [ ] Unit: TextInputState cursor movement
- [ ] Unit: ChipInputState chip management
- [ ] Unit: RadioSelectState selection cycling
- [ ] Unit: TreeSelectState filtering
- [ ] Unit: MultiSelectState toggle behavior
- [ ] Manual: Visual inspection of all components

---

## Task 8: TUI Task Creation Modal

**Priority:** HIGH
**Effort:** 3-4 days
**Depends on:** Task 5, Task 7

### Description

Implement the task creation modal for the TUI using the form components.

### Subtasks

- [ ] Define `TaskCreationModalState` in state.rs
  - [ ] `focused_field: TaskFormField` - current focus
  - [ ] `mode: TaskCreationMode` - AddToExisting or CreateNewFile
  - [ ] `title: TextInputState`
  - [ ] `file_selector: Option<FileSelectState>` - for choosing/creating file
  - [ ] `new_file_path: TextInputState` - path for new file
  - [ ] `new_file_title: TextInputState` - title for new file header
  - [ ] `new_file_description: TextAreaState` - description for new file
  - [ ] `parent_selector: Option<TreeSelectState>`
  - [ ] `labels: ChipInputState`
  - [ ] `status: RadioSelectState<TaskStatus>`
  - [ ] `owner: TextInputState`
  - [ ] `estimate: TextInputState`
  - [ ] `dependencies: MultiSelectState`
  - [ ] `agent_note: TextAreaState`
  - [ ] `errors: HashMap<TaskFormField, String>` - validation errors
  - [ ] `show_preview: bool` - markdown preview toggle
  - [ ] `target_file: Option<FileRecord>` - context (None for new file)
- [ ] Define `TaskFormField` enum for navigation
- [ ] Define `TaskCreationMode` enum (AddToExisting, CreateNewFile)
- [ ] Implement modal state methods
  - [ ] `open_task_creation_modal()` - initialize with context
  - [ ] `open_new_file_modal()` - initialize for new file creation
  - [ ] `close_task_creation_modal()` - cleanup
  - [ ] `toggle_mode()` - switch between add/create modes
  - [ ] `next_field()`, `prev_field()` - Tab navigation
  - [ ] `to_request()` - build TaskCreationRequest
  - [ ] `validate_form()` - real-time validation
  - [ ] `can_submit()` - check for blocking errors
- [ ] Create `task_creation_modal.rs` in ui/
  - [ ] `render()` - main modal rendering
  - [ ] Centered popup layout (70% width, 80% height)
  - [ ] Field layout with proper spacing
  - [ ] Error display inline below fields
  - [ ] Action bar with keyboard hints
- [ ] Implement markdown preview panel
  - [ ] `generate_markdown_preview()` - format task as markdown
  - [ ] Toggle with Ctrl+P
  - [ ] Collapsible to save space
- [ ] Implement responsive layout
  - [ ] Full form for large terminals
  - [ ] Compact scrollable form for small terminals (<80 cols or <30 rows)
- [ ] Add new AppEvent variants in event.rs
  - [ ] `OpenTaskCreation`, `CloseTaskCreation`, `SubmitTaskCreation`
  - [ ] `TaskFormNextField`, `TaskFormPrevField`
  - [ ] `TaskFormTogglePreview`
  - [ ] `TaskFormExpandDropdown`, `TaskFormCollapseDropdown`
  - [ ] `TaskFormToggleSelection`
- [ ] Implement `poll_task_creation_event()` in event.rs
  - [ ] Handle all form-specific key events
  - [ ] Global: Esc (close), Ctrl+S/Ctrl+Enter (submit), Tab/Shift+Tab (navigate)
  - [ ] Text: char input, backspace, delete, cursor movement
  - [ ] Selection: Up/Down, Enter, Space
- [ ] Integrate with TuiApp in app.rs
  - [ ] Add keybinding to open modal (`a` or `n`)
  - [ ] Route events to modal when open
  - [ ] `handle_submit_task_creation()` - call service, show result
  - [ ] Refresh task list after creation
  - [ ] Show success/error in status bar

### Success Criteria

- [ ] Modal opens with proper context from current selection
- [ ] All fields are navigable and functional
- [ ] Real-time validation shows errors inline
- [ ] Markdown preview is accurate
- [ ] Submit creates task and closes modal
- [ ] Cancel closes without changes
- [ ] Can switch between "Add to file" and "Create new file" modes
- [ ] New file mode shows file path, title, and description fields
- [ ] New file is created with proper structure on submit

### Tests

- [ ] Unit: Form state navigation
- [ ] Unit: Request building from form
- [ ] Unit: Markdown preview generation
- [ ] Unit: Mode toggling (AddToExisting <-> CreateNewFile)
- [ ] Integration: Open modal, fill form, submit
- [ ] Integration: Validation error display
- [ ] Integration: Create new file via modal
- [ ] Integration: New file preview shows full file structure
- [ ] Manual: Full form interaction testing

### ASCII Mockup

```
+============================================================================+
|                           Create New Task                                  |
+============================================================================+
|                                                                            |
|  Title *                                                                   |
|  +----------------------------------------------------------------------+  |
|  | Implement sepia filter core                                          |  |
|  +----------------------------------------------------------------------+  |
|                                                                            |
|  Parent Task                                                               |
|  +----------------------------------------------------------------------+  |
|  | None (top-level task)                                           [v]  |  |
|  +----------------------------------------------------------------------+  |
|                                                                            |
|  Labels                                                                    |
|  +----------------------------------------------------------------------+  |
|  | [backend] [image-processing] _                                       |  |
|  +----------------------------------------------------------------------+  |
|                                                                            |
|  Status                                                                    |
|  ( ) Open    ( ) Done    ( ) Waived    ( ) Blocked                        |
|                                                                            |
|  Owner                          Estimate                                   |
|  +---------------------------+  +---------------------------+              |
|  | frank                     |  | 2h                        |              |
|  +---------------------------+  +---------------------------+              |
|                                                                            |
|  Preview:                                                                  |
|  +----------------------------------------------------------------------+  |
|  | - [ ] Implement sepia filter core #backend #image-processing         |  |
|  |   @owner: frank @estimate: 2h                                        |  |
|  +----------------------------------------------------------------------+  |
|                                                                            |
|  Tab: Next field | Ctrl+S: Save | Esc: Cancel | Ctrl+P: Toggle preview    |
+============================================================================+
```

---

## Task 9: Keyboard Navigation & Help

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 8

### Description

Implement comprehensive keyboard navigation and help for the task creation form.

### Subtasks

- [ ] Define keyboard navigation scheme
  - [ ] Tab / Shift+Tab: Move between fields
  - [ ] Up/Down or Ctrl+P/N: Navigate within dropdowns/lists
  - [ ] Enter: Select in dropdown, submit form (on last field)
  - [ ] Space: Toggle selection (multi-select, radio)
  - [ ] Escape: Close dropdown, or close modal
  - [ ] Ctrl+S or Ctrl+Enter: Submit form
  - [ ] Ctrl+P: Toggle markdown preview
  - [ ] Ctrl+U: Clear current field
  - [ ] F1 or ?: Show help overlay
- [ ] Implement focus flow (circular)
  ```
  Title -> Parent -> Labels -> Status -> Owner -> Estimate -> Dependencies -> AgentNote -> Title
  ```
- [ ] Add visual focus indicators
  - [ ] Border color change for focused field
  - [ ] Cursor display in text inputs
  - [ ] Highlight in selection lists
- [ ] Create help overlay for form
  - [ ] List all keyboard shortcuts
  - [ ] Field-specific hints
  - [ ] Show when F1/? pressed
- [ ] Add field-specific shortcuts
  - [ ] Status field: O/D/W/B for Open/Done/Waived/Blocked
  - [ ] Labels field: Enter or comma to add chip
  - [ ] Backspace (empty input): Focus previous chip

### Success Criteria

- [ ] All actions achievable via keyboard only
- [ ] Navigation is intuitive and discoverable
- [ ] Help overlay is comprehensive
- [ ] Focus indicators are clear

### Tests

- [ ] Unit: Focus flow cycling
- [ ] Manual: Navigate entire form with keyboard
- [ ] Manual: Help overlay accuracy

---

## Task 10: Validation UX & Error Display

**Priority:** MEDIUM
**Effort:** 2 days
**Depends on:** Task 2, Task 8

### Description

Implement real-time validation with clear, inline error display in both CLI and TUI.

### Subtasks

- [ ] Implement real-time validation for TUI
  - [ ] Debounced validation (100ms delay)
  - [ ] Validate on each field change
  - [ ] Update `errors` map in modal state
- [ ] Implement inline error display
  - [ ] Show error icon and message below field
  - [ ] Use red/yellow colors for error/warning
  - [ ] Clear error when field becomes valid
- [ ] Implement blocking vs non-blocking errors
  - [ ] Blocking: Empty title, invalid format
  - [ ] Non-blocking: Title very long (warning)
  - [ ] Submit blocked only by blocking errors
- [ ] CLI error formatting
  - [ ] Group errors by field
  - [ ] Show error code, message, and help
  - [ ] Colored output (respecting --no-color)
  - [ ] JSON error output for agents
- [ ] Add validation feedback
  - [ ] Green checkmark for valid required fields
  - [ ] Character count for title
  - [ ] Format hints for estimate field

### Success Criteria

- [ ] Users immediately see what's wrong
- [ ] Error messages suggest fixes
- [ ] Validation doesn't lag UI
- [ ] All error codes are documented

### Tests

- [ ] Unit: Debounced validation timing
- [ ] Unit: Error/warning classification
- [ ] Integration: All error types display correctly
- [ ] Manual: Real-time validation responsiveness

---

## Task 11: Autocomplete & Suggestions

**Priority:** MEDIUM
**Effort:** 2-3 days
**Depends on:** Task 7, Task 8

### Description

Implement autocomplete for labels, owners, and task references.

### Subtasks

- [ ] Load available options from database
  - [ ] Existing labels (with usage counts)
  - [ ] Existing owners (from task metadata)
  - [ ] Existing task IDs (for parent selection)
  - [ ] Existing file paths (for dependencies)
- [ ] Implement label autocomplete
  - [ ] Filter as user types
  - [ ] Show usage count next to label
  - [ ] Highlight matching characters
  - [ ] Allow creating new labels
- [ ] Implement owner autocomplete
  - [ ] Show existing owners from project
  - [ ] Allow typing new owner
- [ ] Implement parent task selector
  - [ ] Hierarchical display with indentation
  - [ ] Filter by typing
  - [ ] Show task status indicators
  - [ ] "None (top-level)" option at top
- [ ] Implement dependency selector
  - [ ] Search across all files and tasks
  - [ ] Show file path + task title
  - [ ] Multi-select with checkboxes
  - [ ] Prevent selecting self or descendants
- [ ] Performance optimization
  - [ ] Cache loaded options
  - [ ] Limit displayed suggestions (max 10-15)
  - [ ] Fuzzy matching for better results

### Success Criteria

- [ ] Autocomplete is fast and responsive
- [ ] Suggestions are relevant and helpful
- [ ] Users can discover existing labels/owners
- [ ] Parent/dependency selection is intuitive

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

### Description

Document the task creation feature for users and update help text.

### Subtasks

- [ ] Update CLI help text
  - [ ] `lash add --help` comprehensive examples
  - [ ] Document all flags with examples
  - [ ] Add to main `lash --help` command list
- [ ] Update TUI help overlay
  - [ ] Add task creation section
  - [ ] Document `a` or `n` keybinding
  - [ ] List form navigation keys
- [ ] Add to user documentation
  - [ ] Create `docs/task-creation.md`
  - [ ] CLI usage examples
  - [ ] TUI walkthrough with screenshots
  - [ ] Common workflows
- [ ] Add to agent documentation
  - [ ] Update `lash agent-prompt` output
  - [ ] Document JSON output format
  - [ ] Add task creation to allowed operations
- [ ] Error code documentation
  - [ ] List all E_CREATE_* codes
  - [ ] Explain causes and fixes
  - [ ] Add to `lash explain` command

### Success Criteria

- [ ] Users can learn feature from --help
- [ ] Documentation is complete and accurate
- [ ] Agents understand how to create tasks
- [ ] Error codes are explained

### Tests

- [ ] Review: Help text accuracy
- [ ] Review: Documentation completeness

---

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1-2)
1. Task 1: Core Types for Task Creation
2. Task 2: Validation Pipeline

### Phase 2: File Operations (Week 2-3)
3. Task 3: Placement Resolution
4. Task 4: Markdown Emitter
5. Task 5: Task Creation Service

### Phase 3: CLI Implementation (Week 3-4)
6. Task 6: CLI `lash add` Command

### Phase 4: TUI Components (Week 4-5)
7. Task 7: TUI Form Components
8. Task 8: TUI Task Creation Modal

### Phase 5: Polish (Week 5-6)
9. Task 9: Keyboard Navigation & Help
10. Task 10: Validation UX & Error Display
11. Task 11: Autocomplete & Suggestions
12. Task 12: Documentation & Help

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
