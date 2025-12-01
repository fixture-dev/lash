//! Rich error formatting for human and machine consumption
//!
//! This module provides formatters for displaying errors in various formats:
//! - Human-readable text with colors and code snippets
//! - Machine-readable JSON for automation
//! - Compact single-line format for logs
//!
//! # Examples
//!
//! ```
//! use lash_types::error::LashError;
//! use lash_types::formatter::ErrorFormatter;
//! use std::path::PathBuf;
//!
//! let err = LashError::parse_invalid_checkbox(
//!     PathBuf::from("tasks.md"),
//!     10,
//!     5,
//!     "[*] invalid checkbox"
//! );
//!
//! let diag = err.to_diagnostic();
//!
//! // Format for humans
//! let formatted = ErrorFormatter::format_human(&diag, true);
//! println!("{}", formatted);
//!
//! // Format as JSON
//! let json = ErrorFormatter::format_json(&diag).unwrap();
//! ```

use crate::error::{Diagnostic, Severity};
use colored::Colorize;
use serde_json;
use std::fs;

/// Formatter for error diagnostics
pub struct ErrorFormatter;

impl ErrorFormatter {
    /// Format a diagnostic for human consumption
    #[must_use]
    ///
    /// Produces rich, colorful output with:
    /// - Error message with severity
    /// - File location
    /// - Code snippet (if available)
    /// - Help text
    ///
    /// # Arguments
    ///
    /// * `diagnostic` - The diagnostic to format
    /// * `use_color` - Whether to use ANSI color codes
    pub fn format_human(diagnostic: &Diagnostic, use_color: bool) -> String {
        let mut output = String::new();

        // Header: error[CODE]: message
        let severity_str = match diagnostic.severity {
            Severity::Error => {
                if use_color {
                    "error".red().bold().to_string()
                } else {
                    "error".to_string()
                }
            }
            Severity::Warning => {
                if use_color {
                    "warning".yellow().bold().to_string()
                } else {
                    "warning".to_string()
                }
            }
            Severity::Info => {
                if use_color {
                    "info".cyan().bold().to_string()
                } else {
                    "info".to_string()
                }
            }
            Severity::Hint => {
                if use_color {
                    "hint".blue().bold().to_string()
                } else {
                    "hint".to_string()
                }
            }
        };

        let code_str = if use_color {
            format!("[{}]", diagnostic.code.bright_black())
        } else {
            format!("[{}]", diagnostic.code)
        };

        output.push_str(&format!(
            "{}{}: {}\n",
            severity_str, code_str, diagnostic.message
        ));

        // Location
        if let Some(location) = &diagnostic.location {
            let loc_str = if use_color {
                format!("  --> {}", location.to_string().cyan())
            } else {
                format!("  --> {}", location)
            };
            output.push_str(&format!("{}\n", loc_str));

            // Try to read file and show snippet with context
            if let (Some(line), Some(col)) = (location.line, location.column) {
                if let Ok(content) = fs::read_to_string(&location.file_path) {
                    let lines: Vec<&str> = content.lines().collect();
                    if line > 0 && line <= lines.len() {
                        let line_idx = line - 1;
                        let line_num_width = line.to_string().len().max(2);

                        // Show context (line before, error line, line after)
                        output.push_str("   |\n");

                        // Line before (if available)
                        if line_idx > 0 {
                            let prev_line = lines[line_idx - 1];
                            output.push_str(&format!(
                                "{:>width$} | {}\n",
                                line - 1,
                                if use_color {
                                    prev_line.bright_black().to_string()
                                } else {
                                    prev_line.to_string()
                                },
                                width = line_num_width
                            ));
                        }

                        // Error line
                        let error_line = lines[line_idx];
                        output.push_str(&format!(
                            "{:>width$} | {}\n",
                            line,
                            error_line,
                            width = line_num_width
                        ));

                        // Caret pointing to error column
                        let caret_padding = " ".repeat(line_num_width);
                        let col_padding = " ".repeat((col - 1).max(0));
                        let caret = if use_color {
                            "^".red().bold().to_string()
                        } else {
                            "^".to_string()
                        };
                        output.push_str(&format!("{} | {}{}\n", caret_padding, col_padding, caret));

                        // Line after (if available)
                        if line_idx + 1 < lines.len() {
                            let next_line = lines[line_idx + 1];
                            output.push_str(&format!(
                                "{:>width$} | {}\n",
                                line + 1,
                                if use_color {
                                    next_line.bright_black().to_string()
                                } else {
                                    next_line.to_string()
                                },
                                width = line_num_width
                            ));
                        }

                        output.push_str("   |\n");
                    }
                } else if let Some(snippet) = &diagnostic.snippet {
                    // Fallback to snippet if file not readable
                    output.push_str(&format!("   | {}\n", snippet));
                }
            }
        }

        // Additional labels
        if let Some(labels) = &diagnostic.labels {
            for (key, value) in labels {
                let label_str = if use_color {
                    format!("  = {}: {}", key.bright_blue(), value)
                } else {
                    format!("  = {}: {}", key, value)
                };
                output.push_str(&format!("{}\n", label_str));
            }
        }

        // Help text
        if let Some(help) = &diagnostic.help {
            let help_prefix = if use_color {
                "help:".green().bold()
            } else {
                colored::ColoredString::from("help:")
            };
            output.push_str(&format!("  {} {}\n", help_prefix, help));
        }

        output
    }

