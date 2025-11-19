# Lash Development Log

## 2025-11-18 - Error Handling Module Complete (Tasks 1-3)

### Summary
Completed comprehensive error handling implementation (Tasks 1-3 from `tasks/tasks.error-handling.md`). This provides the foundation for all error reporting throughout Lash with rich diagnostics, machine-readable output, and error aggregation capabilities.

### Implementation Details

**Task 1: Error Type Taxonomy** (CRITICAL - Complete)
- Enhanced `LashError` enum with 8 comprehensive error categories:
  - Parse, Lint, Index, Dependency, Query, Config, IO, Internal
- Added 30+ stable error codes following `E_<CATEGORY>_<NUMBER>` convention
- Rich context in all errors: file locations, line/column numbers, code snippets, help text
- Ergonomic helper constructors for every error type (e.g., `LashError::parse_invalid_checkbox(...)`)
- Legacy error code aliases for backward compatibility
- All errors implement `std::error::Error` via thiserror

**Task 2: Error Formatting** (CRITICAL - Complete)
- Created `ErrorFormatter` module (`crates/lash-types/src/formatter.rs`) with three output formats:
  - **Human-readable**: Rich terminal output with colors, context lines, carets pointing to errors (similar to rustc)
  - **JSON**: Structured output with stable schema for machine consumption
  - **Compact**: Single-line format for logging
- Automatically reads source files to show context around errors
- Color-coded output (red for errors, cyan for paths, gray for snippets)
- Contextual help messages for every error type

**Task 3: Error Aggregation** (HIGH - Complete)
- Implemented `ErrorReport` for collecting multiple errors (`crates/lash-types/src/report.rs`)
- Flexible grouping strategies via `GroupBy` enum:
  - By file, error code, severity, or chronological order
- Summary statistics with error counts and breakdown
- Filtering capabilities (by severity, file, or error code)
- Both text and JSON report formats

### Key Design Decisions

1. **Large error type accepted**: The 168-byte `LashError` intentionally contains rich context. This is acceptable for a CLI tool where errors are exceptional, not on hot paths. Added `#![allow(clippy::result_large_err)]` with documentation.

2. **Clean module separation**:
   - `error.rs`: Core error types and taxonomy (1000+ lines)
   - `formatter.rs`: Rich formatting logic
   - `report.rs`: Error aggregation and reporting

3. **Helper constructors**: Every error type has an ergonomic constructor making error creation simple and consistent across the codebase.

4. **Backward compatibility**: Added deprecated aliases for old error codes to ease migration.

### Dependencies Added

- `miette 7.0` with fancy features (for rich diagnostics support)
- `colored 2.1` (for terminal colors)
- `insta 1.34` (for snapshot testing)

### Test Coverage

- **123 tests passing** in lash-types
- Comprehensive unit tests for:
  - Every error type constructor
  - Error code stability
  - Diagnostic conversion
  - JSON serialization
  - Formatter output (human, JSON, compact)
  - Report aggregation, grouping, and filtering

### Files Changed

- Enhanced: `crates/lash-types/src/error.rs` (complete rewrite, 1000+ lines)
- New: `crates/lash-types/src/formatter.rs` (error formatting module)
- New: `crates/lash-types/src/report.rs` (error aggregation module)
- Updated: `tasks/tasks.error-handling.md` (marked Tasks 1-3 complete)
- Updated: `Cargo.toml` (added miette, colored, insta dependencies)

### Deferred Tasks

Tasks 4-6 from the error handling module depend on CLI framework and are deferred:
- Task 4: Agent-Friendly Error Messages (needs CLI integration)
- Task 5: Error Reporting in Commands (needs CLI commands)
- Task 6: Error Recovery and Validation (future enhancement)

### Next Steps

Error handling foundation is complete. Ready to proceed to:
1. Phase 2: Core Functionality (Markdown parser, linter, SQLite schema)
2. Next module: `tasks/tasks.markdown-parser.md`

Git commit: 302089e

---

## 2025-11-17 - Planning Phase Complete

