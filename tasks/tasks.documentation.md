# Documentation Tasks

**Module:** Documentation (cross-cutting)
**Dependencies:** All implementation tasks
**Effort:** 6-8 days
**Priority:** HIGH

## Overview

Create comprehensive documentation for Lash covering user guides, developer documentation, examples, API documentation, and README. Documentation should be clear, accurate, and accessible to both humans and AI agents.

## Core Requirements

From CLAUDE.md:
- Keep README current with project purpose, setup, and usage
- Document for both human users and AI agents
- Provide examples and clear instructions

From design-doc.md:
- Document file format specification
- Document CLI commands and usage
- Document agent integration patterns

---

## Task 1: README and Quick Start

**Priority:** CRITICAL
**Effort:** 1 day
**Depends on:** Basic implementation (CLI, indexing, linting)

### Description

Create a comprehensive README that serves as the entry point for new users and provides quick start instructions.

### Subtasks

- [x] Write README.md with sections:
  - [x] **Project description**
    - [x] What is Lash?
    - [x] Key features
    - [x] Use cases
  - [x] **Installation**
    - [x] From source (`cargo install --path .`)
    - [ ] From crates.io (when published)
    - [ ] Binary releases (future)
  - [x] **Quick start**
    - [x] Create a task file
    - [x] Index the project
    - [x] List and query tasks
    - [x] Basic workflow
  - [x] **Project status**
    - [x] Current version
    - [x] Development status
    - [x] Roadmap (link to tasks)
  - [x] **Documentation links**
    - [x] User guide
    - [x] Design document
    - [x] Contributing guide
  - [x] **License and credits**
- [x] Add badges
  - [x] CI status
  - [-] Coverage (if tracked)
  - [x] License
  - [x] Rust version
- [x] Add example snippet
  - [x] Show minimal task file
  - [x] Show basic commands
  - [x] Keep it short and clear
- [-] Add animated demo (optional)
  - [-] GIF or video showing Lash in action
  - [-] Use `asciinema` or similar

### Success Criteria

- README is clear and inviting
- Quick start works for new users
- Links to detailed docs are correct
- Examples are accurate

### Tests

- Manual: Follow quick start instructions
- CI: Verify links are not broken

---

## Task 2: User Guide

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** All CLI commands implemented

### Description

Write a comprehensive user guide covering all features and workflows.

### Subtasks

- [x] Create `docs/user-guide.md` with chapters:
  - [x] **Introduction**
    - [x] What is Lash?
    - [x] When to use Lash
    - [x] Core concepts (tasks, dependencies, labels)
  - [x] **Getting Started**
    - [x] Installation
    - [x] Creating your first project
    - [x] Understanding project structure
  - [x] **Task File Format**
    - [x] File structure
    - [x] Annotations reference (`@id`, `@labels`, etc.)
    - [x] Checkbox statuses
    - [x] Dependency references
    - [x] Examples
  - [x] **CLI Commands**
    - [x] `lash lint` - validation and linting
    - [x] `lash format` - auto-formatting
    - [x] `lash index` - indexing and DB management
    - [x] `lash list` - querying tasks
    - [x] `lash show` - viewing task details
    - [x] `lash search` - fuzzy search
    - [x] `lash graph` - dependency visualization
    - [x] `lash check-links` - link validation
    - [x] `lash agent-prompt` - agent integration
    - [x] `lash tui` - terminal UI
  - [x] **Dependencies**
    - [x] How dependencies work
    - [x] Implicit (hierarchy) dependencies
    - [x] Explicit (`@depends-on`) dependencies
    - [x] Directory-level dependencies
    - [x] Completion rules
    - [x] Handling blockers
  - [x] **Labels and Filtering**
    - [x] Using labels for organization
    - [x] Filtering by labels
    - [x] Cross-cutting concerns
  - [x] **TUI Usage**
    - [x] Launching the TUI
    - [x] Navigation and keyboard shortcuts
    - [x] Viewing and editing tasks
    - [-] Agent view mode
  - [x] **Best Practices**
    - [x] Project organization
    - [x] Task granularity
    - [x] Dependency management
    - [x] Label conventions
  - [x] **Troubleshooting**
    - [x] Common errors and solutions
    - [x] DB consistency issues
    - [x] Performance tips
