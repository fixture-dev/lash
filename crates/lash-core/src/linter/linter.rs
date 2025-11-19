//! Main linter orchestration
//!
//! The `Linter` coordinates the execution of linting rules across files
//! and tasks, collecting diagnostics and optionally applying fixes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use lash_types::{LashConfig, TaskFile};

use crate::linter::{LintConfig, LintContext, LintDiagnostic, LintRule};

/// Main linter that orchestrates rule execution
///
/// The linter maintains a registry of rules and executes them against task files.
/// It can lint individual files or entire projects, and can optionally apply
/// auto-fixes.
///
/// # Example
///
/// ```rust,ignore
/// use lash_core::linter::{Linter, LintConfig};
/// use lash_types::TaskFile;
///
/// let lint_config = LintConfig::default();
/// let mut linter = Linter::new(lint_config);
///
/// // Register rules
/// linter.register_rule(Box::new(DepthLimitRule::new(3)));
/// linter.register_rule(Box::new(DuplicateIdRule::new()));
///
/// // Lint a file
/// let file: TaskFile = /* ... */;
/// let diagnostics = linter.lint_file(&file);
///
/// for diagnostic in diagnostics {
///     println!("{}", diagnostic);
/// }
/// ```
pub struct Linter {
    /// Registered linting rules
    rules: Vec<Arc<dyn LintRule>>,

    /// Linter configuration
    config: LintConfig,
}

impl Linter {
    /// Create a new linter with the given configuration
    #[must_use]
    pub fn new(config: LintConfig) -> Self {
        Self {
            rules: Vec::new(),
            config,
        }
    }

    /// Register a linting rule
    ///
    /// Rules are executed in the order they are registered. Generally, syntax
    /// rules should be registered before semantic rules.
    pub fn register_rule(&mut self, rule: Arc<dyn LintRule>) {
        self.rules.push(rule);
    }

    /// Register multiple rules at once
    pub fn register_rules(&mut self, rules: Vec<Arc<dyn LintRule>>) {
        self.rules.extend(rules);
    }

    /// Lint a single file
    ///
    /// Executes all enabled rules against the file and its tasks, collecting
    /// diagnostics. The file context includes only this file (for cross-file
    /// validation, use `lint_project`).
    ///
    /// # Arguments
    ///
    /// * `file` - The task file to lint
    /// * `project_config` - Project configuration (max depth, custom annotations, etc.)
    ///
    /// # Returns
    ///
    /// A list of diagnostics sorted by location and severity
    #[must_use]
    pub fn lint_file(&self, file: &TaskFile, project_config: &LashConfig) -> Vec<LintDiagnostic> {
        let mut all_files = HashMap::new();
        all_files.insert(file.path.clone(), file.clone());

        self.lint_file_with_context(file, project_config, &all_files)
    }

    /// Lint a file with access to other files for cross-file validation
    ///
    /// This is the internal method used by both `lint_file` and `lint_project`.
    /// It provides access to all files in the project for cross-file validation.
    fn lint_file_with_context(
        &self,
        file: &TaskFile,
        project_config: &LashConfig,
        all_files: &HashMap<PathBuf, TaskFile>,
    ) -> Vec<LintDiagnostic> {
        let context = LintContext::new(project_config, file.path.clone(), all_files);

        let mut diagnostics = Vec::new();

        // Run file-level rules
        for rule in &self.rules {
            // Skip if rule is disabled
            if !self.config.is_rule_enabled(rule.code()) {
                continue;
            }

            // Execute rule and collect diagnostics
            let mut rule_diagnostics = rule.check_file(file, &context);

            // Apply severity overrides
            for diag in &mut rule_diagnostics {
                let overridden_severity = self.config.get_severity(diag.code, diag.severity);
                diag.severity = overridden_severity;
            }

            diagnostics.extend(rule_diagnostics);
        }

        // Run task-level rules on each task
        for task in file.tasks.tasks() {
            for rule in &self.rules {
                // Skip if rule is disabled
                if !self.config.is_rule_enabled(rule.code()) {
                    continue;
                }

                // Execute rule and collect diagnostics
                let mut rule_diagnostics = rule.check_task(task, &context);

                // Apply severity overrides
                for diag in &mut rule_diagnostics {
                    let overridden_severity = self.config.get_severity(diag.code, diag.severity);
                    diag.severity = overridden_severity;
                }

                diagnostics.extend(rule_diagnostics);
            }
        }

        // Sort diagnostics by location and severity
        diagnostics.sort();

        diagnostics
    }

