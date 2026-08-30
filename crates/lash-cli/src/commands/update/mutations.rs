//! Line-editing primitives for `lash update`.
//!
//! Mirrors the codebase's established pattern for markdown mutation:
//! `status_mutation.rs` rewrites the checkbox character in place, and
//! `waive.rs::insert_reason_note` splices a new line into a task's
//! annotation block without disturbing anything else. `update` needs a
//! richer set of the same kind of targeted edit — rewrite the title, add or
//! remove a `@key: value` annotation, add/remove one `@depends-on` entry
//! out of a comma-separated list, replace or append to a possibly
//! multi-line `@agent-note` — so those primitives live here, on a small
//! `TaskLines` type that owns the file's lines and knows which one is the
//! task's own checkbox line.
//!
//! None of this re-serializes the file through an emitter: every operation
//! is a line insert/remove/replace on the `Vec<String>` read from disk, so
//! untouched content (including its exact formatting) survives byte for
//! byte.

use anyhow::{bail, Context, Result};
use lash_types::label::normalize as normalize_label;
use std::fs;
use std::path::Path;

use crate::commands::status_mutation::leading_space_count;

/// A task file loaded as lines, with one task's checkbox line located, ready
/// for targeted edits.
pub struct TaskLines {
    lines: Vec<String>,
    /// Index (0-based) of the task's checkbox line within `lines`.
    task_idx: usize,
    /// Leading-space count of the checkbox line.
    task_indent: usize,
    /// Whether the original content ended in a trailing newline (preserved
    /// on write, matching every other mutation command in this codebase).
    trailing_newline: bool,
}

impl TaskLines {
    /// Read `full_path` and locate the task whose checkbox line is
    /// `line_number` (1-indexed, exactly what [`lash_types::Task::line_number`]
    /// reports from a fresh parse).
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, or `line_number` is out
    /// of range for the file's current line count (which would indicate the
    /// on-disk file and the caller's parsed view of it have gone out of
    /// sync between read and edit).
    pub fn load(full_path: &Path, line_number: usize) -> Result<Self> {
        let content = fs::read_to_string(full_path)
            .with_context(|| format!("Failed to read file: {}", full_path.display()))?;
        let trailing_newline = content.ends_with('\n');
        let lines: Vec<String> = content.lines().map(str::to_string).collect();
        if line_number == 0 || line_number > lines.len() {
            bail!(
                "Task line {line_number} is out of range for {} ({} lines)",
                full_path.display(),
                lines.len()
            );
        }
        let task_idx = line_number - 1;
        let task_indent = leading_space_count(&lines[task_idx]);
        Ok(Self {
            lines,
            task_idx,
            task_indent,
            trailing_newline,
        })
    }

    /// Build a `TaskLines` directly from in-memory content, for unit tests
    /// that exercise the mutation primitives without touching disk.
    #[cfg(test)]
    #[must_use]
    pub fn from_content(content: &str, line_number: usize) -> Self {
        let trailing_newline = content.ends_with('\n');
        let lines: Vec<String> = content.lines().map(str::to_string).collect();
        let task_idx = line_number
            .saturating_sub(1)
            .min(lines.len().saturating_sub(1));
        let task_indent = lines.get(task_idx).map_or(0, |l| leading_space_count(l));
        Self {
            lines,
            task_idx,
            task_indent,
            trailing_newline,
        }
    }

    /// Render the current (possibly edited) lines back into file content,
    /// preserving the original trailing-newline convention.
    #[must_use]
    pub fn render(&self) -> String {
        let mut content = self.lines.join("\n");
        if self.trailing_newline && !content.ends_with('\n') {
            content.push('\n');
        }
        content
    }

