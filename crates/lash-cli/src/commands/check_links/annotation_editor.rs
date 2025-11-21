//! Markdown file annotation editor
//!
//! Safely updates `@depends-on` annotations in Markdown files with backup support.

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

/// Editor for updating `@depends-on` annotations in Markdown files
pub struct AnnotationEditor {
    /// Project root (for creating backup directory)
    project_root: PathBuf,
    /// Whether to create backups before editing
    create_backups: bool,
}

impl AnnotationEditor {
    /// Create a new annotation editor
    ///
    /// # Arguments
    ///
    /// * `project_root` - Project root directory
    /// * `create_backups` - Whether to create backups (default: true)
    pub fn new(project_root: PathBuf, create_backups: bool) -> Self {
        Self {
            project_root,
            create_backups,
        }
    }

    /// Update a `@depends-on` annotation in a file
    ///
    /// This finds the task with the given ID and replaces the broken reference
    /// with the corrected one in its `@depends-on` annotation.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the Markdown file
    /// * `task_id` - Local task ID (without file prefix)
    /// * `old_ref` - The broken reference to replace
    /// * `new_ref` - The corrected reference
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful, `Err(_)` if file couldn't be read/written or annotation not found
    pub fn update_annotation(
        &self,
        file_path: &Path,
        task_id: &str,
        old_ref: &str,
        new_ref: &str,
    ) -> Result<()> {
        // Read file contents
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        // Find and update the annotation
        let updated = self.replace_depends_on(&content, task_id, old_ref, new_ref)?;

        // Create backup if enabled
        if self.create_backups {
            self.create_backup(file_path, &content)?;
        }

        // Write updated content
        fs::write(file_path, updated)
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;

        tracing::info!(
            file = %file_path.display(),
            task_id = %task_id,
            old_ref = %old_ref,
            new_ref = %new_ref,
            "Updated @depends-on annotation"
        );

        Ok(())
    }

    /// Replace a `@depends-on` reference for a specific task
    ///
    /// This looks for the task section (heading with matching ID),
    /// then finds the `@depends-on` annotation within that section,
    /// and replaces the specific broken reference.
    #[allow(clippy::unused_self)] // Method belongs to impl block for consistency
    fn replace_depends_on(
        &self,
        content: &str,
        task_id: &str,
        old_ref: &str,
        new_ref: &str,
    ) -> Result<String> {
        // Find the task section by looking for the ID annotation
        let task_section_pattern = format!(r"(?m)^(@id:\s*{}\s*)$", regex::escape(task_id));
        let task_section_re =
            Regex::new(&task_section_pattern).context("Failed to compile task section regex")?;

        if let Some(task_match) = task_section_re.find(content) {
            let start = task_match.start();

            // Find the next task section or end of file
            let after_task = &content[task_match.end()..];
            let next_task_re =
                Regex::new(r"(?m)^@id:\s*\S+\s*$").context("Failed to compile next task regex")?;
            let end = if let Some(next_match) = next_task_re.find(after_task) {
                task_match.end() + next_match.start()
            } else {
                content.len()
            };

            // Extract the task section
            let task_section = &content[start..end];

            // Find and replace the @depends-on annotation
            let depends_on_pattern = format!(
                r"(?m)^(@depends-on:\s*)(.*)({})(.*)$",
                regex::escape(old_ref)
            );
            let depends_on_re =
                Regex::new(&depends_on_pattern).context("Failed to compile depends-on regex")?;

            if let Some(dep_match) = depends_on_re.find(task_section) {
                // Build the updated content
                let mut result = String::new();
                result.push_str(&content[..start + dep_match.start()]);

                // Replace within the matched line
                let line = &task_section[dep_match.start()..dep_match.end()];
                let updated_line = line.replace(old_ref, new_ref);
                result.push_str(&updated_line);

                result.push_str(&content[start + dep_match.end()..]);

                return Ok(result);
            }

            return Err(anyhow!(
                "Could not find @depends-on annotation with reference '{old_ref}' for task '{task_id}'"
            ));
        }

        Err(anyhow!("Could not find task with ID '{task_id}'"))
    }

