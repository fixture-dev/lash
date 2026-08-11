//! Auto-formatting for Lash Markdown task files
//!
//! This module provides automatic formatting to enforce consistent style
//! and apply linter auto-fixes. The formatter is idempotent - running it
//! multiple times produces the same result.
//!
//! # Key Features
//!
//! - **Indentation normalization**: Enforces exactly 2 spaces per level
//! - **Annotation sorting**: Alphabetizes annotations with @id first
//! - **Whitespace cleanup**: Removes trailing whitespace, normalizes blank lines
//! - **Auto-fix application**: Applies linter fixes (waiving, status consistency)
//! - **Content preservation**: Non-task content (overview, references) unchanged
//! - **Round-trip safety**: parse → format → parse is idempotent
//!
//! # Example
//!
//! ```
//! use lash_core::formatter::{Formatter, FormatOptions};
//! use lash_types::{LashConfig, TaskFile, FileMetadata, TaskTree};
//! use std::path::PathBuf;
//! use std::time::SystemTime;
//!
//! let config = LashConfig::default();
//! let options = FormatOptions::default();
//! let formatter = Formatter::new(config, options);
//!
//! // Create a simple task file for demonstration
//! let file = TaskFile {
//!     path: PathBuf::from("test.md"),
//!     title: "Test File".to_string(),
//!     id: "test".to_string(),
//!     metadata: FileMetadata::default(),
//!     tasks: TaskTree::new(),
//!     hash: "hash".to_string(),
//!     mtime: SystemTime::now(),
//!     description: None,
//!     description_agent_notes: Vec::new(),
//! };
//! let formatted = formatter.format_file("", &file).unwrap();
//! assert!(formatted.contains("# Test File"));
//! ```

pub mod options;

use crate::parser::header;
use lash_types::{LashConfig, Result, TaskFile, TaskStatus};
use std::path::Path;

pub use options::FormatOptions;

