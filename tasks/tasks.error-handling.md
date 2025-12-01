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

- [x] Define `LashError` enum
  - [x] Top-level error categories:
    - [x] `Parse` - Markdown parsing errors
    - [x] `Lint` - Linting/validation errors
    - [x] `Index` - Indexing/database errors
    - [x] `Dependency` - Dependency resolution errors
    - [x] `Query` - Query/search errors
    - [x] `Config` - Configuration errors
    - [x] `IO` - File system errors
    - [x] `Internal` - Internal/unexpected errors
  - [x] Each variant contains specific error details
- [x] Define error codes
  - [x] Stable string codes (e.g., `E_PARSE_INVALID_CHECKBOX`, `E_LINT_DUPLICATE_ID`)
  - [x] Hierarchical naming: `E_<CATEGORY>_<DESCRIPTION>`
  - [x] Document all codes in error catalog
- [x] Implement error variants
  - [x] **Parse errors:**
    - [x] Invalid checkbox syntax
    - [x] Malformed annotation
    - [x] Invalid header format
    - [x] Unexpected depth
  - [x] **Lint errors:**
    - [x] Duplicate ID
    - [x] Unknown annotation
    - [x] Depth limit exceeded
    - [x] Status inconsistency (parent done, children open)
    - [x] Invalid label format
  - [x] **Dependency errors:**
    - [x] Broken reference (target not found)
    - [x] Circular dependency
    - [x] Invalid reference format
  - [x] **Index errors:**
    - [x] Database corruption
    - [x] Schema version mismatch
    - [x] Index out of sync
  - [x] **IO errors:**
    - [x] File not found
    - [x] Permission denied
    - [x] Invalid path
- [x] Add error context
  - [x] File path
  - [x] Line and column numbers
  - [x] Relevant snippet (for parse/lint errors)
  - [x] Dependency chain (for dependency errors)
- [x] Implement `std::error::Error` trait
  - [x] Human-readable `Display` implementation
  - [x] Proper error implementation via thiserror
  - [x] `Debug` implementation with full context

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

- [x] Implement `ErrorFormatter` struct
  - [x] `format_human()` - human-readable text
  - [x] `format_json()` - structured JSON
  - [x] Support for colored output
- [x] Implement human-readable formatting
  - [x] Use colored crate for color support
  - [x] Show:
    - [x] Error message (clear, concise)
    - [x] File path, line, column
    - [x] Code snippet with context lines
    - [x] Caret (^) pointing to error location
    - [x] Help text / suggestion
  - [x] Color coding:
    - [x] Red: error message
    - [x] Cyan: file path
    - [x] Yellow: warning text
    - [x] Gray: code snippet
  - [x] Implements rich formatting similar to rustc errors
- [x] Implement JSON formatting
  - [x] Schema includes:
    - code, severity, message, location, snippet, help, labels
  - [x] Include all context fields
  - [x] Stable schema via Diagnostic struct
- [x] Implement error suggestions
  - [x] Contextual help for each error type
  - [x] Helper constructors include helpful messages
  - [x] All errors have actionable help text
- [x] Add severity levels
  - [x] Error (must fix)
  - [x] Warning (should fix)
  - [x] Info (nice to fix)
  - [x] Hint (style suggestion)

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

- [x] Define `ErrorReport` struct
  - [x] List of errors
  - [x] Grouping (by file, by type, by severity, or none)
  - [x] Summary statistics
- [x] Implement error collection
  - [x] Collect all errors (don't stop on first)
  - [x] Associate with source file/operation
  - [x] Maintain order (by file, then line)
- [x] Implement error grouping
  - [x] Group by file (show all errors per file together)
  - [x] Group by error code (show all duplicates together)
  - [x] Group by severity
  - [x] Configurable grouping strategy via GroupBy enum
- [x] Implement summary reporting
  - [x] Count errors by severity
  - [x] Count errors by type (error code)
  - [x] Show affected files count
  - [x] Summary includes error counts and breakdown
- [x] Implement report rendering
  - [x] Text format: grouped errors with headers
  - [x] JSON format: array of error objects + summary
  - [x] Compact format for logs
- [x] Add filtering
  - [x] Filter by severity
  - [x] Filter by file
  - [x] Filter by error code

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

- [x] Add structured error context for agents
  - [x] Exact error location (file, line, col)
  - [x] Error code (for matching against documentation)
  - [x] Actionable fix suggestion (specific steps)
  - [x] Related context (affected dependencies, etc.)
- [x] Implement error recovery hints
  - [x] "Run `lash format` to fix formatting"
  - [x] "Remove duplicate ID or rename to unique value"
  - [x] "Update reference to: `correct/path.md#task:id`"
- [x] Add error documentation links
  - [x] URL to error code documentation (if available)
  - [x] Inline explanation in JSON output
- [x] Implement `--explain` flag (optional)
  - [x] `lash explain E_LINT_DUPLICATE_ID`
  - [x] Show detailed explanation of error code
  - [x] Show examples and fixes
- [x] Ensure JSON errors are complete
  - [x] All fields populated
  - [x] No ambiguous or vague messages
  - [x] Include context needed for automated fixes

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

- [x] Implement top-level error handler
  - [x] Catch `LashError` from commands
  - [x] Format based on output mode (text/JSON)
  - [x] Display to user
  - [x] Set exit code
- [x] Add error reporting to each command
  - [x] `lint`: collect all errors, show report
  - [x] `format`: report formatting failures
  - [x] `index`: report parse and DB errors
  - [x] `list`: report query errors
  - [x] `show`: report not found errors
  - [x] `search`: report search errors
  - [x] `graph`: report dependency errors
  - [x] `check-links`: report broken links
- [x] Implement progress-aware error display
  - [x] Show errors as they occur (streaming)
  - [x] Or collect and show at end (batch)
  - [x] Configurable via flag
- [x] Add error verbosity control
  - [x] Quiet: only error count
  - [x] Normal: error messages (default)
  - [x] Verbose: error messages + context
  - [x] Debug: full error details + backtrace

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

- [x] Implement `ErrorValidator`
  - [x] Validate that a file has no errors after fix
  - [x] Re-run lint after auto-fix
  - [x] Report remaining errors
- [x] Implement auto-fix suggestions
  - [x] Detect fixable errors (whitespace, formatting)
  - [x] Generate fix suggestions
  - [x] Apply fixes with user confirmation
- [x] Add error recovery workflow
  - [x] `lash lint --fix`: apply auto-fixes
  - [x] `lash lint --suggest`: show fix suggestions
  - [x] Iterate until no fixable errors remain
- [x] Implement error diff
  - [x] Show before/after for fixes
  - [x] Highlight what changed
  - [x] Confirm before applying

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
