# Project Setup Tasks

**Module:** Foundation & Infrastructure
**Priority:** CRITICAL
**Estimated Duration:** 3-5 days
**Dependencies:** None (starting point)

## Overview

These tasks establish the project structure, development tooling, and foundational infrastructure that everything else depends on.

## Tasks

### 1. Initialize Rust Workspace

- [ ] **Create workspace `Cargo.toml` with all planned crates**
  - [ ] Define workspace members: `lash-types`, `lash-core`, `lash-db`, `lash-agent`, `lash-tui`, `lash-cli`
  - [ ] Add workspace-level dependencies (shared versions)
  - [ ] Configure workspace-level settings (edition = "2021", rust-version)
  - [ ] Set up workspace-level features
- [ ] **Create individual crate directories**
  - [ ] `crates/lash-types/` - Shared types
  - [ ] `crates/lash-core/` - Core parsing and validation
  - [ ] `crates/lash-db/` - Database layer
  - [ ] `crates/lash-agent/` - Agent integration
  - [ ] `crates/lash-tui/` - Terminal UI
  - [ ] `crates/lash-cli/` - CLI binary
- [ ] **Add initial dependencies**
  - [ ] `serde` with `derive` feature (workspace)
  - [ ] `thiserror` for error handling (workspace)
  - [ ] `anyhow` for CLI error propagation (lash-cli only)
  - [ ] `clap` v4 with `derive` feature (lash-cli)
  - [ ] Add dependency versions to workspace
- [ ] **Configure git repository**
  - [ ] Update `.gitignore` for Rust (target/, Cargo.lock for libs)
  - [ ] Add `rust-toolchain.toml` for version consistency (stable)
  - [ ] Initial commit with project structure
- [ ] **Verify build**
  - [ ] Run `cargo build --workspace` successfully
  - [ ] Run `cargo test --workspace` successfully

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** None
**Success Criteria:** `cargo build` succeeds for all crates

---

### 2. Set Up Development Tooling

- [ ] **Configure code formatting**
  - [ ] Create `rustfmt.toml` at workspace root
  - [ ] Set max_width = 100
  - [ ] Set edition = "2021"
  - [ ] Enable format_strings, imports_granularity = "Crate"
  - [ ] Verify `cargo fmt --all -- --check` works
- [ ] **Configure linting**
  - [ ] Create `clippy.toml` at workspace root
  - [ ] Enable strict lints (warn on all clippy::pedantic)
  - [ ] Allow specific lints as needed (document why)
  - [ ] Verify `cargo clippy --workspace -- -D warnings` works
- [ ] **Set up dependency auditing**
  - [ ] Add `deny.toml` configuration
  - [ ] Configure for security advisories
  - [ ] Configure license compliance checks
  - [ ] Set up bans for duplicate dependencies
- [ ] **Create pre-commit hook**
  - [ ] Create `.git/hooks/pre-commit` script
  - [ ] Run `cargo fmt --all -- --check`
  - [ ] Run `cargo clippy --workspace -- -D warnings`
  - [ ] Run `cargo test --workspace`
  - [ ] Make hook executable
  - [ ] Test hook blocks commits with issues
- [ ] **Add editor configuration**
  - [ ] Create `.editorconfig` for cross-editor consistency
  - [ ] Set indent_size = 4 for Rust
  - [ ] Set trim_trailing_whitespace = true
  - [ ] Set insert_final_newline = true

**Priority:** HIGH
**Estimate:** 0.5 day
**Dependencies:** Task #1
**Success Criteria:** Pre-commit hook blocks commits with formatting/lint issues

---

### 3. Establish Testing Infrastructure

- [ ] **Create test fixture structure**
  - [ ] Create `tests/fixtures/` directory
  - [ ] Create `tests/fixtures/valid/` for valid examples
  - [ ] Create `tests/fixtures/invalid/` for error cases
  - [ ] Create `tests/fixtures/repos/` for complete test repositories
  - [ ] Add README explaining fixture organization
- [ ] **Create initial test fixtures**
  - [ ] `valid/simple-task.md` - minimal valid task file
  - [ ] `valid/with-dependencies.md` - file with dependencies
  - [ ] `valid/with-labels.md` - file with labels
  - [ ] `valid/nested-hierarchy.md` - deep task hierarchy
  - [ ] `invalid/bad-checkbox.md` - malformed checkbox syntax
  - [ ] `invalid/depth-exceeded.md` - exceeds max depth
  - [ ] `invalid/unknown-annotation.md` - unknown @key
  - [ ] `invalid/broken-dependency.md` - references non-existent task
- [ ] **Set up test helpers**
  - [ ] Create `tests/helpers/mod.rs`
  - [ ] Add `fn load_fixture(path)` helper
  - [ ] Add `fn temp_test_dir()` helper
  - [ ] Add `fn assert_error_contains(result, text)` helper
  - [ ] Add `fn init_test_db()` helper
- [ ] **Configure test running**
  - [ ] Add `cargo-nextest` to project (optional but recommended)
  - [ ] Configure test timeouts
  - [ ] Set up test output formatting
- [ ] **Set up code coverage**
  - [ ] Add `cargo-llvm-cov` to CI toolchain
  - [ ] Create coverage script
  - [ ] Set coverage threshold (aim for 80%+)
  - [ ] Configure coverage excludes (test code, generated code)

