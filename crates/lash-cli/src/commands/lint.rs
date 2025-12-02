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
use lash_cli::diff_display::DiffDisplay;
use lash_cli::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash_cli::error_validator::ErrorValidator;
use lash_cli::formatter::{OutputFormat, Verbosity};
use lash_cli::theme::CliTheme;
use lash_core::linter::{register_default_rules, FixApplicator, LintConfig, LintDiagnostic};
use lash_core::parser::parse_file;
use lash_types::error::Diagnostic;
use lash_types::{error::Result as LashResult, LashConfig, Severity, TaskFile};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::instrument;

use crate::utils::file_discovery::{discover_markdown_files, find_project_root};
use crate::utils::output::create_progress_bar;

/// User's choice when prompted in interactive mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InteractiveChoice {
    /// Apply fixes to this file
    Yes,
    /// Skip this file
    No,
    /// Apply all remaining fixes without prompting
    All,
    /// Quit without applying more fixes
    Quit,
}

/// Prompt the user for a decision in interactive mode
///
/// Returns `None` if stdin is not a TTY or reading fails.
fn prompt_user(file_path: &PathBuf, fix_count: usize) -> Option<InteractiveChoice> {
    // Check if stdin is a TTY
    if !atty::is(atty::Stream::Stdin) {
        return None;
    }

    // Display the prompt
    eprint!(
        "\nApply {} fix(es) to {}? [y]es, [n]o, [a]ll, [q]uit: ",
        fix_count,
        file_path.display()
    );
    io::stderr().flush().ok()?;

    // Read user input
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;

    // Parse the response
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" | "" => Some(InteractiveChoice::Yes), // Default to yes if user just presses Enter
        "n" | "no" => Some(InteractiveChoice::No),
        "a" | "all" => Some(InteractiveChoice::All),
        "q" | "quit" => Some(InteractiveChoice::Quit),
        _ => {
            eprintln!("Invalid choice. Please enter y, n, a, or q.");
            prompt_user(file_path, fix_count) // Retry
        }
    }
}

/// Arguments for the lint command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct LintArgs {
    /// Paths to lint (files or directories)
    pub paths: Vec<PathBuf>,
    /// Output JSON diagnostics
    pub json: bool,
    /// Apply auto-fixes
    pub fix: bool,
    /// Confirm each fix before applying (requires fix)
    pub interactive: bool,
    /// Show fix suggestions without applying them
    pub suggest: bool,
    /// Run only specific rule(s)
    pub rules: Vec<String>,
    /// Only show errors of this severity or higher
    pub min_severity: Option<Severity>,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Verbosity level for output
    pub verbosity: Verbosity,
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

