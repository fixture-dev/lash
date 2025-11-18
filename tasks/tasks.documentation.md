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

- [ ] Write README.md with sections:
  - [ ] **Project description**
    - [ ] What is Lash?
    - [ ] Key features
    - [ ] Use cases
  - [ ] **Installation**
    - [ ] From source (`cargo install --path .`)
    - [ ] From crates.io (when published)
    - [ ] Binary releases (future)
  - [ ] **Quick start**
    - [ ] Create a task file
    - [ ] Index the project
    - [ ] List and query tasks
    - [ ] Basic workflow
  - [ ] **Project status**
    - [ ] Current version
    - [ ] Development status
    - [ ] Roadmap (link to tasks)
  - [ ] **Documentation links**
    - [ ] User guide
    - [ ] Design document
    - [ ] Contributing guide
  - [ ] **License and credits**
- [ ] Add badges
  - [ ] CI status
  - [ ] Coverage (if tracked)
  - [ ] License
  - [ ] Rust version
- [ ] Add example snippet
  - [ ] Show minimal task file
  - [ ] Show basic commands
  - [ ] Keep it short and clear
- [ ] Add animated demo (optional)
  - [ ] GIF or video showing Lash in action
  - [ ] Use `asciinema` or similar

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

- [ ] Create `docs/user-guide.md` with chapters:
  - [ ] **Introduction**
    - [ ] What is Lash?
    - [ ] When to use Lash
    - [ ] Core concepts (tasks, dependencies, labels)
  - [ ] **Getting Started**
    - [ ] Installation
    - [ ] Creating your first project
    - [ ] Understanding project structure
  - [ ] **Task File Format**
    - [ ] File structure
    - [ ] Annotations reference (`@id`, `@labels`, etc.)
    - [ ] Checkbox statuses
    - [ ] Dependency references
    - [ ] Examples
  - [ ] **CLI Commands**
    - [ ] `lash lint` - validation and linting
    - [ ] `lash format` - auto-formatting
    - [ ] `lash index` - indexing and DB management
    - [ ] `lash list` - querying tasks
    - [ ] `lash show` - viewing task details
    - [ ] `lash search` - fuzzy search
    - [ ] `lash graph` - dependency visualization
    - [ ] `lash check-links` - link validation
    - [ ] `lash agent-prompt` - agent integration
    - [ ] `lash tui` - terminal UI
  - [ ] **Dependencies**
    - [ ] How dependencies work
    - [ ] Implicit (hierarchy) dependencies
    - [ ] Explicit (`@depends-on`) dependencies
    - [ ] Directory-level dependencies
    - [ ] Completion rules
    - [ ] Handling blockers
  - [ ] **Labels and Filtering**
    - [ ] Using labels for organization
    - [ ] Filtering by labels
    - [ ] Cross-cutting concerns
  - [ ] **TUI Usage**
    - [ ] Launching the TUI
    - [ ] Navigation and keyboard shortcuts
    - [ ] Viewing and editing tasks
    - [ ] Agent view mode
  - [ ] **Best Practices**
    - [ ] Project organization
    - [ ] Task granularity
    - [ ] Dependency management
    - [ ] Label conventions
  - [ ] **Troubleshooting**
    - [ ] Common errors and solutions
    - [ ] DB consistency issues
    - [ ] Performance tips
- [ ] Add diagrams and screenshots
  - [ ] TUI screenshot
  - [ ] Dependency graph example
  - [ ] File structure diagram
- [ ] Add examples throughout
  - [ ] Real-world task file examples
  - [ ] Common workflows
  - [ ] Complex dependency scenarios

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

- [ ] Create `docs/developer-guide.md` with sections:
  - [ ] **Architecture Overview**
    - [ ] Crate structure (`lash-core`, `lash-db`, etc.)
    - [ ] Module responsibilities
    - [ ] Data flow diagram
  - [ ] **Development Setup**
    - [ ] Prerequisites
    - [ ] Building from source
    - [ ] Running tests
    - [ ] Running benchmarks
  - [ ] **Code Organization**
    - [ ] Directory structure
    - [ ] Naming conventions
    - [ ] Module boundaries
  - [ ] **Core Components**
    - [ ] Markdown parser
    - [ ] Linter
    - [ ] Dependency resolver
    - [ ] Indexing engine
    - [ ] Query engine
    - [ ] CLI framework
    - [ ] TUI
  - [ ] **Database Schema**
    - [ ] Tables and relationships
    - [ ] Indexes
    - [ ] Queries
  - [ ] **Error Handling**
    - [ ] Error types
    - [ ] Error codes
    - [ ] Formatting strategies
  - [ ] **Testing Strategy**
    - [ ] Unit tests
    - [ ] Integration tests
    - [ ] E2E tests
    - [ ] Benchmarks
  - [ ] **Contributing**
    - [ ] How to contribute
    - [ ] Code style (rustfmt, clippy)
    - [ ] PR process
    - [ ] Review criteria
  - [ ] **Release Process**
    - [ ] Versioning
    - [ ] Changelog
    - [ ] Publishing to crates.io
    - [ ] Binary releases
- [ ] Create `CONTRIBUTING.md`
  - [ ] Code of conduct
  - [ ] How to file issues
  - [ ] How to submit PRs
  - [ ] Development workflow
- [ ] Add architecture diagrams
  - [ ] Crate dependency graph
  - [ ] Data flow diagram
  - [ ] CLI command flow

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

- [ ] Add module-level documentation
  - [ ] `lash-core`: purpose and overview
  - [ ] `lash-db`: database layer overview
  - [ ] `lash-cli`: CLI framework overview
  - [ ] `lash-tui`: TUI overview
  - [ ] `lash-agent`: agent utilities overview
