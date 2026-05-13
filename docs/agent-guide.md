# Lash Agent Integration Guide

**Version:** 1.0
**Target Audience:** AI Agents (Claude Code, ChatGPT, and similar LLM-based coding assistants)
**Last Updated:** 2025-01-15

---

## 1. Introduction for Agents

### What is Lash?

Lash is a minimalist, Markdown-native task tracker designed for both humans and AI agents. It treats Markdown files as the single source of truth, with SQLite providing an acceleration layer for fast search and queries.

**Key characteristics:**
- **Markdown-first**: All task data lives in `.md` files with a strict, predictable format
- **Linter-enforced**: Format is validated and enforced for consistency
- **Dependency-aware**: Tasks can depend on other tasks within or across files
- **Token-efficient**: Designed to minimize token usage in agent contexts
- **Fast**: Optimized for quick parsing, indexing, and querying

### Why Agents Should Use Lash

1. **Predictable format**: Strict linting ensures consistent structure you can rely on
2. **Safe modifications**: Clear rules about what can and cannot be changed
3. **Validation feedback**: Immediate feedback via `lash lint` prevents errors
4. **Token optimization**: Built-in tools for generating minimal context (`lash agent-prompt`)
5. **Structured errors**: Machine-readable error codes with clear explanations

### Agent-Friendly Features

- **Formal schema**: Complete specification of allowed syntax and semantics
- **Contextual notes**: Plain bullets for requirements without completion tracking
- **Documentation references**: `@doc` annotations link to relevant documentation
- **Sparse context generation**: Tools to extract only relevant information
- **Workflow commands**: Purpose-built CLI for agent operations

---

## 2. Getting Started (for Agents)

### Obtaining the Lash Schema

To understand the Lash file format, run:

```bash
lash agent-prompt --format plain
```

This generates:
- Complete file format specification
- Allowed operations
- Safety guidelines
- Current project task summaries (if database exists)

**Output formats:**
- `--format plain`: Human-readable Markdown (default)
- `--format json`: Structured JSON with schema and tasks
- `--format agents-md`: Ready-to-paste fragment for AGENTS.md

> To install a static Lash skill into a coding-agent's skills directory
> (Claude Code's `.claude/skills/`, Cursor's `.cursor/rules/`, or an
> `AGENTS.lash.md` sibling for Codex / generic AGENTS.md hosts), use
> `lash skill install --target <claude|codex|cursor|agents-md>` instead —
> see the user guide for details.

### Understanding the File Format

Lash task files have four main sections:

1. **Header**: Title (H1) and metadata annotations
2. **Description**: Optional `## Description` section with context
3. **Tasks**: `## Tasks` section with hierarchical checkboxes
4. **References**: Optional notes or documentation links

**File structure:**
```markdown
# Topic Title

@id: unique-identifier
@labels: tag1, tag2
@owner: assignee-name
@created: YYYY-MM-DD

## Description

Free-form Markdown text explaining scope, constraints, and context.
Can include inline @agent-note: hints for agents.

## Tasks

- [ ] Top-level task
  - Implementation note without checkbox (contextual note)
  - [ ] Child task (indented with 2 spaces)
    - [ ] Grandchild task
- [x] Completed task
- [-] Waived task (not applicable)
```

### Allowed Operations

**Safe operations (always permitted):**
1. Add new tasks using checkbox syntax: `- [ ] Task description`
2. Update task status: `[ ]` → `[x]` (done), `[-]` (waived), `[!]` (blocked)
3. Add contextual notes: plain bullets under tasks (no checkbox)
4. Add annotations: `@labels`, `@owner`, `@estimate`, `@agent-note`
5. Add dependencies: `@depends-on: path/to/file.md#task:id`
6. Add documentation references: `@doc: path/to/doc.md#section`

**Restricted operations (use with caution):**
1. Removing tasks (check for reverse dependencies first)
2. Renaming task IDs (breaks references)
3. Restructuring hierarchy (may affect dependencies)

