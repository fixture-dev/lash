//! Output formatting utilities for diagnostics and summaries
//!
//! This module provides functions to format linter diagnostics for human-readable
//! terminal output with colors and code snippets.

#![allow(clippy::format_push_string)]
#![allow(clippy::trivially_copy_pass_by_ref)]

use lash_core::linter::LintDiagnostic;
use lash_types::Severity;
use owo_colors::{OwoColorize, Stream};
use std::fs;
use std::io::{self, Write};

/// Format a diagnostic for human-readable terminal output
///
/// Format: `path/to/file.md:line:col: error[CODE]: message`
///
/// # Arguments
///
/// * `diagnostic` - The diagnostic to format
/// * `show_snippet` - Whether to show code snippet with error location
/// * `use_color` - Whether to use colored output
///
/// # Returns
///
/// Formatted diagnostic string
pub fn format_diagnostic(
    diagnostic: &LintDiagnostic,
    show_snippet: bool,
    use_color: bool,
) -> String {
    let mut output = String::new();

    // Format the main diagnostic line
    let severity_str = format_severity(&diagnostic.severity, use_color);
    let location = &diagnostic.location;

    if use_color {
        output.push_str(&format!(
            "{}:{}:{}: {}[{}]: {}\n",
            location.file_path.display(),
            location.line.unwrap_or(0),
            location.column.unwrap_or(0),
            severity_str,
            diagnostic.code,
            diagnostic.message
        ));
    } else {
        output.push_str(&format!(
            "{}:{}:{}: {}[{}]: {}\n",
            location.file_path.display(),
            location.line.unwrap_or(0),
            location.column.unwrap_or(0),
            severity_to_string(&diagnostic.severity),
            diagnostic.code,
            diagnostic.message
        ));
    }

    // Show code snippet if requested and available
    if show_snippet {
        if let Some(snippet) = get_code_snippet(diagnostic) {
            output.push_str(&snippet);
            output.push('\n');
        }
    }

    // Show help text if available
    if let Some(help) = &diagnostic.help {
        if use_color {
            output.push_str(&format!(
                "  {}: {}\n",
                "help".if_supports_color(Stream::Stdout, |text| text.blue()),
                help
            ));
        } else {
            output.push_str(&format!("  help: {help}\n"));
        }
    }

    output
}

/// Format severity level with color
fn format_severity(severity: &Severity, use_color: bool) -> String {
    let text = severity_to_string(severity);

    if use_color {
        match severity {
            Severity::Error => text
                .if_supports_color(Stream::Stdout, |t| t.red())
                .to_string(),
            Severity::Warning => text
                .if_supports_color(Stream::Stdout, |t| t.yellow())
                .to_string(),
            Severity::Info => text
                .if_supports_color(Stream::Stdout, |t| t.blue())
                .to_string(),
            Severity::Hint => text
                .if_supports_color(Stream::Stdout, |t| t.cyan())
                .to_string(),
        }
    } else {
        text
    }
}

/// Convert severity to string
fn severity_to_string(severity: &Severity) -> String {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Hint => "hint",
    }
    .to_string()
}

/// Get a code snippet showing the error location
fn get_code_snippet(diagnostic: &LintDiagnostic) -> Option<String> {
    // If the diagnostic already has a snippet, use it
    if let Some(snippet) = &diagnostic.snippet {
        return Some(format!("  {snippet}"));
    }

    // Otherwise, try to read the file and extract the relevant lines
    let location = &diagnostic.location;
    let content = fs::read_to_string(&location.file_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    let line = location.line?;
    let column = location.column?;

    if line == 0 || line > lines.len() {
        return None;
    }

    let line_idx = line - 1; // Convert to 0-based index
    let line_text = lines[line_idx];

    // Show the line with a pointer to the error location
    let mut snippet = String::new();
    snippet.push_str(&format!("  {line} | {line_text}\n"));

    // Add a pointer line
    let pointer_offset = column.saturating_sub(1);
    let pointer = format!(
        "  {} | {}^\n",
        " ".repeat(line.to_string().len()),
        " ".repeat(pointer_offset)
    );
    snippet.push_str(&pointer);

    Some(snippet)
}

/// Print a summary of linting results
///
/// # Arguments
///
/// * `diagnostics` - List of all diagnostics
/// * `files_checked` - Number of files checked
/// * `use_color` - Whether to use colored output
pub fn print_summary(diagnostics: &[LintDiagnostic], files_checked: usize, use_color: bool) {
    let error_count = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warning_count = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    let info_count = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Info)
        .count();
    let hint_count = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Hint)
        .count();

    // If no errors, print success message
    if error_count == 0 {
        if warning_count == 0 && info_count == 0 && hint_count == 0 {
            // Perfect - no issues at all
            if use_color {
                println!("{}", "✓ All files passed linting".green());
            } else {
                println!("✓ All files passed linting");
            }
            return;
        }
        // No errors, but some warnings/info/hints
        if use_color {
            println!("{}", "✓ Linting passed (with warnings)".green());
        } else {
            println!("✓ Linting passed (with warnings)");
        }
    }

    let mut summary = format!("\nChecked {files_checked} files: ");

    if use_color {
        let error_str = if error_count > 0 {
            format!(
                "{}",
                error_count.if_supports_color(Stream::Stdout, |n| n.red())
            )
        } else {
            format!(
                "{}",
                error_count.if_supports_color(Stream::Stdout, |n| n.green())
            )
        };

        let warning_str = if warning_count > 0 {
            format!(
                "{}",
                warning_count.if_supports_color(Stream::Stdout, |n| n.yellow())
            )
        } else {
            format!("{warning_count}")
        };

        let info_str = if info_count > 0 {
            format!(
                "{}",
                info_count.if_supports_color(Stream::Stdout, |n| n.blue())
            )
        } else {
            format!("{info_count}")
        };

        let hint_str = if hint_count > 0 {
            format!(
                "{}",
                hint_count.if_supports_color(Stream::Stdout, |n| n.cyan())
            )
        } else {
            format!("{hint_count}")
        };

        summary.push_str(&format!(
            "{error_str} errors, {warning_str} warnings, {info_str} info, {hint_str} hints"
        ));
    } else {
        summary.push_str(&format!(
            "{error_count} errors, {warning_count} warnings, {info_count} info, {hint_count} hints"
        ));
    }

    eprintln!("{summary}");
}

