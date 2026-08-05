//! Shared machinery for CLI commands that flip a task's status checkbox in
//! the source Markdown (`lash complete` and `lash waive`).
//!
//! Both commands do the same handful of things once a task has been
//! resolved: rewrite the checkbox character in place, optionally cascade
//! the change to unchecked plain-bullet children (or warn about them), look
//! up fuzzy "did you mean" suggestions when a task id doesn't resolve, and
//! re-index the project so the `SQLite` acceleration layer stays in sync
//! without requiring a separate `lash index` step. Centralizing that logic
//! here keeps the two commands from drifting out of sync with each other.

use anyhow::{Context, Result};
use lash_core::fuzzy::FuzzyMatcher;
use lash_db::{open_database, Indexer, IndexerConfig};
use lash_types::config::LashConfig;
use lash_types::TaskStatus;
use regex::Regex;
use std::fs;
use std::path::Path;

/// Checkbox character used in Markdown for each task status.
#[must_use]
pub fn status_checkbox_char(status: TaskStatus) -> char {
    match status {
        TaskStatus::Open => ' ',
        TaskStatus::InProgress => '>',
        TaskStatus::Done => 'x',
        TaskStatus::Waived => '-',
        TaskStatus::Blocked => '!',
    }
}

/// Outcome of updating a task line, including cascade information.
#[derive(Debug, Default, Clone)]
pub struct CascadeOutcome {
    /// Number of plain-bullet children that were also flipped to the new
    /// status.
    pub cascaded: usize,
    /// Plain-bullet children that remained unchecked (truncated to a
    /// reasonable size for display).
    pub unchecked: Vec<String>,
}

/// Statuses that participate in the plain-bullet cascade: flipping a parent
/// to one of these terminal states can optionally cascade to unchecked
/// plain-bullet children (`--cascade`), or warn that they were left behind
/// when it isn't passed. Non-terminal transitions (e.g. `start`, which
/// moves a task to `InProgress`) never cascade.
fn cascades_from(status: TaskStatus) -> bool {
    matches!(status, TaskStatus::Done | TaskStatus::Waived)
}

/// Update task status in the markdown file.
///
/// Finds the task line by matching the title and old status, then updates
/// the checkbox character to reflect the new status. When `cascade` is set
/// and `new_status` is a terminal state (`Done` or `Waived`), also flips
/// unchecked plain-bullet children (children without their own `@id`) of
/// the parent task to that same status. Children with an `@id` are
/// independent tasks and are never touched here. When `cascade` is not
/// set, unchecked plain-bullet children are reported via
/// [`CascadeOutcome::unchecked`] instead of being modified.
///
/// # Errors
///
/// Returns an error if the file cannot be read or written, or if the task
/// line cannot be found.
pub fn update_markdown_task_status(
    project_root: &Path,
    file_path: &Path,
    task_title: &str,
    old_status: TaskStatus,
    new_status: TaskStatus,
    cascade: bool,
) -> Result<CascadeOutcome> {
    // Construct full path
    let full_path = project_root.join(file_path);

    // Read file content
    let content = fs::read_to_string(&full_path)
        .with_context(|| format!("Failed to read file: {}", full_path.display()))?;

    // Build pattern to find the task line
    // Task lines look like: "- [ ] Task title" with optional leading whitespace
    let old_char = status_checkbox_char(old_status);
    let new_char = status_checkbox_char(new_status);

    // Escape special regex characters in the title
    let escaped_title = regex::escape(task_title);

    // Pattern: whitespace, dash, space, checkbox with old status, space, title
    // Handle both uppercase and lowercase 'x' for Done status
    let pattern = if old_status == TaskStatus::Done {
        format!(r"^(\s*- \[)[xX](\] {escaped_title})")
    } else {
        format!(r"^(\s*- \[){old_char}(\] {escaped_title})")
    };

    let re = Regex::new(&pattern).context("Failed to compile regex")?;

    let lines: Vec<String> = content.lines().map(str::to_string).collect();
    let mut updated_lines = lines.clone();
    let mut parent_idx: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        if re.is_match(line) {
            updated_lines[i] = re
                .replace(line, format!("${{1}}{new_char}${{2}}"))
                .to_string();
            parent_idx = Some(i);
            break;
        }
    }

    let Some(parent_idx) = parent_idx else {
        anyhow::bail!("Could not find task '{task_title}' in file");
    };

    // Cascade only applies when we are flipping a parent to a terminal
    // status (Done for `complete`, Waived for `waive`).
    let parent_indent = leading_space_count(&lines[parent_idx]);
    let mut cascaded = 0usize;
    let mut unchecked = Vec::new();

    if cascades_from(new_status) {
        let plain_children = find_plain_child_lines(&lines, parent_idx, parent_indent);
        for child_idx in plain_children {
            let child_line = &lines[child_idx];
            if cascade {
                if let Some(new_line) = flip_open_child(child_line, new_status) {
                    updated_lines[child_idx] = new_line;
                    cascaded += 1;
                }
            } else if child_line.trim_start().strip_prefix("- [ ]").is_some() {
                unchecked.push(child_line.trim().to_string());
            }
        }
    }

    let updated_content = updated_lines.join("\n");
    let final_content = if content.ends_with('\n') && !updated_content.ends_with('\n') {
        format!("{updated_content}\n")
    } else {
        updated_content
    };

    fs::write(&full_path, final_content)
        .with_context(|| format!("Failed to write file: {}", full_path.display()))?;

    Ok(CascadeOutcome {
        cascaded,
        unchecked,
    })
}

