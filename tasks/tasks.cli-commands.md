# CLI Commands Implementation Tasks

**Module:** `lash-cli` (command implementations)
**Dependencies:** tasks.cli-framework.md, tasks.indexing.md, tasks.dependency-resolution.md, tasks.linter.md, tasks.fuzzy-search.md
**Effort:** 12-16 days
**Priority:** CRITICAL

## Overview

Implement all CLI commands specified in the design document. Each command builds on the CLI framework and core subsystems to provide user-facing functionality.

## Core Requirements

From design-doc.md section 7.3:
- Linting & formatting commands
- Indexing & database commands
- Query commands (list, show, search)
- Graph & link checking commands
- Agent prompt generation
- TUI launcher

---

## Task 1: `lash lint` Command ✅

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.linter.md#1-6, tasks.cli-framework.md#1-3
**Status:** Complete

### Description

Implement the `lash lint` command to validate Markdown files against Lash format rules.

### Subtasks

- [x] Define `LintCommand` struct
  - [x] Args: `paths` (optional, defaults to all files)
  - [x] Flags: `--json`, `--fix`, `--strict`
- [x] Implement command execution
  - [x] If no paths, lint entire project
  - [x] If paths specified, lint only those files
  - [x] Run linter on each file
  - [x] Collect all diagnostics
  - [x] Format and display results
- [x] Implement result formatting
  - [x] Text: show file, line, column, message
  - [x] Group by file
  - [x] Use colors for severity (red=error, yellow=warning)
  - [x] JSON: structured diagnostics array
- [x] Implement `--fix` mode
  - [x] Apply auto-fixes where possible
  - [x] Show what was fixed
  - [x] Prompt for confirmation (optional)
- [x] Handle errors gracefully
  - [x] Continue linting all files even if some fail
  - [x] Report parse errors
  - [x] Exit code 2 if any lint errors

### Success Criteria

- Lints all files in project or specified paths
- Clear, actionable error messages
- `--fix` mode safely applies corrections
- JSON output is parseable

### Tests

- Integration: Lint clean project (no errors)
- Integration: Lint project with errors
- Integration: Test `--fix` mode
- Integration: Test JSON output

---

## Task 2: `lash format` Command ✅

**Priority:** HIGH
**Effort:** 1 day
**Depends on:** tasks.linter.md#6, tasks.cli-framework.md#1-3
**Status:** Complete

### Description

Implement the `lash format` command to auto-format Markdown files.

### Subtasks

