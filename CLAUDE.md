# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Lash is a minimalist, ultra-fast, Markdown-native task tracker for devs and agents. This is an early-stage project currently in the design phase.

**Key Principles:**
- Markdown is the single source of truth
- SQLite is the acceleration layer (fully reconstructible from Markdown)
- Strict, linter-enforced format for predictability
- Terminal-first UX (CLI + TUI)
- Agent-friendly design with token minimization

## Project Status

Currently in design phase. The repository contains:
- Design document (`docs/design-doc.md`) - comprehensive specification
- No implementation code yet

## Project development
The project should be developed according to the development practices defined in this section.

### Project Management
- When prompted to begin or continue development, always begin by checking the current `git status` and the `tasks/` directory to gain context that is relevant to the development prompt
- Use the `tasks/` directory to track all project tasks:
  - `tasks/tasks.md` serves as the index file listing all task categories and their files
  - Each major module/area has its own dedicated task file (e.g., `tasks/tasks.markdown-linter.md`, `tasks/tasks.fuzz-search.md`)
  - Break down each requirement into tasks and subtasks within the appropriate task file
- Use markdown checkboxes to track the progress of each task and requirement and check them as they are completed
- Do not check off a requirement unless you have either written an automated test to prove that it works.
- When work is completed, perform git operations that result in a commit and a clean `git status`
- Capture development progress and notes in `devlog.md`, ideally referencing relevant git commits whenever possible
- Request clarification when needed if a requirement is ambiguous or unclear

### Context Management
- **Before starting new tasks:** Clean up lingering background processes to prevent context bloat
- **Git commits:** Run in foreground (not background) to avoid accumulating long commit messages in context
- **Commit messages:** Keep concise (2-3 lines); save detailed documentation for `devlog.md`
- **Background processes:** Use only for truly long-running operations (>30s):
  - CI/CD monitoring (`gh run watch`)
  - Long test suites that take minutes to complete
  - Avoid for: quick git operations, test runs <30s, gh commands
- **Cleanup routine:** Use `/clear` when starting new major tasks or after completing significant milestones to prevent stale context buildup

### General Development Practices
<critical-dev-practice>Never commit with --no-verify</critical-dev-practice>
<critical-dev-practice>Never use simulated behavior or mocking in production code.</critical-dev-practice>

