//! Error reporting system for the Lash CLI
//!
//! This module provides flexible error reporting with support for:
//! - Multiple verbosity levels (Quiet, Normal, Verbose, Debug)
//! - Multiple output formats (Text with colors, JSON)
//! - Multiple display modes (Streaming, Batch)
//! - Error summaries and statistics
//!
//! # Examples
//!
//! ## Streaming Mode (errors printed immediately)
//!
//! ```no_run
//! use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig, ErrorDisplayMode};
//! use lash_cli::formatter::{OutputFormat, Verbosity};
//! use lash_types::error::LashError;
//! use std::path::PathBuf;
//!
//! let config = ErrorReporterConfig {
//!     verbosity: Verbosity::Normal,
//!     output_format: OutputFormat::Text,
//!     display_mode: ErrorDisplayMode::Streaming,
//!     theme: None,
//!     show_summary: true,
//! };
//!
//! let mut reporter = ErrorReporter::new(config);
//!
//! // Errors are printed immediately to stderr
//! let err = LashError::parse_invalid_checkbox(
//!     PathBuf::from("tasks.md"),
//!     5,
//!     3,
//!     "[*] invalid"
//! );
//! reporter.report_error(&err);
//!
//! // Print summary at the end
//! reporter.flush_with_summary();
//! ```
//!
//! ## Batch Mode (errors collected and printed at end)
//!
//! ```no_run
//! use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig, ErrorDisplayMode};
//! use lash_cli::formatter::{OutputFormat, Verbosity};
//! use lash_types::error::LashError;
//! use std::path::PathBuf;
//!
//! let config = ErrorReporterConfig {
//!     verbosity: Verbosity::Normal,
//!     output_format: OutputFormat::Text,
//!     display_mode: ErrorDisplayMode::Batch,
//!     theme: None,
//!     show_summary: true,
//! };
//!
//! let mut reporter = ErrorReporter::new(config);
//!
//! // Collect errors (nothing printed yet)
//! let err1 = LashError::parse_invalid_checkbox(
//!     PathBuf::from("tasks.md"),
//!     5,
//!     3,
//!     "[*] invalid"
//! );
//! reporter.collect_error(err1);
//!
//! let err2 = LashError::lint_duplicate_id(
//!     PathBuf::from("tasks.md"),
//!     10,
//!     5,
//!     "task-id",
//!     8
//! );
//! reporter.collect_error(err2);
//!
//! // Print all errors and summary at once
//! reporter.flush_with_summary();
//! ```

use crate::formatter::{OutputFormat, Verbosity};
use crate::theme::CliTheme;
use lash_types::error::{Diagnostic, ExitCode, LashError, Location, Severity};
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;

/// Error display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorDisplayMode {
    /// Show errors immediately as they occur
    Streaming,
    /// Collect errors and show at end
    Batch,
}

/// Configuration for error reporter
#[derive(Debug, Clone)]
pub struct ErrorReporterConfig {
    /// Verbosity level for error output
    pub verbosity: Verbosity,
    /// Output format (Text or JSON)
    pub output_format: OutputFormat,
    /// Display mode (Streaming or Batch)
    pub display_mode: ErrorDisplayMode,
    /// Optional theme for colored output
    pub theme: Option<CliTheme>,
    /// Whether to show summary at the end
    pub show_summary: bool,
}

impl Default for ErrorReporterConfig {
    fn default() -> Self {
        Self {
            verbosity: Verbosity::Normal,
            output_format: OutputFormat::Text,
            display_mode: ErrorDisplayMode::Streaming,
            theme: None,
            show_summary: true,
        }
    }
}

/// Summary of error reporting statistics
#[derive(Debug, Clone, Default)]
pub struct ErrorSummary {
    /// Number of errors reported
    pub error_count: usize,
    /// Number of warnings reported
    pub warning_count: usize,
    /// Number of info messages reported
    pub info_count: usize,
    /// Number of hints reported
    pub hint_count: usize,
    /// Unique files affected by errors
    pub files_affected: HashSet<PathBuf>,
}

impl ErrorSummary {
    /// Get the total number of diagnostics
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.error_count + self.warning_count + self.info_count + self.hint_count
    }

    /// Check if there are any errors
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Get the most severe exit code based on errors
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        if self.error_count > 0 {
            ExitCode::LintError
        } else {
            ExitCode::Success
        }
    }
}