    /// Lint all files in a project
    ///
    /// Executes all enabled rules against all files, including cross-file
    /// validation rules. Returns diagnostics for all files.
    ///
    /// # Arguments
    ///
    /// * `files` - Map of file paths to parsed task files
    /// * `project_config` - Project configuration
    ///
    /// # Returns
    ///
    /// A list of all diagnostics sorted by location and severity
    #[must_use]
    pub fn lint_project(
        &self,
        files: &HashMap<PathBuf, TaskFile>,
        project_config: &LashConfig,
    ) -> Vec<LintDiagnostic> {
        let mut all_diagnostics = Vec::new();

        for file in files.values() {
            let diagnostics = self.lint_file_with_context(file, project_config, files);
            all_diagnostics.extend(diagnostics);
        }

        // Sort all diagnostics
        all_diagnostics.sort();

        all_diagnostics
    }

    /// Lint files at the given paths
    ///
    /// This is a convenience method that discovers and lints all markdown files
    /// at the given paths (recursively if directories are provided).
    ///
    /// Note: This method requires the parser to be available. For now, it's a
    /// placeholder that will be implemented once the parser integration is complete.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Paths don't exist
    /// - Files can't be read
    /// - Parsing fails
    #[allow(clippy::result_large_err)]
    pub fn lint_paths(
        &self,
        _paths: &[PathBuf],
        _project_config: &LashConfig,
    ) -> lash_types::Result<Vec<LintDiagnostic>> {
        // TODO: Implement file discovery and parsing
        // This will be implemented in Task #6 (CLI Integration)
        todo!("File discovery and parsing integration not yet implemented")
    }

    /// Get the number of registered rules
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Get the number of enabled rules
    #[must_use]
    pub fn enabled_rule_count(&self) -> usize {
        self.rules
            .iter()
            .filter(|rule| self.config.is_rule_enabled(rule.code()))
            .count()
    }

