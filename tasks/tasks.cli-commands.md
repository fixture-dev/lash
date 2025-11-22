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

## Task 10: `lash agent-prompt` Command ✅

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** tasks.agent-integration.md#1-2, tasks.cli-framework.md#1-3
**Status:** Complete

### Description

Implement the `lash agent-prompt` command to generate context for AI agents.

### Subtasks

- [x] Define `AgentPromptCommand` struct
  - [x] Flags:
    - [x] `--format <plain|json|claude-skill|agents-md>` (default: plain)
    - [x] `--label <label>` (filter tasks by labels, repeatable)
    - [x] `--path <path>` (filter tasks by path)
    - [x] `--max-tokens <n>` (limit output size)
- [x] Implement plain text format
  - [x] Schema description
  - [x] Allowed operations
  - [x] Format rules
  - [x] Example snippets
  - [x] Relevant tasks with filtering
- [x] Implement JSON format
  - [x] Structured schema object
  - [x] Operations array
  - [x] Examples array
- [x] Implement Claude skill format (placeholder)
  - [x] JSON spec for Claude Code
  - [x] Command definitions
- [x] Implement Agents.md format
  - [x] Ready-to-paste markdown fragment
- [x] Implement token budget limiting
  - [x] Estimate token count (words * 1.3 heuristic)
  - [x] Truncate or summarize to fit budget
  - [x] Prioritize schema > examples > task list
- [x] Add task filtering
  - [x] Filter by labels
  - [x] Filter by path
  - [x] Load summaries from database

### Success Criteria

- ✅ Generated prompts are clear and actionable
- ✅ Agents can use output to understand Lash
- ✅ Token budgets are respected
- ✅ Examples are accurate and helpful

### Tests

- ✅ Unit: 31 tests in lash-agent (schema, tokens, prompt)
- ✅ Unit: 12 doctests in lash-agent
- ✅ Unit: 3 tests in agent_prompt command
- ✅ All workspace tests pass
- ✅ Clippy clean

### Implementation Notes

Implemented in commit 590edf2:
- Created lash-agent crate with schema.rs, tokens.rs, and prompt.rs modules
- Implements 4 output formats: plain, JSON, claude-skill, agents-md
- Token-aware with budget distribution across sections
- Integrates with lash-db for task summaries
- Supports filtering by labels and path

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

## Task 12: Implement `lash check-links --fix` Mode ✅

**Priority:** LOW
**Effort:** 2-3 days
**Depends on:** Task 9 (check-links command)
**Status:** Complete

### Description

Implement the `--fix` flag for `lash check-links` to automatically repair broken dependency references using fuzzy matching and interactive confirmation.

### Subtasks

- [x] Add `strsim` dependency to workspace
- [x] Implement fuzzy matching algorithm
  - [x] Use Levenshtein distance for similarity scoring
  - [x] Search for tasks with similar IDs across all files
  - [x] Rank candidates by similarity score (threshold: 0.6, max: 5 candidates)
- [x] Implement interactive confirmation UI
  - [x] Show broken reference and suggested fixes with confidence scores
  - [x] Allow user to accept, reject, or manually specify fix
  - [x] Support `--yes` flag for non-interactive mode (auto-accept confidence >= 85%)
  - [x] Support `--dry-run` flag to preview changes
- [x] Implement Markdown file updating
  - [x] Parse file to locate `@depends-on` annotation using regex
  - [x] Replace broken reference with corrected one
  - [x] Preserve formatting and whitespace
  - [x] Create backups in `.lash/backups/TIMESTAMP/` before modifications
- [x] Re-index after fixing
  - [x] Run indexer on modified files
  - [x] Handle re-indexing errors gracefully
- [x] Add comprehensive tests
  - [x] Unit: Fuzzy matcher tests (14 tests)
  - [x] Unit: Annotation editor tests (6 tests)
  - [x] Unit: Interactive prompter tests (2 tests)
  - [x] Integration: Core check-links tests (3 tests)
  - [x] All tests passing with clippy clean

### Success Criteria

- ✅ Accurately suggests fixes for common typos and mistakes (Levenshtein-based scoring)
- ✅ Interactive mode provides clear choices (numbered options with confidence percentages)
- ✅ File updates preserve formatting (regex-based targeted updates)
- ✅ Re-indexing verifies fixes worked (automatic re-index after applying fixes)
- ✅ Safe: creates backups before modifying files (timestamped backups in `.lash/backups/`)

### Tests

