//! Lint command implementation
//!
//! The `lash lint` command validates Lash task files for syntax and semantic correctness.

#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::similar_names)]
#![allow(clippy::trivially_copy_pass_by_ref)]

use anyhow::Context;
use lash_cli::command::Command;
use lash_cli::context::Context as CliContext;
use lash_cli::theme::CliTheme;
use lash_core::linter::{register_default_rules, LintConfig, LintDiagnostic};
use lash_core::parser::parse_file;
use lash_types::{error::Result as LashResult, LashConfig, Severity, TaskFile};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::instrument;

use crate::utils::file_discovery::{discover_markdown_files, find_project_root};
use crate::utils::output::{
    create_progress_bar, format_json_output, print_diagnostics, print_summary,
};

/// Arguments for the lint command
#[derive(Debug, Clone)]
pub struct LintArgs {
    /// Paths to lint (files or directories)
    pub paths: Vec<PathBuf>,
    /// Output JSON diagnostics
    pub json: bool,
    /// Apply auto-fixes
    pub fix: bool,
    /// Run only specific rule(s)
    pub rules: Vec<String>,
    /// Only show errors of this severity or higher
    pub min_severity: Option<Severity>,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
}

impl Command for LintArgs {
    /// Execute the lint command
    ///
    /// # Arguments
    ///
    /// * `ctx` - Shared command context
    ///
    /// # Returns
    ///
    /// `Ok(())` on success (no lint errors found), or a `LashError` on failure or if lint errors are found
    #[instrument(skip(self, ctx), fields(paths = ?self.paths, fix = self.fix, rules = ?self.rules))]
    fn execute(&self, ctx: &CliContext) -> LashResult<()> {
        // For now, use the project config from context
        // In the future, we'll load config more intelligently
        let _ = ctx; // Suppress unused variable warning for now

        // Call the public execute function and convert result
        match execute(self.clone()) {
            Ok(0) => Ok(()),
            Ok(2) => Err(lash_types::error::LashError::internal(
                "Lint errors found",
                Some("Run lash lint to see details".to_string()),
            )),
            Ok(code) => Err(lash_types::error::LashError::internal(
                format!("Unexpected exit code: {code}"),
                None,
            )),
            Err(e) => Err(lash_types::error::LashError::internal(
                format!("Lint command failed: {e}"),
                None,
            )),
        }
    }
}

/// Execute the lint command (public interface for main.rs)
///
/// # Arguments
///
/// * `args` - Lint command arguments
///
/// # Returns
///
/// Exit code: 0 (no errors), 1 (general error), 2 (lint errors found)
#[instrument(skip(args), fields(paths = ?args.paths, fix = args.fix, rules = ?args.rules))]
pub fn execute(args: LintArgs) -> anyhow::Result<i32> {
    // Load theme based on no_color flag
    let theme = CliTheme::load(None, !args.no_color)?;

    // Determine paths to lint
    let paths = if args.paths.is_empty() {
        // No paths specified - lint entire project
        let project_root = if let Some(ref root) = args.project_root {
            root.clone()
        } else {
            let cwd = std::env::current_dir()
                .map_err(|e| anyhow::anyhow!("Failed to get current directory: {e}"))?;
            find_project_root(&cwd)
        };
        vec![project_root]
    } else {
        args.paths.clone()
    };

    // Discover markdown files
    let files = discover_markdown_files(&paths, true).context("Failed to discover files")?;
    tracing::info!(file_count = files.len(), "Discovered files to lint");

    if files.is_empty() {
        let msg = "No markdown files found to lint";
        if let Some(t) = &theme {
            eprintln!("{}", t.style_warning(msg));
        } else {
            eprintln!("{msg}");
        }
        return Ok(0);
    }

    // Load project configuration
    let project_config = load_project_config(&files)?;

    // Configure linter
    let lint_config = configure_linter(&args);

    // Parse all files first
    let (parsed_files, parse_errors) = parse_files(&files, &project_config, &args, theme.as_ref())?;

    // Lint all files
    let mut diagnostics = lint_files(&parsed_files, &project_config, &lint_config, &args)?;

    // Add parse errors to diagnostics
    diagnostics.extend(parse_errors);

    // Filter diagnostics by severity if requested
    let filtered_diagnostics = filter_by_severity(diagnostics, args.min_severity);

    // Apply auto-fixes if requested
    if args.fix {
        apply_fixes(&filtered_diagnostics, &parsed_files, theme.as_ref())?;
    }

    // Output results
    if args.json {
        let json = format_json_output(&filtered_diagnostics, files.len())?;
        println!("{json}");
    } else {
        // Print diagnostics in human-readable format
        print_diagnostics(&filtered_diagnostics, theme.as_ref(), true)?;

        // Print summary
        if !filtered_diagnostics.is_empty() || files.len() > 1 {
            print_summary(&filtered_diagnostics, files.len(), theme.as_ref());
        }
    }

    // Determine exit code
    let has_errors = filtered_diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);

    Ok(if has_errors { 2 } else { 0 })
}

