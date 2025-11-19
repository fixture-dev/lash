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

- [ ] **Parse `@key: value` format**
  - [ ] Create `parse_annotation(line: &str) -> Result<(String, String)>`
  - [ ] Detect `@` at start (after trimming)
  - [ ] Split on first `:` to get key and value
  - [ ] Trim whitespace from both key and value
  - [ ] Validate key format (alphanumeric + hyphen + underscore)
- [ ] **Handle multiline values**
  - [ ] Detect when value continues on next line (indented)
  - [ ] Accumulate continuation lines
  - [ ] Preserve internal newlines/formatting
  - [ ] Trim trailing whitespace only
- [ ] **Parse specific annotation types**
  - [ ] `@id: string` - validate ID format
  - [ ] `@labels: a, b, c` - split on commas, parse as Label list
  - [ ] `@status: string` - validate against known statuses
  - [ ] `@owner: string` - any string
  - [ ] `@created: YYYY-MM-DD` - validate date format
  - [ ] `@estimate: duration` - validate format (e.g., "2h", "3d")
  - [ ] `@depends-on: ref` - parse as DependencyRef
  - [ ] `@agent-note: text` - any text
- [ ] **Validate known annotation keys**
  - [ ] Load custom keys from config
  - [ ] Check against built-in keys: id, labels, status, owner, created, estimate, depends-on, agent-note
  - [ ] Check against custom keys from `.lash/config.toml`
  - [ ] Error on unknown key not in either list
  - [ ] Suggest adding to config if looks like custom key
- [ ] **Parse value types**
  - [ ] String values (default)
  - [ ] Comma-separated lists (for labels, depends-on)
  - [ ] Date values (YYYY-MM-DD format)
  - [ ] Duration values (regex: `\d+[hdwmy]` for hours/days/weeks/months/years)
  - [ ] Return structured errors for invalid formats
- [ ] **Handle multiple `@depends-on` annotations**
  - [ ] Allow multiple `@depends-on` lines
  - [ ] Accumulate into Vec<DependencyRef>
  - [ ] Each can be separate file or task reference
- [ ] **Create annotation collection**
  - [ ] Define `AnnotationBlock` struct:
    - [ ] `annotations: HashMap<String, Vec<String>>` - allow multiple values per key
    - [ ] Helper methods: `get_single()`, `get_list()`, `get_date()`, etc.
  - [ ] Validate no duplicate single-value annotations
  - [ ] Allow multiple values for `depends-on`, `labels`
- [ ] **Write tests**
  - [ ] Parse all built-in annotation types
  - [ ] Parse custom annotations (with config)
  - [ ] Multiline values
  - [ ] Multiple depends-on
  - [ ] Invalid key names
  - [ ] Invalid value formats (dates, durations)
  - [ ] Unknown annotations (error)
  - [ ] 30+ test cases

**Priority:** CRITICAL
**Estimate:** 1.5 days
**Dependencies:** Task #2
**Success Criteria:** Can parse all annotation types; validates keys against config; handles multiline values

---

### 4. Implement Header Block Parser

- [ ] **Parse H1 title**
  - [ ] Use pulldown-cmark to find first `Heading(1)` event
  - [ ] Extract text content
  - [ ] Store as file title
  - [ ] Error if no H1 found
  - [ ] Warning if multiple H1s found (use first)
- [ ] **Extract header annotations**
  - [ ] Parse lines between H1 and first H2
  - [ ] Identify annotation lines (start with `@`)
  - [ ] Skip blank lines
  - [ ] Collect overview text (non-annotation, non-blank lines)
  - [ ] Parse all annotations into `AnnotationBlock`
- [ ] **Parse overview section**
  - [ ] Collect all text between annotations and "## Tasks"
  - [ ] Preserve paragraph structure
  - [ ] Trim excessive whitespace
  - [ ] Store as optional overview text
- [ ] **Detect "## Tasks" section boundary**
  - [ ] Find `Heading(2)` with text "Tasks"
  - [ ] Case-insensitive comparison
  - [ ] Set parser state to TasksSection
  - [ ] Error if no Tasks section found