**Forbidden operations:**
1. Creating tasks beyond maximum depth (3-4 levels)
2. Using invalid status symbols (only `[ ]`, `[x]`, `[-]`, `[!]`)
3. Creating duplicate IDs within a file
4. Breaking existing dependency references
5. Modifying files outside the project root

---

## 3. File Format Schema

### Formal Specification

#### 3.1 File Structure

```
TASK_FILE := HEADER DESCRIPTION? TASKS REFERENCES?

HEADER := H1_TITLE NL ANNOTATIONS*

DESCRIPTION := "## Description" NL FREE_TEXT NL

TASKS := "## Tasks" NL TASK_TREE

TASK_TREE := TASK_ITEM+

TASK_ITEM := CHECKBOX_ITEM | NOTE_ITEM
```

#### 3.2 Tasks and Notes

**Task item (checkbox):**
```
CHECKBOX_ITEM := INDENT* "- [STATUS] " TITLE (INLINE_LABELS?) NL
                 (NOTE_ITEM | CHILD_TASK)*

STATUS := " " | "x" | "-" | "!"

INDENT := "  " (2 spaces per level, max 3-4 levels)
```

**Note item (plain bullet):**
```
NOTE_ITEM := INDENT "- " TEXT NL

where INDENT is exactly 2 spaces deeper than parent task
```

**Key distinction:**
- `- [ ]` or `- [x]` or `- [-]` or `- [!]` = **Task** (checkbox, tracked for completion)
- `- ` (plain bullet) = **Note** (contextual information, not tracked)

#### 3.3 Annotations Reference

All annotations are optional unless marked required.

| Annotation | Type | Description | Example |
|------------|------|-------------|---------|
| `@id` | string | Unique identifier within file | `@id: feature-auth` |
| `@labels` | comma-separated | Tags for filtering | `@labels: backend, api` |
| `@owner` | string | Person/agent responsible | `@owner: alice` |
| `@created` | date | Creation date (YYYY-MM-DD) | `@created: 2025-01-15` |
| `@estimate` | duration | Time estimate | `@estimate: 2d` |
| `@depends-on` | reference | Cross-file dependency | `@depends-on: core/auth.md#task:login` |
| `@agent-note` | text | Hints for AI agents | `@agent-note: Use pattern X` |
| `@doc` | path | Documentation reference | `@doc: ../docs/design.md#section-7` |

**Annotation placement:**
- File-level: After H1 title, before `## Description`
- Task-level: On line following task, indented to match task

#### 3.4 Dependency Syntax

**Within-file dependencies:**
Automatic based on hierarchy. Parent tasks depend on all children.

**Cross-file dependencies:**
```markdown
@depends-on: path/to/file.md#task:task-id
@depends-on: ../sibling/file.md#task:other-id
```

Paths are relative to project root or relative file paths.

**Documentation references (non-blocking):**
```markdown
@doc: ../docs/design-doc.md#section-name
@doc: ../../README.md
```

#### 3.5 Status Values

| Symbol | Name | Meaning | Usage |
|--------|------|---------|-------|
| `[ ]` | open | Not started or in progress | Default for new tasks |
| `[x]` | done | Completed successfully | Mark when work is finished |
| `[-]` | waived | Not applicable or cancelled | Use when task no longer needed |
| `[!]` | blocked | Blocked by dependencies | System may auto-set based on deps |

#### 3.6 Constraints and Rules

1. **Unique IDs**: `@id` must be unique within each file (globally unique = `file-path#task:id`)
2. **Max depth**: Task hierarchies limited to 3-4 levels of nesting
3. **Status consistency**: Parent tasks can only be `[x]` when all children are `[x]` or `[-]`
4. **Valid dependencies**: `@depends-on` targets must exist and be resolvable
5. **Contextual notes**:
   - Must be indented exactly 2 spaces deeper than parent task
   - Cannot have children (no nesting under notes)
   - Should appear before child tasks (convention, soft warning)
   - Are indexed and searchable but not tracked for completion
6. **Description length**: Recommended 500-1000 chars, warning at 1000, error at 2000

---

## 4. Safe Modifications

### 4.1 Adding Tasks

