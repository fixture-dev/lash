# Lash Design Decisions

**Date:** 2025-11-17
**Status:** Finalized for v1.0 Implementation

This document records all design decisions made during the planning phase. These decisions resolve the "TBD" items from the original design document and provide clear guidance for implementation.

---

## 1. Header Format

**Decision:** @-annotations only (no YAML frontmatter)

**Rationale:**
- Consistent with inline task annotations
- Simpler parser implementation
- More agent-friendly (predictable format)
- Aligns with minimalist philosophy

**Example:**
```markdown
# Photo App – Sepia Filter

@id: photo-app.filters.sepia
@labels: photo-app, filters, image-processing
@status: in-progress
@owner: frank
@created: 2025-11-16

Task description here...

## Tasks
...
```

**Implications:**
- Parser needs to handle `@key: value` format only
- No YAML library dependency needed
- Linter can enforce strict annotation syntax

---

## 2. Maximum Task Depth

**Decision:** 3 levels (depth 0, 1, 2)

**Rationale:**
- Encourages shallow, readable hierarchies
- Forces decomposition of complex tasks into separate files
- Agent-friendly (easier to understand and manipulate)
- Balances flexibility with simplicity

**Example:**
```markdown
- [ ] Level 0: Top-level task
  - [ ] Level 1: Subtask
    - [ ] Level 2: Sub-subtask (maximum depth)
```

**Implications:**
- Linter enforces `depth <= 2`
- Parser must track nesting level
- Deep task hierarchies must be split into multiple files

---

## 3. Indentation

**Decision:** 2 spaces per level

**Rationale:**
- Standard Markdown convention
- Compact, saves horizontal space
- Works well with max depth of 3 (max indent = 4 spaces)
- Familiar to most developers

**Format:**
```markdown
- [ ] Level 0
··- [ ] Level 1 (2 spaces)
····- [ ] Level 2 (4 spaces)
```

**Implications:**
- Linter enforces exactly 2 spaces per level
- Parser must validate consistent indentation
- Auto-formatter normalizes to 2 spaces

---

## 4. Fuzzy Search Implementation

**Decision:** Hybrid approach (SQLite FTS5 for CLI + nucleo for TUI)

**Rationale:**
- **CLI (FTS5):** Simple, already using SQLite, good performance, standard query syntax
- **TUI (nucleo):** Fast fuzzy matching, responsive interactive search, great UX
- Use the right tool for each use case

**Implementation:**
- `lash search` command uses FTS5 virtual tables
- TUI search widget uses nucleo for real-time fuzzy filtering
- Both search same content, just different engines

**Implications:**
- Need both FTS5 schema and nucleo integration
- TUI search can be more interactive/responsive than CLI
- Slight complexity increase, but better UX overall

---

## 5. TUI Library

**Decision:** ratatui + crossterm

**Rationale:**
- Industry standard (fork of tui-rs)
- Actively maintained, great documentation
- Rich widget library
- Cross-platform support
- Large community

**Alternatives Considered:**
- cursive (too high-level, less control)
- termion (too low-level, too much work)

**Implications:**
- Add `ratatui` and `crossterm` dependencies
- Follow ratatui patterns and conventions
- Can leverage community widgets and examples

---

## 6. File Organization

**Decision:** Nested directories

**Rationale:**
- More intuitive for file browsers
- Natural hierarchy representation
- Familiar to most developers
- Works well with version control

**Example Structure:**
```
tasks/
├── photo-app/
│   ├── filters/
│   │   ├── sepia.md
│   │   └── vignette.md
│   └── core/
│       └── image-pipeline.md
└── lash.index.md
```

**Implications:**
- File ID synthesis: `photo-app/filters/sepia.md` → `photo-app.filters.sepia`
- Path resolution must handle relative paths (`../core/image-pipeline.md`)
- Directory structure directly reflects conceptual hierarchy

---

## 7. Waived Task Behavior

**Decision:** Automatically waive children when parent is waived

**Rationale:**
- Simpler mental model
- Reduces manual updates needed
- Consistent with "not applicable" semantics
- Parent waived implies children irrelevant

**Behavior:**
```markdown
- [-] Parent task (waived)
  - [-] Child 1 (auto-waived)
  - [-] Child 2 (auto-waived)
```

