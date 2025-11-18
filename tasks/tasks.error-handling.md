# Error Handling Tasks

**Module:** `lash-core` (error types and formatting)
**Dependencies:** tasks.project-setup.md
**Effort:** 4-6 days
**Priority:** CRITICAL

## Overview

Implement comprehensive, user-friendly error handling throughout Lash. Errors must be expressive for humans, structured for machines, and actionable for both users and agents.

## Core Requirements

From design-doc.md section 12:
- Expressive error messages (section 12.1)
- Structured, machine-readable error data (section 12.1)
- Stable error codes (section 12.1)
- Standardized exit codes (section 12.3)

---

## Task 1: Error Type Taxonomy

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.project-setup.md

### Description

Define the complete taxonomy of error types that can occur in Lash, with stable codes and hierarchical organization.

### Subtasks

- [ ] Define `LashError` enum
  - [ ] Top-level error categories:
    - [ ] `Parse` - Markdown parsing errors
    - [ ] `Lint` - Linting/validation errors
    - [ ] `Index` - Indexing/database errors
    - [ ] `Dependency` - Dependency resolution errors
    - [ ] `Query` - Query/search errors
    - [ ] `Config` - Configuration errors
    - [ ] `IO` - File system errors
    - [ ] `Internal` - Internal/unexpected errors
  - [ ] Each variant contains specific error details
- [ ] Define error codes
  - [ ] Stable string codes (e.g., `E_PARSE_001`, `E_LINT_002`)
  - [ ] Hierarchical naming: `<CATEGORY>_<SUBCATEGORY>_<NUMBER>`
  - [ ] Document all codes in error catalog
- [ ] Implement error variants
  - [ ] **Parse errors:**
    - [ ] Invalid checkbox syntax
    - [ ] Malformed annotation
    - [ ] Invalid header format
    - [ ] Unexpected depth
  - [ ] **Lint errors:**
    - [ ] Duplicate ID
    - [ ] Unknown annotation
    - [ ] Depth limit exceeded
    - [ ] Status inconsistency (parent done, children open)
    - [ ] Invalid label format
  - [ ] **Dependency errors:**
    - [ ] Broken reference (target not found)
    - [ ] Circular dependency
    - [ ] Invalid reference format
  - [ ] **Index errors:**
    - [ ] Database corruption
    - [ ] Schema version mismatch
    - [ ] Index out of sync
  - [ ] **IO errors:**
    - [ ] File not found
    - [ ] Permission denied
    - [ ] Invalid path
- [ ] Add error context
  - [ ] File path
  - [ ] Line and column numbers
  - [ ] Relevant snippet (for parse/lint errors)
  - [ ] Dependency chain (for dependency errors)
- [ ] Implement `std::error::Error` trait
  - [ ] Human-readable `Display` implementation
  - [ ] `source()` for error chains
  - [ ] `Debug` implementation with full context

### Success Criteria

- Complete taxonomy covers all error scenarios
- Error codes are stable and well-documented
- Each error variant contains relevant context
- Errors implement standard traits

### Tests

- Unit: Construct each error type
- Unit: Test error Display output
- Unit: Test error code stability
- Documentation: Error catalog is complete

---

## Task 2: Error Formatting

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** Task 1, tasks.cli-framework.md#3

### Description

Implement rich error formatting for both human and machine consumption.

### Subtasks

- [ ] Implement `ErrorFormatter` trait
  - [ ] `format_human()` - human-readable text
  - [ ] `format_json()` - structured JSON
  - [ ] Support for colored output
- [ ] Implement human-readable formatting
  - [ ] Use `miette` or similar crate for rich diagnostics
  - [ ] Show:
    - [ ] Error message (clear, concise)
    - [ ] File path, line, column
    - [ ] Code snippet with highlighting
    - [ ] Caret (^) pointing to error location
    - [ ] Help text / suggestion
  - [ ] Color coding:
    - [ ] Red: error message
    - [ ] Cyan: file path
    - [ ] Yellow: warning text
    - [ ] Gray: code snippet
  - [ ] Example format:
    ```
    error[E_LINT_001]: duplicate task ID 'setup-db'
      --> tasks/database.md:15:3
       |
    15 | @id: setup-db
       |      ^^^^^^^^ duplicate ID
       |
    help: task IDs must be unique within a file
    ```
