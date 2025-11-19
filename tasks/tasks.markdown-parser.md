# Markdown Parser Tasks

**Module:** Parsing & Validation
**Priority:** CRITICAL
**Estimated Duration:** 7-9 days
**Dependencies:** tasks.core-data-model (all tasks)

## Overview

Implement the Markdown parser that transforms raw `.md` files into the core data structures (`TaskFile`, `Task`, etc.). The parser must be robust, performant, and produce helpful error messages.

**Design Approach:** Use `pulldown-cmark` for Markdown parsing, then walk the event stream to build our task structures. See `docs/rust-architecture-recommendations.md` for details.

**Performance Target:** <100ms for typical files (for pre-commit hooks)

## Tasks

### 1. Design Parser Architecture

- [x] **Choose parsing library**
  - [x] Add `pulldown-cmark` dependency to `lash-core`
  - [x] Evaluate streaming vs tree-based parsing (choose streaming)
  - [x] Document decision in parser module
- [x] **Define parser module structure**
  - [x] Create `lash-core/src/parser/mod.rs`
  - [x] Create `lash-core/src/parser/events.rs` - event stream processing
  - [x] Create `lash-core/src/parser/checkbox.rs` - checkbox line parsing
  - [x] Create `lash-core/src/parser/annotations.rs` - annotation parsing
  - [x] Create `lash-core/src/parser/builder.rs` - task tree builder
- [x] **Define `ParseContext` struct**
  - [x] Fields:
    - [x] `file_path: PathBuf` - Current file being parsed
    - [x] `current_line: usize` - Line number tracking
    - [x] `current_section: Section` - Header, Tasks, References, etc.
    - [x] `errors: Vec<Diagnostic>` - Accumulated errors
    - [x] `config: &LashConfig` - Parser configuration
  - [x] Methods for error reporting with location context
- [x] **Define parsing result types**
  - [x] `ParseResult<T> = Result<T, Vec<Diagnostic>>` - can have multiple errors
  - [x] `ParsedFile` struct - intermediate representation before `TaskFile`
- [x] **Document parser architecture**
  - [x] Create `lash-core/src/parser/README.md`
  - [x] Document parsing phases: events → lines → structure → validation
  - [x] Document error handling strategy
  - [x] Add architecture diagram (ASCII art)

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** tasks.core-data-model#2
**Success Criteria:** Architecture documented; module structure created; ready to implement

---

### 2. Implement Checkbox Line Parser

- [x] **Parse checkbox pattern**
  - [x] Create `parse_checkbox_line(line: &str, line_num: usize) -> Result<CheckboxLine>`
  - [x] Detect indentation (count leading spaces)
  - [x] Enforce indentation is multiple of 2 (per design decisions)
  - [x] Parse `- [STATUS]` pattern
  - [x] Extract status character (space, x, -, !)
  - [x] Handle both lowercase 'x' and uppercase 'X'
  - [x] Return error for invalid checkbox patterns
- [x] **Extract task title**
  - [x] Get text after checkbox to end of line
  - [x] Trim leading/trailing whitespace
  - [x] Validate title not empty
  - [-] Handle multiline titles (continuation lines) - not part of current spec
- [x] **Parse inline labels**
  - [x] Scan title for `#word` patterns
  - [x] Extract all hashtag labels
  - [x] Use `Label::parse_inline_labels()` from core-data-model
  - [x] Keep labels in title or remove? (design decision: keep)
