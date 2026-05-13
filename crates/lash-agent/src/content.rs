//! Static content primitives for Lash agent documentation.
//!
//! These functions return self-contained Markdown sections describing
//! Lash's CLI surface, file format, workflows, and safety constraints.
//! Both `lash agent-prompt` (dynamic stdout output) and `lash skill install`
//! (filesystem installation, future) compose from these primitives so the
//! authoritative source for "how to use Lash" lives in one place.
//!
//! # Examples
//!
//! ```
//! use lash_agent::content;
//!
//! let cli_ref = content::cli_reference();
//! assert!(cli_ref.contains("lash lint"));
//! ```

/// Brief overview of Lash for agents.
///
/// Suitable as the lead-in section of a prompt or skill file.
#[must_use]
pub fn overview() -> &'static str {
    "## Overview

Lash is a minimalist, Markdown-native task tracker where:
- Markdown files are the single source of truth
- Tasks are hierarchical checkbox lists with annotations
- Directory structure provides implicit hierarchy (parent directories depend on children)
- SQLite provides fast indexing and search (fully reconstructible from Markdown)
- Format is strictly enforced by linting for predictability

"
}

/// Project layout conventions.
#[must_use]
pub fn project_structure() -> &'static str {
    "## Project Structure

Lash projects follow these conventions:

- **Index file**: `tasks/tasks.md` or `lash.index.md` at project root
- **Task files**: Usually under `tasks/` directory
- **Database**: `.lash/lash.db` (auto-generated, gitignore this)
- **Config**: `.lash/config.toml` or `~/.lash/config.toml`

To find the project structure:
```bash
lash list --tree    # Shows all task files and hierarchy
```

"
}

/// Recommended discover → read → modify → validate → index workflow.
#[must_use]
pub fn workflow() -> &'static str {
    r#"## Recommended Workflow

When working with Lash task files, follow this workflow for consistent results:

1. **Discover** - Understand the project structure first
   ```bash
   lash status                   # Get quick overview of current work
   lash list --tree              # See task hierarchy
   lash search "relevant term"   # Find related tasks
   ```

2. **Read** - Open and understand the file before editing
   - Check existing task structure and annotations
   - Note the file's `@id`, `@labels`, and dependencies
   - Review contextual notes for requirements

3. **Modify** - Make your changes following the format specification
   - Add tasks with proper checkbox syntax: `- [ ] Task description`
   - Use 2-space indentation for subtasks
   - Add contextual notes (plain bullets) for requirements

4. **Validate** - Always lint after editing
   ```bash
   lash lint path/to/file.md    # Validate immediately after changes
   ```

5. **Index** - Update the database
   ```bash
   lash index                    # Rebuild index to reflect changes
   ```

"#
}

/// Full CLI command reference, grouped by category.
#[must_use]
pub fn cli_reference() -> &'static str {
    "## CLI Quick Reference

```bash
# Project Setup
lash init                      # Initialize a new Lash project (creates lash.index.md)

# Discovery & Navigation
lash status                    # Show project status summary (in-progress, blocked, recent)
lash status --compact          # Minimal output for agents
lash list                      # List all tasks
lash list --tree               # Show task hierarchy
lash list --label backend      # Filter by label
lash list --status open        # Filter by status (open, done, blocked, waived)
lash search <QUERY>             # Full-text search tasks and descriptions
lash show <ID>                  # Show task/file details with dependencies

# Task Modification
lash add <DESCRIPTION>          # Add a new task (--file, --label, --depends-on)
lash start <ID>                 # Mark a task as in-progress
lash complete <ID>              # Mark task as done (--cascade for child tasks)

# Validation & Formatting
lash lint [PATH...]             # Validate task files (run after every edit!)
lash lint --fix                 # Auto-fix some lint errors
lash format [PATH...]           # Normalize formatting

# Indexing
lash index                      # Update SQLite index after changes
lash check-index                # Verify database consistency

# Dependencies & Links
lash graph                      # Show dependency graph (ascii)
lash graph --format dot         # Export as DOT/Graphviz
lash check-links                # Find broken references
lash check-links --fix          # Auto-fix broken references

# Agent Integration & Config
lash agent-prompt               # Generate context-minimized prompt for current project
lash skill install --target T   # Install Lash skill into a coding agent (claude/codex/cursor/agents-md)
lash config <SUBCOMMAND>        # Manage configuration (get, set, list, path)

# Error Help
lash explain <CODE>             # Explain error code (e.g., E001)
lash explain --list             # List all error codes
```

