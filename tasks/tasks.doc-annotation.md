# @doc Annotation Support

**Module:** Parsing & Validation
**Priority:** MEDIUM
**Estimated Duration:** 2-3 days
**Dependencies:** tasks.markdown-parser.md, tasks.linter.md
**Status:** COMPLETE ✅

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

- [x] Update annotation key whitelist
  - [x] Add "doc" to built-in keys in parser
  - [x] Update `is_annotation_allowed()` in `LintContext`
- [x] Define `DocRef` struct
  - [x] `path: String` - relative path to document
  - [x] `fragment: Option<String>` - optional `#section` fragment
- [x] Implement `DocRef::parse(value: &str) -> Result<DocRef>`
  - [x] Split on `#` to extract path and fragment
  - [x] Validate path format (no empty paths)
  - [x] Store fragment if present

**Priority:** HIGH
**Estimate:** 0.5 days
**Dependencies:** None
**Success Criteria:** Parser accepts `@doc` annotations without "unknown key" errors

### Tests

- [x] Unit: Parse `@doc: path/to/file.md`
- [x] Unit: Parse `@doc: path/to/file.md#section`
- [x] Unit: Reject empty paths
- [x] Unit: Multiple `@doc` annotations accumulate

---

### 2. Add DocRef to Data Model

- [x] Add `docs: Vec<DocRef>` field to `FileMetadata` struct
  - [x] Populated from file-level `@doc` annotations
- [x] Add `docs: Vec<DocRef>` field to `TaskMetadata` struct
  - [x] Populated from task-level `@doc` annotations
- [x] Update `AnnotationBlock` helper methods
  - [x] Add `get_docs() -> Vec<DocRef>` method
- [x] Update parser to populate doc refs
  - [x] Extract from header annotations
  - [x] Extract from task metadata blocks

**Priority:** HIGH
**Estimate:** 0.5 days
**Dependencies:** Task #1
**Success Criteria:** Parsed files contain populated `docs` vectors

### Tests

- [x] Unit: File-level `@doc` populates `FileMetadata.docs`
- [x] Unit: Task-level `@doc` populates `TaskMetadata.docs`
- [x] Unit: Multiple docs at both levels

---

### 3. Implement Linter Rule for Doc References

- [x] Create `ValidDocReferenceRule`
  - [x] Code: `E_SEM_INVALID_DOC`
  - [x] Severity: Error
  - [x] Check: Referenced file exists on filesystem
  - [x] Check: Path is within project root (no escaping via `../../../`)
  - [x] Check: Reject absolute paths
- [x] Create `BrokenDocFragment` rule
  - [x] Code: `W_SEM_DOC_FRAGMENT`
  - [x] Severity: Warning
  - [x] Check: If fragment specified, file contains matching heading
  - [x] Parse target file for headings using pulldown-cmark
- [x] Add rules to default registry

**Priority:** HIGH
**Estimate:** 1 day
**Dependencies:** Task #2
**Success Criteria:** Linter catches broken doc references with clear error messages

### Tests

- [x] Unit: Valid doc reference passes
- [x] Unit: Missing file fails with `E_SEM_INVALID_DOC`
- [x] Unit: Path escaping project root fails
- [x] Unit: Absolute path rejected
- [x] Unit: Fragment validation passes when heading exists
- [x] Unit: Fragment validation fails with `W_SEM_DOC_FRAGMENT` for missing heading
- [x] Unit: Case-insensitive fragment matching
- [x] Unit: All heading levels (H1-H6) are detected

---

### 4. Add SQLite Storage for Doc References

- [x] Add `doc_refs` table to schema (migration v3)
  - [x] `id: INTEGER PRIMARY KEY`
  - [x] `source_file_id: INTEGER` - FK to files table
  - [x] `source_task_id: INTEGER NULL` - FK to tasks table (NULL for file-level)
  - [x] `target_path: TEXT` - relative path to document
  - [x] `fragment: TEXT NULL` - optional section fragment
- [x] Create `DocRefRepository`
  - [x] `insert(doc_ref: &DocRef, file_id: i64, task_id: Option<i64>)`
  - [x] `find_by_file(file_id: i64) -> Vec<DocRefRow>`
  - [x] `find_by_task(task_id: i64) -> Vec<DocRefRow>`
  - [x] `find_by_target(path: &str) -> Vec<(i64, Option<i64>)>` - reverse lookup
  - [x] `find_by_target_prefix(prefix: &str)` - prefix matching
- [x] Update indexing engine to persist doc refs

**Priority:** MEDIUM
**Estimate:** 0.5 days
**Dependencies:** Task #2, tasks.sqlite-schema.md
**Success Criteria:** Doc references queryable from database

### Tests

- [x] Unit: Insert and retrieve doc refs
- [x] Unit: Reverse lookup by target path
- [x] Integration: Index file with docs, query back

---

### 5. Add CLI Support for Doc References

- [x] Update `lash show` command
  - [x] Display doc references in output
  - [x] Format: `Docs: ../docs/design.md#section-7`
- [x] Add `--docs` filter to `lash list`
  - [x] `lash list --docs design.md` - find tasks referencing this doc
- [-] Update `lash agent-prompt` output (deferred - low priority)
  - [-] Include doc refs in sparse context
  - [-] Enable agents to request doc content on demand
  - Note: Doc refs are available in DB for future enhancement

**Priority:** LOW
**Estimate:** 0.5 days
**Dependencies:** Task #4, tasks.cli-commands.md
**Success Criteria:** Doc references visible and queryable via CLI

### Tests

- [x] Integration: `lash show` displays docs
- [x] Integration: `lash list --docs` filters correctly

---

## Summary

**Total Estimate:** 2-3 days
**Critical Path:** Tasks #1 → #2 → #3 (parsing and validation)

### Completion Criteria

- [x] `@doc` accepted as valid annotation
- [x] Doc refs stored in data model and database
- [x] Linter validates doc references exist
- [x] CLI displays doc references

### Non-Goals (v1)

- Automatic doc content fetching/embedding
- ~~Fragment validation for non-Markdown files~~ (Now implemented for Markdown files)
- Bidirectional doc-to-task linking UI

## References

- Design doc section 4.3 (annotation types)
- `lash-core/src/parser/annotations.rs` - existing annotation parsing
- `lash-core/src/linter/rules/` - existing linter rules

## Implementation Notes

Completed in commits:
- `56ade71` - Add @doc annotation task for semantic documentation links (Tasks 1-2)
- `3dad6df` - Add CLI support for @doc annotation references (Tasks 3-5)
- `730f164` - Fix database initialization to include all migrations
- `2c0f9de` - Add BrokenDocFragmentRule for validating @doc fragment references
