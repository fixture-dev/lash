# Contributing to Lash

Thank you for your interest in contributing to Lash! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Review Process](#review-process)

---

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive environment for all contributors. We pledge to:

- Be respectful and considerate in all interactions
- Welcome diverse perspectives and experiences
- Accept constructive criticism gracefully
- Focus on what is best for the community and project
- Show empathy towards other community members

### Our Standards

**Examples of behavior that contributes to a positive environment:**

- Using welcoming and inclusive language
- Being respectful of differing viewpoints and experiences
- Gracefully accepting constructive criticism
- Focusing on what is best for the community
- Showing empathy towards other community members

**Examples of unacceptable behavior:**

- The use of sexualized language or imagery
- Trolling, insulting/derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information without explicit permission
- Other conduct which could reasonably be considered inappropriate

### Enforcement

Instances of abusive, harassing, or otherwise unacceptable behavior may be reported by opening an issue or contacting the project maintainers. All complaints will be reviewed and investigated promptly and fairly.

---

## Getting Started

### Prerequisites

Before you begin, ensure you have:

- **Rust** 1.75 or later (see `rust-toolchain.toml`)
- **Git** for version control
- A **GitHub account** for submitting pull requests

### Initial Setup

1. **Fork the repository** on GitHub

2. **Clone your fork**:
   ```bash
   git clone https://github.com/YOUR_USERNAME/lash.git
   cd lash
   ```

3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/fixture-dev/lash.git
   ```

4. **Install pre-commit hooks**:
   ```bash
   ./scripts/install-pre-commit-hook.sh
   ```

5. **Verify setup**:
   ```bash
   cargo build --workspace
   cargo test --workspace
   ```

### Development Tools

Install recommended tools:

```bash
# Coverage reporting
cargo install cargo-llvm-cov

# Watch mode for auto-rebuild
cargo install cargo-watch

# Benchmarking (included with workspace)
cargo bench --workspace
```

---

## How to Contribute

### Reporting Bugs

**Before submitting a bug report:**

- Check the [issue tracker](https://github.com/fixture-dev/lash/issues) for existing reports
- Verify the bug exists in the latest version
- Collect relevant information (error messages, environment details, steps to reproduce)

**When submitting a bug report, include:**

1. **Clear title**: Brief description of the issue
2. **Environment**: OS, Rust version, Lash version
3. **Steps to reproduce**: Minimal, reproducible example
4. **Expected behavior**: What you expected to happen
5. **Actual behavior**: What actually happened
6. **Error messages**: Full error output (use code blocks)
7. **Additional context**: Screenshots, logs, related issues

**Example bug report**:

```markdown
**Title**: `lash list` crashes when filtering by non-existent label

**Environment**:
- OS: macOS 14.1
- Rust: 1.75.0
- Lash: 0.2.0

**Steps to reproduce**:
1. Initialize a Lash project: `lash init`
2. Run: `lash list --label nonexistent`

**Expected behavior**:
Should return empty results or a helpful message

**Actual behavior**:
Crashes with panic:

\`\`\`
thread 'main' panicked at 'called `Option::unwrap()` on a `None` value'
\`\`\`

**Additional context**:
This only happens when no tasks have the specified label
```

### Suggesting Features

**Before suggesting a feature:**

- Check the [design document](./docs/design-doc.md) to see if it's already planned
- Search existing issues for similar suggestions
- Consider if it aligns with Lash's core principles (minimalist, fast, agent-friendly)

**When suggesting a feature, include:**

1. **Use case**: What problem does this solve?
2. **Proposed solution**: How would it work?
3. **Alternatives considered**: Other approaches you've thought about
4. **Additional context**: Examples, mockups, prior art

**Example feature request**:

```markdown
**Title**: Add support for task priorities

**Use case**:
As a user, I want to prioritize tasks so I can focus on the most important work first.

**Proposed solution**:
Add a `@priority` annotation with values: `low`, `medium`, `high`, `critical`

\`\`\`markdown
@priority: high

- [ ] Fix critical bug
\`\`\`

Add CLI filter: `lash list --priority high`

**Alternatives considered**:
- Using labels like `#high-priority` (less structured)
- Custom fields (more complex)

**Additional context**:
Similar to how GitHub issues handle priorities
```

### Asking Questions

For questions:

- Check the [README](./README.md) and [developer guide](./docs/developer-guide.md) first
- Search [GitHub Discussions](https://github.com/fixture-dev/lash/discussions)
- If unanswered, start a new discussion (not an issue)

---

## Development Workflow

### 1. Create a Branch

Always work on a feature branch:

```bash
# Update your fork
git fetch upstream
git checkout main
git merge upstream/main

# Create feature branch
git checkout -b feature/my-feature
```

**Branch naming conventions**:
- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates
- `refactor/description` - Code refactoring
- `test/description` - Test additions

### 2. Make Changes

Follow the [coding standards](#coding-standards):

```bash
# Make your changes
vim crates/lash-core/src/parser.rs

# Format code
cargo fmt --all

# Check for issues
cargo clippy --workspace -- -D warnings

# Run tests
cargo test --workspace
```

### 3. Test Your Changes

Ensure comprehensive testing:

```bash
# Unit tests
cargo test --workspace --lib

# Integration tests
cargo test --workspace --test '*'

# Doc tests
cargo test --doc

# All tests
cargo test --workspace --all-targets

# Check coverage (should maintain >80%)
cargo llvm-cov --workspace
```

### 4. Commit Changes

Use [conventional commits](#commit-guidelines):

```bash
git add .
git commit -m "feat: add contextual notes to task parser"
```

### 5. Push to Your Fork

```bash
git push origin feature/my-feature
```

### 6. Open a Pull Request

See [Pull Request Process](#pull-request-process) below.

---

## Coding Standards

### Rust Style Guide

**Formatting**: Use `rustfmt` with default configuration

```bash
cargo fmt --all
```

**Linting**: Zero clippy warnings allowed

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Code Quality Rules

1. **DRY Principle**: Don't Repeat Yourself
   - Extract common logic into functions
   - Use generics for reusable patterns
   - Avoid copy-paste code

2. **Single Responsibility**:
   - Functions should do one thing well
   - Modules should have a clear, focused purpose
   - Refactor when files exceed 500 lines

3. **Interface-Based Design**:
   - Program to traits, not concrete types
   - Keep public APIs minimal
   - Make implementations swappable

4. **Error Handling**:
   - Use `Result<T, LashError>` for fallible operations
   - Provide context with error messages
   - Never `unwrap()` or `expect()` in production code
   - Use appropriate error codes from the [error taxonomy](./docs/error-codes.md)

5. **Performance**:
   - Don't optimize prematurely
   - Benchmark performance-critical code
   - Document Big-O complexity for algorithms
   - Use `#[inline]` judiciously

### Documentation Requirements

**All public APIs must have**:

```rust
/// Brief one-line description
///
/// More detailed explanation of what this function does,
/// its behavior, and important details.
///
/// # Arguments
///
/// * `param1` - Description of parameter
/// * `param2` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// * `E_ERROR_CODE` - When this error occurs
///
/// # Examples
///
/// ```
/// use lash_core::parser::parse_file;
/// use lash_types::LashConfig;
///
/// let config = LashConfig::default();
/// let result = parse_file(path, &config)?;
/// # Ok::<(), lash_types::LashError>(())
/// ```
pub fn parse_file(path: &Path, config: &LashConfig) -> Result<TaskFile> {
    // Implementation
}
```

**Doctests**:
- All public APIs should have executable doctests
- Use `#` prefix to hide boilerplate setup
- Prefer runnable examples (avoid `ignore`)
- Use `no_run` for examples requiring I/O

### Naming Conventions

- **Functions/methods**: `snake_case`
  - Constructors: `new()`, `with_*()`, `from_*()`
  - Conversions: `to_*()` (consumes), `as_*()` (borrows), `into_*()` (consumes)
  - Fallible: `try_*()`, `*_checked()`

- **Types**: `PascalCase`
  - Structs: `TaskFile`, `DependencyGraph`
  - Enums: `TaskStatus`, `LashError`
  - Traits: `Parseable`, `Indexable`

- **Constants**: `SCREAMING_SNAKE_CASE`
  - `DEFAULT_MAX_DEPTH`, `CONFIG_FILE_NAME`

- **Modules**: `snake_case`
  - `parser`, `linter`, `graph_builder`

---

## Testing Requirements

### Test Coverage

Maintain these coverage targets:

- **Overall**: >80% line coverage
- **Critical modules** (parser, linter, dependency resolution): >90%
- **Less critical** (TUI, agent utilities): >70%

Check coverage:

```bash
cargo llvm-cov --workspace --html
open target/llvm-cov/html/index.html
```

### Test Layers

**Unit Tests**: Fast, isolated tests for single functions

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_checkbox_open() {
        let input = "- [ ] Task";
        let result = parse_checkbox(input).unwrap();
        assert_eq!(result.status, TaskStatus::Open);
    }
}
```

**Integration Tests**: Multi-component tests

```rust
// crates/lash-db/tests/indexing_tests.rs
#[test]
fn test_incremental_indexing() {
    let tmp = TempDir::new().unwrap();
    // Test setup...
    let report = indexer.index_project().unwrap();
    assert_eq!(report.files_indexed, 5);
}
```

**End-to-End Tests**: Full CLI workflow tests

```rust
use assert_cmd::Command;

