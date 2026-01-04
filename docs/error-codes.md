# Lash Error Codes

This document describes all error codes used by Lash, organized by category.

## Parse Errors (E_PARSE_*)

### E_PARSE_INVALID_CHECKBOX

**Description:** Invalid checkbox syntax in task list

**Example:**
```markdown
- [?] Invalid checkbox marker
```

**How to fix:** Use only valid checkbox markers: `[ ]` (open), `[x]` (done), `[-]` (waived), or `[!]` (blocked)

---

### E_PARSE_INVALID_ANNOTATION

**Description:** Annotation value has invalid format

**Example:**
```markdown
@created: not-a-date
```

**How to fix:** Ensure annotation values match expected formats (dates as YYYY-MM-DD, etc.)

---

### E_PARSE_INVALID_HEADER

**Description:** Heading structure is malformed

**Example:**
```markdown
##No space after hash
```

**How to fix:** Add space after `#` markers: `## Heading`

---

### E_PARSE_UNEXPECTED_DEPTH

**Description:** Task appears at an unexpected depth level

**Example:**
```markdown
- [ ] Parent
      - [ ] Child (too many spaces)
```

**How to fix:** Ensure consistent indentation using the configured indent size (default: 2 spaces)

---

### E_PARSE_INVALID_DATE

**Description:** Date string cannot be parsed

**Example:**
```markdown
@created: 2024-13-45
```

**How to fix:** Use valid date format: YYYY-MM-DD (e.g., `2024-01-15`)

---

## Lint Errors (E_LINT_*)

### E_LINT_DEPTH_EXCEEDED

**Description:** Task nesting exceeds maximum allowed depth

**Example:**
```markdown
- [ ] Level 1
  - [ ] Level 2
    - [ ] Level 3
      - [ ] Level 4 (exceeds default max depth of 3)
```

**How to fix:** Reduce nesting depth to 3 levels or adjust `max_depth` in config

---

### E_LINT_DUPLICATE_ID

**Description:** Multiple files or sections use the same `@id`

**Example:**
```markdown
File 1: @id: my.task
File 2: @id: my.task  # Duplicate!
```

**How to fix:** Ensure all `@id` values are unique across the project

---

### E_LINT_MISSING_ANNOTATION

**Description:** Task file missing required annotation (such as `@id`)

**Example:**
```markdown
# My Tasks
## Tasks
- [ ] Task without ID
```

**How to fix:** Add the required annotation, e.g., `@id: unique.identifier` to file metadata

---

### E_LINT_STATUS_INCONSISTENCY

**Description:** Parent task marked as done but has incomplete child tasks

**Example:**
```markdown
## Tasks
- [x] Parent task
  - [ ] Incomplete child task
```

**How to fix:** Mark all child tasks as done or waived, or change parent status to open

---

### E_LINT_INVALID_LABEL

**Description:** Label format is invalid

**Example:**
```markdown
@labels: valid-label, invalid label!
```

**How to fix:** Labels must be alphanumeric with hyphens, no spaces or special characters

---

### E_LINT_UNKNOWN_ANNOTATION

**Description:** Annotation key is not recognized

**Example:**
```markdown
@invalid-field: value
```

**How to fix:** Remove unknown annotation or check spelling. Valid annotations: `@id`, `@labels`, `@owner`, `@created`, `@estimate`, `@depends-on`, `@agent-note`

---

### E_LINT_BAD_INDENTATION

**Description:** Task indentation doesn't match configured indent size

**Example:**
```markdown
- [ ] Parent
   - [ ] Child (3 spaces instead of 2)
```

**How to fix:** Use consistent indentation matching `indent_spaces` config (default: 2)

---

## Dependency Errors (E_DEP_*)

### E_DEP_NOT_FOUND

**Description:** Referenced task or file does not exist

**Example:**
```markdown
@depends-on: nonexistent/file.md#task:missing
```

