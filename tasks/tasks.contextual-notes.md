# Contextual Notes Feature

@id: tasks.contextual-notes
@labels: feature, parser, linter, database, tui, cli
@status: not-started
@created: 2025-12-13

## Description

This feature adds support for **contextual notes** - plain bullet points (without checkboxes) nested under tasks that serve as inline context, requirements, or acceptance criteria. This provides a semantic distinction between actionable tasks and informational notes.

Example:
```markdown
- [ ] Integrate procedural level generation
  - Use Foo library to generate 2D map layouts
  - Ensure levels have an appropriate size constraint
  - Use foo, bar, baz and quux
```

Key semantic distinction:
- **Checkbox items** (`- [ ]`) = actionable tasks requiring completion tracking
- **Plain bullet items** (`-`) = contextual notes (no completion tracking)

## Design Decisions

1. Contextual notes appear as direct children of task items only
2. Distinguished from child tasks by absence of checkbox
3. Notes cannot have children (enforced by linter)
4. Notes should appear before child tasks (convention, soft warning)
5. Indexed in SQLite as searchable metadata on parent task
6. Included in FTS5 fuzzy search

## Tasks

### Phase 1: Design Documentation & Specification

- [x] Task 1.1: Update design document
  - Update Section 4.2 Task Line Format with contextual note grammar
  - Define `NOTE_LINE := INDENT* "- " TEXT` (no checkbox)
  - Specify nesting rules (notes as direct children of tasks only)
  - Specify ordering convention (notes before child tasks)
  - Add example showing mixed tasks and notes
  - Update Section 9.2 SQLite Schema to document `contextual_notes` field

### Phase 2: Core Data Model Updates

- [x] Task 2.1: Extend Task struct
  - Add `contextual_notes: Vec<String>` field to Task struct
  - Add `#[serde(default)]` for backward compatibility
  - Update `Task::default()` to initialize empty vec
  - Add unit tests for serialization round-trip

- [x] Task 2.2: Create ContextualNote type (optional refinement)
  - Create struct with `text: String` and `line_number: usize`
  - Add validation for max length (warning at 200, error at 500 chars)
  - Update Task to use `Vec<ContextualNote>` if richer metadata needed

### Phase 3: Parser Updates

- [x] Task 3.1: Add plain bullet detection
  - Add `is_plain_bullet()` function to checkbox parser
  - Create `PlainBulletLine` struct parallel to `CheckboxLine`
  - Ensure markdown links `- [text](url)` are NOT parsed as plain bullets
  - Add unit tests for plain bullet detection and link disambiguation

- [x] Task 3.2: Update parser events
  - Add `ContextualNote { indent, text, line_num }` to `ParserEvent` enum
  - Update parser to emit `ContextualNote` events
  - Ensure events maintain document order
  - Add integration tests for mixed task/note files

- [x] Task 3.3: Update task builder
  - Add `current_notes: Vec<String>` to builder state
  - Update `handle_checkbox()` to flush accumulated notes to previous task
  - Add `handle_contextual_note()` method
  - Enforce rule: notes must follow a task (emit warning if orphaned)
  - Add validation: notes at same or greater depth than parent task
  - Add integration tests for complex hierarchies

### Phase 4: Linter Rules

- [x] Task 4.1: Add note indentation rule
  - Create `NoteIndentationRule` in linter rules
  - Check note indent is multiple of 2 spaces
  - Check note indent is exactly 2 spaces deeper than parent task
  - Add diagnostic code `E_NOTE_INVALID_INDENT`
  - Add auto-fix capability for indentation

- [x] Task 4.2: Add note length validation
  - Create `NoteLengthRule` in linter rules
  - Add `W_NOTE_TOO_LONG` warning at 200 characters
  - Add `E_NOTE_EXCESSIVE_LENGTH` error at 500 characters
  - Measure length excluding indentation and bullet

- [x] Task 4.3: Add note nesting rule
  - Create `NoteNestingRule` in linter rules
  - Add `E_NOTE_HAS_CHILDREN` error code
  - Detect items nested under contextual notes
  - Add help text suggesting conversion to task if children needed

- [x] Task 4.4: Add note ordering guideline (optional)
  - Create `NoteOrderingRule` (style category)
  - Add `W_NOTE_AFTER_CHILD_TASKS` warning
  - Make configurable via `warn_note_ordering: bool`

### Phase 5: Database Schema

