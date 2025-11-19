//! Configuration for the linter
//!
//! The `LintConfig` controls which rules are enabled, severity overrides,
//! and auto-fix behavior.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use lash_types::Severity;

/// Configuration for the linter
///
/// Controls which rules run, their severity levels, and auto-fix behavior.
/// Can be loaded from `.lash/config.toml` under the `[linter]` section.
///
/// # Example TOML
///
/// ```toml
/// [linter]
/// auto_fix = true
/// enabled_rules = ["E_SYNTAX_DEPTH", "E_SEM_DUPLICATE_ID"]
/// disabled_rules = ["I_SYNTAX_ORDER"]
///
/// [linter.severity_overrides]
/// "W_SEM_STATUS_INCONSISTENT" = "error"
/// "E_SYNTAX_ANNOTATION" = "warning"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct LintConfig {
    /// Which rules to run
    ///
    /// If empty (default), all rules are enabled. If non-empty, only these
    /// rules will be executed.
    #[serde(default)]
    pub enabled_rules: HashSet<String>,

    /// Which rules to skip
    ///
    /// Rules in this set will not be executed, even if they appear in
    /// `enabled_rules` or are enabled by default.
    #[serde(default)]
    pub disabled_rules: HashSet<String>,

    /// Override default severity for specific rules
    ///
    /// Maps rule codes to custom severity levels. This allows you to
    /// promote warnings to errors or demote errors to warnings based
    /// on your project's needs.
    #[serde(default)]
    pub severity_overrides: HashMap<String, Severity>,

    /// Apply auto-fixes automatically
    ///
    /// If true, the linter will attempt to apply fixes for all diagnostics
    /// that have fix suggestions. If false, fixes are only reported, not applied.
    #[serde(default)]
    pub auto_fix: bool,
}

impl LintConfig {
    /// Create a new default config
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a rule is enabled
    ///
    /// A rule is enabled if:
    /// - It's not in `disabled_rules`, AND
    /// - Either `enabled_rules` is empty (all enabled) OR it's in `enabled_rules`
    #[must_use]
    pub fn is_rule_enabled(&self, code: &str) -> bool {
        // Check if explicitly disabled
        if self.disabled_rules.contains(code) {
            return false;
        }

        // If enabled_rules is empty, all rules are enabled by default
        if self.enabled_rules.is_empty() {
            return true;
        }

        // Otherwise, check if this rule is in the enabled set
        self.enabled_rules.contains(code)
    }

    /// Get the effective severity for a rule
    ///
    /// Returns the overridden severity if one exists, otherwise the default severity.
    #[must_use]
    pub fn get_severity(&self, code: &str, default: Severity) -> Severity {
        self.severity_overrides
            .get(code)
            .copied()
            .unwrap_or(default)
    }

    /// Enable a specific rule
    pub fn enable_rule(&mut self, code: impl Into<String>) {
        let code = code.into();
        self.disabled_rules.remove(&code);
        self.enabled_rules.insert(code);
    }

    /// Disable a specific rule
    pub fn disable_rule(&mut self, code: impl Into<String>) {
        let code = code.into();
        self.enabled_rules.remove(&code);
        self.disabled_rules.insert(code);
    }

    /// Override severity for a rule
    pub fn set_severity(&mut self, code: impl Into<String>, severity: Severity) {
        self.severity_overrides.insert(code.into(), severity);
    }

    /// Enable auto-fix
    #[must_use]
    pub fn with_auto_fix(mut self, enabled: bool) -> Self {
        self.auto_fix = enabled;
        self
    }

    /// Create a config with only specific rules enabled
    #[must_use]
    pub fn with_only_rules(rules: &[&str]) -> Self {
        Self {
            enabled_rules: rules.iter().map(|s| (*s).to_string()).collect(),
            disabled_rules: HashSet::new(),
            severity_overrides: HashMap::new(),
            auto_fix: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LintConfig::default();
        assert!(config.enabled_rules.is_empty());
        assert!(config.disabled_rules.is_empty());
        assert!(config.severity_overrides.is_empty());
        assert!(!config.auto_fix);
    }

    #[test]
    fn test_all_rules_enabled_by_default() {
        let config = LintConfig::default();
        assert!(config.is_rule_enabled("E_SYNTAX_DEPTH"));
        assert!(config.is_rule_enabled("E_SEM_DUPLICATE_ID"));
        assert!(config.is_rule_enabled("W_SYNTAX_HEADER"));
    }

    #[test]
    fn test_disable_rule() {
        let mut config = LintConfig::default();
        config.disable_rule("E_SYNTAX_DEPTH");

        assert!(!config.is_rule_enabled("E_SYNTAX_DEPTH"));
        assert!(config.is_rule_enabled("E_SEM_DUPLICATE_ID"));
    }

    #[test]
    fn test_enable_specific_rules() {
        let mut config = LintConfig::default();
        config.enable_rule("E_SYNTAX_DEPTH");
        config.enable_rule("E_SEM_DUPLICATE_ID");

        assert!(config.is_rule_enabled("E_SYNTAX_DEPTH"));
        assert!(config.is_rule_enabled("E_SEM_DUPLICATE_ID"));
        assert!(!config.is_rule_enabled("W_SYNTAX_HEADER"));
    }

    #[test]
    fn test_disabled_overrides_enabled() {
        let mut config = LintConfig::default();
        config.enable_rule("E_SYNTAX_DEPTH");
        config.disable_rule("E_SYNTAX_DEPTH");

        assert!(!config.is_rule_enabled("E_SYNTAX_DEPTH"));
    }

    #[test]
    fn test_severity_override() {
        let mut config = LintConfig::default();
        config.set_severity("W_SYNTAX_HEADER", Severity::Error);

        let severity = config.get_severity("W_SYNTAX_HEADER", Severity::Warning);
        assert_eq!(severity, Severity::Error);

        let severity = config.get_severity("E_SYNTAX_DEPTH", Severity::Error);
        assert_eq!(severity, Severity::Error); // No override, use default
    }

    #[test]
    fn test_with_auto_fix() {
        let config = LintConfig::default().with_auto_fix(true);
        assert!(config.auto_fix);
    }

    #[test]
    fn test_with_only_rules() {
        let config = LintConfig::with_only_rules(&["E_SYNTAX_DEPTH", "E_SEM_DUPLICATE_ID"]);

        assert!(config.is_rule_enabled("E_SYNTAX_DEPTH"));
        assert!(config.is_rule_enabled("E_SEM_DUPLICATE_ID"));
        assert!(!config.is_rule_enabled("W_SYNTAX_HEADER"));
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut config = LintConfig::default();
        config.enable_rule("E_SYNTAX_DEPTH");
        config.disable_rule("W_SYNTAX_HEADER");
        config.set_severity("E_SEM_DUPLICATE_ID", Severity::Warning);
        config.auto_fix = true;

        let toml = toml::to_string(&config).unwrap();
        let deserialized: LintConfig = toml::from_str(&toml).unwrap();

        assert_eq!(config, deserialized);
    }
}