#[test]
fn test_lint_command() {
    Command::cargo_bin("lash")
        .unwrap()
        .arg("lint")
        .arg("file.md")
        .assert()
        .success();
}
```

**Doc Tests**: API examples as tests

```rust
/// ```
/// use lash_core::parser::parse_file;
/// let result = parse_file(path, &config)?;
/// # Ok::<(), lash_types::LashError>(())
/// ```
```

### Test Best Practices

1. **Test one thing**: Each test should verify one specific behavior
2. **Descriptive names**: `test_parser_handles_empty_checkbox_list`
3. **Arrange-Act-Assert**: Structure tests clearly
4. **Test error cases**: Don't just test happy paths
5. **Avoid flakiness**: Tests must be deterministic
6. **Keep tests fast**: Unit tests should run in milliseconds

### Pre-commit Checks

Install the pre-commit hook:

```bash
./scripts/install-pre-commit-hook.sh
```

The hook runs before each commit:

- `cargo fmt --check` - Code formatting
- `cargo clippy --workspace -- -D warnings` - Lint checks
- `cargo test --workspace --lib` - Unit tests
- `cargo test --doc` - Doc tests

To bypass (not recommended):

```bash
git commit --no-verify
```

---

## Commit Guidelines

### Conventional Commits

Use the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types**:
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `style` - Code style changes (formatting, no logic changes)
- `refactor` - Code refactoring
- `perf` - Performance improvements
- `test` - Adding or updating tests
- `chore` - Maintenance tasks (dependencies, build config)
- `ci` - CI/CD changes

**Scope** (optional): Module or component affected
- `parser`, `linter`, `db`, `cli`, `tui`, `agent`

**Examples**:

```bash
feat(parser): add support for contextual notes

