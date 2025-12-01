//! Output formatting system
//!
//! This module provides flexible output formatting supporting human-readable text,
//! JSON, and quiet modes. It respects terminal capabilities and environment variables.

use anyhow::Result;
use lash_core::logo::LOGO;
use lash_types::error::{Diagnostic, LashError, Severity};
use lash_types::TaskStatus;
use serde::Serialize;

use crate::error_reporter::ErrorSummary;
use crate::theme::CliTheme;

/// Output format mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text with colors (default)
    Text,
    /// Machine-readable JSON (compact)
    Json,
    /// Pretty-printed JSON for debugging
    JsonPretty,
    /// Minimal output (errors and critical info only)
    Quiet,
}

impl OutputFormat {
    /// Check if this format supports color output
    #[must_use]
    pub fn supports_color(&self) -> bool {
        matches!(self, OutputFormat::Text)
    }

    /// Check if this format is JSON-based
    #[must_use]
    pub fn is_json(&self) -> bool {
        matches!(self, OutputFormat::Json | OutputFormat::JsonPretty)
    }
}

/// Verbosity level for output
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    /// Only errors
    Quiet = 0,
    /// Errors and warnings (default)
    Normal = 1,
    /// + informational messages
    Verbose = 2,
    /// + debug messages
    Debug = 3,
    /// + trace-level messages
    Trace = 4,
}

impl From<u8> for Verbosity {
    fn from(count: u8) -> Self {
        match count {
            0 => Verbosity::Normal,
            1 => Verbosity::Verbose,
            2 => Verbosity::Debug,
            _ => Verbosity::Trace,
        }
    }
}

/// Output formatter trait
///
/// Implementations provide formatting for different output modes.
pub trait OutputFormatter {
    /// Format a success message
    fn format_success(&self, message: &str) -> Result<String>;

    /// Format an error message
    fn format_error(&self, message: &str) -> Result<String>;

    /// Format a warning message
    fn format_warning(&self, message: &str) -> Result<String>;

    /// Format an info message
    fn format_info(&self, message: &str) -> Result<String>;

    /// Format a list of items
    fn format_list(&self, items: &[String]) -> Result<String>;

    /// Format tabular data
    fn format_table(&self, headers: &[String], rows: &[Vec<String>]) -> Result<String>;

    /// Format a `LashError` based on verbosity level
    ///
    /// # Arguments
    ///
    /// * `error` - The error to format
    /// * `verbosity` - Verbosity level controlling detail
    fn format_lash_error(&self, error: &LashError, verbosity: Verbosity) -> Result<String>;

    /// Format a Diagnostic based on verbosity level
    ///
    /// # Arguments
    ///
    /// * `diagnostic` - The diagnostic to format
    /// * `verbosity` - Verbosity level controlling detail
    fn format_diagnostic(&self, diagnostic: &Diagnostic, verbosity: Verbosity) -> Result<String>;

    /// Format an error summary (count by severity)
    ///
    /// # Arguments
    ///
    /// * `summary` - The error summary to format
    fn format_error_summary(&self, summary: &ErrorSummary) -> Result<String>;
}

/// Text-based formatter with optional colors
pub struct TextFormatter {
    theme: Option<CliTheme>,
    verbosity: Verbosity,
}

impl TextFormatter {
    /// Create a new text formatter
    ///
    /// # Arguments
    ///
    /// * `use_color` - Whether to use ANSI colors
    /// * `verbosity` - Verbosity level
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::formatter::{TextFormatter, Verbosity};
    ///
    /// let formatter = TextFormatter::new(true, Verbosity::Normal);
    /// ```
    #[must_use]
    pub fn new(use_color: bool, verbosity: Verbosity) -> Self {
        // For backward compatibility, create a default theme if colors are enabled
        let theme = if use_color && supports_color() {
            CliTheme::load(None, true).ok().flatten()
        } else {
            None
        };

        Self { theme, verbosity }
    }

    /// Create a new text formatter with an optional theme
    ///
    /// # Arguments
    ///
    /// * `theme` - Optional CLI theme for styling
    /// * `verbosity` - Verbosity level
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::formatter::{TextFormatter, Verbosity};
    /// use lash_cli::theme::CliTheme;
    ///
    /// let theme = CliTheme::load(None, true).unwrap();
    /// let formatter = TextFormatter::with_theme(theme, Verbosity::Normal);
    /// ```
    #[must_use]
    pub fn with_theme(theme: Option<CliTheme>, verbosity: Verbosity) -> Self {
        Self { theme, verbosity }
    }