**To add a top-level task:**
```markdown
## Tasks

- [ ] New task description
```

**To add a subtask:**
```markdown
- [ ] Parent task
  - [ ] New child task (indent with 2 spaces)
```

**To add a task with metadata:**
```markdown
- [ ] Implement authentication
  @id: auth-impl
  @labels: backend, security
  @estimate: 3d
  @agent-note: Use bcrypt for password hashing
```

**Validation:**
```bash
lash lint path/to/file.md
```

### 4.2 Adding Contextual Notes

Contextual notes provide requirements, constraints, or implementation hints without being tracked as tasks.

**When to use notes vs. child tasks:**

Use **notes** for:
- Requirements: "Must support multi-tenancy"
- Constraints: "Response time < 100ms for 95th percentile"
- Implementation hints: "Use Redis for session storage"
- API specifics: "Stripe API v3, not v2"

Use **child tasks** for:
- Multi-step processes needing completion tracking
- Independently trackable work items
- Sub-tasks that may have their own sub-tasks

**Example:**
```markdown
- [ ] Implement payment processing
  - Use Stripe API v3 for all transactions
  - Must handle webhooks for async payment confirmation
  - Support credit card, ACH, and digital wallets
  - [ ] Set up Stripe webhook handlers
  - [ ] Implement payment intent creation
  - [ ] Add refund support
```

### 4.3 Updating Task Status

**Mark task as done:**
```markdown
- [x] Completed task
```

**Mark task as waived:**
```markdown
- [-] Task no longer needed
```

**Mark task as blocked:**
```markdown
- [!] Task blocked by external factor
```

**Important:** Parent tasks automatically become `[!]` blocked if any child is `[ ]` open. Don't manually mark parents as `[x]` done when children are incomplete.

### 4.4 Adding Dependencies

**Add cross-file dependency:**
```markdown
@depends-on: features/auth.md#task:login-endpoint
```

**Add documentation reference:**
```markdown
@doc: ../docs/api-spec.md#authentication
```

**Validation:**
```bash
lash check-links  # Verify all references are valid
```

### 4.5 Adding Labels

**File-level labels:**
```markdown
@labels: backend, api, security
```

**Task-level labels (inline):**
```markdown
- [ ] Implement OAuth flow #security #auth
```

Labels enable cross-cutting queries:
```bash
lash list --label security
```

### 4.6 What NOT to Do

**Don't:**
1. Create tasks deeper than 3-4 levels
2. Use arbitrary status symbols (only `[ ]`, `[x]`, `[-]`, `[!]`)
3. Duplicate IDs within a file
4. Break dependency references by deleting target tasks
5. Mark parent tasks as done when children are incomplete
6. Create circular dependencies (A depends on B, B depends on A)
7. Add checkboxes to contextual notes (notes are plain bullets only)
8. Nest notes under other notes (notes cannot have children)
9. Modify files without running `lash lint` afterward

---

## 5. Workflows

### Workflow 1: Get Context

**Goal:** Understand the project structure and current task state.

```bash
# Generate agent-friendly prompt with format spec
lash agent-prompt --format plain > lash-context.txt

# List all task files
lash list --tree

# Search for relevant tasks
lash search "authentication"

# View specific task with dependencies
lash show features/auth.md#task:login --deps
```

**Token optimization:**
```bash
# Filter by labels to reduce context
lash agent-prompt --labels backend,security

# Set token budget (approximate)
lash agent-prompt --max-tokens 2000

# Get only schema without examples
lash agent-prompt --format json
```

### Workflow 2: Read Task Files

**Goal:** Read and understand task files before making changes.

```bash
# Read file with standard tools
cat features/auth.md

# Or use show command for formatted view
lash show features/auth.md
```

**What to check:**
1. Current `@id` and `@labels`
2. Task hierarchy and depth
3. Existing dependencies (`@depends-on`)
4. Contextual notes for requirements
5. Documentation references (`@doc`)

### Workflow 3: Modify Tasks

**Goal:** Make changes to task files safely.