    /// Create a backup of the file before modifying it
    ///
    /// Backups are stored in `.lash/backups/TIMESTAMP/` to allow rollback if needed.
    fn create_backup(&self, file_path: &Path, content: &str) -> Result<()> {
        // Create backup directory with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_dir = self
            .project_root
            .join(".lash")
            .join("backups")
            .join(timestamp.to_string());

        fs::create_dir_all(&backup_dir).with_context(|| {
            format!(
                "Failed to create backup directory: {}",
                backup_dir.display()
            )
        })?;

        // Determine backup file path (preserve relative structure)
        let relative_path = if file_path.is_absolute() {
            file_path
                .strip_prefix(&self.project_root)
                .unwrap_or(file_path)
        } else {
            file_path
        };

        let backup_path = backup_dir.join(relative_path);

        // Create parent directories if needed
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create backup parent directory: {}",
                    parent.display()
                )
            })?;
        }

        // Write backup
        fs::write(&backup_path, content)
            .with_context(|| format!("Failed to write backup: {}", backup_path.display()))?;

        tracing::debug!(
            original = %file_path.display(),
            backup = %backup_path.display(),
            "Created backup"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_file(content: &str) -> (TempDir, PathBuf) {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, content).unwrap();
        (temp, file_path)
    }

    #[test]
    fn test_update_simple_annotation() {
        let content = r"# Test Task

@id: task1
@depends-on: other#task999

Some content here.
";

        let (temp, file_path) = create_test_file(content);
        let editor = AnnotationEditor::new(temp.path().to_path_buf(), false);

        editor
            .update_annotation(&file_path, "task1", "other#task999", "other#task9")
            .unwrap();

        let updated = fs::read_to_string(&file_path).unwrap();
        assert!(updated.contains("@depends-on: other#task9"));
        assert!(!updated.contains("@depends-on: other#task999"));
    }

    #[test]
    fn test_update_with_multiple_dependencies() {
        let content = r"# Test Task

@id: task1
@depends-on: tasks#setup, other#task999, tasks#teardown

Some content here.
";

        let (temp, file_path) = create_test_file(content);
        let editor = AnnotationEditor::new(temp.path().to_path_buf(), false);

        editor
            .update_annotation(&file_path, "task1", "other#task999", "other#task9")
            .unwrap();

        let updated = fs::read_to_string(&file_path).unwrap();
        assert!(updated.contains("@depends-on: tasks#setup, other#task9, tasks#teardown"));
    }

    #[test]
    fn test_update_only_target_task() {
        let content = r"# Task 1

@id: task1
@depends-on: other#task999

# Task 2

@id: task2
@depends-on: other#task999

Some content.
";

        let (temp, file_path) = create_test_file(content);
        let editor = AnnotationEditor::new(temp.path().to_path_buf(), false);

        // Update only task1
        editor
            .update_annotation(&file_path, "task1", "other#task999", "other#task9")
            .unwrap();

        let updated = fs::read_to_string(&file_path).unwrap();

        // task1 should be updated
        let task1_section = updated.split("# Task 2").next().unwrap();
        assert!(task1_section.contains("@depends-on: other#task9"));

        // task2 should still have old reference
        let task2_section = updated.split("# Task 2").nth(1).unwrap();
        assert!(task2_section.contains("@depends-on: other#task999"));
    }

    #[test]
    fn test_error_on_missing_task() {
        let content = r"# Test Task

@id: task1
@depends-on: other#task999
";

        let (temp, file_path) = create_test_file(content);
        let editor = AnnotationEditor::new(temp.path().to_path_buf(), false);

        let result =
            editor.update_annotation(&file_path, "nonexistent", "other#task999", "other#task9");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Could not find task"));
    }

    #[test]
    fn test_error_on_missing_annotation() {
        let content = r"# Test Task

@id: task1

Some content, but no @depends-on annotation.
";

        let (temp, file_path) = create_test_file(content);
        let editor = AnnotationEditor::new(temp.path().to_path_buf(), false);

        let result = editor.update_annotation(&file_path, "task1", "other#task999", "other#task9");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Could not find @depends-on"));
    }

    #[test]
    fn test_backup_creation() {
        let content = r"# Test Task

@id: task1
@depends-on: other#task999
";

        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, content).unwrap();

        let editor = AnnotationEditor::new(temp.path().to_path_buf(), true);

        editor
            .update_annotation(&file_path, "task1", "other#task999", "other#task9")
            .unwrap();

        // Check backup was created
        let backup_dir = temp.path().join(".lash/backups");
        assert!(backup_dir.exists());

        // Find the timestamped backup directory
        let backup_entries: Vec<_> = fs::read_dir(&backup_dir).unwrap().collect();
        assert!(!backup_entries.is_empty(), "Backup directory should exist");

        // Verify backup contains original content
        let backup_file = backup_entries[0].as_ref().unwrap().path().join("test.md");
        let backup_content = fs::read_to_string(&backup_file).unwrap();
        assert_eq!(backup_content, content);
    }

    #[test]
    fn test_preserves_formatting() {
        let content = r"# Test Task

@id: task1
@depends-on: other#task999
@labels: urgent, backend

Some content here.

- [ ] Subtask 1
- [ ] Subtask 2
";

        let (temp, file_path) = create_test_file(content);
        let editor = AnnotationEditor::new(temp.path().to_path_buf(), false);

        editor
            .update_annotation(&file_path, "task1", "other#task999", "other#task9")
            .unwrap();

        let updated = fs::read_to_string(&file_path).unwrap();

        // Check that structure is preserved
        assert!(updated.contains("# Test Task"));
        assert!(updated.contains("@labels: urgent, backend"));
        assert!(updated.contains("- [ ] Subtask 1"));
        assert!(updated.contains("- [ ] Subtask 2"));
    }
}
