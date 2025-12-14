# Documentation Style Guide

This guide establishes conventions for Lash documentation to ensure consistency and clarity.

## General Principles

1. **Be concise** - Use clear, direct language. Avoid filler words.
2. **Be accurate** - Test all examples. Keep documentation in sync with code.
3. **Be accessible** - Write for both new users and experienced developers.
4. **Be consistent** - Follow established patterns throughout.

## File Structure

### Markdown Files

```markdown
# Title (H1)

Brief introduction paragraph explaining the document's purpose.

## Table of Contents (optional for long docs)

## Main Section (H2)

Content goes here.

### Subsection (H3)

More specific content.

#### Details (H4) - use sparingly
```

### Naming Conventions

- Use lowercase with hyphens: `user-guide.md`, `error-codes.md`
- README files: `README.md` (uppercase)
- Task files: descriptive names with `.md` extension

## Writing Style

### Voice and Tone

- Use active voice: "Run the command" not "The command should be run"
- Be direct: "Use `lash lint`" not "You can use `lash lint`"
- Avoid jargon when simpler terms work
- Be consistent with terminology

### Formatting

**Code and Commands:**
- Inline code for short references: `lash lint`
- Code blocks for multi-line examples:
  ```bash
  lash index
  lash list --label backend
  ```
- Specify language for syntax highlighting

**Emphasis:**
- **Bold** for important terms on first use
- *Italics* sparingly for emphasis
- Avoid ALL CAPS

**Lists:**
- Use bullet points for unordered items
- Use numbered lists for sequential steps
- Keep parallel structure in list items

### Command Examples

Always show realistic, working examples:

```bash
# Good: Shows actual usage
lash list --label backend --status open

# Bad: Too abstract
lash list [OPTIONS]
```

Include expected output when helpful:

```bash
$ lash list --label backend
backend/api.md:
  - [ ] Implement authentication endpoint
  - [ ] Add rate limiting
```

### Error Messages

When documenting errors:
1. Show the error code: `E_LINT_DUPLICATE_ID`
2. Explain what causes it
3. Provide a fix

```markdown
### E_LINT_DUPLICATE_ID

**Cause:** Multiple files use the same `@id` value.

**Fix:** Ensure each file has a unique `@id`:
```markdown
@id: unique.file.id
```
```

## Code Examples

### Markdown Task Files

Show complete, valid examples:

```markdown
# Feature Name

@id: feature.name
@labels: backend, api
@status: in-progress
@created: 2024-01-15

## Description

Brief description of the feature's purpose.

## Tasks

- [ ] First task
  - [ ] Subtask
- [ ] Second task
```

### Rust Code

For API documentation:
- Include `use` statements
- Show complete, compilable examples
- Add comments for clarity

```rust
use lash_core::parser::parse_file;
use std::path::Path;

// Parse a task file
let file = parse_file(Path::new("tasks.md"))?;
println!("Found {} tasks", file.tasks.len());
```

## Section Templates

### Command Documentation

```markdown
## `lash <command>`

Brief description of what the command does.

### Usage

```bash
lash <command> [OPTIONS] [ARGS]
```

### Options

| Option | Description |
|--------|-------------|
| `--flag` | What it does |
| `-f, --file <PATH>` | Description |

### Examples

```bash
# Basic usage
lash <command>

# With options
lash <command> --flag value
```

### See Also

- Related command
- Relevant section
```

### Tutorial Section

```markdown
## Tutorial: <Topic>

### Overview

What you'll learn in this tutorial.

### Prerequisites

- What you need before starting

### Steps

1. **Step One Title**

   Explanation of the step.

   ```bash
   command to run
   ```

2. **Step Two Title**

   Continue with instructions.

### Summary

What was accomplished and next steps.
```

## Links and References

### Internal Links

Use relative paths:
```markdown
See [User Guide](./user-guide.md)
See [Error Codes](./error-codes.md#e_lint_duplicate_id)
```

### External Links

Use full URLs:
```markdown
[CommonMark Specification](https://commonmark.org/)
```

## Documentation Review Checklist

Before submitting documentation:

- [ ] All examples compile/run successfully
- [ ] Links work (internal and external)
- [ ] Formatting is consistent
- [ ] Technical accuracy verified
- [ ] No spelling or grammar errors
- [ ] Code snippets use appropriate syntax highlighting
- [ ] Commands show expected output where helpful

## Maintenance

### Keeping Docs Current

- Update docs when features change
- Run `lash lint` on example task files
- Test code examples after updates
- Review quarterly for accuracy

### Deprecation

When deprecating features:
1. Mark with "**Deprecated:**" notice
2. Explain migration path
3. Remove after one major version