**Steps:**
1. Read the file first
2. Make changes following format rules
3. Validate with `lash lint`
4. Update index if needed

**Example:**
```bash
# Edit file (use your preferred method)
# Add new task:
# - [ ] Implement password reset

# Validate immediately
lash lint features/auth.md

# If lint passes, update index
lash index
```

**Using programmatic creation:**
```bash
# Use lash add for safer task creation
lash add "Implement user registration" \
  --file features/auth.md \
  --label backend \
  --label security \
  --format json
```

### Workflow 4: Validate Changes

**Goal:** Ensure changes are valid before committing.

```bash
# Lint specific file
lash lint features/auth.md

# Lint all files
lash lint

# Check for broken links
lash check-links

# Verify index consistency
lash check-index
```

**Handle errors:**
```bash
# Get detailed explanation of error code
lash explain E001

# List all error codes
lash explain --list
```

### Workflow 5: Update Index

**Goal:** Rebuild the SQLite index after making changes.

```bash
# Update index (incremental)
lash index

# Force full rebuild
lash index --force

# Verify consistency
lash check-index
```

**When to run:**
- After adding new tasks
- After modifying task status
- After changing dependencies
- After bulk edits

**Not needed for:**
- Reading tasks
- Generating prompts (uses stale data if index is out of date)

---

## 6. Error Handling

### Common Errors and Solutions

#### E001: Duplicate Task ID
**Cause:** Two tasks in the same file have the same `@id`.

**Solution:**
```bash
# Find the duplicate
lash lint features/auth.md

# Change one of the IDs to be unique
@id: login-endpoint-v2
```

#### E002: Invalid Dependency Reference
**Cause:** `@depends-on` points to a non-existent task.

**Solution:**
```bash
# Check what exists
lash list

# Update reference to correct path/ID
@depends-on: core/auth.md#task:correct-id

# Or remove invalid dependency
```

#### E003: Maximum Depth Exceeded
**Cause:** Task hierarchy too deep (>3-4 levels).

**Solution:**
Flatten the hierarchy or split into multiple files.

#### E004: Invalid Status Symbol
**Cause:** Used a checkbox symbol other than `[ ]`, `[x]`, `[-]`, `[!]`.

**Solution:**
```bash
# Change to valid symbol
- [ ] Task  (not - [o] or - [v])
```

#### E005: Parent Status Inconsistent
**Cause:** Parent task marked `[x]` but children are `[ ]` or `[!]`.

**Solution:**
Either complete all children or waive them:
```markdown
- [ ] Parent (change from [x] to [ ])
  - [x] Child 1
  - [-] Child 2 (waive if not needed)
```

### Lint Error Recovery

**General recovery process:**
1. Run `lash lint` to identify errors
2. Read error message carefully (includes file, line, and cause)
3. Fix the specific issue mentioned
4. Re-run `lash lint` to verify
5. Repeat until clean

**Example error output:**
```
features/auth.md:42: E002: Invalid dependency reference
  @depends-on: core/nonexistent.md#task:foo
  Target file or task not found.

  Suggestion: Verify the path and task ID exist.
```

### Broken Dependency Fixes

**If a dependency target was deleted:**
```bash
# Option 1: Remove the dependency
# (edit file and delete @depends-on line)

# Option 2: Update to new target
@depends-on: new/path.md#task:new-id
```

**If a dependency target was moved:**
```bash
# Update path to new location
@depends-on: features/auth/login.md#task:endpoint
```

**Automated checking:**
```bash
# Find all broken links
lash check-links

# Attempt automatic fix (interactive)
lash check-links --fix
```

### Index Corruption

**Symptoms:**
- Search returns incorrect results
- `lash check-index` fails
- Database errors

**Solution:**
```bash
# Delete index and rebuild
rm -rf .lash/lash.db
lash index
```

The index is fully reconstructible from Markdown files.

---

## 7. Token Minimization

### Using Sparse Context

Instead of loading entire task files, use sparse context generation:

```bash
# Get context for specific task with dependencies
lash show features/auth.md#task:login --deps

# Output shows:
# - The specific task
# - Its immediate dependencies (status only)
# - Summary instead of full content
```

