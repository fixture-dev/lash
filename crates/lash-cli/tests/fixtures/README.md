# Test Fixtures

This directory contains comprehensive test fixtures for Lash integration tests, regression tests, and performance benchmarks.

## Directory Structure

### Single Files

- **`valid/`** - Valid Lash markdown files for testing successful parsing and operations
  - Basic task files with various annotation combinations
  - Edge cases: empty lists, unicode content, large lists (100, 500 tasks), maximum nesting
  - Task files demonstrating estimates, blockers, agent notes, and waived tasks
  - **Unicode filenames:** `日本語-タスク.md`, `задачи-список.md`, `المهام.md`, `emoji-🚀-tasks.md`
  - **File characteristics:** `no-trailing-newline.md`, `crlf-line-endings.md`, `mixed-line-endings.md`
  - **Performance tests:** `very-long-list-100.md` (100 tasks), `very-long-list-500.md` (500 tasks)

- **`invalid/`** - Invalid files for testing error handling and validation
  - Missing or malformed annotations
  - Invalid status values and checkbox formats
  - Duplicate IDs
  - Depth exceeded errors
  - **Broken links:** `broken-link-missing-file.md`, `broken-link-missing-id.md`
  - **Circular dependencies:** `circular-3-files/` directory with A→C→B→A cycle

### Project Repositories

- **`repos/`** - Complete multi-file test repositories

#### Production-Scale Projects

- **`small-project/`** - 3 files, 10-15 tasks
  - Minimal test project for quick integration tests
  - Flat hierarchy
  - Basic CLI command testing, simple indexing

- **`medium-project/`** - 10 files, 30-50 tasks
  - Realistic fullstack application demo
  - Structure: backend/, frontend/, docs/ directories
  - Demonstrates dependency resolution, label filtering, cross-file queries

- **`medium-project-realistic/`** - 23 files, ~250 tasks ⭐
  - **NEW:** Generated realistic e-commerce platform
  - Structure: backend/, frontend/, mobile/, infrastructure/, docs/
  - Comprehensive labels: #backend, #frontend, #mobile, #infrastructure, #docs, #p0, #p1, #security, etc.
  - Cross-file dependencies demonstrating realistic workflow
  - Generated using `generators/mod.rs` fixture generator

- **`large-project/`** - 20+ files, 100+ tasks
  - Enterprise-scale testing
  - Multiple teams/modules
  - Performance testing, graph visualization, search functionality

#### Structure Variants

- **`flat-project/`** - 10 files, all in root directory ⭐
  - **NEW:** Tests scanner behavior with completely flat hierarchy
  - No subdirectories
  - Dependencies across files at same level
  - ~60 tasks total

- **`deeply-nested/`** - 9 files, 8 levels deep ⭐
  - **NEW:** Tests deep directory nesting (level1/.../level8/)
  - Each level depends on parent level
  - Tests path handling and relative dependency resolution
  - ~40 tasks total

- **`mixed-structure/`** - 7 files, hybrid flat + nested ⭐
  - **NEW:** Tests mixed organization patterns
  - Some files in root, others 3-4 levels deep
  - Dependencies crossing between root and nested files
  - ~35 tasks total

### Generator Infrastructure

- **`generators/`** - Fixture generation code
  - **`mod.rs`** - `ProjectGenerator` with fluent API for creating realistic projects
  - Template-based file generation
  - Realistic task text (not Lorem Ipsum)
  - Dependency graph generation utilities
  - See `tests/generate_realistic_project.rs` for usage example

## Usage

### Loading Fixtures in Tests

Test fixtures can be loaded using the helper functions in `tests/common/mod.rs`:

```rust
use common::{load_fixture, TestProject};

// Load a single fixture file
let content = load_fixture("valid/simple-task.md");

// Create a temporary project from a fixture repository
let project = TestProject::from_fixture("small");
let project = TestProject::from_fixture("medium");
let project = TestProject::from_fixture("flat");
let project = TestProject::from_fixture("deeply-nested");
let project = TestProject::from_fixture("mixed-structure");

// Build a custom test project
let project = TestProject::builder()
    .with_index("test-project", "Test Project")
    .with_task_file("tasks.md", "tasks", "Tasks")
    .build();
```

### Using the Fixture Generator

To regenerate `medium-project-realistic` or create similar projects:

