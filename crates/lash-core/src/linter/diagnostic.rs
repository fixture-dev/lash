//! Diagnostic structure for linter issues
//!
//! Diagnostics extend the error-handling infrastructure to support linting-specific
//! features like auto-fixes, multiple locations, and rich formatting.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use lash_types::{Location, Severity};

use crate::linter::Fix;

/// Diagnostic reported by a linting rule
///
/// Diagnostics are the primary output of the linter. They describe issues found
/// in task files and optionally provide suggestions for fixing them.
///
/// Diagnostics can be:
/// - Serialized to JSON for machine consumption
/// - Formatted as rich text for human consumption
/// - Sorted by severity and location
/// - Filtered by rule code or severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LintDiagnostic {
    /// Stable rule code (e.g., `E_SYNTAX_DEPTH`)
    pub code: &'static str,

    /// Severity level
    pub severity: Severity,

    /// Human-readable error message
    pub message: String,

    /// Primary location where the issue occurred
    pub location: Location,

    /// Optional code snippet showing the error context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,

    /// Optional help text or suggestion for fixing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,

    /// Optional auto-fix suggestion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Fix>,

    /// Additional context labels (for multi-location errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<(String, String)>>,

    // === Agent-friendly fields ===
    /// Exact CLI command to run for automated recovery
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_command: Option<String>,

    /// Step-by-step instructions for manually fixing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_steps: Option<Vec<String>>,

    /// Detailed explanation of the error for agents/documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
}

impl LintDiagnostic {
    /// Create a new error diagnostic
    #[must_use]
    pub fn error(
        code: &'static str,
        message: impl Into<String>,
        file_path: PathBuf,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            location: Location::new(file_path, line, column),
            snippet: None,
            help: None,
            fix: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
        }
    }

    /// Create a new warning diagnostic
    #[must_use]
    pub fn warning(
        code: &'static str,
        message: impl Into<String>,
        file_path: PathBuf,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            location: Location::new(file_path, line, column),
            snippet: None,
            help: None,
            fix: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
        }
    }

    /// Create a new info diagnostic
    #[must_use]
    pub fn info(
        code: &'static str,
        message: impl Into<String>,
        file_path: PathBuf,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            code,
            severity: Severity::Info,
            message: message.into(),
            location: Location::new(file_path, line, column),
            snippet: None,
            help: None,
            fix: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
        }
    }

    /// Add a code snippet to this diagnostic
    #[must_use]
    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }

    /// Add help text to this diagnostic
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Add an auto-fix to this diagnostic
    #[must_use]
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }

    /// Add context labels to this diagnostic
    #[must_use]
    pub fn with_labels(mut self, labels: Vec<(String, String)>) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Add a span to the location
    #[must_use]
    pub fn with_span(mut self, start: usize, end: usize) -> Self {
        self.location = self.location.with_span(start, end);
        self
    }

    /// Add a recovery command for agents
    #[must_use]
    pub fn with_recovery_command(mut self, cmd: impl Into<String>) -> Self {
        self.recovery_command = Some(cmd.into());
        self
    }

    /// Add fix steps for agents
    #[must_use]
    pub fn with_fix_steps(mut self, steps: Vec<String>) -> Self {
        self.fix_steps = Some(steps);
        self
    }

    /// Add an explanation for agents
    #[must_use]
    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.explanation = Some(explanation.into());
        self
    }

    /// Enrich this diagnostic with agent-friendly context
    ///
    /// Adds recovery commands, fix steps, and explanations based on the error code.
    #[must_use]
    pub fn enriched(mut self) -> Self {
        use lash_types::error_explanations::explain_error;

        // Add explanation if not already present
        if self.explanation.is_none() {
            if let Some(exp) = explain_error(self.code) {
                self.explanation = Some(exp.description.to_string());
            }
        }

        // Add recovery command based on code type
        if self.recovery_command.is_none() {
            let path = self.location.file_path.display().to_string();
            if self.code.starts_with("E_PARSE") || self.code.starts_with("E_LINT") {
                self.recovery_command = Some(format!("lash format {path}"));
            }
        }

        // Add fix steps if we have an explanation
        if self.fix_steps.is_none() {
            if let Some(exp) = explain_error(self.code) {
                self.fix_steps = Some(vec![exp.how_to_fix.to_string()]);
            }
        }

        self
    }

    /// Convert to JSON string
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Check if this diagnostic has an auto-fix available
    #[must_use]
    pub fn has_fix(&self) -> bool {
        self.fix.is_some()
    }

    /// Check if this is an error-level diagnostic
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Check if this is a warning-level diagnostic
    #[must_use]
    pub fn is_warning(&self) -> bool {
        self.severity == Severity::Warning
    }
}

