//! Output formatting system
//!
//! This module provides flexible output formatting supporting human-readable text,
//! JSON, and quiet modes. It respects terminal capabilities and environment variables.

use anyhow::Result;
use owo_colors::{OwoColorize, Stream, Style};
use serde::Serialize;

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
    use_color: bool,
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
        Self {
            use_color: use_color && supports_color(),
            verbosity,
        }
    }

    /// Check if colors are enabled
    #[must_use]
    pub fn has_color(&self) -> bool {
        self.use_color
    }

    /// Get the verbosity level
    #[must_use]
    pub fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    /// Format text with color if enabled
    fn colorize(&self, text: &str, style: Style) -> String {
        if self.use_color {
            text.if_supports_color(Stream::Stdout, |t| t.style(style))
                .to_string()
        } else {
            text.to_string()
        }
    }
}

impl OutputFormatter for TextFormatter {
    fn format_success(&self, message: &str) -> Result<String> {
        Ok(self.colorize(message, Style::new().green()))
    }

    fn format_error(&self, message: &str) -> Result<String> {
        let formatted = if self.use_color {
            format!(
                "{}: {}",
                "error".if_supports_color(Stream::Stderr, |t| t.style(Style::new().red().bold())),
                message
            )
        } else {
            format!("error: {message}")
        };
        Ok(formatted)
    }

    fn format_warning(&self, message: &str) -> Result<String> {
        let formatted = if self.use_color {
            format!(
                "{}: {}",
                "warning"
                    .if_supports_color(Stream::Stdout, |t| t.style(Style::new().yellow().bold())),
                message
            )
        } else {
            format!("warning: {message}")
        };
        Ok(formatted)
    }

    fn format_info(&self, message: &str) -> Result<String> {
        let formatted = if self.use_color {
            format!(
                "{}: {}",
                "info".if_supports_color(Stream::Stdout, |t| t.style(Style::new().blue())),
                message
            )
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
}
