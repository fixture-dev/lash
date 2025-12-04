# Description Section Feature Tasks

@id: tasks.description-section
@labels: parser, linter, schema, indexing, search, cross-cutting
@status: not-started
@created: 2025-12-03

## Description

Add support for a `## Description` section in task files that allows free-form text (including `@agent-note` annotations) to provide context for both humans and agents. This section appears after the file-level metadata block and before the `## Tasks` section.

**Key Requirements:**
- Free-form text with optional annotations
- Placement: after metadata, before `## Tasks`
- Length limit: 500-1000 characters (enforced by linter)
- Indexed in SQLite and searchable via FTS5
- Supports inline `@agent-note` annotations for LLM-specific hints

**Example Format:**
```markdown
# Module Name

@id: module.name
@labels: backend

## Description

Free-form text explaining scope and intent. Can include @agent-note: for LLM-specific hints.
This provides context for both humans and agents.

## Tasks

- [ ] First task
```

## Tasks

### Task 1: Update Design Document

@id: task-1-design-doc
@priority: HIGH
@effort: 0.5 days
@depends-on: None

Update the design document to formalize the `## Description` section specification.

- [ ] Update section 4.1 (High-Level Structure) in design-doc.md
  - [ ] Replace "optional overview" with formal `## Description` section
  - [ ] Specify placement: after metadata block, before `## Tasks`
  - [ ] Document that it's optional but recommended
- [ ] Update section 4.1 example to include `## Description`
  - [ ] Add example description text
  - [ ] Show inline `@agent-note:` usage
- [ ] Add Description Section specification details
  - [ ] Define allowed content: free-form Markdown text
  - [ ] Specify length limit: 500-1000 characters (configurable)
  - [ ] Document `@agent-note:` inline annotation support
- [ ] Update section 9.2 (SQLite Schema) to mention description storage
  - [ ] Add `description` column to `files` table specification
  - [ ] Note that description is indexed in FTS5

**Success Criteria:**
- Design document clearly specifies `## Description` section format
- Examples demonstrate proper usage
- Schema changes documented

---

### Task 2: Update Parser to Recognize Description Section

@id: task-2-parser
@priority: CRITICAL
@effort: 1-1.5 days
@depends-on: tasks.description-section.md#task-1-design-doc

Extend the Markdown parser to recognize and extract `## Description` sections from task files.

- [ ] Update `Section` enum in parser module
  - [ ] Add `Section::Description` variant
  - [ ] Update section detection logic in event stream processor
  - [ ] Handle transition from Header -> Description -> Tasks
- [ ] Implement description extraction in parser
  - [ ] Detect `## Description` heading
  - [ ] Collect all text until next `##` heading or EOF
  - [ ] Preserve formatting (paragraphs, inline code, etc.)
  - [ ] Extract inline `@agent-note:` annotations
- [ ] Update `TaskFile` struct in core data model
  - [ ] Add `description: Option<String>` field
  - [ ] Add `description_agent_notes: Vec<String>` field
  - [ ] Update constructors and builders
- [ ] Handle edge cases
  - [ ] Empty description section (just heading, no content)
  - [ ] Multiple `## Description` headings (error: duplicate section)
  - [ ] Description after `## Tasks` (error: wrong order)
- [ ] Update parser tests
  - [ ] Parse file with description section
  - [ ] Parse file without description (verify None)
  - [ ] Parse description with inline annotations
  - [ ] Parse description with multiple paragraphs
  - [ ] Error on duplicate description sections

**Success Criteria:**
- Parser correctly extracts description text
- `TaskFile` struct contains description data
- Inline `@agent-note` annotations extracted
- All edge cases handled with appropriate errors
- Existing parser tests still pass

---

### Task 3: Add Linter Rules for Description Validation

@id: task-3-linter
@priority: HIGH
@effort: 1 day
@depends-on: tasks.description-section.md#task-2-parser

Implement linter rules to validate description sections, primarily enforcing the length limit.

- [ ] Implement Rule: Description Length Limit
  - [ ] Code: `W_DESC_TOO_LONG`
  - [ ] Check: Description <= 1000 characters (warning)
  - [ ] Check: Description > 2000 characters (error, hard limit)
  - [ ] Suggestion: "Consider moving detailed content to linked documentation"
- [ ] Implement Rule: Description Section Order
  - [ ] Code: `E_DESC_WRONG_POSITION`
  - [ ] Check: Description appears after header, before Tasks
  - [ ] Error on: Description after `## Tasks` or `## References`
- [ ] Implement Rule: Duplicate Description Section
  - [ ] Code: `E_DESC_DUPLICATE`
  - [ ] Check: Only one `## Description` section per file
- [ ] Add configuration option for length limit
  - [ ] Add `description_max_length` to `LintConfig`
  - [ ] Default: 1000 characters (warning threshold)
  - [ ] Allow projects to configure via `.lash/config.toml`
- [ ] Register rules with rule engine
- [ ] Update linter tests

**Success Criteria:**
- Length limit enforced with clear diagnostics
- Warning at 1000 chars, error at 2000 chars
- Position and duplicate rules catch violations
- Configuration allows customization

---

### Task 4: Update SQLite Schema for Description Storage

@id: task-4-schema
@priority: CRITICAL
@effort: 0.5 days
@depends-on: tasks.description-section.md#task-1-design-doc

Add a `description` column to the `files` table to store description text.

- [ ] Update schema definition in `lash-db/src/schema.rs`
  - [ ] Add `description TEXT` column to `files` table
  - [ ] Make column nullable (files may not have descriptions)