    /// Format a diagnostic as JSON
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails
    pub fn format_json(diagnostic: &Diagnostic) -> serde_json::Result<String> {
        diagnostic.to_json()
    }

    /// Format a diagnostic as a compact single-line string (for logs)
    #[must_use]
    pub fn format_compact(diagnostic: &Diagnostic) -> String {
        let location = diagnostic
            .location
            .as_ref()
            .map_or_else(|| "<unknown>".to_string(), ToString::to_string);

        format!(
            "[{}] {}: {} at {}",
            diagnostic.code, diagnostic.severity, diagnostic.message, location
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{codes, Location};
    use std::path::PathBuf;

    #[test]
    fn test_format_human_basic() {
        let diag = Diagnostic {
            code: codes::E_PARSE_INVALID_CHECKBOX,
            severity: Severity::Error,
            message: "invalid checkbox syntax".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 5, 3)),
            snippet: Some("[*] bad".to_string()),
            help: Some("use [ ], [-], [x], or [!]".to_string()),
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let formatted = ErrorFormatter::format_human(&diag, false);
        assert!(formatted.contains("error"));
        assert!(formatted.contains("E_PARSE_INVALID_CHECKBOX"));
        assert!(formatted.contains("invalid checkbox syntax"));
        assert!(formatted.contains("test.md:5:3"));
        assert!(formatted.contains("help:"));
    }

    #[test]
    fn test_format_human_with_color() {
        let diag = Diagnostic {
            code: codes::E_LINT_DUPLICATE_ID,
            severity: Severity::Warning,
            message: "duplicate ID".to_string(),
            location: Some(Location::new(PathBuf::from("tasks.md"), 10, 5)),
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let formatted = ErrorFormatter::format_human(&diag, true);
        // Should contain ANSI color codes (hard to test exact codes)
        assert!(formatted.len() > 50); // Color codes add length
        assert!(formatted.contains("warning"));
        assert!(formatted.contains("E_LINT_DUPLICATE_ID"));
    }

    #[test]
    fn test_format_human_with_labels() {
        let diag = Diagnostic {
            code: codes::E_DEP_CYCLE,
            severity: Severity::Error,
            message: "circular dependency".to_string(),
            location: None,
            snippet: None,
            help: Some("break the cycle".to_string()),
            labels: Some(vec![(
                "dependency chain".to_string(),
                "A -> B -> C -> A".to_string(),
            )]),
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let formatted = ErrorFormatter::format_human(&diag, false);
        assert!(formatted.contains("dependency chain"));
        assert!(formatted.contains("A -> B -> C -> A"));
        assert!(formatted.contains("help:"));
    }

    #[test]
    fn test_format_json() {
        let diag = Diagnostic {
            code: codes::E_PARSE_INVALID_CHECKBOX,
            severity: Severity::Error,
            message: "invalid checkbox".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 5, 3)),
            snippet: Some("[*] bad".to_string()),
            help: Some("use valid syntax".to_string()),
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let json = ErrorFormatter::format_json(&diag).unwrap();
        assert!(json.contains("E_PARSE_INVALID_CHECKBOX"));
        assert!(json.contains("error"));
        assert!(json.contains("test.md"));
        assert!(json.contains("\"line\": 5"));
        assert!(json.contains("\"column\": 3"));

        // Validate it's actually valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["code"], "E_PARSE_INVALID_CHECKBOX");
        assert_eq!(parsed["severity"], "error");
    }

    #[test]
    fn test_format_compact() {
        let diag = Diagnostic {
            code: codes::E_LINT_DEPTH_EXCEEDED,
            severity: Severity::Error,
            message: "too deep".to_string(),
            location: Some(Location::new(PathBuf::from("tasks.md"), 42, 5)),
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let compact = ErrorFormatter::format_compact(&diag);
        assert!(compact.contains("[E_LINT_DEPTH_EXCEEDED]"));
        assert!(compact.contains("error"));
        assert!(compact.contains("too deep"));
        assert!(compact.contains("tasks.md:42:5"));
        // Should be single line
        assert_eq!(compact.lines().count(), 1);
    }

    #[test]
    fn test_format_compact_no_location() {
        let diag = Diagnostic {
            code: codes::E_INDEX_CORRUPTED,
            severity: Severity::Error,
            message: "database corrupted".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let compact = ErrorFormatter::format_compact(&diag);
        assert!(compact.contains("<unknown>"));
        assert!(compact.contains("database corrupted"));
    }

    #[test]
    fn test_different_severity_levels() {
        for (severity, expected) in [
            (Severity::Error, "error"),
            (Severity::Warning, "warning"),
            (Severity::Info, "info"),
            (Severity::Hint, "hint"),
        ] {
            let diag = Diagnostic {
                code: codes::E_INTERNAL,
                severity,
                message: "test".to_string(),
                location: None,
                snippet: None,
                help: None,
                labels: None,
                recovery_command: None,
                fix_steps: None,
                explanation: None,
                docs_url: None,
            };

            let formatted = ErrorFormatter::format_human(&diag, false);
            assert!(formatted.contains(expected));
        }
    }
}