- [-] Add diagrams and screenshots
  - [-] TUI screenshot
  - [-] Dependency graph example
  - [-] File structure diagram
- [x] Add examples throughout
  - [x] Real-world task file examples
  - [x] Common workflows
  - [x] Complex dependency scenarios

### Success Criteria

- User guide is comprehensive and clear
- All features are documented
- Examples are accurate and helpful
- Guide is easy to navigate

### Tests

- Manual: Review for accuracy and clarity
- Manual: Test all examples
- CI: Verify code examples compile/run (if applicable)

---

## Task 3: Developer Documentation

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Core implementation complete

### Description

Write developer documentation for contributors and maintainers.

### Subtasks

- [x] Create `docs/developer-guide.md` with sections:
  - [x] **Architecture Overview**
    - [x] Crate structure (`lash-core`, `lash-db`, etc.)
    - [x] Module responsibilities
    - [x] Data flow diagram
  - [x] **Development Setup**
    - [x] Prerequisites
    - [x] Building from source
    - [x] Running tests
    - [x] Running benchmarks
  - [x] **Code Organization**
    - [x] Directory structure
    - [x] Naming conventions
    - [x] Module boundaries
  - [x] **Core Components**
    - [x] Markdown parser
    - [x] Linter
    - [x] Dependency resolver
    - [x] Indexing engine
    - [x] Query engine
    - [x] CLI framework
    - [x] TUI
  - [x] **Database Schema**
    - [x] Tables and relationships
    - [x] Indexes
    - [x] Queries
  - [x] **Error Handling**
    - [x] Error types
    - [x] Error codes
    - [x] Formatting strategies
  - [x] **Testing Strategy**
    - [x] Unit tests
    - [x] Integration tests
    - [x] E2E tests
    - [x] Benchmarks
  - [x] **Contributing**
    - [x] How to contribute
    - [x] Code style (rustfmt, clippy)
    - [x] PR process
    - [x] Review criteria
  - [x] **Release Process**
    - [x] Versioning
    - [x] Changelog
    - [x] Publishing to crates.io
    - [x] Binary releases
- [x] Create `CONTRIBUTING.md`
  - [x] Code of conduct
  - [x] How to file issues
  - [x] How to submit PRs
  - [x] Development workflow
- [-] Add architecture diagrams
  - [-] Crate dependency graph
  - [-] Data flow diagram
  - [-] CLI command flow

### Success Criteria

- Developer guide covers all major components
- New contributors can onboard from guide
- Architecture is clearly explained
- Contributing process is documented

### Tests

- Manual: Review for accuracy and completeness
- Manual: Have new contributor follow guide

---

## Task 4: API Documentation (Rustdoc)

**Priority:** MEDIUM
**Effort:** 1-2 days (ongoing with implementation)
**Depends on:** Core implementation

### Description

Write comprehensive Rustdoc comments for all public APIs.

### Subtasks

- [x] Add module-level documentation
  - [x] `lash-core`: purpose and overview
  - [x] `lash-db`: database layer overview
  - [x] `lash-cli`: CLI framework overview
  - [-] `lash-tui`: TUI overview (internal crate, not published)
  - [x] `lash-agent`: agent utilities overview
- [x] Document all public types
  - [x] Structs: purpose, fields, usage examples
  - [x] Enums: variants and when to use each
  - [x] Traits: contract and implementation notes
- [x] Document all public functions
  - [x] Purpose and behavior
  - [x] Parameters and return values
  - [x] Error conditions
  - [x] Examples
  - [-] Panics (none in public API)
- [x] Add usage examples
  - [x] Show common use cases
  - [x] Executable examples (doc tests)
  - [-] Complex scenarios (covered by integration tests)