- [x] **Parse trailing metadata blocks**
  - [x] Detect `[@key: value, @key2: value2]` pattern
  - [x] Extract key-value pairs
  - [x] Validate bracket matching
  - [-] Parse individual annotations within brackets - deferred to annotation parser (Task #3)
  - [-] Merge with inline labels - deferred to annotation parser (Task #3)
- [x] **Handle edge cases**
  - [x] Mixed tabs and spaces (error: must be spaces only)
  - [-] Inconsistent indentation jumps (error: can't skip levels) - validated in tree builder (Task #5)
  - [x] Empty titles
  - [x] Very long titles (>1000 chars warning) - handled, no artificial limit
  - [x] Invalid status characters
- [x] **Create intermediate representation**
  - [x] Define `CheckboxLine` struct:
    - [x] `indent: usize` - number of spaces
    - [x] `depth: u8` - computed nesting level (indent / 2)
    - [x] `status: TaskStatus`
    - [x] `title: String`
    - [x] `labels: Vec<Label>`
    - [-] `metadata: Option<TaskMetadata>` - not in current implementation, deferred
    - [x] `line_num: usize`
- [x] **Write comprehensive tests**
  - [x] Valid patterns: `- [ ]`, `- [x]`, `- [-]`, `- [!]`
  - [x] Uppercase X: `- [X]`
  - [x] With labels: `- [ ] Task #label1 #label2`
  - [x] With metadata: `- [ ] Task [@owner: alice]`
  - [x] Indentation levels: 0, 2, 4 spaces
  - [x] Error cases: invalid indent, bad checkbox, empty title
  - [x] Edge cases: mixed whitespace, weird characters
  - [x] 50+ test cases total (56 tests implemented)

**Priority:** CRITICAL
**Estimate:** 2 days
**Dependencies:** Task #1
**Success Criteria:** Can parse all valid checkbox variations; reports errors clearly with line numbers

---

### 3. Implement Annotation Parser

- [x] **Parse `@key: value` format**
  - [x] Create `parse_annotation(line: &str) -> Result<(String, String)>`
  - [x] Detect `@` at start (after trimming)
  - [x] Split on first `:` to get key and value
  - [x] Trim whitespace from both key and value
  - [x] Validate key format (alphanumeric + hyphen + underscore)
- [x] **Handle multiline values**
  - [x] Detect when value continues on next line (indented)
  - [x] Accumulate continuation lines
  - [x] Preserve internal newlines/formatting
  - [x] Trim trailing whitespace only
- [x] **Parse specific annotation types**
  - [x] `@id: string` - validate ID format
  - [x] `@labels: a, b, c` - split on commas, parse as Label list
  - [x] `@status: string` - validate against known statuses
  - [x] `@owner: string` - any string
  - [x] `@created: YYYY-MM-DD` - validate date format
  - [x] `@estimate: duration` - validate format (e.g., "2h", "3d")
  - [x] `@depends-on: ref` - parse as DependencyRef
  - [x] `@agent-note: text` - any text
- [x] **Validate known annotation keys**
  - [x] Load custom keys from config
  - [x] Check against built-in keys: id, labels, status, owner, created, estimate, depends-on, agent-note
  - [x] Check against custom keys from `.lash/config.toml`
  - [x] Error on unknown key not in either list
  - [x] Suggest adding to config if looks like custom key
- [x] **Parse value types**
  - [x] String values (default)
  - [x] Comma-separated lists (for labels, depends-on)
  - [x] Date values (YYYY-MM-DD format)
  - [x] Duration values (regex: `\d+[hdwmy]` for hours/days/weeks/months/years)
  - [x] Return structured errors for invalid formats
- [x] **Handle multiple `@depends-on` annotations**
  - [x] Allow multiple `@depends-on` lines
  - [x] Accumulate into Vec<DependencyRef>
  - [x] Each can be separate file or task reference
- [x] **Create annotation collection**
  - [x] Define `AnnotationBlock` struct:
    - [x] `annotations: HashMap<String, Vec<String>>` - allow multiple values per key
    - [x] Helper methods: `get_single()`, `get_list()`, `get_date()`, etc.
  - [x] Validate no duplicate single-value annotations
  - [x] Allow multiple values for `depends-on`, `labels`
- [x] **Write tests**
  - [x] Parse all built-in annotation types
  - [x] Parse custom annotations (with config)
  - [x] Multiline values
  - [x] Multiple depends-on
  - [x] Invalid key names
  - [x] Invalid value formats (dates, durations)
  - [x] Unknown annotations (error)
  - [x] 30+ test cases (51 tests implemented)

**Priority:** CRITICAL
**Estimate:** 1.5 days
**Dependencies:** Task #2
**Success Criteria:** Can parse all annotation types; validates keys against config; handles multiline values

---

### 4. Implement Header Block Parser

- [x] **Parse H1 title**
  - [x] Use pulldown-cmark to find first `Heading(1)` event
  - [x] Extract text content
  - [x] Store as file title
  - [x] Warning if no H1 found (synthesize from filename)
  - [x] Warning if multiple H1s found (use first)
- [x] **Extract header annotations**
  - [x] Parse lines between H1 and first H2
  - [x] Identify annotation lines (start with `@`)
  - [x] Skip blank lines
  - [x] Collect overview text (non-annotation, non-blank lines)
  - [x] Parse all annotations into `AnnotationBlock`
- [x] **Parse overview section**
  - [x] Collect all text between annotations and "## Tasks"
  - [x] Preserve paragraph structure
  - [x] Trim excessive whitespace
  - [x] Store as optional overview text
- [x] **Detect "## Tasks" section boundary**
  - [x] Find `Heading(2)` with text "Tasks"
  - [x] Case-insensitive comparison
  - [x] Set parser state to TasksSection
  - [x] Warning if no Tasks section found
- [x] **Parse optional "## References" section**
  - [x] Find `Heading(2)` with text "References"
  - [x] Collect content (markdown list, prose, etc.)
  - [x] Store as optional references text
  - [-] Parse depends-on links from references if present (deferred to Task #6)
- [x] **Handle malformed headers**
  - [x] Missing H1: synthesize from filename
  - [x] Missing Tasks section: treat whole file as tasks
  - [x] Missing annotations: use defaults
  - [x] Graceful degradation, emit warnings
- [x] **Create header representation**
  - [x] Define `ParsedHeader` struct:
    - [x] `title: String`
    - [x] `annotations: AnnotationBlock`
    - [x] `overview: Option<String>`
  - [-] Convert to `FileMetadata` from annotations (deferred to Task #6)
- [x] **Write tests**
  - [x] Complete valid header
  - [x] Minimal header (H1 + Tasks only)
  - [x] With overview text
  - [x] With references
  - [x] Multiple H1s (warning)
  - [x] No H1 (synthesize)
  - [x] No Tasks section (warning, treat all as tasks)
  - [x] 24 test cases (exceeds 20+ requirement)

**Priority:** HIGH
**Estimate:** 1 day
**Dependencies:** Task #3
**Success Criteria:** Can parse complete header block; handles missing sections gracefully

---

### 5. Implement Task Tree Builder

- [x] **Build parent-child relationships from indentation**
  - [x] Create `TaskTreeBuilder` struct
  - [x] Maintain stack of current parents at each depth
  - [x] For each checkbox line:
    - [x] Compute depth from indentation (indent / 2)
    - [x] Validate depth doesn't exceed max (3 levels = depth 0,1,2)
    - [x] Validate depth doesn't jump (can't go from 0 to 2)
    - [x] Pop stack to current depth
    - [x] Set parent as top of stack
    - [x] Push current task onto stack
- [x] **Validate depth limits during construction**
  - [x] Check `depth <= config.max_depth` (max_depth = 2 for 3 levels)
  - [x] Emit error with line number if exceeded
  - [-] Suggest splitting into separate file
- [x] **Handle malformed indentation**
  - [x] Detect inconsistent depth jumps (0 → 2 without 1)
  - [x] Emit error: "Cannot skip indentation levels"
  - [x] Provide suggestion: indent by 2 spaces
  - [-] Attempt recovery: treat as sibling of last task
- [x] **Compute task order indices**
  - [x] Assign sequential order_index to siblings
  - [x] Reset count for each parent
  - [x] Maintain document order for traversal
- [x] **Generate synthetic IDs for tasks without `@id`**
  - [x] Format: `task-{order}` or `{title-slug}`
  - [x] Use task title slug if unique (lowercase, hyphenated)
  - [x] Fall back to numeric index if title slugs collide
  - [x] Ensure IDs unique within file
- [x] **Validate ID uniqueness within file**
  - [x] Track all task IDs in HashSet
  - [x] Error on duplicate IDs
  - [x] Provide line numbers for both occurrences
  - [-] Suggest renaming one (error message is clear enough)
- [x] **Build task hierarchy**
  - [x] Create `Task` instances from `CheckboxLine` data
  - [x] Set parent_id based on indentation stack
  - [x] Set depth, order_index
  - [x] Collect all tasks into `TaskTree`
- [x] **Apply auto-waiving logic**
  - [x] If parent status is Waived, mark all children as Waived
  - [x] Traverse depth-first after tree construction
  - [-] Emit info diagnostic: "Auto-waived due to parent" (applied automatically, no diagnostic needed)
- [x] **Write comprehensive tests**
  - [x] Simple flat list (all depth 0)
  - [x] Two-level hierarchy (parent + children)
  - [x] Three-level hierarchy (max depth)
  - [x] Sibling tasks at same depth
  - [x] Multiple parent-child chains
  - [x] Depth limit exceeded (error)
  - [x] Skipped indentation level (error)
  - [x] Duplicate IDs (error)
  - [x] Synthetic ID generation
  - [x] Auto-waiving propagation
  - [x] 40+ test cases including edge cases (42 tests implemented)

**Priority:** CRITICAL
**Estimate:** 2 days
**Dependencies:** Task #2
**Success Criteria:** Builds correct hierarchical structure; validates constraints; generates IDs

---

### 6. Implement Full File Parser

- [x] **Create main `parse_file()` entry point**
  - [x] Signature: `pub fn parse_file(path: &Path, config: &LashConfig) -> ParseResult<TaskFile>`
  - [x] Read file content
  - [x] Create ParseContext
  - [x] Parse in phases: header → tasks → finalize
  - [x] Collect all errors
  - [x] Return TaskFile or aggregated errors
- [x] **Integrate all parsing phases**
  - [x] Phase 1: Parse header (H1, annotations, overview)
  - [x] Phase 2: Parse task list (checkboxes → tree)
  - [x] Phase 3: Parse references section
  - [x] Phase 4: Validate and build TaskFile
- [x] **Compute content hash**
  - [x] Use blake3 to hash file content
  - [x] Store in TaskFile.hash
  - [x] Used for incremental indexing (detect changes)
- [x] **Extract file metadata**
  - [x] Get mtime from filesystem
  - [x] Synthesize file ID from path if no @id
  - [x] Build FileMetadata from header annotations
  - [x] Validate metadata consistency
- [x] **Implement error collection**
  - [x] Continue parsing after errors when possible
  - [x] Collect all diagnostics in ParseContext
  - [x] Sort diagnostics by line number
  - [x] Return complete error list (don't stop at first error)
- [-] **Add performance optimization**
  - [-] Use arena allocation for temporary parsing structures
  - [-] Add `typed-arena` dependency
  - [-] Allocate CheckboxLine instances in arena during parse
  - [-] Convert to owned Task instances at end
  - [x] Target: <100ms for typical files
- [x] **Create convenience methods**
  - [x] `parse_file_from_string(content: &str, config: &LashConfig) -> ParseResult<TaskFile>`
  - [-] `parse_task_section(content: &str) -> ParseResult<Vec<Task>>` - for testing
  - [-] `validate_file(file: &TaskFile, config: &LashConfig) -> Vec<Diagnostic>` - post-parse validation
- [x] **Write integration tests**
  - [x] Complete valid file (all sections)
  - [x] Minimal valid file (H1 + tasks)
  - [x] File with multiple errors (collect all)
  - [x] Large file (100+ tasks) - performance test
  - [-] Files from fixtures/valid/ directory
  - [-] Files from fixtures/invalid/ directory (expect errors)
  - [x] Round-trip: parse → serialize → parse (preserve structure)
  - [x] 20+ integration tests
- [x] **Benchmark performance**
  - [x] Create benchmark suite using `criterion`
  - [x] Benchmark small file (10 tasks)
  - [x] Benchmark medium file (100 tasks)
  - [x] Benchmark large file (1000 tasks)
  - [x] Target: <100ms for small, <500ms for medium, <5s for large
  - [-] Profile hot paths if needed

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** Tasks #4, #5
**Success Criteria:** Can parse complete files; reports all errors; meets performance targets

---

## Summary

### Total Estimate
**7-9 days** total for Markdown parser implementation

### Completion Criteria
- [x] All tasks above completed (Tasks #1-6 complete, all commits pushed)
- [x] Parser handles all valid Lash Markdown formats (validated through 200+ tests)
- [x] Comprehensive error reporting with line numbers (implemented in ParseContext)
- [x] Performance targets met (<100ms for typical files) - **Achieved: 67.7µs for realistic file**
- [x] 100+ unit tests covering edge cases (180 parser unit tests + 20+ integration tests)
- [-] Integration tests with fixture files (waived: comprehensive integration tests use inline content)
- [x] Benchmarks established (criterion benchmarks show 67.7µs for realistic workload)

### Parser Pipeline

```
File Content
    ↓
pulldown-cmark (Markdown events)
    ↓
Header Parser → ParsedHeader
    ↓
Checkbox Parser → Vec<CheckboxLine>
    ↓
Tree Builder → TaskTree
    ↓
File Builder → TaskFile
```

### Error Handling Strategy

- Continue parsing after errors when possible
- Collect all errors, sort by line number
- Provide actionable error messages with context
- Include suggestions for fixes
- Support `--json` output for tooling

### Performance Targets

| File Size | Task Count | Target Time |
|-----------|------------|-------------|
| Small     | 10 tasks   | <100ms      |
| Medium    | 100 tasks  | <500ms      |
| Large     | 1000 tasks | <5s         |

### Test Coverage Goal

- **80%+ code coverage**
- Edge cases for all parsing rules
- Round-trip tests (parse → format → parse)
- Performance regression tests

### Next Steps

After completing Markdown parser, proceed to:
1. **tasks.linter.md** - Build on parser to validate semantics
2. **tasks.sqlite-schema.md** - Store parsed structures in database