fix(db): resolve indexing crash on empty files

docs: update developer guide with testing section

refactor(cli): extract command handlers to separate module

test(linter): add test cases for depth validation

perf(indexer): optimize file hash computation
```

### Commit Message Guidelines

**Description** (first line):
- Use imperative mood ("add" not "added")
- Keep under 72 characters
- Don't end with a period
- Be specific and clear

**Body** (optional):
- Explain what and why (not how)
- Wrap at 72 characters
- Separate from description with blank line

**Footer** (optional):
- Reference issues: `Fixes #123`, `Closes #456`
- Note breaking changes: `BREAKING CHANGE: description`

**Example with body**:

```
feat(agent): add token counting for prompt generation

This adds a TokenCounter utility that estimates the number of tokens
in a given prompt using a simplified GPT-style tokenization model.
This helps agents stay within token budget constraints.

Closes #234
```

---

## Pull Request Process

### Before Submitting

**Checklist**:

- [ ] Code follows style guidelines (`cargo fmt`, `cargo clippy`)
- [ ] Tests pass (`cargo test --workspace`)
- [ ] New tests added for new functionality
- [ ] Doctests added for new public APIs
- [ ] Documentation updated (if applicable)
- [ ] Error codes documented (if new errors)
- [ ] Benchmarks run (if performance-critical)
- [ ] No clippy warnings
- [ ] Commit messages follow conventional format

### Creating the PR

1. **Push to your fork**:
   ```bash
   git push origin feature/my-feature
   ```

2. **Open PR on GitHub**: Go to https://github.com/fixture-dev/lash/pulls

3. **Fill out PR template**:

   ```markdown
   ## Description
   Brief summary of changes

   ## Motivation
   Why is this change needed?

   ## Changes
   - Added X
   - Modified Y
   - Fixed Z

   ## Testing
   - [ ] Unit tests added
   - [ ] Integration tests added
   - [ ] Manual testing performed

   ## Checklist
   - [ ] Code formatted with `cargo fmt`
   - [ ] No clippy warnings
   - [ ] Tests pass
   - [ ] Documentation updated

   ## Related Issues
   Fixes #123
   ```