- [x] Document invariants and assumptions
  - [x] Preconditions (documented in relevant functions)
  - [-] Postconditions (not applicable)
  - [-] Thread safety (single-threaded CLI)
  - [x] Performance characteristics (benchmarks documented)
- [x] Add links between related items
  - [x] Cross-reference types and functions (deprecated aliases link to canonical names)
  - [-] Link to design document sections (design doc linked from README)
- [x] Generate and review Rustdoc
  - [x] `cargo doc --open`
  - [x] Verify formatting and clarity
  - [x] Fix broken links
  - [x] Ensure examples run
- [x] Enforce documentation coverage with `#![warn(missing_docs)]` in all crates

### Success Criteria

- All public APIs are documented
- Examples are accurate and helpful
- Doc tests pass
- Rustdoc builds without warnings

### Tests

- Doc tests: All examples compile and run
- CI: Verify rustdoc builds without warnings

---

## Task 5: Examples and Tutorials

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** User guide complete

### Description

Create practical examples and tutorials for common use cases.

### Subtasks

- [x] Create `examples/` directory with:
  - [x] **Example 1: Simple TODO list**
    - [x] Single file with basic tasks
    - [x] Show basic CLI workflow
  - [x] **Example 2: Multi-file project**
    - [x] Multiple task files
    - [x] Directory structure
    - [x] Dependencies between files
  - [x] **Example 3: Software project**
    - [x] Feature breakdown
    - [x] Module dependencies
    - [x] Labels for cross-cutting concerns
  - [x] **Example 4: Agent-driven workflow**
    - [x] Agent-tagged tasks
    - [x] Using `lash agent-prompt`
    - [x] Agent making updates
  - [x] **Example 5: Complex dependencies**
    - [x] Nested dependencies
    - [x] Blocked tasks
    - [x] Waived tasks
- [x] Write tutorial walkthrough for each example
  - [x] Step-by-step instructions
  - [x] Expected outputs
  - [x] Explanations
- [-] Add example outputs
  - [-] CLI command outputs
  - [-] Dependency graphs
  - [-] TUI screenshots
- [-] Create video tutorials (optional)
  - [-] Screen recordings with narration
  - [-] Publish to YouTube or similar
  - [-] Link from documentation

### Success Criteria

- Examples cover diverse use cases
- Tutorials are easy to follow
- Examples are realistic and practical
- All examples work as documented

### Tests

- Integration: Run all example workflows
- Manual: Follow each tutorial step-by-step

---

## Task 6: Agent Integration Guide

**Priority:** HIGH
**Effort:** 1 day
**Depends on:** tasks.agent-integration.md complete

### Description

Create specific documentation for AI agents using Lash.

### Subtasks

- [x] Create `docs/agent-guide.md` with sections:
  - [x] **Introduction for Agents**
    - [x] What is Lash?
    - [x] Why agents should use Lash
    - [x] Agent-friendly features
  - [x] **Getting Started (for agents)**
    - [x] Obtaining Lash schema
    - [x] Understanding file format
    - [x] Allowed operations
  - [x] **File Format Schema**
    - [x] Formal specification
    - [x] Annotation reference
    - [x] Dependency syntax
    - [x] Constraints and rules
  - [x] **Safe Modifications**
    - [x] Adding tasks
    - [x] Updating task status
    - [x] Adding dependencies
    - [x] Waiving tasks
    - [x] What NOT to do
  - [x] **Workflows**
    - [x] Workflow 1: Get context (`lash agent-prompt`)
    - [x] Workflow 2: Read task files
    - [x] Workflow 3: Modify tasks
    - [x] Workflow 4: Validate (`lash lint`)
    - [x] Workflow 5: Update index (`lash index`)
  - [x] **Error Handling**
    - [x] Common errors and solutions
    - [x] Lint error recovery
    - [x] Broken dependency fixes
  - [x] **Token Minimization**
    - [x] Using sparse context
    - [x] ID-based references
    - [x] Summarization strategies
  - [x] **Examples**
    - [x] Example prompt
    - [x] Example modifications
    - [x] Example queries