    /// Check if colors are enabled
    #[must_use]
    pub fn has_color(&self) -> bool {
        self.theme.is_some()
    }

    /// Get the verbosity level
    #[must_use]
    pub fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    /// Get the theme if available
    #[must_use]
    pub fn theme(&self) -> Option<&CliTheme> {
        self.theme.as_ref()
    }
}

impl OutputFormatter for TextFormatter {
    fn format_success(&self, message: &str) -> Result<String> {
        if let Some(ref theme) = self.theme {
            Ok(theme.style_success(message))
        } else {
            Ok(message.to_string())
        }
    }

    fn format_error(&self, message: &str) -> Result<String> {
        let formatted = if let Some(ref theme) = self.theme {
            format!("{}: {}", theme.style_error("error"), message)
        } else {
            format!("error: {message}")
        };
        Ok(formatted)
    }

    fn format_warning(&self, message: &str) -> Result<String> {
        let formatted = if let Some(ref theme) = self.theme {
            format!("{}: {}", theme.style_warning("warning"), message)
        } else {
            format!("warning: {message}")
        };
        Ok(formatted)
    }

    fn format_info(&self, message: &str) -> Result<String> {
        let formatted = if let Some(ref theme) = self.theme {
            format!("{}: {}", theme.style_info("info"), message)
        } else {
            format!("info: {message}")
        };
        Ok(formatted)
    }

    fn format_list(&self, items: &[String]) -> Result<String> {
        let mut output = String::new();
        for item in items {
            output.push_str("  - ");
            output.push_str(item);
            output.push('\n');
        }
        Ok(output)
    }

    fn format_table(&self, headers: &[String], rows: &[Vec<String>]) -> Result<String> {
        // Calculate column widths
        let mut widths = headers
            .iter()
            .map(std::string::String::len)
            .collect::<Vec<_>>();
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        let mut output = String::new();

        // Header
        for (i, header) in headers.iter().enumerate() {
            if i > 0 {
                output.push_str("  ");
            }
            output.push_str(&format!("{:<width$}", header, width = widths[i]));
        }
        output.push('\n');

        // Separator
        for (i, width) in widths.iter().enumerate() {
            if i > 0 {
                output.push_str("  ");
            }
            output.push_str(&"-".repeat(*width));
        }
        output.push('\n');

        // Rows
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i > 0 {
                    output.push_str("  ");
                }
                let width = widths.get(i).copied().unwrap_or(cell.len());
                output.push_str(&format!("{cell:<width$}"));
            }
            output.push('\n');
        }

        Ok(output)
    }

    fn format_lash_error(&self, error: &LashError, verbosity: Verbosity) -> Result<String> {
        let diagnostic = error.to_diagnostic();
        self.format_diagnostic(&diagnostic, verbosity)
    }

    fn format_diagnostic(&self, diagnostic: &Diagnostic, verbosity: Verbosity) -> Result<String> {
        match verbosity {
            Verbosity::Quiet => Ok(Self::format_diagnostic_quiet(diagnostic)),
            Verbosity::Normal => Ok(self.format_diagnostic_normal(diagnostic)),
            Verbosity::Verbose | Verbosity::Debug | Verbosity::Trace => {
                Ok(self.format_diagnostic_verbose(diagnostic, verbosity))
            }
        }
    }

    fn format_error_summary(&self, summary: &ErrorSummary) -> Result<String> {
        if summary.total_count() == 0 {
            return Ok(String::new());
        }

        let mut output = String::new();
        output.push_str("\nSummary:\n");

        // Format counts with appropriate styling
        let error_str = if summary.error_count > 0 {
            if let Some(ref theme) = self.theme {
                theme.style_error(&summary.error_count.to_string())
            } else {
                summary.error_count.to_string()
            }
        } else {
            summary.error_count.to_string()
        };

        let warning_str = if summary.warning_count > 0 {
            if let Some(ref theme) = self.theme {
                theme.style_warning(&summary.warning_count.to_string())
            } else {
                summary.warning_count.to_string()
            }
        } else {
            summary.warning_count.to_string()
        };

        output.push_str(&format!(
            "  {} errors, {} warnings, {} info, {} hints\n",
            error_str, warning_str, summary.info_count, summary.hint_count
        ));

        if !summary.files_affected.is_empty() {
            output.push_str(&format!(
                "  {} files affected\n",
                summary.files_affected.len()
            ));
        }

        Ok(output)
    }
}