/// Load project configuration
fn load_project_config(files: &[PathBuf]) -> anyhow::Result<LashConfig> {
    // Try to find .lash/config.toml in project root
    if let Some(first_file) = files.first() {
        if let Some(parent) = first_file.parent() {
            let project_root = find_project_root(parent);
            let config_path = project_root.join(".lash").join("config.toml");

            if config_path.exists() {
                let config_str =
                    std::fs::read_to_string(&config_path).context("Failed to read config file")?;
                let config: LashConfig =
                    toml::from_str(&config_str).context("Failed to parse config file")?;
                return Ok(config);
            }
        }
    }

    // Use default config if no config file found
    Ok(LashConfig::default())
}

/// Configure the linter based on command arguments
fn configure_linter(args: &LintArgs) -> LintConfig {
    let mut config = LintConfig::default();

    // If specific rules are requested, disable all others
    if !args.rules.is_empty() {
        config.enabled_rules.clear();
        for rule in &args.rules {
            config.enabled_rules.insert(rule.clone());
        }
    }

    config.auto_fix = args.fix;

    config
}

/// Parse all files with progress reporting
///
/// Returns a tuple of (successfully parsed files, parse error diagnostics)
#[instrument(skip(files, config, args, theme), fields(file_count = files.len()))]
fn parse_files(
    files: &[PathBuf],
    config: &LashConfig,
    args: &LintArgs,
    theme: Option<&CliTheme>,
) -> anyhow::Result<(HashMap<PathBuf, TaskFile>, Vec<LintDiagnostic>)> {
    let mut parsed_files = HashMap::new();
    let mut parse_errors = Vec::new();

    let show_progress = !args.json && files.len() > 1;
    let pb = if show_progress {
        Some(create_progress_bar(files.len()))
    } else {
        None
    };

    for file_path in files {
        if let Some(ref pb) = pb {
            pb.set_message(format!("Parsing {}", file_path.display()));
        }

        match parse_file(file_path, config) {
            Ok(task_file) => {
                parsed_files.insert(file_path.clone(), task_file);
            }
            Err(e) => {
                if let Some(ref pb) = pb {
                    pb.finish_and_clear();
                }

                // Format the error message with theme
                let file_str = if let Some(t) = theme {
                    t.style_muted(&file_path.display().to_string())
                } else {
                    file_path.display().to_string()
                };

                let error_label = if let Some(t) = theme {
                    t.style_error("Error parsing")
                } else {
                    "Error parsing".to_string()
                };

                eprintln!("{error_label} {file_str}: {e}");

                // Create a lint diagnostic for the parse error
                parse_errors.push(LintDiagnostic::error(
                    "E_PARSE",
                    format!("Failed to parse file: {e}"),
                    file_path.clone(),
                    1,
                    1,
                ));

                // Continue parsing other files even if one fails
            }
        }

        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    tracing::info!(
        parsed_count = parsed_files.len(),
        parse_errors = parse_errors.len(),
        "File parsing complete"
    );
    Ok((parsed_files, parse_errors))
}

/// Lint all files with progress reporting
#[instrument(skip(parsed_files, project_config, lint_config, args), fields(file_count = parsed_files.len()))]
fn lint_files(
    parsed_files: &HashMap<PathBuf, TaskFile>,
    project_config: &LashConfig,
    lint_config: &LintConfig,
    args: &LintArgs,
) -> anyhow::Result<Vec<LintDiagnostic>> {
    // Create linter with all rules
    let registry = register_default_rules();
    let linter = registry.create_linter(lint_config.clone());

    let show_progress = !args.json && parsed_files.len() > 1;
    let pb = if show_progress {
        Some(create_progress_bar(parsed_files.len()))
    } else {
        None
    };

    // Lint all files in the project (enables cross-file validation)
    if let Some(ref pb) = pb {
        pb.set_message("Running linter...".to_string());
    }

    let mut all_diagnostics = linter.lint_project(parsed_files, project_config);

    if let Some(ref pb) = pb {
        pb.inc(parsed_files.len() as u64);
    }

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    // Sort diagnostics by file, then line, then column
    all_diagnostics.sort_by(|a, b| {
        a.location
            .file_path
            .cmp(&b.location.file_path)
            .then(a.location.line.cmp(&b.location.line))
            .then(a.location.column.cmp(&b.location.column))
    });

    tracing::info!(
        diagnostic_count = all_diagnostics.len(),
        error_count = all_diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count(),
        warning_count = all_diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count(),
        "Linting complete"
    );

    Ok(all_diagnostics)
}

/// Filter diagnostics by minimum severity level
fn filter_by_severity(
    diagnostics: Vec<LintDiagnostic>,
    min_severity: Option<Severity>,
) -> Vec<LintDiagnostic> {
    if let Some(min) = min_severity {
        diagnostics
            .into_iter()
            .filter(|d| meets_severity_threshold(&d.severity, &min))
            .collect()
    } else {
        diagnostics
    }
}

/// Check if a severity meets the minimum threshold
fn meets_severity_threshold(severity: &Severity, min: &Severity) -> bool {
    match min {
        Severity::Error => *severity == Severity::Error,
        Severity::Warning => matches!(severity, Severity::Error | Severity::Warning),
        Severity::Info => !matches!(severity, Severity::Hint), // Show all except hints
        Severity::Hint => true,                                // Show all
    }
}

/// Apply auto-fixes to files
#[instrument(skip(diagnostics, parsed_files, theme), fields(diagnostic_count = diagnostics.len()))]
fn apply_fixes(
    diagnostics: &[LintDiagnostic],
    parsed_files: &HashMap<PathBuf, TaskFile>,
    theme: Option<&CliTheme>,
) -> anyhow::Result<()> {
    // Group diagnostics by file
    let mut fixes_by_file: HashMap<&PathBuf, Vec<&LintDiagnostic>> = HashMap::new();

    for diagnostic in diagnostics {
        if diagnostic.fix.is_some() {
            fixes_by_file
                .entry(&diagnostic.location.file_path)
                .or_default()
                .push(diagnostic);
        }
    }

    if fixes_by_file.is_empty() {
        let msg = "No auto-fixes available";
        if let Some(t) = theme {
            eprintln!("{}", t.style_warning(msg));
        } else {
            eprintln!("{msg}");
        }
        return Ok(());
    }

    let info_msg = format!("Applying fixes to {} file(s)...", fixes_by_file.len());
    if let Some(t) = theme {
        eprintln!("{}", t.style_info(&info_msg));
    } else {
        eprintln!("{info_msg}");
    }

    for (file_path, file_diagnostics) in fixes_by_file {
        let file_str = if let Some(t) = theme {
            t.style_muted(&file_path.display().to_string())
        } else {
            file_path.display().to_string()
        };

        eprintln!("  Fixing {file_str}: {} fixes", file_diagnostics.len());

        // For now, we'll use the formatter to apply fixes
        // A more sophisticated implementation would apply individual fixes
        if let Some(task_file) = parsed_files.get(file_path) {
            let config = LashConfig::default();
            let options = lash_core::formatter::FormatOptions::default();
            let formatter = lash_core::formatter::Formatter::new(config, options);

            match formatter.format_file(task_file) {
                Ok(formatted) => {
                    std::fs::write(file_path, formatted)
                        .with_context(|| format!("Failed to write {}", file_path.display()))?;
                }
                Err(e) => {
                    let warning_label = if let Some(t) = theme {
                        t.style_warning("Warning")
                    } else {
                        "Warning".to_string()
                    };
                    eprintln!("  {warning_label}: Failed to format {file_str}: {e}");
                }
            }
        }
    }

    let success_msg = "Fixes applied successfully";
    if let Some(t) = theme {
        eprintln!("{}", t.style_success(success_msg));
    } else {
        eprintln!("{success_msg}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meets_severity_threshold() {
        // Error threshold only shows errors
        assert!(meets_severity_threshold(&Severity::Error, &Severity::Error));
        assert!(!meets_severity_threshold(
            &Severity::Warning,
            &Severity::Error
        ));
        assert!(!meets_severity_threshold(&Severity::Info, &Severity::Error));
        assert!(!meets_severity_threshold(&Severity::Hint, &Severity::Error));

        // Warning threshold shows errors and warnings
        assert!(meets_severity_threshold(
            &Severity::Error,
            &Severity::Warning
        ));
        assert!(meets_severity_threshold(
            &Severity::Warning,
            &Severity::Warning
        ));
        assert!(!meets_severity_threshold(
            &Severity::Info,
            &Severity::Warning
        ));
        assert!(!meets_severity_threshold(
            &Severity::Hint,
            &Severity::Warning
        ));

        // Info threshold shows error, warning, info (not hint)
        assert!(meets_severity_threshold(&Severity::Error, &Severity::Info));
        assert!(meets_severity_threshold(
            &Severity::Warning,
            &Severity::Info
        ));
        assert!(meets_severity_threshold(&Severity::Info, &Severity::Info));
        assert!(!meets_severity_threshold(&Severity::Hint, &Severity::Info));

        // Hint threshold shows everything
        assert!(meets_severity_threshold(&Severity::Error, &Severity::Hint));
        assert!(meets_severity_threshold(
            &Severity::Warning,
            &Severity::Hint
        ));
        assert!(meets_severity_threshold(&Severity::Info, &Severity::Hint));
        assert!(meets_severity_threshold(&Severity::Hint, &Severity::Hint));
    }

    #[test]
    fn test_filter_by_severity() {
        let diagnostics = vec![
            LintDiagnostic::error("E1", "Error", PathBuf::from("test.md"), 1, 1),
            LintDiagnostic::warning("W1", "Warning", PathBuf::from("test.md"), 2, 1),
            LintDiagnostic::info("I1", "Info", PathBuf::from("test.md"), 3, 1),
        ];

        // Filter to errors only
        let filtered = filter_by_severity(diagnostics.clone(), Some(Severity::Error));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].code, "E1");

        // Filter to warnings and errors
        let filtered = filter_by_severity(diagnostics.clone(), Some(Severity::Warning));
        assert_eq!(filtered.len(), 2);

        // No filter shows all
        let filtered = filter_by_severity(diagnostics.clone(), None);
        assert_eq!(filtered.len(), 3);
    }
}
