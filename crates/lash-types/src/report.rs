//! Error aggregation and reporting
//!
//! This module provides utilities for collecting, grouping, and reporting
//! multiple errors from batch operations like linting and indexing.
//!
//! # Examples
//!
//! ```
//! use lash_types::error::LashError;
//! use lash_types::report::{ErrorReport, GroupBy};
//! use std::path::PathBuf;
//!
//! let mut report = ErrorReport::new();
//!
//! // Collect errors
//! report.add(LashError::parse_invalid_checkbox(
//!     PathBuf::from("tasks.md"),
//!     5,
//!     3,
//!     "[*] bad"
//! ));
//! report.add(LashError::lint_duplicate_id(
//!     PathBuf::from("tasks.md"),
//!     10,
//!     3,
//!     "setup",
//!     5
//! ));
//!
//! // Get summary
//! let summary = report.summary();
//! println!("Found {} errors in {} files", summary.total_errors, summary.files_affected);
//!
//! // Format report
//! let formatted = report.format_text(GroupBy::File, false);
//! println!("{}", formatted);
//! ```

use crate::error::{Diagnostic, LashError, Severity};
use crate::formatter::ErrorFormatter;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// Strategy for grouping errors in reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBy {
    /// Group errors by source file
    File,
    /// Group errors by error code
    ErrorCode,
    /// Group errors by severity level
    Severity,
    /// No grouping (chronological order)
    None,
}

/// Report containing multiple errors
#[derive(Debug, Clone, Default)]
pub struct ErrorReport {
    /// List of errors in the order they were added
    errors: Vec<LashError>,
}

impl ErrorReport {
    /// Create a new empty error report
    #[must_use]
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add an error to the report
    pub fn add(&mut self, error: LashError) {
        self.errors.push(error);
    }

    /// Add multiple errors to the report
    pub fn add_many(&mut self, errors: Vec<LashError>) {
        self.errors.extend(errors);
    }

    /// Check if the report is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of errors in the report
    #[must_use]
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Get all errors
    #[must_use]
    pub fn errors(&self) -> &[LashError] {
        &self.errors
    }

    /// Filter errors by severity
    #[must_use]
    pub fn filter_by_severity(&self, severity: Severity) -> Vec<&LashError> {
        self.errors
            .iter()
            .filter(|e| e.to_diagnostic().severity == severity)
            .collect()
    }

    /// Filter errors by error code
    #[must_use]
    pub fn filter_by_code(&self, code: &str) -> Vec<&LashError> {
        self.errors.iter().filter(|e| e.code() == code).collect()
    }

    /// Filter errors by file path
    #[must_use]
    pub fn filter_by_file(&self, path: &PathBuf) -> Vec<&LashError> {
        self.errors
            .iter()
            .filter(|e| {
                let diag = e.to_diagnostic();
                diag.location.as_ref().is_some_and(|l| &l.file_path == path)
            })
            .collect()
    }

    /// Get summary statistics
    #[must_use]
    pub fn summary(&self) -> ReportSummary {
        let mut by_severity = HashMap::new();
        let mut by_code = HashMap::new();
        let mut files = HashMap::new();

        for error in &self.errors {
            let diag = error.to_diagnostic();

            // Count by severity
            *by_severity.entry(diag.severity).or_insert(0) += 1;

            // Count by error code
            *by_code.entry(diag.code).or_insert(0) += 1;

            // Track affected files
            if let Some(location) = &diag.location {
                *files.entry(location.file_path.clone()).or_insert(0) += 1;
            }
        }

        ReportSummary {
            total_errors: self.errors.len(),
            errors_by_severity: by_severity,
            errors_by_code: by_code,
            files_affected: files.len(),
            errors_by_file: files,
        }
    }

