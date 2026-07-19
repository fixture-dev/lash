# Task 10: Tree View Support - Implementation Summary

## Quick Reference

**Total Effort:** 4-5 days
**Phases:** 4 phases with clear dependencies
**Risk Level:** Medium (manageable with proper testing)
**Key Files:** 15+ files to create/modify

---

## Phased Approach

### Phase 1: Foundation (1 day)
**Goal:** Configuration infrastructure and core data structures

**Deliverables:**
- `TreeViewConfig` in user config (`~/.lash/config.toml`)
- CLI flags: `--tree-view`, `--no-tree-view`, `--max-depth`, `--ascii`
- Generic `TreeNode<T>` data structure
- `TreeChars` enum for Unicode/ASCII rendering
- Full unit test coverage

**Risk:** Low - Pure data structures with no UI dependencies

---

### Phase 2: TUI Integration (1.5 days)
**Goal:** Interactive tree views in TUI

**Deliverables:**
- File tree in navigation pane (expandable directories)
- Task tree in detail pane (expandable task hierarchies)
- Keyboard navigation: `h`/`l`, `H`/`L` for expand/collapse
- Tree rendering with Unicode/ASCII support
- Expansion state tracking

**Risk:** Medium - Complex state management and rendering

**Critical Success Factors:**
- Tree building algorithm is efficient
- Expansion state updates correctly
- Rendering aligns properly with selection

---

### Phase 3: CLI Commands (1.5 days)
**Goal:** Tree output in CLI commands

**Deliverables:**
- `TreeFormatter` utility for CLI output
- `lash list --tree` with directory hierarchy
- `lash search --tree` with results grouped by file
- `lash show` with subtask hierarchy and parent context
- Theme integration for colored output

**Risk:** Medium - Formatting complexity

**Critical Success Factors:**
- Tree characters align correctly in terminal
- Output is readable and informative
- Theme colors enhance readability

---

### Phase 4: Edge Cases & Polish (1 day)
**Goal:** Handle edge cases and complete documentation

**Deliverables:**
- Deep hierarchy overflow indicators (`...`)
- Empty directory handling
- Circular dependency detection
- Unicode terminal detection
- Complete documentation and help text
- Manual testing on multiple terminals

**Risk:** Low - Incremental improvements

---

## Key Technical Decisions

### 1. Generic Tree Structure
```rust
pub struct TreeNode<T> {
    pub data: T,
    pub children: Vec<TreeNode<T>>,
    pub expanded: bool,
    pub depth: usize,
}
```

**Rationale:** Enables code reuse for both file trees and task hierarchies

---

### 2. Tree Character Sets
```
Unicode: ├── └── │ ▸ ▾
ASCII:   +-- \-- | > v
```

**Auto-detection:** Check `LANG` environment variable for UTF-8
**Override:** `--ascii` flag or `tree_view.ascii_mode` config

---

### 3. Configuration Priority
```
CLI flag > User config > Default
```

**Example:**
1. `--max-depth 3` (highest priority)
2. `~/.lash/config.toml` → `tree_view.max_depth = 5`
3. Default: `5`

---

### 4. Keyboard Shortcuts (TUI)

| Key | Action |
|-----|--------|
| `h` / `←` | Collapse node or go to parent |
| `l` / `→` / `Enter` | Expand node or enter |
| `H` | Collapse all |
| `L` | Expand all (to max_depth) |
| `j`/`k`, `↑`/`↓` | Navigate up/down |
| `gg` / `G` | Top / Bottom |

**Note:** Vim-style `zo`, `zc`, `zM`, `zR` commands deferred (require key state tracking)

---

## Testing Strategy

### Unit Tests (throughout)
- `TreeNode<T>` methods: 100% coverage
- Tree building algorithms: comprehensive test cases
- Edge cases: empty trees, single nodes, deep trees, wide trees

### Integration Tests
- CLI flag parsing and priority
- TUI keyboard navigation
- Tree rendering with selection
- Config file loading/saving
- Each command with `--tree` flag

### Manual Tests
- Visual inspection on 5+ terminals
- Unicode vs ASCII rendering
- Long file path handling
- Terminal resize behavior
- Performance with large fixtures (1000+ files)

### Test Fixtures
Create in `fixtures/tree-view-tests/`:
- `deep-hierarchy/` (10+ levels)
- `wide-hierarchy/` (many siblings)
- `mixed-hierarchy/` (varied depths)
- `empty-dirs/` (empty directories)

---

## Critical Path

```
Phase 1.1 (Config)
    ↓
Phase 1.2 (Tree Data Structure)
    ↓
    ├─→ Phase 2.1 (TUI State)
    │       ↓
    │   Phase 2.2 (TUI Events) ──┐
    │       ↓                     │
    │   Phase 2.3 (TUI Rendering) │
    │       ↓                     │
    └─→ Phase 3.1 (CLI Formatter) │
            ↓                     │
        Phase 3.2-3.4 (CLI Cmds)  │
            ↓                     │
            └─────────────────────┘
                    ↓
            Phase 4.1 (Edge Cases)
                    ↓
            Phase 4.2 (Documentation)
```

**Parallel Opportunities:**
- Phase 2.2 and 3.1 can be done in parallel
- Phase 3.2, 3.3, 3.4 can be done in parallel (after 3.1)

---

## Risk Mitigation

### High-Risk: Tree Building Performance
**Concern:** Large projects (1000+ files) could be slow
**Mitigation:**
- Use HashMap for O(1) parent lookups
- Cache built trees in `AppState`
- Profile with large fixtures
- Lazy-load on expansion (TUI only)

