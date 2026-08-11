# Lash User Guide

> Minimalist, Markdown-native task tracker for developers and AI agents

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Getting Started](#2-getting-started)
3. [Task File Format](#3-task-file-format)
4. [CLI Commands](#4-cli-commands)
5. [Dependencies](#5-dependencies)
6. [Labels and Filtering](#6-labels-and-filtering)
7. [TUI Usage](#7-tui-usage)
8. [Best Practices](#8-best-practices)
9. [Troubleshooting](#9-troubleshooting)

---

## 1. Introduction

### What is Lash?

Lash is a terminal-first task management system that uses **Markdown as the single source of truth**. Unlike traditional task trackers, Lash stores all your tasks in simple, structured Markdown files that you can edit with any text editor, version control with Git, and process with standard Unix tools.

### When to Use Lash

Lash is ideal for:

- **Developers** who prefer text-based workflows and want tasks in version control
- **AI agents** (like Claude Code) that need predictable, linter-enforced formats
- **Terminal enthusiasts** who want fast, keyboard-driven task management
- **Teams** practicing "docs as code" and Infrastructure as Code workflows
- **Projects** where tasks need clear dependency graphs and cross-file references

### Core Concepts

#### Tasks

Tasks are checkbox items in Markdown files. Each task has:
- A **status**: open `[ ]`, done `[x]`, waived `[-]`, or blocked `[!]`
- Optional **metadata**: ID, labels, owner, estimate, dependencies
- Optional **contextual notes**: Plain bullet points providing context

#### Dependencies

Tasks can depend on other tasks in two ways:
- **Implicit**: Parent tasks depend on their nested children
- **Explicit**: Cross-file dependencies via `@depends-on` annotations

#### Labels

Labels are lightweight tags (e.g., `#backend`, `#urgent`) that create cross-cutting slices across your task hierarchy. Filter and search by labels to find related work.

#### The Index File

Every Lash project has a root index file (`lash.index.md`) that provides the high-level project structure and links to topic files.

#### SQLite Acceleration Layer

Lash maintains a SQLite database (`.lash/lash.db`) for fast querying and filtering. This database is **fully reconstructible** from your Markdown files and should not be committed to version control.

---

## 2. Getting Started

### Installation

Build from source (requires Rust stable toolchain):

```bash
git clone https://github.com/fixture-dev/lash.git
cd lash
cargo build --release
# Binary at: target/release/lash
```

Add to your PATH or install directly:

```bash
cargo install --path crates/lash-cli
```

### Creating Your First Project

Initialize a new Lash project in your current directory:

```bash
lash init
```

This creates:
- `lash.index.md` - Root index file
- `.lash/` - Configuration and database directory
- `.lash/lash.db` - SQLite database (auto-generated)

The generated index file looks like this:

```markdown
# my-project

@id: index
@created: 2025-12-14

## Overview

This is the root index file for the Lash task tracker.
Edit this file to define your project's task structure.

## Tasks

- [ ] Set up project structure
- [ ] Define task files and categories
- [ ] Add initial tasks
```

### Understanding Project Structure

A typical Lash project structure:

```
my-project/
├── lash.index.md           # Root index
├── .lash/
│   ├── config.toml         # Project configuration
│   └── lash.db            # SQLite database (don't commit)
├── features/
│   ├── authentication.md   # Feature tasks
│   └── user-profile.md
├── backend/
│   ├── api.md
│   └── database.md
└── docs/
    └── architecture.md
```

**Best practices:**
- Use descriptive directory names (`features/`, `backend/`, `docs/`)
- Keep related tasks in the same file
- Reference task files from your index
- Add `.lash/lash.db` to `.gitignore`

### Try the Playground

Want to explore Lash features without setting up your own project? Use the playground:

```bash
lash playground init
cd playground
lash list --label gameplay
lash tui
```

The playground creates "PixelQuest" - a realistic game development demo with 24 task files, hundreds of tasks, and examples of all Lash features. See `playground/PLAYGROUND_GUIDE.md` for details.

---

## 3. Task File Format

### File Structure

Each task file has four sections:

```markdown
# Title (H1 heading)

@id: unique.identifier
@labels: label1, label2
@owner: alice
@created: 2025-12-14

## Description (optional)

Free-form Markdown describing the file's purpose, context,
constraints, and implementation notes.

## Tasks

- [ ] Parent task
  - [ ] Child task
  - [ ] Another child

## References (optional)

Links to related files, documentation, or external resources.
```

### Annotations Reference

Annotations appear after the title and before `## Description`:

#### @id (required)

Unique identifier for the file, used in cross-file references.

```markdown
@id: features.authentication
```

**Rules:**
- Must be unique across the project
- Use alphanumeric characters, hyphens, underscores, dots, colons
- Convention: use dot-delimited paths matching file structure

#### @labels

Comma-separated tags for categorization and filtering.

```markdown
@labels: backend, security, api
```

**Common label conventions:**
- Domain: `backend`, `frontend`, `infra`, `docs`
- Priority: `p0`, `p1`, `p2`
- Type: `bug`, `feature`, `refactor`
- Team: `team-platform`, `team-api`

#### @owner

Person or team responsible.

```markdown
@owner: alice
```

#### @created

Creation date (YYYY-MM-DD format).

```markdown
@created: 2025-12-14
```

#### @depends-on

Explicit cross-file dependencies.

```markdown
@depends-on: backend/database.md#task:migrations
@depends-on: features/authentication.md
```

**Format:**
- `path/to/file.md` - Depend on entire file
- `path/to/file.md#task:id` - Depend on specific task
- `#task:id` - Same-file reference

#### @estimate

Time estimate for completion.

```markdown
@estimate: 2h
@estimate: 3d
@estimate: 1w
```

**Valid units:** `m` (minutes), `h` (hours), `d` (days), `w` (weeks)

#### @agent-note

Special notes for AI agents working with the codebase.

```markdown
@agent-note: Use existing auth middleware pattern at src/middleware/auth.rs
```

### Checkbox Statuses

Four checkbox states represent task status:

| Syntax | Status | Meaning |
|--------|--------|---------|
| `[ ]` | open | Task not yet started |
| `[x]` | done | Task completed |
| `[-]` | waived | Task no longer applicable |
| `[!]` | blocked | Task waiting on dependencies |

**Example:**

```markdown
## Tasks

- [x] Set up database schema
- [ ] Implement authentication
  - [x] Research OAuth libraries
  - [ ] Integrate Passport.js
  - [-] Custom token implementation (using OAuth instead)
- [!] Deploy to production (blocked by security audit)
```

### Contextual Notes

**Contextual notes** are plain bullet points (without checkboxes) that provide inline context, requirements, or acceptance criteria:

```markdown
- [ ] Implement payment processing
  - Use Stripe API v3 for all transactions
  - Support credit cards and ACH payments
  - Must handle refunds and partial captures
  - [ ] Set up Stripe account
  - [ ] Implement checkout flow
  - [ ] Add webhook handling
```

**Key points:**
- **Plain bullets** (`- Text`) provide context, NOT tracked for completion
- **Checkbox bullets** (`- [ ]`) are actionable tasks tracked for completion
- Notes should appear before child tasks (convention)
- Notes cannot have children (enforced by linter)
- Notes are searchable via `lash search`

**When to use notes vs. child tasks:**

Use **notes** for:
- Requirements and constraints
- Acceptance criteria
- Implementation hints
- API/library specifics
- Design context

Use **child tasks** for:
- Multi-step processes needing completion tracking
- Independently trackable work items
- Work that can be done in parallel

See `examples/contextual-notes.md` for comprehensive examples.

### Description Section

The optional `## Description` section provides detailed context:

```markdown
## Description

This module handles all authentication flows including login, logout,
password reset, and session management. It integrates with OAuth providers
and implements JWT-based token authentication.

Key constraints:
- Sessions expire after 24 hours
- Support Google and GitHub OAuth
- Must pass external security audit requirements
```

**Guidelines:**
- Place after annotations, before `## Tasks`
- Keep under 1000 characters (warning threshold)
- Use for explaining scope, constraints, and design decisions
- Descriptions are searchable and displayed in task views

### Task Nesting and Depth

Tasks can be nested up to 3-4 levels deep:

```markdown
- [ ] Level 1: Major feature
  - [ ] Level 2: Component
    - [ ] Level 3: Specific task
      - [ ] Level 4: Subtask (max depth)
```

**Best practices:**
- Keep hierarchies shallow (prefer 2-3 levels)
- If you need more depth, split into multiple files
- Use `@depends-on` for cross-file relationships

### Task References

Reference tasks by their full ID:

```markdown
- [ ] Deploy API server @id: deploy-api
  @depends-on: backend/api.md#task:api-tests
  @depends-on: infrastructure/k8s.md
```

**ID format:**
- Within file: Just the task ID
- Cross-file: `path/to/file.md#task:id`

### Complete Example

```markdown
# User Authentication System

@id: features.authentication
@labels: backend, security, p0
@owner: alice
@created: 2025-12-10
@depends-on: backend/database.md#task:user-schema

## Description

Implements secure user authentication with OAuth 2.0 support for Google
and GitHub providers. Uses JWT tokens for session management with 24-hour
expiration. Must pass external security audit scheduled for January 2026.

## Tasks

- [x] Research authentication libraries
  - Evaluated Passport.js, Auth0, and custom implementation
  - Selected Passport.js for OAuth integration

- [ ] Implement OAuth flows @id: oauth-flows
  - Support Google and GitHub as initial providers
  - Must handle email verification before account creation
  - Store OAuth tokens securely in encrypted database field
  - [ ] Set up OAuth applications
  - [ ] Implement Google OAuth flow
  - [ ] Implement GitHub OAuth flow
  - [ ] Add email verification step

- [ ] JWT token management @id: jwt-tokens
  - Tokens expire after 24 hours
  - Use RS256 algorithm (not HS256)
  - Store refresh tokens with 30-day expiration
  - [ ] Generate RSA key pair
  - [ ] Implement token generation
  - [ ] Implement token validation
  - [ ] Add refresh token logic

- [-] Custom password implementation
  - Decided to use OAuth-only for initial release

## References

- OAuth 2.0 Spec: https://oauth.net/2/
- Security audit requirements: docs/security-requirements.md
```

---

## 4. CLI Commands

### Initialization

#### `lash init`

Initialize a new Lash project.

```bash
# Initialize in current directory
lash init

# Initialize in specific directory
lash init --path ~/projects/my-app

# Force re-initialization
lash init --force

# Skip initial indexing
lash init --no-index
```

**Options:**
- `--path DIR` - Target directory (default: current)
- `--force` - Overwrite existing project
- `--no-index` - Skip running indexer
- `--json` - JSON output format

#### `lash playground init`

Create the PixelQuest demo project for exploration.

```bash
# Initialize playground in ./playground
lash playground init

# Reset existing playground
lash playground init --reset
```

### Linting and Formatting

#### `lash lint`

Validate task file format and semantics.

```bash
# Lint all files in project
lash lint

# Lint specific files
lash lint features/auth.md backend/api.md

# Auto-fix issues
lash lint --fix

# Interactive fix mode
lash lint --interactive

# JSON output for scripts
lash lint --json

# Show diff without applying
lash lint --fix --diff
```

**Options:**
- `PATH...` - Files or directories to lint (default: all)
- `--fix` - Auto-fix issues when possible
- `--interactive` - Prompt before each fix
- `--diff` - Show changes without applying
- `--json` - Machine-readable output
- `--no-color` - Disable colored output

**What it checks:**
- Checkbox syntax validity
- Annotation formats and values
- Task nesting depth limits
- Duplicate IDs within files
- Contextual note placement
- Indentation consistency

#### `lash format`

Normalize file formatting (alias: `lash fmt`).

```bash
# Format all files
lash format

# Format specific files
lash format features/*.md

# Check without modifying
lash format --check

# Show diff
lash format --diff
```

**Options:**
- `PATH...` - Files to format (default: all)
- `--check` - Check formatting without modifying
- `--diff` - Show formatting changes
- `--json` - JSON output

**What it does:**
- Normalizes indentation (2 spaces)
- Orders annotations consistently
- Fixes whitespace issues
- Aligns checkbox markers

### Indexing and Database

#### `lash index`

Build or rebuild the SQLite database from Markdown files.

```bash
# Incremental index (only changed files)
lash index

# Full rebuild
lash index --force

# Show indexed files
lash index --show-files

# Use specific project root
lash index --root ~/projects/my-app
```

**Options:**
- `--force` - Full rebuild instead of incremental
- `--show-files` - List indexed files
- `--json` - JSON output
- `--root PATH` - Project root directory

**When to run:**
- After creating or editing task files
- After pulling changes from version control
- If database seems out of sync
- Automatically runs after `lash add` commands

#### `lash check-index`

Verify database consistency with Markdown files.

```bash
# Check for drift
lash check-index

# Show detailed diff
lash check-index --diff
```

**Options:**
- `--diff` - Show differences between DB and files
- `--json` - JSON output

**Exit codes:**
- `0` - Database is consistent
- `1` - Inconsistencies found

Alongside the usual checks (stale records, missing files, hash mismatches),
`check-index` compares the stored task IDs against what Lash derives today.
A file whose content has not changed still drifts if the derivation rules
changed under it, and hash comparison alone cannot see that.

#### `lash migrate-ids`

Rewrite `@depends-on` references left dangling by a task-ID derivation change.

```bash
# Show what changed and which references it affects
lash migrate-ids

# Rewrite them, then re-index
lash migrate-ids --write

# Discard the pending renames without rewriting anything
lash migrate-ids --forget
```

**Options:**
- `--write` - Apply the rewrites (without it, nothing is written)
- `--forget` - Discard the pending renames, for repairs done by hand
- `--json` - JSON output

**Exit codes:**
- `0` - Nothing pending, or the rewrite succeeded
- `1` - Renames are pending and nothing has been written yet

**Background:** a task with no explicit `@id:` gets its ID derived from its
title, so a release that changes the derivation rules moves every such ID.
`lash index` notices, re-derives the stored IDs, and records what each one used
to be — the only moment both spellings exist. `lash migrate-ids` consumes that
record.

Only whole references on `@depends-on:` lines are rewritten. Prose that happens
to mention an old ID is left alone, and so is the unqualified `old-id` form,
since a bare token can name a file as readily as a task. Run `lash lint` after
migrating to catch anything left.

To keep an ID stable across future changes, pin it with `@id:`.

### Querying Tasks

#### `lash list`

List tasks with optional filters.

```bash
# List all tasks
lash list

# Filter by label
lash list --label backend
lash list --label p0 --label urgent

# Filter by status
lash list --status open
lash list --status blocked

# Filter by owner
lash list --owner alice

# Show only blocked tasks
lash list --blocked

# Tree view with hierarchy
lash list --tree

# Include descriptions
lash list --show-descriptions

# Include contextual notes
lash list --show-notes

# Combine filters
lash list --label backend --status open --owner alice
```

**Options:**
- `--label TAG` - Filter by label (repeatable)
- `--status STATUS` - Filter by status (open, done, waived, blocked)
- `--owner NAME` - Filter by owner
- `--blocked` - Show only blocked tasks
- `--tree` - Display as tree hierarchy
- `--show-descriptions` - Include file descriptions
- `--show-notes` - Include contextual notes
- `--json` - JSON output
- `--no-color` - Disable colors

#### `lash search`

Full-text search across tasks, notes, and descriptions.

```bash
# Search for text
lash search "authentication"

# Limit results
lash search "OAuth" --limit 10

# JSON output
lash search "payment" --json
```

**Options:**
- `QUERY` - Search query (required)
- `--limit N` - Max results (default: 20)
- `--json` - JSON output

**What it searches:**
- Task titles
- Task bodies
- Contextual notes
- File descriptions
- File paths
- Labels

#### `lash show`

Display detailed information about a specific task or file.

For a task, the default output includes ID/title/status/file, owner and
estimate (if set), labels, doc references, description and contextual
notes, the full `@agent-note` (multi-line, line breaks preserved), each
`@depends-on` reference resolved to its current status (e.g. `[done] ✓` /
`[open] ✗` / `[unresolved]`, with a "N/M satisfied" summary), and a
one-line-per-child summary of direct children with their checkbox state
(plus a "N/M done" summary). Empty fields are omitted rather than printed
blank.

```bash
# Show task by ID
lash show features/auth.md#task:oauth-flows

# Show entire file
lash show features/auth.md

# Terse output: only ID/Title/Status/File/Labels
lash show features/auth.md#task:oauth-flows --short

# Include dependencies
lash show features/auth.md --deps

# Include reverse dependencies (what depends on this)
lash show features/auth.md --rdeps

# Show both deps and rdeps
lash show features/auth.md --deps --rdeps
```

**Options:**
- `TASK_ID` - Task or file reference (required)
- `--deps` - Show dependencies
- `--rdeps` - Show reverse dependencies
- `--short` - Terse output: only ID/Title/Status/File/Labels
- `--json` - JSON output (includes `agent_note`, `depends_on` with resolved
  statuses, and `children` alongside the task record)

### Task Creation

#### `lash add`

Create a new task in a file.

```bash
# Add task to current file
lash add "Implement user login"

# Specify target file
lash add "Add database migration" --file backend/database.md

# Create new file with task
lash add "Setup CI pipeline" --file infrastructure/ci.md \
  --file-title "CI/CD Pipeline" \
  --file-description "Automated build and deployment configuration"

# Add as child of existing task
lash add "Write unit tests" --parent oauth-flows

# Add with metadata
lash add "Fix authentication bug" \
  --label backend --label urgent \
  --owner alice \
  --estimate 2h \
  --status open

# Add with explicit ID
lash add "Deploy to staging" --id deploy-staging

# Add with dependencies (each target must already resolve, or the task is
# not created)
lash add "Integration tests" \
  --depends-on backend/api.md#task:api-endpoints \
  --depends-on backend/database.md#task:migrations

# Add a dependency on a task that doesn't exist yet (create-in-any-order);
# writes the reference as a warning instead of a hard error
lash add "Integration tests" --depends-on not-yet-created --allow-forward-ref

# Add with agent note
lash add "Refactor auth middleware" \
  --agent-note "Existing implementation at src/middleware/auth.rs"

# Validate without creating
lash add "Test task" --dry-run

# Interactive mode (prompts for fields)
lash add --interactive
```

**Options:**
- `TITLE` - Task title (required unless --interactive)
- `--file PATH` - Target file (creates if doesn't exist)
- `--file-title TEXT` - Title for new file
- `--file-description TEXT` - Description for new file
- `--parent ID` - Parent task ID
- `--after ID` - Insert after this task
- `--before ID` - Insert before this task
- `--label TAG` - Add label (repeatable)
- `--owner NAME` - Set owner
- `--estimate DURATION` - Set estimate (e.g., 30m, 2h, 1d)
- `--status STATUS` - Initial status (default: open)
- `--id ID` - Explicit task ID; written as `@id:` so `file#ID` resolves immediately
- `--depends-on REF` - Add dependency (repeatable); each must resolve or the task is not created
- `--allow-forward-ref` - Downgrade an unresolved `--depends-on` target to a warning and write anyway
- `--agent-note TEXT` - Add agent note
- `--dry-run` - Validate without creating
- `--interactive` - Interactive mode
- `--format FORMAT` - Output format (text, json)
- `--no-color` - Disable colors

**Note:** After adding a task, Lash automatically re-indexes the database.

### Task Completion

#### `lash complete`

Mark one or more tasks as complete.

```bash
# Complete a single task
lash complete features#implement-login

# Complete multiple tasks at once
lash complete features#task-1 features#task-2 features#task-3

# Preview what would be changed (dry run)
lash complete --dry-run features#implement-login

# JSON output for scripting
lash complete --json features#implement-login
```

**What it does:**
- Updates the checkbox in the source Markdown file from `[ ]` or `[!]` to `[x]`
- Automatically re-indexes the database
- Supports fuzzy matching with suggestions if task ID not found

**Exit codes:**
- `0` - All tasks completed successfully
- `1` - Validation error (task already complete, waived, etc.)
- `5` - Task not found

**Options:**
- `TASK_ID...` - One or more task IDs to complete (required)
- `--dry-run` - Preview changes without modifying files
- `--json` - JSON output for scripting

**Example output:**
```
[x] features#implement-login -> features/auth.md
```

**JSON output:**
```json
{
  "success": true,
  "completed": [
    {
      "task_id": "features#implement-login",
      "file_path": "features/auth.md",
      "previous_status": "open"
    }
  ],
  "errors": []
}
```

**Note:** After completing tasks, Lash automatically re-indexes the database.

#### `lash waive`

Mark one or more tasks as waived (not applicable). Mirrors `lash complete`,
but doesn't require dependencies to be resolved — waiving abandons the
task rather than finishing it.

```bash
# Waive a single task
lash waive features#legacy-oauth-flow

# Record why it's being waived (written as a contextual note)
lash waive --reason "Superseded by the new OAuth2 flow" features#legacy-oauth-flow

# Also waive unchecked plain-bullet children
lash waive --cascade features#legacy-oauth-flow

# Preview what would be changed (dry run)
lash waive --dry-run features#legacy-oauth-flow

# JSON output for scripting
lash waive --json features#legacy-oauth-flow
```

**What it does:**
- Updates the checkbox in the source Markdown file to `[-]`
- Automatically re-indexes the database
- With `--reason`, appends the text as a contextual note (a plain bullet
  indented 2 spaces under the task) after any existing `@id`/`@depends-on`
  annotations, so it round-trips through the parser and passes `lash lint`
- Supports fuzzy matching with suggestions if task ID not found

**Status transitions:**
- `open`, `in-progress`, `blocked` → `waived`: allowed
- Already `waived`: refused with `E_ALREADY_WAIVED`
- `done` → `waived`: refused with `E_DONE` (completed work shouldn't be
  silently waived; hand-edit the checkbox to `[-]` if this is truly
  intended)
- No `@depends-on` gating — abandoning a task doesn't require its
  dependencies to be resolved first

**Exit codes:**
- `0` - All tasks waived successfully
- `1` - Validation error (already waived, task is done, etc.)
- `5` - Task not found

**Options:**
- `TASK_ID...` - One or more task IDs to waive (required)
- `--dry-run` - Preview changes without modifying files
- `--cascade` - Also waive unchecked plain-bullet children (without their
  own `@id`)
- `--reason TEXT` - One-line rationale recorded as a contextual note
- `--json` - JSON output for scripting

**Example output:**
```
[-] features#legacy-oauth-flow -> features/auth.md
  reason: Superseded by the new OAuth2 flow
```

**JSON output:**
```json
{
  "success": true,
  "waived": [
    {
      "task_id": "features#legacy-oauth-flow",
      "file_path": "features/auth.md",
      "previous_status": "open",
      "reason": "Superseded by the new OAuth2 flow"
    }
  ],
  "errors": []
}
```

**Note:** After waiving tasks, Lash automatically re-indexes the database.

### Task Editing

#### `lash update`

Edit fields on a single existing task without hand-editing Markdown.

```bash
# Rewrite a task's title
lash update features#legacy-oauth-flow --title "New OAuth2 flow"

# Labels
lash update features#legacy-oauth-flow --add-label urgent --remove-label backend

# Owner and estimate ("" removes the annotation)
lash update features#legacy-oauth-flow --owner alice --estimate 2h
lash update features#legacy-oauth-flow --owner ""

# Agent note: replace, or append a continuation line
lash update features#legacy-oauth-flow --agent-note "Full replacement text"
lash update features#legacy-oauth-flow --append-agent-note "One more detail"

# Dependencies, validated against the project like `lash add --depends-on`
lash update features#legacy-oauth-flow --add-depends-on backend/api.md#task:api-endpoints
lash update features#legacy-oauth-flow --remove-depends-on backend/api.md#task:api-endpoints
lash update features#legacy-oauth-flow --add-depends-on not-yet-created --allow-forward-ref

# Preview without writing
lash update features#legacy-oauth-flow --title "New title" --dry-run
```

**ID stability on retitle (the important part):** a task's derived id comes
from the first 40 characters of its kebab-cased title unless it has an
explicit `@id:`. Retitling a task with no explicit `@id` would normally
change that derived id — silently orphaning every `@depends-on` reference
elsewhere in the project that pointed at the old slug. `lash update --title`
prevents this: if the task has no explicit `@id:` yet, it first writes
`@id: <old-derived-slug>` under the task (pinning the id the title used to
imply), *then* changes the title, and prints an informational line:
`pinned @id: <slug> to preserve references`. Tasks that already carry an
explicit `@id:` are unaffected — only the title changes.

**What it does:**
- Edits the task's Markdown in place with targeted line changes (not a full
  file reformat) and automatically re-indexes the database
- `--add-label`/`--remove-label` edit whichever form the task already
  uses — inline `#tag` on the title line, or an `@labels:` annotation — and
  default to the inline form (matching `lash add --label`) for a task with
  no labels yet
- `--owner`/`--estimate` set, replace, or (given `""`) remove the
  annotation
- `--agent-note` replaces the note (including any existing multi-line
  continuation); `--append-agent-note` adds a new continuation line,
  creating the note if the task doesn't have one yet
- `--add-depends-on` is validated against the current project the same way
  `lash add --depends-on` is — an unresolvable reference is a hard error and
  the file is left untouched, unless `--allow-forward-ref` downgrades it to
  a warning. `--remove-depends-on` matches by exact reference string and
  errors if the task doesn't have it
- Supports fuzzy matching with suggestions if the task ID isn't found

**Exit codes:**
- `0` - Update applied successfully
- `1` - Validation error (no mutation flags, unresolved `--add-depends-on`
  target, `--remove-label`/`--remove-depends-on` target not present, etc.)
- `3` - Database or file I/O error
- `5` - Task not found

**Options:**
- `TASK_ID` - The task to update (required)
- `--title TEXT` - Rewrite the title
- `--add-label LABEL` / `--remove-label LABEL` - Repeatable
- `--owner NAME` / `--estimate DURATION` - Pass `""` to remove
- `--agent-note TEXT` - Replace (or add) the agent note
- `--append-agent-note TEXT` - Append a continuation line
- `--add-depends-on REF` / `--remove-depends-on REF` - Repeatable
- `--allow-forward-ref` - Downgrade an unresolved `--add-depends-on` target
  to a warning
- `--dry-run` - Preview changes without modifying files
- `--json` - JSON output for scripting

**Note:** After updating a task, Lash automatically re-indexes the database.

### Dependencies

#### `lash graph`

Visualize the dependency graph.

```bash
# ASCII art graph to terminal
lash graph

# Export to DOT format
lash graph --format dot > graph.dot

# Generate PNG with Graphviz
lash graph --format dot | dot -Tpng > graph.png

# JSON format
lash graph --format json

# Mermaid diagram
lash graph --format mermaid

# Scope to specific file
lash graph --scope features/auth.md

# Hide completed tasks
lash graph --hide-completed
```

**Options:**
- `--format FORMAT` - Output format: ascii, dot, json, mermaid (default: ascii)
- `--scope PATH` - Limit to specific file or directory
- `--hide-completed` - Exclude done/waived tasks
- `--json` - JSON output for metadata

**Formats:**
- `ascii` - Terminal-friendly text visualization
- `dot` - Graphviz DOT format (use with `dot`, `neato`, etc.)
- `json` - Structured graph data
- `mermaid` - Mermaid.js diagram syntax

#### `lash check-links`

Validate cross-file dependency references.

```bash
# Check all links
lash check-links

# Attempt to fix broken links
lash check-links --fix

# Show what would be fixed
lash check-links --fix --dry-run
```

**Options:**
- `--fix` - Attempt auto-fix of broken links
- `--dry-run` - Show fixes without applying
- `--json` - JSON output

**What it checks:**
- File paths exist
- Referenced task IDs exist
- No circular dependencies

### Agent Integration

#### `lash agent-prompt`

Generate context-minimized prompts for AI agents.

```bash
# Plain text format (dynamic, project-specific)
lash agent-prompt

# JSON format
lash agent-prompt --format json

# Agents.md format
lash agent-prompt --format agents-md

# Filter by label
lash agent-prompt --label backend --label p0

# Token budget limit
lash agent-prompt --max-tokens 4000

# Include task descriptions
lash agent-prompt --include-descriptions

# Include contextual notes
lash agent-prompt --include-notes

# Install a static skill into a coding agent's conventional directory
lash skill install --target claude          # Claude Code (SKILL.md + references/)
lash skill install --target codex           # Codex / AGENTS.md hosts
lash skill install --target cursor          # Cursor IDE rules
lash skill install --target agents-md       # generic AGENTS.lash.md sibling
```

**Options:**
- `--format FORMAT` - Output format: plain, json, agents-md (default: plain)
- `--label TAG` - Filter by label (repeatable)
- `--max-tokens N` - Token budget limit
- `--include-descriptions` - Include file descriptions
- `--include-notes` - Include contextual notes
- `--json` - Metadata as JSON
- `--no-color` - Disable colors

**Use cases:**
- Generate instructions for Claude Code or other AI coding assistants
- Create context for LLM-based automation
- Export task subset for agent workflows
- Use `lash skill install` for one-time setup; use `lash agent-prompt` for
  live, per-request context

### Configuration

#### `lash config`

Manage project and user configuration.

```bash
# List all settings
lash config list

# Show only changed settings
lash config list --changed

# Get specific value
lash config get max_depth

# Set value
lash config set max_depth 4

# Get with default
lash config get color_scheme
```

**Options:**
- `get KEY` - Get configuration value
- `set KEY VALUE` - Set configuration value
- `list` - Show all configuration
- `--changed` - Show only non-default values

**Common settings:**
- `max_depth` - Maximum task nesting (default: 3)
- `indent_spaces` - Indentation width (default: 2)
- `color_scheme` - TUI color scheme (default: Base2Tone Desert)

**Configuration files:**
- Project: `.lash/config.toml`
- User: `~/.lash/config.toml`

#### `lash completion`

Generate shell completion scripts.

```bash
# Bash
lash completion bash > ~/.bash_completion.d/lash

# Zsh
lash completion zsh > ~/.zsh/completions/_lash

# Fish
lash completion fish > ~/.config/fish/completions/lash.fish

# PowerShell
lash completion powershell > lash.ps1

# Elvish
lash completion elvish > ~/.elvish/lib/lash.elv
```

**Shells:** bash, zsh, fish, powershell, elvish

#### `lash explain`

Explain error codes.

```bash
# List all error codes
lash explain --list

# Explain specific code
lash explain E_LINT_DEPTH_EXCEEDED

# JSON format
lash explain E_CREATE_DUPLICATE_ID --json
```

See `docs/error-codes.md` for complete error catalog.

### User Interface

#### `lash tui`

Launch the terminal user interface.

```bash
# Launch TUI
lash tui

# Use specific color scheme
lash tui --color-scheme Nord

# Use project at specific path
lash tui --root ~/projects/my-app
```

**Options:**
- `--color-scheme NAME` - Color scheme (e.g., Nord, Dracula, Solarized Dark)
- `--root PATH` - Project root directory

See section 7 for TUI usage details.

---

## 5. Dependencies

### How Dependencies Work

Lash supports two types of dependencies:

1. **Implicit (Hierarchical)**: Parent tasks automatically depend on their children
2. **Explicit**: Cross-file dependencies declared with `@depends-on`

### Implicit Dependencies

Parent tasks depend on all nested child tasks:

```markdown
- [ ] Implement authentication system
  - [ ] Create user model
  - [ ] Add login endpoint
  - [ ] Add registration endpoint
```

**Completion rules:**
- Parent completes only when ALL children are done or waived
- Marking parent as done when children are open triggers linter error
- Children can be waived `[-]` to unblock parent

### Explicit Dependencies

Use `@depends-on` to create cross-file dependencies:

**File-level dependency:**

```markdown
# Backend API

@id: backend.api
@depends-on: backend/database.md
@depends-on: infrastructure/docker.md
```

**Task-level dependency:**

```markdown
- [ ] Deploy API server @id: deploy-api
  @depends-on: backend/api.md#task:integration-tests
  @depends-on: infrastructure/k8s.md#task:cluster-setup
```

**Syntax:**
- `path/to/file.md` - Depend on entire file completing
- `path/to/file.md#task:id` - Depend on specific task
- `#task:id` - Reference task in same file

### Directory-Level Dependencies

The project structure implicitly creates dependencies:

```
project/
├── lash.index.md          # Root
├── core/
│   ├── foundation.md      # Core foundations
│   └── utilities.md
└── features/
    └── user-profile.md    # Depends on core/ (implicit)
```

Configure directory dependencies in the index file.

### Completion Rules

A task is **complete** when:
1. Its status is `[x]` (done), AND
2. All child tasks are done or waived `[-]`, AND
3. All explicit dependencies are complete or waived

A task is **blocked** when:
1. Any dependency is incomplete and not waived

**Example:**

```markdown
- [ ] Deploy to production
  @depends-on: #task:run-tests
  @depends-on: #task:security-audit
  - [ ] Update deployment config
  - [ ] Run smoke tests

- [x] Run tests @id: run-tests

- [ ] Security audit @id: security-audit (still pending)
```

In this case, "Deploy to production" is **blocked** because:
- Child task "Update deployment config" is incomplete
- Dependency "Security audit" is incomplete

### Handling Blockers

**Option 1: Complete the dependency**

Mark the blocking task as done:

```markdown
- [x] Security audit @id: security-audit
```

**Option 2: Waive the dependency**

If a task is no longer needed:

```markdown
- [-] Security audit @id: security-audit (waived - already audited in Q3)
```

**Option 3: Remove the dependency**

Delete the `@depends-on` annotation if it was incorrect.

### Checking Dependencies

```bash
# View dependency graph
lash graph

# Show task with its dependencies
lash show features/auth.md#task:deploy --deps

# Show what depends on this task
lash show features/auth.md#task:oauth --rdeps

# Find broken links
lash check-links
```

### Circular Dependencies

Lash detects and reports circular dependencies:

```markdown
# File A
@depends-on: file-b.md

# File B
@depends-on: file-a.md  # ERROR: Circular dependency!
```

Run `lash lint` or `lash check-links` to detect cycles.

---

## 6. Labels and Filtering

### Using Labels for Organization

Labels create cross-cutting slices across your task hierarchy:

```markdown
# Backend API

@labels: backend, api, p0

## Tasks

- [ ] Implement user endpoints @labels: backend, api
- [ ] Add caching layer @labels: backend, performance, p1
- [ ] Write API documentation @labels: docs, api
```

### Label Conventions

**By Domain:**
- `backend` - Server-side code
- `frontend` - Client-side code
- `infra` - Infrastructure and DevOps
- `docs` - Documentation
- `testing` - Test code and QA

**By Priority:**
- `p0` - Critical (ship blockers)
- `p1` - High priority
- `p2` - Medium priority
- `p3` - Low priority / nice-to-have

**By Type:**
- `bug` - Bug fixes
- `feature` - New features
- `refactor` - Code improvements
- `tech-debt` - Technical debt

**By Team:**
- `team-platform` - Platform team
- `team-api` - API team
- `team-mobile` - Mobile team

### Filtering by Labels

```bash
# Single label
lash list --label backend

# Multiple labels (AND logic)
lash list --label backend --label p0

# Combine with other filters
lash list --label backend --status open --owner alice

# In TUI (press 'l')
lash tui
# Then filter interactively
```

### Search by Labels

```bash
# Find all tasks with label
lash search "#backend"

# Combine with text search
lash search "authentication #backend"
```

### Labels in Agent Prompts

Generate prompts for specific work areas:

```bash
# Backend high-priority work
lash agent-prompt --label backend --label p0

# Documentation tasks
lash agent-prompt --label docs
```

---

## 7. TUI Usage

### Launching the TUI

```bash
# Launch with default settings
lash tui

# Use specific color scheme
lash tui --color-scheme "Solarized Dark"

# Use different project
lash tui --root ~/projects/my-app
```

### TUI Layout

```
┌─────────────────────┬──────────────────────────────────┐
│ Navigation Tree     │ Task Details                     │
│                     │                                  │
│ ▼ features/         │ Title: Implement OAuth flows     │
│   ▸ auth.md         │ Status: in-progress              │
│   ▸ profile.md      │ Owner: alice                     │
│ ▼ backend/          │ Labels: backend, security        │
│   ▸ api.md          │                                  │
│   ▸ database.md     │ Description:                     │
│                     │ OAuth integration for Google...  │
│                     │                                  │
│                     │ Tasks:                           │
│                     │ ☐ Set up OAuth applications      │
│                     │ ☐ Implement Google flow          │
└─────────────────────┴──────────────────────────────────┘
 Status: 45 tasks (30 open, 12 done, 3 blocked)  [?] Help
```

### Keyboard Shortcuts

#### Navigation

- `j` / `↓` - Move down
- `k` / `↑` - Move up
- `h` / `←` - Collapse/go to parent
- `l` / `→` - Expand/enter directory
- `g` - Go to top
- `G` - Go to bottom
- `PgUp` / `PgDn` - Page up/down

#### Actions

- `Space` - Toggle task status (open ↔ done)
- `w` - Mark as waived
- `b` - Mark as blocked
- `e` - Edit file in `$EDITOR`
- `a` - Add new task
- `d` - Delete task
- `r` - Rename task
- `/` - Search/filter
- `l` (lowercase L) - Filter by label
- `n` - Clear filters

#### Views

- `t` - Change color theme
- `Tab` - Switch between panes
- `?` - Show help overlay
- `q` - Quit

### Features

#### Task Creation

Press `a` to create a new task:

1. Enter task title
2. Optionally add labels, owner, estimate
3. Choose parent task or create top-level
4. Task is created and file is saved

#### Task Editing

Press `e` to open the current file in your default editor:

```bash
# Set your editor preference
export EDITOR=vim  # or nano, emacs, code, etc.
```

The TUI automatically reloads the file after you save and exit.

#### Filtering and Search

Press `/` to enter filter mode:
- Type to filter tasks by title or content
- Press `Esc` to clear filter

Press `l` (lowercase L) to filter by label:
- Select from available labels
- Multiple labels use AND logic

#### Color Schemes

Press `t` to open the theme selector:
- 300+ themes from Gogh collection
- Arrow keys to browse
- Enter to apply
- Popular themes: Nord, Dracula, Solarized, Monokai

Set permanent theme in config:

```bash
lash config set color_scheme "Nord"
```

---

## 8. Best Practices

### Project Organization

#### Use Descriptive Directory Names

```
project/
├── features/          # User-facing features
├── systems/          # Core systems
├── infrastructure/   # DevOps and tooling
├── docs/            # Documentation
└── milestones/      # Release planning
```

#### Keep Related Tasks Together

**Good:** One file per logical component

```
features/
├── authentication.md
├── user-profile.md
└── notifications.md
```

**Avoid:** Too many tiny files or huge monolithic files

#### Use the Index Wisely

Your `lash.index.md` should provide high-level navigation:

```markdown
# My Project

## Core Features

- [ ] [Authentication](features/authentication.md)
- [ ] [User Profiles](features/user-profile.md)

## Infrastructure

- [ ] [CI/CD Pipeline](infrastructure/ci.md)
- [ ] [Deployment](infrastructure/deploy.md)
```

### Task Granularity

#### Break Down Large Tasks

**Too large:**
```markdown
- [ ] Implement entire user management system
```

**Better:**
```markdown
- [ ] User management system
  - [ ] User model and database schema
  - [ ] Registration endpoint
  - [ ] Login/logout endpoints
  - [ ] Password reset flow
  - [ ] Email verification
```

#### Don't Go Too Granular

**Too granular:**
```markdown
- [ ] Implement login
  - [ ] Import bcrypt library
  - [ ] Write function signature
  - [ ] Add password hashing
  - [ ] Add password comparison
  - [ ] Add error handling
  - [ ] Add tests
```

**Better:**
```markdown
- [ ] Implement login endpoint
  - Use bcrypt for password hashing (min cost 12)
  - Return JWT token on success
  - Rate limit to 5 attempts per 15 minutes
  - [ ] Implement core logic
  - [ ] Add tests
```

### Dependency Management

#### Minimize Cross-File Dependencies

Keep most dependencies within files using hierarchy:

```markdown
- [ ] Deploy application
  - [ ] Build Docker image
  - [ ] Push to registry
  - [ ] Update Kubernetes manifests
  - [ ] Apply to cluster
```

Use `@depends-on` only when truly cross-cutting:

```markdown
- [ ] Deploy application @id: deploy
  @depends-on: testing/integration-tests.md#task:all-passing
  @depends-on: infrastructure/k8s-cluster.md
```

#### Document Dependency Rationale

Use contextual notes to explain why dependencies exist:

```markdown
- [ ] Launch public beta @id: public-beta
  @depends-on: security/audit.md#task:pentest
  - Security audit required before public launch per company policy
  - Must complete penetration test with no critical findings
  - [ ] Fix any critical security issues
  - [ ] Get sign-off from security team
```

### Label Conventions

#### Establish Project-Wide Labels

Create a labels guide in your project:

```markdown
# docs/labels.md

## Standard Labels

### Priority
- `p0` - Critical, ship blocker
- `p1` - High priority
- `p2` - Medium priority
- `p3` - Low priority

### Type
- `bug` - Bug fix
- `feature` - New feature
- `refactor` - Code improvement
```

#### Don't Over-Label

**Avoid:**
```markdown
@labels: backend, api, rest, http, server, nodejs, javascript, web, network
```

**Better:**
```markdown
@labels: backend, api
```

#### Use Labels for Workflow

```markdown
# Track review status
@labels: needs-review

# Track deployment stages
@labels: deployed-staging, ready-for-prod

# Track agent ownership
@labels: agent-claude, agent-copilot
```

### Working with AI Agents

#### Use @agent-note Liberally

```markdown
- [ ] Refactor authentication middleware @id: refactor-auth
  @agent-note: Current implementation at src/middleware/auth.rs uses deprecated passport-local. Migrate to passport-jwt. Preserve existing API contract for backward compatibility.
```

#### Add Contextual Notes for Context

```markdown
- [ ] Implement rate limiting
  - Use token bucket algorithm (not sliding window)
  - Store state in Redis (existing connection pool at src/redis.js)
  - Apply to all /api/* routes except /api/health
  - Limit: 100 requests per minute per API key
  - [ ] Implement rate limiter middleware
  - [ ] Add tests for edge cases
  - [ ] Deploy with feature flag
```

#### Use Descriptions for Complex Files

```markdown
## Description

This file tracks the migration from MongoDB to PostgreSQL. The migration
is happening in phases: (1) dual-write to both DBs, (2) verify consistency,
(3) switch reads to Postgres, (4) decommission Mongo. Currently in phase 2.

See ADR-023 for rationale. Existing Mongo schemas are in src/models/mongo/.
New Postgres migrations are in migrations/postgres/.
```

### Version Control

#### Add to .gitignore

```gitignore
# Lash
.lash/lash.db
.lash/*.db-shm
.lash/*.db-wal
```

**Do commit:**
- `lash.index.md`
- All task files (`*.md`)
- `.lash/config.toml` (if project-specific)

**Don't commit:**
- `.lash/lash.db` (database is regenerated from Markdown)

#### Commit Messages

Reference task IDs in commits:

```bash
git commit -m "feat: implement OAuth flows [features.auth#oauth-flows]"
git commit -m "fix: resolve race condition in token refresh [features.auth#jwt-tokens]"
```

#### Pre-commit Hooks

Lint before committing:

```bash
# .git/hooks/pre-commit
#!/bin/bash
lash lint --fix
if [ $? -ne 0 ]; then
  echo "Linting failed. Please fix errors before committing."
  exit 1
fi
```

---

## 9. Troubleshooting

### Common Errors and Solutions

#### E_CFG_ROOT_NOT_FOUND

**Error:** No Lash project root found

**Cause:** Running `lash` outside a Lash project

**Solution:**
```bash
# Initialize a project
lash init

# Or specify project root
lash list --root ~/projects/my-app
```

#### E_LINT_DEPTH_EXCEEDED

**Error:** Task nesting exceeds maximum allowed depth

**Example:**
```markdown
- [ ] Level 1
  - [ ] Level 2
    - [ ] Level 3
      - [ ] Level 4 (exceeds default max of 3)
```

**Solution:**
1. Reduce nesting depth
2. Split into multiple files
3. Or increase max_depth: `lash config set max_depth 4`

#### E_LINT_DUPLICATE_ID

**Error:** Multiple tasks/files use the same `@id`

**Solution:**
1. Find duplicates: `lash lint`
2. Rename conflicting IDs to be unique
3. Update any `@depends-on` references

#### E_DEP_CYCLE

**Error:** Circular dependency detected

**Example:**
```
Task A depends on Task B
Task B depends on Task C
Task C depends on Task A  # Cycle!
```

**Solution:**
1. Run `lash check-links` to identify cycle
2. Review dependency graph: `lash graph`
3. Remove one dependency to break the cycle
4. Restructure task hierarchy if needed

#### E_PARSE_BAD_CHECKBOX

**Error:** Invalid checkbox syntax

**Example:**
```markdown
- [?] Invalid checkbox
- [o] Wrong format
```

**Solution:** Use only valid markers: `[ ]`, `[x]`, `[-]`, `[!]`

```markdown
- [ ] Open task
- [x] Done task
- [-] Waived task
- [!] Blocked task
```

### Database Consistency Issues

#### Symptoms

- Tasks not appearing in `lash list`
- Stale data in queries
- Inconsistent task counts

#### Solutions

**Rebuild the database:**
```bash
lash index --force
```

**Check for drift:**
```bash
lash check-index --diff
```

**Delete and rebuild:**
```bash
rm .lash/lash.db
lash index
```

### Performance Tips

#### Large Projects (>1000 files)

**Use incremental indexing:**
```bash
# Faster - only indexes changed files
lash index

# Slower - rebuilds everything
lash index --force
```

**Filter queries:**
```bash
# Slow - lists all tasks
lash list

# Faster - filtered results
lash list --label backend --status open
```

**Scope graph generation:**
```bash
# Slow - full project graph
lash graph

# Faster - specific scope
lash graph --scope features/
```

#### TUI Performance

For large projects, the TUI may be slow. Optimize by:

1. Filtering before launching: `lash list --label backend | less`
2. Using CLI for bulk operations
3. Splitting large files into smaller ones

#### Search Performance

Full-text search uses SQLite FTS5:

```bash
# Fast - indexed search
lash search "authentication"

# Slower but works - regex patterns
lash search "auth.*token"
```

### Getting Help

#### Built-in Help

```bash
# Command help
lash --help
lash add --help

# Error code explanations
lash explain E_LINT_DEPTH_EXCEEDED
lash explain --list

# TUI help
lash tui  # Press ? for help
```

#### Documentation

- **User Guide:** `docs/user-guide.md` (this file)
- **Design Document:** `docs/design-doc.md`
- **Error Codes:** `docs/error-codes.md`
- **Testing Guide:** `docs/TESTING.md`

#### Logging and Debugging

Enable verbose logging:

```bash
# Verbose output
lash -v list

# Very verbose (debug)
lash -vv index

# Trace level
lash -vvv lint
```

Check logs at: `.lash/logs/lash.log`

#### Common Issues

**Issue:** `lash: command not found`

**Solution:** Ensure `lash` is in your PATH or use full path

```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$PATH:$HOME/.cargo/bin"
```

**Issue:** Database locked error

**Solution:** Close other Lash processes accessing the same project

```bash
# Find processes
ps aux | grep lash

# Kill if needed
kill <PID>

# Rebuild database
lash index --force
```

**Issue:** File parsing errors after manual edits

**Solution:**
```bash
# Check what's wrong
lash lint

# Auto-fix if possible
lash lint --fix

# Interactive fixes
lash lint --interactive
```

**Issue:** Theme/colors not working

**Solution:**
```bash
# Check terminal support
echo $COLORTERM
echo $TERM

# Force colors off if needed
lash list --no-color

# Try different theme
lash tui --color-scheme "Default"
```

---

## Appendix: Quick Reference

### Common Workflows

**Starting a new project:**
```bash
lash init
$EDITOR lash.index.md
lash add "First task" --file features/core.md
lash tui
```

**Daily workflow:**
```bash
lash list --status open --owner me
lash show features/auth.md
lash tui
```

**Adding work:**
```bash
lash add "New feature" --file features/new.md --label p1
$EDITOR features/new.md  # Add details
lash index
lash list --label p1
```

**Before committing:**
```bash
lash lint --fix
lash check-links
git add .
git commit -m "Update task tracking"
```

### Essential Commands Cheat Sheet

| Task | Command |
|------|---------|
| Initialize project | `lash init` |
| Add task | `lash add "Task title"` |
| List all tasks | `lash list` |
| Search tasks | `lash search "query"` |
| Show task details | `lash show task-id` |
| Lint files | `lash lint --fix` |
| Rebuild database | `lash index --force` |
| Launch TUI | `lash tui` |
| Dependency graph | `lash graph` |
| Generate agent prompt | `lash agent-prompt` |
| Explain error | `lash explain E_CODE` |

### File Format Quick Reference

```markdown
# Title

@id: unique.id
@labels: tag1, tag2
@owner: name
@created: YYYY-MM-DD
@depends-on: path/to/file.md#task:id

## Description

Context and implementation notes.

## Tasks

- [ ] Parent task
  - Note providing context
  - [ ] Child task @id: task-id
    @labels: label
    @estimate: 2h
  - [ ] Another child
```

### Status Markers

| Marker | Status | Meaning |
|--------|--------|---------|
| `[ ]` | open | Not started |
| `[x]` | done | Completed |
| `[-]` | waived | Not applicable |
| `[!]` | blocked | Waiting on dependencies |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Parse/lint errors |
| 3 | Dependency errors |
| 4 | I/O errors |
| 5 | Database errors |
| 6 | Configuration errors |

---

**Happy task tracking with Lash!**

For more information, visit the [GitHub repository](https://github.com/fixture-dev/lash).