    /// Write the current lines back to `full_path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn write(&self, full_path: &Path) -> Result<()> {
        fs::write(full_path, self.render())
            .with_context(|| format!("Failed to write file: {}", full_path.display()))
    }

    /// The task's raw title text exactly as written on the checkbox line
    /// (including any trailing inline `#label` tokens).
    #[must_use]
    pub fn title_text(&self) -> &str {
        let line = &self.lines[self.task_idx];
        line.get(self.task_indent + 5..).unwrap_or("").trim_start()
    }

    /// Overwrite the title portion of the checkbox line, leaving the
    /// indentation and `- [X]` marker untouched.
    fn set_title_text(&mut self, new_title: &str) {
        let indent = " ".repeat(self.task_indent);
        let checkbox = self.lines[self.task_idx]
            .get(self.task_indent..self.task_indent + 5)
            .unwrap_or("- [ ]")
            .to_string();
        self.lines[self.task_idx] = format!("{indent}{checkbox} {new_title}");
    }

    /// Rewrite the task's title, preserving any trailing inline `#label`
    /// tokens from the old title (GitHub issue #25: retitling must not
    /// silently drop labels stored inline in the title text).
    pub fn retitle(&mut self, new_title: &str) {
        let old_title = self.title_text().to_string();
        let suffix = trailing_label_suffix(&old_title);
        let combined = if suffix.is_empty() {
            new_title.trim().to_string()
        } else {
            format!("{} {suffix}", new_title.trim())
        };
        self.set_title_text(&combined);
    }

    /// Append an inline `#label` token to the title line.
    pub fn add_inline_label(&mut self, label: &str) {
        let old_title = self.title_text().to_string();
        let new_title = format!("{} #{label}", old_title.trim_end());
        self.set_title_text(&new_title);
    }

    /// Remove an inline `#label` token from the title line, matching by
    /// normalized name. Returns `true` if a token was removed.
    pub fn remove_inline_label(&mut self, label: &str) -> bool {
        let old_title = self.title_text().to_string();
        let target = normalize_label(label);
        let mut removed = false;
        let mut kept: Vec<&str> = Vec::new();
        for word in old_title.split_whitespace() {
            if let Some(tag) = word.strip_prefix('#') {
                let clean = tag.trim_end_matches(|c: char| !c.is_alphanumeric());
                if !clean.is_empty() && normalize_label(clean) == target {
                    removed = true;
                    continue;
                }
            }
            kept.push(word);
        }
        if removed {
            self.set_title_text(&kept.join(" "));
        }
        removed
    }

    /// `[start, end)` line range of the annotation block immediately
    /// following the task's checkbox line: consecutive `@key: value` lines
    /// and their indented continuation lines. Mirrors the parser's own
    /// lookahead in `lash_core::parser::parse_task_section_internal` closely
    /// enough for editing purposes: stops at a `- ` bullet (nested checkbox
    /// or contextual note), a blank line, an `## ` heading, or a dedent back
    /// to the task's own indent or shallower.
    fn annotation_block_range(&self) -> (usize, usize) {
        let start = self.task_idx + 1;
        let mut j = start;
        let mut started = false;
        let mut seen_blank = false;
        while j < self.lines.len() {
            let line = &self.lines[j];
            let trimmed = line.trim();
            if trimmed.is_empty() {
                // Mirror the parser's own tolerance for a single blank line
                // before annotations start (see
                // `parse_task_section_internal`'s lookahead).
                if !started && !seen_blank {
                    seen_blank = true;
                    j += 1;
                    continue;
                }
                break;
            }
            if leading_space_count(line) <= self.task_indent {
                break;
            }
            if trimmed.starts_with("- ") || trimmed.starts_with("## ") {
                break;
            }
            if trimmed.starts_with('@') {
                started = true;
                j += 1;
            } else if started {
                // Continuation line: indented, not itself an annotation or a
                // bullet, so it's part of the previous annotation's value.
                j += 1;
            } else {
                break;
            }
        }
        (start, j)
    }

    /// End (exclusive) of the task's whole body region: every line up to
    /// the next checkbox line or `## ` heading. The parser merges `@key:`
    /// lines found past free-text body lines into the most recent task as
    /// orphaned annotations (see `parse_task_section_internal`), so an edit
    /// that only searched the contiguous annotation block would miss such
    /// an annotation and write a duplicate `@key:` line that lint then
    /// rejects (GitHub issue #74).
    fn task_body_end(&self) -> usize {
        let mut j = self.task_idx + 1;
        while j < self.lines.len() {
            let trimmed = self.lines[j].trim();
            if trimmed.starts_with("- [") || trimmed.starts_with("## ") {
                break;
            }
            j += 1;
        }
        j
    }

    /// Locate the line index of the `@key:` line belonging to this task, if
    /// present: within the annotation block, or anywhere later in the
    /// task's body (an orphaned annotation after free-text body lines,
    /// which the parser still attributes to this task).
    fn find_annotation_line(&self, key: &str) -> Option<usize> {
        let start = self.task_idx + 1;
        let end = self.task_body_end();
        let prefix = format!("@{key}:");
        (start..end).find(|&i| self.lines[i].trim_start().starts_with(&prefix))
    }

    /// `[start, end)` range of continuation lines directly following an
    /// annotation start line at `idx` (its multi-line value). Within the
    /// annotation block, any non-blank non-`@` line continues the value
    /// (mirroring the parser's lookahead); for an orphaned annotation past
    /// the block, only lines indented deeper than the annotation itself
    /// count, so trailing body text is never swallowed.
    fn continuation_range(&self, idx: usize) -> (usize, usize) {
        let (_, block_end) = self.annotation_block_range();
        let body_end = self.task_body_end();
        let annotation_indent = leading_space_count(&self.lines[idx]);
        let mut j = idx + 1;
        while j < body_end {
            let line = &self.lines[j];
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('@') {
                break;
            }
            if j >= block_end
                && (trimmed.starts_with("- ") || leading_space_count(line) <= annotation_indent)
            {
                break;
            }
            j += 1;
        }
        (idx + 1, j)
    }

    /// Set (or clear, when `value` is `None`) a single-value annotation
    /// (`@owner`, `@estimate`). Replacing preserves the annotation's current
    /// position; adding a new one appends it at the end of the block (before
    /// any contextual notes / children), matching the boundary
    /// `waive.rs::insert_reason_note` already establishes.
    pub fn set_single_annotation(&mut self, key: &str, value: Option<&str>) {
        let annotation_indent = " ".repeat(self.task_indent + 2);
        if let Some(idx) = self.find_annotation_line(key) {
            let (cont_start, cont_end) = self.continuation_range(idx);
            self.lines.drain(cont_start..cont_end);
            match value {
                Some(v) => self.lines[idx] = format!("{annotation_indent}@{key}: {v}"),
                None => {
                    self.lines.remove(idx);
                }
            }
        } else if let Some(v) = value {
            let (_, block_end) = self.annotation_block_range();
            self.lines
                .insert(block_end, format!("{annotation_indent}@{key}: {v}"));
        }
    }

    /// Always-first insertion of an `@id:` annotation, for pinning a
    /// title-derived id before retitling (GitHub issue #25). Only valid to
    /// call when the task has no `@id:` line yet.
    pub fn pin_id(&mut self, slug: &str) {
        let annotation_indent = " ".repeat(self.task_indent + 2);
        self.lines
            .insert(self.task_idx + 1, format!("{annotation_indent}@id: {slug}"));
    }

    /// Replace the task's `@agent-note` (including any existing continuation
    /// lines) with `text`, or add one if absent. Multi-line `text` (containing
    /// `\n`) is written as the annotation line plus indented continuation
    /// lines, matching the multi-line annotation format the parser accepts.
    pub fn set_agent_note(&mut self, text: &str) {
        let new_lines = agent_note_lines(text, self.task_indent);
        if let Some(idx) = self.find_annotation_line("agent-note") {
            let (_, cont_end) = self.continuation_range(idx);
            self.lines.splice(idx..cont_end, new_lines);
        } else {
            let (_, block_end) = self.annotation_block_range();
            self.lines.splice(block_end..block_end, new_lines);
        }
    }

    /// Append `text` as a new continuation line under the existing
    /// `@agent-note`, or create one (identical to [`Self::set_agent_note`])
    /// if the task has none yet.
    pub fn append_agent_note(&mut self, text: &str) {
        let Some(idx) = self.find_annotation_line("agent-note") else {
            self.set_agent_note(text);
            return;
        };
        let (_, cont_end) = self.continuation_range(idx);
        let continuation_indent = " ".repeat(leading_space_count(&self.lines[idx]) + 2);
        let new_lines: Vec<String> = text
            .lines()
            .map(|l| format!("{continuation_indent}{l}"))
            .collect();
        for (offset, line) in new_lines.into_iter().enumerate() {
            self.lines.insert(cont_end + offset, line);
        }
    }

    /// Add a `@depends-on: <reference>` line, grouped immediately after the
    /// last existing `@depends-on` line if any, else appended at the end of
    /// the annotation block. Callers are responsible for validating the
    /// reference before calling this (see `add_dependency_check`).
    pub fn add_depends_on(&mut self, reference: &str) {
        let annotation_indent = " ".repeat(self.task_indent + 2);
        let (start, end) = self.annotation_block_range();
        let insert_at = (start..end)
            .rfind(|&i| self.lines[i].trim_start().starts_with("@depends-on:"))
            .map_or(end, |i| i + 1);
        self.lines.insert(
            insert_at,
            format!("{annotation_indent}@depends-on: {reference}"),
        );
    }

    /// Remove one reference from a `@depends-on` line, matched by exact
    /// string against the stored (trimmed) references — a single
    /// `@depends-on: a, b` line splits on commas the same way the parser
    /// does. If removing the reference empties the line, the whole line is
    /// deleted.
    ///
    /// # Errors
    ///
    /// Returns an error if no `@depends-on` entry matches `reference`.
    pub fn remove_depends_on(&mut self, reference: &str) -> Result<()> {
        let (start, end) = self.annotation_block_range();
        let target = reference.trim();
        for i in start..end {
            let trimmed = self.lines[i].trim_start();
            let Some(value) = trimmed.strip_prefix("@depends-on:") else {
                continue;
            };
            let parts: Vec<&str> = value
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.contains(&target) {
                continue;
            }
            let remaining: Vec<&str> = parts.into_iter().filter(|p| *p != target).collect();
            if remaining.is_empty() {
                self.lines.remove(i);
            } else {
                let indent = leading_space_count(&self.lines[i]);
                self.lines[i] = format!(
                    "{}@depends-on: {}",
                    " ".repeat(indent),
                    remaining.join(", ")
                );
            }
            return Ok(());
        }
        bail!("No matching @depends-on reference '{reference}' found on this task");
    }

    /// Locate the task's `@labels:` annotation line, if it has one.
    #[must_use]
    pub fn has_labels_annotation(&self) -> bool {
        self.find_annotation_line("labels").is_some()
    }

    /// Add `label` to the task's `@labels:` annotation (creating one if
    /// absent). Used only when the task already carries `@labels:`
    /// (checked via [`Self::has_labels_annotation`]) — otherwise
    /// [`Self::add_inline_label`] is the canonical form, matching how `lash
    /// add --label` writes new tasks.
    pub fn add_labels_annotation_value(&mut self, label: &str) {
        let annotation_indent = " ".repeat(self.task_indent + 2);
        if let Some(idx) = self.find_annotation_line("labels") {
            let mut parts = self.labels_annotation_values(idx);
            if !parts
                .iter()
                .any(|p| normalize_label(p) == normalize_label(label))
            {
                parts.push(label.to_string());
            }
            self.lines[idx] = format!("{annotation_indent}@labels: {}", parts.join(", "));
        } else {
            let (_, block_end) = self.annotation_block_range();
            self.lines
                .insert(block_end, format!("{annotation_indent}@labels: {label}"));
        }
    }

    /// Remove `label` from the task's `@labels:` annotation. Returns `true`
    /// if it was present and removed (the whole line is deleted if that was
    /// the last label).
    pub fn remove_labels_annotation_value(&mut self, label: &str) -> bool {
        let Some(idx) = self.find_annotation_line("labels") else {
            return false;
        };
        let parts = self.labels_annotation_values(idx);
        let target = normalize_label(label);
        let remaining: Vec<String> = parts
            .iter()
            .filter(|p| normalize_label(p) != target)
            .cloned()
            .collect();
        if remaining.len() == parts.len() {
            return false;
        }
        if remaining.is_empty() {
            self.lines.remove(idx);
        } else {
            let annotation_indent = " ".repeat(self.task_indent + 2);
            self.lines[idx] = format!("{annotation_indent}@labels: {}", remaining.join(", "));
        }
        true
    }

    fn labels_annotation_values(&self, idx: usize) -> Vec<String> {
        let trimmed = self.lines[idx].trim_start();
        let value = trimmed.strip_prefix("@labels:").unwrap_or("").trim();
        value
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    }
}

