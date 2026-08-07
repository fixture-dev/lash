# Testing Guide

This document describes how to run tests and measure coverage for the Lash project.

## Running Tests

### All Tests

Run all tests across the workspace:

```bash
cargo test --workspace
```

### Unit Tests Only

Run only unit tests (library tests):

```bash
cargo test --workspace --lib
```

### Integration Tests

Run integration tests for a specific crate:

```bash
cargo test -p lash-core --test '*'
cargo test -p lash-db --test '*'
cargo test -p lash --test '*'
```

### End-to-End CLI Tests

Run E2E tests that exercise the actual binary:

```bash
cargo test -p lash --test e2e_cli_tests
```

### Doc Tests

Run documentation examples:

```bash
cargo test --workspace --doc
```

### Benchmarks

Run performance benchmarks:

```bash
cargo bench --workspace
```

Specific benchmarks:

```bash
# Parser benchmarks
cargo bench -p lash-core --bench parser_bench

# Graph benchmarks
cargo bench -p lash-core --bench graph_bench

# Indexing benchmarks
cargo bench -p lash-db --bench indexing

# Search benchmarks
cargo bench -p lash-db --bench search_bench
```

## Test Coverage

### Install Coverage Tool

Install `cargo-llvm-cov`:

```bash
cargo install cargo-llvm-cov
```

### Generate Coverage Report

Generate HTML coverage report:

```bash
cargo llvm-cov --workspace --html
```

This creates a report in `target/llvm-cov/html/index.html`. Open it in your browser:

```bash
open target/llvm-cov/html/index.html
```

### Generate Coverage Summary

Get a text summary:

```bash
cargo llvm-cov --workspace
```

### Generate LCOV Format

For CI integration or editor plugins:

```bash
cargo llvm-cov --workspace --lcov --output-path lcov.info
```

### Coverage for Specific Crate

```bash
cargo llvm-cov -p lash-core --html
```

### Exclude Files from Coverage

Coverage is already configured to exclude:
- Test files (`tests/`, `benches/`)
- Generated code
- The binary entrypoint (`main.rs`)

## Coverage Targets

The project aims for:
- **Overall: >80% line coverage**
- **Critical modules: >90%** (parser, linter, dependency resolution)
- **Less critical: >70%** (TUI, agent utilities)

## Test Organization

### Unit Tests

Unit tests are colocated with the code they test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // ...
    }
}
```

### Integration Tests

Integration tests are in `tests/` directories:
- `crates/lash-cli/tests/` - CLI integration tests
- `crates/lash-core/tests/` - Core integration tests
- `crates/lash-db/tests/` - Database integration tests

### Test Fixtures

Test fixtures are in `crates/lash-cli/tests/fixtures/`:
- `valid/` - Valid task files for positive tests
- `invalid/` - Invalid task files for error testing
- `repos/` - Complete project fixtures

### Test Helpers

Common test utilities are in:
- `crates/lash-cli/tests/common/mod.rs` - Shared helpers

## Best Practices

### Writing Good Tests

1. **Test one thing** - Each test should verify one specific behavior
2. **Use descriptive names** - `test_parse_task_with_labels()` not `test1()`
3. **Arrange-Act-Assert** - Structure tests clearly:
   ```rust
   // Arrange - set up test data
   let input = "test data";

   // Act - execute the code under test
   let result = parse(input);

   // Assert - verify the outcome
   assert_eq!(result.unwrap(), expected);
   ```
4. **Test error cases** - Don't just test happy paths
5. **Avoid flakiness** - Tests must be deterministic
6. **Keep tests fast** - Unit tests should run in milliseconds

### What to Test

✅ **Do test:**
- Public API behavior
- Edge cases and boundary conditions
- Error handling and validation
- Integration between components

❌ **Don't test:**
- Standard library functions
- Third-party library behavior
- Implementation details
- Trivial getters/setters

### Doctest Guidelines

All public APIs should have executable doctests:

```rust
/// Parse a task file from a string
///
/// ```
/// use lash_core::parser::parse_file_from_string;
/// use lash_types::LashConfig;
///
/// let content = "# Test\n\n## Tasks\n\n- [ ] Task 1\n";
/// let config = LashConfig::default();
/// let result = parse_file_from_string(content, &config);
///
/// assert!(result.is_ok());
/// ```
pub fn parse_file_from_string(content: &str, config: &LashConfig) -> Result<TaskFile> {
    // ...
}
```

**Doctest Best Practices:**
- All doctests should be runnable by default (`cargo test --doc`)
- Use `no_run` only for examples that need I/O or external resources
- Hide boilerplate setup with `#` prefix
- Keep examples minimal and focused

## Continuous Integration

Tests run automatically on every PR via GitHub Actions. See `.github/workflows/ci.yml`.

The CI pipeline:
1. Runs all tests on Linux, macOS, and Windows
2. Checks formatting with `rustfmt`
3. Runs linter with `clippy`
4. Measures test coverage
5. Runs benchmarks (report only, doesn't fail)

## Pre-commit Hooks

Install pre-commit hooks to run tests before committing:

```bash
./scripts/install-pre-commit-hooks.sh
```

This ensures:
- Tests pass
- Code is formatted
- No lint errors

## Troubleshooting

### Tests Fail Locally But Pass in CI

- Check you're on the same Rust version: `rustc --version`
- Clear cache: `cargo clean`
- Update dependencies: `cargo update`

### Coverage Report Missing Files

- Ensure tests actually execute the code
- Check that files aren't excluded in coverage config
- Verify you're using `--workspace` flag

### Benchmarks Don't Run

- Benchmarks require a nightly compiler feature (criterion uses stable)
- Install with: `cargo bench`

## Performance Targets

Based on benchmarks, Lash should achieve:

- **Parsing:** >1000 tasks/sec
- **Linting:** >500 tasks/sec
- **Full Index:** <5s for 1000 files
- **Incremental Index:** <1s for 100 changed files
- **Query:** <100ms for typical filters
- **Search:** <200ms for typical queries

Run `cargo bench` to verify performance meets targets.
