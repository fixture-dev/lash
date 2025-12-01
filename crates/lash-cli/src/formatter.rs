//! Output formatting system
//!
//! This module provides flexible output formatting supporting human-readable text,
//! JSON, and quiet modes. It respects terminal capabilities and environment variables.

use anyhow::Result;
use lash_core::logo::LOGO;
use lash_types::TaskStatus;
use serde::Serialize;

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
