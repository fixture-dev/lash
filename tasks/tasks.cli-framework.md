# CLI Framework Tasks

**Module:** `lash-cli`
**Dependencies:** tasks.error-handling.md, tasks.core-data-model.md
**Effort:** 6-8 days
**Priority:** CRITICAL

## Overview

The CLI framework provides the foundation for all `lash` commands: argument parsing, configuration management, output formatting, progress reporting, and integration with core subsystems. It must be ergonomic for both human and agent usage.

## Core Requirements

From design-doc.md:
- Single binary CLI with subcommands (section 7.1)
- Project root detection and configuration (section 7.2)
- Human-friendly and agent-friendly output (section 7.2)
- JSON output mode for machine parsing (section 7.2)
- Standardized exit codes (section 12.3)

---

## Task 1: CLI Argument Parsing Setup

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.project-setup.md

### Description

Set up the CLI argument parsing framework using `clap` with subcommands for all Lash operations.

### Subtasks

- [x] Add `clap` dependency with features
  - [x] `derive` feature for declarative syntax
  - [x] `color` feature for colored help
  - [x] `suggestions` feature for typo corrections
- [x] Define top-level `LashCli` struct
  - [x] Global flags: `--root <PATH>`, `--json`, `--verbose`, `--quiet`
  - [x] Version and help flags
  - [x] Subcommand enum
- [x] Define subcommands structure
  - [x] `lint`, `format`, `index`, `check-index`
  - [x] `list`, `show`, `search`
  - [x] `graph`, `check-links`
  - [x] `agent-prompt`, `tui`
  - [x] Each subcommand as separate struct with args
- [x] Implement global flag handling
  - [x] `--root`: Override project root detection
  - [x] `--json`: Enable machine-readable output
  - [x] `--verbose`: Increase logging level
  - [x] `--quiet`: Suppress non-essential output
- [x] Add shell completion generation
  - [x] Generate completions for bash, zsh, fish
  - [x] Hidden `lash completion <shell>` command

### Success Criteria

- All planned subcommands defined with appropriate arguments
- Help text is clear and comprehensive
- Global flags work across all subcommands
- Shell completions generate correctly

### Tests

- Unit: Test argument parsing for each subcommand
- Unit: Test global flag combinations
- Integration: Test help output formatting
- Manual: Test shell completions in actual shells

---

## Task 2: Project Root Detection

**Priority:** CRITICAL
**Effort:** 1 day
**Depends on:** Task 1

### Description

Implement logic to find the Lash project root directory automatically or use explicit configuration.

### Subtasks

- [x] Implement `ProjectRootFinder` struct
  - [x] Search strategy configuration
  - [x] Cache discovered root
- [x] Implement root detection algorithm
  - [x] Start from current directory
  - [x] Search upward for markers:
    - [x] `lash.index.md` or `index.lash.md`
    - [x] `.lash/` directory
    - [x] Stop at filesystem root or home directory
  - [x] Return path to project root
- [x] Handle explicit `--root` flag
  - [x] Validate path exists
  - [x] Validate path contains Lash markers
  - [x] Override automatic detection
- [x] Error handling
  - [x] No project root found: clear error message
  - [x] Suggest `lash init` command (future feature)
  - [x] Show search path for debugging
- [ ] Add configuration file support (optional)
  - [ ] `.lash/config.toml` for project settings
  - [ ] Override default marker filenames

### Success Criteria

- Correctly detects project root in nested directories
- Handles missing root gracefully with helpful message
- `--root` flag overrides automatic detection
- Fast: <10ms for typical search depth

### Tests

- Unit: Test upward search from various depths
- Unit: Test with different marker files
- Unit: Test `--root` override
- Unit: Test error cases (no root found)
- Integration: Test from various directories in fixture project

---

## Task 3: Output Formatting System

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 1, tasks.error-handling.md#1

### Description

Implement flexible output formatting supporting human-readable text, JSON, and other formats.

### Subtasks

- [x] Define `OutputFormat` enum
  - [x] `Text` - human-readable, colored (default)
  - [x] `Json` - machine-readable, structured
  - [x] `JsonPretty` - formatted JSON for debugging
  - [x] `Quiet` - minimal output
- [x] Implement `OutputFormatter` trait
  - [x] `format_success()` - format success messages
  - [x] `format_error()` - format errors (delegate to error handler)
  - [x] `format_list()` - format lists of items
  - [x] `format_table()` - format tabular data
  - [x] `format_progress()` - format progress indicators (via separate progress module)
- [x] Implement `TextFormatter`
  - [x] Use `owo-colors` for ANSI colors
  - [x] Respect `NO_COLOR` env var
  - [x] Auto-disable colors for non-TTY output
  - [x] Pretty tables with alignment
  - [ ] Unicode box-drawing characters (with ASCII fallback) (deferred - current impl uses ASCII)
