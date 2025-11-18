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

- [ ] Add `clap` dependency with features
  - [ ] `derive` feature for declarative syntax
  - [ ] `color` feature for colored help
  - [ ] `suggestions` feature for typo corrections
- [ ] Define top-level `LashCli` struct
  - [ ] Global flags: `--root <PATH>`, `--json`, `--verbose`, `--quiet`
  - [ ] Version and help flags
  - [ ] Subcommand enum
- [ ] Define subcommands structure
  - [ ] `lint`, `format`, `index`, `check-index`
  - [ ] `list`, `show`, `search`
  - [ ] `graph`, `check-links`
  - [ ] `agent-prompt`, `tui`
  - [ ] Each subcommand as separate struct with args
- [ ] Implement global flag handling
  - [ ] `--root`: Override project root detection
  - [ ] `--json`: Enable machine-readable output
  - [ ] `--verbose`: Increase logging level
  - [ ] `--quiet`: Suppress non-essential output
- [ ] Add shell completion generation
  - [ ] Generate completions for bash, zsh, fish
  - [ ] Hidden `lash completion <shell>` command

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

- [ ] Implement `ProjectRootFinder` struct
  - [ ] Search strategy configuration
  - [ ] Cache discovered root
- [ ] Implement root detection algorithm
  - [ ] Start from current directory
  - [ ] Search upward for markers:
    - [ ] `lash.index.md` or `index.lash.md`
    - [ ] `.lash/` directory
    - [ ] Stop at filesystem root or home directory
  - [ ] Return path to project root
- [ ] Handle explicit `--root` flag
  - [ ] Validate path exists
  - [ ] Validate path contains Lash markers
  - [ ] Override automatic detection
- [ ] Error handling
  - [ ] No project root found: clear error message
  - [ ] Suggest `lash init` command (future feature)
  - [ ] Show search path for debugging
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

- [ ] Define `OutputFormat` enum
  - [ ] `Text` - human-readable, colored (default)
  - [ ] `Json` - machine-readable, structured
  - [ ] `JsonPretty` - formatted JSON for debugging
  - [ ] `Quiet` - minimal output
- [ ] Implement `OutputFormatter` trait
  - [ ] `format_success()` - format success messages
  - [ ] `format_error()` - format errors (delegate to error handler)
  - [ ] `format_list()` - format lists of items
  - [ ] `format_table()` - format tabular data
  - [ ] `format_progress()` - format progress indicators
- [ ] Implement `TextFormatter`
  - [ ] Use `termcolor` or `colored` for ANSI colors
  - [ ] Respect `NO_COLOR` env var
  - [ ] Auto-disable colors for non-TTY output
  - [ ] Pretty tables with alignment
  - [ ] Unicode box-drawing characters (with ASCII fallback)
- [ ] Implement `JsonFormatter`
  - [ ] Serialize all output to JSON objects
  - [ ] Stable schema (document format)
  - [ ] Include metadata (timestamp, version, etc.)
- [ ] Add output writer abstraction
  - [ ] Default: stdout/stderr
  - [ ] Allow custom writers for testing
  - [ ] Buffer output for atomic writes
- [ ] Integrate with verbosity levels
  - [ ] Quiet: only errors and critical info
  - [ ] Normal: errors, warnings, results
  - [ ] Verbose: + informational messages
  - [ ] Debug: + debug messages (enable via env var)

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

- [ ] Define `ProgressReporter` trait
  - [ ] `start(total_items)` - begin operation
  - [ ] `update(current, message)` - update progress
  - [ ] `finish(message)` - complete operation
  - [ ] `set_message(message)` - update status message
- [ ] Implement `TerminalProgressReporter`
  - [ ] Use `indicatif` crate for progress bars
  - [ ] Show: [===>    ] 45% (123/456) Status message
  - [ ] Spinner for indeterminate progress
  - [ ] Multi-line support for parallel operations
  - [ ] Auto-clear on completion
- [ ] Implement `JsonProgressReporter`
  - [ ] Emit progress events as JSON lines
  - [ ] Format: `{"event": "progress", "current": 123, "total": 456, "percent": 45, "message": "..."}`
- [ ] Implement `QuietProgressReporter`
  - [ ] No-op implementation (suppresses all progress)
- [ ] Add progress rate estimation
  - [ ] Items per second
  - [ ] ETA calculation
  - [ ] Show in progress bar
- [ ] Handle terminal resize
  - [ ] Re-render progress bar on window size change

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

