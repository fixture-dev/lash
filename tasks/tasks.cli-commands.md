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

## Task 7: `lash search` Command

**Priority:** HIGH
**Effort:** 1-2 days
**Depends on:** tasks.fuzzy-search.md#1-3, tasks.cli-framework.md#1-3

### Description

Implement the `lash search` command for fuzzy searching across tasks and files.

### Subtasks

- [ ] Define `SearchCommand` struct
  - [ ] Arg: `query` (search string)
  - [ ] Flags: `--json`, `--limit <n>`, `--scope <path>`
- [ ] Implement search execution
  - [ ] Run fuzzy search against index
  - [ ] Rank results by relevance
  - [ ] Limit results (default 20)
- [ ] Implement result display
  - [ ] Show task ID, title, file path
  - [ ] Highlight matching terms
  - [ ] Show relevance score (optional)
  - [ ] Truncate and show context around match
- [ ] Implement `--scope` filtering
  - [ ] Search only within specified path
  - [ ] Combine with query
- [ ] Handle no results
  - [ ] "No results found for 'query'"
  - [ ] Suggest alternative searches

### Success Criteria

- Fast search (<200ms for typical queries)
- Results are relevant and well-ranked
- Highlighting makes matches clear
- JSON output includes scores

### Tests

- Integration: Search for common terms
- Integration: Search with no results
- Integration: Test `--scope` filter
- Integration: Test result ranking

---

## Task 8: `lash graph` Command

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** tasks.dependency-resolution.md#6, tasks.cli-framework.md#1-3

### Description

Implement the `lash graph` command to export dependency graphs.

### Subtasks

- [ ] Define `GraphCommand` struct
  - [ ] Flags:
    - [ ] `--format <dot|json|text>` (default: dot)
    - [ ] `--scope <path|label>` (filter subgraph)
    - [ ] `--hide-completed` (exclude done tasks)
    - [ ] `--output <file>` (write to file instead of stdout)
- [ ] Implement graph building
  - [ ] Load dependency graph from DB
  - [ ] Apply filters (scope, hide-completed)
  - [ ] Extract subgraph if scoped
- [ ] Implement DOT format output
  - [ ] Generate Graphviz DOT syntax
  - [ ] Color-code nodes by status
  - [ ] Cluster by file (optional)
  - [ ] Label edges by type
- [ ] Implement JSON format output
  - [ ] Nodes array with metadata
  - [ ] Edges array with type
- [ ] Implement text format output
  - [ ] ASCII tree or list format
  - [ ] Show dependency chains
- [ ] Add usage examples in help
  - [ ] `lash graph | dot -Tpng -o graph.png`

### Success Criteria

- DOT output renders correctly in Graphviz
- JSON format is complete and parseable
- Text format is readable
- Filtering works as expected

### Tests

- Integration: Export full graph to DOT
- Integration: Export to JSON
- Integration: Test scope filtering
- Manual: Render DOT file visually, inspect

---

## Task 9: `lash check-links` Command

**Priority:** MEDIUM
**Effort:** 1 day
**Depends on:** tasks.dependency-resolution.md#3, tasks.cli-framework.md#1-3

### Description

Implement the `lash check-links` command to find broken dependency references.

### Subtasks

- [ ] Define `CheckLinksCommand` struct
  - [ ] Flags: `--json`, `--fix` (attempt auto-fix)
- [ ] Implement link checking
  - [ ] Query all `@depends-on` annotations
  - [ ] Verify target tasks exist
  - [ ] Collect broken links
- [ ] Implement result reporting
  - [ ] Text: list each broken link with source location
  - [ ] Group by file
  - [ ] JSON: structured array of issues
- [ ] Implement `--fix` mode (optional, future)
  - [ ] Fuzzy match to find likely target
  - [ ] Prompt user to confirm fix
  - [ ] Update annotation
- [ ] Exit codes
  - [ ] 0: no broken links
  - [ ] 1: broken links found

### Success Criteria

- Detects all broken dependency links
- Clear error messages with locations
- Fast (<500ms for typical projects)

### Tests

- Integration: Check project with no broken links
- Integration: Introduce broken link, verify detection
- Integration: Test JSON output

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