**Implications:**
- Linter/auto-formatter propagates waived status down hierarchy
- UI should make this behavior clear to users
- Reversing a waive requires explicitly unmarking children
- Document this behavior in user guide

---

## 8. Database Location

**Decision:** `.lash/lash.db` at project root

**Rationale:**
- Follows .git pattern (familiar)
- Keeps project root clean
- Easy to .gitignore
- Clear project-local data location

**Directory Structure:**
```
project/
├── .lash/
│   ├── lash.db          # SQLite database
│   └── config.toml      # Project config (optional)
├── lash.index.md        # Root index
└── tasks/               # Task files
```

**Implications:**
- Create `.lash/` directory on first `lash index`
- Add `.lash/` to default .gitignore template
- Database is project-scoped, not global
- Easy to reset: just delete `.lash/` and re-index

---

## 9. Unknown Annotation Handling

**Decision:** Strict validation with opt-in custom keys via config

**Rationale:**
- Catches typos and mistakes by default
- Prevents drift and inconsistency
- Allows extensibility where needed
- Users explicitly declare custom keys

**Implementation:**

**Built-in annotations (always allowed):**
- `@id`, `@labels`, `@status`, `@owner`, `@created`, `@estimate`, `@depends-on`, `@agent-note`

**Custom annotations (via config):**

`.lash/config.toml`:
```toml
[annotations]
# Define custom annotation keys
custom_keys = [
  { key = "priority", description = "Task priority (1-5)" },
  { key = "sprint", description = "Sprint number" },
  { key = "reviewer", description = "Code reviewer name" },
]
```

**Linter behavior:**
- Error on unknown `@key` not in built-in list
- Error on unknown `@key` not in config.toml custom_keys
- Allow any custom key defined in config
- Validate custom key names (alphanumeric + hyphen)

**Implications:**
- Linter must read config to validate annotations
- Config schema must support custom annotation definitions
- Error messages should suggest adding to config if custom key detected
- Documentation must explain how to add custom keys

---

## 10. Root Index Filename

**Decision:** Support both `lash.index.md` and `index.lash.md`

**Rationale:**
- Flexibility for user preference
- Both naming conventions have merit
- Easy to support both in root detection

**Behavior:**
- `find_project_root()` checks for both filenames
- Prefers `lash.index.md` if both exist
- `lash init` creates `lash.index.md` by default
- Documentation shows `lash.index.md` as canonical

**Implications:**
- Root detection checks both names
- Simple preference order avoids ambiguity
- Minimal implementation complexity

---

## Summary Table

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Header format | @-annotations only | Simple, consistent, agent-friendly |
| Max depth | 3 levels (0, 1, 2) | Shallow hierarchies, encourages decomposition |
| Indentation | 2 spaces | Standard Markdown convention |
| Fuzzy search | Hybrid (FTS5 + nucleo) | Right tool for each use case |
| TUI library | ratatui + crossterm | Industry standard, great docs |
| File organization | Nested directories | Intuitive, natural hierarchy |
| Waived children | Automatically waive | Simpler mental model |
| Database location | `.lash/lash.db` | Follows .git pattern |
| Unknown annotations | Strict + opt-in custom | Catches errors, allows extension |
| Index filename | Support both | User flexibility |

---

## Implementation Checklist

When implementing, ensure:

- [ ] Config supports `max_depth = 3`, `indent_spaces = 2`
- [ ] Config supports `[annotations.custom_keys]` array
- [ ] Parser enforces 2-space indentation
- [ ] Parser rejects depth > 2
- [ ] Linter validates annotations against built-in + config custom keys
- [ ] Linter auto-waives children when parent waived
- [ ] Search uses FTS5 for CLI, nucleo for TUI
- [ ] Root detection checks both `lash.index.md` and `index.lash.md`
- [ ] Database created in `.lash/lash.db`
- [ ] File ID synthesis handles nested directories correctly

---

## Future Considerations

These decisions are for v1.0. Future versions may revisit:

- Support for YAML frontmatter (if users strongly request)
- Configurable max depth (currently fixed at 3)
- Additional built-in annotation types
- Global vs project-local database options
- Configurable indentation (currently fixed at 2)

Any changes to these decisions should be thoroughly discussed and documented.