/// Error reporter for collecting and displaying errors
pub struct ErrorReporter {
    config: ErrorReporterConfig,
    errors: Vec<LashError>,
    diagnostics: Vec<Diagnostic>,
    summary: ErrorSummary,
}

impl ErrorReporter {
    /// Create a new error reporter with the given configuration
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig};
    /// use lash_cli::formatter::{OutputFormat, Verbosity};
    ///
    /// let config = ErrorReporterConfig {
    ///     verbosity: Verbosity::Normal,
    ///     output_format: OutputFormat::Text,
    ///     display_mode: lash_cli::error_reporter::ErrorDisplayMode::Streaming,
    ///     theme: None,
    ///     show_summary: true,
    /// };
    ///
    /// let reporter = ErrorReporter::new(config);
    /// assert_eq!(reporter.error_count(), 0);
    /// ```
    #[must_use]
    pub fn new(config: ErrorReporterConfig) -> Self {
        Self {
            config,
            errors: Vec::new(),
            diagnostics: Vec::new(),
            summary: ErrorSummary::default(),
        }
    }

    /// Report an error (either immediately or collect for later)
    ///
    /// In Streaming mode, the error is printed immediately to stderr.
    /// In Batch mode, the error is collected for later display.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig, ErrorDisplayMode};
    /// use lash_cli::formatter::{OutputFormat, Verbosity};
    /// use lash_types::error::LashError;
    /// use std::path::PathBuf;
    ///
    /// let config = ErrorReporterConfig {
    ///     verbosity: Verbosity::Normal,
    ///     output_format: OutputFormat::Text,
    ///     display_mode: ErrorDisplayMode::Streaming,
    ///     theme: None,
    ///     show_summary: false,
    /// };
    ///
    /// let mut reporter = ErrorReporter::new(config);
    /// let err = LashError::parse_invalid_checkbox(
    ///     PathBuf::from("test.md"),
    ///     5,
    ///     3,
    ///     "[*] bad"
    /// );
    /// reporter.report_error(&err);
    /// ```
    pub fn report_error(&mut self, error: &LashError) {
        let diagnostic = error.to_diagnostic();
        self.update_summary(&diagnostic);

        match self.config.display_mode {
            ErrorDisplayMode::Streaming => {
                // Print immediately to stderr
                let formatted = self.format_diagnostic(&diagnostic);
                let _ = writeln!(io::stderr(), "{formatted}");
            }
            ErrorDisplayMode::Batch => {
                // Collect for later
                self.errors.push(error.clone());
                self.diagnostics.push(diagnostic);
            }
        }
    }

    /// Report a diagnostic (either immediately or collect for later)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig};
    /// use lash_types::error::{Diagnostic, Severity};
    ///
    /// let config = ErrorReporterConfig::default();
    /// let mut reporter = ErrorReporter::new(config);
    ///
    /// let diagnostic = Diagnostic {
    ///     code: "E_TEST",
    ///     severity: Severity::Error,
    ///     message: "Test error".to_string(),
    ///     location: None,
    ///     snippet: None,
    ///     help: None,
    ///     labels: None,
    ///     recovery_command: None,
    ///     fix_steps: None,
    ///     explanation: None,
    ///     docs_url: None,
    /// };
    ///
    /// reporter.report_diagnostic(&diagnostic);
    /// ```
    pub fn report_diagnostic(&mut self, diagnostic: &Diagnostic) {
        self.update_summary(diagnostic);

        match self.config.display_mode {
            ErrorDisplayMode::Streaming => {
                // Print immediately to stderr
                let formatted = self.format_diagnostic(diagnostic);
                let _ = writeln!(io::stderr(), "{formatted}");
            }
            ErrorDisplayMode::Batch => {
                // Collect for later
                self.diagnostics.push(diagnostic.clone());
            }
        }
    }

    /// Collect an error for batch display (always collects, regardless of mode)
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig};
    /// use lash_types::error::LashError;
    /// use std::path::PathBuf;
    ///
    /// let mut reporter = ErrorReporter::new(ErrorReporterConfig::default());
    /// let err = LashError::parse_invalid_checkbox(
    ///     PathBuf::from("test.md"),
    ///     5,
    ///     3,
    ///     "[*] bad"
    /// );
    ///
    /// reporter.collect_error(err);
    /// assert_eq!(reporter.error_count(), 1);
    /// ```
    pub fn collect_error(&mut self, error: LashError) {
        let diagnostic = error.to_diagnostic();
        self.update_summary(&diagnostic);
        self.errors.push(error);
        self.diagnostics.push(diagnostic);
    }

    /// Report error while respecting an active progress bar
    ///
    /// In streaming mode, this suspends the progress bar (if provided) while
    /// printing the error to stderr, then resumes it. This prevents the error
    /// message from getting mangled by progress bar updates.
    ///
    /// In batch mode, errors are collected for later display as usual.
    ///
    /// # Arguments
    ///
    /// * `error` - The error to report
    /// * `progress_bar` - Optional progress bar to suspend while printing
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig, ErrorDisplayMode};
    /// use lash_cli::formatter::{OutputFormat, Verbosity};
    /// use lash_types::error::LashError;
    /// use std::path::PathBuf;
    /// use indicatif::ProgressBar;
    ///
    /// let config = ErrorReporterConfig {
    ///     verbosity: Verbosity::Normal,
    ///     output_format: OutputFormat::Text,
    ///     display_mode: ErrorDisplayMode::Streaming,
    ///     theme: None,
    ///     show_summary: false,
    /// };
    ///
    /// let mut reporter = ErrorReporter::new(config);
    /// let pb = ProgressBar::new(100);
    /// let err = LashError::parse_invalid_checkbox(
    ///     PathBuf::from("test.md"),
    ///     5,
    ///     3,
    ///     "[*] bad"
    /// );
    ///
    /// reporter.report_error_with_progress(&err, Some(&pb));
    /// ```
    pub fn report_error_with_progress(
        &mut self,
        error: &LashError,
        progress_bar: Option<&indicatif::ProgressBar>,
    ) {
        let diagnostic = error.to_diagnostic();
        self.update_summary(&diagnostic);

        match self.config.display_mode {
            ErrorDisplayMode::Streaming => {
                // Format the error message
                let formatted = self.format_diagnostic(&diagnostic);

                // If there's a progress bar, suspend it while printing
                if let Some(pb) = progress_bar {
                    pb.suspend(|| {
                        let _ = writeln!(io::stderr(), "{formatted}");
                    });
                } else {
                    // No progress bar - just print normally
                    let _ = writeln!(io::stderr(), "{formatted}");
                }
            }
            ErrorDisplayMode::Batch => {
                // Collect for later display
                self.errors.push(error.clone());
                self.diagnostics.push(diagnostic);
            }
        }
    }

    /// Display all collected errors (for batch mode)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig, ErrorDisplayMode};
    /// use lash_cli::formatter::{OutputFormat, Verbosity};
    ///
    /// let config = ErrorReporterConfig {
    ///     verbosity: Verbosity::Normal,
    ///     output_format: OutputFormat::Text,
    ///     display_mode: ErrorDisplayMode::Batch,
    ///     theme: None,
    ///     show_summary: false,
    /// };
    ///
    /// let reporter = ErrorReporter::new(config);
    /// reporter.flush();
    /// ```
    pub fn flush(&self) {
        if self.config.output_format.is_json() {
            self.flush_json();
        } else {
            self.flush_text();
        }
    }

    /// Display all collected errors with summary
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig};
    ///
    /// let reporter = ErrorReporter::new(ErrorReporterConfig::default());
    /// reporter.flush_with_summary();
    /// ```
    pub fn flush_with_summary(&self) {
        self.flush();

        if self.config.show_summary && !self.config.output_format.is_json() {
            self.print_summary();
        }
    }

    /// Format an error based on verbosity level
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig};
    /// use lash_cli::formatter::Verbosity;
    /// use lash_types::error::LashError;
    /// use std::path::PathBuf;
    ///
    /// let mut config = ErrorReporterConfig::default();
    /// config.verbosity = Verbosity::Verbose;
    ///
    /// let reporter = ErrorReporter::new(config);
    /// let err = LashError::parse_invalid_checkbox(
    ///     PathBuf::from("test.md"),
    ///     5,
    ///     3,
    ///     "[*] bad"
    /// );
    ///
    /// let formatted = reporter.format_error(&err);
    /// assert!(!formatted.is_empty());
    /// ```
    #[must_use]
    pub fn format_error(&self, error: &LashError) -> String {
        let diagnostic = error.to_diagnostic();
        self.format_diagnostic(&diagnostic)
    }

    /// Format a diagnostic based on verbosity level
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig};
    /// use lash_types::error::{Diagnostic, Severity};
    ///
    /// let reporter = ErrorReporter::new(ErrorReporterConfig::default());
    /// let diagnostic = Diagnostic {
    ///     code: "E_TEST",
    ///     severity: Severity::Error,
    ///     message: "Test error".to_string(),
    ///     location: None,
    ///     snippet: None,
    ///     help: None,
    ///     labels: None,
    ///     recovery_command: None,
    ///     fix_steps: None,
    ///     explanation: None,
    ///     docs_url: None,
    /// };
    ///
    /// let formatted = reporter.format_diagnostic(&diagnostic);
    /// assert!(formatted.contains("E_TEST"));
    /// ```
    #[must_use]
    pub fn format_diagnostic(&self, diagnostic: &Diagnostic) -> String {
        match self.config.verbosity {
            Verbosity::Quiet => Self::format_quiet(diagnostic),
            Verbosity::Normal => self.format_normal(diagnostic),
            Verbosity::Verbose | Verbosity::Debug | Verbosity::Trace => {
                self.format_verbose(diagnostic)
            }
        }
    }

    /// Get the error summary
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig};
    /// use lash_types::error::LashError;
    /// use std::path::PathBuf;
    ///
    /// let mut reporter = ErrorReporter::new(ErrorReporterConfig::default());
    /// let err = LashError::parse_invalid_checkbox(
    ///     PathBuf::from("test.md"),
    ///     5,
    ///     3,
    ///     "[*] bad"
    /// );
    /// reporter.collect_error(err);
    ///
    /// let summary = reporter.summary();
    /// assert_eq!(summary.error_count, 1);
    /// ```
    #[must_use]
    pub fn summary(&self) -> &ErrorSummary {
        &self.summary
    }

    /// Check if any errors have been reported
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig};
    ///
    /// let reporter = ErrorReporter::new(ErrorReporterConfig::default());
    /// assert!(!reporter.has_errors());
    /// ```
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.summary.has_errors()
    }

    /// Get the number of errors reported
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig};
    ///
    /// let reporter = ErrorReporter::new(ErrorReporterConfig::default());
    /// assert_eq!(reporter.error_count(), 0);
    /// ```
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.summary.error_count
    }

    /// Determine the appropriate exit code based on reported errors
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::error_reporter::{ErrorReporter, ErrorReporterConfig};
    /// use lash_types::error::{ExitCode, LashError};
    /// use std::path::PathBuf;
    ///
    /// let mut reporter = ErrorReporter::new(ErrorReporterConfig::default());
    /// assert_eq!(reporter.exit_code(), ExitCode::Success);
    ///
    /// let err = LashError::parse_invalid_checkbox(
    ///     PathBuf::from("test.md"),
    ///     5,
    ///     3,
    ///     "[*] bad"
    /// );
    /// reporter.collect_error(err);
    /// assert_eq!(reporter.exit_code(), ExitCode::LintError);
    /// ```
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        // Find the most severe error among all collected errors
        self.errors
            .iter()
            .map(ExitCode::from)
            .max_by_key(|code| code.as_i32())
            .unwrap_or(self.summary.exit_code())
    }

    // Private methods

    fn update_summary(&mut self, diagnostic: &Diagnostic) {
        match diagnostic.severity {
            Severity::Error => self.summary.error_count += 1,
            Severity::Warning => self.summary.warning_count += 1,
            Severity::Info => self.summary.info_count += 1,
            Severity::Hint => self.summary.hint_count += 1,
        }

        if let Some(location) = &diagnostic.location {
            self.summary
                .files_affected
                .insert(location.file_path.clone());
        }
    }

    fn format_quiet(diagnostic: &Diagnostic) -> String {
        // In quiet mode, only show error count (minimal output)
        format!("{}: {}", diagnostic.severity, diagnostic.message)
    }

    fn format_normal(&self, diagnostic: &Diagnostic) -> String {
        let mut output = String::new();

        // Format: error[CODE]: message
        let severity_str =
            self.style_severity(diagnostic.severity, &diagnostic.severity.to_string());
        let code_str = self.style_code(diagnostic.code);

        output.push_str(&format!(
            "{}[{}]: {}",
            severity_str, code_str, diagnostic.message
        ));

        // Add location if available
        if let Some(location) = &diagnostic.location {
            let location_str = self.style_location(&format_location(location));
            output.push_str(&format!("\n  at {location_str}"));
        }

        output
    }

    fn format_verbose(&self, diagnostic: &Diagnostic) -> String {
        let mut output = self.format_normal(diagnostic);

        // Add code snippet if available
        if let Some(snippet) = &diagnostic.snippet {
            output.push_str("\n\n");
            output.push_str(&self.style_snippet(snippet));
        }

        // Add help text if available
        if let Some(help) = &diagnostic.help {
            output.push_str("\n\n");
            let help_label = self.style_help_label("help");
            output.push_str(&format!("  {help_label}: {help}"));
        }

        // In debug/trace mode, add all extra context
        if self.config.verbosity >= Verbosity::Debug {
            if let Some(labels) = &diagnostic.labels {
                for (key, value) in labels {
                    output.push_str(&format!("\n  {key}: {value}"));
                }
            }

            if let Some(recovery) = &diagnostic.recovery_command {
                output.push_str(&format!("\n  recovery: {recovery}"));
            }

            if let Some(steps) = &diagnostic.fix_steps {
                output.push_str("\n  fix steps:");
                for (i, step) in steps.iter().enumerate() {
                    let step_num = i + 1;
                    output.push_str(&format!("\n    {step_num}. {step}"));
                }
            }

            if let Some(explanation) = &diagnostic.explanation {
                output.push_str(&format!("\n  explanation: {explanation}"));
            }

            if let Some(url) = &diagnostic.docs_url {
                output.push_str(&format!("\n  docs: {url}"));
            }
        }

        output
    }

    fn flush_text(&self) {
        let stderr = io::stderr();
        let mut handle = stderr.lock();

        for diagnostic in &self.diagnostics {
            let formatted = self.format_diagnostic(diagnostic);
            let _ = writeln!(handle, "{formatted}\n");
        }
    }

    fn flush_json(&self) {
        let output = serde_json::json!({
            "diagnostics": self.diagnostics,
            "summary": {
                "error_count": self.summary.error_count,
                "warning_count": self.summary.warning_count,
                "info_count": self.summary.info_count,
                "hint_count": self.summary.hint_count,
                "files_affected": self.summary.files_affected.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            }
        });

        let json_str = if self.config.output_format == OutputFormat::JsonPretty {
            serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
        } else {
            serde_json::to_string(&output).unwrap_or_else(|_| "{}".to_string())
        };

        let _ = writeln!(io::stderr(), "{json_str}");
    }

    fn print_summary(&self) {
        let stderr = io::stderr();
        let mut handle = stderr.lock();

        if self.summary.total_count() == 0 {
            return;
        }

        let _ = writeln!(handle, "\nSummary:");
        let _ = writeln!(
            handle,
            "  {} errors, {} warnings, {} info, {} hints",
            self.style_count(self.summary.error_count, self.summary.error_count > 0),
            self.style_count(self.summary.warning_count, self.summary.warning_count > 0),
            self.summary.info_count,
            self.summary.hint_count
        );

        if !self.summary.files_affected.is_empty() {
            let _ = writeln!(
                handle,
                "  {} files affected",
                self.summary.files_affected.len()
            );
        }
    }

    // Styling helpers

    fn style_severity(&self, severity: Severity, text: &str) -> String {
        if let Some(theme) = &self.config.theme {
            match severity {
                Severity::Error => theme.style_error(text),
                Severity::Warning => theme.style_warning(text),
                Severity::Info => theme.style_info(text),
                Severity::Hint => theme.style_label(text),
            }
        } else {
            text.to_string()
        }
    }

    fn style_code(&self, code: &str) -> String {
        if let Some(theme) = &self.config.theme {
            theme.style_label(code)
        } else {
            code.to_string()
        }
    }

    fn style_location(&self, location: &str) -> String {
        if let Some(theme) = &self.config.theme {
            theme.style_muted(location)
        } else {
            location.to_string()
        }
    }

    fn style_snippet(&self, snippet: &str) -> String {
        if let Some(theme) = &self.config.theme {
            theme.style_muted(snippet)
        } else {
            snippet.to_string()
        }
    }

    fn style_help_label(&self, label: &str) -> String {
        if let Some(theme) = &self.config.theme {
            theme.style_info(label)
        } else {
            label.to_string()
        }
    }

    fn style_count(&self, count: usize, is_error: bool) -> String {
        let count_str = count.to_string();
        if let Some(theme) = &self.config.theme {
            if is_error {
                theme.style_error(&count_str)
            } else {
                count_str
            }
        } else {
            count_str
        }
    }
}