### PR Title Format

Use conventional commit format:

- `feat: add contextual notes support`
- `fix: resolve parser crash on malformed headings`
- `docs: improve developer guide`

### PR Size Guidelines

**Keep PRs focused and manageable**:

- **Small PRs** (<200 lines): Easier to review, faster to merge
- **Medium PRs** (200-500 lines): Acceptable if well-organized
- **Large PRs** (>500 lines): Consider splitting into smaller PRs

**Tips for large changes**:
- Break into logical, incremental PRs
- Submit infrastructure/setup PRs first
- Add features in separate, focused PRs

---

## Review Process

### What Reviewers Look For

1. **Correctness**: Does it work? Are edge cases handled?
2. **Code Quality**: Follows style guide, no anti-patterns
3. **Tests**: Comprehensive coverage, tests the right things
4. **Documentation**: Public APIs documented, clear explanations
5. **Performance**: No obvious performance issues or regressions
6. **Maintainability**: Clear, readable code with good abstractions

### Review Timeline

- **Initial review**: Within 48 hours (typically)
- **Follow-up reviews**: Within 24 hours of updates
- **Merge**: After approval and CI passes

### Responding to Feedback

**Best practices**:

- Be receptive to feedback
- Ask questions if something is unclear
- Make requested changes in new commits (don't force-push during review)
- Respond to all comments (even if just "Done")
- Discuss disagreements respectfully

**Example responses**:

```markdown
> Consider extracting this into a helper function

Good idea! I've extracted it into `parse_annotation_value()` in commit abc123.

> This test seems to duplicate test_parse_checkbox_open

You're right. I've removed this test and enhanced the existing one instead.

> Why not use the existing `normalize_path()` function?

I tried that initially, but it doesn't handle relative paths correctly for
this use case. I can add a comment explaining the difference if that helps?
```

### After Approval

Once approved and CI passes:

1. **Squash commits** (if requested by maintainer)
2. **Update branch** with latest `main` (if needed)
3. **Wait for merge** (maintainers will merge)

### If Changes Are Requested

1. **Make the changes** locally
2. **Add tests** for the fixes
3. **Commit** with clear message
4. **Push** to update the PR
5. **Reply** to review comments

```bash
# Make changes
vim src/parser.rs

# Test
cargo test

# Commit
git add .
git commit -m "refactor: extract helper function per review feedback"

# Push
git push origin feature/my-feature
```

---

## Additional Guidelines

### Working with Task Tracking

This project uses Lash itself for task tracking (meta!):

- Check `tasks/tasks.md` for current development status
- Update task status when completing work
- Add new tasks for discovered work
- Link PRs to task IDs where applicable

### Performance Considerations

**When optimizing**:

1. **Measure first**: Use `cargo bench` to establish baseline
2. **Profile**: Identify actual bottlenecks
3. **Optimize**: Make targeted improvements
4. **Verify**: Re-benchmark to confirm improvement
5. **Document**: Add comments explaining optimizations

**Benchmark example**:

```bash
# Establish baseline
cargo bench --bench parser_bench -- --save-baseline before

# Make changes...

# Compare
cargo bench --bench parser_bench -- --baseline before
```

### Security

**If you discover a security vulnerability**:

1. **Do NOT** open a public issue
2. **Email** maintainers directly (see README for contact)
3. **Include** full details and steps to reproduce
4. **Wait** for response before public disclosure

**Security-focused changes**:
- Add tests demonstrating the vulnerability is fixed
- Include CVE numbers if applicable
- Document in `CHANGELOG.md` under "Security"

### License

By contributing, you agree that your contributions will be licensed under the same terms as the project (Apache License, Version 2.0).

---

## Getting Help

**If you need help**:

1. **Documentation**: Check [README](./README.md), [developer guide](./docs/developer-guide.md), [design doc](./docs/design-doc.md)
2. **Discussions**: Browse or start a [GitHub Discussion](https://github.com/fixture-dev/lash/discussions)
3. **Issues**: Search existing issues for similar problems
4. **Ask**: If stuck, open a discussion or comment on a related issue

**Be specific when asking**:
- What you're trying to accomplish
- What you've tried so far
- Error messages or unexpected behavior
- Environment details (OS, Rust version)

---

## Recognition

Contributors will be recognized in:

- `CHANGELOG.md` for each release
- GitHub contributors page
- Mentioned in release notes (for significant contributions)

Thank you for contributing to Lash!