- [ ] Define configuration schema
  - [ ] Use `serde` with `toml` for TOML parsing
  - [ ] Support for:
    - [ ] Default output format
    - [ ] Verbosity level
    - [ ] Linter settings (depth limits, etc.)
    - [ ] Search settings (fuzzy threshold, etc.)
    - [ ] Agent settings (token budgets, etc.)
- [ ] Implement configuration file locations
  - [ ] Project: `.lash/config.toml` (in project root)
  - [ ] User: `~/.config/lash/config.toml`
  - [ ] Merge strategy: CLI flags > project > user > defaults
- [ ] Implement `Config` struct
  - [ ] Load from file(s)
  - [ ] Merge with CLI flags
  - [ ] Validate settings
  - [ ] Provide defaults for all settings
- [ ] Add configuration validation
  - [ ] Check for unknown keys (warn)
  - [ ] Validate value types and ranges
  - [ ] Return helpful error messages
- [ ] Add `lash config` command (optional)
  - [ ] `lash config get <key>` - show current value
  - [ ] `lash config set <key> <value>` - update config
  - [ ] `lash config list` - show all settings

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

### Description

Set up structured logging for debugging and diagnostics.

### Subtasks

- [ ] Add logging dependencies
  - [ ] `tracing` for structured logging
  - [ ] `tracing-subscriber` for log output
  - [ ] `tracing-appender` for file logging (optional)
- [ ] Implement log level configuration
  - [ ] Map verbosity flags to log levels:
    - [ ] Quiet: ERROR only
    - [ ] Normal: WARN
    - [ ] Verbose: INFO
    - [ ] Debug (env var): DEBUG or TRACE
  - [ ] Environment variable: `LASH_LOG=debug`
- [ ] Configure log output format
  - [ ] Terminal: compact, colored
  - [ ] File: full, structured
  - [ ] JSON mode: emit logs as JSON events
- [ ] Add diagnostic spans
  - [ ] Wrap major operations in spans (indexing, parsing, etc.)
  - [ ] Include timing information
  - [ ] Nest spans for detailed traces
- [ ] Add crash reporting
  - [ ] Catch panics and log backtrace
  - [ ] Suggest filing bug report with logs
  - [ ] Include version and platform info

### Success Criteria

- Logs are helpful for debugging issues
- Log levels control verbosity appropriately
- Structured logs are machine-parseable
- Crash reports include useful diagnostic info

### Tests

- Unit: Test log level mapping
- Integration: Verify logs appear with verbose flag
- Manual: Inspect log output format

---

## Task 7: Command Execution Framework

**Priority:** HIGH
**Effort:** 1-2 days
**Depends on:** Task 1-3

### Description

Implement the command dispatch and execution framework that all subcommands use.

### Subtasks

- [ ] Define `Command` trait
  - [ ] `execute(&self, context: &Context) -> Result<()>`
  - [ ] Each subcommand implements this trait
- [ ] Implement `Context` struct
  - [ ] Hold shared state: config, project root, DB connection, formatter
  - [ ] Lazy initialization of expensive resources
- [ ] Implement command dispatch
  - [ ] Parse CLI args
  - [ ] Detect project root
  - [ ] Load configuration
  - [ ] Initialize context
  - [ ] Dispatch to appropriate command
  - [ ] Handle result and format output
- [ ] Implement error propagation
  - [ ] Commands return `Result<(), LashError>`
  - [ ] Errors bubbled to top-level handler
  - [ ] Format and display errors
  - [ ] Set appropriate exit code
- [ ] Add common command utilities
  - [ ] `ensure_indexed()` - verify DB is up to date
  - [ ] `get_db()` - get database connection
  - [ ] `get_parser()` - get Markdown parser
  - [ ] `prompt_confirmation()` - ask user yes/no

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

### Description

Define and implement standardized exit codes as specified in design doc section 12.3.

### Subtasks

- [ ] Define `ExitCode` enum
  - [ ] `Success = 0`
  - [ ] `GeneralError = 1`
  - [ ] `LintError = 2`
  - [ ] `IndexError = 3`
  - [ ] `ConfigError = 4`
  - [ ] `NotFound = 5` (task/file not found)
  - [ ] `CycleDetected = 6`
- [ ] Implement exit code mapping
  - [ ] Map `LashError` variants to exit codes
  - [ ] Document exit codes in help text and man page
- [ ] Add `--exit-zero` flag (optional)
  - [ ] Force exit code 0 even on errors
  - [ ] Useful for scripts that handle errors via output parsing
- [ ] Test exit codes
  - [ ] Verify each error type produces correct code

### Success Criteria

- Exit codes are consistent and documented
- Agents and scripts can rely on exit codes for error detection

### Tests

- Integration: Test exit code for each command scenario
- Integration: Test `--exit-zero` flag

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