/// Build the line(s) for a (possibly multi-line) `@agent-note` value:
/// `@agent-note: <first line>` followed by indented continuation lines for
/// any further lines in `text`.
fn agent_note_lines(text: &str, task_indent: usize) -> Vec<String> {
    let annotation_indent = " ".repeat(task_indent + 2);
    let continuation_indent = " ".repeat(task_indent + 4);
    let mut lines: Vec<String> = text
        .lines()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                format!("{annotation_indent}@agent-note: {l}")
            } else {
                format!("{continuation_indent}{l}")
            }
        })
        .collect();
    if lines.is_empty() {
        lines.push(format!("{annotation_indent}@agent-note: {text}"));
    }
    lines
}

/// Extract a trailing run of `#label` tokens from a title, e.g.
/// `"Fix bug #backend #urgent"` -> `"#backend #urgent"`. Returns an empty
/// string if the title has no trailing inline labels.
fn trailing_label_suffix(title: &str) -> String {
    let words: Vec<&str> = title.split_whitespace().collect();
    let mut split_at = words.len();
    for word in words.iter().rev() {
        if word.starts_with('#') && word.len() > 1 {
            split_at -= 1;
        } else {
            break;
        }
    }
    words[split_at..].join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        "# Tasks\n\
         \n\
         @id: tasks\n\
         \n\
         ## Tasks\n\
         \n\
         - [ ] Fix bug #backend\n  \
         @id: fix-bug\n  \
         @owner: alice\n  \
         @depends-on: other-task, another-task\n  \
         @agent-note: First line\n    \
         continuation line\n  \
         - A contextual note\n"
    }

    fn line_number_of(content: &str, needle: &str) -> usize {
        content
            .lines()
            .position(|l| l.contains(needle))
            .map(|i| i + 1)
            .expect("needle present")
    }

    #[test]
    fn title_text_includes_inline_labels() {
        let content = sample();
        let ln = line_number_of(content, "Fix bug");
        let tl = TaskLines::from_content(content, ln);
        assert_eq!(tl.title_text(), "Fix bug #backend");
    }

    #[test]
    fn retitle_preserves_trailing_inline_labels() {
        let content = sample();
        let ln = line_number_of(content, "Fix bug");
        let mut tl = TaskLines::from_content(content, ln);
        tl.retitle("Fix the bug properly");
        assert_eq!(tl.title_text(), "Fix the bug properly #backend");
    }

    #[test]
    fn retitle_without_inline_labels_is_a_plain_replace() {
        let content = "- [ ] Plain task\n";
        let mut tl = TaskLines::from_content(content, 1);
        tl.retitle("Renamed task");
        assert_eq!(tl.title_text(), "Renamed task");
    }

    #[test]
    fn add_inline_label_appends_hashtag() {
        let content = "- [ ] Plain task\n";
        let mut tl = TaskLines::from_content(content, 1);
        tl.add_inline_label("urgent");
        assert_eq!(tl.title_text(), "Plain task #urgent");
    }

    #[test]
    fn remove_inline_label_strips_matching_token() {
        let content = sample();
        let ln = line_number_of(content, "Fix bug");
        let mut tl = TaskLines::from_content(content, ln);
        assert!(tl.remove_inline_label("backend"));
        assert_eq!(tl.title_text(), "Fix bug");
    }

    #[test]
    fn remove_inline_label_is_case_and_normalization_insensitive() {
        let content = "- [ ] Task #Backend-API\n";
        let mut tl = TaskLines::from_content(content, 1);
        assert!(tl.remove_inline_label("backend-api"));
        assert_eq!(tl.title_text(), "Task");
    }

    #[test]
    fn remove_inline_label_missing_returns_false() {
        let content = "- [ ] Task #backend\n";
        let mut tl = TaskLines::from_content(content, 1);
        assert!(!tl.remove_inline_label("nope"));
        assert_eq!(tl.title_text(), "Task #backend");
    }

    #[test]
    fn set_single_annotation_replaces_existing_value() {
        let content = sample();
        let ln = line_number_of(content, "Fix bug");
        let mut tl = TaskLines::from_content(content, ln);
        tl.set_single_annotation("owner", Some("bob"));
        let rendered = tl.render();
        assert!(rendered.contains("@owner: bob"));
        assert!(!rendered.contains("@owner: alice"));
    }

    #[test]
    fn set_single_annotation_adds_when_absent() {
        let content = "- [ ] Task\n  @id: task-1\n";
        let mut tl = TaskLines::from_content(content, 1);
        tl.set_single_annotation("estimate", Some("2h"));
        assert!(tl.render().contains("@estimate: 2h"));
    }

    #[test]
    fn set_single_annotation_none_removes_existing() {
        let content = sample();
        let ln = line_number_of(content, "Fix bug");
        let mut tl = TaskLines::from_content(content, ln);
        tl.set_single_annotation("owner", None);
        assert!(!tl.render().contains("@owner"));
    }

    #[test]
    fn set_single_annotation_none_when_absent_is_a_noop() {
        let content = "- [ ] Task\n  @id: task-1\n";
        let before = TaskLines::from_content(content, 1).render();
        let mut tl = TaskLines::from_content(content, 1);
        tl.set_single_annotation("estimate", None);
        assert_eq!(tl.render(), before);
    }

    #[test]
    fn pin_id_inserts_immediately_after_task_line_and_first() {
        let content = "- [ ] Fix bug\n  @depends-on: other-task\n";
        let mut tl = TaskLines::from_content(content, 1);
        tl.pin_id("fix-bug");
        let rendered = tl.render();
        let lines: Vec<&str> = rendered.lines().collect();
        assert_eq!(lines[1].trim(), "@id: fix-bug");
        assert_eq!(lines[2].trim(), "@depends-on: other-task");
    }

    #[test]
    fn set_agent_note_replaces_multiline_value() {
        let content = sample();
        let ln = line_number_of(content, "Fix bug");
        let mut tl = TaskLines::from_content(content, ln);
        tl.set_agent_note("Replacement note");
        let rendered = tl.render();
        assert!(rendered.contains("@agent-note: Replacement note"));
        assert!(!rendered.contains("First line"));
        assert!(!rendered.contains("continuation line"));
        // The contextual note bullet must survive untouched.
        assert!(rendered.contains("- A contextual note"));
    }

    #[test]
    fn set_agent_note_adds_when_absent() {
        let content = "- [ ] Task\n  @id: task-1\n";
        let mut tl = TaskLines::from_content(content, 1);
        tl.set_agent_note("New note");
        assert!(tl.render().contains("@agent-note: New note"));
    }

    #[test]
    fn append_agent_note_adds_continuation_line() {
        let content = sample();
        let ln = line_number_of(content, "Fix bug");
        let mut tl = TaskLines::from_content(content, ln);
        tl.append_agent_note("Extra detail");
        let rendered = tl.render();
        assert!(rendered.contains("First line"));
        assert!(rendered.contains("continuation line"));
        assert!(rendered.contains("Extra detail"));
        // Order: appended line must come after the original continuation.
        let cont_idx = rendered.find("continuation line").unwrap();
        let extra_idx = rendered.find("Extra detail").unwrap();
        assert!(cont_idx < extra_idx);
    }

    /// GitHub issue #74: free-text body lines between the checkbox and the
    /// `@agent-note` must not hide the note from `--append-agent-note` —
    /// appending used to insert a second `@agent-note:` line that lint then
    /// rejected as a duplicate annotation.
    fn sample_with_body_text() -> &'static str {
        "# Tasks\n\
         \n\
         @id: tasks\n\
         \n\
         ## Tasks\n\
         \n\
         - [ ] Beta task with body text #demo\n  \
         Some free-text body line recorded earlier.\n  \
         @agent-note: Route: original first line.\n    \
         Second line of the note.\n"
    }

    #[test]
    fn append_agent_note_past_body_text_extends_existing_note() {
        let content = sample_with_body_text();
        let ln = line_number_of(content, "Beta task");
        let mut tl = TaskLines::from_content(content, ln);
        tl.append_agent_note("APPEND ONE: first appended line.");
        let rendered = tl.render();
        assert_eq!(rendered.matches("@agent-note:").count(), 1);
        // Appended line lands after the note's existing continuation, at
        // continuation indent.
        let second_idx = rendered.find("Second line of the note.").unwrap();
        let appended_idx = rendered.find("APPEND ONE").unwrap();
        assert!(second_idx < appended_idx);
        assert!(rendered.contains("\n    APPEND ONE: first appended line."));
    }

    #[test]
    fn set_agent_note_past_body_text_replaces_in_place() {
        let content = sample_with_body_text();
        let ln = line_number_of(content, "Beta task");
        let mut tl = TaskLines::from_content(content, ln);
        tl.set_agent_note("Replacement note");
        let rendered = tl.render();
        assert_eq!(rendered.matches("@agent-note:").count(), 1);
        assert!(rendered.contains("@agent-note: Replacement note"));
        assert!(!rendered.contains("Second line of the note."));
        // The body text before the note survives untouched.
        assert!(rendered.contains("Some free-text body line recorded earlier."));
    }

    #[test]
    fn set_single_annotation_past_body_text_replaces_in_place() {
        let content = "- [ ] Task\n  \
             Body text line.\n  \
             @owner: alice\n";
        let mut tl = TaskLines::from_content(content, 1);
        tl.set_single_annotation("owner", Some("bob"));
        let rendered = tl.render();
        assert_eq!(rendered.matches("@owner:").count(), 1);
        assert!(rendered.contains("@owner: bob"));
    }

    #[test]
    fn find_past_body_text_does_not_reach_next_task() {
        let content = "- [ ] First task\n  \
             Body text line.\n\
             - [ ] Second task\n  \
             @agent-note: Belongs to second task.\n";
        let mut tl = TaskLines::from_content(content, 1);
        tl.append_agent_note("Note for first task.");
        let rendered = tl.render();
        // A new note is created for the first task; the second task's note
        // is untouched.
        assert_eq!(rendered.matches("@agent-note:").count(), 2);
        assert!(rendered.contains("@agent-note: Note for first task."));
        let first_note = rendered.find("Note for first task.").unwrap();
        let second_task = rendered.find("- [ ] Second task").unwrap();
        assert!(first_note < second_task);
    }

    #[test]
    fn append_past_body_text_does_not_swallow_trailing_body_text() {
        let content = "- [ ] Task\n  \
             Body text before.\n  \
             @agent-note: The note.\n  \
             Body text after, same indent as the note.\n";
        let mut tl = TaskLines::from_content(content, 1);
        tl.append_agent_note("Appended line.");
        let rendered = tl.render();
        assert_eq!(rendered.matches("@agent-note:").count(), 1);
        // Appended continuation goes directly after the note line, before
        // the same-indent trailing body text.
        let appended_idx = rendered.find("Appended line.").unwrap();
        let trailing_idx = rendered.find("Body text after").unwrap();
        assert!(appended_idx < trailing_idx);
    }

    #[test]
    fn append_agent_note_creates_when_absent() {
        let content = "- [ ] Task\n  @id: task-1\n";
        let mut tl = TaskLines::from_content(content, 1);
        tl.append_agent_note("First note");
        assert!(tl.render().contains("@agent-note: First note"));
    }

    #[test]
    fn add_depends_on_groups_with_existing() {
        let content = sample();
        let ln = line_number_of(content, "Fix bug");
        let mut tl = TaskLines::from_content(content, ln);
        tl.add_depends_on("third-task");
        let rendered = tl.render();
        let dep_idx = rendered.find("@depends-on: other-task").unwrap();
        let new_idx = rendered.find("@depends-on: third-task").unwrap();
        let note_idx = rendered.find("@agent-note").unwrap();
        assert!(dep_idx < new_idx);
        assert!(new_idx < note_idx);
    }

    #[test]
    fn remove_depends_on_from_comma_list_keeps_others() {
        let content = sample();
        let ln = line_number_of(content, "Fix bug");
        let mut tl = TaskLines::from_content(content, ln);
        tl.remove_depends_on("other-task").unwrap();
        let rendered = tl.render();
        assert!(!rendered.contains("other-task,"));
        assert!(rendered.contains("@depends-on: another-task"));
    }

    #[test]
    fn remove_depends_on_last_entry_deletes_line() {
        let content = "- [ ] Task\n  @depends-on: solo-dep\n";
        let mut tl = TaskLines::from_content(content, 1);
        tl.remove_depends_on("solo-dep").unwrap();
        assert!(!tl.render().contains("@depends-on"));
    }

    #[test]
    fn remove_depends_on_missing_errors() {
        let content = "- [ ] Task\n  @depends-on: solo-dep\n";
        let mut tl = TaskLines::from_content(content, 1);
        assert!(tl.remove_depends_on("ghost").is_err());
    }

    #[test]
    fn labels_annotation_add_and_remove_roundtrip() {
        let content = "- [ ] Task\n  @labels: backend\n";
        let mut tl = TaskLines::from_content(content, 1);
        assert!(tl.has_labels_annotation());
        tl.add_labels_annotation_value("urgent");
        assert!(tl.render().contains("@labels: backend, urgent"));
        assert!(tl.remove_labels_annotation_value("backend"));
        assert_eq!(
            tl.render().lines().nth(1).unwrap().trim(),
            "@labels: urgent"
        );
        assert!(tl.remove_labels_annotation_value("urgent"));
        assert!(!tl.render().contains("@labels"));
    }

    #[test]
    fn labels_annotation_absent_reports_false() {
        let content = "- [ ] Task\n";
        let tl = TaskLines::from_content(content, 1);
        assert!(!tl.has_labels_annotation());
    }

    #[test]
    fn trailing_label_suffix_extracts_hashtags() {
        assert_eq!(
            trailing_label_suffix("Fix bug #backend #urgent"),
            "#backend #urgent"
        );
        assert_eq!(trailing_label_suffix("Plain title"), "");
        assert_eq!(trailing_label_suffix("Weird #mid word #end"), "#end");
    }

    #[test]
    fn load_out_of_range_line_number_errors() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("tasks.md");
        std::fs::write(&path, "- [ ] Task\n").unwrap();
        assert!(TaskLines::load(&path, 99).is_err());
    }

    #[test]
    fn write_round_trips_through_disk() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("tasks.md");
        std::fs::write(&path, "- [ ] Task\n").unwrap();
        let mut tl = TaskLines::load(&path, 1).unwrap();
        tl.retitle("Renamed");
        tl.write(&path).unwrap();
        let updated = std::fs::read_to_string(&path).unwrap();
        assert_eq!(updated, "- [ ] Renamed\n");
    }
}
