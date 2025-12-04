//! Duplicate description section validation rule
//!
//! Ensures that task files contain at most one `## Description` section.
//! The parser only processes the first Description section it finds, so
//! duplicate sections would be silently ignored, which could confuse users.

use lash_types::{Severity, TaskFile};
use std::fs;

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates there's only one Description section
///
/// This rule checks that task files have at most one `## Description` section.
/// Multiple Description sections are not allowed because:
/// - The parser only processes the first one
/// - Subsequent sections would be silently ignored
/// - This could lead to confusion about which description applies
///
/// **Code:** `E_SYNTAX_DUPLICATE_DESCRIPTION`
/// **Severity:** Error
///
/// # Implementation Note
///
/// This rule requires access to the raw file content because the parsed
/// `TaskFile` only contains the first description (subsequent ones are
/// discarded during parsing). We read the file directly to detect all
/// occurrences of `## Description` headings.
///
/// # Examples
///
/// Valid (no description):
/// ```markdown
/// # Project Name
///
/// @id: project
///
/// ## Tasks
///
/// - [ ] First task
/// ```
///
/// Valid (one description):
/// ```markdown
/// # Project Name
///
/// @id: project
///
/// ## Description
///
/// This is the project description.
///
/// ## Tasks
///
/// - [ ] First task
/// ```
///
/// Invalid (duplicate description):
/// ```markdown
/// # Project Name
///
/// ## Description
///
/// First description.
///
/// ## Description
///
/// Second description - this would be ignored by the parser!
///
/// ## Tasks
///
/// - [ ] First task
/// ```
pub struct DuplicateDescriptionRule;

impl DuplicateDescriptionRule {
    /// Create a new duplicate description rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Find all occurrences of `## Description` headings in the content
    ///
    /// Returns a vector of (`line_number`, `line_content`) tuples for each
    /// occurrence found. Line numbers are 1-indexed.
    fn find_all_description_headings(content: &str) -> Vec<(usize, String)> {
        content
            .lines()
            .enumerate()
            .filter_map(|(idx, line)| {
                let trimmed = line.trim();
                // Check for ## Description (case-insensitive)
                if let Some(stripped) = trimmed.strip_prefix("## ") {
                    let heading_text = stripped.trim();
                    if heading_text.eq_ignore_ascii_case("description") {
                        return Some((idx + 1, line.to_string())); // 1-indexed line number
                    }
                }
                None
            })
            .collect()
    }
}