    /// Format the report as human-readable text
    ///
    /// # Arguments
    ///
    /// * `group_by` - Grouping strategy
    /// * `use_color` - Whether to use ANSI color codes
    #[must_use]
    pub fn format_text(&self, group_by: GroupBy, use_color: bool) -> String {
        if self.errors.is_empty() {
            return "No errors found.".to_string();
        }

        let mut output = String::new();

        // Format errors based on grouping strategy
        match group_by {
            GroupBy::File => {
                output.push_str(&self.format_by_file(use_color));
            }
            GroupBy::ErrorCode => {
                output.push_str(&self.format_by_code(use_color));
            }
            GroupBy::Severity => {
                output.push_str(&self.format_by_severity(use_color));
            }
            GroupBy::None => {
                output.push_str(&self.format_chronological(use_color));
            }
        }

        // Add summary at the end
        output.push('\n');
        output.push_str(&self.format_summary(use_color));

        output
    }

    /// Format the report as JSON
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails
    pub fn format_json(&self) -> serde_json::Result<String> {
        let diagnostics: Vec<Diagnostic> =
            self.errors.iter().map(LashError::to_diagnostic).collect();

        let summary = self.summary();

        let report = JsonReport {
            errors: diagnostics,
            summary: JsonSummary {
                total: summary.total_errors,
                by_severity: summary.errors_by_severity,
                by_code: summary
                    .errors_by_code
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), *v))
                    .collect(),
                files_affected: summary.files_affected,
            },
        };

        serde_json::to_string_pretty(&report)
    }

    // Private formatting methods

    fn format_by_file(&self, use_color: bool) -> String {
        let mut output = String::new();
        let mut by_file: BTreeMap<String, Vec<&LashError>> = BTreeMap::new();
        let mut no_location = Vec::new();

        // Group errors by file
        for error in &self.errors {
            let diag = error.to_diagnostic();
            if let Some(location) = &diag.location {
                by_file
                    .entry(location.file_path.display().to_string())
                    .or_default()
                    .push(error);
            } else {
                no_location.push(error);
            }
        }

        // Format each file's errors
        for (file, errors) in by_file {
            output.push_str(&format!("\n{} ({} errors):\n", file, errors.len()));
            for error in errors {
                let diag = error.to_diagnostic();
                output.push_str(&ErrorFormatter::format_human(&diag, use_color));
                output.push('\n');
            }
        }

        // Format errors without location
        if !no_location.is_empty() {
            output.push_str(&format!("\nOther errors ({}):\n", no_location.len()));
            for error in no_location {
                let diag = error.to_diagnostic();
                output.push_str(&ErrorFormatter::format_human(&diag, use_color));
                output.push('\n');
            }
        }

        output
    }

    fn format_by_code(&self, use_color: bool) -> String {
        let mut output = String::new();
        let mut by_code: BTreeMap<&'static str, Vec<&LashError>> = BTreeMap::new();

        // Group errors by code
        for error in &self.errors {
            by_code.entry(error.code()).or_default().push(error);
        }

        // Format each code's errors
        for (code, errors) in by_code {
            output.push_str(&format!("\n{} ({} occurrences):\n", code, errors.len()));
            for error in errors {
                let diag = error.to_diagnostic();
                output.push_str(&ErrorFormatter::format_human(&diag, use_color));
                output.push('\n');
            }
        }

        output
    }

    fn format_by_severity(&self, use_color: bool) -> String {
        let mut output = String::new();
        let severities = [
            Severity::Error,
            Severity::Warning,
            Severity::Info,
            Severity::Hint,
        ];

        for severity in &severities {
            let errors = self.filter_by_severity(*severity);
            if !errors.is_empty() {
                output.push_str(&format!("\n{} ({}):\n", severity, errors.len()));
                for error in errors {
                    let diag = error.to_diagnostic();
                    output.push_str(&ErrorFormatter::format_human(&diag, use_color));
                    output.push('\n');
                }
            }
        }

        output
    }

    fn format_chronological(&self, use_color: bool) -> String {
        let mut output = String::new();
        for error in &self.errors {
            let diag = error.to_diagnostic();
            output.push_str(&ErrorFormatter::format_human(&diag, use_color));
            output.push('\n');
        }
        output
    }

    fn format_summary(&self, _use_color: bool) -> String {
        let summary = self.summary();
        let mut output = String::new();

        output.push_str(&format!(
            "Found {} error{} in {} file{}:\n",
            summary.total_errors,
            if summary.total_errors == 1 { "" } else { "s" },
            summary.files_affected,
            if summary.files_affected == 1 { "" } else { "s" }
        ));

        // Breakdown by severity
        if !summary.errors_by_severity.is_empty() {
            for (severity, count) in &summary.errors_by_severity {
                output.push_str(&format!("  - {} {}\n", count, severity));
            }
        }

        output
    }
}