- [x] Implement `JsonFormatter`
  - [x] Serialize all output to JSON objects
  - [x] Stable schema (document format)
  - [x] Include metadata (status field)
- [x] Add output writer abstraction
  - [x] Default: stdout/stderr (via trait methods)
  - [x] Allow custom writers for testing (via trait)
  - [ ] Buffer output for atomic writes (deferred - not critical for v1)
- [x] Integrate with verbosity levels
  - [x] Quiet: only errors and critical info
  - [x] Normal: errors, warnings, results
  - [x] Verbose: + informational messages
  - [x] Debug: + debug messages (enable via env var)

### Success Criteria

- Text output is readable and well-formatted
- JSON output is valid and parseable
- Colors work in terminals, disabled otherwise
- Verbosity levels control output appropriately

### Tests

- Unit: Test each formatter independently
- Unit: Test color enable/disable logic
- Unit: Test JSON schema validity
- Integration: Test CLI commands with different formats
- Snapshot: Capture and verify formatted output

---

## Task 4: Progress Reporting

**Priority:** MEDIUM
**Effort:** 2 days
**Depends on:** Task 3

### Description

Implement progress reporting for long-running operations (indexing, searching, etc.).

### Subtasks

- [x] Define `ProgressReporter` trait
  - [x] `start(total_items)` - begin operation
  - [x] `update(current, message)` - update progress
  - [x] `finish(message)` - complete operation
  - [x] `set_message(message)` - update status message
- [x] Implement `TerminalProgressReporter`
  - [x] Use `indicatif` crate for progress bars
  - [x] Show: [===>    ] 45% (123/456) Status message
  - [x] Spinner for indeterminate progress
  - [ ] Multi-line support for parallel operations (deferred - not needed for v1)
  - [x] Auto-clear on completion
- [x] Implement `JsonProgressReporter`
  - [x] Emit progress events as JSON lines
  - [x] Format: `{"event": "progress", "current": 123, "total": 456, "percent": 45, "message": "..."}`
- [x] Implement `QuietProgressReporter`
  - [x] No-op implementation (suppresses all progress)
- [x] Add progress rate estimation
  - [x] Items per second
  - [x] ETA calculation
  - [x] Show in progress bar
- [ ] Handle terminal resize
  - [ ] Re-render progress bar on window size change (deferred - indicatif handles this automatically)

### Success Criteria

- Progress bars display correctly in terminals
- JSON progress events are parseable
- ETA estimates are reasonable
- No flicker or rendering artifacts

### Tests

- Unit: Test progress update logic
- Unit: Test rate estimation
- Integration: Test with long-running operations (mocked)
- Manual: Visual inspection of progress bars

---

## Task 5: Configuration Management

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 2

### Description

Support project-level and user-level configuration files for customizing Lash behavior.

### Subtasks

- [x] Define configuration schema
  - [x] Use `serde` with `toml` for TOML parsing
  - [x] Support for:
    - [x] Default output format
    - [x] Verbosity level
    - [x] Linter settings (depth limits, etc.)
    - [x] Search settings (fuzzy threshold, etc.)
    - [x] Agent settings (token budgets, etc.)
- [x] Implement configuration file locations
  - [x] Project: `.lash/config.toml` (in project root)
  - [x] User: `~/.config/lash/config.toml`
  - [x] Merge strategy: CLI flags > project > user > defaults
- [x] Implement `Config` struct
  - [x] Load from file(s)
  - [x] Merge with CLI flags
  - [x] Validate settings
  - [x] Provide defaults for all settings
- [x] Add configuration validation
  - [x] Check for unknown keys (warn)
  - [x] Validate value types and ranges
  - [x] Return helpful error messages
- [x] Add `lash config` command (optional)
  - [x] `lash config get <key>` - show current value
  - [x] `lash config set <key> <value>` - update config
  - [x] `lash config list` - show all settings

### Success Criteria

- Config files load and parse correctly
- Merging priority works as expected
- Validation catches invalid settings
- Default values work when no config present

### Tests

- Unit: Test config loading from TOML
- Unit: Test merge strategy
- Unit: Test validation logic
- Integration: Test with fixture config files

---

## Task 6: Logging and Diagnostics

**Priority:** MEDIUM
**Effort:** 1 day
**Depends on:** Task 3
**Status:** COMPLETE

### Description

Set up structured logging for debugging and diagnostics.

### Subtasks

- [x] Add logging dependencies
  - [x] `tracing` for structured logging
  - [x] `tracing-subscriber` for log output
  - [x] `tracing-appender` for file logging (optional)
- [x] Implement log level configuration
  - [x] Map verbosity flags to log levels:
    - [x] Quiet: ERROR only
    - [x] Normal: WARN
    - [x] Verbose (-v): INFO
    - [x] Very Verbose (-vv): DEBUG
    - [x] Debug (-vvv): TRACE
  - [x] Environment variable: `LASH_LOG=debug` (overrides flags)
  - [x] `RUST_LOG` as fallback
