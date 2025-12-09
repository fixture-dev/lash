# Lash Development Tasks - Master Index

This directory contains the comprehensive task breakdown for implementing Lash v1.0.

## Project Status

**Current Phase:** Phase 8 - Testing & Documentation
**Target:** v1.0 Production Release
**Estimated Duration:** 40-60 days (8-13 weeks)
**Completed:** Project Setup, Core Data Model, Error Handling, Markdown Parser, Linter & Formatter, SQLite Schema, Indexing, Dependency Resolution, CLI Framework, CLI Commands, Fuzzy Search, Agent Integration, TUI, Testing

## Task Organization

Tasks are organized by major module/component. Each task file contains:
- Task descriptions with priority levels
- Dependencies on other tasks
- Estimated effort
- Success criteria
- Subtasks broken down to actionable units

## Task Files

### Foundation & Infrastructure
- [x] [tasks.project-setup.md](tasks.project-setup.md) - Project initialization, tooling, error taxonomy ✅
- [x] [tasks.core-data-model.md](tasks.core-data-model.md) - Core data structures (Task, File, Dependency, Label models) ✅
- [x] [tasks.error-handling.md](tasks.error-handling.md) - Error types, formatting, aggregation (Tasks 1-3 complete) ✅

### Parsing & Validation
- [x] [tasks.markdown-parser.md](tasks.markdown-parser.md) - Markdown parsing implementation ✅
- [x] [tasks.linter.md](tasks.linter.md) - Linting rules, validation, auto-formatting ✅
- [x] [tasks.doc-annotation.md](tasks.doc-annotation.md) - @doc annotation for semantic documentation links ✅

### Database Layer
- [x] [tasks.sqlite-schema.md](tasks.sqlite-schema.md) - SQLite schema, repositories, queries ✅
- [x] [tasks.indexing.md](tasks.indexing.md) - File scanning, indexing engine, verification ✅
- [x] [tasks.dependency-resolution.md](tasks.dependency-resolution.md) - Dependency graph, cycle detection, state computation, incremental updates ✅

### CLI & Commands
- [x] [tasks.cli-framework.md](tasks.cli-framework.md) - CLI infrastructure, output formatting, progress reporting (Tasks 1-8 complete) ✅
- [x] [tasks.cli-commands.md](tasks.cli-commands.md) - Individual command implementations: lint, format, index, check-index, list, show, search, graph, check-links, agent-prompt (Tasks 1-14 complete) ✅
- [x] [tasks.fuzzy-search.md](tasks.fuzzy-search.md) - Full-text search, ranking, search command (Tasks 1-5 complete) ✅

### User Interfaces
- [x] [tasks.tui.md](tasks.tui.md) - Terminal UI implementation ✅
- [ ] [tasks.task-creation-ui.md](tasks.task-creation-ui.md) - Task creation via CLI (`lash add`) and TUI form modal

### Advanced Features
- [x] [tasks.agent-integration.md](tasks.agent-integration.md) - Agent prompts, token minimization (Tasks 1-6 complete) ✅

### Content Features
- [ ] [tasks.description-section.md](tasks.description-section.md) - `## Description` section support (Tasks 1-6, 8 complete; Task 7 CLI in progress)

### Quality & Documentation
- [x] [tasks.testing.md](tasks.testing.md) - Test strategy, integration tests, benchmarks, CI/CD (Tasks 1-9 complete) ✅
- [ ] [tasks.documentation.md](tasks.documentation.md) - User guide, developer docs, examples, README

## Implementation Phases

### Phase 1: Foundation (Weeks 1-2) - COMPLETE ✅
**Goal:** Set up project infrastructure and core data model

1. ✅ tasks.project-setup (ALL) - Complete
2. ✅ tasks.core-data-model (ALL) - Complete
3. ✅ tasks.error-handling#1 - Complete

**Deliverable:** Compiling project with core types; can create tasks programmatically
**Status:** Complete. Project infrastructure established with 6 crates, error taxonomy, and core data model.

### Phase 2: Core Functionality (Weeks 3-5) - COMPLETE ✅
**Goal:** Parse Markdown files and implement linting/validation

1. ✅ tasks.markdown-parser (ALL) - Complete
2. ✅ tasks.linter (ALL) - Complete

**Deliverable:** Can parse task files into data structures; lint and format files
**Status:** Parser complete (390+ tests, 67.7µs benchmark). Linter complete with 20 rules, auto-formatter, and CLI integration (607 total tests).

### Phase 3: Database & Indexing (Weeks 6-7) - COMPLETE ✅
**Goal:** Build database schema and indexing system

1. ✅ tasks.sqlite-schema#1-2 (define schema, create tables) - Complete
2. ✅ tasks.sqlite-schema#3-6 (repositories, queries) - Complete
3. ✅ tasks.indexing#0-6 (project root, file scanning, diff, indexing engine, verification, dependency updates, performance optimization) - Complete

**Deliverable:** `lash index` command works; can query database
**Status:** Complete. Indexing engine production-ready with 238 tests passing. Performance exceeds all targets by 8-12x (10.5ms for 10 files, 61ms for 100 files, 425ms for 1000 files).

### Phase 4: Dependencies & Queries (Weeks 7-8) - COMPLETE ✅
**Goal:** Resolve dependencies; implement query commands