**Priority:** HIGH
**Estimate:** 1 day
**Dependencies:** Task #1
**Success Criteria:** Can run tests with coverage reporting; fixtures loadable

---

### 4. Define Error Taxonomy

- [ ] **Create base error types in `lash-types`**
  - [ ] Define `LashError` enum with variants:
    - [ ] `ParseError` - Markdown parsing failures
    - [ ] `LintError` - Validation failures
    - [ ] `IoError` - File system errors
    - [ ] `DatabaseError` - SQLite errors
    - [ ] `ConfigError` - Configuration issues
    - [ ] `DependencyError` - Broken references, cycles
  - [ ] Derive `Debug`, `Clone` for error types
  - [ ] Implement `Display` for human-readable messages
  - [ ] Implement `std::error::Error` trait
- [ ] **Define error codes**
  - [ ] Create `error_codes.rs` with constants
  - [ ] `E_PARSE_*` - Parsing errors (E_PARSE_BAD_CHECKBOX, etc.)
  - [ ] `E_LINT_*` - Linting errors (E_LINT_DEPTH_EXCEEDED, etc.)
  - [ ] `E_DEP_*` - Dependency errors (E_DEP_NOT_FOUND, E_DEP_CYCLE, etc.)
  - [ ] `E_IO_*` - I/O errors (E_IO_FILE_NOT_FOUND, etc.)
  - [ ] `E_DB_*` - Database errors (E_DB_CONSTRAINT, etc.)
  - [ ] Document each code with description and example
- [ ] **Create diagnostic structure**
  - [ ] Define `Diagnostic` struct with fields:
    - [ ] `code: &'static str` - Error code
    - [ ] `severity: Severity` - Error/Warning/Info
    - [ ] `message: String` - Human-readable message
    - [ ] `location: Option<Location>` - File, line, column
    - [ ] `suggestion: Option<String>` - How to fix
  - [ ] Define `Location` struct (file_path, line, column, span)
  - [ ] Define `Severity` enum (Error, Warning, Info, Help)
- [ ] **Add JSON serialization**
  - [ ] Derive `Serialize` on all error types (via serde)
  - [ ] Create `to_json()` method on `Diagnostic`
  - [ ] Test JSON output format
  - [ ] Ensure stable field names for machine parsing
- [ ] **Create `Result<T>` type alias**
  - [ ] `pub type Result<T> = std::result::Result<T, LashError>;`
  - [ ] Export from `lash-types` for use across crates
- [ ] **Document error codes**
  - [ ] Create `docs/error-codes.md`
  - [ ] Document each error code with:
    - [ ] Code and name
    - [ ] Description
    - [ ] Example that triggers it
    - [ ] How to fix
  - [ ] Organize by category (parse, lint, dependency, etc.)

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** Task #1
**Success Criteria:** All error codes documented; JSON serialization works

---

### 5. Create Project Configuration Model

- [ ] **Define `LashConfig` struct in `lash-types`**
  - [ ] `root_path: PathBuf` - Project root directory
  - [ ] `index_file: String` - Root index filename (default: "lash.index.md")
  - [ ] `max_depth: u8` - Maximum task nesting (default: 3)
  - [ ] `indent_spaces: u8` - Indentation size (default: 2)
  - [ ] `db_path: Option<PathBuf>` - Database location (default: .lash/lash.db)
  - [ ] Derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] **Implement root detection**
  - [ ] Create `find_project_root(start_dir: &Path) -> Result<PathBuf>`
  - [ ] Search upward from start_dir for `lash.index.md` or `.lash/`
  - [ ] Stop at filesystem root or home directory
  - [ ] Return `ConfigError::RootNotFound` if not found
  - [ ] Add tests for various directory structures
- [ ] **Implement config file loading**
  - [ ] Create `.lash/config.toml` support
  - [ ] Parse TOML using `toml` crate
  - [ ] Merge with default config
  - [ ] Validate config values (e.g., max_depth in range 2-5)
  - [ ] Return errors for invalid values
- [ ] **Add CLI override support**
  - [ ] Accept `--root` argument to override detection
  - [ ] Validate that provided root contains index file
  - [ ] Create `Config::from_cli_args(args)` method
- [ ] **Create config builder**
  - [ ] Implement builder pattern for `LashConfig`
  - [ ] `ConfigBuilder::default()` -> uses defaults
  - [ ] `.root(path)` - override root
  - [ ] `.max_depth(n)` - override depth
  - [ ] `.build() -> Result<LashConfig>` - validate and build
- [ ] **Add validation**
  - [ ] Validate root path exists and is readable
  - [ ] Validate index file exists (or can be created)
  - [ ] Validate max_depth in reasonable range (2-5)
  - [ ] Validate indent_spaces is 2 or 4
  - [ ] Return detailed validation errors

**Priority:** HIGH
**Estimate:** 1 day
**Dependencies:** Task #4 (error types)
**Success Criteria:** Can detect project root; config loads and validates correctly

---

## Summary

### Total Estimate
**3-5 days** total for all foundation tasks

### Completion Criteria
- [ ] All tasks above completed
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] Pre-commit hook installed and working
- [ ] Test fixtures created and loadable
- [ ] Error taxonomy documented
- [ ] Config system working with root detection

### Next Steps
After completing project setup, proceed to:
1. **tasks.core-data-model.md** - Define core data structures
2. **tasks.markdown-parser.md** - Implement parsing (depends on data model)