- [x] Configure log output format
  - [x] Terminal: compact, colored (respects NO_COLOR)
  - [x] File: full, structured JSON
  - [x] JSON mode: emit logs as JSON events to stderr
- [x] Add diagnostic spans
  - [x] Wrap major operations in spans (lint, format commands)
  - [x] Include timing information via #[instrument]
  - [x] Nest spans for detailed traces
- [x] Add crash reporting
  - [x] Catch panics and log backtrace
  - [x] Suggest filing bug report with logs
  - [x] Include version and platform info

### Success Criteria

- [x] Logs are helpful for debugging issues
- [x] Log levels control verbosity appropriately
- [x] Structured logs are machine-parseable
- [x] Crash reports include useful diagnostic info

### Tests

- [x] Unit: Test log level mapping
- [x] Integration: Verify logs appear with verbose flag
- [x] Manual: Inspect log output format

---

## Task 7: Command Execution Framework

**Priority:** HIGH
**Effort:** 1-2 days
**Depends on:** Task 1-3

### Description

Implement the command dispatch and execution framework that all subcommands use.

### Subtasks

- [x] Define `Command` trait
  - [x] `execute(&self, context: &Context) -> Result<()>`
  - [x] Each subcommand implements this trait
- [x] Implement `Context` struct
  - [x] Hold shared state: config, project root, DB connection, formatter
  - [x] Lazy initialization of expensive resources
- [x] Implement command dispatch (partial - foundation in place)
  - [x] Parse CLI args
  - [x] Detect project root
  - [x] Load configuration
  - [x] Initialize context
  - [ ] Dispatch to appropriate command (deferred - keeping current main.rs pattern)
  - [ ] Handle result and format output (deferred - keeping current pattern)
- [x] Implement error propagation
  - [x] Commands return `Result<(), LashError>` via Command trait
  - [x] Errors bubbled to top-level handler
  - [x] Format and display errors (existing)
  - [x] Set appropriate exit code (existing in main.rs)
- [x] Add common command utilities
  - [x] `ensure_indexed()` - verify DB is up to date (placeholder for future)
  - [x] `get_db()` - get database connection (placeholder for future)
  - [x] `get_parser()` - get Markdown parser (placeholder for future)
  - [x] `prompt_confirmation()` - ask user yes/no

### Success Criteria

- All commands follow consistent execution pattern
- Context initialization is efficient (lazy loading)
- Error handling is consistent across commands
- Utilities reduce boilerplate in command implementations

### Tests

- Integration: Test command dispatch for each subcommand
- Unit: Test context initialization
- Unit: Test utility functions

---

## Task 8: Exit Code Standardization

**Priority:** LOW
**Effort:** 0.5 days
**Depends on:** Task 7
**Status:** COMPLETE

### Description

Define and implement standardized exit codes as specified in design doc section 12.3.

### Subtasks

- [x] Define `ExitCode` enum
  - [x] `Success = 0`
  - [x] `GeneralError = 1`
  - [x] `LintError = 2`
  - [x] `IndexError = 3`
  - [x] `ConfigError = 4`
  - [x] `NotFound = 5` (task/file not found)
  - [x] `CycleDetected = 6`
- [x] Implement exit code mapping
  - [x] Map `LashError` variants to exit codes
  - [x] Document exit codes in help text and man page
- [ ] Add `--exit-zero` flag (optional, deferred)
  - [ ] Force exit code 0 even on errors
  - [ ] Useful for scripts that handle errors via output parsing
- [x] Test exit codes
  - [x] Verify each error type produces correct code

### Success Criteria

- [x] Exit codes are consistent and documented
- [x] Agents and scripts can rely on exit codes for error detection

### Tests

- [x] Unit: Test exit code mapping for all error variants (15 tests)
- [ ] Integration: Test exit code for each command scenario (deferred)
- [ ] Integration: Test `--exit-zero` flag (deferred - feature not implemented)

---

## Non-Goals (for v1)

- Interactive CLI prompts (except simple confirmations)
- Paging of long output (user can pipe to `less`)
- Custom themes or color schemes (use sensible defaults)
- Plugin system for custom commands

---

## Open Questions

- **Progress bars:** Always show for long operations or only with `--progress`?
- **JSON schema:** Document formally or just by example?
- **Config format:** TOML vs YAML vs JSON? (Suggest TOML for readability)
- **Logging to file:** Default enabled or opt-in?

---

## References

- Design doc section 7 (CLI Design)
- Design doc section 12 (Error Handling & UX)
- `clap` documentation: https://docs.rs/clap/
- `indicatif` documentation: https://docs.rs/indicatif/
