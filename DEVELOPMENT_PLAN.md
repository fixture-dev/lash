# Lash Development Plan - Executive Summary

**Date:** 2025-11-18 (Updated)
**Status:** Planning Complete - Ready for Implementation
**Target:** v1.0 Production Release
**Last Updated:** Commit 3776540 - All task breakdowns completed

## Overview

This document provides a high-level summary of the comprehensive development plan for Lash v1.0. Detailed task breakdowns are located in the `tasks/` directory.

## Planning Process

Three specialized subagents analyzed the design document and created comprehensive task breakdowns:

1. **dev-project-manager** - Created task breakdown and implementation sequencing
2. **graph-systems-architect** - Analyzed dependency model and graph algorithms
3. **rust-dev-engineer** - Provided Rust-specific architecture recommendations

Their analyses are documented in:
- `docs/dependency-graph-analysis.md` - Graph architecture deep-dive
- `docs/rust-architecture-recommendations.md` - Rust implementation guidance
- `tasks/tasks.md` - Master task index with 8 implementation phases
- `tasks/tasks.*.md` - 16 detailed task files covering all modules

### Recent Accomplishments (Nov 17-18, 2025)

- ✅ Created comprehensive design document with full specification
- ✅ Completed expert analysis from 3 specialized agents
- ✅ Generated 16 detailed task files with ~80 actionable tasks
- ✅ Defined 8-phase implementation timeline (40-60 days)
- ✅ Made core technology and architecture decisions
- ✅ Established development practices and workflow in CLAUDE.md
- ✅ Ready to begin Phase 1: Project Setup

## Project Scope

### What is Lash?

Lash is a minimalist, ultra-fast, Markdown-native task tracker for developers and AI agents.

**Key Characteristics:**
- Markdown files are the single source of truth
- SQLite provides a fully-reconstructible acceleration layer
- Strict, linter-enforced format for predictability
- Terminal-first UX (CLI + TUI)
- Agent-friendly with token minimization strategies

### Success Criteria for v1.0

**Must Have (Critical):**
- ✅ Parse valid Lash Markdown files
- ✅ Lint files with clear, actionable error messages
- ✅ Index files into SQLite database
- ✅ Resolve dependencies and detect cycles
- ✅ Core CLI commands: `list`, `show`, `search`, `graph`
- ✅ Agent integration (`lash agent-prompt`)
- ✅ Comprehensive test coverage (>80%)
- ✅ User documentation

**Should Have (High Priority):**
- ✅ TUI for interactive use
- ✅ Incremental indexing
- ✅ Auto-formatting
- ✅ Performance benchmarks

**Nice to Have (Future):**
- ⚠️ Advanced agent views
- ⚠️ Fuzzy link fixing
- ⚠️ Archive command

## Architecture

### Crate Structure

The project uses a **6-crate workspace** to maintain clean separation:

```
lash/
├── crates/
│   ├── lash-types/       # Shared types, error taxonomy
│   ├── lash-core/        # Parsing, linting, validation
│   ├── lash-db/          # SQLite schema, repositories
│   ├── lash-agent/       # Agent prompts, token minimization
│   ├── lash-tui/         # Terminal UI (ratatui)
│   └── lash-cli/         # CLI binary, command integration
├── tasks/                # Task tracking (this system)
├── docs/                 # Design docs, analysis
└── tests/                # Integration tests, fixtures
```

### Key Technologies

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Markdown Parsing | `pulldown-cmark` | Fast, streaming, CommonMark compliant |
| CLI Framework | `clap` v4 | Modern, derive macros, shell completion |
| Database | `rusqlite` | Embedded SQLite, no server needed |
| TUI | `ratatui` + `crossterm` | Industry standard, actively maintained |
| Fuzzy Search | `nucleo` + FTS5 | TUI: nucleo, CLI: SQLite FTS5 |
| Error Handling | `thiserror` + `anyhow` | Structured errors + ergonomic CLI |
| Testing | `proptest` + `criterion` | Property-based + benchmarking |
| Graph Algorithms | `petgraph` | Mature, optimized graph library |

### Data Model

**Core entities:**
- `Task` - Individual checklist item with status, metadata, hierarchy
- `TaskFile` - Markdown file containing task tree plus metadata
- `RootIndex` - Project-level index mapping file structure
- `Dependency` - Explicit or implicit dependency between tasks
- `Label` - Cross-cutting tag for filtering and grouping

**Dependency types:**
1. **Hierarchy** - Parent task depends on children (implicit)
2. **Explicit ID** - `@depends-on: file-id#task-id`
3. **Explicit Path** - `@depends-on: path/to/file.md`
4. **Directory** - File depends on subdirectory completion

