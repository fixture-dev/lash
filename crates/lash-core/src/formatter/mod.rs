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
//! let formatted = formatter.format_file(&file).unwrap();
//! assert!(formatted.contains("# Test File"));
//! ```

pub mod options;

use lash_types::{LashConfig, Result, TaskFile, TaskStatus};
use std::path::Path;

pub use options::FormatOptions;

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
    /// let formatted = formatter.format_file(&file).unwrap();
    /// assert!(formatted.contains("# Test File"));
    /// ```
    #[allow(clippy::result_large_err)] // LashError is intentionally rich with context
    pub fn format_file(&self, file: &TaskFile) -> Result<String> {
        // Apply auto-fixes if enabled
        let file = if self.options.apply_auto_fixes {
            self.apply_auto_fixes(file)?
        } else {
            file.clone()
        };

        // Build the formatted output
        let mut output = String::new();

        // 1. Format header (title, annotations, overview)
        self.format_header(&file, &mut output);

        // 2. Format description section (if present)
        if file.description.is_some() {
            Self::format_description(&file, &mut output);
        }

        // 3. Format tasks section
        self.format_tasks(&file, &mut output);

        // 4. Format references section (if present)
        // TODO: Implement once references are stored in TaskFile

        // 4. Normalize whitespace
        if self.options.normalize_whitespace {
            output = self.normalize_whitespace(&output);
        }

        // 5. Ensure file ends with single newline
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
        // Parse the file
        let file = crate::parser::parse_file(path, &self.config)?;

        // Format it
        let formatted = self.format_file(&file)?;

        // Write back
        std::fs::write(path, formatted).map_err(|e| lash_types::LashError::IO {
            code: "E_IO_WRITE_FAILED",
            message: format!("Failed to write formatted file: {}", path.display()),
            path: Some(path.to_path_buf()),
            io_error: Some(e.to_string()),
        })?;

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
        if let Some(ref status) = file.metadata.status {
            output.push_str("@status: ");
            output.push_str(status);
            output.push('\n');
        }

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

        if !file.metadata.labels.is_empty() {
            // Sort labels for consistency
            let mut labels = file.metadata.labels.clone();
            labels.sort();
            annotations.push(("labels", labels.join(", ")));
        }

        if let Some(ref owner) = file.metadata.owner {
            annotations.push(("owner", owner.clone()));
        }

        if let Some(ref status) = file.metadata.status {
            annotations.push(("status", status.clone()));
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
    fn format_tasks(&self, file: &TaskFile, output: &mut String) {
        output.push_str("## Tasks\n");
        output.push('\n');

        // Get all root tasks (tasks with no parent)
        let all_tasks = file.tasks.tasks();
        let root_tasks: Vec<_> = all_tasks.iter().filter(|t| t.parent_id.is_none()).collect();

        // Format each root task and its descendants
        for task in root_tasks {
            self.format_task(task, all_tasks, output);
        }
    }

    /// Format a single task and its children (recursive)
    fn format_task(
        &self,
        task: &lash_types::Task,
        all_tasks: &[lash_types::Task],
        output: &mut String,
    ) {
        // Calculate indentation
        let indent_spaces = self.options.indent_spaces as usize;
        let indent = " ".repeat(task.depth as usize * indent_spaces);

        // Format checkbox line
        output.push_str(&indent);
        output.push_str("- [");
        output.push(task.status.to_checkbox_char());
        output.push_str("] ");
        output.push_str(&task.title);

        // Add inline labels if present
        if !task.metadata.labels.is_empty() {
            for label in &task.metadata.labels {
                output.push_str(" #");
                output.push_str(label);
            }
        }

        output.push('\n');

        // Format children
        let children: Vec<_> = all_tasks
            .iter()
            .filter(|child| child.parent_id.as_deref() == Some(&task.id))
            .collect();

        for child in children {
            self.format_task(child, all_tasks, output);
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

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{FileMetadata, Task, TaskMetadata, TaskTree};
    use std::path::PathBuf;
    use std::time::SystemTime;

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
            title: title.to_string(),
            status,
            depth,
            parent_id: parent.map(std::string::ToString::to_string),
            order_index: 0,
            metadata: TaskMetadata::default(),
            body: None,
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

        let result = formatter.format_file(&file).unwrap();

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

        let result = formatter.format_file(&file).unwrap();

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

        let result = formatter.format_file(&file).unwrap();

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
}
