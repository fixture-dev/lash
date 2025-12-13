//! Rule registry for organizing and managing linting rules
//!
//! The registry provides a central place to organize rules by category
//! and create pre-configured linters with standard rule sets.

use std::sync::Arc;

use crate::linter::{LintConfig, LintRule, Linter};

/// Rule categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleCategory {
    /// Syntax and formatting rules (`E_SYNTAX_*`, `W_SYNTAX_*`, `I_SYNTAX_*`)
    Syntax,
    /// Semantic validation rules (`E_SEM_*`, `W_SEM_*`, `I_SEM_*`)
    Semantic,
    /// Cross-file validation rules (`E_LINK_*`, `W_INDEX_*`)
    CrossFile,
}

/// Registry for linting rules
///
/// The registry organizes rules by category and provides methods to create
/// pre-configured linters. This makes it easy to:
/// - Enable only syntax rules for fast checks
/// - Run all rules for comprehensive validation
/// - Create custom rule sets for specific use cases
///
/// # Example
///
/// ```
/// use lash_core::linter::{RuleRegistry, LintConfig};
///
/// let registry = RuleRegistry::new();
/// let linter = registry.create_linter(LintConfig::default());
/// assert_eq!(linter.rule_count(), 0);
/// ```
pub struct RuleRegistry {
    /// All registered rules
    rules: Vec<(RuleCategory, Arc<dyn LintRule>)>,
}