"
}

/// Safety rules: always lint, respect depth limits, keep IDs unique, etc.
#[must_use]
pub fn safety_guidelines() -> &'static str {
    "## Safety Guidelines

When working with Lash files:

1. **Always run `lash lint` after modifications** to validate your changes
2. **Respect depth limits** (3-4 levels maximum for task hierarchies)
3. **Don't break dependency references** - ensure `@depends-on` targets exist
4. **Maintain status consistency** - parent tasks complete only when children are done/waived
5. **Use unique IDs** within each file
6. **Run `lash index`** after making changes to update the search index
7. **Keep `@doc` references valid** - ensure referenced documentation files exist

### `@doc:` Fragment Slugs

When a `@doc:` annotation includes a `#fragment`, Lash matches it against the
target document's headings using case- and punctuation-insensitive
normalization: lowercase the text, treat `-` as a word separator, then drop
every character that is not alphanumeric or whitespace (`<`, `>`, `/`, `.`,
`_`, `(`, `)`, backticks, etc. are stripped *without* introducing a hyphen).
For example, the heading ``Pack manifest (`<pack>/SKILL.md`)`` matches the
fragment `pack-manifest-packskillmd`. Run
`lash explain W_SEM_DOC_FRAGMENT` for full details.

"
}

/// Common error codes with recovery instructions.
#[must_use]
pub fn error_recovery() -> &'static str {
    "## Common Issues & Recovery

### Lint Errors

| Code | Issue | Fix |
|------|-------|-----|
| E001 | Duplicate task ID | Ensure `@id` values are unique within each file |
| E002 | Invalid dependency | Verify `@depends-on` target file and task exist |
| E003 | Max depth exceeded | Flatten hierarchy (max 3-4 levels) |
| E004 | Invalid status | Use valid checkbox: `[ ]`, `[x]`, `[-]`, `[!]` |

Run `lash explain <CODE>` for detailed explanations.

### Index Out of Sync

If search results seem stale or incorrect:
```bash
lash index --force    # Force full reindex
lash check-index      # Verify consistency
```

### Broken References

If `@depends-on` or `@doc` references are broken:
```bash
lash check-links              # Find broken references
lash check-links --fix        # Attempt auto-fix
```

"
}

/// One-line description for use in skill frontmatter or trigger phrases.
///
/// Phrased so an agent reading it can decide whether Lash is relevant to the
/// current task. Kept short (under ~280 characters) so it survives any
/// truncation the host imposes on skill index summaries.
#[must_use]
pub fn when_to_use() -> &'static str {
    "Use Lash for task management — a fast Markdown-native task tracker with hierarchical checkboxes, labels, and dependencies. Trigger when the project has tasks/ or .lash/ directories, or when the user asks to create, list, find, complete, or update tasks."
}

/// Top hot commands for inclusion in SKILL.md / one-page references.
///
/// These cover roughly the 90% case: discovery, search, modification, and
/// validation. Less common commands live in [`cli_reference`].
#[must_use]
pub fn hot_commands() -> &'static str {
    "```bash
lash status                  # quick overview of in-progress/blocked
lash list --status open      # list open tasks (--label, --tree, --path)
lash search <query>          # full-text search across tasks
lash show <id>               # task details + dependencies
lash add <description>       # create a new task
lash complete <id>           # mark task done (--cascade for children)
lash lint <path>             # validate after every edit
lash agent-prompt            # get project-specific live context
```
"
}

/// Reference content for cross-file dependencies, `@doc` links, and slugs.
///
/// Used as a standalone reference file in progressive-disclosure skill layouts.
#[must_use]
pub fn dependencies_reference() -> &'static str {
    "# Dependencies & Documentation References

Lash supports two ways of linking tasks and files together: explicit
dependencies that block completion, and documentation references that
provide context without affecting status.

## `@depends-on` — Hard Dependencies

A task or file cannot be considered complete until every task it depends on
is `done` or `waived`.

```markdown
- [ ] Build payment processor
  @id: payment.processor
  @depends-on: features/auth.md#task:auth.session
```

Forms:
- `path/to/file.md` — depend on the entire file
- `path/to/file.md#task:<id>` — depend on a specific task
- `#task:<id>` — depend on a task in the same file

Validate references with `lash check-links`. Auto-fix typos in references
with `lash check-links --fix`.