impl TextFormatter {
    /// Returns the Lash logo banner for CLI output.
    ///
    /// This is displayed at the start of text-mode CLI output when not suppressed.
    ///
    /// # Returns
    ///
    /// A string containing the logo with a trailing newline for separation.
    #[must_use]
    pub fn logo_banner() -> String {
        format!("{LOGO}\n")
    }

    /// Format text based on task status with theme-aware colors
    ///
    /// # Arguments
    ///
    /// * `text` - The text to format
    /// * `status` - The task status determining the color
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::formatter::{TextFormatter, Verbosity};
    /// use lash_types::TaskStatus;
    ///
    /// let formatter = TextFormatter::new(true, Verbosity::Normal);
    /// let formatted = formatter.format_task_status("[x]", TaskStatus::Done);
    /// ```
    #[must_use]
    pub fn format_task_status(&self, text: &str, status: TaskStatus) -> String {
        if let Some(ref theme) = self.theme {
            theme.style_task_status(text, status)
        } else {
            text.to_string()
        }
    }

    /// Format text as a label with theme-aware colors
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::formatter::{TextFormatter, Verbosity};
    ///
    /// let formatter = TextFormatter::new(true, Verbosity::Normal);
    /// let formatted = formatter.format_label("#backend");
    /// ```
    #[must_use]
    pub fn format_label(&self, label: &str) -> String {
        if let Some(ref theme) = self.theme {
            theme.style_label(label)
        } else {
            label.to_string()
        }
    }

    /// Format text as muted/secondary text with theme-aware colors
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::formatter::{TextFormatter, Verbosity};
    ///
    /// let formatter = TextFormatter::new(true, Verbosity::Normal);
    /// let formatted = formatter.format_muted("(optional)");
    /// ```
    #[must_use]
    pub fn format_muted(&self, text: &str) -> String {
        if let Some(ref theme) = self.theme {
            theme.style_muted(text)
        } else {
            text.to_string()
        }
    }

    // Private helper methods for diagnostic formatting

    fn format_diagnostic_quiet(diagnostic: &Diagnostic) -> String {
        // In quiet mode, only show severity and message (minimal output)
        format!("{}: {}", diagnostic.severity, diagnostic.message)
    }

    fn format_diagnostic_normal(&self, diagnostic: &Diagnostic) -> String {
        let mut output = String::new();

        // Format: error[CODE]: message
        let severity_str =
            self.style_severity(diagnostic.severity, &diagnostic.severity.to_string());
        let code_str = self.style_code(diagnostic.code);

        output.push_str(&format!(
            "{}[{}]: {}",
            severity_str, code_str, diagnostic.message
        ));

        // Add location if available (file:line:col format)
        if let Some(location) = &diagnostic.location {
            let location_str = Self::format_location(location);
            let styled_location = self.style_location(&location_str);
            output.push_str(&format!("\n  at {styled_location}"));
        }

        output
    }

