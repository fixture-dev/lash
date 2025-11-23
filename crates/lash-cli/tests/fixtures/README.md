# Test Fixtures

This directory contains test fixtures for Lash integration tests.

## Directory Structure

- `valid/` - Valid Lash markdown files for testing successful parsing and operations
  - Basic task files with various annotation combinations
  - Edge cases: empty lists, unicode content, large lists, maximum nesting
  - Task files demonstrating estimates, blockers, agent notes, and waived tasks
- `invalid/` - Invalid files for testing error handling and validation
  - Missing or malformed annotations
  - Invalid status values and checkbox formats
  - Circular dependencies and duplicate IDs
  - Depth exceeded errors
- `repos/` - Complete multi-file test repositories
  - `small-project/` - 3 files, 10-15 tasks (minimal test project)
  - `medium-project/` - 10 files, 30-50 tasks (fullstack app demo)
  - `large-project/` - 20+ files, 100+ tasks (enterprise project demo)

## Usage

Test fixtures can be loaded using the helper functions in `tests/common/mod.rs`:

```rust
use common::{load_fixture, TestProject};

// Load a single fixture file
let content = load_fixture("valid/simple-task.md");

// Create a temporary project from a fixture
let project = TestProject::from_fixture("small");

// Build a custom test project
let project = TestProject::builder()
    .with_index("test-project", "Test Project")
    .with_task_file("tasks.md", "tasks", "Tasks")
    .build();
```

## Fixture Projects

### Small Project
- **Purpose:** Quick integration tests
- **Structure:** 3 files, flat hierarchy
- **Tasks:** ~10-15 tasks
- **Use cases:** Basic CLI command testing, simple indexing

### Medium Project
- **Purpose:** Realistic fullstack application
- **Structure:** 10 files in backend/, frontend/, docs/ directories
- **Tasks:** 30-50 tasks with dependencies
- **Use cases:** Dependency resolution, label filtering, cross-file queries

### Large Project
- **Purpose:** Enterprise-scale testing
- **Structure:** 20+ files across multiple teams/modules
- **Tasks:** 100+ tasks with complex dependencies
- **Use cases:** Performance testing, graph visualization, search functionality

## Adding New Fixtures

When adding new fixtures:
1. Place them in the appropriate subdirectory
2. Name them descriptively (e.g., `invalid/bad-checkbox.md`, `valid/with-estimates.md`)
3. Add comments at the top explaining what the fixture tests
4. Update this README if adding new categories or project types
5. Ensure valid fixtures pass `lash lint`
6. Ensure invalid fixtures produce appropriate errors