- ✅ Unit: 22 tests covering fuzzy matching, annotation editing, and interactive UI
- ✅ Integration: 3 existing check-links integration tests
- ✅ All tests pass; clippy clean (with auto-fixes applied)

### Implementation Notes

Implemented in the following modules:
- `check_links/fuzzy_matcher.rs` - Levenshtein-based fuzzy matching (136 lines)
- `check_links/interactive.rs` - Interactive confirmation UI (265 lines)
- `check_links/annotation_editor.rs` - Safe Markdown file editing (386 lines)
- `check_links/mod.rs` - Fix orchestration and CLI integration (393 lines)

Key design decisions:
- Similarity threshold: 0.6 minimum, 0.85 for auto-fix
- Max candidates: 5 (prevents overwhelming users)
- Backup location: `.lash/backups/TIMESTAMP/` (allows rollback)
- Uses existing `index` command for re-indexing (DRY principle)

---

## Task 13: Clean Up Unimplemented Search Command Features ✅

**Priority:** HIGH
**Effort:** 0.5 days
**Depends on:** Task 7 (search command)
**Status:** Complete

### Description

Remove or properly implement unimplemented features in the search command to maintain code quality and avoid confusion.

### Subtasks

- [x] Remove `--threshold` flag references
  - [x] Note: Flag was never in `SearchArgs` struct or CLI args
  - [x] Removed misleading suggestion from no-results output
  - [x] Verified tests still pass
  - [x] Note: FTS5 doesn't support fuzzy threshold; this flag doesn't apply
- [x] Review `highlight_matches()` function
  - [x] Note: No dead code found - function doesn't exist in current implementation
  - [x] No `#[allow(dead_code)]` attributes present
- [x] Address `--scope` flag
  - [x] Decision: Defer to future enhancement (not blocking)
  - [x] Scope parameter exists in SearchQuery but not exposed in CLI
  - [x] Can be added in future task if needed
- [x] Update documentation
  - [x] Removed outdated "partially implemented" status
  - [x] Updated module docs to reflect actual FTS5 implementation

### Success Criteria

- ✅ No `#[allow(dead_code)]` attributes in search command
- ✅ All defined flags are implemented and functional
- ✅ Code is clean with no misleading suggestions or outdated docs

### Tests

- ✅ All existing search tests pass
- ✅ Clippy passes with no warnings

### Implementation Notes

Completed cleanup in search.rs:
- Removed misleading `--threshold` suggestion from no-results message (line 181)
- Updated module documentation to reflect actual FTS5 implementation
- No code removal needed - the `--threshold` flag was never actually implemented
- The search command is fully functional with FTS5-based full-text search

---

## Task 14: Implement `lash show --deps` and `--rdeps` Flags

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 6 (show command), tasks.dependency-resolution.md
**Status:** Not Started

### Description

Fully implement the `--deps` and `--rdeps` flags for the `lash show` command to display task dependencies and reverse dependencies.

### Subtasks

- [ ] Add repository method to query tasks by database ID
  - [ ] `get_task_by_db_id(id: i64) -> Result<Task>`
  - [ ] Implement in `lash-db` repository
  - [ ] Add tests for new method
- [ ] Implement `--deps` flag
  - [ ] Query dependencies table for task's outgoing edges
  - [ ] Resolve target task IDs to full task records
  - [ ] Format and display dependency list
  - [ ] Show dependency kind (depends-on, blocks, etc.)
- [ ] Implement `--rdeps` flag
  - [ ] Query dependencies table for incoming edges
  - [ ] Resolve source task IDs to full task records
  - [ ] Format and display reverse dependency list
- [ ] Update output formatting
  - [ ] Show task ID, title, status for each dependency
  - [ ] Group by dependency type
  - [ ] Use colors for status
- [ ] Remove placeholder empty Vec returns
  - [ ] Remove comments about missing implementation
  - [ ] Ensure full functionality
- [ ] Add comprehensive tests
  - [ ] Integration: Show task with dependencies
  - [ ] Integration: Show task with reverse dependencies
  - [ ] Integration: Show task with both
  - [ ] Integration: Show task with no dependencies

### Success Criteria

- `--deps` shows all task dependencies accurately
- `--rdeps` shows all tasks that depend on this one
- Output is clear and well-formatted
- Fast queries (<100ms)

### Tests

- Integration: Task with multiple dependencies
- Integration: Task with multiple dependents
- Integration: Task in dependency chain
- Integration: Isolated task (no deps)

---

## Non-Goals (for v1)

- `lash init` command (manually create project structure)
- `lash archive` command (future)
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