    fn format_diagnostic_verbose(&self, diagnostic: &Diagnostic, verbosity: Verbosity) -> String {
        let mut output = self.format_diagnostic_normal(diagnostic);

        // Add code snippet if available and verbosity >= Verbose
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
        if verbosity >= Verbosity::Debug {
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

    // Styling helper methods

    fn style_severity(&self, severity: Severity, text: &str) -> String {
        if let Some(ref theme) = self.theme {
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
        if let Some(ref theme) = self.theme {
            theme.style_label(code)
        } else {
            code.to_string()
        }
    }

    fn style_location(&self, location: &str) -> String {
        if let Some(ref theme) = self.theme {
            theme.style_muted(location)
        } else {
            location.to_string()
        }
    }

    fn style_snippet(&self, snippet: &str) -> String {
        if let Some(ref theme) = self.theme {
            theme.style_muted(snippet)
        } else {
            snippet.to_string()
        }
    }

    fn style_help_label(&self, label: &str) -> String {
        if let Some(ref theme) = self.theme {
            theme.style_info(label)
        } else {
            label.to_string()
        }
    }

    fn format_location(location: &lash_types::error::Location) -> String {
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
}

/// JSON-based formatter
pub struct JsonFormatter {
    pretty: bool,
}

impl JsonFormatter {
    /// Create a new JSON formatter
    ///
    /// # Arguments
    ///
    /// * `pretty` - Whether to pretty-print JSON
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::formatter::JsonFormatter;
    ///
    /// let formatter = JsonFormatter::new(true);
    /// ```
    #[must_use]
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }

    /// Serialize a value to JSON
    fn to_json<T: Serialize>(&self, value: &T) -> Result<String> {
        if self.pretty {
            Ok(serde_json::to_string_pretty(value)?)
        } else {
            Ok(serde_json::to_string(value)?)
        }
    }
}

impl OutputFormatter for JsonFormatter {
    fn format_success(&self, message: &str) -> Result<String> {
        self.to_json(&serde_json::json!({
            "status": "success",
            "message": message
        }))
    }

    fn format_error(&self, message: &str) -> Result<String> {
        self.to_json(&serde_json::json!({
            "status": "error",
            "message": message
        }))
    }

    fn format_warning(&self, message: &str) -> Result<String> {
        self.to_json(&serde_json::json!({
            "status": "warning",
            "message": message
        }))
    }

    fn format_info(&self, message: &str) -> Result<String> {
        self.to_json(&serde_json::json!({
            "status": "info",
            "message": message
        }))
    }

    fn format_list(&self, items: &[String]) -> Result<String> {
        self.to_json(&serde_json::json!({
            "items": items
        }))
    }

    fn format_table(&self, headers: &[String], rows: &[Vec<String>]) -> Result<String> {
        self.to_json(&serde_json::json!({
            "headers": headers,
            "rows": rows
        }))
    }

    fn format_lash_error(&self, error: &LashError, _: Verbosity) -> Result<String> {
        let diagnostic = error.to_diagnostic();
        // For JSON, verbosity doesn't matter - output complete diagnostic as JSON
        self.to_json(&diagnostic)
    }

    fn format_diagnostic(&self, diagnostic: &Diagnostic, _: Verbosity) -> Result<String> {
        // For JSON, verbosity doesn't matter - output complete diagnostic as JSON
        self.to_json(diagnostic)
    }

    fn format_error_summary(&self, summary: &ErrorSummary) -> Result<String> {
        // Convert to structured JSON with counts
        self.to_json(&serde_json::json!({
            "error_count": summary.error_count,
            "warning_count": summary.warning_count,
            "info_count": summary.info_count,
            "hint_count": summary.hint_count,
            "total_count": summary.total_count(),
            "files_affected": summary.files_affected.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        }))
    }
}

/// Quiet formatter (suppresses most output)
pub struct QuietFormatter;

impl QuietFormatter {
    /// Create a new quiet formatter
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::formatter::QuietFormatter;
    ///
    /// let formatter = QuietFormatter::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuietFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFormatter for QuietFormatter {
    fn format_success(&self, _message: &str) -> Result<String> {
        Ok(String::new())
    }

    fn format_error(&self, message: &str) -> Result<String> {
        Ok(format!("error: {message}"))
    }

    fn format_warning(&self, _message: &str) -> Result<String> {
        Ok(String::new())
    }

    fn format_info(&self, _message: &str) -> Result<String> {
        Ok(String::new())
    }

    fn format_list(&self, _items: &[String]) -> Result<String> {
        Ok(String::new())
    }

    fn format_table(&self, _headers: &[String], _rows: &[Vec<String>]) -> Result<String> {
        Ok(String::new())
    }

    fn format_lash_error(&self, error: &LashError, _: Verbosity) -> Result<String> {
        // In quiet mode, just show error code and message
        Ok(format!("{}: {}", error.code(), error))
    }

    fn format_diagnostic(&self, diagnostic: &Diagnostic, _: Verbosity) -> Result<String> {
        // In quiet mode, just show severity and message
        Ok(format!("{}: {}", diagnostic.severity, diagnostic.message))
    }

    fn format_error_summary(&self, summary: &ErrorSummary) -> Result<String> {
        // In quiet mode, only show counts if there are errors
        if summary.error_count > 0 {
            Ok(format!("{} errors\n", summary.error_count))
        } else {
            Ok(String::new())
        }
    }
}

/// Check if the terminal supports color output
///
/// Respects the `NO_COLOR` environment variable and checks if stdout is a TTY.
fn supports_color() -> bool {
    // NO_COLOR environment variable takes precedence
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }

    // Check if stdout is a TTY
    atty::is(atty::Stream::Stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_supports_color() {
        assert!(OutputFormat::Text.supports_color());
        assert!(!OutputFormat::Json.supports_color());
        assert!(!OutputFormat::JsonPretty.supports_color());
        assert!(!OutputFormat::Quiet.supports_color());
    }

    #[test]
    fn test_output_format_is_json() {
        assert!(!OutputFormat::Text.is_json());
        assert!(OutputFormat::Json.is_json());
        assert!(OutputFormat::JsonPretty.is_json());
        assert!(!OutputFormat::Quiet.is_json());
    }

    #[test]
    fn test_verbosity_from_u8() {
        assert_eq!(Verbosity::from(0), Verbosity::Normal);
        assert_eq!(Verbosity::from(1), Verbosity::Verbose);
        assert_eq!(Verbosity::from(2), Verbosity::Debug);
        assert_eq!(Verbosity::from(3), Verbosity::Trace);
        assert_eq!(Verbosity::from(10), Verbosity::Trace);
    }

    #[test]
    fn test_verbosity_ordering() {
        assert!(Verbosity::Quiet < Verbosity::Normal);
        assert!(Verbosity::Normal < Verbosity::Verbose);
        assert!(Verbosity::Verbose < Verbosity::Debug);
        assert!(Verbosity::Debug < Verbosity::Trace);
    }

    #[test]
    fn test_text_formatter_no_color() {
        let formatter = TextFormatter::new(false, Verbosity::Normal);
        assert!(!formatter.has_color());

        let error = formatter.format_error("test error").unwrap();
        assert_eq!(error, "error: test error");

        let warning = formatter.format_warning("test warning").unwrap();
        assert_eq!(warning, "warning: test warning");

        let info = formatter.format_info("test info").unwrap();
        assert_eq!(info, "info: test info");
    }

    #[test]
    fn test_text_formatter_list() {
        let formatter = TextFormatter::new(false, Verbosity::Normal);
        let items = vec!["item1".to_string(), "item2".to_string()];
        let output = formatter.format_list(&items).unwrap();
        assert!(output.contains("  - item1"));
        assert!(output.contains("  - item2"));
    }

    #[test]
    fn test_text_formatter_table() {
        let formatter = TextFormatter::new(false, Verbosity::Normal);
        let headers = vec!["Name".to_string(), "Status".to_string()];
        let rows = vec![
            vec!["Task1".to_string(), "Done".to_string()],
            vec!["Task2".to_string(), "Open".to_string()],
        ];
        let output = formatter.format_table(&headers, &rows).unwrap();
        assert!(output.contains("Name"));
        assert!(output.contains("Status"));
        assert!(output.contains("Task1"));
        assert!(output.contains("Done"));
    }

    #[test]
    fn test_json_formatter() {
        let formatter = JsonFormatter::new(false);

        let success = formatter.format_success("test success").unwrap();
        assert!(success.contains("\"status\":\"success\""));
        assert!(success.contains("\"message\":\"test success\""));

        let error = formatter.format_error("test error").unwrap();
        assert!(error.contains("\"status\":\"error\""));
    }

    #[test]
    fn test_json_formatter_list() {
        let formatter = JsonFormatter::new(false);
        let items = vec!["item1".to_string(), "item2".to_string()];
        let output = formatter.format_list(&items).unwrap();
        assert!(output.contains("\"items\""));
        assert!(output.contains("\"item1\""));
        assert!(output.contains("\"item2\""));
    }

    #[test]
    fn test_json_formatter_pretty() {
        let formatter = JsonFormatter::new(true);
        let output = formatter.format_success("test").unwrap();
        // Pretty JSON should have newlines
        assert!(output.contains('\n'));
    }

    #[test]
    fn test_quiet_formatter() {
        let formatter = QuietFormatter::new();

        // Only errors should produce output
        let error = formatter.format_error("test error").unwrap();
        assert_eq!(error, "error: test error");

        // Everything else should be suppressed
        assert_eq!(formatter.format_success("test").unwrap(), "");
        assert_eq!(formatter.format_warning("test").unwrap(), "");
        assert_eq!(formatter.format_info("test").unwrap(), "");
        assert_eq!(formatter.format_list(&["item".to_string()]).unwrap(), "");
        assert_eq!(formatter.format_table(&[], &[]).unwrap(), "");
    }

    #[test]
    fn test_quiet_formatter_default() {
        let _ = QuietFormatter;
        // Just ensure it compiles
    }

    #[test]
    fn test_text_formatter_verbosity() {
        let formatter = TextFormatter::new(false, Verbosity::Debug);
        assert_eq!(formatter.verbosity(), Verbosity::Debug);
    }

    #[test]
    fn test_text_formatter_with_theme() {
        use crate::theme::CliTheme;
        use lash_tui::colors::{Theme, REGISTRY};

        // Create a formatter with a theme
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        let tui_theme = Theme::new(scheme.clone());
        let cli_theme = CliTheme::new(tui_theme, true);

        let formatter = TextFormatter::with_theme(Some(cli_theme), Verbosity::Normal);
        assert!(formatter.has_color());
        assert!(formatter.theme().is_some());

        // Test that formatting methods work (they should return non-empty strings)
        let success = formatter.format_success("success").unwrap();
        assert!(!success.is_empty());

        let error = formatter.format_error("error").unwrap();
        assert!(error.contains("error"));

        let warning = formatter.format_warning("warning").unwrap();
        assert!(warning.contains("warning"));

        let info = formatter.format_info("info").unwrap();
        assert!(info.contains("info"));
    }

    #[test]
    fn test_text_formatter_theme_aware_methods() {
        use crate::theme::CliTheme;
        use lash_tui::colors::{Theme, REGISTRY};

        // Create a formatter with a theme
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        let tui_theme = Theme::new(scheme.clone());
        let cli_theme = CliTheme::new(tui_theme, true);

        let formatter = TextFormatter::with_theme(Some(cli_theme), Verbosity::Normal);

        // Test task status formatting
        let done = formatter.format_task_status("[x]", TaskStatus::Done);
        assert!(done.contains('x'));

        let blocked = formatter.format_task_status("[!]", TaskStatus::Blocked);
        assert!(blocked.contains('!'));

        // Test label formatting
        let label = formatter.format_label("#backend");
        assert!(label.contains("#backend"));

        // Test muted formatting
        let muted = formatter.format_muted("(optional)");
        assert!(muted.contains("(optional)"));
    }

    #[test]
    fn test_text_formatter_theme_aware_methods_no_color() {
        let formatter = TextFormatter::with_theme(None, Verbosity::Normal);
        assert!(!formatter.has_color());
        assert!(formatter.theme().is_none());

        // Without theme, methods should return unstyled text
        let done = formatter.format_task_status("[x]", TaskStatus::Done);
        assert_eq!(done, "[x]");

        let label = formatter.format_label("#backend");
        assert_eq!(label, "#backend");

        let muted = formatter.format_muted("(optional)");
        assert_eq!(muted, "(optional)");
    }
}

#[cfg(test)]
mod error_formatting_tests {
    use super::*;
    use lash_types::error::{Diagnostic, LashError, Location, Severity};
    use std::path::PathBuf;

    #[test]
    fn test_text_formatter_format_lash_error() {
        let formatter = TextFormatter::new(false, Verbosity::Normal);
        let error =
            LashError::parse_invalid_checkbox(PathBuf::from("test.md"), 5, 3, "[*] invalid");

        let result = formatter
            .format_lash_error(&error, Verbosity::Normal)
            .unwrap();
        assert!(result.contains("error"));
        assert!(result.contains("E_PARSE_INVALID_CHECKBOX"));
        assert!(result.contains("test.md:5:3"));
    }

    #[test]
    fn test_text_formatter_format_diagnostic_quiet() {
        let formatter = TextFormatter::new(false, Verbosity::Normal);
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

        let result = formatter
            .format_diagnostic(&diagnostic, Verbosity::Quiet)
            .unwrap();
        assert_eq!(result, "error: Test error");
    }

    #[test]
    fn test_text_formatter_format_diagnostic_normal() {
        let formatter = TextFormatter::new(false, Verbosity::Normal);
        let diagnostic = Diagnostic {
            code: "E_TEST",
            severity: Severity::Error,
            message: "Test error".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 10, 5)),
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let result = formatter
            .format_diagnostic(&diagnostic, Verbosity::Normal)
            .unwrap();
        assert!(result.contains("error[E_TEST]"));
        assert!(result.contains("Test error"));
        assert!(result.contains("test.md:10:5"));
    }

    #[test]
    fn test_text_formatter_format_diagnostic_verbose() {
        let formatter = TextFormatter::new(false, Verbosity::Normal);
        let diagnostic = Diagnostic {
            code: "E_TEST",
            severity: Severity::Error,
            message: "Test error".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 10, 5)),
            snippet: Some("- [*] invalid".to_string()),
            help: Some("Use valid syntax".to_string()),
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let result = formatter
            .format_diagnostic(&diagnostic, Verbosity::Verbose)
            .unwrap();
        assert!(result.contains("error[E_TEST]"));
        assert!(result.contains("Test error"));
        assert!(result.contains("- [*] invalid"));
        assert!(result.contains("help: Use valid syntax"));
    }

    #[test]
    fn test_text_formatter_format_error_summary() {
        let formatter = TextFormatter::new(false, Verbosity::Normal);
        let summary = ErrorSummary {
            error_count: 5,
            warning_count: 3,
            info_count: 1,
            files_affected: [PathBuf::from("test1.md"), PathBuf::from("test2.md")]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        let result = formatter.format_error_summary(&summary).unwrap();
        assert!(result.contains("5 errors"));
        assert!(result.contains("3 warnings"));
        assert!(result.contains("1 info"));
        assert!(result.contains("2 files affected"));
    }

    #[test]
    fn test_text_formatter_format_error_summary_empty() {
        let formatter = TextFormatter::new(false, Verbosity::Normal);
        let summary = ErrorSummary::default();

        let result = formatter.format_error_summary(&summary).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_json_formatter_format_diagnostic() {
        let formatter = JsonFormatter::new(false);
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

        let result = formatter
            .format_diagnostic(&diagnostic, Verbosity::Normal)
            .unwrap();
        assert!(result.contains("\"code\":\"E_TEST\""));
        assert!(result.contains("\"severity\":\"error\""));
        assert!(result.contains("\"message\":\"Test error\""));
    }

    #[test]
    fn test_json_formatter_format_error_summary() {
        let formatter = JsonFormatter::new(false);
        let summary = ErrorSummary {
            error_count: 5,
            warning_count: 3,
            ..Default::default()
        };

        let result = formatter.format_error_summary(&summary).unwrap();
        assert!(result.contains("\"error_count\":5"));
        assert!(result.contains("\"warning_count\":3"));
        assert!(result.contains("\"total_count\":8"));
    }

    #[test]
    fn test_quiet_formatter_format_diagnostic() {
        let formatter = QuietFormatter::new();
        let diagnostic = Diagnostic {
            code: "E_TEST",
            severity: Severity::Warning,
            message: "Test warning".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let result = formatter
            .format_diagnostic(&diagnostic, Verbosity::Normal)
            .unwrap();
        assert_eq!(result, "warning: Test warning");
    }

    #[test]
    fn test_quiet_formatter_format_error_summary() {
        let formatter = QuietFormatter::new();
        let summary = ErrorSummary {
            error_count: 5,
            warning_count: 3,
            ..Default::default()
        };

        let result = formatter.format_error_summary(&summary).unwrap();
        assert_eq!(result, "5 errors\n");
    }

    #[test]
    fn test_quiet_formatter_format_error_summary_no_errors() {
        let formatter = QuietFormatter::new();
        let summary = ErrorSummary {
            warning_count: 3,
            ..Default::default()
        };

        let result = formatter.format_error_summary(&summary).unwrap();
        assert_eq!(result, "");
    }
}