- Add a pre-commit hook which ensures that linting and tests pass.
- Use the DRY principle (Don't Repeat Yourself) to avoid duplication of code whenever possible.
- Avoid having files over 500 lines of code. Refactor at that point.
- Use the design pattern of programming to an interface so that implementations can be swapped out easily.
- Use a clear project structure with separate directories for source code, tests, docs, and config.
- Write appropriate layers of tests (unit, integration, end-to-end) to prove functionality works and to guard against regressions.
- Do not write frivolous tests to meet coverage thresholds.
- Do not add special test-related cases to production code to make tests pass.
- Keep the `README.md` file current with the project's purpose, setup instructions, and usage instructions.

#### Doctest Best Practices
All public APIs should have executable doctests that serve as both documentation and tests. Follow these guidelines to keep doctests painless and maintainable:

**Default to Executable:**
- ALL doctests should be runnable by default (`cargo test --doc`)
- Avoid `rust,ignore` unless absolutely necessary (e.g., requires network, external resources)
- Use `no_run` for examples that compile but require file I/O or other external resources

**Minimal, Clear Examples:**
- Use crate-level imports: `use lash_core::parser::parse_file;`
- Keep examples focused on demonstrating one thing
- Factor out repetitive setup into hidden lines using `#` prefix
- Show the simplest possible usage that compiles and runs

**Hidden Lines for Boilerplate:**
```rust
/// Example function
///
/// ```
/// # use lash_core::TaskFile;
/// # use std::path::PathBuf;
/// # let file = TaskFile {
/// #     path: PathBuf::from("test.md"),
/// #     // ... other required fields hidden from docs
/// # };
/// // The actual example the user sees
/// println!("Task count: {}", file.tasks.len());
/// ```
```

**When to Use Each Attribute:**
- No attribute: Fully executable example (preferred)
- `no_run`: Compiles but doesn't execute (for I/O, network, etc.)
- `compile_fail`: Example that should fail to compile (for demonstrating errors)
- `ignore`: Only as last resort when example truly can't be made testable

**Verification:**
- Run `cargo test --doc` regularly to ensure all doctests pass
- The pre-commit hook should fail if any doctests are failing
- Aim for 0 ignored doctests across the codebase

## Architecture (Planned)

The project will be implemented in Rust with these main components:

### Core Crates (Planned)
- `lash-cli` - CLI parsing and integration layer
- `lash-core` - Markdown parser, task model, linter, dependency resolution
- `lash-db` - SQLite schema, indexing, and query layer
- `lash-tui` - Terminal UI implementation
- `lash-agent` - Prompt generation and token minimization utilities

### Data Model
- **Task files**: Markdown files with hierarchical checkbox lists
- **Root index**: `lash.index.md` or `index.lash.md` at project root
- **Dependencies**: Within-file (parent/child checkboxes) and cross-file (explicit links)
- **Labels**: Cross-cutting tags (e.g., `#backend`, `#agent`)

### Task Format (Planned)
```markdown
# Topic Title

@id: unique.identifier
@labels: tag1, tag2
@owner: name
@created: YYYY-MM-DD

## Tasks

- [ ] Parent task
  - [ ] Child task (max depth: 3-4 levels)
  - [ ] Another child #label
- [-] Waived task (not applicable)
- [x] Completed task
```

## CLI Commands (Planned)

**Linting & Formatting:**
- `lash lint [PATH...]` - Validate format and semantics
- `lash format [PATH...]` - Normalize formatting

**Indexing:**
- `lash index` - Rebuild SQLite DB from Markdown
- `lash check-index` - Verify DB consistency

**Querying:**
- `lash list [FILTERS]` - List tasks by label, status, path, etc.
- `lash search <QUERY>` - Fuzzy search
- `lash show <TASK_ID>` - Display specific task

**Dependency Management:**
- `lash graph` - Output dependency graph
- `lash check-links` - Scan for broken references

**Agent Integration:**
- `lash agent-prompt [OPTIONS]` - Generate context-minimized prompts for LLMs

**TUI:**
- `lash tui` - Launch terminal UI

## Design Considerations

**Dependency Model:**
- Parent tasks implicitly depend on children
- Cross-file dependencies via `@depends-on: path/to/file.md#task:id`
- Directory-level dependencies for hierarchical structure
- Tasks complete only when all dependencies are done or waived

**Status Values:**
- `[ ]` - open
- `[-]` - waived/not applicable
- `[x]` - done
- `[!]` - blocked (optional)

**Annotations:**
- `@id` - unique within file
- `@labels` - comma-separated tags
- `@owner` - assignee
- `@estimate` - time estimate
- `@depends-on` - explicit dependencies
- `@agent-note` - hints for LLM agents

**Token Minimization for Agents:**
- Schema-first prompts with minimal examples
- Sparse context (only relevant files + dependency summaries)
- ID-based references instead of full descriptions
- Optional summarization layer for large files

## Development Workflow (When Implemented)

**Adding Features:**
1. Update design doc if needed
2. Implement in appropriate crate
3. Add tests
4. Run linter on test fixtures

**Working with Task Files:**
- Always validate with `lash lint` after manual edits
- Respect max depth limits (3-4 levels)
- Use consistent annotation format
- Ensure dependencies are resolvable

## References

See `docs/design-doc.md` for comprehensive specification including:
- Complete file format specification
- SQLite schema design
- Linter requirements
- TUI design
- Agent integration details
- Error handling patterns