**Benefits:**
- Reduced token usage
- Focused context
- Faster processing

### ID-Based References

Instead of copying full task descriptions, refer by ID:

**Verbose (more tokens):**
```
Work on the task "Implement user authentication with OAuth 2.0
and JWT tokens" in features/auth.md
```

**Concise (fewer tokens):**
```
Work on features/auth.md#task:auth-impl
```

Use `lash show features/auth.md#task:auth-impl` to get details only when needed.

### Summarization Strategies

**Use task counts instead of full lists:**
```bash
lash list --format json | jq '.summary'

# Output:
# {
#   "total": 50,
#   "completed": 30,
#   "open": 15,
#   "blocked": 5
# }
```

**Filter to relevant subset:**
```bash
# Only backend tasks
lash agent-prompt --labels backend

# Only specific directory
lash agent-prompt --path features/auth/

# Combination
lash agent-prompt --labels security --path features/
```

### Progressive Disclosure

Start with minimal context, expand as needed:

1. **First:** Get overview
   ```bash
   lash list --tree
   ```

2. **Then:** Get specific file
   ```bash
   lash show features/auth.md
   ```

3. **Finally:** Get full context if needed
   ```bash
   lash agent-prompt --path features/auth/
   ```

---

## 8. Examples

### Example 1: Obtaining Agent Prompt

**Command:**
```bash
lash agent-prompt --format plain
```

**Output (truncated):**
```markdown
# Lash Agent Usage Guide

## Overview

Lash is a minimalist, Markdown-native task tracker where:
- Markdown files are the single source of truth
- Tasks are hierarchical checkbox lists with annotations
...

## File Format

# Lash Task File Format

Hierarchical Markdown checkboxes with annotations

**Version:** 1.0

## Annotations
- `@id`: Unique identifier within file
  - Example: `@id: feature-auth`
...
```

### Example 2: Reading and Modifying a Task

**Step 1: Read the file**
```bash
cat features/auth.md
```

**Content:**
```markdown
# Feature: Authentication

@id: feature-auth
@labels: backend, security

## Description

User authentication system with OAuth 2.0 and JWT tokens.

## Tasks

- [ ] Implement login endpoint
  - [ ] Add password validation
  - [ ] Generate JWT tokens
- [ ] Implement registration
```

**Step 2: Make modifications**
```markdown
# Feature: Authentication

@id: feature-auth
@labels: backend, security

## Description

User authentication system with OAuth 2.0 and JWT tokens.

## Tasks

- [ ] Implement login endpoint
  - Use bcrypt for password hashing
  - JWT tokens expire after 24 hours
  - [x] Add password validation
  - [ ] Generate JWT tokens
- [ ] Implement registration
  - [ ] Validate email format
  - [ ] Send confirmation email
```

**Step 3: Validate**
```bash
lash lint features/auth.md
# Output: ✓ features/auth.md is valid
```

**Step 4: Update index**
```bash
lash index
# Output: Indexed 1 file, 5 tasks
```

### Example 3: Querying Tasks

**Find tasks by label:**
```bash
lash list --label backend --status open

# Output:
# - features/auth.md#task:login-endpoint: Implement login endpoint (open)
# - features/auth.md#task:registration: Implement registration (open)
```

**Search for specific term:**
```bash
lash search "JWT"

# Output:
# features/auth.md:12: Generate JWT tokens
# features/auth.md:15: JWT tokens expire after 24 hours
```

**View task with dependencies:**
```bash
lash show features/profile.md#task:profile-page --deps

# Output:
# Task: Profile page implementation
# Status: blocked
# Dependencies:
#   - features/auth.md#task:login-endpoint (open) [BLOCKING]
```

### Example 4: Creating Tasks Programmatically

**Using lash add command:**
```bash
lash add "Implement password reset" \
  --file features/auth.md \
  --label backend \
  --label security \
  --estimate 4h \
  --agent-note "Use email-based reset tokens with 1-hour expiry" \
  --format json
```