### Markdown Format

**Example task file:**

```markdown
# Photo App – Sepia Filter

@id: photo-app.filters.sepia
@labels: photo-app, filters, image-processing
@owner: frank
@created: 2025-11-16

Short description of the sepia filter feature.

## Tasks

- [ ] Implement sepia filter core #backend
  - [ ] Define parameter schema
  - [ ] Write Rust core function
  - [ ] Add tests
- [ ] Integrate with UI #frontend
  - [ ] Wire up settings panel
  - [ ] Hook into preview
- [-] Performance optimization (waived for v1)

## References

- Depends on: `../core/image-pipeline.md`
```

## Implementation Timeline

### 8 Phases Over 40-60 Days (8-13 weeks)

| Phase | Duration | Goal | Key Deliverables |
|-------|----------|------|------------------|
| **1. Foundation** | Weeks 1-2 | Infrastructure | Project setup, core types, error taxonomy |
| **2. Core** | Weeks 3-5 | Parsing & Schema | Markdown parser, linter, SQLite schema |
| **3. Indexing** | Weeks 6-7 | Build Index | File scanning, indexing, CLI framework |
| **4. Dependencies** | Weeks 7-8 | Resolve Deps | Graph building, cycle detection, queries |
| **5. Search** | Weeks 9-10 | Find Tasks | Fuzzy search, advanced commands |
| **6. Agents** | Week 11 | AI Integration | Prompt generation, token minimization |
| **7. TUI** | Week 12 | Interactive UI | Terminal interface |
| **8. Polish** | Week 13 | Production Ready | Testing, docs, benchmarks |

### Critical Path

```
Foundation (project setup, data model, errors)
    ↓
Parsing (Markdown → data structures)
    ↓
Linting (validation rules)
    ↓
Schema (SQLite design)
    ↓
Indexing (files → database)
    ↓
Dependencies (graph resolution)
    ↓
Commands (CLI functionality)
    ↓
Polish (tests, docs, performance)
```

**Parallelization opportunities** exist within each phase - see `tasks/tasks.md` for details.

## Risk Assessment

### High-Risk Areas

1. **Parser Complexity**
   - **Risk:** Hand-written parser may be complex
   - **Mitigation:** Start simple, add features incrementally, use pulldown-cmark
   - **Contingency:** Reduce scope (drop inline metadata if needed)

2. **Dependency Resolution Performance**
   - **Risk:** Graph algorithms slow on large projects
   - **Mitigation:** Benchmark early, aggressive caching, use petgraph
   - **Contingency:** Incremental computation, limit graph depth

3. **TUI Complexity**
   - **Risk:** May take longer than estimated
   - **Mitigation:** Build incrementally, core features first
   - **Contingency:** Ship v1 without TUI, add in v1.1

4. **Cross-Platform Issues**
   - **Risk:** Platform-specific bugs (SQLite, terminals)
   - **Mitigation:** Test on all platforms early
   - **Contingency:** Target Unix-like systems first, Windows in v1.1

## Task Organization

### Task Files Created

The `tasks/` directory contains detailed breakdowns:

1. ✅ **tasks.md** - Master index and phase overview
2. ✅ **tasks.project-setup.md** - Foundation (5 tasks, 3-5 days)
3. ✅ **tasks.core-data-model.md** - Data structures (6 tasks, 5-7 days)
4. ✅ **tasks.markdown-parser.md** - Parsing (6 tasks, 7-9 days)
5. ✅ **tasks.linter.md** - Validation (6 tasks, 8-10 days)
6. ✅ **tasks.sqlite-schema.md** - Database (6 tasks, 7-9 days)
7. ✅ **tasks.indexing.md** - Indexing (5 tasks, 5-7 days)
8. ✅ **tasks.dependency-resolution.md** - Graph (5 tasks, 6-8 days)
9. ✅ **tasks.cli-framework.md** - CLI infra (4 tasks, 3-4 days)
10. ✅ **tasks.cli-commands.md** - Commands (10 tasks, 8-10 days)
11. ✅ **tasks.fuzzy-search.md** - Search (4 tasks, 4-5 days)
12. ✅ **tasks.tui.md** - Terminal UI (8 tasks, 7-10 days)
13. ✅ **tasks.agent-integration.md** - Agents (6 tasks, 5-6 days)
14. ✅ **tasks.error-handling.md** - Errors (4 tasks, 3-4 days)
15. ✅ **tasks.testing.md** - Testing (4 tasks, 4-6 days)
16. ✅ **tasks.documentation.md** - Docs (4 tasks, 4-5 days)

