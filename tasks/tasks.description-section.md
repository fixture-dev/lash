# Description Section Feature Tasks

@id: tasks.description-section
@labels: parser, linter, schema, indexing, search, cross-cutting
@status: in-progress
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

- [x] Update section 4.1 (High-Level Structure) in design-doc.md
  - [x] Replace "optional overview" with formal `## Description` section
  - [x] Specify placement: after metadata block, before `## Tasks`
  - [x] Document that it's optional but recommended
- [x] Update section 4.1 example to include `## Description`
  - [x] Add example description text
  - [x] Show inline `@agent-note:` usage
- [x] Add Description Section specification details
  - [x] Define allowed content: free-form Markdown text
  - [x] Specify length limit: 500-1000 characters (configurable)
  - [x] Document `@agent-note:` inline annotation support
- [x] Update section 9.2 (SQLite Schema) to mention description storage
  - [x] Add `description` column to `files` table specification
  - [x] Note that description is indexed in FTS5

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

- [x] Update `Section` enum in parser module
  - [x] Add `Section::Description` variant
  - [x] Update section detection logic in event stream processor
  - [x] Handle transition from Header -> Description -> Tasks
- [x] Implement description extraction in parser
  - [x] Detect `## Description` heading
  - [x] Collect all text until next `##` heading or EOF
  - [x] Preserve formatting (paragraphs, inline code, etc.)
  - [x] Extract inline `@agent-note:` annotations
- [x] Update `TaskFile` struct in core data model
  - [x] Add `description: Option<String>` field
  - [x] Add `description_agent_notes: Vec<String>` field
  - [x] Update constructors and builders
- [x] Handle edge cases
  - [x] Empty description section (just heading, no content)
  - [x] Multiple `## Description` headings (error: duplicate section)
  - [x] Description after `## Tasks` (error: wrong order)
- [x] Update parser tests
  - [x] Parse file with description section
  - [x] Parse file without description (verify None)
  - [x] Parse description with inline annotations
  - [x] Parse description with multiple paragraphs
  - [x] Error on duplicate description sections

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

- [x] Implement Rule: Description Length Limit
  - [x] Code: `W_SEM_DESC_TOO_LONG` (warning) / `E_SEM_DESC_TOO_LONG` (error)
  - [x] Check: Description <= 1000 characters (warning)
  - [x] Check: Description > 2000 characters (error, hard limit)
  - [x] Suggestion: "Consider moving detailed content to linked documentation"
- [x] Implement Rule: Description Section Order
  - [x] Code: `E_PARSE_DESCRIPTION_AFTER_TASKS` (handled by parser)
  - [x] Check: Description appears after header, before Tasks
  - [x] Error on: Description after `## Tasks` or `## References`
- [x] Implement Rule: Duplicate Description Section
  - [x] Code: `E_SYNTAX_DUPLICATE_DESCRIPTION`
  - [x] Check: Only one `## Description` section per file
- [x] Add configuration option for length limit
  - [x] Add `description_max_length` to `LintConfig`
  - [x] Default: 1000 characters (warning threshold)
  - [x] Allow projects to configure via `.lash/config.toml`
- [x] Register rules with rule engine
- [x] Update linter tests

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

- [x] Update schema definition in `lash-db/src/schema.rs`
  - [x] Add `description TEXT` column to `files` table
  - [x] Make column nullable (files may not have descriptions)
- [x] Create migration for existing databases
  - [x] Write migration using `ALTER TABLE files ADD COLUMN description TEXT`
  - [x] Test migration on existing test databases
- [x] Update `FileRecord` struct
  - [x] Add `description: Option<String>` field
  - [x] Update SQL queries to include description
  - [x] Update `from_row()` and `to_params()` methods
- [x] Update file repository queries
  - [x] `insert_file()`: include description column
  - [x] `update_file()`: include description column
  - [x] `get_file_by_id()`: retrieve description
  - [x] `get_file_by_path()`: retrieve description

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

- [x] Update `FileIndexer` in indexing engine
  - [x] Extract `description` field from parsed `TaskFile`
  - [x] Pass description to file repository during insert/update
  - [x] Handle None case (files without descriptions)
- [x] Update incremental indexing
  - [x] Re-index file if description changes
  - [x] Update FTS5 index when description changes
- [x] Update index verification
  - [x] `check-index` command verifies description matches Markdown
  - [x] Report drift if description in DB doesn't match parsed file
- [x] Add description to index statistics
  - [x] Count files with descriptions vs without
  - [x] Include in `lash index --verbose` output

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

- [x] Update FTS5 search schema
  - [x] Add description content to `search_index` virtual table
  - [x] Set column weight (higher than task body, lower than title)
- [x] Update search index population
  - [x] During indexing, extract description from `files` table
  - [x] Insert description into FTS5 `search_index`
  - [x] Handle NULL descriptions
- [x] Update search result presentation
  - [x] Include description in search results
  - [x] Show description snippet if matched
  - [x] Highlight matched terms in description
- [x] Implement file-level search results
  - [x] When description matches but no tasks match, return file-level result
  - [x] Show file path and description excerpt
- [x] Update search scoring
  - [x] Weight: file title > description > task title > task body
  - [x] Tune weights based on testing

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
- [x] Update `lash list` command
  - [x] Add `--show-descriptions` flag
  - [x] Show first 100 chars of description for each file
  - [x] Truncate with "..." if longer
  - [x] Support both flat and tree view display modes
  - [x] Support JSON output format (include/exclude based on flag)
  - [x] Handle multi-byte UTF-8 characters correctly in truncation
- [x] Update `lash agent-prompt` command
  - [x] Include descriptions in generated prompts
  - [x] Use descriptions for context-minimized prompts
  - [x] Add `--include-descriptions` flag (default: true)

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