/// Convert a `LintDiagnostic` from lash-core to a `Diagnostic` from lash-types
fn lint_diagnostic_to_diagnostic(lint_diag: &LintDiagnostic) -> Diagnostic {
    Diagnostic {
        code: lint_diag.code,
        severity: lint_diag.severity,
        message: lint_diag.message.clone(),
        location: Some(lint_diag.location.clone()),
        snippet: lint_diag.snippet.clone(),
        help: lint_diag.help.clone(),
        labels: lint_diag.labels.clone(),
        recovery_command: lint_diag.recovery_command.clone(),
        fix_steps: lint_diag.fix_steps.clone(),
        explanation: lint_diag.explanation.clone(),
        docs_url: None, // LintDiagnostic doesn't have docs_url field
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
    let mut all_diagnostics = lint_files(&parsed_files, &project_config, &lint_config, &args)?;

    // Add parse errors to diagnostics
    all_diagnostics.extend(parse_errors);

    // Filter diagnostics by severity if requested
    let filtered_diagnostics = filter_by_severity(all_diagnostics, args.min_severity);

    // Warn if --interactive is used without --fix
    if args.interactive && !args.fix {
        let warning_msg = "Warning: --interactive flag has no effect without --fix";
        if let Some(t) = &theme {
            eprintln!("{}", t.style_warning(warning_msg));
        } else {
            eprintln!("{warning_msg}");
        }
    }

    // Apply auto-fixes if requested
    if args.fix {
        apply_fixes(
            &filtered_diagnostics,
            &parsed_files,
            theme.as_ref(),
            &project_config,
            args.interactive,
        )?;
    }

    // Convert LintDiagnostic to Diagnostic for output
    let diagnostics: Vec<Diagnostic> = filtered_diagnostics
        .iter()
        .map(lint_diagnostic_to_diagnostic)
        .collect();

    // Output results
    if args.json {
        // JSON output to stdout
        output_json_diagnostics(&diagnostics, files.len(), args.suggest)?;
    } else {
        // Text output to stdout
        output_text_diagnostics(
            &diagnostics,
            files.len(),
            theme.as_ref(),
            args.verbosity,
            args.suggest,
        )?;
    }

    // Determine exit code based on errors
    let has_errors = diagnostics.iter().any(|d| d.severity == Severity::Error);

    Ok(if has_errors { 2 } else { 0 })
}

/// Output diagnostics in JSON format to stdout
fn output_json_diagnostics(
    diagnostics: &[Diagnostic],
    files_checked: usize,
    _suggest: bool,
) -> anyhow::Result<()> {
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

    // Count fixable diagnostics
    let fixable_count = diagnostics
        .iter()
        .filter(|d| d.fix_steps.is_some() || d.recovery_command.is_some())
        .count();

    let output = serde_json::json!({
        "diagnostics": diagnostics,
        "summary": {
            "files_checked": files_checked,
            "errors": error_count,
            "warnings": warning_count,
            "info": info_count,
            "hints": hint_count,
            "fixable": fixable_count,
        }
    });

    let json_str = serde_json::to_string_pretty(&output)?;
    println!("{json_str}");

    Ok(())
}

/// Output diagnostics in human-readable text format to stdout
fn output_text_diagnostics(
    diagnostics: &[Diagnostic],
    files_checked: usize,
    theme: Option<&CliTheme>,
    verbosity: Verbosity,
    suggest: bool,
) -> anyhow::Result<()> {
    // Create an ErrorReporter to format the diagnostics
    let reporter_config = ErrorReporterConfig {
        verbosity,
        output_format: OutputFormat::Text,
        display_mode: ErrorDisplayMode::Batch,
        theme: theme.cloned(),
        show_summary: false, // We'll print our own summary
    };

    let mut reporter = ErrorReporter::new(reporter_config);

    // Collect all diagnostics
    for diagnostic in diagnostics {
        reporter.report_diagnostic(diagnostic);
    }

    // Print diagnostics to stdout instead of stderr
    for diagnostic in diagnostics {
        let formatted = reporter.format_diagnostic(diagnostic);
        println!("{formatted}");

        // If --suggest is set, show fix suggestions
        if suggest {
            print_suggestion(diagnostic, theme);
        }
    }

    // Print summary
    print_summary(diagnostics, files_checked, theme, suggest);

    Ok(())
}

/// Print fix suggestion for a diagnostic
fn print_suggestion(diagnostic: &Diagnostic, theme: Option<&CliTheme>) {
    // Only show suggestions if there are fix steps or recovery commands
    if diagnostic.fix_steps.is_none() && diagnostic.recovery_command.is_none() {
        return;
    }

    println!();

    let suggest_label = if let Some(t) = theme {
        t.style_info("  Suggestion:")
    } else {
        "  Suggestion:".to_string()
    };
    println!("{suggest_label}");

    // Show recovery command if available
    if let Some(recovery) = &diagnostic.recovery_command {
        let cmd_label = if let Some(t) = theme {
            t.style_label("    Command:")
        } else {
            "    Command:".to_string()
        };
        println!("  {cmd_label} {recovery}");
    }

    // Show fix steps if available
    if let Some(steps) = &diagnostic.fix_steps {
        let steps_label = if let Some(t) = theme {
            t.style_label("    Steps:")
        } else {
            "    Steps:".to_string()
        };
        println!("  {steps_label}");
        for (i, step) in steps.iter().enumerate() {
            println!("      {}. {step}", i + 1);
        }
    }

    // Show explanation if available
    if let Some(explanation) = &diagnostic.explanation {
        let explanation_label = if let Some(t) = theme {
            t.style_label("    Why:")
        } else {
            "    Why:".to_string()
        };
        println!("  {explanation_label} {explanation}");
    }
}

/// Print a summary of linting results to stdout
fn print_summary(
    diagnostics: &[Diagnostic],
    _files_checked: usize,
    theme: Option<&CliTheme>,
    suggest: bool,
) {
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
            let msg = "✓ All files passed linting";
            if let Some(t) = theme {
                println!("{}", t.style_success(msg));
            } else {
                println!("{msg}");
            }
            return;
        }
        // No errors, but some warnings/info/hints
        let msg = "✓ Linting passed (with warnings)";
        if let Some(t) = theme {
            println!("{}", t.style_success(msg));
        } else {
            println!("{msg}");
        }
    }

    // Print detailed summary
    println!("\nSummary:");

    if let Some(t) = theme {
        let error_str = if error_count > 0 {
            t.style_error(&error_count.to_string())
        } else {
            t.style_success(&error_count.to_string())
        };

        let warning_str = if warning_count > 0 {
            t.style_warning(&warning_count.to_string())
        } else {
            warning_count.to_string()
        };

        println!(
            "  {error_str} errors, {warning_str} warnings, {info_count} info, {hint_count} hints"
        );
    } else {
        println!("  {error_count} errors, {warning_count} warnings, {info_count} info, {hint_count} hints");
    }

    // If suggest mode is enabled, show fixable count
    if suggest {
        let fixable_count = diagnostics
            .iter()
            .filter(|d| d.fix_steps.is_some() || d.recovery_command.is_some())
            .count();

        if fixable_count > 0 {
            let fixable_str = if let Some(t) = theme {
                t.style_info(&fixable_count.to_string())
            } else {
                fixable_count.to_string()
            };
            println!("  {fixable_str} fixable");
        }
    }

    // Count unique files affected
    let files_affected: std::collections::HashSet<_> = diagnostics
        .iter()
        .filter_map(|d| d.location.as_ref())
        .map(|loc| &loc.file_path)
        .collect();

    if !files_affected.is_empty() {
        println!("  {} files affected", files_affected.len());
    }
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
#[instrument(skip(files, config, args, _theme), fields(file_count = files.len()))]
fn parse_files(
    files: &[PathBuf],
    config: &LashConfig,
    args: &LintArgs,
    _theme: Option<&CliTheme>,
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

                // Create a lint diagnostic for the parse error
                // (will be reported later with other diagnostics)
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

/// Result of applying fixes to a single file
struct FileFixResult {
    /// Number of fixes applied
    fixes_applied: usize,
    /// Whether the file still has errors
    has_errors: bool,
    /// Number of new errors introduced (if any)
    new_error_count: usize,
}

/// Apply fixes to a single file with iteration
///
/// This function iterates up to `MAX_FIX_ITERATIONS` times, applying fixes
/// and validating after each iteration.
fn apply_fixes_to_file(
    file_path: &PathBuf,
    initial_diagnostics: Vec<&LintDiagnostic>,
    project_config: &LashConfig,
    theme: Option<&CliTheme>,
) -> anyhow::Result<FileFixResult> {
    const MAX_FIX_ITERATIONS: usize = 3;

    let mut file_fixes_applied = 0;
    let mut current_diagnostics: Vec<LintDiagnostic> =
        initial_diagnostics.iter().copied().cloned().collect();
    let mut iteration = 0;
    let mut final_has_errors = false;
    let mut final_new_error_count = 0;

    // Iterate fix application
    while iteration < MAX_FIX_ITERATIONS {
        iteration += 1;

        // Filter to diagnostics with fixes
        let fixable: Vec<LintDiagnostic> = current_diagnostics
            .iter()
            .filter(|d| d.fix.is_some())
            .cloned()
            .collect();

        if fixable.is_empty() {
            if iteration == 1 {
                eprintln!("    No fixable errors");
            }
            break;
        }

        // Read current file content
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;

        // Apply fixes using FixApplicator
        let applicator = FixApplicator::new(&content);
        let result = applicator.apply_fixes(&fixable);

        if result.applied_count == 0 && result.skipped_fixes.is_empty() {
            eprintln!("    Iteration {iteration}: No fixes could be applied");
            break;
        }

        // Report what was applied and skipped
        if result.applied_count > 0 {
            eprintln!(
                "    Iteration {iteration}: Applied {} fix(es)",
                result.applied_count
            );
            file_fixes_applied += result.applied_count;
        }

        for skipped in &result.skipped_fixes {
            eprintln!("    Skipped: {} ({})", skipped.description, skipped.reason);
        }

        // Write fixed content back to file
        std::fs::write(file_path, &result.fixed_content)
            .with_context(|| format!("Failed to write {}", file_path.display()))?;

        // Validate the fixed content
        let validator = ErrorValidator::with_config(project_config.clone());
        let validation_result = validator
            .validate_content(file_path, &result.fixed_content, &current_diagnostics)
            .with_context(|| format!("Failed to validate {}", file_path.display()))?;

        // Report validation results
        if validation_result.fixed_count() > 0 {
            eprintln!("    Fixed {} error(s)", validation_result.fixed_count());
        }

        if !validation_result.remaining_errors.is_empty() {
            eprintln!(
                "    {} error(s) remaining",
                validation_result.remaining_errors.len()
            );
        }

        if !validation_result.new_errors.is_empty() {
            let warning_label = if let Some(t) = theme {
                t.style_warning("Warning")
            } else {
                "Warning".to_string()
            };
            eprintln!(
                "    {}: {} new error(s) introduced",
                warning_label,
                validation_result.new_errors.len()
            );
        }

        // Check if we're done or should continue
        if validation_result.is_fully_fixed() {
            eprintln!("    All errors fixed!");
            break;
        }

        // If no improvement, stop iterating
        if !validation_result.is_improved() {
            let warning_label = if let Some(t) = theme {
                t.style_warning("Warning")
            } else {
                "Warning".to_string()
            };
            eprintln!("    {warning_label}: Fixes did not improve the file");
            final_has_errors = true;
            final_new_error_count = validation_result.new_errors.len();
            break;
        }

        // Prepare for next iteration with remaining errors
        let mut next_diagnostics = validation_result.remaining_errors;
        next_diagnostics.extend(validation_result.new_errors);

        if next_diagnostics.is_empty() {
            break;
        }

        current_diagnostics = next_diagnostics;
    }

    Ok(FileFixResult {
        fixes_applied: file_fixes_applied,
        has_errors: final_has_errors,
        new_error_count: final_new_error_count,
    })
}

/// Apply auto-fixes to files
///
/// This function uses the `FixApplicator` to apply per-rule fixes and the
/// `ErrorValidator` to verify that fixes were successful. It iterates up to
/// `MAX_FIX_ITERATIONS` times to handle cascading fixes.
///
/// When `interactive` is true and stdin is a TTY, prompts the user before
/// applying fixes to each file.
#[allow(clippy::too_many_lines)] // Interactive flow naturally creates longer function
#[instrument(skip(diagnostics, _parsed_files, theme, project_config), fields(diagnostic_count = diagnostics.len()))]
fn apply_fixes(
    diagnostics: &[LintDiagnostic],
    _parsed_files: &HashMap<PathBuf, TaskFile>,
    theme: Option<&CliTheme>,
    project_config: &LashConfig,
    interactive: bool,
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

    let mut total_fixes_applied = 0;
    let mut total_files_fixed = 0;
    let mut files_with_errors = Vec::new();
    let mut apply_all = false; // Track if user selected "all"

    // Process each file with iteration support
    for (file_path, initial_diagnostics) in fixes_by_file {
        let file_str = if let Some(t) = theme {
            t.style_muted(&file_path.display().to_string())
        } else {
            file_path.display().to_string()
        };

        // Show file header
        eprintln!("\n  File: {file_str}");

        // In interactive mode, show diagnostics and prompt for confirmation
        if interactive && !apply_all {
            // Show each diagnostic and its fix preview
            eprintln!("  Fixes to apply:");

            // Read file content for diff display
            let content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read {}", file_path.display()))?;

            let diff_display = if let Some(t) = theme {
                DiffDisplay::with_theme(t.clone())
            } else {
                DiffDisplay::new()
            };

            for diagnostic in &initial_diagnostics {
                eprintln!("\n    - {} ({})", diagnostic.message, diagnostic.code);
                let line_num = diagnostic.location.line.unwrap_or(0);
                let col_num = diagnostic.location.column.unwrap_or(0);
                eprintln!("      Location: line {line_num}, column {col_num}");

                // Show diff if available
                if let Some(diff) = diff_display.format_fix_diff(&content, diagnostic) {
                    // Indent each line of the diff
                    for line in diff.lines() {
                        eprintln!("      {line}");
                    }
                }
            }

            // Prompt the user
            match prompt_user(file_path, initial_diagnostics.len()) {
                Some(InteractiveChoice::Yes) => {
                    eprintln!("  Applying fixes...");
                    // Continue to apply fixes
                }
                Some(InteractiveChoice::No) => {
                    eprintln!("  Skipping file");
                    continue; // Skip this file
                }
                Some(InteractiveChoice::All) => {
                    eprintln!("  Applying fixes to all remaining files...");
                    apply_all = true;
                    // Continue to apply fixes
                }
                Some(InteractiveChoice::Quit) => {
                    eprintln!("\nStopping (user requested quit)");
                    break; // Exit the loop
                }
                None => {
                    // Not a TTY or read failed, fall back to non-interactive
                    eprintln!("  (non-interactive mode - applying fixes)");
                    // Continue to apply fixes
                }
            }
        } else {
            eprintln!("  Processing...");
        }

        // Apply fixes to this file with iteration
        let result = apply_fixes_to_file(file_path, initial_diagnostics, project_config, theme)?;

        if result.fixes_applied > 0 {
            total_fixes_applied += result.fixes_applied;
            total_files_fixed += 1;
        }

        if result.has_errors {
            files_with_errors.push((file_path.clone(), result.new_error_count));
        }
    }

    // Print summary
    eprintln!();
    let summary_msg =
        format!("Applied {total_fixes_applied} fix(es) across {total_files_fixed} file(s)");
    if let Some(t) = theme {
        eprintln!("{}", t.style_success(&summary_msg));
    } else {
        eprintln!("{summary_msg}");
    }

    if !files_with_errors.is_empty() {
        let warning_label = if let Some(t) = theme {
            t.style_warning("Note")
        } else {
            "Note".to_string()
        };
        eprintln!(
            "\n{}: {} file(s) still have errors after fixes",
            warning_label,
            files_with_errors.len()
        );
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