impl std::fmt::Display for LintDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{} {}[{}]: {}",
            self.location, self.severity, self.code, self.severity, self.message
        )?;

        if let Some(snippet) = &self.snippet {
            write!(f, "\n  snippet: {snippet}")?;
        }

        if let Some(help) = &self.help {
            write!(f, "\n  help: {help}")?;
        }

        if let Some(fix) = &self.fix {
            write!(f, "\n  fix: {}", fix.description)?;
        }

        if let Some(labels) = &self.labels {
            for (key, value) in labels {
                write!(f, "\n  {key}: {value}")?;
            }
        }

        Ok(())
    }
}

// Implement ordering for diagnostics (sort by location, then severity)
impl Ord for LintDiagnostic {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // First compare by file path
        match self.location.file_path.cmp(&other.location.file_path) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        // Then by line number
        match (self.location.line, other.location.line) {
            (Some(a), Some(b)) => match a.cmp(&b) {
                std::cmp::Ordering::Equal => {}
                ord => return ord,
            },
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, None) => {}
        }

        // Then by column
        match (self.location.column, other.location.column) {
            (Some(a), Some(b)) => match a.cmp(&b) {
                std::cmp::Ordering::Equal => {}
                ord => return ord,
            },
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, None) => {}
        }

        // Finally by severity (errors first)
        self.severity.cmp(&other.severity)
    }
}

impl PartialOrd for LintDiagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_diagnostic() {
        let diag = LintDiagnostic::error("E_TEST", "test error", PathBuf::from("test.md"), 10, 5);
        assert_eq!(diag.code, "E_TEST");
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.is_error());
        assert!(!diag.is_warning());
    }

    #[test]
    fn test_warning_diagnostic() {
        let diag =
            LintDiagnostic::warning("W_TEST", "test warning", PathBuf::from("test.md"), 10, 5);
        assert_eq!(diag.severity, Severity::Warning);
        assert!(!diag.is_error());
        assert!(diag.is_warning());
    }

    #[test]
    fn test_with_snippet() {
        let diag = LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1)
            .with_snippet("- [x] task");
        assert_eq!(diag.snippet, Some("- [x] task".to_string()));
    }

    #[test]
    fn test_with_help() {
        let diag = LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1)
            .with_help("try this instead");
        assert_eq!(diag.help, Some("try this instead".to_string()));
    }

    #[test]
    fn test_has_fix() {
        use crate::linter::Replacement;

        let fix = Fix {
            description: "fix it".to_string(),
            replacement: Replacement::TextReplace {
                old: "bad".to_string(),
                new: "good".to_string(),
            },
        };

        let diag =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix);
        assert!(diag.has_fix());
    }

    #[test]
    fn test_diagnostic_ordering() {
        let diag1 = LintDiagnostic::error("E_TEST", "test", PathBuf::from("a.md"), 10, 5);
        let diag2 = LintDiagnostic::error("E_TEST", "test", PathBuf::from("b.md"), 5, 3);
        let diag3 = LintDiagnostic::error("E_TEST", "test", PathBuf::from("a.md"), 20, 1);

        assert!(diag1 < diag2); // a.md < b.md
        assert!(diag1 < diag3); // line 10 < line 20
    }

    #[test]
    fn test_json_serialization() {
        let diag = LintDiagnostic::error("E_TEST", "test error", PathBuf::from("test.md"), 10, 5)
            .with_help("try this");

        let json = diag.to_json().unwrap();
        assert!(json.contains("E_TEST"));
        assert!(json.contains("test error"));
        assert!(json.contains("try this"));
    }
}
