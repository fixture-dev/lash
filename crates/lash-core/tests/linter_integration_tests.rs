//! Integration tests for the linter infrastructure
//!
//! These tests verify that the linter components work together correctly.

use lash_core::linter::{
    Fix, LintConfig, LintContext, LintDiagnostic, LintRule, Linter, RuleCategory, RuleRegistry,
};
use lash_types::{
    FileMetadata, LashConfig, Severity, Task, TaskFile, TaskMetadata, TaskStatus, TaskTree,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

// ===== Test Helper Functions =====

fn make_test_config() -> LashConfig {
    LashConfig {
        root_path: PathBuf::from("/test"),
        index_file: "lash.index.md".to_string(),
        max_depth: 2,
        indent_spaces: 2,
        db_path: PathBuf::from(".lash/lash.db"),
        custom_annotation_keys: vec!["custom".to_string()],
    }
}

fn make_test_file(path: &str, tasks: Vec<Task>) -> TaskFile {
    let mut tree = TaskTree::new();
    for task in tasks {
        tree.add_task(task).unwrap();
    }

    TaskFile {
        path: PathBuf::from(path),
        title: "Test File".to_string(),
        id: "test".to_string(),
        metadata: FileMetadata::default(),
        description: None,
        description_agent_notes: Vec::new(),
        tasks: tree,
        hash: "test-hash".to_string(),
        mtime: SystemTime::now(),
    }
}

fn make_task(id: &str, title: &str, depth: u8) -> Task {
    Task {
        id: id.to_string(),
        title: title.to_string(),
        status: TaskStatus::Open,
        depth,
        parent_id: None,
        order_index: 0,
        line_number: 0,
        metadata: TaskMetadata::default(),
        body: None,
        contextual_notes: Vec::new(),
    }
}

// ===== Test Rules =====

/// Rule that checks task depth
struct DepthRule {
    max_depth: u8,
}

impl LintRule for DepthRule {
    fn code(&self) -> &'static str {
        "E_SYNTAX_DEPTH"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        if task.depth > self.max_depth {
            vec![LintDiagnostic::error(
                self.code(),
                format!(
                    "Task depth {} exceeds maximum {}",
                    task.depth, self.max_depth
                ),
                ctx.file_path.clone(),
                1,
                1,
            )
            .with_help(format!(
                "Reduce nesting to {} levels or fewer",
                self.max_depth
            ))]
        } else {
            Vec::new()
        }
    }

    fn description(&self) -> &'static str {
        "Enforces maximum task nesting depth"
    }
}

/// Rule that provides auto-fixes
struct AutoFixRule;

impl LintRule for AutoFixRule {
    fn code(&self) -> &'static str {
        "W_SYNTAX_FIXABLE"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        if task.title.contains("FIXME") {
            vec![LintDiagnostic::warning(
                self.code(),
                "Task contains FIXME marker".to_string(),
                ctx.file_path.clone(),
                1,
                1,
            )
            .with_fix(Fix::replace("Remove FIXME marker", "FIXME: ", ""))]
        } else {
            Vec::new()
        }
    }

    fn description(&self) -> &'static str {
        "Detects and removes FIXME markers"
    }
}

/// File-level rule
struct FileHeaderRule;

impl LintRule for FileHeaderRule {
    fn code(&self) -> &'static str {
        "W_SYNTAX_HEADER"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        if file.title.is_empty() {
            vec![LintDiagnostic::warning(
                self.code(),
                "File has no title".to_string(),
                ctx.file_path.clone(),
                1,
                1,
            )
            .with_help("Add an H1 title to the file")]
        } else {
            Vec::new()
        }
    }

    fn description(&self) -> &'static str {
        "Validates file header structure"
    }
}

// ===== Integration Tests =====

