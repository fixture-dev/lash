# Task Creation Guide

This guide explains how to create tasks using Lash's CLI and TUI interfaces.

## Overview

Lash provides two ways to create tasks:

1. **CLI**: `lash add` command for quick, scriptable task creation
2. **TUI**: Interactive modal for visual task creation with autocomplete

Both interfaces share the same validation rules and create identical Markdown output.

## CLI Task Creation

### Basic Usage

```bash
# Create a simple top-level task
lash add "Implement user authentication"

# Create a task in a specific file
lash add "Add login form" --file features/auth.md

# Create a subtask under an existing parent
lash add "Write unit tests" --parent setup-database

# Create a task with labels
lash add "Fix memory leak" --label bug --label urgent

# Create a task with full metadata
lash add "Design API schema" \
  --file api/design.md \
  --label backend \
  --owner alice \
  --estimate 4h \
  --id api-schema-design
```

### Command Reference

```
lash add [OPTIONS] <TITLE>
```

#### Required Argument

- `<TITLE>` - The task title (max 200 characters)

#### File Options

| Option | Short | Description |
|--------|-------|-------------|
| `--file <PATH>` | `-f` | Target file path (creates if doesn't exist) |
| `--file-title <TITLE>` | | Title for new file header (only when creating) |
| `--file-description <DESC>` | | Description for new file (only when creating) |

#### Position Options

| Option | Short | Description |
|--------|-------|-------------|
| `--parent <ID>` | `-p` | Parent task ID (creates subtask) |
| `--after <ID>` | | Insert after this task ID |
| `--before <ID>` | | Insert before this task ID |

#### Metadata Options

| Option | Short | Description |
|--------|-------|-------------|
| `--label <LABEL>` | `-l` | Add label (repeatable: `-l bug -l urgent`) |
| `--owner <OWNER>` | `-o` | Task owner/assignee |
| `--estimate <TIME>` | `-e` | Time estimate (e.g., `30m`, `2h`, `1d`, `2w`) |
| `--id <ID>` | | Explicit task ID (auto-generated if omitted) |
| `--status <STATUS>` | | Initial status: `open`, `done`, `waived`, `blocked` |
| `--depends-on <DEPS>` | | Dependencies (repeatable) |
| `--agent-note <NOTE>` | | Note for AI agents |

#### Output Options

| Option | Description |
|--------|-------------|
| `--format <FORMAT>` | Output format: `text` (default) or `json` |
| `--dry-run` | Validate without creating the task |
| `--interactive, -i` | Interactive mode (prompt for missing fields) |

### Examples

#### Creating a Task File from Scratch

```bash
# Create a new task file with initial tasks
lash add "Set up project" \
  --file tasks/setup.md \
  --file-title "Project Setup Tasks" \
  --file-description "Tasks for initial project configuration"
```

#### Creating a Task Hierarchy

```bash
# Create parent task
lash add "Build authentication system" --file auth.md --id auth-system

# Create subtasks
lash add "Design database schema" --file auth.md --parent auth-system
lash add "Implement login endpoint" --file auth.md --parent auth-system
lash add "Add password hashing" --file auth.md --parent auth-system
```

#### Agent-Friendly Task Creation

```bash
# Create task with JSON output for parsing
lash add "Fix regression bug" \
  --file bugs.md \
  --label bug \
  --format json

# Output:
# {
#   "success": true,
#   "task_id": "fix-regression-bug",
#   "file_path": "/project/bugs.md",
#   "line_number": 42,
#   "is_new_file": false
# }
```

#### Dry Run Validation

```bash
# Check if task would be valid without creating it
lash add "Test task" --file tasks.md --dry-run

# On success: "Validation passed. Task would be created at line 15"
# On failure: Shows validation errors
```

## TUI Task Creation

### Opening the Modal

In the TUI, press `a` or `n` to open the task creation modal.

### Form Fields

The modal presents these fields (top to bottom):

1. **Title** (required)
   - Enter the task title
   - Max 200 characters
   - Shows character count and validation status

2. **Parent Task**
   - Select from tree of existing tasks
   - "None (top-level task)" for root tasks
   - Hierarchical display with indentation

3. **Labels**
   - Add labels with autocomplete
   - Shows existing labels with usage counts
   - Enter to add, Backspace to remove

4. **Status**
   - Radio buttons: Open, Done, Waived, Blocked
   - Default: Open

5. **Owner & Estimate**
   - Owner: Optional assignee
   - Estimate: Time format (e.g., `2h`, `1d`)

6. **Agent Note**
   - Multi-line text for AI agent instructions
   - Optional

### Keyboard Navigation

| Key | Action |
|-----|--------|
| `Tab` | Move to next field |
| `Shift+Tab` | Move to previous field |
| `Ctrl+S` or `Ctrl+Enter` | Submit form |
| `Esc` or `Ctrl+C` | Close modal |
| `Ctrl+U` | Clear current field |
| `F1` or `?` | Show help |

#### In Text Fields

| Key | Action |
|-----|--------|
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Home` or `Ctrl+A` | Go to start of line |
| `End` or `Ctrl+E` | Go to end of line |

#### In Dropdowns/Selectors

| Key | Action |
|-----|--------|
| `Up/Down` | Navigate options |
| `Enter` | Select item |

### Autocomplete Features

The TUI provides intelligent autocomplete for:

- **Labels**: Shows existing labels with usage counts
- **Owners**: Suggests owners from other tasks
- **Parent Tasks**: Tree view with search filtering
- **Dependencies**: Multi-select with file path + task title

Autocomplete highlights matching characters and limits suggestions to 15 items for performance.

## Validation Rules

Both CLI and TUI enforce these rules:

### Title

- Cannot be empty or whitespace-only
- Maximum 200 characters
- Must be meaningful text

### Task ID

- Must be unique within the file
- Only alphanumeric characters, hyphens, underscores, and colons
- Auto-generated from title if not specified

### Labels

- Alphanumeric with hyphens only
- No spaces or special characters
- Case-sensitive

### Estimate

Valid formats:
- Minutes: `30m`, `45m`
- Hours: `2h`, `1.5h`
- Days: `1d`, `3d`
- Weeks: `1w`, `2w`
- Combined: `2d 4h`

### Parent Task

- Must exist in the target file
- Cannot exceed maximum nesting depth (default: 3)
- Cannot create circular parent references

### Dependencies

- Must reference existing tasks
- Format: `path/to/file.md#task:id`
- Cannot create circular dependencies
- Cannot depend on self or descendants

## Error Handling

### Common Errors

| Error Code | Description | Solution |
|------------|-------------|----------|
| `E_CREATE_EMPTY_TITLE` | Title is empty | Provide a non-empty title |
| `E_CREATE_TITLE_TOO_LONG` | Title exceeds limit | Shorten to 200 characters |
| `E_CREATE_FILE_NOT_FOUND` | File doesn't exist | Create file first or use `--file` to create automatically |
| `E_CREATE_PARENT_NOT_FOUND` | Parent task not found | Verify parent task exists |
| `E_CREATE_DEPTH_LIMIT_EXCEEDED` | Too deeply nested | Choose a shallower parent |
| `E_CREATE_DUPLICATE_ID` | ID already used | Choose different ID or omit for auto-generation |
| `E_CREATE_INVALID_ESTIMATE` | Bad time format | Use format like `2h`, `1d`, `1w` |
| `E_CREATE_WOULD_CREATE_CYCLE` | Circular dependency | Remove cyclic dependency |

Run `lash explain <ERROR_CODE>` for detailed help on any error.

### JSON Error Output

When using `--format json`, errors return:

```json
{
  "success": false,
  "errors": [
    {
      "code": "E_CREATE_EMPTY_TITLE",
      "message": "task title cannot be empty",
      "help": "provide a non-empty title for the task"
    }
  ]
}
```

## Workflow Examples

### Daily Task Creation

```bash
# Quick task for today
lash add "Review PR #123" --label review

# Task with deadline context
lash add "Prepare demo for Friday" --estimate 4h --owner self
```

### Sprint Planning

```bash
# Create sprint file
lash add "Sprint 12 Goals" \
  --file sprints/sprint-12.md \
  --file-title "Sprint 12" \
  --file-description "Two-week sprint starting 2024-01-15"

# Add user stories
lash add "User can reset password" \
  --file sprints/sprint-12.md \
  --id story-password-reset \
  --label user-story \
  --estimate 3d

# Break down into tasks
lash add "Add forgot password form" \
  --file sprints/sprint-12.md \
  --parent story-password-reset \
  --label frontend

lash add "Create password reset API" \
  --file sprints/sprint-12.md \
  --parent story-password-reset \
  --label backend
```

### Bug Tracking

```bash
# Report a bug
lash add "Login fails with special characters in password" \
  --file bugs.md \
  --label bug \
  --label auth \
  --agent-note "Check password encoding in auth.rs"

# Critical bug with urgency
lash add "Production database timeout" \
  --file bugs.md \
  --label bug \
  --label critical \
  --label production \
  --owner oncall
```

### Agent Integration

AI agents can create tasks programmatically:

```bash
# Agent creates task with full metadata
lash add "Refactor database connection pooling" \
  --file tasks/tech-debt.md \
  --label refactor \
  --label database \
  --estimate 1d \
  --agent-note "Consider using bb8 or deadpool crate" \
  --format json
```

The JSON output allows agents to:
1. Verify task creation succeeded
2. Get the assigned task ID
3. Know the file location and line number
4. Chain with other operations

## Best Practices

1. **Use descriptive titles**: "Fix login bug" is better than "Fix bug"
2. **Apply relevant labels**: Helps with filtering and organization
3. **Set realistic estimates**: Use when planning sprints or tracking time
4. **Keep hierarchy shallow**: Prefer 2-3 levels of nesting
5. **Use agent notes**: Document context that's useful for AI assistance
6. **Validate with lint**: Run `lash lint` after manual edits
7. **Update index**: Run `lash index` after creating many tasks

## Integration with Other Commands

After creating tasks:

```bash
# List new tasks
lash list --status open

# Search for your task
lash search "authentication"

# Show task details
lash show auth.md#task:auth-system

# View in TUI
lash tui
```