/// Format a location for display
fn format_location(location: &Location) -> String {
    let mut result = location.file_path.display().to_string();

    if let Some(line) = location.line {
        result.push(':');
        result.push_str(&line.to_string());

        if let Some(column) = location.column {
            result.push(':');
            result.push_str(&column.to_string());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_reporter_new() {
        let config = ErrorReporterConfig::default();
        let reporter = ErrorReporter::new(config);

        assert_eq!(reporter.error_count(), 0);
        assert!(!reporter.has_errors());
        assert_eq!(reporter.exit_code(), ExitCode::Success);
    }

    #[test]
    fn test_collect_error() {
        let config = ErrorReporterConfig::default();
        let mut reporter = ErrorReporter::new(config);

        let err = LashError::parse_invalid_checkbox(PathBuf::from("test.md"), 5, 3, "[*] invalid");

        reporter.collect_error(err);

        assert_eq!(reporter.error_count(), 1);
        assert!(reporter.has_errors());
        assert_eq!(reporter.exit_code(), ExitCode::LintError);
    }

    #[test]
    fn test_collect_multiple_errors() {
        let config = ErrorReporterConfig::default();
        let mut reporter = ErrorReporter::new(config);

        let err1 = LashError::parse_invalid_checkbox(PathBuf::from("test.md"), 5, 3, "[*] invalid");
        let err2 = LashError::lint_duplicate_id(PathBuf::from("test.md"), 10, 5, "task-id", 8);

        reporter.collect_error(err1);
        reporter.collect_error(err2);

        assert_eq!(reporter.error_count(), 2);
        assert_eq!(reporter.summary().files_affected.len(), 1); // Same file
    }

    #[test]
    fn test_summary() {
        let config = ErrorReporterConfig::default();
        let mut reporter = ErrorReporter::new(config);

        let diag1 = Diagnostic {
            code: "E_TEST",
            severity: Severity::Error,
            message: "Test error".to_string(),
            location: Some(Location::new(PathBuf::from("test1.md"), 5, 3)),
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let diag2 = Diagnostic {
            code: "W_TEST",
            severity: Severity::Warning,
            message: "Test warning".to_string(),
            location: Some(Location::new(PathBuf::from("test2.md"), 10, 5)),
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        reporter.report_diagnostic(&diag1);
        reporter.report_diagnostic(&diag2);

        let summary = reporter.summary();
        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(summary.info_count, 0);
        assert_eq!(summary.hint_count, 0);
        assert_eq!(summary.total_count(), 2);
        assert_eq!(summary.files_affected.len(), 2);
    }

    #[test]
    fn test_format_quiet() {
        let config = ErrorReporterConfig {
            verbosity: Verbosity::Quiet,
            ..Default::default()
        };

        let reporter = ErrorReporter::new(config);

        let diagnostic = Diagnostic {
            code: "E_TEST",
            severity: Severity::Error,
            message: "Test error".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let formatted = reporter.format_diagnostic(&diagnostic);
        assert!(formatted.contains("error"));
        assert!(formatted.contains("Test error"));
        // Should NOT contain code in quiet mode
        assert!(!formatted.contains("E_TEST"));
    }

    #[test]
    fn test_format_normal() {
        let config = ErrorReporterConfig {
            verbosity: Verbosity::Normal,
            ..Default::default()
        };

        let reporter = ErrorReporter::new(config);

        let diagnostic = Diagnostic {
            code: "E_TEST",
            severity: Severity::Error,
            message: "Test error".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 5, 3)),
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let formatted = reporter.format_diagnostic(&diagnostic);
        assert!(formatted.contains("error[E_TEST]"));
        assert!(formatted.contains("Test error"));
        assert!(formatted.contains("test.md:5:3"));
    }

    #[test]
    fn test_format_verbose() {
        let config = ErrorReporterConfig {
            verbosity: Verbosity::Verbose,
            ..Default::default()
        };

        let reporter = ErrorReporter::new(config);

        let diagnostic = Diagnostic {
            code: "E_TEST",
            severity: Severity::Error,
            message: "Test error".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 5, 3)),
            snippet: Some("- [*] invalid".to_string()),
            help: Some("Use valid checkbox syntax".to_string()),
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let formatted = reporter.format_diagnostic(&diagnostic);
        assert!(formatted.contains("error[E_TEST]"));
        assert!(formatted.contains("Test error"));
        assert!(formatted.contains("- [*] invalid"));
        assert!(formatted.contains("help: Use valid checkbox syntax"));
    }

    #[test]
    fn test_format_debug() {
        let config = ErrorReporterConfig {
            verbosity: Verbosity::Debug,
            ..Default::default()
        };

        let reporter = ErrorReporter::new(config);

        let diagnostic = Diagnostic {
            code: "E_TEST",
            severity: Severity::Error,
            message: "Test error".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 5, 3)),
            snippet: Some("- [*] invalid".to_string()),
            help: Some("Use valid checkbox syntax".to_string()),
            labels: Some(vec![("context".to_string(), "parsing".to_string())]),
            recovery_command: Some("lash format test.md".to_string()),
            fix_steps: Some(vec!["Fix the checkbox".to_string()]),
            explanation: Some("Checkboxes must use valid syntax".to_string()),
            docs_url: Some("https://docs.example.com/checkboxes".to_string()),
        };

        let formatted = reporter.format_diagnostic(&diagnostic);
        assert!(formatted.contains("error[E_TEST]"));
        assert!(formatted.contains("context: parsing"));
        assert!(formatted.contains("recovery: lash format test.md"));
        assert!(formatted.contains("fix steps:"));
        assert!(formatted.contains("explanation: Checkboxes must use valid syntax"));
        assert!(formatted.contains("docs: https://docs.example.com/checkboxes"));
    }

    #[test]
    fn test_exit_code_from_errors() {
        let config = ErrorReporterConfig::default();
        let mut reporter = ErrorReporter::new(config);

        // No errors -> Success
        assert_eq!(reporter.exit_code(), ExitCode::Success);

        // Parse error -> LintError
        let err1 = LashError::parse_invalid_checkbox(PathBuf::from("test.md"), 5, 3, "[*] invalid");
        reporter.collect_error(err1);
        assert_eq!(reporter.exit_code(), ExitCode::LintError);

        // Config error should override (higher exit code)
        let err2 = LashError::config_root_not_found(PathBuf::from("/tmp"));
        reporter.collect_error(err2);
        assert_eq!(reporter.exit_code(), ExitCode::ConfigError);
    }

    #[test]
    fn test_format_location() {
        let loc1 = Location::new(PathBuf::from("test.md"), 5, 3);
        assert_eq!(format_location(&loc1), "test.md:5:3");

        let loc2 = Location::file_only(PathBuf::from("test.md"));
        assert_eq!(format_location(&loc2), "test.md");

        let mut loc3 = Location::new(PathBuf::from("test.md"), 5, 3);
        loc3.column = None;
        assert_eq!(format_location(&loc3), "test.md:5");
    }

    #[test]
    fn test_error_summary_default() {
        let summary = ErrorSummary::default();
        assert_eq!(summary.error_count, 0);
        assert_eq!(summary.warning_count, 0);
        assert_eq!(summary.info_count, 0);
        assert_eq!(summary.hint_count, 0);
        assert_eq!(summary.total_count(), 0);
        assert!(!summary.has_errors());
        assert_eq!(summary.exit_code(), ExitCode::Success);
    }

    #[test]
    fn test_error_display_mode() {
        assert_eq!(ErrorDisplayMode::Streaming, ErrorDisplayMode::Streaming);
        assert_eq!(ErrorDisplayMode::Batch, ErrorDisplayMode::Batch);
        assert_ne!(ErrorDisplayMode::Streaming, ErrorDisplayMode::Batch);
    }

    #[test]
    fn test_config_default() {
        let config = ErrorReporterConfig::default();
        assert_eq!(config.verbosity, Verbosity::Normal);
        assert_eq!(config.output_format, OutputFormat::Text);
        assert_eq!(config.display_mode, ErrorDisplayMode::Streaming);
        assert!(config.theme.is_none());
        assert!(config.show_summary);
    }
}