### Summary
Completed comprehensive development planning for Lash v1.0 using three specialized subagents:
- **dev-project-manager** - Task breakdown and implementation sequencing
- **graph-systems-architect** - Dependency model and graph algorithm analysis
- **rust-dev-engineer** - Rust architecture and implementation recommendations

### Deliverables Created

**Analysis Documents:**
1. `docs/dependency-graph-analysis.md` - Comprehensive graph architecture analysis
   - Graph representation strategies (adjacency list with dual indexing)
   - Algorithm specifications (Kahn's algorithm, three-color DFS, reverse topological traversal)
   - SQLite schema with transitive closure optimization
   - Performance targets and phased implementation plan
   - Library recommendations (petgraph, rusqlite)

2. `docs/rust-architecture-recommendations.md` - Rust-specific implementation guidance
   - Refined crate structure (added lash-types for shared types)
   - Critical dependency selections (pulldown-cmark, clap, ratatui, nucleo)
   - Hybrid data structure approach (arena allocation → flat indexed)
   - Performance optimization strategies for critical paths
   - Error handling strategy (thiserror + anyhow)
   - Testing strategy (unit, fixture-based, property-based, integration, snapshot, benchmarks)
   - 10-phase implementation order with vertical slice approach

**Task Management:**
1. `tasks/tasks.md` - Master task index
   - 16 task categories organized by module
   - 8 implementation phases mapped to 13-week timeline
   - Critical path and parallelization opportunities
   - v1.0 success criteria (Must/Should/Nice to Have)

2. `tasks/tasks.project-setup.md` - Foundation tasks (5 tasks, 3-5 days)
   - Rust workspace initialization
   - Development tooling (rustfmt, clippy, pre-commit hooks)
   - Testing infrastructure and fixtures
   - Error taxonomy and diagnostic system
   - Project configuration model

3. `tasks/tasks.core-data-model.md` - Core data structures (6 tasks, 5-7 days)
   - TaskStatus enum with checkbox char mapping
   - Task model with hierarchical parent-child relationships
   - TaskFile model with content hashing
   - Dependency types (Hierarchy, ExplicitId, ExplicitPath, Directory)
   - Label model with parsing and normalization
   - RootIndex model for project structure

4. `DEVELOPMENT_PLAN.md` - Executive summary
   - High-level overview of planning process
   - Architecture and technology decisions
   - 8-phase timeline (40-60 days)
   - Risk assessment and mitigation strategies
   - Getting started guide

### Key Architectural Decisions

**Crate Structure:**
- Refined from 5 to 6 crates (added `lash-types` for shared types)
- Clean separation: types → core → db/agent/tui → cli

**Data Model:**
- Hybrid approach: arena allocation during parsing, flat indexed for storage
- Four dependency types covering all use cases
- Task depth limit: 3 levels recommended
- Indentation: 2 spaces per level recommended

**Technology Stack:**
- Markdown: `pulldown-cmark` (streaming, fast, CommonMark compliant)
- CLI: `clap` v4 with derive macros
- Database: `rusqlite` with bundled SQLite
- TUI: `ratatui` + `crossterm`
- Search: `nucleo` (TUI) + FTS5 (CLI)
- Graphs: `petgraph` for dependency resolution
- Errors: `thiserror` (libs) + `anyhow` (CLI)
- Testing: `proptest` + `criterion` + `insta`

**Performance Targets:**
- Parsing: <100ms for pre-commit hooks
- Indexing: 1000+ files in <5s
- Search: <100ms for typical queries
- Blocker checks: <1ms

### Open Design Decisions (Non-blocking)

These can be finalized during Phase 1 implementation:
1. Header format: Recommend `@key: value` (no YAML frontmatter)
2. Max depth: Recommend 3 levels
3. Indentation: Recommend 2 spaces
4. Fuzzy search: Recommend FTS5 initially
5. TUI library: Recommend ratatui

### Timeline Estimate

**Total: 40-60 days (8-13 weeks)**

- Phase 1: Foundation (Weeks 1-2) - Project setup, core types
- Phase 2: Core (Weeks 3-5) - Parsing, linting, schema
- Phase 3: Indexing (Weeks 6-7) - File scanning, database building
- Phase 4: Dependencies (Weeks 7-8) - Graph resolution, cycle detection
- Phase 5: Search (Weeks 9-10) - Fuzzy search, advanced commands
- Phase 6: Agents (Week 11) - Prompt generation, token minimization
- Phase 7: TUI (Week 12) - Terminal interface
- Phase 8: Polish (Week 13) - Testing, docs, benchmarks

### Risk Assessment

**High-risk areas identified:**
1. Parser complexity - mitigated by using pulldown-cmark
2. Dependency resolution performance - mitigated by petgraph + caching
3. TUI complexity - contingency: ship v1 without TUI if needed
4. Cross-platform issues - mitigate with early multi-platform testing

### Next Steps

**Immediate:**
1. Create remaining 14 detailed task files (parser, linter, db, etc.)
2. Finalize open design decisions
3. Begin Phase 1: tasks.project-setup.md

**Week 1-2:**
- Complete project setup (workspace, tooling, tests, errors, config)
- Complete core data model (status, task, file, dependency, label, index)
- Verify foundation is solid before proceeding

**Week 3+:**
- Follow phase plan in tasks/tasks.md
- Track progress by checking off tasks
- Update devlog with decisions and progress

### Notes

Planning leveraged three specialized subagents working in parallel:
- Each provided deep analysis in their domain (PM, graph theory, Rust)
- Analysis documents provide detailed guidance for implementation
- Task breakdown is comprehensive with clear dependencies and estimates
- Architecture decisions are well-justified with trade-offs documented

The design document (docs/design-doc.md) proved to be comprehensive and implementation-ready with only minor gaps (header format, depth limit) that don't block starting development.

Project is ready to begin implementation in Phase 1.

---

## 2025-11-17 - Design Decisions Finalized

### Summary
All open design decisions have been resolved through iterative user consultation. Decisions documented in `docs/design-decisions.md`.

### Decisions Made

1. **Header Format:** @-annotations only (no YAML frontmatter)
   - Simpler, consistent, agent-friendly

2. **Maximum Task Depth:** 3 levels (depth 0, 1, 2)
   - Encourages shallow hierarchies and file decomposition

3. **Indentation:** 2 spaces per level
   - Standard Markdown convention

4. **Fuzzy Search:** Hybrid approach
   - SQLite FTS5 for CLI commands
   - nucleo for TUI interactive search

5. **TUI Library:** ratatui + crossterm
   - Industry standard, great documentation

6. **File Organization:** Nested directories
   - Natural hierarchy, intuitive for users

7. **Waived Task Behavior:** Automatically waive children
   - Simpler mental model, consistent semantics

8. **Database Location:** `.lash/lash.db`
   - Follows .git pattern, keeps root clean

9. **Unknown Annotations:** Strict validation with opt-in custom keys
   - Config file allows users to define custom @keys with descriptions
   - Catches typos while enabling extensibility

10. **Root Index Filename:** Support both `lash.index.md` and `index.lash.md`
    - User flexibility, prefer lash.index.md if both exist

### Key Design Choice: Custom Annotations

The custom annotation approach is particularly noteworthy:
- Users can define custom @keys in `.lash/config.toml`
- Each custom key includes a description
- Linter strictly validates against built-in + configured custom keys
- Prevents drift while enabling project-specific metadata

Example config:
```toml
[annotations]
custom_keys = [
  { key = "priority", description = "Task priority (1-5)" },
  { key = "sprint", description = "Sprint number" },
]
```

### Impact on Implementation

These decisions clarify several implementation details:
- Parser is simpler (no YAML support needed)
- Linter rules are now concrete (depth=3, indent=2, strict annotations)
- Search implementation split clearly (FTS5 vs nucleo)
- Config schema expanded to support custom annotation definitions

### Next Steps

All design decisions resolved. Ready to:
1. Create remaining 14 detailed task files
2. Begin Phase 1 implementation (tasks.project-setup.md)

Project is fully planned and ready to begin implementation.