/// Summary statistics for an error report
#[derive(Debug, Clone)]
pub struct ReportSummary {
    /// Total number of errors
    pub total_errors: usize,
    /// Errors grouped by severity
    pub errors_by_severity: HashMap<Severity, usize>,
    /// Errors grouped by error code
    pub errors_by_code: HashMap<&'static str, usize>,
    /// Number of unique files affected
    pub files_affected: usize,
    /// Errors grouped by file
    pub errors_by_file: HashMap<PathBuf, usize>,
}

/// JSON representation of error report
#[derive(Debug, Serialize)]
struct JsonReport {
    errors: Vec<Diagnostic>,
    summary: JsonSummary,
}

/// JSON representation of report summary
#[derive(Debug, Serialize)]
struct JsonSummary {
    total: usize,
    by_severity: HashMap<Severity, usize>,
    by_code: HashMap<String, usize>,
    files_affected: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::codes;

    #[test]
    fn test_empty_report() {
        let report = ErrorReport::new();
        assert!(report.is_empty());
        assert_eq!(report.len(), 0);

        let summary = report.summary();
        assert_eq!(summary.total_errors, 0);
        assert_eq!(summary.files_affected, 0);
    }

    #[test]
    fn test_add_errors() {
        let mut report = ErrorReport::new();

        report.add(LashError::parse_invalid_checkbox(
            PathBuf::from("test.md"),
            5,
            3,
            "[*] bad",
        ));

        report.add(LashError::lint_duplicate_id(
            PathBuf::from("test.md"),
            10,
            3,
            "id1",
            5,
        ));

        assert_eq!(report.len(), 2);
        assert!(!report.is_empty());
    }

    #[test]
    fn test_summary() {
        let mut report = ErrorReport::new();

        report.add(LashError::parse_invalid_checkbox(
            PathBuf::from("file1.md"),
            5,
            3,
            "[*]",
        ));

        report.add(LashError::lint_duplicate_id(
            PathBuf::from("file1.md"),
            10,
            3,
            "id",
            5,
        ));

        report.add(LashError::parse_invalid_header(
            PathBuf::from("file2.md"),
            1,
            "#Bad",
        ));

        let summary = report.summary();

        assert_eq!(summary.total_errors, 3);
        assert_eq!(summary.files_affected, 2);
        assert_eq!(summary.errors_by_severity.get(&Severity::Error), Some(&3));

        // Check code counts
        assert_eq!(
            summary.errors_by_code.get(codes::E_PARSE_INVALID_CHECKBOX),
            Some(&1)
        );
        assert_eq!(
            summary.errors_by_code.get(codes::E_LINT_DUPLICATE_ID),
            Some(&1)
        );
        assert_eq!(
            summary.errors_by_code.get(codes::E_PARSE_INVALID_HEADER),
            Some(&1)
        );
    }