**Status:**
- ✅ **All 16 task files created and completed** (as of commit 3776540)
- ✅ **Comprehensive task breakdowns for all modules**

Each task file contains:
- Priority levels (CRITICAL, HIGH, MEDIUM, LOW)
- Time estimates
- Dependencies on other tasks
- Success criteria
- Detailed subtasks (checkbox format)
- Test requirements

## Design Decisions Made

The following design decisions have been made and are reflected in the task breakdowns:

1. ✅ **Header Format** - `@key: value` annotations (clean, parseable, no YAML complexity)
2. ✅ **Max Task Depth** - 3-4 levels recommended (balance between flexibility and simplicity)
3. ✅ **Indentation** - 2 spaces per level (standard Markdown convention)
4. ✅ **Fuzzy Search** - Hybrid approach: SQLite FTS5 for CLI + `nucleo` for TUI
5. ✅ **TUI Library** - `ratatui` + `crossterm` (industry standard, actively maintained)
6. ✅ **Graph Library** - `petgraph` for dependency resolution and cycle detection
7. ✅ **Testing Strategy** - `proptest` for property-based testing + `criterion` for benchmarks
8. ✅ **Crate Structure** - 6-crate workspace for clean separation of concerns

### Remaining Open Questions

Minor details that can be refined during implementation:
- Exact error message formatting and color scheme
- Specific TUI keybindings and navigation patterns
- Token minimization strategies for different agent contexts

## Development Practices

Per `CLAUDE.md`, this project follows:

- **Task Management:** All work tracked in `tasks/` directory with checkboxes
- **Git Workflow:** Commit when work complete; keep clean `git status`
- **Pre-commit Hooks:** Linting and tests must pass
- **DRY Principle:** No code duplication
- **File Size:** Max 500 lines per file (refactor if larger)
- **Testing Layers:** Unit, integration, end-to-end as appropriate
- **No Mocking:** Real behavior only in production code
- **Documentation:** Keep README and docs current

**Context Management:**
- Clean up background processes before new tasks
- Git commits in foreground (concise messages)
- Use `/clear` after major milestones
- Background processes only for truly long operations (>30s)

## Getting Started

### For Development

1. **Review design documents:**
   - `docs/design-doc.md` - Complete specification
   - `docs/dependency-graph-analysis.md` - Graph architecture
   - `docs/rust-architecture-recommendations.md` - Rust guidance

2. **Check current status:**
   - `git status` - See what's committed
   - `tasks/tasks.md` - See task organization
   - `tasks/tasks.project-setup.md` - Start here for implementation

3. **Begin Phase 1:**
   - Complete `tasks.project-setup.md` (5 tasks)
   - Complete `tasks.core-data-model.md` (6 tasks)
   - This establishes foundation for all other work

### For Project Management

- Track progress by checking off tasks in `tasks/*.md` files
- Update `devlog.md` with progress notes and decisions
- Reference git commits in devlog entries
- Ask for clarification when requirements are ambiguous

## Next Steps

**Immediate (Week 1):**
1. ✅ Complete planning and task breakdown (DONE - commit 3776540)
2. ✅ Create all task files (DONE - 16 files created)
3. ✅ Finalize core design decisions (DONE - see above)
4. ⏭️ **BEGIN Phase 1: Project Setup** - Start with `tasks/tasks.project-setup.md`

**Short-term (Weeks 2-4):**
- Complete Foundation phase
- Begin Parsing implementation
- Set up CI/CD pipeline
- Create initial test fixtures

**Medium-term (Weeks 5-10):**
- Complete core functionality
- Implement all CLI commands
- Achieve 80%+ test coverage

**Long-term (Weeks 11-13):**
- Polish and optimize
- Complete documentation
- Prepare for v1.0 release

## Resources

- **Design Document:** `/docs/design-doc.md`
- **Task Index:** `/tasks/tasks.md`
- **Development Guide:** `/CLAUDE.md`
- **Error Codes:** `/docs/error-codes.md` (to be created)
- **Architecture:** `/docs/rust-architecture-recommendations.md`
- **Graph Analysis:** `/docs/dependency-graph-analysis.md`

## Questions or Clarifications

For questions during development:
1. Check design documents first
2. Check task files for context
3. Ask user for clarification on ambiguous requirements
4. Document decisions in `devlog.md`

---

**This plan represents approximately 40-60 days of focused development work to reach v1.0 production release.**