    /// Get a list of all registered rule codes
    #[must_use]
    pub fn rule_codes(&self) -> Vec<&'static str> {
        self.rules.iter().map(|rule| rule.code()).collect()
    }

    /// Get access to the linter configuration
    #[must_use]
    pub fn config(&self) -> &LintConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{
        FileMetadata, LashConfig, Severity, Task, TaskMetadata, TaskStatus, TaskTree,
    };
    use std::time::SystemTime;

    // Test rule that always returns one diagnostic per task
    struct AlwaysFailRule;

    impl LintRule for AlwaysFailRule {
        fn code(&self) -> &'static str {
            "E_TEST_ALWAYS_FAIL"
        }

        fn severity(&self) -> Severity {
            Severity::Error
        }

        fn check_task(&self, _task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
            vec![LintDiagnostic::error(
                self.code(),
                "test error",
                ctx.file_path.clone(),
                1,
                1,
            )]
        }
    }

    // Test rule that checks file-level issues
    struct FileRule;

    impl LintRule for FileRule {
        fn code(&self) -> &'static str {
            "W_TEST_FILE"
        }

        fn severity(&self) -> Severity {
            Severity::Warning
        }

        fn check_file(&self, _file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
            vec![LintDiagnostic::warning(
                self.code(),
                "file warning",
                ctx.file_path.clone(),
                1,
                1,
            )]
        }
    }

    fn make_test_file(path: &str, task_count: usize) -> TaskFile {
        let mut tree = TaskTree::new();

        for i in 0..task_count {
            let task = Task {
                id: format!("task-{i}"),
                title: format!("Task {i}"),
                status: TaskStatus::Open,
                depth: 0,
                parent_id: None,
                order_index: i,
                metadata: TaskMetadata::default(),
                body: None,
            };
            tree.add_task(task).unwrap();
        }

        TaskFile {
            path: PathBuf::from(path),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            tasks: tree,
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_linter_creation() {
        let config = LintConfig::default();
        let linter = Linter::new(config);
        assert_eq!(linter.rule_count(), 0);
    }

    #[test]
    fn test_register_rule() {
        let config = LintConfig::default();
        let mut linter = Linter::new(config);

        linter.register_rule(Arc::new(AlwaysFailRule));
        assert_eq!(linter.rule_count(), 1);
        assert_eq!(linter.rule_codes(), vec!["E_TEST_ALWAYS_FAIL"]);
    }

    #[test]
    fn test_lint_file_with_tasks() {
        let config = LintConfig::default();
        let mut linter = Linter::new(config);
        linter.register_rule(Arc::new(AlwaysFailRule));

        let file = make_test_file("test.md", 3);
        let project_config = LashConfig::default();

        let diagnostics = linter.lint_file(&file, &project_config);
        assert_eq!(diagnostics.len(), 3); // One per task
        assert!(diagnostics.iter().all(|d| d.code == "E_TEST_ALWAYS_FAIL"));
    }

    #[test]
    fn test_lint_file_level_rule() {
        let config = LintConfig::default();
        let mut linter = Linter::new(config);
        linter.register_rule(Arc::new(FileRule));

        let file = make_test_file("test.md", 0); // No tasks
        let project_config = LashConfig::default();

        let diagnostics = linter.lint_file(&file, &project_config);
        assert_eq!(diagnostics.len(), 1); // One file-level diagnostic
        assert_eq!(diagnostics[0].code, "W_TEST_FILE");
    }

    #[test]
    fn test_disabled_rule() {
        let mut config = LintConfig::default();
        config.disable_rule("E_TEST_ALWAYS_FAIL");

        let mut linter = Linter::new(config);
        linter.register_rule(Arc::new(AlwaysFailRule));

        let file = make_test_file("test.md", 3);
        let project_config = LashConfig::default();

        let diagnostics = linter.lint_file(&file, &project_config);
        assert_eq!(diagnostics.len(), 0); // Rule disabled, no diagnostics
    }

    #[test]
    fn test_severity_override() {
        let mut config = LintConfig::default();
        config.set_severity("E_TEST_ALWAYS_FAIL", Severity::Warning);

        let mut linter = Linter::new(config);
        linter.register_rule(Arc::new(AlwaysFailRule));

        let file = make_test_file("test.md", 1);
        let project_config = LashConfig::default();

        let diagnostics = linter.lint_file(&file, &project_config);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Warning); // Overridden
    }

    #[test]
    fn test_lint_project() {
        let config = LintConfig::default();
        let mut linter = Linter::new(config);
        linter.register_rule(Arc::new(AlwaysFailRule));

        let mut files = HashMap::new();
        files.insert(PathBuf::from("file1.md"), make_test_file("file1.md", 2));
        files.insert(PathBuf::from("file2.md"), make_test_file("file2.md", 3));

        let project_config = LashConfig::default();
        let diagnostics = linter.lint_project(&files, &project_config);

        assert_eq!(diagnostics.len(), 5); // 2 + 3 tasks
    }

    #[test]
    fn test_enabled_rule_count() {
        let mut config = LintConfig::default();
        config.disable_rule("E_TEST_ALWAYS_FAIL");

        let mut linter = Linter::new(config);
        linter.register_rule(Arc::new(AlwaysFailRule));
        linter.register_rule(Arc::new(FileRule));

        assert_eq!(linter.rule_count(), 2);
        assert_eq!(linter.enabled_rule_count(), 1); // Only FileRule enabled
    }
}
