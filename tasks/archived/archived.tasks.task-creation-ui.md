# Archived: Task Creation UI Tasks

This file contains completed tasks from the Task Creation UI feature implementation.

## Archived on 2025-12-09

**Commit:** `0a5bb3e Implement task creation feature with TUI modal and CLI support`

---

## Task 1: Core Types for Task Creation ✅ COMPLETE

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** tasks.core-data-model.md

### Description

Add task creation request types, builders, and error types to `lash-types` crate.

### Files Created/Modified

- `crates/lash-types/src/creation.rs` - TaskCreationRequest, builders, results
- `crates/lash-types/src/creation_errors.rs` - TaskCreationError enum with all variants
- `crates/lash-types/src/lib.rs` - Module exports

### Subtasks

- [x] Create `creation.rs` module in lash-types
  - [x] Define `FileTarget` enum (Current, Path, ContainingTask, NewFile)
  - [x] Define `ParentRef` enum (None, Id, FullRef, AppendAtDepth)
  - [x] Define `InsertPosition` enum (Append, AtIndex, Before, After)
  - [x] Define `TaskCreationRequest` struct with all task fields
  - [x] Implement `TaskCreationRequestBuilder` with fluent API
  - [x] Define `TaskCreationResult` struct (created task, file path, line number)
- [x] Create `creation_errors.rs` module in lash-types
  - [x] Define `TaskCreationError` enum with all validation error variants:
    - [x] EmptyTitle, TitleTooLong
    - [x] FileNotFound, FileNotWritable, FileParseFailed
    - [x] ParentNotFound, DepthLimitExceeded
    - [x] DuplicateId, InvalidIdFormat
    - [x] InvalidLabel, InvalidEstimate
    - [x] DependencyNotFound, WouldCreateCycle
    - [x] InvalidPosition
  - [x] Implement `message()` method for user-friendly messages
  - [x] Implement `help()` method for fix suggestions
  - [x] Implement `to_diagnostic()` for consistent error display
  - [x] Implement `error_code()` for stable error identifiers
- [x] Export new modules from lash-types lib.rs
- [x] Add comprehensive unit tests for builders and error formatting

### Success Criteria

- [x] All creation types are defined with proper documentation
- [x] Builder pattern provides ergonomic task creation API
- [x] Error types provide both human and machine-readable output
- [x] All tests pass

### Tests

- [x] Unit: TaskCreationRequestBuilder fluent API
- [x] Unit: TaskCreationError message formatting
- [x] Unit: Error to diagnostic conversion
- [x] Unit: Serialization/deserialization of request types

---

## Task 2: Validation Pipeline ✅ COMPLETE

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 1, tasks.linter.md

### Description

Implement the shared validation layer in `lash-core` that validates task creation requests for both CLI and TUI interfaces.

### Files Created/Modified

- `crates/lash-core/src/creation/validation.rs` - TaskValidator, ValidationContext

### Subtasks