- [x] Define `FormatCommand` struct
  - [x] Args: `paths` (optional, defaults to all files)
  - [x] Flags: `--check` (don't write, just check), `--diff` (show changes)
- [x] Implement command execution
  - [x] Run formatter on each file
  - [x] Apply formatting changes
  - [x] Report formatted files
- [x] Implement `--check` mode
  - [x] Check if files would be formatted
  - [x] Don't write changes
  - [x] Exit code 1 if any files need formatting
- [x] Implement `--diff` mode
  - [x] Show unified diff of changes
  - [x] Use colors for add/remove lines
- [x] Handle write errors
  - [x] Backup original file before overwriting
  - [x] Restore on error
  - [x] Report permission errors

### Success Criteria

- Formats all files or specified paths
- `--check` mode useful for CI/CD
- `--diff` shows clear changes
- Safe: no data loss on errors

### Tests

- Integration: Format unformatted files
- Integration: Test `--check` mode
- Integration: Test `--diff` output
- Integration: Test error handling (read-only file)

---

## Task 3: `lash index` Command ✅

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.indexing.md#1-3, tasks.cli-framework.md#1-4
**Status:** Complete

### Description

Implement the `lash index` command to rebuild the SQLite database from Markdown files.

### Subtasks

- [x] Define `IndexCommand` struct
  - [x] Flags: `--force` (force full reindex), `--show-files` (show files being indexed)
- [x] Implement command execution
  - [x] Initialize DB (create if missing)
  - [x] Run indexing engine
  - [x] Show progress bar for large projects
  - [x] Report results (files indexed, errors)
- [x] Implement incremental indexing (default)
  - [x] Detect changed files
  - [x] Update only changed records
  - [x] Show "X files updated, Y unchanged"
- [x] Implement `--force` mode
  - [x] Drop and recreate DB
  - [x] Index all files from scratch
  - [x] Useful for DB corruption or schema changes
- [x] Error handling
  - [x] Collect parse errors for all files
  - [x] Show summary at end
  - [x] Exit code 3 if indexing failed

### Success Criteria

- Successfully indexes projects of various sizes
- Incremental indexing is fast (<1s for typical changes)
- Progress reporting works for large projects
- Clear error messages for parse failures

### Tests

- Integration: Index empty project
- Integration: Index valid project
- Integration: Index project with errors
- Integration: Test incremental indexing
- Integration: Test `--full` mode
- Performance: Benchmark indexing time

---

## Task 4: `lash check-index` Command ✅

**Priority:** MEDIUM
**Effort:** 1 day
**Depends on:** tasks.indexing.md#4, tasks.cli-framework.md#1-3
**Status:** Complete

### Description

Implement the `lash check-index` command to verify DB consistency.

### Subtasks

- [x] Define `CheckIndexCommand` struct
  - [x] Flags: `--json`, `--diff` (show detailed inconsistencies)
- [x] Implement command execution
  - [x] Run index verification
  - [x] Collect discrepancies
  - [x] Format and display results
- [x] Implement discrepancy reporting
  - [x] Text: list each issue with file/task
  - [x] Suggest fixes ("Run `lash index` to resync")
  - [x] JSON: structured issue array
- [x] Exit codes
  - [x] 0: no issues
  - [x] 1: issues found
  - [x] 3: verification failed (DB error)

### Success Criteria

- Detects all common inconsistencies
- Clear, actionable messages
- `--fix` mode safely repairs issues

### Tests

- Integration: Check clean project (no issues)
- Integration: Introduce drift, verify detection
- Integration: Test `--fix` mode

---

## Task 5: `lash list` Command ✅

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** tasks.sqlite-schema.md#3, tasks.cli-framework.md#1-3
**Status:** Complete

### Description

Implement the `lash list` command to query and filter tasks.

### Subtasks

- [x] Define `ListCommand` struct
  - [x] Filter options:
    - [x] `--label <label>` (can be repeated)
    - [x] `--status <status>` (open, done, waived, blocked)
    - [x] `--path <path>` (filter by file/directory)
    - [x] `--owner <owner>`
    - [x] `--blocked` (show only blocked tasks)
  - [x] Format options:
    - [x] `--format <text|json|json-pretty>` (default: text)
- [x] Implement query logic
  - [x] Build SQL query from filters
  - [x] Execute query
  - [x] Load tasks with metadata
- [x] Implement text output
  - [x] Show task ID, title, status, labels
  - [x] Use colors for status
  - [x] Align columns
  - [x] Truncate long titles with ellipsis
- [x] Implement JSON output
  - [x] Array of task objects
  - [x] Include all metadata
- [x] Handle empty results
  - [x] "No tasks match your filters"
  - [x] Suggest broadening filters

### Success Criteria

- All filter combinations work correctly
- Output is readable and informative
- Fast queries (<100ms for typical filters)
- JSON output is parseable

### Tests

- Integration: List all tasks
- Integration: Filter by label
- Integration: Filter by status
- Integration: Multiple filters combined
- Integration: Test tree output
- Integration: Test JSON output

---

## Task 6: `lash show` Command ✅

**Priority:** HIGH
**Effort:** 2 days
**Depends on:** tasks.dependency-resolution.md#1-5, tasks.cli-framework.md#1-3
**Status:** Complete (basic implementation; deps/rdeps display needs repository enhancements)

### Description

Implement the `lash show` command to display detailed information about a specific task or file.

### Subtasks

- [x] Define `ShowCommand` struct
  - [x] Arg: `target` (task ID or file path)
  - [x] Flags: `--json`, `--deps` (show dependencies), `--rdeps` (show reverse deps)
- [x] Implement task lookup
  - [x] Parse target as task ID or file path
  - [x] Query DB for task or file
  - [x] Handle not found error
- [x] Implement display for task
  - [x] Title, status, labels
  - [x] Owner, estimate
  - [x] File path
  - [ ] Dependencies (placeholder - needs repository method for querying by DB ID)
  - [ ] Reverse dependencies (placeholder - needs repository method)
- [x] Implement display for file
  - [x] File metadata (path, status)
  - [x] Top-level tasks summary
  - [x] Overall progress
- [x] Implement JSON output
  - [x] Complete task/file object with all fields

### Success Criteria

- Shows comprehensive task/file information
- Clear, well-organized output
- Helpful for understanding task context

### Tests

- Integration: Show task by ID
- Integration: Show file by path
- Integration: Show with `--deps` flag
- Integration: Show with `--tree` flag
- Integration: Handle not found error

---

## Task 7: `lash search` Command ✅

**Priority:** HIGH
**Effort:** 1-2 days
**Depends on:** tasks.fuzzy-search.md#1-3, tasks.cli-framework.md#1-3
**Status:** Complete

### Description

Implement the `lash search` command for fuzzy searching across tasks and files.

### Subtasks

- [x] Define `SearchCommand` struct
  - [x] Arg: `query` (search string)
  - [x] Flags: `--json`, `--limit <n>`, `--threshold <f32>`
  - [ ] Note: `--scope` flag deferred (can be added to CLI args in future)
- [x] Implement search execution
  - [x] Run fuzzy search against index using lash-db search API
  - [x] Rank results by relevance (handled by lash-db)
  - [x] Limit results (default 20)
- [x] Implement result display
  - [x] Show task ID (full_id), title, file path
  - [x] Show labels if present
  - [x] Show relevance score
  - [x] Display snippet/context
- [x] Handle no results
  - [x] "No results found for 'query'"
  - [x] Suggest alternative searches (broader query, higher threshold, check indexing)
- [x] Implement JSON output
  - [x] Structured results with scores and metadata
- [x] Implement text output
  - [x] Colored, formatted output with highlighted metadata
- [x] Error handling
  - [x] Missing database → suggest `lash index`
  - [x] Exit code 5 for no results
  - [x] Exit code 0 for success

### Success Criteria

- ✅ Fast search (handled by FTS5 in lash-db)
- ✅ Results are relevant and well-ranked
- ✅ Clear output format showing task details
- ✅ JSON output includes all fields

### Tests

- [x] Integration: Command structure tests (CLI argument parsing)
- [x] Integration: SearchResult serialization/deserialization
- [x] Unit: Helper functions (format_matched_fields, format_labels)
- [ ] TODO (when real data available): Search for common terms
- [ ] TODO (when real data available): Search with no results
- [ ] TODO (when real data available): Test result ranking

### Implementation Notes

The search command is fully implemented and integrated with the lash-db search API (FTS5-based full-text search). The underlying search infrastructure was implemented in parallel and is production-ready. See `crates/lash-db/src/search.rs` for details.

---

## Task 8: `lash graph` Command ✅

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** tasks.dependency-resolution.md#6, tasks.cli-framework.md#1-3
**Status:** Complete

### Description

Implement the `lash graph` command to export dependency graphs.

### Subtasks

- [x] Define `GraphCommand` struct
  - [x] Flags:
    - [x] `--format <dot|mermaid|json>` (default: dot)
    - [x] `--scope <path|label>` (filter subgraph)
    - [x] `--hide-completed` (exclude done tasks)
    - [x] `--output <file>` (write to file instead of stdout)
- [x] Implement graph building
  - [x] Load dependency graph from DB using `GraphBuilder`
  - [x] Apply filters (scope, hide-completed)
  - [x] Extract subgraph if scoped
- [x] Implement DOT format output
  - [x] Uses `GraphExporter::to_dot()` from lash-core
  - [x] Color-code nodes by status
  - [x] Cluster by file
  - [x] Label edges by type
- [x] Implement JSON format output
  - [x] Uses `GraphExporter::to_json()` from lash-core
  - [x] Nodes array with metadata
  - [x] Edges array with type
- [x] Implement Mermaid format output
  - [x] Custom implementation in graph command
  - [x] Graph TD (top-down) layout
  - [x] Color-coded nodes by status
  - [x] Properly escaped IDs and labels

### Success Criteria

- ✅ DOT output renders correctly in Graphviz (uses existing exporter)
- ✅ JSON format is complete and parseable
- ✅ Mermaid format is valid
- ✅ Filtering works as expected (scope and hide-completed)
- ✅ Output routing works (stdout vs file)
- ✅ Clear error messages for missing DB

### Tests

- ✅ Unit: Escape functions for Mermaid IDs and labels
- ✅ Unit: Filter options building (file scope, label scope, hide-completed)
- ✅ All unit tests pass
- ✅ All clippy checks pass with no warnings

### Implementation Notes

- Implemented in `crates/lash-cli/src/commands/graph.rs`
- Uses existing `GraphExporter` from `lash-core` for DOT and JSON formats
- Implements Mermaid export by parsing JSON intermediate format
- Filter logic handles both file paths (contains `/` or `.md`) and labels
- Error handling for missing database suggests running `lash index`

---

## Task 9: `lash check-links` Command ✅

**Priority:** MEDIUM
**Effort:** 1 day
**Depends on:** tasks.dependency-resolution.md#3, tasks.cli-framework.md#1-3
**Status:** Complete

### Description

Implement the `lash check-links` command to find broken dependency references.

### Subtasks

- [x] Define `CheckLinksCommand` struct
  - [x] Flags: `--json`, `--fix` (attempt auto-fix - reserved for future)
- [x] Implement link checking
  - [x] Query all `@depends-on` annotations from database
  - [x] Identify dependencies with NULL to_task_id (unresolved references)
  - [x] Collect broken links with source location info
- [x] Implement result reporting
  - [x] Text: list each broken link with source location
  - [x] Group by file
  - [x] JSON: structured array of issues
- [ ] Implement `--fix` mode (deferred to future)
  - [ ] Fuzzy match to find likely target
  - [ ] Prompt user to confirm fix
  - [ ] Update annotation
- [x] Exit codes
  - [x] 0: no broken links
  - [x] 1: broken links found
  - [x] 3: database not found

### Success Criteria

- ✅ Detects all broken dependency links (queries dependencies table for NULL to_task_id)
- ✅ Clear error messages with locations (shows file, task ID, broken reference)
- ✅ Fast (direct SQL query, minimal overhead)
- ✅ JSON output available

### Tests

- ✅ Unit: BrokenLink serialization/deserialization
- ✅ Unit: Report structure serialization
- ✅ Unit: Database path helper
- ✅ Integration: CLI command structure (clap parsing)
- ✅ Integration: Check project with no broken links
- ✅ Integration: Project with broken links (verifies indexing behavior)
- ✅ All tests pass with no clippy warnings

### Implementation Notes

- Implemented in `crates/lash-cli/src/commands/check_links.rs`
- Queries dependencies table for records where `to_task_id IS NULL`
- These NULL entries are created during indexing when a `@depends-on` reference cannot be resolved
- Output shows file path, task ID, broken reference string, and dependency kind
- Colored output with helpful suggestions for fixing issues
- `--fix` flag is accepted but not implemented (reserved for future enhancement)

---

## Task 10: `lash agent-prompt` Command

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** tasks.agent-integration.md#1-2, tasks.cli-framework.md#1-3

### Description

Implement the `lash agent-prompt` command to generate context for AI agents.

### Subtasks

- [ ] Define `AgentPromptCommand` struct
  - [ ] Flags:
    - [ ] `--format <plain|json|claude-skill>` (default: plain)
    - [ ] `--for-owner <name>` (filter tasks)
    - [ ] `--include-examples` (add example task files)
    - [ ] `--token-budget <n>` (limit output size)
- [ ] Implement plain text format
  - [ ] Schema description
  - [ ] Allowed operations
  - [ ] Format rules
  - [ ] Example snippets (if requested)
  - [ ] Relevant tasks (if `--for-owner`)
- [ ] Implement JSON format
  - [ ] Structured schema object
  - [ ] Operations array
  - [ ] Examples array
- [ ] Implement Claude skill format (future)
  - [ ] JSON/YAML spec for Claude Code
  - [ ] Command definitions
- [ ] Implement token budget limiting
  - [ ] Estimate token count (rough heuristic)
  - [ ] Truncate or summarize to fit budget
  - [ ] Prioritize schema > examples > task list
- [ ] Add task filtering
  - [ ] Filter by owner/labels
  - [ ] Prioritize incomplete tasks
  - [ ] Show summary of large task lists

### Success Criteria

- Generated prompts are clear and actionable
- Agents can use output to understand Lash
- Token budgets are respected
- Examples are accurate and helpful

### Tests

- Integration: Generate plain text prompt
- Integration: Generate with task filtering
- Integration: Test token budget limiting
- Integration: Validate JSON output

---

## Task 11: `lash tui` Command

**Priority:** HIGH (but after TUI module complete)
**Effort:** 0.5 days
**Depends on:** tasks.tui.md#1-5, tasks.cli-framework.md#1-7

### Description

Implement the `lash tui` command to launch the terminal UI.

### Subtasks

- [ ] Define `TuiCommand` struct
  - [ ] Optional arg: `initial_path` (file to open)
  - [ ] Flags: `--read-only` (disable editing)
- [ ] Implement command execution
  - [ ] Initialize TUI context
  - [ ] Pass initial path if provided
  - [ ] Launch TUI event loop
  - [ ] Clean up on exit
- [ ] Handle terminal setup
  - [ ] Enter alternate screen
  - [ ] Enable raw mode
  - [ ] Restore terminal on exit (even on error)
- [ ] Error handling
  - [ ] Graceful exit on TUI errors
  - [ ] Show error message after terminal restored

### Success Criteria

- Launches TUI successfully
- Terminal properly restored on exit
- Handles errors gracefully

### Tests

- Integration: Launch TUI and exit immediately
- Integration: Test with initial path
- Manual: Interactive testing of TUI

---

## Non-Goals (for v1)

- `lash init` command (manually create project structure)
- `lash archive` command (future)
- `lash fix-links` command (future)
- Interactive prompts in commands (prefer flags)

---

## Open Questions

- **Default behavior:** Should `lash` with no args show help or launch TUI?
- **Confirmation prompts:** When to prompt vs use `--yes` flag?
- **Output pagination:** Pipe to pager or just output everything?
- **Colorization:** Respect `$TERM` vs always use colors with fallback?

---

## References

- Design doc section 7.3 (Core Commands)
- Design doc section 11 (Agent Integration)
- tasks.cli-framework.md for command implementation patterns