- [x] Task 5.1: Add database column
  - Create migration `v6_contextual_notes.rs`
  - Add `ALTER TABLE tasks ADD COLUMN contextual_notes TEXT DEFAULT '[]'`
  - Update `CURRENT_SCHEMA_VERSION` to 6
  - Register migration and write tests

- [x] Task 5.2: Update task repository
  - Update SQL queries to include `contextual_notes`
  - Add JSON serialization/deserialization for notes
  - Add unit tests for persistence round-trip

### Phase 6: Search Integration

- [x] Task 6.1: Add notes to FTS index
  - Update FTS5 schema to add `contextual_notes` column
  - Update FTS triggers (INSERT, UPDATE, DELETE)
  - Set appropriate BM25 weight (lower than title, similar to body)
  - Add integration tests for searching notes content

- [x] Task 6.2: Update search display
  - Show "matched in note" indicator in search results
  - Display matched note text with highlighting
  - Handle truncation for long notes

### Phase 7: CLI Commands

- [x] Task 7.1: Update `lash show` command
  - Display notes under parent task in detail view
  - Render notes with dimmed color or special marker (`·` or `○`)
  - Indent notes at parent task depth + 1
  - Add CLI integration tests

- [x] Task 7.2: Update `lash list` command
  - Add `--show-notes` flag
  - Format notes as sub-items with special marker
  - Default to hidden for concise output

### Phase 8: TUI Integration

- [x] Task 8.1: Render notes in task tree
  - Create `TreeItem::Note` variant
  - Render notes with `·` or `○` prefix (no checkbox)
  - Apply dimmed/italic styling
  - Update tree navigation to skip notes (not selectable)
  - Handle expand/collapse with notes

- [x] Task 8.2: Show notes in detail pane
  - Add "Notes:" section header when notes exist
  - List notes with bullet points
  - Apply dimmed styling

### Phase 9: Agent Integration

- [x] Task 9.1: Update agent schema
  - Document contextual note syntax in schema
  - Add examples showing notes usage
  - Document nesting rules and guidelines

- [x] Task 9.2: Update `lash agent-prompt`
  - Add `--include-notes` flag
  - Include notes in sparse context when flag set
  - Adjust token budget calculation for notes

### Phase 10: Testing & Documentation

- [x] Task 10.1: Integration tests
  - Test Parse → Lint → Index → Query workflow
  - Test mixed task/note hierarchies
  - Create fixture files with complex note patterns
  - Cover edge cases (orphaned notes, deep nesting)

- [x] Task 10.2: Documentation
  - Add contextual notes section to README
  - Create example task files with notes
  - Document best practices (notes vs. child tasks)
  - Update agent documentation

- [ ] Task 10.3: Performance testing
  - Benchmark parsing with varying note densities
  - Benchmark indexing and search with notes
  - Compare against baseline, document results

## Dependencies

### Internal Dependencies
```
Phase 1 (Design)
  └─> Phase 2 (Data Model)
       ├─> Phase 3 (Parser)
       │    └─> Phase 4 (Linter)
       ├─> Phase 5 (Database)
       │    └─> Phase 6 (Search)
       ├─> Phase 7 (CLI)
       ├─> Phase 8 (TUI)
       └─> Phase 9 (Agent)
            └─> Phase 10 (Testing & Docs)
```

### Critical Path
1.1 → 2.1 → 3.1 → 3.2 → 3.3 → 4.1 → 5.1 → 5.2 → 6.1 → 7.1 → 8.1 → 10.1 → 10.2

### Parallel Opportunities
- Phase 4 tasks (4.1-4.4) can run in parallel after 3.3
- Phase 7 and 8 can run in parallel after 5.2
- Phase 9 can start after 2.1

## Success Criteria

### Must Have
- [x] Parse plain bullets as contextual notes
- [x] Store notes in database
- [ ] Display notes in `lash show` and TUI
- [ ] Search notes content via FTS5
- [ ] Linter validates note structure
- [ ] >90% test coverage for new code

### Should Have
- [ ] Notes searchable with appropriate relevance weighting
- [ ] Visual distinction in TUI (dimmed/styled)
- [ ] Agent schema includes notes
- [ ] Performance within 10% of baseline

### Nice to Have
- [ ] Note ordering style warnings
- [ ] `--show-notes` flag in list command
- [ ] Performance benchmarks documented

## References

- Design doc: `docs/design-doc.md` (Sections 4.2, 9.2)
- Parser: `crates/lash-core/src/parser/`
- Linter: `crates/lash-core/src/linter/rules/`
- Database: `crates/lash-db/src/`
- TUI: `crates/lash-tui/src/`
