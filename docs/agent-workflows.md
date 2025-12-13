# Lash Agent Workflows

This guide describes how AI agents (like Claude Code, ChatGPT, etc.) should use Lash to manage tasks effectively and safely.

## Quick Start for Agents

### Step 1: Get Lash Instructions

```bash
lash agent-prompt --format plain
```

This generates a complete guide containing:
- File format specification
- Allowed operations
- Safety guidelines
- Current project task summaries

### Step 2: Read Relevant Task Files

```bash
# List all task files
lash list

# Show specific task with dependencies
lash show auth.md#task:login --deps

# Search for specific tasks
lash search "authentication"
```

### Step 3: Modify Task Files

Edit `.md` files directly using the format specified in the agent prompt. Common operations:

```markdown
# Adding a new task
- [ ] Implement new feature

# Marking a task complete
- [x] Completed task

# Adding a subtask (indent with 2 spaces)
- [ ] Parent task
  - [ ] Child task

# Waiving a task (not applicable)
- [-] Task no longer needed

# Adding metadata
@labels: backend, api
@owner: alice
@depends-on: core/auth.md#task:login

# Adding contextual notes (plain bullets, no checkbox)
- [ ] Implement payment gateway
  - Use Stripe API v3 for transactions
  - Must handle refunds and partial captures
  - [ ] Set up Stripe account
  - [ ] Implement checkout flow
```

**Important:** Contextual notes (plain bullets without checkboxes) provide requirements, constraints, and implementation hints. They are NOT tracked for completion but are searchable and indexed. Use them for:
- Requirements and constraints
- Acceptance criteria
- Implementation hints
- API/library specifics

See `examples/contextual-notes.md` for comprehensive guidance on when to use notes vs. child tasks.

### Step 4: Validate Changes

**ALWAYS** run lint after editing:

```bash
lash lint path/to/modified-file.md
```

Fix any reported errors before proceeding.

### Step 5: Update Index (Optional)

If you've made significant changes, update the search index:

```bash
lash index
```

## Detailed Workflow

### Workflow 1: Understanding Project Structure

**Goal**: Get an overview of the project's task organization.

1. **Generate exploration prompt**:
   ```bash
   lash agent-prompt --format plain > lash-guide.txt
   ```

2. **List all task files**:
   ```bash
   lash list
   ```

3. **Examine the index file** (if it exists):
   ```bash
   cat lash.index.md  # or index.lash.md
   ```

4. **Use the TUI for interactive exploration**:
   ```bash
   lash tui
   ```

### Workflow 2: Working on a Specific Task

**Goal**: Complete a specific task and update its status.

1. **Find the task**:
   ```bash
   lash search "task description"
   # or
   lash show features/auth.md#task:login
   ```

2. **Check dependencies**:
   ```bash
   lash show features/auth.md#task:login --deps
   ```

3. **Verify no blockers**:
   - Look for `[!]` blocked status in dependencies
   - Use `lash list --blocked` to see all blocked tasks

4. **Complete the work** (your actual implementation)

5. **Update task status**:
   - Edit the `.md` file
   - Change `- [ ]` to `- [x]`
   - Add any relevant notes or annotations

6. **Validate the change**:
   ```bash
   lash lint features/auth.md
   ```

7. **Update index**:
   ```bash
   lash index
   ```

### Workflow 3: Creating Tasks with `lash add`

**Goal**: Programmatically create tasks using the CLI.

The `lash add` command is the recommended way for agents to create tasks. It provides:
- Automatic validation before writing
- JSON output for parsing results
- Support for all task metadata
- Automatic file creation when needed

1. **Create a simple task**:
   ```bash
   lash add "Implement user authentication" --file features/auth.md --format json
   ```

   **JSON Response (success)**:
   ```json
   {
     "success": true,
     "task_id": "implement-user-authentication",
     "file_path": "/project/features/auth.md",
     "line_number": 15,
     "is_new_file": false
   }
   ```

2. **Create a task with full metadata**:
   ```bash
   lash add "Design API schema" \
     --file api/design.md \
     --label backend \
     --label api \
     --owner agent \
     --estimate 4h \
     --agent-note "Use OpenAPI 3.0 spec format" \
     --format json
   ```