**JSON response:**
```json
{
  "success": true,
  "task_id": "implement-password-reset",
  "file_path": "/project/features/auth.md",
  "line_number": 23,
  "is_new_file": false
}
```

### Example 5: Completing Tasks Programmatically

**Using lash complete command:**
```bash
# Complete a single task
lash complete features.auth#implement-login --json

# Complete multiple tasks at once
lash complete features.auth#task-1 features.auth#task-2 --json
```

**JSON response (success):**
```json
{
  "success": true,
  "completed": [
    {
      "task_id": "features.auth#implement-login",
      "file_path": "features/auth.md",
      "previous_status": "open"
    }
  ],
  "errors": []
}
```

**JSON response (task not found with suggestions):**
```json
{
  "success": false,
  "completed": [],
  "errors": [
    {
      "task_id": "features.auth#implment-login",
      "code": "E_NOT_FOUND",
      "message": "Task not found: features.auth#implment-login",
      "suggestions": ["features.auth#implement-login"]
    }
  ]
}
```

**Dry run (preview without changes):**
```bash
lash complete --dry-run features.auth#implement-login

# Output:
# Would complete:
#   [x] features.auth#implement-login (features/auth.md)
```

**Exit codes:**
- `0` - All tasks completed successfully
- `1` - Validation error (task already complete, waived, etc.)
- `5` - Task not found

### Example 6: Handling Errors

**Scenario: Accidentally create duplicate ID**

**Edit:**
```markdown
- [ ] Task one
  @id: duplicate-id
- [ ] Task two
  @id: duplicate-id
```

**Validate:**
```bash
lash lint features/auth.md

# Output:
# features/auth.md:15: E001: Duplicate task ID 'duplicate-id'
#   First occurrence: line 12
#   Duplicate found: line 15
#
#   Fix: Ensure each @id is unique within the file.
```

**Fix:**
```markdown
- [ ] Task one
  @id: duplicate-id
- [ ] Task two
  @id: unique-id-2
```

**Re-validate:**
```bash
lash lint features/auth.md
# Output: ✓ features/auth.md is valid
```

---

## 9. Integration Patterns

### Pattern 1: Session-Based Workflow

At the start of each agent session:

1. Get fresh instructions: `lash agent-prompt --format plain`
2. Understand current state: `lash list --tree`
3. Filter to relevant area: `lash search <relevant-term>`
4. Work on tasks, validating after each change
5. Update index before session ends

### Pattern 2: Task-Focused Workflow

When working on a specific task:

1. Find the task: `lash search <description>` or `lash list --label <label>`
2. Get task details: `lash show <task-id> --deps`
3. Read the task file
4. Make changes
5. Validate: `lash lint <file>`
6. Update index: `lash index`
7. Mark task as done when complete

### Pattern 3: Documentation-Driven

Use `@doc` references to find relevant documentation:

1. Read task file, note `@doc` annotations
2. Read referenced documentation
3. Apply guidance from docs to implementation
4. Update task with progress

**Example:**
```markdown
- [ ] Implement caching layer
  @doc: ../docs/architecture.md#caching-strategy
  - Use Redis for distributed cache
  - TTL of 1 hour for user sessions
```

### Pattern 4: Dependency-First

Before starting work, check dependency chain:

1. Show task with dependencies: `lash show <id> --deps`
2. Verify no blockers: `lash list --blocked`
3. If blocked, work on dependencies first
4. Mark dependencies as done
5. Proceed with original task

---

## 10. Best Practices

### For Agents

1. **Always validate**: Run `lash lint` after every file modification
2. **Check dependencies first**: Use `lash show --deps` before starting work
3. **Use contextual notes**: Add plain bullets for requirements and constraints
4. **Keep descriptions concise**: Target 500-1000 characters
5. **Respect depth limits**: Maximum 3-4 levels of nesting
6. **Update index after batch changes**: Run `lash index` when done
7. **Reference documentation**: Use `@doc` to link relevant resources
8. **Use progressive disclosure**: Start with minimal context, expand as needed
9. **Prefer programmatic creation**: Use `lash add` for safer task creation
10. **Handle errors gracefully**: Use `lash explain <code>` for error details