#[test]
fn test_linter_with_single_rule() {
    let config = LintConfig::default();
    let mut linter = Linter::new(config);

    linter.register_rule(Arc::new(DepthRule { max_depth: 2 }));

    let tasks = vec![
        make_task("task1", "Task 1", 0),
        make_task("task2", "Task 2", 1),
        make_task("task3", "Task 3", 3), // Too deep
    ];

    let file = make_test_file("test.md", tasks);
    let project_config = make_test_config();

    let diagnostics = linter.lint_file(&file, &project_config);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E_SYNTAX_DEPTH");
    assert_eq!(diagnostics[0].severity, Severity::Error);
    assert!(diagnostics[0].help.is_some());
}

#[test]
fn test_linter_with_multiple_rules() {
    let config = LintConfig::default();
    let mut linter = Linter::new(config);

    linter.register_rule(Arc::new(DepthRule { max_depth: 2 }));
    linter.register_rule(Arc::new(AutoFixRule));

    let tasks = vec![
        make_task("task1", "FIXME: Fix this", 0),
        make_task("task2", "Task 2", 3), // Too deep
    ];

    let file = make_test_file("test.md", tasks);
    let project_config = make_test_config();

    let diagnostics = linter.lint_file(&file, &project_config);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|d| d.code == "W_SYNTAX_FIXABLE"));
    assert!(diagnostics.iter().any(|d| d.code == "E_SYNTAX_DEPTH"));
}

#[test]
fn test_file_level_rule() {
    let config = LintConfig::default();
    let mut linter = Linter::new(config);

    linter.register_rule(Arc::new(FileHeaderRule));

    let file = make_test_file("test.md", vec![]);
    let project_config = make_test_config();

    let diagnostics = linter.lint_file(&file, &project_config);

    // FileHeaderRule won't trigger because make_test_file sets a title
    assert_eq!(diagnostics.len(), 0);
}

#[test]
fn test_rule_disabling() {
    let mut config = LintConfig::default();
    config.disable_rule("E_SYNTAX_DEPTH");

    let mut linter = Linter::new(config);
    linter.register_rule(Arc::new(DepthRule { max_depth: 2 }));

    let tasks = vec![make_task("task1", "Task 1", 5)]; // Way too deep

    let file = make_test_file("test.md", tasks);
    let project_config = make_test_config();

    let diagnostics = linter.lint_file(&file, &project_config);

    // Rule is disabled, so no diagnostics
    assert_eq!(diagnostics.len(), 0);
}

#[test]
fn test_severity_override() {
    let mut config = LintConfig::default();
    config.set_severity("E_SYNTAX_DEPTH", Severity::Warning);

    let mut linter = Linter::new(config);
    linter.register_rule(Arc::new(DepthRule { max_depth: 2 }));

    let tasks = vec![make_task("task1", "Task 1", 3)];

    let file = make_test_file("test.md", tasks);
    let project_config = make_test_config();

    let diagnostics = linter.lint_file(&file, &project_config);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Warning); // Overridden from Error
}

#[test]
fn test_auto_fix_detection() {
    let config = LintConfig::default();
    let mut linter = Linter::new(config);

    linter.register_rule(Arc::new(AutoFixRule));

    let tasks = vec![make_task("task1", "FIXME: Fix this", 0)];

    let file = make_test_file("test.md", tasks);
    let project_config = make_test_config();

    let diagnostics = linter.lint_file(&file, &project_config);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].has_fix());
    assert_eq!(
        diagnostics[0].fix.as_ref().unwrap().description,
        "Remove FIXME marker"
    );
}

#[test]
fn test_lint_multiple_files() {
    let config = LintConfig::default();
    let mut linter = Linter::new(config);

    linter.register_rule(Arc::new(DepthRule { max_depth: 2 }));

    let mut files = HashMap::new();
    files.insert(
        PathBuf::from("file1.md"),
        make_test_file("file1.md", vec![make_task("task1", "Task 1", 3)]),
    );
    files.insert(
        PathBuf::from("file2.md"),
        make_test_file("file2.md", vec![make_task("task2", "Task 2", 3)]),
    );

    let project_config = make_test_config();
    let diagnostics = linter.lint_project(&files, &project_config);

    assert_eq!(diagnostics.len(), 2); // One error per file
}