- [ ] **Parse optional "## References" section**
  - [ ] Find `Heading(2)` with text "References"
  - [ ] Collect content (markdown list, prose, etc.)
  - [ ] Store as optional references text
  - [ ] Parse depends-on links from references if present
- [ ] **Handle malformed headers**
  - [ ] Missing H1: synthesize from filename
  - [ ] Missing Tasks section: treat whole file as tasks
  - [ ] Missing annotations: use defaults
  - [ ] Graceful degradation, emit warnings
- [ ] **Create header representation**
  - [ ] Define `ParsedHeader` struct:
    - [ ] `title: String`
    - [ ] `annotations: AnnotationBlock`
    - [ ] `overview: Option<String>`
    - [ ] `references: Option<String>`
  - [ ] Convert to `FileMetadata` from annotations
- [ ] **Write tests**
  - [ ] Complete valid header
  - [ ] Minimal header (H1 + Tasks only)
  - [ ] With overview text
  - [ ] With references
  - [ ] Multiple H1s (warning)
  - [ ] No H1 (synthesize)
  - [ ] No Tasks section (warning, treat all as tasks)
  - [ ] 20+ test cases

**Priority:** HIGH
**Estimate:** 1 day
**Dependencies:** Task #3
**Success Criteria:** Can parse complete header block; handles missing sections gracefully

---

### 5. Implement Task Tree Builder

- [ ] **Build parent-child relationships from indentation**
  - [ ] Create `TaskTreeBuilder` struct
  - [ ] Maintain stack of current parents at each depth
  - [ ] For each checkbox line:
    - [ ] Compute depth from indentation (indent / 2)
    - [ ] Validate depth doesn't exceed max (3 levels = depth 0,1,2)
    - [ ] Validate depth doesn't jump (can't go from 0 to 2)
    - [ ] Pop stack to current depth
    - [ ] Set parent as top of stack
    - [ ] Push current task onto stack
- [ ] **Validate depth limits during construction**
  - [ ] Check `depth <= config.max_depth` (max_depth = 2 for 3 levels)
  - [ ] Emit error with line number if exceeded
  - [ ] Suggest splitting into separate file
- [ ] **Handle malformed indentation**
  - [ ] Detect inconsistent depth jumps (0 → 2 without 1)
  - [ ] Emit error: "Cannot skip indentation levels"
  - [ ] Provide suggestion: indent by 2 spaces
  - [ ] Attempt recovery: treat as sibling of last task
- [ ] **Compute task order indices**
  - [ ] Assign sequential order_index to siblings
  - [ ] Reset count for each parent
  - [ ] Maintain document order for traversal
- [ ] **Generate synthetic IDs for tasks without `@id`**
  - [ ] Format: `task-{order}` or `{title-slug}`
  - [ ] Use task title slug if unique (lowercase, hyphenated)
  - [ ] Fall back to numeric index if title slugs collide
  - [ ] Ensure IDs unique within file
- [ ] **Validate ID uniqueness within file**
  - [ ] Track all task IDs in HashSet
  - [ ] Error on duplicate IDs
  - [ ] Provide line numbers for both occurrences
  - [ ] Suggest renaming one
- [ ] **Build task hierarchy**
  - [ ] Create `Task` instances from `CheckboxLine` data
  - [ ] Set parent_id based on indentation stack
  - [ ] Set depth, order_index
  - [ ] Collect all tasks into `TaskTree`
- [ ] **Apply auto-waiving logic**
  - [ ] If parent status is Waived, mark all children as Waived
  - [ ] Traverse depth-first after tree construction
  - [ ] Emit info diagnostic: "Auto-waived due to parent"