- [ ] Implement JSON formatting
  - [ ] Schema:
    ```json
    {
      "code": "E_LINT_001",
      "severity": "error",
      "message": "duplicate task ID 'setup-db'",
      "location": {
        "file": "tasks/database.md",
        "line": 15,
        "column": 3
      },
      "snippet": "@id: setup-db",
      "help": "task IDs must be unique within a file"
    }
    ```
  - [ ] Include all context fields
  - [ ] Stable schema (document version)
- [ ] Implement error suggestions
  - [ ] Contextual help for each error type
  - [ ] Suggest fixes where possible
  - [ ] Link to documentation (optional)
- [ ] Add severity levels
  - [ ] Error (must fix)
  - [ ] Warning (should fix)
  - [ ] Info (nice to fix)
  - [ ] Hint (style suggestion)

### Success Criteria

- Human-readable errors are clear and actionable
- JSON errors are parseable and complete
- Color coding improves readability
- Suggestions help users fix errors quickly

### Tests

- Unit: Test human formatting for each error type
- Unit: Test JSON formatting
- Unit: Validate JSON schema
- Snapshot: Capture formatted output for regression testing
- Manual: Visual inspection of formatted errors

---

## Task 3: Error Aggregation

**Priority:** HIGH
**Effort:** 1-2 days
**Depends on:** Task 1, Task 2

### Description

Implement error collection and aggregation for batch operations (linting, indexing).

### Subtasks

- [ ] Define `ErrorReport` struct
  - [ ] List of errors
  - [ ] Grouping (by file, by type)
  - [ ] Summary statistics