- [x] Add agent-friendly formatting
  - [x] Machine-readable sections
  - [x] JSON schema snippets
  - [x] Minimal examples (token-efficient)
- [-] Create Claude Code skill spec (future)
  - [-] JSON/YAML skill definition
  - [-] Command specifications
  - [-] Usage examples

### Success Criteria

- Agent guide is clear and actionable for LLMs
- Schema specification is complete
- Workflows are safe and effective
- Examples are minimal yet comprehensive

### Tests

- Manual: Use guide with actual agent (Claude Code)
- Manual: Verify agent can use Lash safely
- Integration: Test agent workflows

---

## Task 7: Error Code Reference

**Priority:** LOW
**Effort:** 0.5 days
**Depends on:** tasks.error-handling.md complete

### Description

Create a comprehensive reference for all error codes.

### Subtasks

- [x] Create `docs/error-codes.md` with:
  - [x] Table of all error codes
  - [x] Code, severity, category
  - [x] Short description
  - [x] Detailed explanation
  - [x] Common causes
  - [x] How to fix
  - [x] Examples
- [x] Organize by category
  - [x] Parse errors (E_PARSE_*)
  - [x] Lint errors (E_LINT_*)
  - [x] Dependency errors (E_DEP_*)
  - [x] Index errors (E_INDEX_*)
  - [x] Config errors (E_CONFIG_*)
  - [x] IO errors (E_IO_*)
- [-] Add searchability
  - [-] Table of contents
  - [x] Anchor links for each code
  - [-] Search tips
- [-] Generate from code (optional)
  - [-] Extract error definitions from source
  - [-] Auto-generate reference
  - [-] Keep docs in sync with code

### Success Criteria

- All error codes are documented
- Explanations are clear and helpful
- Fixes are actionable
- Reference is easy to navigate

### Tests

- Manual: Verify all codes are listed
- CI: Verify code list matches implementation

---

## Task 8: Documentation Maintenance

**Priority:** LOW
**Effort:** Ongoing
**Depends on:** All documentation tasks

### Description

Set up processes to keep documentation up to date as code evolves.

### Subtasks

- [ ] Add documentation review to PR process
  - [ ] Require doc updates for new features
  - [x] Check for broken links
  - [ ] Verify examples still work
- [x] Set up link checker
  - [x] Use `markdown-link-check` or similar
  - [x] Run in CI
  - [x] Report broken links
- [x] Add doc tests to CI
  - [-] Run all code examples
  - [x] Verify examples compile
  - [-] Catch outdated examples
- [x] Create documentation style guide
  - [x] Tone and voice
  - [x] Formatting conventions
  - [x] Example structure
- [ ] Schedule periodic doc reviews
  - [ ] Review quarterly or on major releases
  - [ ] Update for accuracy
  - [ ] Improve clarity based on feedback
- [ ] Add user feedback mechanism
  - [ ] Link to issues for doc feedback
  - [-] Add "Was this helpful?" prompts (optional)
  - [ ] Track common questions (improve docs)

### Success Criteria

- Documentation stays current
- Broken links are caught automatically
- Examples remain accurate
- Feedback is incorporated

### Tests

- CI: Link checker passes
- CI: Doc tests pass
- Manual: Periodic doc review

---

## Non-Goals (for v1)

- Multi-language documentation (English only)
- Interactive documentation site (markdown files are sufficient)
- Video tutorial series (optional, nice-to-have)
- Auto-generated API docs from examples (manual curation is fine)

---

## Open Questions

- **Documentation hosting:** GitHub README vs dedicated docs site?
- **Tutorial format:** Text vs video vs both?
- **Agent guide format:** Markdown vs JSON schema vs both?
- **Versioning:** How to document multiple versions?

---

## References

- CLAUDE.md (Development practices)
- Design doc (Source of truth for specifications)
- Rust documentation guidelines: https://rust-lang.github.io/api-guidelines/documentation.html
- Writing great documentation: https://documentation.divio.com/
