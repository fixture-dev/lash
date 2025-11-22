# Testing Strategy Tasks

**Module:** All modules (cross-cutting)
**Dependencies:** All implementation tasks
**Effort:** 8-12 days (ongoing throughout development)
**Priority:** CRITICAL

## Overview

Establish comprehensive testing strategy covering unit tests, integration tests, end-to-end tests, and performance benchmarks. Testing ensures correctness, prevents regressions, and validates that Lash meets its design goals.

## Core Requirements

From CLAUDE.md:
- Write appropriate layers of tests (unit, integration, end-to-end)
- Prove functionality works and guard against regressions
- Don't write frivolous tests to meet coverage thresholds
- Don't add test-related cases to production code
- Target: >80% test coverage for v1.0 release

---

## Task 1: Testing Infrastructure Setup

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.project-setup.md

### Description

Set up testing infrastructure, fixtures, and utilities for all test types.

### Subtasks

- [ ] Configure Cargo test framework
  - [ ] Set up `tests/` directory for integration tests
  - [ ] Configure test profiles (dev, CI)
  - [ ] Enable parallel test execution
- [ ] Add testing dependencies
  - [ ] `assert_cmd` for CLI testing
  - [ ] `predicates` for assertions
  - [ ] `tempfile` for temp directories
  - [ ] `insta` for snapshot testing
  - [ ] `rstest` for parameterized tests (optional)
  - [ ] `mockall` for mocking (minimal use)
- [ ] Create test fixture library
  - [ ] Valid task files (various scenarios)
  - [ ] Invalid task files (for error testing)
  - [ ] Small, medium, large projects (for performance)
  - [ ] Projects with dependencies, cycles, etc.
  - [ ] Store in `tests/fixtures/`
- [ ] Implement test utilities
  - [ ] `TestProject` builder for creating temp projects
  - [ ] `assert_file_contents()` helper
  - [ ] `run_lash_command()` helper for CLI tests
  - [ ] `parse_json_output()` helper
  - [ ] DB inspection utilities
- [ ] Set up test databases
  - [ ] In-memory SQLite for fast unit tests
  - [ ] File-based SQLite for integration tests
  - [ ] Cleanup strategy (temp dirs)
- [ ] Configure coverage tracking
  - [ ] Use `cargo-tarpaulin` or `cargo-llvm-cov`
  - [ ] Generate coverage reports
  - [ ] Set minimum coverage threshold (80%)

### Success Criteria

- Test infrastructure is easy to use
- Fixtures cover common scenarios
- Test utilities reduce boilerplate
- Coverage tracking is automated

### Tests

- Meta: Tests for test utilities (if complex)
- CI: Coverage reports generate successfully

---

## Task 2: Unit Tests

**Priority:** CRITICAL
**Effort:** Ongoing (2-3 days distributed across modules)
**Depends on:** Task 1, each module implementation

### Description

Write comprehensive unit tests for all core functions and structs in each module.

### Subtasks

- [ ] **Core data model tests**
  - [ ] Task creation and validation
  - [ ] Status transitions
  - [ ] Label parsing and formatting
  - [ ] ID generation and uniqueness
- [ ] **Markdown parser tests**
  - [ ] Parse valid task files
  - [ ] Parse invalid task files (errors)
  - [ ] Parse edge cases (empty, deeply nested, etc.)
  - [ ] Annotation parsing
  - [ ] Checkbox status parsing
- [ ] **Linter tests**
  - [ ] Each linting rule
  - [ ] Error message formatting
  - [ ] Auto-fix logic
  - [ ] Severity levels
- [ ] **Dependency resolution tests**
  - [ ] Graph construction
  - [ ] Cycle detection
  - [ ] Status computation
  - [ ] Blocker identification
  - [ ] Reference resolution (various formats)
- [ ] **Database tests**
  - [ ] Schema creation
  - [ ] CRUD operations
  - [ ] Query correctness
  - [ ] Transaction handling
  - [ ] Index integrity
- [ ] **Indexing tests**
  - [ ] File discovery
  - [ ] Diff computation
  - [ ] Index execution
  - [ ] Incremental updates
  - [ ] Verification logic
- [ ] **Search tests**
  - [ ] Query parsing
  - [ ] FTS5 queries
  - [ ] Ranking algorithm
  - [ ] Filtering
  - [ ] Pagination
- [ ] **CLI framework tests**
  - [ ] Argument parsing
  - [ ] Project root detection
  - [ ] Output formatting (text, JSON)
  - [ ] Configuration loading
  - [ ] Error handling