**How to fix:** Verify the dependency path and task ID exist

---

### E_DEP_CYCLE

**Description:** Circular dependency detected

**Example:**
```
Task A depends on Task B
Task B depends on Task C
Task C depends on Task A  # Cycle!
```

**How to fix:** Remove circular dependencies to create a valid dependency graph

---

### E_DEP_INVALID_REF

**Description:** Dependency reference format is invalid

**Example:**
```markdown
@depends-on: bad-format
```

**How to fix:** Use format: `path/to/file.md#task:id` or `#task:id` for same-file references

---

## I/O Errors (E_IO_*)

### E_IO_FILE_NOT_FOUND

**Description:** File or directory does not exist

**Example:**
Attempting to read `/path/to/missing.md`

**How to fix:** Verify the file path is correct and the file exists

---

### E_IO_READ_ERROR

**Description:** File cannot be read

**Example:**
Permission denied when reading file

**How to fix:** Check file permissions and ensure the process has read access

---

### E_IO_WRITE_ERROR

**Description:** File cannot be written

**Example:**
Disk full or permission denied when writing

**How to fix:** Check disk space and file/directory permissions

---

### E_IO_PERMISSION_DENIED

**Description:** Insufficient permissions for file operation

**Example:**
Attempting to write to read-only file

**How to fix:** Adjust file permissions or run with appropriate privileges

---

### E_IO_INVALID_PATH

**Description:** File path is invalid or malformed

**Example:**
Path contains invalid characters or is not a valid UTF-8 string

**How to fix:** Ensure path is valid and uses correct path separators for your OS

---

## Index Errors (E_INDEX_*)

### E_INDEX_CORRUPTED

**Description:** Database index is corrupted or cannot be accessed

**Example:**
Database file is locked, corrupted, or query failed

**How to fix:** Delete `.lash/lash.db` and rebuild index with `lash index --force`

---

### E_INDEX_VERSION_MISMATCH

**Description:** Database schema version doesn't match current Lash version

**Example:**
Opening a database created by a different Lash version

**How to fix:** Rebuild database with `lash index --force` to migrate to current schema

---

### E_INDEX_OUT_OF_SYNC

**Description:** Database index is out of sync with Markdown files

**Example:**
Running `lash check-index` shows differences between DB and files

**How to fix:** Run `lash index` to synchronize the database with current files

---

## Query Errors (E_QUERY_*)

### E_QUERY_INVALID_SYNTAX

**Description:** Search query has invalid syntax

**Example:**
```bash
lash search "unclosed quote
```

**How to fix:** Check query syntax; ensure quotes are balanced and operators are valid

---

### E_QUERY_NO_RESULTS

**Description:** Query returned no matching results

**Example:**
```bash
lash search "nonexistent-task-xyz"
```

**How to fix:** Broaden search terms or check that tasks exist in the index

---

## Configuration Errors (E_CONFIG_*)

### E_CONFIG_ROOT_NOT_FOUND

**Description:** No Lash project root found

**Example:**
Running `lash` command outside of a Lash project

**How to fix:** Run from within a Lash project (containing `lash.index.md` or `.lash/` directory) or initialize with `lash init`

---

### E_CONFIG_INVALID_VALUE

**Description:** Configuration value is invalid

**Example:**
```toml
max_depth = 10  # Must be 2-5
```

**How to fix:** Use valid configuration values as documented

---

### E_CONFIG_PARSE_ERROR

**Description:** Configuration file cannot be parsed

**Example:**
Invalid TOML syntax in `.lash/config.toml`

**How to fix:** Fix TOML syntax errors in configuration file

---

### E_CONFIG_MISSING_INDEX

**Description:** Project root index file is missing

**Example:**
`.lash/` directory exists but no `lash.index.md` file

**How to fix:** Create `lash.index.md` at project root

---

## Creation Errors (E_CREATE_*)

### E_CREATE_EMPTY_TITLE

**Description:** Task title is empty or whitespace-only

