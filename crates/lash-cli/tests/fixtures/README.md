# Test Fixtures

This directory contains test fixtures for Lash integration tests.

## Directory Structure

- `valid/` - Valid Lash markdown files for testing successful parsing and operations
- `invalid/` - Invalid files for testing error handling and validation
- `repos/` - Complete multi-file test repositories

## Usage

Test fixtures can be loaded using the helper functions in `tests/helpers/mod.rs`:

```rust
use helpers::load_fixture;

let content = load_fixture("valid/simple-task.md");
```

## Adding New Fixtures

When adding new fixtures:
1. Place them in the appropriate subdirectory
2. Name them descriptively (e.g., `invalid/bad-checkbox.md`)
3. Add comments explaining what the fixture tests
4. Update this README if adding new categories