- [ ] **Error handling tests**
  - [ ] Error construction
  - [ ] Error formatting (text, JSON)
  - [ ] Error aggregation
  - [ ] Exit code mapping

### Success Criteria

- All public functions have unit tests
- Edge cases and error paths are covered
- Tests are fast (<100ms each)
- Tests are deterministic (no flakiness)

### Tests

- Unit tests for each module (see subtasks)
- Coverage: >80% line coverage per module

---

## Task 3: Integration Tests

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 1, major module implementations

### Description

Write integration tests that exercise multiple modules working together.

### Subtasks

- [ ] **Parse → Lint workflow**
  - [ ] Parse file, lint, verify errors
  - [ ] Parse and lint valid files (no errors)
  - [ ] Auto-fix integration
- [ ] **Index → Query workflow**
  - [ ] Index project, query tasks
  - [ ] Filter by label, status, path
  - [ ] Search integration
  - [ ] Verify DB consistency
- [ ] **Dependency resolution workflow**
  - [ ] Parse files with dependencies
  - [ ] Build graph
  - [ ] Compute status
  - [ ] Identify blockers
  - [ ] Export graph
- [ ] **CLI command integration**
  - [ ] `lash lint` on fixture projects
  - [ ] `lash index` on fixture projects
  - [ ] `lash list` with various filters
  - [ ] `lash show` for tasks and files
  - [ ] `lash search` with queries
  - [ ] `lash graph` export
  - [ ] `lash check-links`
  - [ ] `lash agent-prompt` generation
- [ ] **Incremental operations**
  - [ ] Index, modify file, re-index (incremental)
  - [ ] Verify only changed files re-indexed
  - [ ] Verify dependency updates
- [ ] **Error handling across modules**
  - [ ] Parse error → formatted output
  - [ ] Lint error → error report
  - [ ] Broken dependency → error message
  - [ ] DB error → recovery

### Success Criteria

- Integration tests cover major workflows
- Tests use realistic fixture data
- Tests verify end-to-end correctness
- Tests are reliable and reproducible

### Tests

- Integration tests for each workflow (see subtasks)
- Coverage: Major code paths exercised

---

## Task 4: End-to-End (E2E) CLI Tests

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 1, tasks.cli-commands.md (all commands implemented)

### Description

Write end-to-end tests that invoke the `lash` binary as a user would, verifying CLI behavior.

### Subtasks

- [ ] **Command execution tests**
  - [ ] Test each command with various flags
  - [ ] Verify stdout output
  - [ ] Verify stderr output
  - [ ] Verify exit codes
  - [ ] Verify file modifications (for `format`)
- [ ] **User workflow tests**
  - [ ] Create project → index → lint → query
  - [ ] Add tasks → re-index → verify in DB
  - [ ] Search → show task → verify details
  - [ ] Generate graph → verify DOT syntax
- [ ] **Error scenario tests**
  - [ ] Command on non-project directory (error)
  - [ ] Lint invalid files (show errors)
  - [ ] Query non-existent tasks (not found)
  - [ ] Index corrupted files (collect errors)
- [ ] **Output format tests**
  - [ ] `--json` flag for all commands
  - [ ] Verify JSON is parseable
  - [ ] Text output is readable
- [ ] **Config and flags tests**
  - [ ] `--root` flag overrides detection
  - [ ] `--verbose` increases output
  - [ ] `--quiet` suppresses output
  - [ ] Config file settings are applied
- [ ] **Cross-platform tests** (if applicable)
  - [ ] Test on Linux, macOS, Windows
  - [ ] Path handling differences
  - [ ] Line ending differences

### Success Criteria

- E2E tests cover all commands
- Tests verify actual binary behavior (not mocked)
- Tests catch integration issues
- Tests run in CI automatically

### Tests

- E2E tests for each command (see subtasks)
- Snapshot tests for output formatting

---

## Task 5: Performance Benchmarks

**Priority:** MEDIUM
**Effort:** 2-3 days
**Depends on:** Task 1, major module implementations

### Description

Implement performance benchmarks to ensure Lash meets speed targets and to detect regressions.

### Subtasks

- [ ] Add benchmarking infrastructure
  - [ ] Use `criterion` crate
  - [ ] Set up `benches/` directory
  - [ ] Configure benchmark profiles
- [ ] Implement parsing benchmarks
  - [ ] Parse small file (10 tasks)
  - [ ] Parse medium file (100 tasks)
  - [ ] Parse large file (1000 tasks)
  - [ ] Measure throughput (tasks/sec)
- [ ] Implement linting benchmarks
  - [ ] Lint small, medium, large files
  - [ ] Measure throughput