    #[test]
    fn test_filter_by_severity() {
        let mut report = ErrorReport::new();

        report.add(LashError::parse_invalid_checkbox(
            PathBuf::from("test.md"),
            5,
            3,
            "[*]",
        ));

        let errors = report.filter_by_severity(Severity::Error);
        assert_eq!(errors.len(), 1);

        let warnings = report.filter_by_severity(Severity::Warning);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_filter_by_code() {
        let mut report = ErrorReport::new();

        report.add(LashError::parse_invalid_checkbox(
            PathBuf::from("test.md"),
            5,
            3,
            "[*]",
        ));

        report.add(LashError::parse_invalid_checkbox(
            PathBuf::from("test.md"),
            10,
            3,
            "[?]",
        ));

        report.add(LashError::lint_duplicate_id(
            PathBuf::from("test.md"),
            15,
            3,
            "id",
            10,
        ));

        let parse_errors = report.filter_by_code(codes::E_PARSE_INVALID_CHECKBOX);
        assert_eq!(parse_errors.len(), 2);

        let lint_errors = report.filter_by_code(codes::E_LINT_DUPLICATE_ID);
        assert_eq!(lint_errors.len(), 1);
    }

    #[test]
    fn test_filter_by_file() {
        let mut report = ErrorReport::new();
        let file1 = PathBuf::from("file1.md");
        let file2 = PathBuf::from("file2.md");

        report.add(LashError::parse_invalid_checkbox(
            file1.clone(),
            5,
            3,
            "[*]",
        ));

        report.add(LashError::lint_duplicate_id(file1.clone(), 10, 3, "id", 5));

        report.add(LashError::parse_invalid_header(file2.clone(), 1, "#Bad"));

        let file1_errors = report.filter_by_file(&file1);
        assert_eq!(file1_errors.len(), 2);

        let file2_errors = report.filter_by_file(&file2);
        assert_eq!(file2_errors.len(), 1);
    }

    #[test]
    fn test_format_text_chronological() {
        let mut report = ErrorReport::new();

        report.add(LashError::parse_invalid_checkbox(
            PathBuf::from("test.md"),
            5,
            3,
            "[*]",
        ));

        let formatted = report.format_text(GroupBy::None, false);
        assert!(formatted.contains("error"));
        assert!(formatted.contains("E_PARSE_INVALID_CHECKBOX"));
        assert!(formatted.contains("Found 1 error in"));
    }

    #[test]
    fn test_format_json() {
        let mut report = ErrorReport::new();

        report.add(LashError::parse_invalid_checkbox(
            PathBuf::from("test.md"),
            5,
            3,
            "[*]",
        ));

        let json = report.format_json().unwrap();
        assert!(json.contains("E_PARSE_INVALID_CHECKBOX"));
        assert!(json.contains("\"errors\""));
        assert!(json.contains("\"summary\""));

        // Validate it's actually valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["errors"].is_array());
        assert_eq!(parsed["summary"]["total"], 1);
    }

    #[test]
    fn test_format_by_file() {
        let mut report = ErrorReport::new();

        report.add(LashError::parse_invalid_checkbox(
            PathBuf::from("file1.md"),
            5,
            3,
            "[*]",
        ));

        report.add(LashError::lint_duplicate_id(
            PathBuf::from("file1.md"),
            10,
            3,
            "id",
            5,
        ));

        report.add(LashError::parse_invalid_header(
            PathBuf::from("file2.md"),
            1,
            "#Bad",
        ));

        let formatted = report.format_text(GroupBy::File, false);
        assert!(formatted.contains("file1.md (2 errors)"));
        assert!(formatted.contains("file2.md (1 error"));
    }

    #[test]
    fn test_format_by_code() {
        let mut report = ErrorReport::new();

        report.add(LashError::parse_invalid_checkbox(
            PathBuf::from("test.md"),
            5,
            3,
            "[*]",
        ));

        report.add(LashError::parse_invalid_checkbox(
            PathBuf::from("test.md"),
            10,
            3,
            "[?]",
        ));

        let formatted = report.format_text(GroupBy::ErrorCode, false);
        assert!(formatted.contains("E_PARSE_INVALID_CHECKBOX (2 occurrences)"));
    }

    #[test]
    fn test_empty_report_format() {
        let report = ErrorReport::new();
        let formatted = report.format_text(GroupBy::None, false);
        assert_eq!(formatted, "No errors found.");
    }

    #[test]
    fn test_add_many() {
        let mut report = ErrorReport::new();

        let errors = vec![
            LashError::parse_invalid_checkbox(PathBuf::from("test.md"), 5, 3, "[*]"),
            LashError::lint_duplicate_id(PathBuf::from("test.md"), 10, 3, "id", 5),
        ];

        report.add_many(errors);
        assert_eq!(report.len(), 2);
    }
}
