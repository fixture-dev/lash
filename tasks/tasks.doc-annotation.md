# @doc Annotation Support

**Module:** Parsing & Validation
**Priority:** MEDIUM
**Estimated Duration:** 2-3 days
**Dependencies:** tasks.markdown-parser.md, tasks.linter.md

## Overview

Add support for the `@doc` annotation, which provides semantic linking to external documentation (design docs, requirements, specifications). Unlike `@depends-on` which expresses blocking dependencies, `@doc` indicates "read this for context"—informational, non-blocking references.

This enables lean task files that link to richer context on demand, supporting both human developers and agents who can selectively fetch documentation when needed.

## Format

```markdown
@doc: ../docs/design-doc.md
@doc: ../docs/design-doc.md#section-7.2
@doc: requirements/auth-spec.md
```

- Value is a validated relative path (must exist)
- Optional fragment identifier for section targeting
- Multiple `@doc` annotations allowed per file/task
- Can appear at file level (header) or task level (inline metadata)

## Tasks

### 1. Add @doc to Built-in Annotation Keys

- [ ] Update annotation key whitelist
  - [ ] Add "doc" to built-in keys in parser
  - [ ] Update `is_annotation_allowed()` in `LintContext`
- [ ] Define `DocRef` struct
  - [ ] `path: PathBuf` - relative path to document
  - [ ] `fragment: Option<String>` - optional `#section` fragment
  - [ ] `line_num: usize` - source location for error reporting
- [ ] Implement `DocRef::parse(value: &str) -> Result<DocRef>`
  - [ ] Split on `#` to extract path and fragment
  - [ ] Validate path format (no absolute paths, no `..` escaping project root)
  - [ ] Store fragment if present

**Priority:** HIGH
**Estimate:** 0.5 days
**Dependencies:** None
**Success Criteria:** Parser accepts `@doc` annotations without "unknown key" errors

### Tests

- [ ] Unit: Parse `@doc: path/to/file.md`
- [ ] Unit: Parse `@doc: path/to/file.md#section`
- [ ] Unit: Reject absolute paths
- [ ] Unit: Multiple `@doc` annotations accumulate

---

### 2. Add DocRef to Data Model

- [ ] Add `docs: Vec<DocRef>` field to `TaskFile` struct
  - [ ] Populated from file-level `@doc` annotations
- [ ] Add `docs: Vec<DocRef>` field to `Task` struct
  - [ ] Populated from task-level inline `@doc` annotations
- [ ] Update `AnnotationBlock` helper methods
  - [ ] Add `get_docs() -> Vec<DocRef>` method
- [ ] Update parser to populate doc refs
  - [ ] Extract from header annotations
  - [ ] Extract from task metadata blocks

**Priority:** HIGH
**Estimate:** 0.5 days
**Dependencies:** Task #1
**Success Criteria:** Parsed files contain populated `docs` vectors

### Tests

- [ ] Unit: File-level `@doc` populates `TaskFile.docs`
- [ ] Unit: Task-level `@doc` populates `Task.docs`
- [ ] Unit: Multiple docs at both levels

---

### 3. Implement Linter Rule for Doc References

- [ ] Create `ValidDocReference` rule
  - [ ] Code: `E_SEM_INVALID_DOC`
  - [ ] Severity: Error
  - [ ] Check: Referenced file exists on filesystem
  - [ ] Check: Path is within project root (no escaping via `../../../`)
- [ ] Create `BrokenDocFragment` rule (optional, lower priority)
  - [ ] Code: `W_SEM_DOC_FRAGMENT`
  - [ ] Severity: Warning
  - [ ] Check: If fragment specified, file contains matching heading
  - [ ] Note: Requires parsing target file for headings
- [ ] Add rules to default registry

**Priority:** HIGH
**Estimate:** 1 day
**Dependencies:** Task #2
**Success Criteria:** Linter catches broken doc references with clear error messages

### Tests

- [ ] Unit: Valid doc reference passes
- [ ] Unit: Missing file fails with `E_SEM_INVALID_DOC`
- [ ] Unit: Path escaping project root fails
- [ ] Unit: Broken fragment warns (if implemented)

---

### 4. Add SQLite Storage for Doc References

- [ ] Add `doc_refs` table to schema
  - [ ] `id: INTEGER PRIMARY KEY`
  - [ ] `source_file_id: INTEGER` - FK to files table
  - [ ] `source_task_id: INTEGER NULL` - FK to tasks table (NULL for file-level)
  - [ ] `target_path: TEXT` - relative path to document
  - [ ] `fragment: TEXT NULL` - optional section fragment
- [ ] Create `DocRefRepository`
  - [ ] `insert(doc_ref: &DocRef, file_id: i64, task_id: Option<i64>)`
  - [ ] `find_by_file(file_id: i64) -> Vec<DocRef>`
  - [ ] `find_by_task(task_id: i64) -> Vec<DocRef>`
  - [ ] `find_by_target(path: &Path) -> Vec<(FileId, Option<TaskId>)>` - reverse lookup
- [ ] Update indexing engine to persist doc refs

**Priority:** MEDIUM
**Estimate:** 0.5 days
**Dependencies:** Task #2, tasks.sqlite-schema.md
**Success Criteria:** Doc references queryable from database

### Tests

- [ ] Unit: Insert and retrieve doc refs
- [ ] Unit: Reverse lookup by target path
- [ ] Integration: Index file with docs, query back

---

### 5. Add CLI Support for Doc References

- [ ] Update `lash show` command
  - [ ] Display doc references in output
  - [ ] Format: `Docs: ../docs/design.md#section-7`
- [ ] Add `--docs` filter to `lash list`
  - [ ] `lash list --docs design.md` - find tasks referencing this doc
- [ ] Update `lash agent-prompt` output
  - [ ] Include doc refs in sparse context
  - [ ] Enable agents to request doc content on demand

**Priority:** LOW
**Estimate:** 0.5 days
**Dependencies:** Task #4, tasks.cli-commands.md
**Success Criteria:** Doc references visible and queryable via CLI

### Tests

- [ ] Integration: `lash show` displays docs
- [ ] Integration: `lash list --docs` filters correctly

---

## Summary

**Total Estimate:** 2-3 days
**Critical Path:** Tasks #1 → #2 → #3 (parsing and validation)

### Completion Criteria

- [ ] `@doc` accepted as valid annotation
- [ ] Doc refs stored in data model and database
- [ ] Linter validates doc references exist
- [ ] CLI displays doc references

### Non-Goals (v1)

- Automatic doc content fetching/embedding
- Fragment validation for non-Markdown files
- Bidirectional doc-to-task linking UI

## References

- Design doc section 4.3 (annotation types)
- `lash-core/src/parser/annotations.rs` - existing annotation parsing
- `lash-core/src/linter/rules/` - existing linter rules