/// Count the leading whitespace characters in a line.
pub fn leading_space_count(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

/// Collect immediate plain-bullet child line indexes under the parent.
///
/// A "plain-bullet child" is a `- [ ]` (or `- [!]`, `- [>]`) checkbox line
/// indented more deeply than the parent, that does NOT carry its own
/// `@id:` annotation in the indented block following it. The walk stops at
/// the first line whose indent drops back to (or below) the parent's
/// indent, since that ends the parent's scope.
pub fn find_plain_child_lines(
    lines: &[String],
    parent_idx: usize,
    parent_indent: usize,
) -> Vec<usize> {
    let mut children = Vec::new();
    let mut i = parent_idx + 1;
    while i < lines.len() {
        let line = &lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        let indent = leading_space_count(line);
        if indent <= parent_indent {
            // Out of the parent's scope.
            break;
        }
        if trimmed.starts_with("- [") {
            // Decide whether this child has its own @id by peeking at the
            // following indented annotation lines (deeper than this child).
            let child_indent = indent;
            if !child_has_id_annotation(lines, i, child_indent) {
                children.push(i);
            }
        }
        i += 1;
    }
    children
}

/// Walk lines following a checkbox child to see if it has `@id:` in its
/// own annotation block (lines indented more deeply than the child line).
fn child_has_id_annotation(lines: &[String], child_idx: usize, child_indent: usize) -> bool {
    let mut j = child_idx + 1;
    while j < lines.len() {
        let line = &lines[j];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            j += 1;
            continue;
        }
        let indent = leading_space_count(line);
        // Annotation lines belong to the child only if they are *more*
        // indented than the child itself and are not a new checkbox.
        if indent <= child_indent {
            break;
        }
        if trimmed.starts_with("- [") {
            // A nested checkbox under the child — annotations would have
            // come before this, so we can stop.
            break;
        }
        if trimmed.starts_with("@id:") {
            return true;
        }
        j += 1;
    }
    false
}

/// Flip a `- [ ]` (or `[!]`, `[>]`) checkbox line to the checkbox for
/// `new_status`. Returns the new line, or `None` if the line isn't a
/// flippable open-ish checkbox.
fn flip_open_child(line: &str, new_status: TaskStatus) -> Option<String> {
    let new_char = status_checkbox_char(new_status);
    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let rest = &line[indent.len()..];
    for prefix in ["- [ ]", "- [!]", "- [>]"] {
        if let Some(remainder) = rest.strip_prefix(prefix) {
            return Some(format!("{indent}- [{new_char}]{remainder}"));
        }
    }
    None
}

/// Scan a markdown file for plain-bullet children of the given parent task
/// without modifying the file. Returns the unchecked child labels so the
/// `--dry-run` path can warn about them.
///
/// `old_status` is the parent task's current status (as read from the
/// index), used to match its checkbox character in the source file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or the parent regex fails
/// to compile.
pub fn preview_cascade_children(
    project_root: &Path,
    file_path: &Path,
    task_title: &str,
    old_status: TaskStatus,
) -> Result<Vec<String>> {
    let full_path = project_root.join(file_path);
    let content = fs::read_to_string(&full_path)
        .with_context(|| format!("Failed to read file: {}", full_path.display()))?;
    let old_char = status_checkbox_char(old_status);
    let escaped = regex::escape(task_title);
    let parent_re = Regex::new(&format!(r"^(\s*)- \[{old_char}\] {escaped}\b"))
        .context("Failed to compile parent regex")?;
    let mut unchecked = Vec::new();
    let line_vec: Vec<&str> = content.lines().collect();
    for (idx, line) in line_vec.iter().enumerate() {
        if let Some(caps) = parent_re.captures(line) {
            let parent_indent = caps[1].len();
            collect_unchecked_plain_children(&line_vec, idx, parent_indent, &mut unchecked);
            break;
        }
    }
    Ok(unchecked)
}