- [ ] Implement indexing benchmarks
  - [ ] Index small project (10 files)
  - [ ] Index medium project (100 files)
  - [ ] Index large project (1000 files)
  - [ ] Incremental indexing (modify 10% of files)
  - [ ] Measure time and throughput
- [ ] Implement query benchmarks
  - [ ] Simple query (by label)
  - [ ] Complex query (multiple filters)
  - [ ] Search query
  - [ ] Graph export
  - [ ] Measure query time
- [ ] Implement dependency resolution benchmarks
  - [ ] Build graph (small, medium, large)
  - [ ] Detect cycles
  - [ ] Compute status
  - [ ] Measure time
- [ ] Set performance targets
  - [ ] Parse: >1000 tasks/sec
  - [ ] Lint: >500 tasks/sec
  - [ ] Index (full): <5s for 1000 files
  - [ ] Index (incremental): <1s for 100 changed files
  - [ ] Query: <100ms for typical filters
  - [ ] Search: <200ms for typical query
- [ ] Document benchmarks
  - [ ] How to run benchmarks
  - [ ] How to interpret results
  - [ ] Historical performance data (track regressions)

### Success Criteria

- Benchmarks cover major operations
- Performance targets are met
- Benchmarks run in CI (report results)
- Regressions are detected

### Tests

- Benchmark suite for each module (see subtasks)
- CI: Benchmarks run and report to dashboard (optional)

---

## Task 6: Regression Tests and Fixtures

**Priority:** MEDIUM
**Effort:** 1-2 days (ongoing)
**Depends on:** Task 1

### Description

Build a comprehensive set of regression test fixtures and tests to prevent bugs from reoccurring.

### Subtasks

- [ ] Create fixture library
  - [ ] Small projects (5-10 files)
  - [ ] Medium projects (50-100 files)
  - [ ] Large projects (500-1000 files)
  - [ ] Projects with various structures:
    - [ ] Flat (all files in root)
    - [ ] Nested (deep directory trees)
    - [ ] Mixed (some nested, some flat)
  - [ ] Projects with edge cases:
    - [ ] Circular dependencies
    - [ ] Broken links
    - [ ] Deeply nested tasks
    - [ ] Very long task lists
    - [ ] Unicode filenames and content
- [ ] Implement regression test suite
  - [ ] Test each fixture with all commands
  - [ ] Verify expected behavior (snapshots)
  - [ ] Test error cases (broken fixtures)
- [ ] Add snapshot testing
  - [ ] Use `insta` crate
  - [ ] Capture command output
  - [ ] Review and approve snapshots
  - [ ] Detect unexpected changes
- [ ] Document fixtures
  - [ ] README in `tests/fixtures/`
  - [ ] Explain each fixture scenario
  - [ ] How to add new fixtures

### Success Criteria

- Fixtures cover diverse scenarios
- Regression tests prevent known bugs
- Snapshots catch unexpected output changes
- Fixtures are documented and maintainable

### Tests

- Regression test suite (see subtasks)
- Snapshot tests for command outputs

---

## Task 7: Test Coverage and Quality

**Priority:** MEDIUM
**Effort:** 1 day (plus ongoing monitoring)
**Depends on:** Task 1-6

### Description

Ensure test coverage is high and tests are of good quality (not frivolous).

### Subtasks

- [ ] Measure test coverage
  - [ ] Use `cargo-llvm-cov` or `cargo-tarpaulin`
  - [ ] Generate coverage report (HTML)
  - [ ] Identify uncovered code
- [ ] Set coverage targets
  - [ ] Overall: >80% line coverage
  - [ ] Critical modules: >90% (parser, linter, dependency)
  - [ ] Less critical: >70% (TUI, agent utils)
- [ ] Review test quality
  - [ ] Remove frivolous tests (testing stdlib)
  - [ ] Ensure tests are meaningful
  - [ ] Tests don't rely on implementation details
  - [ ] Tests are maintainable
- [ ] Add coverage to CI
  - [ ] Run coverage on every PR
  - [ ] Fail CI if coverage drops below threshold
  - [ ] Report coverage to PR comments (optional)
- [ ] Document testing guidelines
  - [ ] What to test (and what not to)
  - [ ] How to write good tests
  - [ ] How to run tests locally
  - [ ] How to update snapshots

### Success Criteria

- Coverage targets are met
- Tests are high quality and maintainable
- Coverage is tracked in CI
- Guidelines help contributors write good tests

### Tests

- Meta: Test quality review (manual)
- CI: Coverage gates pass

---

## Task 8: CI/CD Integration

**Priority:** HIGH
**Effort:** 1-2 days
**Depends on:** Task 1-7

### Description