## `@doc:` — Documentation References

Soft, informational links to documentation. They never affect task status,
but the linter validates the target exists and the optional fragment
matches a heading.

```markdown
- [ ] Implement OAuth flow
  @id: auth.oauth
  @doc: ../docs/auth-spec.md#oauth-flow
```

### Fragment Slug Matching

Fragment slugs are matched against headings with case- and
punctuation-insensitive normalization: lowercase, treat `-` as a word
separator, drop everything that isn't alphanumeric or whitespace (without
introducing a hyphen for the dropped chars).

Example: the heading ``Pack manifest (`<pack>/SKILL.md`)`` matches the
fragment `pack-manifest-packskillmd`. Run `lash explain
W_SEM_DOC_FRAGMENT` for the canonical algorithm.

## Implicit Hierarchy

Parent directories implicitly depend on the task files inside them. A
folder of features is not complete until every file in the folder is
done — no annotation needed.

## Cycle Detection

`lash index` and `lash graph` will reject circular dependencies with a
clear error (`E_DEP_CYCLE`). If you hit a cycle, restructure the
dependency direction or split a task.
"
}

/// Canonical list of top-level Lash subcommands.
///
/// Drift-guard tests assert this matches the clap-defined CLI surface, so when
/// a new subcommand is added the test fails until this list and the relevant
/// content sections are updated.
pub const TOP_LEVEL_SUBCOMMANDS: &[&str] = &[
    "add",
    "agent-prompt",
    "check-index",
    "check-links",
    "complete",
    "completion",
    "config",
    "explain",
    "format",
    "graph",
    "index",
    "init",
    "lint",
    "list",
    "playground",
    "search",
    "show",
    "skill",
    "start",
    "status",
    "tui",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_mentions_markdown_source_of_truth() {
        assert!(overview().contains("Markdown"));
        assert!(overview().contains("SQLite"));
    }

    #[test]
    fn project_structure_lists_canonical_paths() {
        let text = project_structure();
        assert!(text.contains("tasks/tasks.md"));
        assert!(text.contains(".lash/lash.db"));
    }

    #[test]
    fn workflow_covers_five_phases() {
        let text = workflow();
        assert!(text.contains("Discover"));
        assert!(text.contains("Read"));
        assert!(text.contains("Modify"));
        assert!(text.contains("Validate"));
        assert!(text.contains("Index"));
    }

    #[test]
    fn cli_reference_lists_core_commands() {
        let text = cli_reference();
        for cmd in &[
            "lash status",
            "lash list",
            "lash lint",
            "lash index",
            "lash search",
            "lash show",
        ] {
            assert!(text.contains(cmd), "cli_reference missing: {cmd}");
        }
    }

    #[test]
    fn safety_guidelines_cover_lint_and_depth() {
        let text = safety_guidelines();
        assert!(text.contains("lash lint"));
        assert!(text.contains("depth"));
        assert!(text.contains("unique IDs"));
    }

    #[test]
    fn error_recovery_lists_lint_codes() {
        let text = error_recovery();
        for code in &["E001", "E002", "E003", "E004"] {
            assert!(text.contains(code), "error_recovery missing: {code}");
        }
    }

    #[test]
    fn when_to_use_is_short_enough_for_skill_frontmatter() {
        let text = when_to_use();
        assert!(
            text.len() <= 280,
            "when_to_use() too long for safe skill frontmatter use: {} chars",
            text.len()
        );
        assert!(text.contains("Lash"));
    }

    #[test]
    fn hot_commands_lists_top_workflow_commands() {
        let text = hot_commands();
        for cmd in &[
            "lash status",
            "lash list",
            "lash search",
            "lash show",
            "lash add",
            "lash complete",
            "lash lint",
            "lash agent-prompt",
        ] {
            assert!(text.contains(cmd), "hot_commands missing: {cmd}");
        }
    }

    #[test]
    fn dependencies_reference_covers_depends_on_and_doc() {
        let text = dependencies_reference();
        assert!(text.contains("@depends-on"));
        assert!(text.contains("@doc"));
        assert!(text.contains("Fragment Slug"));
    }

    #[test]
    fn subcommand_list_is_sorted_and_unique() {
        let mut sorted = TOP_LEVEL_SUBCOMMANDS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.as_slice(),
            TOP_LEVEL_SUBCOMMANDS,
            "TOP_LEVEL_SUBCOMMANDS must be sorted and unique"
        );
    }
}