impl RuleRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Register a rule in a specific category
    pub fn register(&mut self, category: RuleCategory, rule: Arc<dyn LintRule>) {
        self.rules.push((category, rule));
    }

    /// Get all rules in a specific category
    #[must_use]
    pub fn rules_in_category(&self, category: RuleCategory) -> Vec<Arc<dyn LintRule>> {
        self.rules
            .iter()
            .filter(|(cat, _)| *cat == category)
            .map(|(_, rule)| Arc::clone(rule))
            .collect()
    }

    /// Get all rules
    #[must_use]
    pub fn all_rules(&self) -> Vec<Arc<dyn LintRule>> {
        self.rules
            .iter()
            .map(|(_, rule)| Arc::clone(rule))
            .collect()
    }

    /// Create a linter with all registered rules
    #[must_use]
    pub fn create_linter(&self, config: LintConfig) -> Linter {
        let mut linter = Linter::new(config);
        linter.register_rules(self.all_rules());
        linter
    }

    /// Create a linter with only syntax rules
    #[must_use]
    pub fn create_syntax_linter(&self, config: LintConfig) -> Linter {
        let mut linter = Linter::new(config);
        linter.register_rules(self.rules_in_category(RuleCategory::Syntax));
        linter
    }

    /// Create a linter with syntax and semantic rules (no cross-file validation)
    #[must_use]
    pub fn create_single_file_linter(&self, config: LintConfig) -> Linter {
        let mut linter = Linter::new(config);
        linter.register_rules(self.rules_in_category(RuleCategory::Syntax));
        linter.register_rules(self.rules_in_category(RuleCategory::Semantic));
        linter
    }

    /// Get the number of registered rules
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Get the number of rules in a specific category
    #[must_use]
    pub fn category_count(&self, category: RuleCategory) -> usize {
        self.rules
            .iter()
            .filter(|(cat, _)| *cat == category)
            .count()
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Register all default built-in rules
///
/// Creates a registry with all built-in linting rules:
/// - 8 syntax rules (Task #2 - Complete)
/// - 15 semantic rules (Task #3 - Complete, plus contextual notes rules)
/// - 5 cross-file rules (Task #4 - Complete)
///
/// # Arguments
///
/// * `config` - Optional configuration for customizing rule behavior.
///   If `None`, default values are used for all rules.
///
/// # Syntax Rules
///
/// 1. Valid Checkbox Pattern (`E_SYNTAX_CHECKBOX`)
/// 2. Consistent Indentation (`E_SYNTAX_INDENT`)
/// 3. Depth Limit (`E_SYNTAX_DEPTH`)
/// 4. Valid Annotation Syntax (`E_SYNTAX_ANNOTATION`)
/// 5. Unknown Annotation Keys (`E_SYNTAX_UNKNOWN_KEY`)
/// 6. Header Structure (`W_SYNTAX_HEADER`)
/// 7. Annotation Ordering (`I_SYNTAX_ORDER`)
/// 8. Duplicate Description Section (`E_SYNTAX_DUPLICATE_DESCRIPTION`)
///
/// # Semantic Rules
///
/// 1. ID Uniqueness Within File (`E_SEM_DUPLICATE_ID`)
/// 2. Parent-Child Status Consistency (`W_SEM_STATUS_INCONSISTENT`)
/// 3. Auto-Waive Children (`I_SEM_AUTO_WAIVE`)
/// 4. Valid Label Format (`E_SEM_INVALID_LABEL`)
/// 5. Valid Date Format (`E_SEM_INVALID_DATE`)
/// 6. Valid Estimate Format (`E_SEM_INVALID_ESTIMATE`)
/// 7. Valid Owner Format (`W_SEM_OWNER_FORMAT`)
/// 8. Empty Task Title (`E_SEM_EMPTY_TITLE`)
/// 9. Description Length Limit (`W_SEM_DESC_TOO_LONG`, `E_SEM_DESC_TOO_LONG`)
/// 10. Valid Documentation Reference (`E_SEM_INVALID_DOC`)
/// 11. Broken Documentation Fragment (`W_SEM_DOC_FRAGMENT`)
/// 12. Note Indentation (`E_NOTE_INVALID_INDENT`)
/// 13. Note Length (`W_NOTE_TOO_LONG`, `E_NOTE_EXCESSIVE_LENGTH`)
/// 14. Note Nesting (`E_NOTE_HAS_CHILDREN`)
/// 15. Note Ordering (`W_NOTE_AFTER_CHILD_TASKS`)
///
/// # Cross-File Rules
///
/// 1. Dependency Reference Exists (`E_LINK_NOT_FOUND`)
/// 2. Circular Dependencies (`E_LINK_CYCLE`)
/// 3. Root Index File References (`E_INDEX_FILE_MISSING`)
/// 4. Orphaned Files (`W_INDEX_ORPHAN`)
/// 5. Valid Dependency Path Resolution (`E_LINK_INVALID_PATH`)
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn register_default_rules(config: Option<&LintConfig>) -> RuleRegistry {
    use crate::linter::rules;

    let mut registry = RuleRegistry::new();

    // Register syntax rules (Task #2)
    registry.register(
        RuleCategory::Syntax,
        Arc::new(rules::CheckboxPatternRule::new()),
    );
    registry.register(
        RuleCategory::Syntax,
        Arc::new(rules::IndentationRule::new()),
    );
    registry.register(RuleCategory::Syntax, Arc::new(rules::DepthLimitRule::new()));
    registry.register(
        RuleCategory::Syntax,
        Arc::new(rules::AnnotationSyntaxRule::new()),
    );
    registry.register(
        RuleCategory::Syntax,
        Arc::new(rules::UnknownAnnotationRule::new()),
    );
    registry.register(
        RuleCategory::Syntax,
        Arc::new(rules::HeaderStructureRule::new()),
    );
    registry.register(
        RuleCategory::Syntax,
        Arc::new(rules::AnnotationOrderRule::new()),
    );
    registry.register(
        RuleCategory::Syntax,
        Arc::new(rules::DuplicateDescriptionRule::new()),
    );

    // Register semantic rules (Task #3)
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::DuplicateIdRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::StatusConsistencyRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::AutoWaiveRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::ValidLabelRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::ValidDateRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::ValidEstimateRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::ValidOwnerRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::EmptyTitleRule::new()),
    );

    // Configure DescriptionLengthRule with custom threshold if provided
    let description_rule = if let Some(cfg) = config {
        let max_len = cfg.description_max_length();
        // Error threshold is always 2x the warning threshold
        rules::DescriptionLengthRule::with_thresholds(max_len, max_len * 2)
    } else {
        rules::DescriptionLengthRule::new()
    };
    registry.register(RuleCategory::Semantic, Arc::new(description_rule));

    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::ValidDocReferenceRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::BrokenDocFragmentRule::new()),
    );

    // Register contextual notes rules
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::NoteIndentationRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::NoteLengthRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::NoteNestingRule::new()),
    );
    registry.register(
        RuleCategory::Semantic,
        Arc::new(rules::NoteOrderingRule::new()),
    );

    // Register cross-file rules (Task #4)
    registry.register(
        RuleCategory::CrossFile,
        Arc::new(rules::DependencyExistsRule::new()),
    );
    registry.register(
        RuleCategory::CrossFile,
        Arc::new(rules::CircularDepsRule::new()),
    );
    registry.register(
        RuleCategory::CrossFile,
        Arc::new(rules::IndexFileRefsRule::new()),
    );
    registry.register(
        RuleCategory::CrossFile,
        Arc::new(rules::OrphanedFilesRule::new()),
    );
    registry.register(
        RuleCategory::CrossFile,
        Arc::new(rules::ValidPathResolutionRule::new()),
    );

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::Severity;

    // Test rule implementations
    struct TestSyntaxRule;
    impl LintRule for TestSyntaxRule {
        fn code(&self) -> &'static str {
            "E_SYNTAX_TEST"
        }
        fn severity(&self) -> Severity {
            Severity::Error
        }
    }

    struct TestSemanticRule;
    impl LintRule for TestSemanticRule {
        fn code(&self) -> &'static str {
            "E_SEM_TEST"
        }
        fn severity(&self) -> Severity {
            Severity::Error
        }
    }

    struct TestCrossFileRule;
    impl LintRule for TestCrossFileRule {
        fn code(&self) -> &'static str {
            "E_LINK_TEST"
        }
        fn severity(&self) -> Severity {
            Severity::Error
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = RuleRegistry::new();
        assert_eq!(registry.rule_count(), 0);
    }

    #[test]
    fn test_register_rules() {
        let mut registry = RuleRegistry::new();

        registry.register(RuleCategory::Syntax, Arc::new(TestSyntaxRule));
        registry.register(RuleCategory::Semantic, Arc::new(TestSemanticRule));
        registry.register(RuleCategory::CrossFile, Arc::new(TestCrossFileRule));

        assert_eq!(registry.rule_count(), 3);
        assert_eq!(registry.category_count(RuleCategory::Syntax), 1);
        assert_eq!(registry.category_count(RuleCategory::Semantic), 1);
        assert_eq!(registry.category_count(RuleCategory::CrossFile), 1);
    }

    #[test]
    fn test_rules_in_category() {
        let mut registry = RuleRegistry::new();

        registry.register(RuleCategory::Syntax, Arc::new(TestSyntaxRule));
        registry.register(RuleCategory::Syntax, Arc::new(TestSyntaxRule));
        registry.register(RuleCategory::Semantic, Arc::new(TestSemanticRule));

        let syntax_rules = registry.rules_in_category(RuleCategory::Syntax);
        assert_eq!(syntax_rules.len(), 2);

        let semantic_rules = registry.rules_in_category(RuleCategory::Semantic);
        assert_eq!(semantic_rules.len(), 1);
    }

    #[test]
    fn test_create_linter() {
        let mut registry = RuleRegistry::new();
        registry.register(RuleCategory::Syntax, Arc::new(TestSyntaxRule));
        registry.register(RuleCategory::Semantic, Arc::new(TestSemanticRule));

        let linter = registry.create_linter(LintConfig::default());
        assert_eq!(linter.rule_count(), 2);
    }

    #[test]
    fn test_create_syntax_linter() {
        let mut registry = RuleRegistry::new();
        registry.register(RuleCategory::Syntax, Arc::new(TestSyntaxRule));
        registry.register(RuleCategory::Semantic, Arc::new(TestSemanticRule));

        let linter = registry.create_syntax_linter(LintConfig::default());
        assert_eq!(linter.rule_count(), 1);
    }

    #[test]
    fn test_create_single_file_linter() {
        let mut registry = RuleRegistry::new();
        registry.register(RuleCategory::Syntax, Arc::new(TestSyntaxRule));
        registry.register(RuleCategory::Semantic, Arc::new(TestSemanticRule));
        registry.register(RuleCategory::CrossFile, Arc::new(TestCrossFileRule));

        let linter = registry.create_single_file_linter(LintConfig::default());
        assert_eq!(linter.rule_count(), 2); // Syntax + Semantic, no CrossFile
    }

    #[test]
    fn test_default_registry() {
        let registry = register_default_rules(None);
        // Should have 8 syntax rules + 15 semantic rules (11 base + 4 contextual notes) + 5 cross-file rules
        assert_eq!(registry.rule_count(), 28);
        assert_eq!(registry.category_count(RuleCategory::Syntax), 8);
        assert_eq!(registry.category_count(RuleCategory::Semantic), 15);
        assert_eq!(registry.category_count(RuleCategory::CrossFile), 5);
    }

    #[test]
    fn test_registry_with_custom_description_length() {
        let config = LintConfig {
            description_max_length: Some(1500),
            ..Default::default()
        };

        let registry = register_default_rules(Some(&config));
        assert_eq!(registry.rule_count(), 28);

        // Create linter and verify it was configured properly
        let linter = registry.create_linter(config.clone());
        assert_eq!(linter.rule_count(), 28);
    }
}