### High-Risk: Rendering Complexity
**Concern:** Tree rendering logic error-prone
**Mitigation:**
- Comprehensive unit tests
- Snapshot tests for output
- Manual visual testing
- Start simple, iterate

### Medium-Risk: Unicode Compatibility
**Concern:** Characters may not render correctly
**Mitigation:**
- Robust terminal detection
- `--ascii` flag for override
- Test on multiple terminals
- Graceful ASCII fallback

### Medium-Risk: State Management (TUI)
**Concern:** Expansion state inconsistency
**Mitigation:**
- Centralize state updates
- Test state transitions
- Add assertions for invariants

---

## Performance Targets

**Tree Building:**
- < 100ms for 1000 files
- Use `cargo bench` for micro-benchmarks

**TUI Rendering:**
- < 16ms (60 FPS)
- No lag when expanding/collapsing

**CLI Formatting:**
- < 50ms for 1000 files
- Readable output at various terminal widths

**Keyboard Navigation:**
- < 10ms response time
- Immediate visual feedback

---

## Success Metrics

### Technical Metrics
- [ ] All 208+ tests pass (existing + new)
- [ ] Clippy clean (zero warnings)
- [ ] Benchmark targets met
- [ ] Zero regressions in existing functionality

### User Experience Metrics
- [ ] Tree view makes hierarchy obvious
- [ ] Keyboard shortcuts feel natural
- [ ] Configuration is discoverable
- [ ] Output is visually appealing
- [ ] ASCII fallback works seamlessly

### Functional Completeness
- [ ] All 10 subtasks in Task 10 completed
- [ ] TUI file tree with expand/collapse
- [ ] TUI task tree with expand/collapse
- [ ] CLI list/search/show with tree output
- [ ] Configuration persists correctly
- [ ] Edge cases handled gracefully
- [ ] Documentation complete

---

## Example Outputs

### List Command (Unicode)
```
lash list --tree

tasks/
├── tasks.md [3/5]
├── backend/
│   ├── api.md [2/4]
│   └── db.md [5/5] ✓
└── frontend/
    ├── components.md [1/3]
    └── styles.md [0/2]
```

### Search Command (Unicode)
```
lash search --tree "TODO"

tasks/
└── backend/
    └── api.md [2 matches]
        ├── Line 12: "TODO: Implement authentication"
        └── Line 45: "TODO: Add rate limiting"
```

### Show Command (Unicode)
```
lash show tasks.md#implementation

Parent Context:
tasks.md > Backend > Implementation

Task: Implementation
Status: [ ] Open
Labels: #backend, #high-priority

Subtasks:
├── Database schema [x]
├── API endpoints [ ]
│   ├── Authentication [ ]
│   └── CRUD operations [ ]
└── Testing [!] Blocked
```

---

## Configuration Example

```toml
# ~/.lash/config.toml

color_scheme = "Base2Tone Desert"

[tree_view]
enabled = true           # Enable tree view by default
max_depth = 5            # Maximum depth to display
default_expanded = false # Start with nodes collapsed
ascii_mode = false       # Use Unicode characters (auto-detect)
```

---

## Files to Create/Modify

### Create (5 files)
1. `crates/lash-types/src/tree.rs`
2. `crates/lash-cli/src/tree_formatter.rs`
3. `fixtures/tree-view-tests/` (directory structure)
4. `docs/task-10-implementation-plan.md`
5. `docs/task-10-summary.md`

### Modify (10+ files)
1. `crates/lash-types/src/config.rs`
2. `crates/lash-types/src/lib.rs`
3. `crates/lash-cli/src/main.rs`
4. `crates/lash-tui/src/state.rs`
5. `crates/lash-tui/src/event.rs`
6. `crates/lash-tui/src/app.rs`
7. `crates/lash-tui/src/ui/nav_pane.rs`
8. `crates/lash-tui/src/ui/detail_pane.rs`
9. `crates/lash-cli/src/commands/list.rs`
10. `crates/lash-cli/src/commands/search.rs`
11. `crates/lash-cli/src/commands/show.rs`
12. `crates/lash-tui/src/ui/help.rs`
13. `docs/user-guide.md`

---

## Next Steps

1. **Review this plan** with stakeholders
2. **Set up test fixtures** (can be done early)
3. **Begin Phase 1.1** (configuration)
4. **Iterate through phases** sequentially
5. **Test continuously** (don't wait until the end)
6. **Document as you go** (update user guide alongside implementation)

---

## Questions to Resolve

1. **Vim fold commands:** Implement `zo`, `zc`, `zM`, `zR` or just use simpler `h`, `l`, `H`, `L`?
   - **Recommendation:** Start with `h`/`l`/`H`/`L`, add vim-style later if users request

2. **Empty directories:** Show with "(empty)" or hide entirely?
   - **Recommendation:** Make configurable, default to hiding

3. **Expansion persistence:** Should TUI expansion state persist across sessions?
   - **Recommendation:** No (session-only), add later if needed

4. **Performance threshold:** What file count triggers performance warnings?
   - **Recommendation:** 1000+ files, optimize if needed

5. **Tree depth overflow:** Show `...` or truncate silently?
   - **Recommendation:** Show `...` indicator for transparency

---

**Plan Status:** Ready for Review
**Estimated Start Date:** TBD
**Target Completion:** 4-5 days from start