- [ ] **Write comprehensive tests**
  - [ ] Simple flat list (all depth 0)
  - [ ] Two-level hierarchy (parent + children)
  - [ ] Three-level hierarchy (max depth)
  - [ ] Sibling tasks at same depth
  - [ ] Multiple parent-child chains
  - [ ] Depth limit exceeded (error)
  - [ ] Skipped indentation level (error)
  - [ ] Duplicate IDs (error)
  - [ ] Synthetic ID generation
  - [ ] Auto-waiving propagation
  - [ ] 40+ test cases including edge cases

**Priority:** CRITICAL
**Estimate:** 2 days
**Dependencies:** Task #2
**Success Criteria:** Builds correct hierarchical structure; validates constraints; generates IDs

---

### 6. Implement Full File Parser

- [ ] **Create main `parse_file()` entry point**
  - [ ] Signature: `pub fn parse_file(path: &Path, config: &LashConfig) -> ParseResult<TaskFile>`
  - [ ] Read file content
  - [ ] Create ParseContext
  - [ ] Parse in phases: header → tasks → finalize
  - [ ] Collect all errors
  - [ ] Return TaskFile or aggregated errors
- [ ] **Integrate all parsing phases**
  - [ ] Phase 1: Parse header (H1, annotations, overview)
  - [ ] Phase 2: Parse task list (checkboxes → tree)
  - [ ] Phase 3: Parse references section
  - [ ] Phase 4: Validate and build TaskFile
- [ ] **Compute content hash**
  - [ ] Use blake3 to hash file content
  - [ ] Store in TaskFile.hash
  - [ ] Used for incremental indexing (detect changes)
- [ ] **Extract file metadata**
  - [ ] Get mtime from filesystem
  - [ ] Synthesize file ID from path if no @id
  - [ ] Build FileMetadata from header annotations
  - [ ] Validate metadata consistency
- [ ] **Implement error collection**
  - [ ] Continue parsing after errors when possible
  - [ ] Collect all diagnostics in ParseContext
  - [ ] Sort diagnostics by line number
  - [ ] Return complete error list (don't stop at first error)
- [ ] **Add performance optimization**
  - [ ] Use arena allocation for temporary parsing structures
  - [ ] Add `typed-arena` dependency
  - [ ] Allocate CheckboxLine instances in arena during parse
  - [ ] Convert to owned Task instances at end
  - [ ] Target: <100ms for typical files
- [ ] **Create convenience methods**
  - [ ] `parse_file_from_string(content: &str, config: &LashConfig) -> ParseResult<TaskFile>`
  - [ ] `parse_task_section(content: &str) -> ParseResult<Vec<Task>>` - for testing
  - [ ] `validate_file(file: &TaskFile, config: &LashConfig) -> Vec<Diagnostic>` - post-parse validation
- [ ] **Write integration tests**
  - [ ] Complete valid file (all sections)
  - [ ] Minimal valid file (H1 + tasks)
  - [ ] File with multiple errors (collect all)
  - [ ] Large file (100+ tasks) - performance test
  - [ ] Files from fixtures/valid/ directory
  - [ ] Files from fixtures/invalid/ directory (expect errors)
  - [ ] Round-trip: parse → serialize → parse (preserve structure)
  - [ ] 20+ integration tests
- [ ] **Benchmark performance**
  - [ ] Create benchmark suite using `criterion`
  - [ ] Benchmark small file (10 tasks)
  - [ ] Benchmark medium file (100 tasks)
  - [ ] Benchmark large file (1000 tasks)
  - [ ] Target: <100ms for small, <500ms for medium, <5s for large
  - [ ] Profile hot paths if needed

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** Tasks #4, #5
**Success Criteria:** Can parse complete files; reports all errors; meets performance targets

---

## Summary

### Total Estimate
**7-9 days** total for Markdown parser implementation

### Completion Criteria
- [ ] All tasks above completed
- [ ] Parser handles all valid Lash Markdown formats
- [ ] Comprehensive error reporting with line numbers
- [ ] Performance targets met (<100ms for typical files)
- [ ] 100+ unit tests covering edge cases
- [ ] Integration tests with fixture files
- [ ] Benchmarks established

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
