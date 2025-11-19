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
/// ```rust,ignore
/// use lash_core::linter::{RuleRegistry, LintConfig};
///
/// let registry = RuleRegistry::new();
/// let linter = registry.create_linter(LintConfig::default());
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
/// This function will be implemented in Task #2 and #3 when individual
/// linting rules are created. For now, it returns an empty registry.
///
/// Once rules are implemented, this will register:
/// - 7 syntax rules
/// - 8 semantic rules
/// - 5 cross-file rules
#[must_use]
pub fn register_default_rules() -> RuleRegistry {
    // TODO: Register syntax rules (Task #2)
    // registry.register(RuleCategory::Syntax, Arc::new(ValidCheckboxRule::new()));
    // registry.register(RuleCategory::Syntax, Arc::new(ConsistentIndentRule::new()));
    // registry.register(RuleCategory::Syntax, Arc::new(DepthLimitRule::new()));
    // registry.register(RuleCategory::Syntax, Arc::new(ValidAnnotationSyntaxRule::new()));
    // registry.register(RuleCategory::Syntax, Arc::new(UnknownAnnotationRule::new()));
    // registry.register(RuleCategory::Syntax, Arc::new(HeaderStructureRule::new()));
    // registry.register(RuleCategory::Syntax, Arc::new(AnnotationOrderRule::new()));

    // TODO: Register semantic rules (Task #3)
    // registry.register(RuleCategory::Semantic, Arc::new(DuplicateIdRule::new()));
    // registry.register(RuleCategory::Semantic, Arc::new(StatusConsistencyRule::new()));
    // registry.register(RuleCategory::Semantic, Arc::new(AutoWaiveRule::new()));
    // registry.register(RuleCategory::Semantic, Arc::new(ValidLabelRule::new()));
    // registry.register(RuleCategory::Semantic, Arc::new(ValidDateRule::new()));
    // registry.register(RuleCategory::Semantic, Arc::new(ValidEstimateRule::new()));
    // registry.register(RuleCategory::Semantic, Arc::new(ValidOwnerRule::new()));
    // registry.register(RuleCategory::Semantic, Arc::new(EmptyTitleRule::new()));

    // TODO: Register cross-file rules (Task #4)
    // registry.register(RuleCategory::CrossFile, Arc::new(DependencyExistsRule::new()));
    // registry.register(RuleCategory::CrossFile, Arc::new(CircularDepsRule::new()));
    // registry.register(RuleCategory::CrossFile, Arc::new(IndexFileRefsRule::new()));
    // registry.register(RuleCategory::CrossFile, Arc::new(OrphanedFilesRule::new()));
    // registry.register(RuleCategory::CrossFile, Arc::new(ValidPathResolutionRule::new()));

    RuleRegistry::new()
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
        let registry = register_default_rules();
        // For now, should be empty until rules are implemented
        assert_eq!(registry.rule_count(), 0);
    }
}