Integrate all tests and checks into CI/CD pipeline (GitHub Actions or similar).

### Subtasks

- [ ] Set up CI configuration
  - [ ] Use GitHub Actions (or GitLab CI, etc.)
  - [ ] Define workflows for:
    - [ ] Unit tests
    - [ ] Integration tests
    - [ ] E2E tests
    - [ ] Linting (clippy)
    - [ ] Formatting (rustfmt)
    - [ ] Coverage
    - [ ] Benchmarks (optional, report only)
- [ ] Configure test matrix
  - [ ] Test on multiple Rust versions (stable, beta, MSRV)
  - [ ] Test on multiple platforms (Linux, macOS, Windows)
  - [ ] Use matrix strategy for parallelism
- [ ] Add pre-commit hooks
  - [ ] Run tests before commit (optional)
  - [ ] Run lint and format checks
  - [ ] Use `pre-commit` framework or shell script
- [ ] Configure failure policies
  - [ ] Fail on test failures
  - [ ] Fail on coverage drop
  - [ ] Fail on lint errors
  - [ ] Warn on benchmark regressions
- [ ] Add status badges
  - [ ] CI status badge in README
  - [ ] Coverage badge
  - [ ] License badge
- [ ] Document CI/CD
  - [ ] How to run CI locally (with `act` or similar)
  - [ ] How to debug CI failures
  - [ ] How to update CI config

### Success Criteria

- All tests run automatically on every PR
- CI catches bugs before merge
- CI is fast (<10 minutes for full suite)
- CI is reliable (no flaky tests)

### Tests

- CI: All workflows pass on main branch
- CI: Verify failure scenarios trigger correctly

---

## Task 9: Playground Mode for Demos and Exploration

**Priority:** MEDIUM
**Effort:** 2-3 days
**Depends on:** Task 1 (Testing Infrastructure), Task 6 (Fixtures), CLI and TUI implementations

### Description

Create an interactive playground mode that seeds a realistic, complex demo project for manual testing, demos, and exploratory usage. The playground uses a fictional 2D platformer game development project that showcases all of Lash's features in a relatable, engaging context.

### Subtasks

- [ ] **Playground CLI command**
  - [ ] Add `lash playground init [--path PATH]` command
  - [ ] Create playground directory with fresh demo content
  - [ ] Support `--reset` flag to regenerate from scratch
  - [ ] Print welcome message with usage instructions
  - [ ] Auto-index after generation
- [ ] **Demo project theme: "PixelQuest" 2D Platformer**
  - [ ] Game concept: Retro-style platformer with procedural levels
  - [ ] Project structure mirrors real game development
  - [ ] Realistic complexity (50-80 task files)
  - [ ] Mix of code, art, design, and management tasks
- [ ] **Core task file structure**
  - [ ] `index.lash.md` - Master project index
  - [ ] `features/` - Game features and mechanics
    - [ ] `features/player-movement.md` - Physics, controls, animations
    - [ ] `features/enemy-ai.md` - Behavior trees, pathfinding
    - [ ] `features/level-generation.md` - Procedural algorithms
    - [ ] `features/power-ups.md` - Item system, effects
    - [ ] `features/boss-fights.md` - Special encounters
  - [ ] `systems/` - Core engine systems
    - [ ] `systems/rendering.md` - Graphics pipeline
    - [ ] `systems/audio.md` - Sound engine, music
    - [ ] `systems/physics.md` - Collision, forces
    - [ ] `systems/input.md` - Controller mapping
  - [ ] `content/` - Art and design tasks
    - [ ] `content/sprites.md` - Character and tile art
    - [ ] `content/animations.md` - Frame sequences
    - [ ] `content/music.md` - Soundtrack composition
    - [ ] `content/sfx.md` - Sound effects
    - [ ] `content/levels.md` - Level design
  - [ ] `infrastructure/` - Dev ops and tools
    - [ ] `infrastructure/build-pipeline.md` - CI/CD, releases
    - [ ] `infrastructure/asset-pipeline.md` - Import, optimization
    - [ ] `infrastructure/testing.md` - Test framework
  - [ ] `design/` - Game design documents
    - [ ] `design/core-loop.md` - Gameplay flow
    - [ ] `design/progression.md` - Difficulty curve
    - [ ] `design/story.md` - Narrative elements
  - [ ] `milestones/` - Release planning
    - [ ] `milestones/alpha.md` - Initial playable
    - [ ] `milestones/beta.md` - Feature complete
    - [ ] `milestones/release.md` - Polish and ship