impl Default for DuplicateDescriptionRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for DuplicateDescriptionRule {
    fn code(&self) -> &'static str {
        "E_SYNTAX_DUPLICATE_DESCRIPTION"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Duplicate description section".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures files have at most one ## Description section"
    }

    fn check_file(&self, _file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Read the raw file content
        // Note: We use the file path from the context, which should be the actual file path
        let Ok(content) = fs::read_to_string(&ctx.file_path) else {
            // If we can't read the file, we can't check for duplicates
            // This might happen in tests or when validating in-memory content
            // Just return no diagnostics rather than failing
            return diagnostics;
        };

        // Find all Description section headings
        let description_headings = Self::find_all_description_headings(&content);

        // If more than one, report an error
        if description_headings.len() > 1 {
            // Create a message listing all the locations
            let locations: Vec<String> = description_headings
                .iter()
                .map(|(line_num, _)| format!("line {line_num}"))
                .collect();

            let message = format!(
                "Found {} '## Description' sections ({}), but only one is allowed",
                description_headings.len(),
                locations.join(", ")
            );

            // Report error at the location of the SECOND occurrence (first is valid)
            let (second_line, second_content) = &description_headings[1];

            diagnostics.push(
                LintDiagnostic::error(self.code(), message, ctx.file_path.clone(), *second_line, 0)
                    .with_snippet(second_content.clone())
                    .with_help(
                        "Remove duplicate '## Description' sections - only one is allowed per file",
                    ),
            );
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{FileMetadata, LashConfig, TaskTree};
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tempfile::TempDir;

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

    fn make_file() -> TaskFile {
        TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test Project".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            description: Some("First description".to_string()),
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_find_all_description_headings_none() {
        let content = r"# Title

@id: test

## Tasks

- [ ] Task 1
";
        let headings = DuplicateDescriptionRule::find_all_description_headings(content);
        assert_eq!(headings.len(), 0);
    }

    #[test]
    fn test_find_all_description_headings_one() {
        let content = r"# Title

@id: test

## Description

This is the description.

## Tasks

- [ ] Task 1
";
        let headings = DuplicateDescriptionRule::find_all_description_headings(content);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].0, 5); // Line 5
        assert!(headings[0].1.contains("## Description"));
    }

    #[test]
    fn test_find_all_description_headings_two() {
        let content = r"# Title

@id: test

## Description

First description.

## Description

Second description.

## Tasks

- [ ] Task 1
";
        let headings = DuplicateDescriptionRule::find_all_description_headings(content);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].0, 5); // Line 5
        assert_eq!(headings[1].0, 9); // Line 9
    }

    #[test]
    fn test_find_all_description_headings_case_insensitive() {
        let content = r"# Title

## Description

First description.

## description

Second description.

## DESCRIPTION

Third description.
";
        let headings = DuplicateDescriptionRule::find_all_description_headings(content);
        assert_eq!(headings.len(), 3);
    }

    #[test]
    fn test_find_all_description_headings_extra_whitespace() {
        let content = r"# Title

##   Description

Description with extra spaces.

##  description

Another one.
";
        let headings = DuplicateDescriptionRule::find_all_description_headings(content);
        assert_eq!(headings.len(), 2);
    }

    #[test]
    fn test_file_with_no_description() {
        // Create a temporary file
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let mut file = File::create(&file_path).unwrap();
        writeln!(
            file,
            r"# Test

@id: test

## Tasks

- [ ] Task 1
"
        )
        .unwrap();
        file.flush().unwrap();

        let rule = DuplicateDescriptionRule::new();
        let config = make_config();
        let mut task_file = make_file();
        task_file.path = file_path.clone();
        task_file.description = None;
        let files = HashMap::new();
        let ctx = LintContext::new(&config, file_path, &files);

        let diagnostics = rule.check_file(&task_file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_file_with_one_description() {
        // Create a temporary file
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let mut file = File::create(&file_path).unwrap();
        writeln!(
            file,
            r"# Test

@id: test

## Description

This is a description.

## Tasks

- [ ] Task 1
"
        )
        .unwrap();
        file.flush().unwrap();

        let rule = DuplicateDescriptionRule::new();
        let config = make_config();
        let mut task_file = make_file();
        task_file.path = file_path.clone();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, file_path, &files);

        let diagnostics = rule.check_file(&task_file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_file_with_two_descriptions() {
        // Create a temporary file
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let mut file = File::create(&file_path).unwrap();
        writeln!(
            file,
            r"# Test

@id: test

## Description

First description.

## Description

Second description.

## Tasks

- [ ] Task 1
"
        )
        .unwrap();
        file.flush().unwrap();

        let rule = DuplicateDescriptionRule::new();
        let config = make_config();
        let mut task_file = make_file();
        task_file.path = file_path.clone();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, file_path, &files);

        let diagnostics = rule.check_file(&task_file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SYNTAX_DUPLICATE_DESCRIPTION");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(diagnostics[0].message.contains('2'));
        assert!(diagnostics[0].message.contains("line 9"));
        assert!(diagnostics[0].help.is_some());
        assert!(diagnostics[0].snippet.is_some());
    }

    #[test]
    fn test_file_with_three_descriptions() {
        // Create a temporary file
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let mut file = File::create(&file_path).unwrap();
        writeln!(
            file,
            r"# Test

## Description

First.

## Description

Second.

## Description

Third.

## Tasks

- [ ] Task
"
        )
        .unwrap();
        file.flush().unwrap();

        let rule = DuplicateDescriptionRule::new();
        let config = make_config();
        let mut task_file = make_file();
        task_file.path = file_path.clone();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, file_path, &files);

        let diagnostics = rule.check_file(&task_file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SYNTAX_DUPLICATE_DESCRIPTION");
        assert!(diagnostics[0].message.contains('3'));
        assert!(diagnostics[0].message.contains("line 3"));
        assert!(diagnostics[0].message.contains("line 7"));
        assert!(diagnostics[0].message.contains("line 11"));
    }

    #[test]
    fn test_rule_metadata() {
        let rule = DuplicateDescriptionRule::new();
        assert_eq!(rule.code(), "E_SYNTAX_DUPLICATE_DESCRIPTION");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Duplicate description section");
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_default_trait() {
        let rule = DuplicateDescriptionRule;
        assert_eq!(rule.code(), "E_SYNTAX_DUPLICATE_DESCRIPTION");
    }

    #[test]
    fn test_cannot_read_file() {
        // Use a path that doesn't exist
        let rule = DuplicateDescriptionRule::new();
        let config = make_config();
        let task_file = make_file();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("/nonexistent/file.md"), &files);

        let diagnostics = rule.check_file(&task_file, &ctx);
        // Should not fail, just return empty diagnostics
        assert!(diagnostics.is_empty());
    }
}