### Common Pitfalls

1. **Forgetting to lint**: Always validate before considering changes complete
2. **Ignoring depth limits**: Don't create deeply nested hierarchies
3. **Breaking dependencies**: Check reverse dependencies before deleting tasks
4. **Inconsistent status**: Don't mark parents done when children are incomplete
5. **Skipping index update**: Database queries use stale data until index is updated
6. **Confusing notes and tasks**: Remember plain bullets are notes, checkboxes are tasks

### Performance Tips

1. **Use filters**: `--label` and `--path` reduce context size
2. **Set token budgets**: `--max-tokens` prevents context overflow
3. **Use JSON format**: Structured output is easier to parse programmatically
4. **Batch operations**: Make multiple edits, then lint once
5. **Incremental indexing**: `lash index` is incremental by default

---

## 11. Quick Reference

### Essential Commands

```bash
# Get agent instructions
lash agent-prompt --format plain

# Validate files
lash lint [file]

# List tasks
lash list [--label <label>] [--status <status>]

# Search tasks
lash search <query>

# Show task details
lash show <task-id> [--deps]

# Update index
lash index

# Check for broken links
lash check-links

# Explain error
lash explain <error-code>
```

### File Format Quick Reference

```markdown
# Topic Title

@id: unique-id
@labels: tag1, tag2
@depends-on: other/file.md#task:other-id
@doc: ../docs/reference.md#section

## Description

Context and requirements (500-1000 chars recommended).

## Tasks

- [ ] Task (checkbox = tracked)
  - Plain bullet = note (not tracked)
  - Requirement or constraint here
  - [ ] Child task
    - [ ] Grandchild task
- [x] Completed task
- [-] Waived task
- [!] Blocked task
```

### Status Reference

- `[ ]` = open (not started or in progress)
- `[x]` = done (completed)
- `[-]` = waived (not applicable)
- `[!]` = blocked (dependencies incomplete)

### Annotation Reference

- `@id` = unique identifier
- `@labels` = comma-separated tags
- `@owner` = assignee
- `@created` = creation date (YYYY-MM-DD)
- `@estimate` = time estimate
- `@depends-on` = cross-file dependency
- `@agent-note` = hints for AI agents
- `@doc` = documentation reference

---

## 12. Troubleshooting

### "Project root not found"

**Cause:** Not in a Lash project directory.

**Solution:**
```bash
# Create an index file at project root
echo "# Project Tasks" > lash.index.md
lash index
```

### "Database does not exist"

**Cause:** Index not yet created.

**Solution:**
```bash
lash index
```

### "Lint errors prevent indexing"

**Cause:** Invalid task files.

**Solution:**
```bash
# Fix all lint errors first
lash lint

# Then rebuild index
lash index
```

### Search returns stale results

**Cause:** Index out of date.

**Solution:**
```bash
lash index  # Update index
```

### Task appears blocked incorrectly

**Cause:** Dependency may be resolved but index not updated.

**Solution:**
```bash
lash index        # Update index
lash show <id>    # Verify current status
```

---

## 13. Additional Resources

- **User Guide**: `docs/user-guide.md` - Comprehensive user documentation
- **Design Document**: `docs/design-doc.md` - Technical specification
- **Error Codes**: `docs/error-codes.md` - Complete error reference
- **Examples**: `examples/` - Sample projects and tutorials

---

## 14. Conclusion

This guide provides a complete reference for AI agents to use Lash safely and effectively. Key takeaways:

1. **Format is strict**: Follow the schema exactly
2. **Validation is essential**: Always run `lash lint` after changes
3. **Context is tunable**: Use filters and token budgets to minimize overhead
4. **Errors are structured**: Machine-readable codes with clear explanations
5. **Dependencies matter**: Check before deleting or restructuring

For the most up-to-date information, run:
```bash
lash agent-prompt --format plain
```

This command generates fresh instructions based on the current project state.

---

**Document Version:** 1.0
**Last Updated:** 2025-01-15
**Feedback:** File issues at project repository