```bash
# Run the generator test (ignored by default)
cargo test --test generate_realistic_project -- --ignored --nocapture

# Or use the generator API directly in your tests
use fixtures::generators::generate_ecommerce_project;
generate_ecommerce_project(&output_dir)?;
```

See `generators/mod.rs` for the fluent API:

```rust
use generators::ProjectGenerator;

let gen = ProjectGenerator::new("my-project", "My Project")
    .with_labels(vec!["test".into()])
    .add_file("tasks.md", "tasks", "Tasks")
        .with_description("Task list")
        .add_task("Implement feature")
            .with_labels(vec!["p0".into()])
            .add_subtask("Step 1", 'x', vec![])
            .add_subtask("Step 2", ' ', vec![])
            .end_task()
        .done()
    .generate_to(&output_dir)?;
```

## Regression Testing

The `tests/regression_tests.rs` file uses snapshot testing (insta crate) to capture expected output:

```bash
# Run regression tests
cargo test --test regression_tests

# Review and accept new snapshots
cargo insta accept

# Update snapshots after intentional changes
INSTA_FORCE_UPDATE=1 cargo test --test regression_tests
```

Snapshots are stored in `tests/snapshots/` and version-controlled to detect unintended output changes.

## Fixture Coverage Matrix

| Scenario | Fixture | Coverage |
|----------|---------|----------|
| Basic parsing | `valid/simple-task.md` | ✅ |
| Unicode filenames | `valid/日本語-タスク.md`, etc. | ✅ |
| Large lists | `valid/very-long-list-{100,500}.md` | ✅ |
| Line endings | `valid/{crlf,mixed}-line-endings.md` | ✅ |
| Flat structure | `repos/flat-project/` | ✅ |
| Deep nesting (8 levels) | `repos/deeply-nested/` | ✅ |
| Mixed structure | `repos/mixed-structure/` | ✅ |
| Realistic project | `repos/medium-project-realistic/` | ✅ |
| Circular dependencies | `invalid/circular-3-files/` | ✅ |
| Broken links | `invalid/broken-link-*.md` | ✅ |

## Adding New Fixtures

When adding new fixtures:

1. **Choose the right location:**
   - Single files: `valid/` or `invalid/`
   - Multi-file projects: `repos/`
   - Generated projects: Use `generators/` API

2. **Name descriptively:**
   - Good: `invalid/bad-checkbox.md`, `valid/with-estimates.md`
   - Avoid: `test1.md`, `foo.md`

3. **Document the fixture:**
   - Add comment at top explaining purpose
   - Include expected behavior (pass/fail)
   - Note any special characteristics

4. **Verify behavior:**
   - Valid fixtures: Must pass `lash lint`
   - Invalid fixtures: Must produce expected errors
   - Run: `cargo test` to ensure tests pass

5. **Update this README:**
   - Add to appropriate section
   - Update coverage matrix if applicable
   - Document any new patterns or edge cases

6. **Add regression tests:**
   - Add snapshot test in `tests/regression_tests.rs`
   - Run `cargo insta accept` to capture baseline

## Performance Benchmarks

Some fixtures are specifically designed for performance testing:

- `very-long-list-100.md` - Baseline for list processing (100 tasks)
- `very-long-list-500.md` - Stress test for large lists (500 tasks)
- `repos/large-project/` - Baseline for project-scale operations (100+ tasks)
- `repos/medium-project-realistic/` - Realistic workload benchmark (~250 tasks)

See `crates/lash-core/benches/` and `crates/lash-db/benches/` for benchmark usage.

## Maintenance

### Regenerating Fixtures

If you need to regenerate `medium-project-realistic`:

```bash
cargo test --test generate_realistic_project -- --ignored --nocapture
```

The generator is deterministic, so output should be identical unless the generator code changes.

### Linting All Fixtures

To verify all valid fixtures lint correctly:

```bash
# Lint all repo fixtures
for dir in crates/lash-cli/tests/fixtures/repos/*; do
    echo "Linting $dir..."
    cargo run --bin lash -- lint "$dir" || echo "FAILED: $dir"
done

# Lint all single files
cargo run --bin lash -- lint crates/lash-cli/tests/fixtures/valid/
```

### Snapshot Maintenance

Snapshots in `tests/snapshots/` should be reviewed when:
- Intentional changes are made to output format
- New features change command behavior
- Error messages are improved

Always review diffs carefully:
```bash
cargo insta review
```