**Example:**
```bash
lash add ""
```

**How to fix:** Provide a non-empty title for the task

---

### E_CREATE_TITLE_TOO_LONG

**Description:** Task title exceeds maximum allowed length (default 256 characters)

**Example:**
```bash
lash add "Very long title that exceeds the limit..."
```

**How to fix:** Shorten the title to the maximum allowed characters

---

### E_CREATE_FILE_NOT_FOUND

**Description:** Target file specified with `--file` does not exist and auto-creation is not enabled

**Example:**
```bash
lash add "Task" --file nonexistent.md
```

**How to fix:** Create the file first, or use `--file` which creates automatically if the file doesn't exist

---

### E_CREATE_FILE_NOT_WRITABLE

**Description:** Target file exists but cannot be written to

**Example:**
Attempting to add a task to a read-only file

**How to fix:** Check file permissions and ensure the file is writable

---

### E_CREATE_FILE_PARSE_FAILED

**Description:** Target file exists but failed to parse as a valid Lash task file

**Example:**
```bash
lash add "Task" --file malformed.md
```

**How to fix:** Run `lash lint <file>` to identify and fix parsing errors

---

### E_CREATE_PARENT_NOT_FOUND

**Description:** Specified parent task ID does not exist in the target file

**Example:**
```bash
lash add "Subtask" --parent nonexistent-id
```

**How to fix:** Ensure the parent task exists, or omit `--parent` for a top-level task

---

### E_CREATE_DEPTH_LIMIT_EXCEEDED

**Description:** Creating the task would exceed the maximum nesting depth

**Example:**
Adding a subtask to a task already at maximum depth

**How to fix:** Choose a parent task at a shallower depth (default max is 3)

---

### E_CREATE_DUPLICATE_ID

**Description:** Specified task ID is already in use in the target file

**Example:**
```bash
lash add "Task" --id existing-id
```

**How to fix:** Choose a different ID, or omit `--id` for auto-generated ID

---

### E_CREATE_INVALID_ID_FORMAT

**Description:** Task ID format is invalid

**Example:**
```bash
lash add "Task" --id "invalid id!"
```

**How to fix:** Use only alphanumeric characters, hyphens, underscores, and colons

---

### E_CREATE_INVALID_LABEL

**Description:** Label format is invalid

**Example:**
```bash
lash add "Task" --label "bad label!"
```

**How to fix:** Labels must be alphanumeric with hyphens, no spaces or special characters

---

### E_CREATE_INVALID_ESTIMATE

**Description:** Time estimate format is invalid

**Example:**
```bash
lash add "Task" --estimate "invalid"
```

**How to fix:** Use format like `30m`, `2h`, `1d`, `2w`

---

### E_CREATE_DEPENDENCY_NOT_FOUND

**Description:** Specified dependency target does not exist

**Example:**
```bash
lash add "Task" --depends-on "path/to/nonexistent.md#task:id"
```

**How to fix:** Ensure the referenced task exists before adding the dependency

---

### E_CREATE_WOULD_CREATE_CYCLE

**Description:** Creating the task with specified dependencies would create a circular dependency

**Example:**
Task A depends on B, B depends on A

**How to fix:** Remove the cyclic dependency or restructure the task hierarchy

---

### E_CREATE_INVALID_POSITION

**Description:** Specified insert position is invalid

**Example:**
```bash
lash add "Task" --before nonexistent-task
```

**How to fix:** Use a valid task ID for `--before` or `--after` position

---

### E_CREATE_IO_ERROR

**Description:** I/O error occurred during file operations

**Example:**
Disk full when trying to write the task

**How to fix:** Check disk space and file permissions

---

## Exit Codes

Lash uses the following exit codes:

- `0` - Success
- `1` - General error
- `2` - Parse/lint errors
- `3` - Dependency errors
- `4` - I/O errors
- `5` - Database errors
- `6` - Configuration errors
