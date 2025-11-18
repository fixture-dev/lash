# Lash Error Codes

This document describes all error codes used by Lash, organized by category.

## Parse Errors (E_PARSE_*)

### E_PARSE_BAD_CHECKBOX

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

### E_PARSE_MALFORMED_HEADING

**Description:** Heading structure is malformed

**Example:**
```markdown
##No space after hash
```

**How to fix:** Add space after `#` markers: `## Heading`

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

### E_LINT_MISSING_ID

**Description:** Task file missing required `@id` annotation

**Example:**
```markdown
# My Tasks
## Tasks
- [ ] Task without ID
```

**How to fix:** Add `@id: unique.identifier` annotation to file metadata

---

### E_LINT_INVALID_STATUS

**Description:** `@status` value is not recognized

**Example:**
```markdown
@status: in-flight  # Invalid
```

**How to fix:** Use valid status values: `open`, `in-progress`, `blocked`, `done`

---

### E_LINT_UNKNOWN_ANNOTATION

**Description:** Annotation key is not recognized

**Example:**
```markdown
@invalid-field: value
```

**How to fix:** Remove unknown annotation or check spelling. Valid annotations: `@id`, `@status`, `@labels`, `@owner`, `@created`, `@estimate`, `@depends-on`, `@agent-note`

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

## Database Errors (E_DB_*)

### E_DB_CONNECTION

**Description:** Cannot connect to or open database

**Example:**
Database file is locked or corrupted

**How to fix:** Close other connections to the database or rebuild index with `lash index`

---

### E_DB_QUERY

**Description:** Database query failed

**Example:**
SQL syntax error or constraint violation

**How to fix:** Rebuild the database index with `lash index`

---

### E_DB_CONSTRAINT

**Description:** Database constraint violated

**Example:**
Duplicate key insertion

**How to fix:** Resolve conflicts in task data and rebuild index

---

### E_DB_MIGRATION

**Description:** Database schema migration failed

**Example:**
Incompatible schema version

**How to fix:** Backup data and rebuild database with current version

---

## Configuration Errors (E_CFG_*)

### E_CFG_ROOT_NOT_FOUND

**Description:** No Lash project root found

**Example:**
Running `lash` command outside of a Lash project

**How to fix:** Run from within a Lash project (containing `lash.index.md` or `.lash/` directory) or initialize with `lash init`

---

### E_CFG_INVALID_VALUE

**Description:** Configuration value is invalid

**Example:**
```toml
max_depth = 10  # Must be 2-5
```

**How to fix:** Use valid configuration values as documented

---

### E_CFG_PARSE_ERROR

**Description:** Configuration file cannot be parsed

**Example:**
Invalid TOML syntax in `.lash/config.toml`

**How to fix:** Fix TOML syntax errors in configuration file

---

### E_CFG_MISSING_INDEX

**Description:** Project root index file is missing

**Example:**
`.lash/` directory exists but no `lash.index.md` file

**How to fix:** Create `lash.index.md` at project root

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