/// Create a progress bar for file processing
///
/// # Arguments
///
/// * `total_files` - Total number of files to process
///
/// # Returns
///
/// A configured progress bar
pub fn create_progress_bar(total_files: usize) -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new(total_files as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Invalid progress bar template")
            .progress_chars("#>-"),
    );
    pb
}

/// Format diagnostics as JSON
///
/// # Arguments
///
/// * `diagnostics` - List of diagnostics
/// * `files_checked` - Number of files checked
///
/// # Returns
///
/// JSON string
pub fn format_json_output(
    diagnostics: &[LintDiagnostic],
    files_checked: usize,
) -> Result<String, serde_json::Error> {
    let error_count = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warning_count = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    let info_count = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Info)
        .count();
    let hint_count = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Hint)
        .count();

    let output = serde_json::json!({
        "diagnostics": diagnostics,
        "summary": {
            "files_checked": files_checked,
            "errors": error_count,
            "warnings": warning_count,
            "info": info_count,
            "hints": hint_count,
        }
    });

    serde_json::to_string_pretty(&output)
}

/// Print diagnostics to stdout
///
/// # Arguments
///
/// * `diagnostics` - List of diagnostics to print
/// * `use_color` - Whether to use colored output
/// * `show_snippets` - Whether to show code snippets
pub fn print_diagnostics(
    diagnostics: &[LintDiagnostic],
    use_color: bool,
    show_snippets: bool,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for diagnostic in diagnostics {
        let formatted = format_diagnostic(diagnostic, show_snippets, use_color);
        write!(handle, "{formatted}")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    #[test]
    fn test_severity_to_string() {
        assert_eq!(severity_to_string(&Severity::Error), "error");
        assert_eq!(severity_to_string(&Severity::Warning), "warning");
        assert_eq!(severity_to_string(&Severity::Info), "info");
        assert_eq!(severity_to_string(&Severity::Hint), "hint");
    }

    #[test]
    fn test_format_diagnostic_no_color() {
        let diag = LintDiagnostic::error(
            "E_TEST",
            "Test error message",
            PathBuf::from("test.md"),
            10,
            5,
        );

        let output = format_diagnostic(&diag, false, false);
        assert!(output.contains("test.md:10:5"));
        assert!(output.contains("error[E_TEST]"));
        assert!(output.contains("Test error message"));
    }

    #[test]
    fn test_format_diagnostic_with_help() {
        let mut diag = LintDiagnostic::error(
            "E_TEST",
            "Test error message",
            PathBuf::from("test.md"),
            10,
            5,
        );
        diag.help = Some("Try fixing this way".to_string());

        let output = format_diagnostic(&diag, false, false);
        assert!(output.contains("help: Try fixing this way"));
    }

    #[test]
    fn test_format_json_output() {
        let diag1 = LintDiagnostic::error("E_TEST", "Error", PathBuf::from("test.md"), 1, 1);
        let diag2 = LintDiagnostic::warning("W_TEST", "Warning", PathBuf::from("test.md"), 2, 1);

        let json = format_json_output(&[diag1, diag2], 1).unwrap();
        assert!(json.contains("\"diagnostics\""));
        assert!(json.contains("\"summary\""));
        assert!(json.contains("\"errors\": 1"));
        assert!(json.contains("\"warnings\": 1"));
    }
}