/// Something the formatter regenerates, found at a source line
///
/// Everything inside `## Tasks` that is not one of these is copied through
/// from the source, because the model does not represent it.
enum Anchor<'a> {
    /// A task's checkbox line, which also carries its annotation block.
    Task(&'a lash_types::Task),

    /// A single contextual-note bullet, with the depth of the task it belongs
    /// to. Notes are anchored individually: only a note's first line lives in
    /// the model, so its wrapped continuation lines are copied through and the
    /// first line has to stay in front of them.
    Note(usize, &'a lash_types::task::ContextualNote),
}

/// Formatter for Lash task files
///
/// The formatter takes a parsed `TaskFile` and produces formatted Markdown
/// that follows Lash style conventions. It can apply linter auto-fixes and
/// ensures round-trip safety (formatting is idempotent).
///
/// # Philosophy
///
/// The formatter follows these principles:
///
/// 1. **Simplicity**: Uses straightforward string manipulation
/// 2. **Safety**: Never loses content or changes semantics
/// 3. **Idempotence**: Formatting twice produces same result
/// 4. **Preservation**: Non-task content remains unchanged
///
/// # Implementation Notes
///
/// The formatter operates on the parsed AST, not raw text. This ensures
/// we never accidentally corrupt the file structure. It reconstructs the
/// file from the AST with proper formatting applied.
pub struct Formatter {
    /// Project configuration
    config: LashConfig,

    /// Formatting options
    options: FormatOptions,
}

impl Formatter {
    /// Create a new formatter with the given configuration and options
    #[must_use]
    pub fn new(config: LashConfig, options: FormatOptions) -> Self {
        Self { config, options }
    }

    /// Format a task file and return the formatted Markdown string
    ///
    /// This method takes a parsed `TaskFile` and produces a formatted
    /// Markdown string that follows Lash style conventions.
    ///
    /// # Arguments
    ///
    /// * `file` - The parsed task file to format
    ///
    /// # Returns
    ///
    /// Returns the formatted Markdown as a `String`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Auto-fixes cannot be applied
    /// - Formatting would produce invalid syntax
    /// - Re-parsing the formatted output fails
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::formatter::{Formatter, FormatOptions};
    /// use lash_types::{LashConfig, TaskFile, FileMetadata, TaskTree};
    /// use std::path::PathBuf;
    /// use std::time::SystemTime;
    ///
    /// # let config = LashConfig::default();
    /// # let options = FormatOptions::default();
    /// # let formatter = Formatter::new(config, options);
    /// # let file = TaskFile {
    /// #     path: PathBuf::from("test.md"),
    /// #     title: "Test File".to_string(),
    /// #     id: "test".to_string(),
    /// #     metadata: FileMetadata::default(),
    /// #     tasks: TaskTree::new(),
    /// #     hash: "hash".to_string(),
    /// #     mtime: SystemTime::now(),
    /// #     description: None,
    /// #     description_agent_notes: Vec::new(),
    /// # };
    /// let formatted = formatter.format_file("", &file).unwrap();
    /// assert!(formatted.contains("# Test File"));
    /// ```
    #[allow(clippy::result_large_err)] // LashError is intentionally rich with context
    pub fn format_file(&self, source: &str, file: &TaskFile) -> Result<String> {
        // Apply auto-fixes if enabled
        let file = if self.options.apply_auto_fixes {
            self.apply_auto_fixes(file)?
        } else {
            file.clone()
        };

        let lines: Vec<&str> = source.lines().collect();
        let header = header::header_span(source);
        let description = header::section_span(source, "description");
        let tasks = header::section_span(source, "tasks");

        let mut output = String::new();

        // The header is always regenerated, whether or not the source had one.
        self.format_header(&file, &mut output);

        // Then walk the rest of the source. Two spans are ours to replace with
        // generated text; everything else is copied through byte for byte,
        // because the model does not represent it and regenerating the file
        // from the model alone silently deletes it.
        // Each generated block ends with its own blank separator, so the
        // source's is skipped rather than copied; emitting both would add a
        // blank line on every run and formatting would not be idempotent.
        let skip_blanks = |line: &mut usize| {
            while lines.get(*line).is_some_and(|l| l.trim().is_empty()) {
                *line += 1;
            }
        };

        let mut line = header.end;
        skip_blanks(&mut line);

        while line < lines.len() {
            if description.as_ref().is_some_and(|span| span.start == line) {
                Self::format_description(&file, &mut output);
                line = description.as_ref().map_or(line + 1, |span| span.end);
                skip_blanks(&mut line);
            } else if tasks.as_ref().is_some_and(|span| span.start == line) {
                // The body is walked rather than regenerated: the model holds
                // the checkbox lines, their annotations and their contextual
                // notes, and nothing else in the section. Bodies, separators
                // and anything else present are copied through.
                let body = tasks.as_ref().map_or(line + 1..line + 1, |span| {
                    (span.start + 1).min(span.end)..span.end
                });
                self.format_tasks(&file, Some((&lines[body.clone()], body.start)), &mut output);
                output.push('\n');
                line = tasks.as_ref().map_or(line + 1, |span| span.end);
                skip_blanks(&mut line);
            } else {
                output.push_str(lines[line]);
                output.push('\n');
                line += 1;
            }
        }

        // A source with no `## Tasks` heading still gets one. The parser treats
        // such a file as all-tasks and warns, and emitting the section is what
        // makes `lash format` able to repair it.
        if tasks.is_none() {
            if description.is_none() && file.description.is_some() {
                Self::format_description(&file, &mut output);
            }
            self.format_tasks(&file, None, &mut output);
        }

        // Normalize whitespace. This only collapses runs of blank lines and
        // trims trailing whitespace, so it is safe to run over copied-through
        // content as well as generated content.
        if self.options.normalize_whitespace {
            output = self.normalize_whitespace(&output);
        }

        // Ensure file ends with single newline
        if !output.ends_with('\n') {
            output.push('\n');
        } else if output.ends_with("\n\n") {
            output = output.trim_end().to_string();
            output.push('\n');
        }

        Ok(output)
    }

    /// Format a file in place, writing the result back to disk
    ///
    /// This is a convenience method that formats the file and writes it
    /// back to its original location.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to format
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if formatting succeeds, or an error if formatting
    /// or writing fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be parsed
    /// - Formatting fails
    /// - The file cannot be written
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_core::formatter::{Formatter, FormatOptions};
    /// use lash_types::LashConfig;
    /// use std::path::Path;
    ///
    /// # let config = LashConfig::default();
    /// # let options = FormatOptions::default();
    /// # let formatter = Formatter::new(config, options);
    /// // This requires actual file I/O, so we mark it no_run
    /// formatter.format_file_in_place(Path::new("tasks.md")).unwrap();
    /// ```
    #[allow(clippy::result_large_err)] // LashError is intentionally rich with context
    pub fn format_file_in_place(&self, path: &Path) -> Result<()> {
        // Parse the file, keeping the source: the formatter needs it to carry
        // through sections the model does not represent.
        let source = std::fs::read_to_string(path).map_err(|e| lash_types::LashError::IO {
            code: lash_types::error::codes::E_IO_READ_ERROR,
            message: format!("Failed to read file for formatting: {e}"),
            path: Some(path.to_path_buf()),
            io_error: Some(e.to_string()),
        })?;
        let file = crate::parser::parse_file(path, &self.config)?;

        // Format it
        let formatted = self.format_file(&source, &file)?;

        // Write back atomically (tmp file + rename) so a crash mid-write
        // can't leave a partial Markdown file on disk. This shares the same
        // helper that `lash_core::store::write_atomic` uses for status
        // toggles, keeping all production write paths crash-safe in the
        // same way.
        crate::store::write_atomic(path, formatted.as_bytes())?;

        Ok(())
    }

    /// Apply auto-fixes from linter rules
    ///
    /// This applies semantic fixes to the file:
    /// - Auto-waiving children when parent is waived
    /// - Fixing parent-child status consistency
    ///
    /// Since `TaskTree` doesn't expose mutable access, we rebuild the tree
    /// with fixed tasks.
    #[allow(clippy::unused_self)] // Future extension point for config-based fixes
    #[allow(clippy::result_large_err)] // LashError is intentionally rich with context
    fn apply_auto_fixes(&self, file: &TaskFile) -> Result<TaskFile> {
        let all_tasks = file.tasks.tasks();
        let mut fixed_tasks = Vec::new();

        // Build status fix map
        let mut status_fixes = std::collections::HashMap::new();

        // 1. Auto-waive children when parent is waived
        for task in all_tasks {
            if task.status == TaskStatus::Waived {
                let descendants = Self::get_all_descendants(task, all_tasks);
                for descendant in descendants {
                    if descendant.status != TaskStatus::Waived {
                        status_fixes.insert(descendant.id.clone(), TaskStatus::Waived);
                    }
                }
            }
        }

        // 2. Fix parent-child status consistency
        for task in all_tasks {
            if task.status == TaskStatus::Done {
                let children = Self::get_direct_children(task, all_tasks);
                let has_incomplete = children
                    .iter()
                    .any(|c| c.status != TaskStatus::Done && c.status != TaskStatus::Waived);

                if has_incomplete {
                    status_fixes.insert(task.id.clone(), TaskStatus::Open);
                }
            }
        }

        // Apply fixes and rebuild task list
        for task in all_tasks {
            let mut fixed_task = task.clone();
            if let Some(new_status) = status_fixes.get(&task.id) {
                fixed_task.status = *new_status;
            }
            fixed_tasks.push(fixed_task);
        }

        // Rebuild the task tree
        let mut new_tree = lash_types::TaskTree::new();
        for task in fixed_tasks {
            new_tree.add_task(task)?;
        }

        // Create a new file with the fixed tree
        Ok(TaskFile {
            path: file.path.clone(),
            title: file.title.clone(),
            id: file.id.clone(),
            metadata: file.metadata.clone(),
            description: file.description.clone(),
            description_agent_notes: file.description_agent_notes.clone(),
            tasks: new_tree,
            hash: file.hash.clone(),
            mtime: file.mtime,
        })
    }

    /// Get all descendants of a task (recursive)
    fn get_all_descendants<'a>(
        task: &lash_types::Task,
        all_tasks: &'a [lash_types::Task],
    ) -> Vec<&'a lash_types::Task> {
        let mut descendants = Vec::new();

        let children: Vec<_> = all_tasks
            .iter()
            .filter(|child| child.parent_id.as_deref() == Some(&task.id))
            .collect();

        for child in children {
            descendants.push(child);
            descendants.extend(Self::get_all_descendants(child, all_tasks));
        }

        descendants
    }

    /// Get direct children of a task
    fn get_direct_children<'a>(
        task: &lash_types::Task,
        all_tasks: &'a [lash_types::Task],
    ) -> Vec<&'a lash_types::Task> {
        all_tasks
            .iter()
            .filter(|child| child.parent_id.as_deref() == Some(&task.id))
            .collect()
    }

    /// Format the header section (title, annotations, overview)
    fn format_header(&self, file: &TaskFile, output: &mut String) {
        // 1. Title (H1)
        output.push_str("# ");
        output.push_str(&file.title);
        output.push('\n');
        output.push('\n');

        // 2. Annotations
        if self.options.sort_annotations {
            self.format_annotations_sorted(file, output);
        } else {
            self.format_annotations(file, output);
        }

        // 3. Overview (if present in metadata - for now skip as not stored)
        // TODO: Add overview support when it's stored in TaskFile

        // Add blank line after header
        output.push('\n');
    }

    /// Format annotations in their original order
    #[allow(clippy::unused_self)] // Keeps consistent API with format_annotations_sorted
    fn format_annotations(&self, file: &TaskFile, output: &mut String) {
        // Always output @id first
        output.push_str("@id: ");
        output.push_str(&file.id);
        output.push('\n');

        // Output other annotations from metadata
        if !file.metadata.labels.is_empty() {
            output.push_str("@labels: ");
            // Sort labels for consistency
            let mut labels = file.metadata.labels.clone();
            labels.sort();
            output.push_str(&labels.join(", "));
            output.push('\n');
        }

        if let Some(ref owner) = file.metadata.owner {
            output.push_str("@owner: ");
            output.push_str(owner);
            output.push('\n');
        }

        if let Some(ref created) = file.metadata.created {
            output.push_str("@created: ");
            output.push_str(created);
            output.push('\n');
        }

        if !file.metadata.depends_on.is_empty() {
            output.push_str("@depends-on: ");
            let deps: Vec<_> = file
                .metadata
                .depends_on
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
            output.push_str(&deps.join(", "));
            output.push('\n');
        }

        // Output @doc references (note: singular annotation key, plural field name)
        if !file.metadata.docs.is_empty() {
            output.push_str("@doc: ");
            let docs: Vec<_> = file
                .metadata
                .docs
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
            output.push_str(&docs.join(", "));
            output.push('\n');
        }

        // Custom annotations (alphabetically sorted by key)
        let mut custom_keys: Vec<_> = file.metadata.custom.keys().collect();
        custom_keys.sort();
        for key in custom_keys {
            if let Some(value) = file.metadata.custom.get(key) {
                output.push('@');
                output.push_str(key);
                output.push_str(": ");
                output.push_str(value);
                output.push('\n');
            }
        }
    }

    /// Format annotations sorted alphabetically (except @id which is always first)
    #[allow(clippy::unused_self)] // Keeps consistent API with format_annotations
    fn format_annotations_sorted(&self, file: &TaskFile, output: &mut String) {
        // @id is always first
        output.push_str("@id: ");
        output.push_str(&file.id);
        output.push('\n');

        // Collect all other annotations
        let mut annotations = Vec::new();

        if let Some(ref created) = file.metadata.created {
            annotations.push(("created", created.clone()));
        }

        if !file.metadata.depends_on.is_empty() {
            let deps: Vec<_> = file
                .metadata
                .depends_on
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
            annotations.push(("depends-on", deps.join(", ")));
        }

        // Output @doc references (note: singular annotation key, plural field name)
        if !file.metadata.docs.is_empty() {
            let docs: Vec<_> = file
                .metadata
                .docs
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
            annotations.push(("doc", docs.join(", ")));
        }

        if !file.metadata.labels.is_empty() {
            // Sort labels for consistency
            let mut labels = file.metadata.labels.clone();
            labels.sort();
            annotations.push(("labels", labels.join(", ")));
        }

        if let Some(ref owner) = file.metadata.owner {
            annotations.push(("owner", owner.clone()));
        }

        // Add custom annotations
        for (key, value) in &file.metadata.custom {
            annotations.push((key.as_str(), value.clone()));
        }

        // Sort alphabetically by key
        annotations.sort_by_key(|(key, _)| *key);

        // Output sorted annotations
        for (key, value) in annotations {
            output.push('@');
            output.push_str(key);
            output.push_str(": ");
            output.push_str(&value);
            output.push('\n');
        }
    }

    /// Format the description section
    fn format_description(file: &TaskFile, output: &mut String) {
        output.push_str("## Description\n");
        output.push('\n');

        if let Some(ref description) = file.description {
            output.push_str(description);
            output.push('\n');
            output.push('\n');
        }
    }

    /// Format the tasks section
    ///
    /// `body` is the source of the section below the heading, as the lines
    /// themselves plus the 0-indexed position of the first of them. Given it,
    /// the section is walked rather than rebuilt.
    ///
    /// Rebuilding was the bug. A parsed task carries its checkbox line, its
    /// annotations and its contextual notes, and the model holds nothing else
    /// that lives inside `## Tasks`. Everything else the section contained —
    /// a task's free-text body of prose, numbered steps or acceptance
    /// criteria, a `---` separator, a comment — had no representation to be
    /// rebuilt from and was deleted, with exit code 0. #44 stopped `format`
    /// from dropping whole sections the same way; this is the same fix
    /// applied one level down, to the one section the formatter does own.
    ///
    /// So the formatter regenerates only the lines it can account for and
    /// copies every other line through. Without a source (a file whose
    /// `## Tasks` heading is missing entirely, which `format` adds) there is
    /// nothing to walk and the whole section is generated from the model.
    fn format_tasks(&self, file: &TaskFile, body: Option<(&[&str], usize)>, output: &mut String) {
        output.push_str("## Tasks\n");
        output.push('\n');

        let all_tasks = file.tasks.tasks();

        let Some((body_lines, body_start)) = body else {
            for task in all_tasks.iter().filter(|t| t.parent_id.is_none()) {
                self.format_task(task, all_tasks, output);
            }
            return;
        };

        // 1-indexed source lines the walk covers.
        let walked = body_start + 1..=body_start + body_lines.len();

        // What the formatter regenerates, keyed by the source line it starts
        // on. A task's checkbox line and each of its contextual notes are
        // separate anchors: a note is regenerated where it sits, not alongside
        // the task, because only a note's *first* line is in the model. Its
        // wrapped continuation lines are copied through, so hoisting the first
        // line up to the task would strand the rest of the note behind it.
        let mut starts_at: std::collections::HashMap<usize, Anchor> =
            std::collections::HashMap::new();

        // Annotation lines, which the task's own anchor regenerates.
        let mut owned: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for task in all_tasks {
            if !walked.contains(&task.line_number) {
                continue;
            }
            starts_at.insert(task.line_number, Anchor::Task(task));
            for offset in 1..=task.annotation_line_count {
                owned.insert(task.line_number + offset);
            }
            for note in &task.contextual_notes {
                if walked.contains(&note.line_number()) {
                    starts_at.insert(note.line_number(), Anchor::Note(task.depth as usize, note));
                }
            }
        }

        let mut emitted: std::collections::HashSet<&str> = std::collections::HashSet::new();

        // Blank lines bounding the section are the separators around it, and
        // both ends are written by the caller — the heading above carries its
        // own, and the section is followed by one. Copying them through as
        // well would double them on every run.
        let leading_blanks = body_lines
            .iter()
            .position(|line| !line.trim().is_empty())
            .unwrap_or(body_lines.len());
        let body_end = body_lines
            .iter()
            .rposition(|line| !line.trim().is_empty())
            .map_or(leading_blanks, |last| last + 1);

        for (offset, line) in body_lines[..body_end]
            .iter()
            .enumerate()
            .skip(leading_blanks)
        {
            let line_number = body_start + offset + 1;

            match starts_at.get(&line_number) {
                Some(Anchor::Task(task)) => {
                    self.format_task_head(task, output);
                    // A note with no line the walk will reach cannot be placed,
                    // so it goes with its task rather than being dropped.
                    for note in &task.contextual_notes {
                        if !walked.contains(&note.line_number()) {
                            self.format_contextual_note(task.depth as usize, note, output);
                        }
                    }
                    emitted.insert(task.id.as_str());
                }
                Some(Anchor::Note(depth, note)) => {
                    self.format_contextual_note(*depth, note, output);
                }
                None if !owned.contains(&line_number) => {
                    output.push_str(line);
                    output.push('\n');
                }
                None => {}
            }
        }

        // A task the walk never reached has no source line to have been found
        // at — it was built in memory, or `format_file` was handed a source
        // that is not the one the file was parsed from. Dropping it would be
        // the very data loss this walk exists to prevent, so it goes at the
        // end of the section.
        for task in all_tasks {
            if !emitted.contains(task.id.as_str()) {
                self.format_task_entry(task, output);
            }
        }
    }

    /// Format a single task and its children (recursive)
    fn format_task(
        &self,
        task: &lash_types::Task,
        all_tasks: &[lash_types::Task],
        output: &mut String,
    ) {
        self.format_task_entry(task, output);

        let children: Vec<_> = all_tasks
            .iter()
            .filter(|child| child.parent_id.as_deref() == Some(&task.id))
            .collect();

        for child in children {
            self.format_task(child, all_tasks, output);
        }
    }

    /// Format one task's own lines: checkbox, annotations, contextual notes
    ///
    /// Children are not included. Walking the source reaches each task at its
    /// own line, so recursing here would emit a subtree twice.
    fn format_task_entry(&self, task: &lash_types::Task, output: &mut String) {
        self.format_task_head(task, output);
        for note in &task.contextual_notes {
            self.format_contextual_note(task.depth as usize, note, output);
        }
    }

    /// Format one contextual note as a plain bullet
    fn format_contextual_note(
        &self,
        depth: usize,
        note: &lash_types::task::ContextualNote,
        output: &mut String,
    ) {
        let indent_spaces = self.options.indent_spaces as usize;
        output.push_str(&" ".repeat((depth + 1) * indent_spaces));
        output.push_str("- ");
        output.push_str(note.text());
        output.push('\n');
    }

    /// Format a task's checkbox line and its annotation block
    fn format_task_head(&self, task: &lash_types::Task, output: &mut String) {
        // Calculate indentation
        let indent_spaces = self.options.indent_spaces as usize;
        let indent = " ".repeat(task.depth as usize * indent_spaces);
        let annotation_indent = " ".repeat((task.depth as usize + 1) * indent_spaces);

        // Format checkbox line
        output.push_str(&indent);
        output.push_str("- [");
        output.push(task.status.to_checkbox_char());
        output.push_str("] ");

        // The parser leaves inline labels in the title *and* records them in
        // metadata, so writing the title verbatim and then appending the
        // labels emits each one twice. Formatting was therefore not
        // idempotent: every run grew the line by another copy of every inline
        // label, and `format --check` reported the file as needing formatting
        // forever. Strip them from the title and let the canonical, sorted
        // list below be the only place they appear.
        output.push_str(&strip_inline_labels(&task.title));

        // Add inline labels if present
        if !task.metadata.labels.is_empty() {
            let mut labels = task.metadata.labels.clone();
            labels.sort();
            for label in &labels {
                output.push_str(" #");
                output.push_str(label);
            }
        }

        output.push('\n');

        // Format task-level annotations (indented one level deeper)
        self.format_task_annotations(task, &annotation_indent, output);
    }

    /// Format task-level annotations
    ///
    /// Outputs annotations like @id, @owner, @estimate, @depends-on, @agent-note, etc.
    /// Only outputs annotations that are present (non-empty).
    #[allow(clippy::unused_self)] // Keeps consistent API pattern
    fn format_task_annotations(&self, task: &lash_types::Task, indent: &str, output: &mut String) {
        // Output @id only if the task has an explicit ID (not synthesized)
        if task.has_explicit_id {
            output.push_str(indent);
            output.push_str("@id: ");
            output.push_str(&task.id);
            output.push('\n');
        }

        // Output @owner if present
        if let Some(ref owner) = task.metadata.owner {
            output.push_str(indent);
            output.push_str("@owner: ");
            output.push_str(owner);
            output.push('\n');
        }

        // Output @estimate if present
        if let Some(ref estimate) = task.metadata.estimate {
            output.push_str(indent);
            output.push_str("@estimate: ");
            output.push_str(estimate);
            output.push('\n');
        }

        // Output @depends-on if present
        if !task.metadata.depends_on.is_empty() {
            output.push_str(indent);
            output.push_str("@depends-on: ");
            let deps: Vec<_> = task
                .metadata
                .depends_on
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
            output.push_str(&deps.join(", "));
            output.push('\n');
        }

        // Output @doc if present (note: singular, not @docs)
        if !task.metadata.docs.is_empty() {
            output.push_str(indent);
            output.push_str("@doc: ");
            let docs: Vec<_> = task
                .metadata
                .docs
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
            output.push_str(&docs.join(", "));
            output.push('\n');
        }

        // Output @agent-note if present
        if let Some(ref agent_note) = task.metadata.agent_note {
            output.push_str(indent);
            output.push_str("@agent-note: ");
            output.push_str(agent_note);
            output.push('\n');
        }

        // Output custom annotations (alphabetically sorted by key)
        let mut custom_keys: Vec<_> = task.metadata.custom.keys().collect();
        custom_keys.sort();
        for key in custom_keys {
            if let Some(value) = task.metadata.custom.get(key) {
                output.push_str(indent);
                output.push('@');
                output.push_str(key);
                output.push_str(": ");
                output.push_str(value);
                output.push('\n');
            }
        }
    }

    /// Normalize whitespace in the output
    ///
    /// This:
    /// - Trims trailing whitespace from all lines
    /// - Collapses multiple blank lines to max configured amount
    /// - Ensures single blank line between major sections
    fn normalize_whitespace(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut output = Vec::new();
        let mut blank_count = 0;
        let max_blanks = if self.options.preserve_blank_lines {
            2
        } else {
            1
        };

        for line in lines {
            let trimmed = line.trim_end();

            if trimmed.is_empty() {
                blank_count += 1;
                if blank_count <= max_blanks {
                    output.push("");
                }
            } else {
                blank_count = 0;
                output.push(trimmed);
            }
        }

        // Remove trailing blank lines (they'll be added back later if needed)
        while output.last() == Some(&"") {
            output.pop();
        }

        output.join("\n")
    }
}