3. **Create subtasks under a parent**:
   ```bash
   # First create parent task
   lash add "Build authentication system" \
     --file auth.md \
     --id auth-system \
     --format json

   # Then add subtasks
   lash add "Design database schema" \
     --file auth.md \
     --parent auth-system \
     --format json

   lash add "Implement login endpoint" \
     --file auth.md \
     --parent auth-system \
     --format json
   ```

4. **Create a new task file**:
   ```bash
   lash add "Initial setup" \
     --file features/profile.md \
     --file-title "User Profile Feature" \
     --file-description "Tasks for implementing user profile management" \
     --format json
   ```

5. **Validate before creating (dry run)**:
   ```bash
   lash add "Test task" --file tasks.md --dry-run --format json
   ```

**Error Handling**:

JSON error response:
```json
{
  "success": false,
  "errors": [
    {
      "code": "E_CREATE_PARENT_NOT_FOUND",
      "message": "parent task not found: 'nonexistent-id'",
      "help": "ensure parent task 'nonexistent-id' exists in the target file"
    }
  ]
}
```

Run `lash explain <ERROR_CODE>` for detailed help on any error.

### Workflow 4: Creating a New Task File (Manual)

**Goal**: Add a new feature area with its own task file by editing Markdown directly.

1. **Get the file format**:
   ```bash
   lash agent-prompt --format plain --examples-only
   ```

2. **Create the new file** following the format:
   ```markdown
   # Feature: User Profile Management

   @id: feature-profile
   @labels: frontend, user-mgmt
   @status: open
   @owner: agent
   @created: 2025-01-15

   ## Tasks

   - [ ] Design profile UI
     - [ ] Create wireframes
     - [ ] Get design approval
   - [ ] Implement profile component
   - [ ] Add tests
   ```

3. **Lint the new file**:
   ```bash
   lash lint features/profile.md
   ```

4. **Add to index file** (lash.index.md or index.lash.md):
   ```markdown
   - [features/profile.md](features/profile.md)
   ```

5. **Verify index**:
   ```bash
   lash lint lash.index.md
   ```

6. **Update database**:
   ```bash
   lash index
   ```

### Workflow 5: Adding Cross-File Dependencies

**Goal**: Link a task to work in another file.

1. **Identify the target task**:
   ```bash
   lash show core/auth.md#task:login
   ```

2. **Add dependency annotation** to the dependent task:
   ```markdown
   # In features/profile.md

   @id: feature-profile
   @depends-on: core/auth.md#task:login
   ```

3. **Validate both files**:
   ```bash
   lash lint features/profile.md
   lash lint core/auth.md
   ```

4. **Check for dependency cycles**:
   ```bash
   lash lint  # Runs cross-file validation
   ```

5. **Update index**:
   ```bash
   lash index
   ```

### Workflow 6: Token-Optimized Context Generation

**Goal**: Generate a minimal prompt for a specific task area.

1. **Filter by labels**:
   ```bash
   lash agent-prompt --labels backend,api
   ```

2. **Filter by path**:
   ```bash
   lash agent-prompt --path features/
   ```

3. **Set token budget**:
   ```bash
   lash agent-prompt --max-tokens 2000
   ```

4. **Get JSON output** for programmatic use:
   ```bash
   lash agent-prompt --format json > context.json
   ```

5. **Generate Claude Code skill spec**:
   ```bash
   lash agent-prompt --format claude-skill
   ```

## Safety Guidelines

### Always Do

1. **Run `lash lint` after every modification**
   - This validates your changes before they cause problems
   - Fix all errors before proceeding

2. **Respect depth limits**
   - Maximum nesting: 3-4 levels
   - Don't create deeply nested hierarchies

3. **Maintain status consistency**
   - Parent tasks complete only when all children are done or waived
   - Mark impossible tasks as `[-]` waived

4. **Use unique IDs within each file**
   - IDs must be unique per file
   - Use descriptive, stable identifiers

5. **Validate dependency references**
   - Ensure `@depends-on` targets exist
   - Use correct path resolution (relative paths from project root)

6. **Run `lash index` after significant changes**
   - Updates the search index
   - Rebuilds dependency graph

### Never Do

1. **Don't modify files outside the project**
   - Stay within the project root
   - Respect `.gitignore` and `.lashignore`

2. **Don't break dependency references**
   - Check dependencies before deleting tasks
   - Use `lash show <id> --rdeps` to see reverse dependencies

3. **Don't create circular dependencies**
   - The linter will catch these, but avoid them
   - Task A depends on B, B depends on A = error