- [x] Create `creation/` module directory in lash-core
- [x] Implement `validation.rs`
  - [x] Define `ValidationContext` struct (config, resolved file, parent task, computed depth)
  - [x] Implement `TaskValidator` struct
    - [x] `validate()` - main entry point, collects all errors
    - [x] `validate_title()` - non-empty, length limits (max 200 chars)
    - [x] `validate_id()` - format check, uniqueness within file
    - [x] `validate_label()` - alphanumeric with dashes only
    - [x] `validate_estimate()` - format check (e.g., 1h, 2d, 3w, 30m)
    - [x] `validate_owner()` - format validation
    - [x] `resolve_parent()` - parent existence, depth computation
    - [x] `resolve_file_target()` - file path resolution
  - [x] Support collecting multiple errors (don't stop at first)
  - [x] Return `ValidationResult<ValidationContext>` on success
- [x] Implement dependency validation
  - [x] `validate_dependencies()` - check targets exist
  - [x] Integration with `CycleDetector` from lash-core
  - [x] `dependency_target_exists()` helper
- [x] Add validation configuration
  - [x] Max title length (default: 200)
  - [x] Max depth (from LashConfig)
  - [x] ID format regex pattern
- [x] Write comprehensive tests
  - [x] Valid requests pass validation
  - [x] Each error type is triggered correctly
  - [x] Multiple errors collected in single validation

### Success Criteria

- [x] Validation catches all invalid inputs
- [x] Error messages are clear and actionable
- [x] Validation is fast (<10ms for typical requests)
- [x] All error types have test coverage

### Tests

- [x] Unit: Title validation (empty, too long, valid)
- [x] Unit: ID validation (format, uniqueness)
- [x] Unit: Label validation (format)
- [x] Unit: Estimate validation (formats)
- [x] Unit: Parent resolution (exists, not found, depth)
- [x] Unit: File target resolution
- [x] Unit: Dependency cycle detection
- [x] Integration: Full validation pipeline

---

## Task 3: Placement Resolution ✅ COMPLETE

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 1, Task 2

### Description

Implement logic to determine where to insert new tasks within existing Markdown files.

### Files Created/Modified

- `crates/lash-core/src/creation/placement.rs` - PlacementResolver

### Subtasks

- [x] Implement `placement.rs` in lash-core/creation/
  - [x] Define `PlacementInfo` struct (line_number, order_index, indent_level)
  - [x] Implement `PlacementResolver::resolve()` entry point
  - [x] Implement `resolve_append()` for parent/no-parent cases
  - [x] Implement `resolve_at_index()` for specific position
  - [x] Implement `resolve_before()` for inserting before task
  - [x] Implement `resolve_after()` for inserting after task (including descendants)
- [x] Implement helper methods
  - [x] `find_end_of_tasks_section()` - locate ## Tasks section end
  - [x] `find_insertion_after_parent()` - after parent's last child
  - [x] `find_task_line()` - locate task's line number
  - [x] `find_end_of_task_subtree()` - after all descendants
- [x] Handle edge cases
  - [x] Empty file (create new ## Tasks section)
  - [x] No ## Tasks section exists
  - [x] Parent has no existing children
  - [x] Parent is last task in file
- [x] Add source location tracking during parsing (if not present)
- [x] Write tests for all placement scenarios

### Success Criteria

- [x] Tasks inserted at correct positions in all cases
- [x] Indentation matches parent/sibling levels
- [x] Order indices are correct among siblings
- [x] Edge cases handled gracefully

### Tests

- [x] Unit: Append to empty file
- [x] Unit: Append as top-level task
- [x] Unit: Append as child of parent
- [x] Unit: Insert before specific task
- [x] Unit: Insert after task with children
- [x] Integration: Complex file with multiple levels

---

## Task 4: Markdown Emitter ✅ COMPLETE

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 3

### Description

Implement Markdown generation and file writing for new tasks.

### Files Created/Modified

- `crates/lash-core/src/creation/emitter.rs` - MarkdownEmitter

### Subtasks

- [x] Implement `emitter.rs` in lash-core/creation/
  - [x] Implement `MarkdownEmitter::emit()` entry point
  - [x] Implement `insert_into_existing()` for existing files
  - [x] Implement `create_new_file()` for new task files
  - [x] Implement `format_task_line()` for proper markdown output
- [x] Task line formatting
  - [x] Correct indentation based on depth
  - [x] Status checkbox ([ ], [x], [-], [!])
  - [x] Task title
  - [x] Inline labels (#label1 #label2)
  - [x] Inline metadata block [@id: x, @owner: y]
- [x] Multi-line annotation support
  - [x] @depends-on on separate indented line
  - [x] @agent-note on separate indented line
  - [x] @estimate, @owner inline or separate
- [x] File creation for new files
  - [x] Generate proper header (# Title from filename or user input)
  - [x] Generate file-level metadata block (@id, @labels, @owner)
  - [x] Create optional ## Description section
  - [x] Create ## Tasks section
  - [x] Insert the new task as first task
  - [x] Ensure parent directories exist (create recursively)
  - [x] Validate file path (no special characters, .md extension)
- [x] Safe file writing
  - [x] Create backup before modification (optional)
  - [x] Atomic write where possible
  - [x] Handle write errors gracefully

### Success Criteria

- [x] Generated markdown passes `lash lint`
- [x] Existing file formatting is preserved
- [x] New files follow project conventions
- [x] No data loss on write errors

### Tests

- [x] Unit: Task line formatting (all status types)
- [x] Unit: Inline labels formatting
- [x] Unit: Metadata block formatting
- [x] Unit: Multi-line annotations
- [x] Integration: Insert into existing file
- [x] Integration: Create new file
- [x] Integration: Round-trip (create, parse, verify)

---

## Task 5: Task Creation Service ✅ COMPLETE

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 2, Task 3, Task 4

### Description

Implement the orchestration service that ties together validation, placement, and emission.

### Files Created/Modified

- `crates/lash-core/src/creation/service.rs` - TaskCreationService
- `crates/lash-core/src/creation/mod.rs` - Module exports
- `crates/lash-core/src/lib.rs` - pub mod creation

### Subtasks

- [x] Implement `service.rs` in lash-core/creation/
  - [x] Define `TaskCreationService` struct
  - [x] Implement `create_task()` main entry point
    - [x] Step 1: Load target file (if exists)
    - [x] Step 2: Validate request
    - [x] Step 3: Resolve placement
    - [x] Step 4: Build task from request
    - [x] Step 5: Emit to markdown
    - [x] Step 6: Return result
  - [x] Implement `load_target_file()` helper
  - [x] Implement `build_task()` from request and context
  - [x] Implement `generate_id()` for auto-generated IDs (from title slug)
  - [x] Implement `identify_new_labels()` for reporting
- [x] Add database update support (optional)
  - [x] Insert task record after file write
  - [x] Insert label records for new labels
  - [x] Update file record (hash, mtime)
- [x] Create module exports
  - [x] `mod.rs` with pub use statements
  - [x] Export from lash-core lib.rs
- [x] Integration with existing parser
  - [x] Reuse `parse_file()` for loading
  - [x] Ensure consistency with parser output

### Success Criteria

- [x] End-to-end task creation works
- [x] Database stays in sync (if enabled)
- [x] Clear error reporting at each step
- [x] Service is reusable by CLI and TUI

### Tests

- [x] Unit: Task building from request
- [x] Unit: ID generation from title
- [x] Integration: Create task in existing file
- [x] Integration: Create task in new file
- [x] Integration: Create nested task
- [x] Integration: Verify database update

---

## Task 6: CLI `lash add` Command ✅ COMPLETE

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 5, tasks.cli-framework.md

### Description

Implement the `lash add` CLI command for creating tasks from the command line.

### Files Created/Modified

- `crates/lash-cli/src/commands/add.rs` - Add command implementation
- `crates/lash-cli/src/commands/mod.rs` - Export add command
- `crates/lash-cli/src/cli.rs` - Commands enum
- `crates/lash-cli/src/main.rs` - Command routing

### Subtasks

- [x] Define `AddArgs` struct with clap
  - [x] `title` - required positional argument
  - [x] `--file / -f` - target file path (creates new file if doesn't exist)
  - [x] `--file-title` - title for new file header (defaults to filename, only used when creating)
  - [x] `--file-description` - description for new file's ## Description section (only used when creating)
  - [x] `--parent / -p` - parent task ID
  - [x] `--after` - insert after task ID
  - [x] `--before` - insert before task ID
  - [x] `--label / -l` - labels (comma-separated, repeatable)
  - [x] `--owner / -o` - task owner
  - [x] `--estimate / -e` - time estimate
  - [x] `--status` - initial status (enum: open, done, waived, blocked)
  - [x] `--id` - explicit task ID
  - [x] `--depends-on / -d` - dependencies (comma-separated, repeatable)
  - [x] `--agent-note` - agent note text
  - [x] `--edit` - open editor for extended description
  - [x] `--format` - output format (text, json)
  - [x] `--dry-run` - validate without creating
  - [x] `--interactive / -i` - interactive mode
- [x] Implement `execute()` function
  - [x] Find project root
  - [x] Load config
  - [x] Handle interactive mode
  - [x] Build TaskCreationRequest
  - [x] Call TaskCreationService
  - [x] Format and display result
- [x] Implement interactive mode (using `dialoguer` crate)
  - [x] `prompt_for_missing_fields()` function
  - [x] File selection from discovered files
  - [x] Parent task selection from file tasks
  - [x] Label multi-select from existing labels
  - [x] Owner input with suggestions
- [x] Implement output formatting
  - [x] `output_success()` - text and JSON formats
  - [x] `output_errors()` - clear error display with help text
- [x] Implement dry-run mode
  - [x] Validate without writing
  - [x] Show what would be created
- [x] Register command in CLI
  - [x] Add to `Commands` enum in cli.rs
  - [x] Route in main.rs

### Success Criteria

- [x] All flags work as documented
- [x] Interactive mode guides users through creation
- [x] Dry-run mode is useful for validation
- [x] JSON output is parseable
- [x] Exit codes follow convention (0=success, 1=validation error)

### Tests

- [x] Unit: Argument parsing
- [x] Unit: Request building from args
- [x] Integration: Create simple task
- [x] Integration: Create task with all options
- [x] Integration: Create task in new file (file doesn't exist)
- [x] Integration: New file with custom title and description
- [x] Integration: Interactive mode (manual)
- [x] Integration: Dry-run mode
- [x] Integration: JSON output format
- [x] Integration: Error cases

---

## Task 7: TUI Form Components ✅ COMPLETE

**Priority:** HIGH
**Effort:** 4-5 days
**Depends on:** tasks.tui.md

### Description

Implement reusable form components for the TUI task creation modal.

### Files Created/Modified

- `crates/lash-tui/src/components/mod.rs` - Component module declaration
- `crates/lash-tui/src/components/text_input.rs` - TextInput component
- `crates/lash-tui/src/components/chip_input.rs` - ChipInput component (labels)
- `crates/lash-tui/src/components/radio_select.rs` - RadioSelect component (status)
- `crates/lash-tui/src/components/tree_select.rs` - TreeSelect component (parent)
- `crates/lash-tui/src/components/multi_select.rs` - MultiSelect component (dependencies)
- `crates/lash-tui/src/components/text_area.rs` - TextArea component (agent note)

### Subtasks

- [x] Create `components/` module in lash-tui
- [x] Implement `TextInputState` component
  - [x] `value: String` - current input
  - [x] `cursor_position: usize` - cursor location
  - [x] `placeholder: String` - hint text
  - [x] `required: bool` - validation flag
  - [x] `max_length: usize` - limit (0 = unlimited)
  - [x] `suggestions: Vec<String>` - autocomplete options
  - [x] `selected_suggestion: Option<usize>`
  - [x] `show_suggestions: bool`
  - [x] Input methods: `input_char()`, `backspace()`, `delete()`
  - [x] Navigation: `cursor_left()`, `cursor_right()`, `home()`, `end()`
  - [x] Autocomplete: `next_suggestion()`, `prev_suggestion()`, `accept_suggestion()`
- [x] Implement `ChipInputState` component (for labels)
  - [x] `chips: Vec<String>` - committed values
  - [x] `input: String` - current partial input
  - [x] `focused_chip: Option<usize>` - for deletion
  - [x] `suggestions: Vec<String>` - autocomplete
  - [x] Methods: `add_chip()`, `remove_chip()`, `focus_chip()`
  - [x] Input handling for comma/enter to commit
- [x] Implement `RadioSelectState<T>` component (for status)
  - [x] `options: Vec<RadioOption<T>>` - available choices
  - [x] `selected_index: usize` - current selection
  - [x] Methods: `select_next()`, `select_prev()`, `select_by_key()`
- [x] Implement `TreeSelectState` component (for parent task)
  - [x] `input: String` - filter text
  - [x] `filtered_tasks: Vec<TreeSelectItem>` - filtered list
  - [x] `selected_index: usize` - highlighted item
  - [x] `selected_parent: Option<TaskRecord>` - committed selection
  - [x] `is_expanded: bool` - dropdown state
  - [x] Methods: `filter()`, `select()`, `expand()`, `collapse()`
- [x] Implement `MultiSelectState` component (for dependencies)
  - [x] `input: String` - search filter
  - [x] `all_options: Vec<DependencyOption>` - all available
  - [x] `filtered_indices: Vec<usize>` - visible options
  - [x] `selected_indices: HashSet<usize>` - checked items
  - [x] Methods: `toggle_selection()`, `filter()`, `get_selected()`
- [x] Implement `TextAreaState` component (for agent note)
  - [x] `lines: Vec<String>` - multi-line content
  - [x] `cursor_row: usize`, `cursor_col: usize`
  - [x] `scroll_offset: usize` - for long content
  - [x] Methods: `input_char()`, `newline()`, `backspace()`
- [x] Write rendering functions for each component
  - [x] `render_text_input()` - with focus highlight, error display
  - [x] `render_chip_input()` - chips as tags, input area
  - [x] `render_radio_select()` - horizontal options
  - [x] `render_tree_select()` - dropdown with hierarchy
  - [x] `render_multi_select()` - checkboxes in list
  - [x] `render_text_area()` - multi-line with scroll

### Success Criteria

- [x] All components handle keyboard input correctly
- [x] Components are visually clear and consistent
- [x] Focus states are obvious
- [x] Components are reusable for other forms

### Tests

- [x] Unit: TextInputState cursor movement
- [x] Unit: ChipInputState chip management
- [x] Unit: RadioSelectState selection cycling
- [x] Unit: TreeSelectState filtering
- [x] Unit: MultiSelectState toggle behavior
- [x] Manual: Visual inspection of all components

---

## Task 8: TUI Task Creation Modal ✅ COMPLETE

**Priority:** HIGH
**Effort:** 3-4 days
**Depends on:** Task 5, Task 7

### Description

Implement the task creation modal for the TUI using the form components.

### Files Created/Modified

- `crates/lash-tui/src/ui/task_creation_modal.rs` - Task creation modal rendering
- `crates/lash-tui/src/state.rs` - TaskCreationModalState
- `crates/lash-tui/src/event.rs` - AppEvent variants
- `crates/lash-tui/src/app.rs` - TuiApp integration

### Subtasks

- [x] Define `TaskCreationModalState` in state.rs
  - [x] `focused_field: TaskFormField` - current focus
  - [x] `mode: TaskCreationMode` - AddToExisting or CreateNewFile
  - [x] `title: TextInputState`
  - [x] `file_selector: Option<FileSelectState>` - for choosing/creating file
  - [x] `new_file_path: TextInputState` - path for new file
  - [x] `new_file_title: TextInputState` - title for new file header
  - [x] `new_file_description: TextAreaState` - description for new file
  - [x] `parent_selector: Option<TreeSelectState>`
  - [x] `labels: ChipInputState`
  - [x] `status: RadioSelectState<TaskStatus>`
  - [x] `owner: TextInputState`
  - [x] `estimate: TextInputState`
  - [x] `dependencies: MultiSelectState`
  - [x] `agent_note: TextAreaState`
  - [x] `errors: HashMap<TaskFormField, String>` - validation errors
  - [x] `show_preview: bool` - markdown preview toggle
  - [x] `target_file: Option<FileRecord>` - context (None for new file)
- [x] Define `TaskFormField` enum for navigation
- [x] Define `TaskCreationMode` enum (AddToExisting, CreateNewFile)
- [x] Implement modal state methods
  - [x] `open_task_creation_modal()` - initialize with context
  - [x] `open_new_file_modal()` - initialize for new file creation
  - [x] `close_task_creation_modal()` - cleanup
  - [x] `toggle_mode()` - switch between add/create modes
  - [x] `next_field()`, `prev_field()` - Tab navigation
  - [x] `to_request()` - build TaskCreationRequest
  - [x] `validate_form()` - real-time validation
  - [x] `can_submit()` - check for blocking errors
- [x] Create `task_creation_modal.rs` in ui/
  - [x] `render()` - main modal rendering
  - [x] Centered popup layout (70% width, 80% height)
  - [x] Field layout with proper spacing
  - [x] Error display inline below fields
  - [x] Action bar with keyboard hints
- [x] Implement markdown preview panel
  - [x] `generate_markdown_preview()` - format task as markdown
  - [x] Toggle with Ctrl+P
  - [x] Collapsible to save space
- [x] Implement responsive layout
  - [x] Full form for large terminals
  - [x] Compact scrollable form for small terminals (<80 cols or <30 rows)
- [x] Add new AppEvent variants in event.rs
  - [x] `OpenTaskCreation`, `CloseTaskCreation`, `SubmitTaskCreation`
  - [x] `TaskFormNextField`, `TaskFormPrevField`
  - [x] `TaskFormTogglePreview`
  - [x] `TaskFormExpandDropdown`, `TaskFormCollapseDropdown`
  - [x] `TaskFormToggleSelection`
- [x] Implement `poll_task_creation_event()` in event.rs
  - [x] Handle all form-specific key events
  - [x] Global: Esc (close), Ctrl+S/Ctrl+Enter (submit), Tab/Shift+Tab (navigate)
  - [x] Text: char input, backspace, delete, cursor movement
  - [x] Selection: Up/Down, Enter, Space
- [x] Integrate with TuiApp in app.rs
  - [x] Add keybinding to open modal (`a` or `n`)
  - [x] Route events to modal when open
  - [x] `handle_submit_task_creation()` - call service, show result
  - [x] Refresh task list after creation
  - [x] Show success/error in status bar

### Success Criteria

- [x] Modal opens with proper context from current selection
- [x] All fields are navigable and functional
- [x] Real-time validation shows errors inline
- [x] Markdown preview is accurate
- [x] Submit creates task and closes modal
- [x] Cancel closes without changes
- [x] Can switch between "Add to file" and "Create new file" modes
- [x] New file mode shows file path, title, and description fields
- [x] New file is created with proper structure on submit

### Tests

- [x] Unit: Form state navigation
- [x] Unit: Request building from form
- [x] Unit: Markdown preview generation
- [x] Unit: Mode toggling (AddToExisting <-> CreateNewFile)
- [x] Integration: Open modal, fill form, submit
- [x] Integration: Validation error display
- [x] Integration: Create new file via modal
- [x] Integration: New file preview shows full file structure
- [x] Manual: Full form interaction testing

---

## Task 9: Keyboard Navigation & Help ✅ COMPLETE

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 8

### Description

Implement comprehensive keyboard navigation and help for the task creation form.

### Files Created/Modified

- `crates/lash-tui/src/event.rs` - Key event handling
- `crates/lash-tui/src/ui/help.rs` - render_task_creation_help()

### Subtasks

- [x] Define keyboard navigation scheme
  - [x] Tab / Shift+Tab: Move between fields
  - [x] Up/Down or Ctrl+P/N: Navigate within dropdowns/lists
  - [x] Enter: Select in dropdown, submit form (on last field)
  - [x] Space: Toggle selection (multi-select, radio)
  - [x] Escape: Close dropdown, or close modal
  - [x] Ctrl+S or Ctrl+Enter: Submit form
  - [x] Ctrl+P: Toggle markdown preview
  - [x] Ctrl+U: Clear current field
  - [x] F1 or ?: Show help overlay
- [x] Implement focus flow (circular)
  ```
  Title -> Parent -> Labels -> Status -> Owner -> Estimate -> Dependencies -> AgentNote -> Title
  ```
- [x] Add visual focus indicators
  - [x] Border color change for focused field
  - [x] Cursor display in text inputs
  - [x] Highlight in selection lists
- [x] Create help overlay for form
  - [x] List all keyboard shortcuts
  - [x] Field-specific hints
  - [x] Show when F1/? pressed
- [x] Add field-specific shortcuts
  - [x] Status field: O/D/W/B for Open/Done/Waived/Blocked
  - [x] Labels field: Enter or comma to add chip
  - [x] Backspace (empty input): Focus previous chip

### Success Criteria

- [x] All actions achievable via keyboard only
- [x] Navigation is intuitive and discoverable
- [x] Help overlay is comprehensive
- [x] Focus indicators are clear

### Tests

- [x] Unit: Focus flow cycling
- [x] Manual: Navigate entire form with keyboard
- [x] Manual: Help overlay accuracy

---

## Task 10: Validation UX & Error Display ✅ COMPLETE

**Priority:** MEDIUM
**Effort:** 2 days
**Depends on:** Task 2, Task 8

### Description

Implement real-time validation with clear, inline error display in both CLI and TUI.

### Files Created/Modified

- `crates/lash-tui/src/ui/task_creation_modal.rs` - Inline error display
- `crates/lash-cli/src/commands/add.rs` - CLI error formatting
- `docs/error-codes.md` - E_CREATE_* error codes

### Subtasks

- [x] Implement real-time validation for TUI
  - [x] Debounced validation (100ms delay)
  - [x] Validate on each field change
  - [x] Update `errors` map in modal state
- [x] Implement inline error display
  - [x] Show error icon and message below field
  - [x] Use red/yellow colors for error/warning
  - [x] Clear error when field becomes valid
- [x] Implement blocking vs non-blocking errors
  - [x] Blocking: Empty title, invalid format
  - [x] Non-blocking: Title very long (warning)
  - [x] Submit blocked only by blocking errors
- [x] CLI error formatting
  - [x] Group errors by field
  - [x] Show error code, message, and help
  - [x] Colored output (respecting --no-color)
  - [x] JSON error output for agents
- [x] Add validation feedback
  - [x] Green checkmark for valid required fields
  - [x] Character count for title
  - [x] Format hints for estimate field

### Success Criteria

- [x] Users immediately see what's wrong
- [x] Error messages suggest fixes
- [x] Validation doesn't lag UI
- [x] All error codes are documented

### Tests

- [x] Unit: Debounced validation timing
- [x] Unit: Error/warning classification
- [x] Integration: All error types display correctly
- [x] Manual: Real-time validation responsiveness

---

## Summary

**Total Tasks Archived:** 10 of 12
**Completion Date:** 2025-12-09
**Main Commit:** `0a5bb3e Implement task creation feature with TUI modal and CLI support`

### Files Created (29 total)
- Core types: `lash-types/src/creation.rs`, `lash-types/src/creation_errors.rs`
- Validation: `lash-core/src/creation/validation.rs`
- Placement: `lash-core/src/creation/placement.rs`
- Emission: `lash-core/src/creation/emitter.rs`
- Service: `lash-core/src/creation/service.rs`, `lash-core/src/creation/mod.rs`
- CLI: `lash-cli/src/commands/add.rs`
- TUI Components: `text_input.rs`, `chip_input.rs`, `radio_select.rs`, `tree_select.rs`, `multi_select.rs`, `text_area.rs`
- TUI Modal: `task_creation_modal.rs`

### Remaining Work (Tasks 11-12)
See `tasks/tasks.task-creation-ui.md` for remaining tasks focused on autocomplete enhancements and documentation.
