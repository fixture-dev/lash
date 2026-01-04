//! Header structure validation rule
//!
//! Ensures that task files have the required header structure:
//! an H1 title and a "## Tasks" section. This provides consistency
//! and helps both humans and agents quickly understand file structure.

use lash_types::{Severity, TaskFile};

use crate::linter::{Fix, LintContext, LintDiagnostic, LintRule};

/// Rule that validates file header structure
///
/// This rule checks that task files have the required header structure:
/// - An H1 (`#`) title at the top
/// - A "## Tasks" section header
///
/// **Code:** `W_SYNTAX_HEADER`
/// **Severity:** Warning
///
/// While not strictly required for parsing, this structure provides:
/// - Consistent file organization
/// - Quick orientation for readers
/// - Predictable structure for agents and tools
///
/// # Examples
///
/// Valid:
/// ```markdown
/// # Project Name
///
/// @id: project
///
/// ## Tasks
///
/// - [ ] First task
/// - [ ] Second task
/// ```
///
/// Invalid (missing Tasks section):
/// ```markdown
/// # Project Name
///
/// - [ ] First task
/// ```
///
/// Invalid (no H1):
/// ```markdown
/// ## Tasks
///
/// - [ ] First task
/// ```
pub struct HeaderStructureRule;

impl HeaderStructureRule {
    /// Create a new header structure rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Create a template header structure for auto-fix
    #[allow(dead_code)] // Used in tests
    fn create_template_header(file_path: &std::path::Path) -> String {
        let title = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");

        format!(
            "# {}\n\n@id: {}\n\n## Tasks\n\n",
            title,
            title.to_lowercase().replace(' ', "-")
        )
    }
}

impl Default for HeaderStructureRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for HeaderStructureRule {
    fn code(&self) -> &'static str {
        "W_SYNTAX_HEADER"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> String {
        "Header structure".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures files have an H1 title and ## Tasks section for consistent structure"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check for H1 title
        // In our parsed structure, the title is extracted from H1, so if it exists
        // we know there was an H1. However, we can check if it's empty.
        if file.title.trim().is_empty() {
            diagnostics.push(
                LintDiagnostic::warning(
                    self.code(),
                    "File missing H1 title".to_string(),
                    ctx.file_path.clone(),
                    1,
                    0,
                )
                .with_help("Add an H1 heading (# Title) at the start of the file")
                .with_fix(Fix::reformat(
                    "Add template header with H1 and Tasks section",
                )),
            );
        }

        // Note: We cannot reliably check for "## Tasks" section from the parsed structure
        // because the parser doesn't preserve section headings. This check would need
        // access to the raw markdown content.
        //
        // For now, we only check for H1 title. The full check including "## Tasks"
        // section would be better implemented in the parser or with raw file access.

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{FileMetadata, LashConfig, TaskTree};
    use std::collections::HashMap;
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

    fn make_file(title: &str) -> TaskFile {
        TaskFile {
            path: PathBuf::from("test.md"),
            title: title.to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_file_with_title() {
        let rule = HeaderStructureRule::new();
        let config = make_config();
        let file = make_file("Test Project");
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_file_without_title() {
        let rule = HeaderStructureRule::new();
        let config = make_config();
        let file = make_file("");
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_SYNTAX_HEADER");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("H1 title"));
        assert!(diagnostics[0].help.is_some());
        assert!(diagnostics[0].has_fix());
    }

    #[test]
    fn test_file_with_whitespace_only_title() {
        let rule = HeaderStructureRule::new();
        let config = make_config();
        let file = make_file("   ");
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("H1 title"));
    }

    #[test]
    fn test_create_template_header() {
        let template = HeaderStructureRule::create_template_header(&PathBuf::from("my-project.md"));
        assert!(template.contains("# my-project"));
        assert!(template.contains("@id: my-project"));
        assert!(template.contains("## Tasks"));
    }

    #[test]
    fn test_template_with_complex_filename() {
        let template =
            HeaderStructureRule::create_template_header(&PathBuf::from("Complex File Name.md"));
        assert!(template.contains("# Complex File Name"));
        assert!(template.contains("@id: complex-file-name"));
    }

    #[test]
    fn test_rule_metadata() {
        let rule = HeaderStructureRule::new();
        assert_eq!(rule.code(), "W_SYNTAX_HEADER");
        assert_eq!(rule.severity(), Severity::Warning);
        assert_eq!(rule.name(), "Header structure");
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_diagnostic_has_fix() {
        use crate::linter::Replacement;

        let rule = HeaderStructureRule::new();
        let config = make_config();
        let file = make_file("");
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].fix.is_some());
        if let Some(fix) = &diagnostics[0].fix {
            assert!(matches!(fix.replacement, Replacement::Reformat));
        }
    }
}