/// Same scan as the in-place cascade, but read-only — used by `--dry-run`.
fn collect_unchecked_plain_children(
    lines: &[&str],
    parent_idx: usize,
    parent_indent: usize,
    out: &mut Vec<String>,
) {
    let owned: Vec<String> = lines.iter().map(|s| (*s).to_string()).collect();
    for idx in find_plain_child_lines(&owned, parent_idx, parent_indent) {
        let line = &owned[idx];
        if line.trim_start().starts_with("- [ ]") {
            out.push(line.trim().to_string());
        }
    }
}

/// Find similar task IDs using fuzzy matching, for "did you mean"
/// suggestions when a target task id doesn't resolve.
#[must_use]
pub fn find_similar_task_ids(query: &str, candidates: &[String]) -> Vec<(String, f64)> {
    let fuzzy_matcher = FuzzyMatcher::new(0.5, 5);
    let results = fuzzy_matcher.find_matches(query, candidates);
    results.into_iter().map(|c| (c.task_id, c.score)).collect()
}

/// Re-index the project after a markdown edit.
///
/// `context_label` is folded into the error message on failure (e.g.
/// `"task completion"`, `"waiving task"`) so callers get a command-specific
/// message without duplicating this function.
///
/// # Errors
///
/// Returns an error if the database cannot be opened or the re-index pass
/// fails.
pub fn reindex_project(project_root: &Path, context_label: &str) -> Result<()> {
    let db_path = project_root.join(".lash").join("lash.db");
    let conn = open_database(&db_path).context("Failed to open database for re-indexing")?;

    let config = LashConfig::from_root(project_root).unwrap_or_default();
    let indexer_config = IndexerConfig::new(project_root.to_path_buf()).with_incremental(true);
    let mut indexer = Indexer::new(&conn, indexer_config, &config);

    indexer
        .index_project()
        .with_context(|| format!("Failed to re-index after {context_label}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_checkbox_char() {
        assert_eq!(status_checkbox_char(TaskStatus::Open), ' ');
        assert_eq!(status_checkbox_char(TaskStatus::InProgress), '>');
        assert_eq!(status_checkbox_char(TaskStatus::Done), 'x');
        assert_eq!(status_checkbox_char(TaskStatus::Waived), '-');
        assert_eq!(status_checkbox_char(TaskStatus::Blocked), '!');
    }

    #[test]
    fn test_flip_open_child_to_done() {
        assert_eq!(
            flip_open_child("  - [ ] Sub task", TaskStatus::Done),
            Some("  - [x] Sub task".to_string())
        );
        assert_eq!(
            flip_open_child("    - [!] Blocked", TaskStatus::Done),
            Some("    - [x] Blocked".to_string())
        );
        assert_eq!(
            flip_open_child("    - [>] In progress", TaskStatus::Done),
            Some("    - [x] In progress".to_string())
        );
        // Already done — not flippable
        assert_eq!(flip_open_child("- [x] Done", TaskStatus::Done), None);
        // Not a checkbox
        assert_eq!(flip_open_child("regular line", TaskStatus::Done), None);
    }

    #[test]
    fn test_flip_open_child_to_waived() {
        assert_eq!(
            flip_open_child("  - [ ] Sub task", TaskStatus::Waived),
            Some("  - [-] Sub task".to_string())
        );
    }

    #[test]
    fn test_find_plain_child_lines_skips_id_tagged_children() {
        // Two children: one with @id (independent task, should be skipped),
        // one plain-bullet (should be returned).
        let lines: Vec<String> = vec![
            "- [ ] Parent",
            "  @id: parent-1",
            "  - [ ] Tracked child",
            "    @id: tracked-child",
            "  - [ ] Plain child",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        let plain = find_plain_child_lines(&lines, 0, 0);
        assert_eq!(plain, vec![4]);
    }

    #[test]
    fn test_find_plain_child_lines_stops_at_dedent() {
        // The loop must stop when indent returns to parent level.
        let lines: Vec<String> = vec![
            "- [ ] Parent",
            "  - [ ] Plain child 1",
            "- [ ] Sibling parent",
            "  - [ ] Sibling's child",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        let plain = find_plain_child_lines(&lines, 0, 0);
        // Only the first parent's plain child should be returned.
        assert_eq!(plain, vec![1]);
    }

    #[test]
    fn test_update_markdown_cascade_flips_plain_children_to_done() {
        // End-to-end exercise of cascade=true on a parent with mixed children.
        let temp = tempfile::TempDir::new().unwrap();
        let path = std::path::PathBuf::from("tasks.md");
        let full = temp.path().join(&path);
        let content = "# Tasks\n\
                       \n\
                       ## Tasks\n\
                       \n\
                       - [ ] Parent task\n  \
                       @id: parent\n  \
                       - [ ] Plain step one\n  \
                       - [ ] Plain step two\n  \
                       - [ ] Tracked child\n    \
                       @id: tracked\n";
        std::fs::write(&full, content).unwrap();

        let outcome = update_markdown_task_status(
            temp.path(),
            &path,
            "Parent task",
            TaskStatus::Open,
            TaskStatus::Done,
            true, // cascade
        )
        .unwrap();

        let updated = std::fs::read_to_string(&full).unwrap();
        assert!(updated.contains("- [x] Parent task"));
        assert!(updated.contains("- [x] Plain step one"));
        assert!(updated.contains("- [x] Plain step two"));
        // Tracked child must NOT be flipped — it has its own @id.
        assert!(updated.contains("- [ ] Tracked child"));

        assert_eq!(outcome.cascaded, 2);
        assert!(outcome.unchecked.is_empty());
    }

    #[test]
    fn test_update_markdown_cascade_flips_plain_children_to_waived() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = std::path::PathBuf::from("tasks.md");
        let full = temp.path().join(&path);
        let content = "# Tasks\n\
                       \n\
                       ## Tasks\n\
                       \n\
                       - [ ] Parent task\n  \
                       @id: parent\n  \
                       - [ ] Plain step one\n";
        std::fs::write(&full, content).unwrap();

        let outcome = update_markdown_task_status(
            temp.path(),
            &path,
            "Parent task",
            TaskStatus::Open,
            TaskStatus::Waived,
            true, // cascade
        )
        .unwrap();

        let updated = std::fs::read_to_string(&full).unwrap();
        assert!(updated.contains("- [-] Parent task"));
        assert!(updated.contains("- [-] Plain step one"));
        assert_eq!(outcome.cascaded, 1);
    }

    #[test]
    fn test_update_markdown_warns_when_cascade_disabled() {
        // Without --cascade we leave the children alone but report them.
        let temp = tempfile::TempDir::new().unwrap();
        let path = std::path::PathBuf::from("tasks.md");
        let full = temp.path().join(&path);
        let content = "# Tasks\n\
                       \n\
                       ## Tasks\n\
                       \n\
                       - [ ] Parent task\n  \
                       @id: parent\n  \
                       - [ ] Plain step one\n  \
                       - [ ] Plain step two\n";
        std::fs::write(&full, content).unwrap();

        let outcome = update_markdown_task_status(
            temp.path(),
            &path,
            "Parent task",
            TaskStatus::Open,
            TaskStatus::Done,
            false, // cascade off
        )
        .unwrap();

        let updated = std::fs::read_to_string(&full).unwrap();
        assert!(updated.contains("- [x] Parent task"));
        assert!(updated.contains("- [ ] Plain step one"));
        assert!(updated.contains("- [ ] Plain step two"));

        assert_eq!(outcome.cascaded, 0);
        assert_eq!(outcome.unchecked.len(), 2);
    }

    #[test]
    fn test_no_cascade_for_non_terminal_status() {
        // `start` (Open -> InProgress) must never scan for/flip children,
        // even if --cascade-shaped logic were accidentally reused.
        let temp = tempfile::TempDir::new().unwrap();
        let path = std::path::PathBuf::from("tasks.md");
        let full = temp.path().join(&path);
        let content = "- [ ] Parent task\n  - [ ] Plain child\n";
        std::fs::write(&full, content).unwrap();

        let outcome = update_markdown_task_status(
            temp.path(),
            &path,
            "Parent task",
            TaskStatus::Open,
            TaskStatus::InProgress,
            true,
        )
        .unwrap();

        assert_eq!(outcome.cascaded, 0);
        assert!(outcome.unchecked.is_empty());
        let updated = std::fs::read_to_string(&full).unwrap();
        assert!(updated.contains("- [>] Parent task"));
        assert!(updated.contains("- [ ] Plain child")); // untouched
    }

    #[test]
    fn test_preview_cascade_children_matches_current_status() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = std::path::PathBuf::from("tasks.md");
        let full = temp.path().join(&path);
        // Parent is Blocked ([!]), not Open — the preview must match on
        // the task's actual current status, not assume Open.
        let content = "- [!] Parent task\n  - [ ] Plain child\n";
        std::fs::write(&full, content).unwrap();

        let unchecked =
            preview_cascade_children(temp.path(), &path, "Parent task", TaskStatus::Blocked)
                .unwrap();
        assert_eq!(unchecked, vec!["- [ ] Plain child".to_string()]);
    }
}