- [ ] Create migration for existing databases
  - [ ] Write migration using `ALTER TABLE files ADD COLUMN description TEXT`
  - [ ] Test migration on existing test databases
- [ ] Update `FileRecord` struct
  - [ ] Add `description: Option<String>` field
  - [ ] Update SQL queries to include description
  - [ ] Update `from_row()` and `to_params()` methods
- [ ] Update file repository queries
  - [ ] `insert_file()`: include description column
  - [ ] `update_file()`: include description column
  - [ ] `get_file_by_id()`: retrieve description
  - [ ] `get_file_by_path()`: retrieve description

**Success Criteria:**
- Schema migration applies cleanly
- Files table has description column
- All repository methods updated
- Existing tests pass after migration

---

### Task 5: Update Indexing to Populate Description Field

@id: task-5-indexing
@priority: CRITICAL
@effort: 0.5 days
@depends-on: tasks.description-section.md#task-2-parser, tasks.description-section.md#task-4-schema

Update the indexing engine to extract and store description text when indexing files.

- [ ] Update `FileIndexer` in indexing engine
  - [ ] Extract `description` field from parsed `TaskFile`
  - [ ] Pass description to file repository during insert/update
  - [ ] Handle None case (files without descriptions)
- [ ] Update incremental indexing
  - [ ] Re-index file if description changes
  - [ ] Update FTS5 index when description changes
- [ ] Update index verification
  - [ ] `check-index` command verifies description matches Markdown
  - [ ] Report drift if description in DB doesn't match parsed file
- [ ] Add description to index statistics
  - [ ] Count files with descriptions vs without
  - [ ] Include in `lash index --verbose` output

**Success Criteria:**
- Indexing extracts and stores descriptions
- Files with and without descriptions both handled
- Incremental indexing detects description changes
- Index verification includes description comparison

---

### Task 6: Update FTS5 Search to Include Description Content

@id: task-6-fts5
@priority: HIGH
@effort: 1 day
@depends-on: tasks.description-section.md#task-4-schema, tasks.description-section.md#task-5-indexing

Update the FTS5 search index to include file description text, making descriptions searchable.

- [ ] Update FTS5 search schema
  - [ ] Add description content to `search_index` virtual table
  - [ ] Set column weight (higher than task body, lower than title)
- [ ] Update search index population
  - [ ] During indexing, extract description from `files` table
  - [ ] Insert description into FTS5 `search_index`
  - [ ] Handle NULL descriptions
- [ ] Update search result presentation
  - [ ] Include description in search results
  - [ ] Show description snippet if matched
  - [ ] Highlight matched terms in description
- [ ] Implement file-level search results
  - [ ] When description matches but no tasks match, return file-level result
  - [ ] Show file path and description excerpt
- [ ] Update search scoring
  - [ ] Weight: file title > description > task title > task body
  - [ ] Tune weights based on testing

**Success Criteria:**
- Descriptions are searchable via FTS5
- Search results show description matches
- Highlighting works in description snippets
- Scoring appropriately weights description matches

---

### Task 7: Update CLI Commands to Display Descriptions

@id: task-7-cli
@priority: MEDIUM
@effort: 0.5 days
@depends-on: tasks.description-section.md#task-5-indexing

Update CLI commands (`list`, `show`, `search`) to display file descriptions where relevant.

- [x] Update `lash show` command
  - [x] Display description section after metadata, before tasks
  - [x] Format with appropriate heading and indentation
  - [x] Respect `--format=json` (include description in JSON output)
- [ ] Update `lash list` command
  - [ ] Add `--show-descriptions` flag
  - [ ] Show first 100 chars of description for each file
  - [ ] Truncate with "..." if longer
- [ ] Update `lash agent-prompt` command
  - [ ] Include descriptions in generated prompts
  - [ ] Use descriptions for context-minimized prompts
  - [ ] Add `--include-descriptions` flag (default: true)

**Success Criteria:**
- Commands display descriptions appropriately
- JSON output includes descriptions
- Formatting is clean and readable

---

### Task 8: Update TUI to Display Descriptions

@id: task-8-tui
@priority: LOW
@effort: 0.5 days
@depends-on: tasks.description-section.md#task-5-indexing

Update the TUI detail pane to show file descriptions when viewing task files.

- [x] Update detail pane layout
  - [x] Add description section after file metadata
  - [x] Show description text with word wrapping
  - [x] Use distinct styling (e.g., italic or different color)
  - [x] Show "No description available" if absent
- [x] Handle long descriptions
  - [x] Truncate if needed to fit pane height
  - [x] Support scrolling within detail pane

**Success Criteria:**
- TUI shows descriptions in detail pane
- Layout is clean and readable
- Long descriptions handled gracefully

---

## Dependency Map

```
Task 1 (Design Doc)
    |
    v
Task 2 (Parser) -----> Task 3 (Linter)
    |
    v
Task 4 (Schema)
    |
    v
Task 5 (Indexing)
    |
    +-------+-------+
    |       |       |
    v       v       v
Task 6   Task 7   Task 8
(FTS5)   (CLI)    (TUI)
```

**Parallel Opportunities:**
- Task 3 (Linter) and Task 4 (Schema) can run in parallel after Task 2
- Task 6, 7, and 8 can run in parallel after Task 5

## References

- @doc: docs/design-doc.md - Main design specification
- Depends on: tasks.markdown-parser.md - Parser implementation
- Depends on: tasks.linter.md - Linter infrastructure
- Depends on: tasks.sqlite-schema.md - Database schema
- Depends on: tasks.indexing.md - Indexing engine
- Depends on: tasks.fuzzy-search.md - FTS5 search