- [ ] **Dependency examples**
  - [ ] Parent/child relationships (features broken into subtasks)
  - [ ] Cross-file dependencies (rendering depends on physics)
  - [ ] Directory-level dependencies (alpha milestone depends on core features)
  - [ ] Circular dependency example (intentional, for testing)
  - [ ] Broken link example (intentional, for testing)
- [ ] **Label examples**
  - [ ] `#backend` - Engine and systems code
  - [ ] `#frontend` - UI and player-facing features
  - [ ] `#art` - Graphics and visual content
  - [ ] `#audio` - Sound and music
  - [ ] `#design` - Game design work
  - [ ] `#tooling` - Dev tools and pipeline
  - [ ] `#p0`, `#p1`, `#p2` - Priority levels
  - [ ] `#blocked` - Waiting on something
  - [ ] `#bug` - Known issues
  - [ ] `#polish` - Quality improvements
- [ ] **Status variety**
  - [ ] Mix of open, done, waived, blocked tasks
  - [ ] Completed early milestones (demo alpha phase)
  - [ ] In-progress current work (beta phase)
  - [ ] Future work (release phase mostly open)
- [ ] **Annotation examples**
  - [ ] `@owner` - Various team members (Alice, Bob, Carol, etc.)
  - [ ] `@estimate` - Time estimates (1d, 3d, 1w)
  - [ ] `@created` - Dates spanning several months
  - [ ] `@depends-on` - Cross-file dependencies
  - [ ] `@agent-note` - Hints for AI assistants
  - [ ] `@blocked-by` - Explicit blockers
- [ ] **Playground utilities**
  - [ ] Reuse `TestProject` builder from test infrastructure
  - [ ] Reuse fixture generation logic from Task 6
  - [ ] Add playground-specific utilities:
    - [ ] `reset_playground()` - Clear and regenerate
    - [ ] `add_random_task()` - Insert tasks for experimentation
    - [ ] `simulate_work()` - Toggle random tasks to simulate progress
  - [ ] Store templates in `playground/templates/`
- [ ] **Interactive walkthrough**
  - [ ] Generate `PLAYGROUND_GUIDE.md` with the project
  - [ ] Suggest CLI commands to try (list, search, show, graph)
  - [ ] Suggest TUI workflows (navigate, toggle, filter)
  - [ ] Point out interesting features to explore:
    - [ ] Dependency chains
    - [ ] Search for specific labels
    - [ ] Graph visualization of subsystems
    - [ ] Broken links and cycles (for error testing)
- [ ] **Integration with test fixtures**
  - [ ] Playground shares code with Task 6 fixture library
  - [ ] Playground templates can be used as regression fixtures
  - [ ] Maintain DRY principle - single source for demo content
- [ ] **Documentation**
  - [ ] Add playground section to README
  - [ ] Document how to reset and regenerate
  - [ ] Explain demo project structure
  - [ ] Provide example command workflows

### Success Criteria

- Playground initializes in <5 seconds
- Demo project feels realistic and engaging
- All Lash features are represented (dependencies, labels, statuses, etc.)
- Both CLI and TUI work seamlessly with playground data
- Easy to reset and experiment without fear
- Useful for demos, tutorials, and manual exploration

### Tests

- Unit: Playground generation functions
- Integration: `lash playground init` creates valid project
- E2E: Full workflow (init → index → TUI → CLI commands)
- Verify all generated files pass `lash lint`
- Verify `lash index` succeeds on playground
- Verify `lash graph` produces valid DOT output

### Example Usage

```bash
# Initialize playground in current directory
lash playground init

# Or specify a path
lash playground init --path ~/lash-demo

# Explore with CLI
cd ~/lash-demo
lash list --label art
lash search "boss fight"
lash show features/boss-fights.md#final-boss
lash graph --output game-systems.dot

# Explore with TUI
lash tui

# Reset to fresh state
lash playground init --reset
```

---

## Non-Goals (for v1)

- Property-based testing (fuzzing) - defer to v2
- Mutation testing - defer to v2
- Stress testing (>10k files) - manual only
- Performance regression tracking dashboard - basic reporting only

---

## Open Questions

- **Test parallelism:** Run tests in parallel by default or serial?
- **Snapshot review:** Automate or manual review?
- **Benchmark CI:** Run on every PR or nightly only?
- **MSRV:** What's the minimum supported Rust version?

---

## References

- CLAUDE.md (General Development Practices)
- Design doc section 13 (Architecture & Implementation Plan)
- Rust testing guide: https://doc.rust-lang.org/book/ch11-00-testing.html
- `criterion` docs: https://docs.rs/criterion/
- `insta` docs: https://docs.rs/insta/
- `assert_cmd` docs: https://docs.rs/assert_cmd/