/// Remove inline `#label` words from a task title
///
/// The parser records inline labels in `TaskMetadata` without removing them
/// from the title, so a formatter that writes both would emit each label
/// twice. Only whole words that parsing would turn into labels are removed, so
/// a `#` inside prose survives.
fn strip_inline_labels(title: &str) -> String {
    title
        .split_whitespace()
        .filter(|word| !lash_types::label::is_inline_label(word))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{FileMetadata, Task, TaskMetadata, TaskTree};
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[test]
    fn format_file_in_place_writes_atomically_and_leaves_no_temp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        // Hand-written file with whitespace the formatter will normalize.
        std::fs::write(
            &path,
            "# Demo\n\n@id: demo\n\n## Tasks\n\n- [ ] First task\n\n\n- [ ] Second task\n",
        )
        .unwrap();

        let formatter = Formatter::new(LashConfig::default(), FormatOptions::default());
        formatter.format_file_in_place(&path).unwrap();

        // The file content survived and is still parseable.
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.contains("First task") && after.contains("Second task"),
            "formatted output should still contain both tasks, got: {after}"
        );

        // No temp file leaked alongside it.
        let leaked = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .any(|e| e.file_name().to_string_lossy().ends_with(".lash-tmp"));
        assert!(!leaked, "atomic write should leave no .lash-tmp behind");
    }

    fn make_config() -> LashConfig {
        LashConfig {
            root_path: PathBuf::from("/test"),
            index_file: "index.md".to_string(),
            max_depth: 2,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/test.db"),
            custom_annotation_keys: vec![],
        }
    }

    fn make_task(
        id: &str,
        title: &str,
        status: TaskStatus,
        parent: Option<&str>,
        depth: u8,
    ) -> Task {
        Task {
            id: id.to_string(),
            has_explicit_id: false,
            title: title.to_string(),
            status,
            depth,
            parent_id: parent.map(std::string::ToString::to_string),
            order_index: 0,
            line_number: 0,
            annotation_line_count: 0,
            metadata: TaskMetadata::default(),
            body: None,
            contextual_notes: Vec::new(),
        }
    }

    fn make_file_with_tasks(tasks: Vec<Task>) -> TaskFile {
        let mut tree = TaskTree::new();
        for task in tasks {
            tree.add_task(task).unwrap();
        }

        TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test File".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: tree,
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_format_simple_file() {
        let config = make_config();
        let options = FormatOptions::default();
        let formatter = Formatter::new(config, options);

        let file = make_file_with_tasks(vec![
            make_task("task1", "First task", TaskStatus::Open, None, 0),
            make_task("task2", "Second task", TaskStatus::Done, None, 0),
        ]);

        let result = formatter.format_file("", &file).unwrap();

        assert!(result.contains("# Test File"));
        assert!(result.contains("@id: test"));
        assert!(result.contains("## Tasks"));
        assert!(result.contains("- [ ] First task"));
        assert!(result.contains("- [x] Second task"));
    }

    #[test]
    fn test_format_with_hierarchy() {
        let config = make_config();
        let options = FormatOptions::default();
        let formatter = Formatter::new(config, options);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent task", TaskStatus::Open, None, 0),
            make_task("child1", "Child 1", TaskStatus::Open, Some("parent"), 1),
            make_task("child2", "Child 2", TaskStatus::Done, Some("parent"), 1),
        ]);

        let result = formatter.format_file("", &file).unwrap();

        assert!(result.contains("- [ ] Parent task"));
        assert!(result.contains("  - [ ] Child 1"));
        assert!(result.contains("  - [x] Child 2"));
    }

    #[test]
    fn test_auto_waive_children() {
        let config = make_config();
        let options = FormatOptions {
            apply_auto_fixes: true,
            ..Default::default()
        };
        let formatter = Formatter::new(config, options);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Waived, None, 0),
            make_task("child1", "Child 1", TaskStatus::Open, Some("parent"), 1),
            make_task("child2", "Child 2", TaskStatus::Open, Some("parent"), 1),
        ]);

        let result = formatter.format_file("", &file).unwrap();

        // Children should be auto-waived
        assert!(result.contains("- [-] Parent"));
        assert!(result.contains("  - [-] Child 1"));
        assert!(result.contains("  - [-] Child 2"));
    }

    #[test]
    fn test_whitespace_normalization() {
        let config = make_config();
        let options = FormatOptions::default();
        let formatter = Formatter::new(config, options);

        let content = "Line 1   \n\n\n\nLine 2\nLine 3  \n\n\n";
        let result = formatter.normalize_whitespace(content);

        // Trailing spaces removed
        assert!(!result.contains("   "));
        // Multiple blank lines collapsed
        assert!(!result.contains("\n\n\n\n"));
    }

    /// Format `source` with `normalize_whitespace` set either way.
    ///
    /// The parse is what production does — `format_file_in_place` parses the
    /// same source it hands the formatter — so these tests exercise the real
    /// path rather than a hand-built `TaskFile`.
    fn format_with_normalize(source: &str, normalize: bool) -> String {
        let config = LashConfig::default();
        let file =
            crate::parser::parse_file_from_string(source, &config).expect("fixture should parse");
        let options = FormatOptions {
            normalize_whitespace: normalize,
            ..FormatOptions::default()
        };
        Formatter::new(config, options)
            .format_file(source, &file)
            .expect("formatting should succeed")
    }

    /// `test_whitespace_normalization` above calls `normalize_whitespace`
    /// directly, so nothing pinned the flag that decides whether it runs at
    /// all: inverting it left every test passing. This asserts both settings
    /// produce their own result.
    #[test]
    fn normalize_whitespace_option_decides_whether_blank_runs_collapse() {
        let source = "# Demo\n\n@id: demo\n\n## Notes\n\nalpha\n\n\n\n\nbeta\n\n## Tasks\n\n- [ ] First task\n";

        let collapsed = format_with_normalize(source, true);
        assert!(
            !collapsed.contains("\n\n\n\n"),
            "normalize_whitespace = true should collapse the blank run, got: {collapsed:?}"
        );

        let preserved = format_with_normalize(source, false);
        assert!(
            preserved.contains("\n\n\n\n"),
            "normalize_whitespace = false should leave the blank run alone, got: {preserved:?}"
        );
    }

    /// The trailing-newline branch only runs when the output actually ends in
    /// a blank line, which `normalize_whitespace` otherwise hides by stripping
    /// trailing blanks first. With it off, the generated tasks section leaves a
    /// blank line at EOF and this is the only thing that trims it.
    #[test]
    fn format_ends_file_with_exactly_one_newline() {
        let source = "# Demo\n\n@id: demo\n\n## Tasks\n\n- [ ] First task\n";

        let formatted = format_with_normalize(source, false);

        assert!(
            formatted.ends_with('\n'),
            "formatted file should end with a newline, got: {formatted:?}"
        );
        assert!(
            !formatted.ends_with("\n\n"),
            "formatted file should not end with a blank line, got: {formatted:?}"
        );
    }

    /// A file with no `## Tasks` heading gets one synthesized. The description
    /// must not be synthesized alongside it when the source already had a
    /// `## Description` section — the walk copied that through already, so
    /// emitting it again duplicates it.
    #[test]
    fn synthesized_tasks_section_does_not_duplicate_an_existing_description() {
        let source = "# Demo\n\n@id: demo\n\n## Description\n\nProse that belongs to the file.\n";

        let formatted = format_with_normalize(source, true);

        assert_eq!(
            formatted.matches("Prose that belongs to the file.").count(),
            1,
            "description should appear exactly once, got: {formatted:?}"
        );
        assert!(
            formatted.contains("## Tasks"),
            "a missing tasks section should still be synthesized, got: {formatted:?}"
        );
    }
}