4. **Don't skip linting**
   - Always validate before committing
   - Broken files can corrupt the index

5. **Don't use arbitrary status symbols**
   - Only use: `[ ]`, `[x]`, `[-]`, `[!]`
   - Other symbols will cause lint errors

## Error Recovery

### Lint Errors

**Problem**: `lash lint` reports errors.

**Solution**:
1. Read the error message carefully
2. Note the file and line number
3. Fix the reported issue
4. Run `lash lint` again
5. Repeat until clean

Common errors:
- **Duplicate ID**: Change one of the IDs
- **Invalid annotation**: Check spelling and format
- **Depth limit exceeded**: Reduce nesting
- **Inconsistent status**: Fix parent/child status mismatch

### Broken Dependencies

**Problem**: `@depends-on` references a missing task.

**Solution**:
1. **If target was deleted**: Remove the `@depends-on` annotation
2. **If target was moved**: Update the path
3. **If target was renamed**: Update the task ID

```bash
# Find broken links
lash lint  # Shows cross-file dependency errors

# Fix interactively
lash check-links --fix
```

### Circular Dependencies

**Problem**: Tasks depend on each other in a cycle.

**Solution**:
1. Run `lash lint` to identify the cycle
2. Review the cycle path in the error message
3. Remove the weakest dependency link
4. Re-validate with `lash lint`

Example:
```
Error: Circular dependency detected:
  features/auth.md#task:login
  → features/profile.md#task:profile-page
  → features/auth.md#task:login

Suggestion: Remove dependency in features/profile.md
```

### Index Corruption

**Problem**: `lash index` fails or produces errors.

**Solution**:
1. Delete the index: `rm -rf .lash/`
2. Fix any lint errors: `lash lint`
3. Rebuild index: `lash index`

The index is fully reconstructible from Markdown files.

## Integration Examples

### Claude Code Integration

Claude Code can use Lash directly via shell commands:

```python
# In a Claude Code agent script

# Get agent instructions
prompt = shell("lash agent-prompt --format plain")

# Find tasks for current work
tasks = shell("lash list --labels backend --status open")

# Update task status
edit_file("features/auth.md", update_task_status)

# Validate changes
result = shell("lash lint features/auth.md")
if result.exit_code != 0:
    raise Error("Lint failed: " + result.stderr)

# Update index
shell("lash index")
```

### Custom Script Integration

```bash
#!/bin/bash
# agent-task-update.sh

TASK_FILE="$1"
TASK_ID="$2"

# Get current task details
lash show "${TASK_FILE}#task:${TASK_ID}" --format json > task.json

# Agent does work here...

# Update task to done
sed -i "s/@id: ${TASK_ID}/&\n@status: done/" "${TASK_FILE}"

# Validate
if ! lash lint "${TASK_FILE}"; then
    echo "Lint failed, reverting changes"
    git checkout "${TASK_FILE}"
    exit 1
fi

# Update index
lash index

echo "Task ${TASK_ID} completed successfully"
```

### CI/CD Integration

```yaml
# .github/workflows/lash-validation.yml
name: Lash Validation

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Lash
        run: cargo install --path .

      - name: Lint all task files
        run: lash lint

      - name: Verify index is up to date
        run: |
          lash index
          if ! git diff --quiet .lash/; then
            echo "Index is out of date. Run 'lash index' locally."
            exit 1
          fi

      - name: Check for broken links
        run: lash check-links
```

## Advanced Patterns

### Pattern 1: Agent-Owned Tasks

Mark tasks for specific agents:

```markdown
@id: api-implementation
@owner: coding-agent
@agent-note: Use existing auth patterns from core/auth.md
@labels: backend, agent-task
```

Filter for agent tasks:
```bash
lash list --owner coding-agent --status open
```

### Pattern 2: Progressive Disclosure

Generate minimal context first, expand as needed:

```bash
# Start with overview
lash agent-prompt --max-tokens 500 --no-examples

# Get specific area details
lash show features/auth.md --deps

# Full context if needed
lash agent-prompt
```

### Pattern 3: Dependency-First Planning

Before starting work, check the full dependency chain:

```bash
# Show all dependencies (transitive)
lash show features/profile.md#task:profile-page --deps

# Check for blockers
lash list --blocked

# Generate dependency graph
lash graph --format mermaid > deps.mmd
```

### Pattern 4: Incremental Validation

Validate as you go, not all at once:

```bash
# After each file edit
lash lint modified-file.md

# Before committing
lash lint

# After batch updates
lash index
lash check-index
```

## Output Formats Reference

### Plain Text (Default)

Human-readable Markdown format:

```bash
lash agent-prompt --format plain
```

Use for:
- Reading in documentation
- Pasting into agent contexts
- Manual reference

### JSON

Structured data format:

```bash
lash agent-prompt --format json
```

Use for:
- Programmatic parsing
- API integration
- Data processing

### Claude Code Skill

Skill specification format:

```bash
lash agent-prompt --format claude-skill
```

Use for:
- Claude Code skill definitions
- Tool configuration
- Command reference

### Agents.md Fragment

Ready-to-paste documentation:

```bash
lash agent-prompt --format agents-md >> AGENTS.md
```

Use for:
- Project documentation
- Team onboarding
- Agent setup guides

## Best Practices Summary

1. **Always get fresh instructions**: Run `lash agent-prompt` at the start of each session
2. **Validate early and often**: Lint after every change
3. **Understand dependencies**: Check deps before starting work
4. **Use token budgets**: Generate minimal context for efficiency
5. **Maintain consistency**: Follow parent/child status rules
6. **Document agent notes**: Use `@agent-note` for important context
7. **Keep IDs stable**: Don't rename IDs without updating references
8. **Test before committing**: Ensure `lash lint` passes
9. **Update the index**: Run `lash index` after batch changes
10. **Recover gracefully**: Know how to fix common errors

## Troubleshooting

### "No project root found"

**Cause**: Not in a Lash project.

**Solution**: Create an index file at the project root:
```bash
echo "# Project Tasks" > lash.index.md
lash index
```

### "Database does not exist"

**Cause**: Index not yet created.

**Solution**:
```bash
lash index
```

### "Lint errors prevent indexing"

**Cause**: Invalid task files.

**Solution**:
```bash
lash lint  # Fix all reported errors first
lash index  # Then rebuild index
```

### "Token budget too small"

**Cause**: Requested budget can't fit critical sections.

**Solution**: Increase budget or reduce filters:
```bash
lash agent-prompt --max-tokens 4000  # Increase budget
# or
lash agent-prompt --labels specific-area  # Narrow scope
```

## Reference

### Commands Summary

| Command | Purpose | Example |
|---------|---------|---------|
| `lash add` | Create a new task | `lash add "Fix bug" --file bugs.md --format json` |
| `lash agent-prompt` | Generate agent instructions | `lash agent-prompt --format plain` |
| `lash lint` | Validate task files | `lash lint path/to/file.md` |
| `lash format` | Auto-format task files | `lash format --fix` |
| `lash index` | Build/rebuild search index | `lash index` |
| `lash list` | List tasks with filters | `lash list --labels backend` |
| `lash search` | Full-text search | `lash search "auth"` |
| `lash show` | Display task details | `lash show file.md#task:id` |
| `lash explain` | Explain an error code | `lash explain E_CREATE_EMPTY_TITLE` |
| `lash graph` | Export dependency graph | `lash graph --format mermaid` |
| `lash check-links` | Find broken dependencies | `lash check-links --fix` |
| `lash tui` | Launch interactive UI | `lash tui` |

### Status Symbols

| Symbol | Name | Meaning |
|--------|------|---------|
| `[ ]` | open | Not started or in progress |
| `[x]` | done | Completed successfully |
| `[-]` | waived | Not applicable or cancelled |
| `[!]` | blocked | Blocked by dependencies |

### Annotations

| Annotation | Purpose | Example |
|------------|---------|---------|
| `@id` | Unique identifier | `@id: feature-auth` |
| `@labels` | Tags for filtering | `@labels: backend, api` |
| `@status` | Overall status | `@status: in-progress` |
| `@owner` | Assignee | `@owner: alice` |
| `@created` | Creation date | `@created: 2025-01-15` |
| `@estimate` | Time estimate | `@estimate: 2d` |
| `@depends-on` | Cross-file dependency | `@depends-on: auth.md#task:login` |
| `@agent-note` | Agent instructions | `@agent-note: Use pattern X` |

## Getting Help

- Run `lash --help` for command reference
- Run `lash <command> --help` for command-specific help
- Generate fresh instructions: `lash agent-prompt`
- Check project documentation in `/docs/`
- Lint early, lint often: `lash lint`