#[test]
fn test_diagnostic_ordering() {
    let config = LintConfig::default();
    let mut linter = Linter::new(config);

    linter.register_rule(Arc::new(DepthRule { max_depth: 1 }));

    let tasks = vec![
        make_task("task1", "Task 1", 2),
        make_task("task2", "Task 2", 2),
        make_task("task3", "Task 3", 2),
    ];

    let file = make_test_file("test.md", tasks);
    let project_config = make_test_config();

    let diagnostics = linter.lint_file(&file, &project_config);

    // All diagnostics should be sorted by location
    assert_eq!(diagnostics.len(), 3);
    for window in diagnostics.windows(2) {
        assert!(window[0] <= window[1]);
    }
}

#[test]
fn test_rule_registry() {
    let mut registry = RuleRegistry::new();

    registry.register(RuleCategory::Syntax, Arc::new(DepthRule { max_depth: 2 }));
    registry.register(RuleCategory::Syntax, Arc::new(AutoFixRule));
    registry.register(RuleCategory::Semantic, Arc::new(FileHeaderRule));

    assert_eq!(registry.rule_count(), 3);
    assert_eq!(registry.category_count(RuleCategory::Syntax), 2);
    assert_eq!(registry.category_count(RuleCategory::Semantic), 1);

    let linter = registry.create_linter(LintConfig::default());
    assert_eq!(linter.rule_count(), 3);

    let syntax_linter = registry.create_syntax_linter(LintConfig::default());
    assert_eq!(syntax_linter.rule_count(), 2);
}

#[test]
fn test_context_provides_config() {
    let project_config = make_test_config();
    let files = HashMap::new();
    let ctx = LintContext::new(&project_config, PathBuf::from("test.md"), &files);

    assert_eq!(ctx.max_depth(), 2);
    assert_eq!(ctx.indent_spaces(), 2);
    assert!(ctx.is_annotation_allowed("id"));
    assert!(ctx.is_annotation_allowed("custom"));
    assert!(!ctx.is_annotation_allowed("unknown"));
}

#[test]
fn test_fix_application() {
    let fix = Fix::replace("fix it", "bad", "good");
    let content = "this is bad content";
    let result = fix.replacement.apply(content).unwrap();
    assert_eq!(result, "this is good content");
}

#[test]
fn test_diagnostic_json_serialization() {
    let diag = LintDiagnostic::error("E_TEST", "test error", PathBuf::from("test.md"), 10, 5)
        .with_help("try this");

    let json = diag.to_json().unwrap();
    assert!(json.contains("E_TEST"));
    assert!(json.contains("test error"));
    assert!(json.contains("try this"));
    assert!(json.contains("test.md"));
}

#[test]
fn test_enabled_rule_count() {
    let mut config = LintConfig::default();
    config.disable_rule("E_SYNTAX_DEPTH");

    let mut linter = Linter::new(config);
    linter.register_rule(Arc::new(DepthRule { max_depth: 2 }));
    linter.register_rule(Arc::new(AutoFixRule));

    assert_eq!(linter.rule_count(), 2);
    assert_eq!(linter.enabled_rule_count(), 1); // Only AutoFixRule enabled
}

#[test]
fn test_rule_codes() {
    let config = LintConfig::default();
    let mut linter = Linter::new(config);

    linter.register_rule(Arc::new(DepthRule { max_depth: 2 }));
    linter.register_rule(Arc::new(AutoFixRule));

    let codes = linter.rule_codes();
    assert_eq!(codes.len(), 2);
    assert!(codes.contains(&"E_SYNTAX_DEPTH"));
    assert!(codes.contains(&"W_SYNTAX_FIXABLE"));
}
