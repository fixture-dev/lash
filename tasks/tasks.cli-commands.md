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

## Task 1: `lash lint` Command

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.linter.md#1-6, tasks.cli-framework.md#1-3

### Description

Implement the `lash lint` command to validate Markdown files against Lash format rules.

### Subtasks

- [ ] Define `LintCommand` struct
  - [ ] Args: `paths` (optional, defaults to all files)
  - [ ] Flags: `--json`, `--fix`, `--strict`
- [ ] Implement command execution
  - [ ] If no paths, lint entire project
  - [ ] If paths specified, lint only those files
  - [ ] Run linter on each file
  - [ ] Collect all diagnostics
  - [ ] Format and display results
- [ ] Implement result formatting
  - [ ] Text: show file, line, column, message
  - [ ] Group by file
  - [ ] Use colors for severity (red=error, yellow=warning)
  - [ ] JSON: structured diagnostics array
- [ ] Implement `--fix` mode
  - [ ] Apply auto-fixes where possible
  - [ ] Show what was fixed
  - [ ] Prompt for confirmation (optional)
- [ ] Handle errors gracefully
  - [ ] Continue linting all files even if some fail
  - [ ] Report parse errors
  - [ ] Exit code 2 if any lint errors

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

## Task 2: `lash format` Command

**Priority:** HIGH
**Effort:** 1 day
**Depends on:** tasks.linter.md#6, tasks.cli-framework.md#1-3

### Description

Implement the `lash format` command to auto-format Markdown files.

### Subtasks

- [ ] Define `FormatCommand` struct
  - [ ] Args: `paths` (optional, defaults to all files)
  - [ ] Flags: `--check` (don't write, just check), `--diff` (show changes)
- [ ] Implement command execution
  - [ ] Run formatter on each file
  - [ ] Apply formatting changes
  - [ ] Report formatted files
- [ ] Implement `--check` mode
  - [ ] Check if files would be formatted
  - [ ] Don't write changes
  - [ ] Exit code 1 if any files need formatting
- [ ] Implement `--diff` mode
  - [ ] Show unified diff of changes
  - [ ] Use colors for add/remove lines
- [ ] Handle write errors
  - [ ] Backup original file before overwriting
  - [ ] Restore on error
  - [ ] Report permission errors

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

## Task 3: `lash index` Command

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.indexing.md#1-3, tasks.cli-framework.md#1-4

### Description

Implement the `lash index` command to rebuild the SQLite database from Markdown files.

### Subtasks

- [ ] Define `IndexCommand` struct
  - [ ] Flags: `--full` (force full reindex), `--verify` (check after indexing)
- [ ] Implement command execution
  - [ ] Initialize DB (create if missing)
  - [ ] Run indexing engine
  - [ ] Show progress bar for large projects
  - [ ] Report results (files indexed, errors)
- [ ] Implement incremental indexing (default)
  - [ ] Detect changed files
  - [ ] Update only changed records
  - [ ] Show "X files updated, Y unchanged"
- [ ] Implement `--full` mode
  - [ ] Drop and recreate DB
  - [ ] Index all files from scratch
  - [ ] Useful for DB corruption or schema changes
- [ ] Implement `--verify` mode
  - [ ] Run verification after indexing
  - [ ] Report any inconsistencies
- [ ] Error handling
  - [ ] Collect parse errors for all files
  - [ ] Show summary at end
  - [ ] Exit code 3 if indexing failed

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

## Task 4: `lash check-index` Command

**Priority:** MEDIUM
**Effort:** 1 day
**Depends on:** tasks.indexing.md#4, tasks.cli-framework.md#1-3

### Description

Implement the `lash check-index` command to verify DB consistency.

### Subtasks

- [ ] Define `CheckIndexCommand` struct
  - [ ] Flags: `--json`, `--fix` (auto-repair if possible)
- [ ] Implement command execution
  - [ ] Run index verification
  - [ ] Collect discrepancies
  - [ ] Format and display results
- [ ] Implement discrepancy reporting
  - [ ] Text: list each issue with file/task
  - [ ] Suggest fixes ("Run `lash index` to resync")
  - [ ] JSON: structured issue array
- [ ] Implement `--fix` mode
  - [ ] Attempt to repair common issues
  - [ ] Remove orphaned records
  - [ ] Re-index inconsistent files
  - [ ] Report what was fixed
- [ ] Exit codes
  - [ ] 0: no issues
  - [ ] 1: issues found
  - [ ] 3: verification failed (DB error)

### Success Criteria

- Detects all common inconsistencies
- Clear, actionable messages
- `--fix` mode safely repairs issues

### Tests

- Integration: Check clean project (no issues)
- Integration: Introduce drift, verify detection
- Integration: Test `--fix` mode

---

## Task 5: `lash list` Command

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** tasks.sqlite-schema.md#3, tasks.cli-framework.md#1-3

### Description

Implement the `lash list` command to query and filter tasks.

### Subtasks

- [ ] Define `ListCommand` struct
  - [ ] Filter options:
    - [ ] `--label <label>` (can be repeated)
    - [ ] `--status <status>` (open, done, waived, blocked)
    - [ ] `--path <path>` (filter by file/directory)
    - [ ] `--owner <owner>`
    - [ ] `--blocked` (show only blocked tasks)
    - [ ] `--ready` (show only ready-to-start tasks)
  - [ ] Format options:
    - [ ] `--format <text|json|ids>` (default: text)
    - [ ] `--tree` (show hierarchical structure)
  - [ ] Limit options:
    - [ ] `--limit <n>` (max results)
- [ ] Implement query logic
  - [ ] Build SQL query from filters
  - [ ] Execute query
  - [ ] Load tasks with metadata
- [ ] Implement text output
  - [ ] Show task ID, title, status, labels
  - [ ] Use colors for status
  - [ ] Align columns
  - [ ] Truncate long titles with ellipsis
- [ ] Implement tree output
  - [ ] Show parent-child relationships
  - [ ] Indent by depth
  - [ ] Show dependency indicators
- [ ] Implement JSON output
  - [ ] Array of task objects
  - [ ] Include all metadata
- [ ] Handle empty results
  - [ ] "No tasks match your filters"
  - [ ] Suggest broadening filters

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

## Task 6: `lash show` Command

**Priority:** HIGH
**Effort:** 2 days
**Depends on:** tasks.dependency-resolution.md#1-5, tasks.cli-framework.md#1-3

### Description

Implement the `lash show` command to display detailed information about a specific task or file.

### Subtasks

- [ ] Define `ShowCommand` struct
  - [ ] Arg: `target` (task ID or file path)
  - [ ] Flags: `--json`, `--deps` (show dependencies), `--tree` (show subtasks)
- [ ] Implement task lookup
  - [ ] Parse target as task ID or file path
  - [ ] Query DB for task or file
  - [ ] Handle not found error
- [ ] Implement display for task
  - [ ] Title, status, labels
  - [ ] Owner, estimate, created date
  - [ ] File path and line number
  - [ ] Dependencies (if `--deps`)
  - [ ] Blockers (if blocked)
  - [ ] Subtasks (if `--tree`)
- [ ] Implement display for file
  - [ ] File metadata (path, status)
  - [ ] Top-level tasks summary
  - [ ] Dependency count
  - [ ] Overall progress (X/Y tasks done)
- [ ] Implement JSON output
  - [ ] Complete task/file object with all fields

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