- [ ] Document all public types
  - [ ] Structs: purpose, fields, usage examples
  - [ ] Enums: variants and when to use each
  - [ ] Traits: contract and implementation notes
- [ ] Document all public functions
  - [ ] Purpose and behavior
  - [ ] Parameters and return values
  - [ ] Error conditions
  - [ ] Examples
  - [ ] Panics (if any)
- [ ] Add usage examples
  - [ ] Show common use cases
  - [ ] Executable examples (doc tests)
  - [ ] Complex scenarios
- [ ] Document invariants and assumptions
  - [ ] Preconditions
  - [ ] Postconditions
  - [ ] Thread safety
  - [ ] Performance characteristics
- [ ] Add links between related items
  - [ ] Cross-reference types and functions
  - [ ] Link to design document sections
- [ ] Generate and review Rustdoc
  - [ ] `cargo doc --open`
  - [ ] Verify formatting and clarity
  - [ ] Fix broken links
  - [ ] Ensure examples run

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

- [ ] Create `examples/` directory with:
  - [ ] **Example 1: Simple TODO list**
    - [ ] Single file with basic tasks
    - [ ] Show basic CLI workflow
  - [ ] **Example 2: Multi-file project**
    - [ ] Multiple task files
    - [ ] Directory structure
    - [ ] Dependencies between files
  - [ ] **Example 3: Software project**
    - [ ] Feature breakdown
    - [ ] Module dependencies
    - [ ] Labels for cross-cutting concerns
  - [ ] **Example 4: Agent-driven workflow**
    - [ ] Agent-tagged tasks
    - [ ] Using `lash agent-prompt`
    - [ ] Agent making updates
  - [ ] **Example 5: Complex dependencies**
    - [ ] Nested dependencies
    - [ ] Blocked tasks
    - [ ] Waived tasks
- [ ] Write tutorial walkthrough for each example
  - [ ] Step-by-step instructions
  - [ ] Expected outputs
  - [ ] Explanations
- [ ] Add example outputs
  - [ ] CLI command outputs
  - [ ] Dependency graphs
  - [ ] TUI screenshots
- [ ] Create video tutorials (optional)
  - [ ] Screen recordings with narration
  - [ ] Publish to YouTube or similar
  - [ ] Link from documentation

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

- [ ] Create `docs/agent-guide.md` with sections:
  - [ ] **Introduction for Agents**
    - [ ] What is Lash?
    - [ ] Why agents should use Lash
    - [ ] Agent-friendly features
  - [ ] **Getting Started (for agents)**
    - [ ] Obtaining Lash schema
    - [ ] Understanding file format
    - [ ] Allowed operations
  - [ ] **File Format Schema**
    - [ ] Formal specification
    - [ ] Annotation reference
    - [ ] Dependency syntax
    - [ ] Constraints and rules
  - [ ] **Safe Modifications**
    - [ ] Adding tasks
    - [ ] Updating task status
    - [ ] Adding dependencies
    - [ ] Waiving tasks
    - [ ] What NOT to do
  - [ ] **Workflows**
    - [ ] Workflow 1: Get context (`lash agent-prompt`)
    - [ ] Workflow 2: Read task files
    - [ ] Workflow 3: Modify tasks
    - [ ] Workflow 4: Validate (`lash lint`)
    - [ ] Workflow 5: Update index (`lash index`)
  - [ ] **Error Handling**
    - [ ] Common errors and solutions
    - [ ] Lint error recovery
    - [ ] Broken dependency fixes
  - [ ] **Token Minimization**
    - [ ] Using sparse context
    - [ ] ID-based references
    - [ ] Summarization strategies
  - [ ] **Examples**
    - [ ] Example prompt
    - [ ] Example modifications
    - [ ] Example queries
- [ ] Add agent-friendly formatting
  - [ ] Machine-readable sections
  - [ ] JSON schema snippets
  - [ ] Minimal examples (token-efficient)
- [ ] Create Claude Code skill spec (future)
  - [ ] JSON/YAML skill definition
  - [ ] Command specifications
  - [ ] Usage examples

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

- [ ] Create `docs/error-codes.md` with:
  - [ ] Table of all error codes
  - [ ] Code, severity, category
  - [ ] Short description
  - [ ] Detailed explanation
  - [ ] Common causes
  - [ ] How to fix
  - [ ] Examples
- [ ] Organize by category
  - [ ] Parse errors (E_PARSE_*)
  - [ ] Lint errors (E_LINT_*)
  - [ ] Dependency errors (E_DEP_*)
  - [ ] Index errors (E_INDEX_*)
  - [ ] Config errors (E_CONFIG_*)
  - [ ] IO errors (E_IO_*)
- [ ] Add searchability
  - [ ] Table of contents
  - [ ] Anchor links for each code
  - [ ] Search tips
- [ ] Generate from code (optional)
  - [ ] Extract error definitions from source
  - [ ] Auto-generate reference
  - [ ] Keep docs in sync with code

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
  - [ ] Check for broken links
  - [ ] Verify examples still work
- [ ] Set up link checker
  - [ ] Use `markdown-link-check` or similar
  - [ ] Run in CI
  - [ ] Report broken links
- [ ] Add doc tests to CI
  - [ ] Run all code examples
  - [ ] Verify examples compile
  - [ ] Catch outdated examples
- [ ] Create documentation style guide
  - [ ] Tone and voice
  - [ ] Formatting conventions
  - [ ] Example structure
- [ ] Schedule periodic doc reviews
  - [ ] Review quarterly or on major releases
  - [ ] Update for accuracy
  - [ ] Improve clarity based on feedback
- [ ] Add user feedback mechanism
  - [ ] Link to issues for doc feedback
  - [ ] Add "Was this helpful?" prompts (optional)
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