1. ✅ tasks.dependency-resolution#1-7 (graph, cycles, resolution, status, blockers, export, incremental updates) - Complete
2. ✅ tasks.cli-framework#1-8 (CLI infrastructure: arg parsing, root detection, output formatting, progress, config, logging, command execution, exit codes) - Complete
3. ✅ tasks.cli-commands#1-6 (lint, format, index, check-index, list, show commands) - Complete
4. tasks.linter#4 (dependency validation) - Deferred to Phase 5

**Deliverable:** Can query tasks; dependency resolution works
**Status:** Complete. All core CLI commands implemented with 862 tests passing. Dependency resolution (495 tests), CLI framework (626 tests), and command implementations (1,056 total tests across all crates) all working.

### Phase 5: Search & Advanced Commands (Weeks 9-10) - COMPLETE ✅
**Goal:** Fuzzy search; additional features

1. ✅ tasks.fuzzy-search (ALL) - Complete
2. ✅ tasks.cli-commands#7-14 (search, graph, check-links, agent-prompt, advanced features) - Complete
3. ✅ tasks.dependency-resolution#4-5 - Complete

**Deliverable:** All core CLI commands functional
**Status:** Complete. All advanced commands implemented including search (FTS5), graph (ASCII/DOT/Mermaid/JSON), check-links with fuzzy fix suggestions. 1,604+ total tests passing.

### Phase 6: Agent Integration (Week 11) - COMPLETE ✅
**Goal:** Make Lash usable by AI agents

1. ✅ tasks.agent-integration#1-6 (schema, prompts, token utils, sparse context, CLI command, workflows) - Complete
2. tasks.error-handling#2-4 - Deferred

**Deliverable:** Agents can use Lash effectively
**Status:** Complete. Agent integration production-ready with schema generation, prompt templates, token minimization, sparse context generation, `lash agent-prompt` command, and comprehensive workflow documentation. 40 tests + 22 doctests passing.

### Phase 7: TUI (Week 12) - COMPLETE ✅
**Goal:** Terminal UI for interactive use

1. ✅ tasks.tui (ALL) - Complete (Task 5 Agent View deferred to future)

**Deliverable:** `lash tui` command provides interactive interface
**Status:** Complete. Full TUI with navigation pane, detail view, search modal, label filter, theme selector (300+ Gogh schemes), tree view with expand/collapse. Task 5 (Agent View Mode) deferred intentionally.

### Phase 8: Testing & Documentation (Week 13) - IN PROGRESS ⚙️
**Goal:** Polish and prepare for release

1. ✅ tasks.testing (ALL) - Complete
2. tasks.documentation#1-8 - Not started

**Deliverable:** Production-ready v1.0 release
**Status:** Testing complete with 1,604+ tests, >80% coverage, CI/CD configured. Documentation tasks not yet started.

### Phase 9: Content Features (Week 14) - IN PROGRESS ⚙️
**Goal:** Add description section feature for richer file context

1. ✅ tasks.description-section#1-6 (design doc, parser, linter, schema, indexing, FTS5 search) - Complete
2. ⚙️ tasks.description-section#7 (CLI commands) - In progress (`show` done; `list`, `agent-prompt` pending)
3. ✅ tasks.description-section#8 (TUI display) - Complete

**Deliverable:** Task files support optional `## Description` section with length validation and full-text search
**Status:** Core functionality complete. CLI enhancements for `list` and `agent-prompt` pending.

## Critical Path

The following tasks form the critical path and must be completed in order:

```
Foundation → Parsing → Linting → Schema → Indexing → Dependencies → Commands → Polish
```

Parallelization opportunities exist within each phase (see individual task files for details).

## Success Criteria for v1.0 Release

### Must Have
- ✅ Parse valid Lash Markdown files
- ✅ Lint files with clear error messages
- ✅ SQLite database schema with repositories
- ✅ Index files into SQLite database
- ✅ Full dependency resolution and cycle detection
- ✅ `lash list`, `lash show` commands work
- ✅ `lash search` command with full-text search and filters
- ✅ `lash graph` exports dependency graph (ASCII, DOT, Mermaid, JSON formats)
- ✅ Agent integration (`lash agent-prompt`)
- ✅ Comprehensive test coverage (>80%)
- [ ] User documentation

### Should Have
- ✅ TUI for interactive use (Tasks 1-4, 6-10 complete; Task 5 Agent View deferred)
- ✅ Incremental indexing
- ✅ Auto-formatting
- ✅ Performance benchmarks

### Nice to Have
- ⚠️ Advanced agent views (deferred to future)
- ✅ Fuzzy link fixing (`check-links --fix` with suggestions)
- ⚠️ Archive command
- ⚙️ `## Description` section for task files (core complete; CLI enhancements pending)

## Supporting Documentation

The following analysis documents informed this task breakdown:

- `/docs/design-doc.md` - Comprehensive design specification
- `/docs/dependency-graph-analysis.md` - Graph architecture analysis
- `/docs/rust-architecture-recommendations.md` - Rust implementation guidance

## Notes

- All time estimates are rough guidelines; adjust based on actual progress
- Dependencies between tasks are clearly marked in individual task files
- Priority levels: CRITICAL (must have), HIGH (should have), MEDIUM (nice to have), LOW (future)
- Task files use checkbox format to track progress as work proceeds