- [ ] Implement error collection
  - [ ] Collect all errors (don't stop on first)
  - [ ] Associate with source file/operation
  - [ ] Maintain order (by file, then line)
- [ ] Implement error grouping
  - [ ] Group by file (show all errors per file together)
  - [ ] Group by type (show all duplicates together)
  - [ ] Configurable grouping strategy
- [ ] Implement summary reporting
  - [ ] Count errors by severity
  - [ ] Count errors by type
  - [ ] Show affected files count
  - [ ] Example:
    ```
    Found 5 errors in 3 files:
      - 3 lint errors
      - 2 dependency errors
    ```
- [ ] Implement report rendering
  - [ ] Text format: grouped errors with headers
  - [ ] JSON format: array of error objects + summary
  - [ ] Limit display (show first N, summarize rest)
- [ ] Add filtering
  - [ ] Filter by severity
  - [ ] Filter by file
  - [ ] Filter by error code

### Success Criteria

- Error reports are comprehensive and organized
- Grouping makes large error sets manageable
- Summary provides quick overview
- Filtering works correctly

### Tests

- Unit: Test error collection
- Unit: Test grouping strategies
- Integration: Generate report from fixture errors
- Integration: Test filtering

---

## Task 4: Agent-Friendly Error Messages

**Priority:** HIGH
**Effort:** 1 day
**Depends on:** Task 1, Task 2

### Description

Ensure error messages are optimized for AI agent consumption and error recovery.

### Subtasks

- [ ] Add structured error context for agents
  - [ ] Exact error location (file, line, col)
  - [ ] Error code (for matching against documentation)
  - [ ] Actionable fix suggestion (specific steps)
  - [ ] Related context (affected dependencies, etc.)
- [ ] Implement error recovery hints
  - [ ] "Run `lash format` to fix formatting"
  - [ ] "Remove duplicate ID or rename to unique value"
  - [ ] "Update reference to: `correct/path.md#task:id`"
- [ ] Add error documentation links
  - [ ] URL to error code documentation (if available)
  - [ ] Inline explanation in JSON output
- [ ] Implement `--explain` flag (optional)
  - [ ] `lash lint --explain E_LINT_001`
  - [ ] Show detailed explanation of error code
  - [ ] Show examples and fixes
- [ ] Ensure JSON errors are complete
  - [ ] All fields populated
  - [ ] No ambiguous or vague messages
  - [ ] Include context needed for automated fixes

### Success Criteria

- Agents can parse and understand errors
- Recovery hints are actionable
- JSON output is complete and unambiguous
- Error explanations are helpful

### Tests

- Integration: Generate errors, validate JSON completeness
- Integration: Test recovery hints
- Manual: Use errors with agent, assess usability

---

## Task 5: Error Reporting in Commands

**Priority:** MEDIUM
**Effort:** 1 day
**Depends on:** Task 2, Task 3, tasks.cli-framework.md#7

### Description

Integrate error formatting and reporting into all CLI commands.

### Subtasks

- [ ] Implement top-level error handler
  - [ ] Catch `LashError` from commands
  - [ ] Format based on output mode (text/JSON)
  - [ ] Display to user
  - [ ] Set exit code
- [ ] Add error reporting to each command
  - [ ] `lint`: collect all errors, show report
  - [ ] `format`: report formatting failures
  - [ ] `index`: report parse and DB errors
  - [ ] `list`: report query errors
  - [ ] `show`: report not found errors
  - [ ] `search`: report search errors
  - [ ] `graph`: report dependency errors
  - [ ] `check-links`: report broken links
- [ ] Implement progress-aware error display
  - [ ] Show errors as they occur (streaming)
  - [ ] Or collect and show at end (batch)
  - [ ] Configurable via flag
- [ ] Add error verbosity control
  - [ ] Quiet: only error count
  - [ ] Normal: error messages (default)
  - [ ] Verbose: error messages + context
  - [ ] Debug: full error details + backtrace

### Success Criteria

- All commands report errors consistently
- Error formatting respects output mode
- Verbosity control works across commands
- Exit codes are set correctly

### Tests

- Integration: Test error reporting for each command
- Integration: Test with different verbosity levels
- Integration: Verify exit codes

---

## Task 6: Error Recovery and Validation

**Priority:** LOW
**Effort:** 1 day
**Depends on:** Task 1-4

### Description

Implement utilities for error recovery and validation to help users fix errors quickly.

### Subtasks

- [ ] Implement `ErrorValidator`
  - [ ] Validate that a file has no errors after fix
  - [ ] Re-run lint after auto-fix
  - [ ] Report remaining errors
- [ ] Implement auto-fix suggestions
  - [ ] Detect fixable errors (whitespace, formatting)
  - [ ] Generate fix suggestions
  - [ ] Apply fixes with user confirmation
- [ ] Add error recovery workflow
  - [ ] `lash lint --fix`: apply auto-fixes
  - [ ] `lash lint --suggest`: show fix suggestions
  - [ ] Iterate until no fixable errors remain
- [ ] Implement error diff
  - [ ] Show before/after for fixes
  - [ ] Highlight what changed
  - [ ] Confirm before applying

### Success Criteria

- Auto-fix successfully resolves common errors
- Suggestions are helpful for manual fixes
- Recovery workflow is smooth
- Users can quickly fix errors

### Tests

- Integration: Test auto-fix on various error types
- Integration: Test suggestion generation
- Integration: Verify fixes resolve errors

---

## Non-Goals (for v1)

- Error telemetry or crash reporting to external service
- Interactive error fixing wizard
- Error translations (English only for v1)
- Custom error messages per user

---

## Open Questions

- **Error display:** Stream errors as found or batch at end?
- **Severity:** Fail on warnings or only on errors?
- **JSON schema:** Version and document formally?
- **Help URLs:** Include links to docs or keep messages self-contained?

---

## References

- Design doc section 12 (Error Handling & UX)
- `miette` crate: https://docs.rs/miette/ (for rich diagnostics)
- `thiserror` crate: https://docs.rs/thiserror/ (for error derive macros)
- Error handling patterns: https://doc.rust-lang.org/book/ch09-00-error-handling.html
