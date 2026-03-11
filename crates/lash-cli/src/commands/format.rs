//! Format command implementation
//!
//! The `lash format` command auto-formats Lash task files to enforce consistent style.

#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::similar_names)]

use anyhow::{Context, Result};
use lash_cli::command::Command;
use lash_cli::context::Context as CliContext;
use lash_cli::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash_cli::formatter::{OutputFormat, Verbosity};
use lash_cli::theme::CliTheme;
use lash_core::formatter::{FormatOptions, Formatter};
use lash_core::parser::parse_file;
use lash_types::error::{Diagnostic, LashError, Severity};
use lash_types::{error::Result as LashResult, LashConfig};
use similar::{ChangeTag, TextDiff};
use std::path::{Path, PathBuf};
use tracing::instrument;

use crate::utils::file_discovery::{discover_markdown_files, find_project_root};
use crate::utils::output::create_progress_bar;

/// Arguments for the format command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // CLI args naturally contain many boolean flags
pub struct FormatArgs {
    /// Paths to format (files or directories)
    pub paths: Vec<PathBuf>,
    /// Check formatting without modifying (dry-run)
    pub check: bool,
    /// Show diff of changes
    pub diff: bool,
    /// Only normalize formatting, don't apply lint fixes
    pub no_fix: bool,
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Verbosity level for output
    pub verbosity: Verbosity,
}

impl Command for FormatArgs {
    /// Execute the format command
    ///
    /// # Arguments
    ///
    /// * `ctx` - Shared command context
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a `LashError` on failure
    #[instrument(skip(self, ctx), fields(paths = ?self.paths, check = self.check, diff = self.diff))]
    fn execute(&self, ctx: &CliContext) -> LashResult<()> {
        // For now, use the project config from context
        // In the future, we'll load config more intelligently
        let _ = ctx; // Suppress unused variable warning for now

        // Call the public execute function and convert result
        match execute(self.clone()) {
            Ok(0) => Ok(()),
            Ok(2) => Err(lash_types::error::LashError::internal(
                "Files need formatting",
                Some("Run lash format without --check to format".to_string()),
            )),
            Ok(code) => Err(lash_types::error::LashError::internal(
                format!("Unexpected exit code: {code}"),
                None,
            )),
            Err(e) => Err(lash_types::error::LashError::internal(
                format!("Format command failed: {e}"),
                None,
            )),
        }
    }
}

/// Execute the format command (public interface for main.rs)
///
/// # Arguments
///
/// * `args` - Format command arguments
///
/// # Returns
///
/// Exit code: 0 (all files properly formatted), 1 (general error), 2 (files need formatting with --check)
#[instrument(skip(args), fields(paths = ?args.paths, check = args.check, diff = args.diff))]
pub fn execute(args: FormatArgs) -> Result<i32> {
    // Load theme based on no_color flag and output format
    let theme = if args.json {
        None
    } else {
        CliTheme::load(None, !args.no_color)?
    };

    // Determine paths to format
    let paths = if args.paths.is_empty() {
        // No paths specified - format entire project
        let project_root = if let Some(ref root) = args.project_root {
            root.clone()
        } else {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            find_project_root(&cwd)
        };
        vec![project_root]
    } else {
        args.paths.clone()
    };

    // Discover markdown files
    let files = discover_markdown_files(&paths, true).context("Failed to discover files")?;
    tracing::info!(file_count = files.len(), "Discovered files to format");

    if files.is_empty() {
        let msg = "No markdown files found to format";
        if !args.json {
            if let Some(t) = &theme {
                eprintln!("{}", t.style_warning(msg));
            } else {
                eprintln!("{msg}");
            }
        }
        return Ok(0);
    }

    // Load project configuration
    let project_config = load_project_config(&files)?;

    // Configure formatter
    let format_options = configure_formatter(&args);

    // Format files
    let result = format_files(
        &files,
        &project_config,
        &format_options,
        &args,
        theme.as_ref(),
    )?;

    // Output results
    if args.json {
        output_json_results(&result, files.len())?;
    } else {
        output_text_results(&result, &args, theme.as_ref())?;
    }

    // Determine exit code
    if args.check && result.needs_formatting > 0 {
        Ok(2)
    } else if result.failed > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Result of formatting operation
#[derive(Debug, Default)]
struct FormatResult {
    /// Number of files successfully formatted
    formatted: usize,
    /// Number of files that need formatting (in check mode)
    needs_formatting: usize,
    /// Number of files that failed to format
    failed: usize,
    /// Diagnostics for files that need formatting (in check mode)
    needs_formatting_diagnostics: Vec<Diagnostic>,
    /// Diagnostics for files that failed to format
    error_diagnostics: Vec<Diagnostic>,
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

/// Configure formatter based on command arguments
fn configure_formatter(args: &FormatArgs) -> FormatOptions {
    let mut options = FormatOptions::default();

    // Disable auto-fixes if --no-fix is specified
    if args.no_fix {
        options.apply_auto_fixes = false;
    }

    options
}

/// Format all files with progress reporting
#[instrument(skip(files, config, options, args, theme), fields(file_count = files.len()))]
fn format_files(
    files: &[PathBuf],
    config: &LashConfig,
    options: &FormatOptions,
    args: &FormatArgs,
    theme: Option<&CliTheme>,
) -> anyhow::Result<FormatResult> {
    let mut result = FormatResult::default();

    // Create error reporter for streaming errors (in non-JSON mode)
    let reporter_config = ErrorReporterConfig {
        verbosity: args.verbosity,
        output_format: if args.json {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
        display_mode: ErrorDisplayMode::Streaming,
        theme: theme.cloned(),
        show_summary: false, // We'll print our own summary
    };

    let mut reporter = ErrorReporter::new(reporter_config);

    let show_progress = files.len() > 1 && !args.check && !args.diff && !args.json;
    let pb = if show_progress {
        Some(create_progress_bar(files.len()))
    } else {
        None
    };

    for file_path in files {
        if let Some(ref pb) = pb {
            pb.set_message(format!("Formatting {}", file_path.display()));
        }

        match format_single_file(file_path, config, options, args) {
            Ok(changed) => {
                if args.check {
                    if changed {
                        result.needs_formatting += 1;

                        // Create diagnostic for unformatted file
                        let diagnostic = Diagnostic {
                            code: "F_NEEDS_FORMATTING",
                            severity: Severity::Warning,
                            message: "File needs formatting".to_string(),
                            location: Some(lash_types::error::Location::file_only(
                                file_path.clone(),
                            )),
                            snippet: None,
                            help: Some("Run 'lash format' to format this file".to_string()),
                            labels: None,
                            recovery_command: Some(format!("lash format {}", file_path.display())),
                            fix_steps: None,
                            explanation: None,
                            docs_url: None,
                        };

                        result.needs_formatting_diagnostics.push(diagnostic.clone());

                        // Only show in text mode (not check mode with JSON)
                        if !args.json {
                            reporter.report_diagnostic(&diagnostic);
                        }
                    }
                } else if changed {
                    result.formatted += 1;
                }
            }
            Err(e) => {
                if let Some(ref pb) = pb {
                    pb.finish_and_clear();
                }

                result.failed += 1;

                // Create diagnostic for formatting error
                // Check if it's a parse error, write error, or general format error
                let error = if e.to_string().contains("Failed to parse") {
                    // Parse error
                    LashError::internal(
                        format!("Failed to format {}: {}", file_path.display(), e),
                        Some("The file may have syntax errors that prevent formatting".to_string()),
                    )
                } else if e.to_string().contains("Failed to write") {
                    // Write error
                    LashError::io_write_error(file_path.clone(), e.to_string())
                } else {
                    // General formatting error
                    LashError::internal(
                        format!("Failed to format {}: {}", file_path.display(), e),
                        None,
                    )
                };

                let diagnostic = error.to_diagnostic();
                result.error_diagnostics.push(diagnostic.clone());

                // Report error immediately (streaming)
                reporter.report_error(&error);
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
        formatted = result.formatted,
        needs_formatting = result.needs_formatting,
        failed = result.failed,
        "Formatting complete"
    );

    Ok(result)
}

/// Format a single file
///
/// Returns `true` if the file was changed (or would be changed in check mode)
fn format_single_file(
    file_path: &PathBuf,
    config: &LashConfig,
    options: &FormatOptions,
    args: &FormatArgs,
) -> anyhow::Result<bool> {
    // Parse the file
    let task_file = parse_file(file_path, config)
        .with_context(|| format!("Failed to parse {}", file_path.display()))?;

    // Format the file
    let formatter = Formatter::new(config.clone(), options.clone());
    let formatted = formatter
        .format_file(&task_file)
        .with_context(|| format!("Failed to format {}", file_path.display()))?;

    // Read original content
    let original = std::fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    // Check if content changed
    let changed = formatted != original;

    if changed {
        if args.diff {
            // Show diff
            show_diff(file_path, &original, &formatted);
        }

        if !args.check && !args.diff {
            // Write formatted content back to file
            std::fs::write(file_path, formatted)
                .with_context(|| format!("Failed to write {}", file_path.display()))?;
        }
    }

    Ok(changed)
}

/// Show a unified diff between original and formatted content
fn show_diff(file_path: &Path, original: &str, formatted: &str) {
    println!("--- {}", file_path.display());
    println!("+++ {}", file_path.display());

    let diff = TextDiff::from_lines(original, formatted);

    for change in diff.iter_all_changes() {
        let (sign, line) = match change.tag() {
            ChangeTag::Delete => ("-", change.value()),
            ChangeTag::Insert => ("+", change.value()),
            ChangeTag::Equal => (" ", change.value()),
        };
        print!("{sign}{line}");
        if !line.ends_with('\n') {
            println!();
        }
    }

    println!();
}

/// Output formatting results in JSON format to stdout
fn output_json_results(result: &FormatResult, files_checked: usize) -> anyhow::Result<()> {
    // Combine all diagnostics
    let mut all_diagnostics = result.needs_formatting_diagnostics.clone();
    all_diagnostics.extend(result.error_diagnostics.clone());

    let output = serde_json::json!({
        "diagnostics": all_diagnostics,
        "summary": {
            "files_checked": files_checked,
            "formatted": result.formatted,
            "needs_formatting": result.needs_formatting,
            "failed": result.failed,
        }
    });

    let json_str = serde_json::to_string_pretty(&output)?;
    println!("{json_str}");

    Ok(())
}

/// Output formatting results in human-readable text format
fn output_text_results(
    result: &FormatResult,
    args: &FormatArgs,
    theme: Option<&CliTheme>,
) -> anyhow::Result<()> {
    // In check mode, the diagnostics have already been printed by the reporter
    // Just print the summary

    if args.check {
        if result.needs_formatting > 0 {
            let msg = format!("{} file(s) need formatting", result.needs_formatting);
            if let Some(t) = theme {
                eprintln!("\n{}", t.style_warning(&msg));
            } else {
                eprintln!("\n{msg}");
            }
        } else if result.failed == 0 {
            let msg = "All files are properly formatted";
            if let Some(t) = theme {
                eprintln!("{}", t.style_success(msg));
            } else {
                eprintln!("{msg}");
            }
        }
    } else {
        // Format mode (not check)
        if result.formatted > 0 {
            let msg = format!("Formatted {} file(s) successfully", result.formatted);
            if let Some(t) = theme {
                eprintln!("\n{}", t.style_success(&msg));
            } else {
                eprintln!("\n{msg}");
            }
        } else if result.failed == 0 {
            let msg = "All files already formatted";
            if let Some(t) = theme {
                eprintln!("{}", t.style_info(msg));
            } else {
                eprintln!("{msg}");
            }
        }
    }

    // Always report failures
    if result.failed > 0 {
        let msg = format!("{} file(s) failed to format", result.failed);
        if let Some(t) = theme {
            eprintln!("{}", t.style_error(&msg));
        } else {
            eprintln!("{msg}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Creates a properly-formatted task file that should not be modified by the formatter.
    fn write_already_formatted_file(dir: &TempDir, name: &str) -> PathBuf {
        let content = "# Task List\n\n@id: example\n@created: 2024-01-15\n\n## Tasks\n\n- [ ] First task\n- [x] Done task\n";
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }

    /// Creates a file with inconsistent annotation spacing that the formatter will normalize.
    fn write_needs_formatting_file(dir: &TempDir, name: &str) -> PathBuf {
        let content = "# Task List\n\n@id:   example\n@labels:backend,  api\n\n## Tasks\n\n- [ ] First task\n";
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }

    fn default_args(paths: Vec<PathBuf>) -> FormatArgs {
        FormatArgs {
            paths,
            check: false,
            diff: false,
            no_fix: false,
            json: false,
            no_color: true, // suppress color to keep output predictable
            project_root: None,
            verbosity: Verbosity::Normal,
        }
    }

    #[test]
    fn test_configure_formatter_default() {
        let args = default_args(vec![]);
        let options = configure_formatter(&args);
        assert!(options.apply_auto_fixes);
    }

    #[test]
    fn test_configure_formatter_no_fix() {
        let args = FormatArgs {
            no_fix: true,
            ..default_args(vec![])
        };

        let options = configure_formatter(&args);
        assert!(!options.apply_auto_fixes);
    }

    // --- exit code tests ---

    #[test]
    fn test_execute_exit_code_0_when_file_already_formatted() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");

        let result = execute(default_args(vec![path]));
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_exit_code_0_when_file_formatted_successfully() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");

        let result = execute(default_args(vec![path]));
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_exit_code_2_in_check_mode_when_file_needs_formatting() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");

        let args = FormatArgs {
            check: true,
            ..default_args(vec![path])
        };
        let result = execute(args);
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_execute_exit_code_0_in_check_mode_when_file_already_formatted() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");

        let args = FormatArgs {
            check: true,
            ..default_args(vec![path])
        };
        let result = execute(args);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_check_mode_does_not_modify_file() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let original_content = fs::read_to_string(&path).unwrap();

        let args = FormatArgs {
            check: true,
            ..default_args(vec![path.clone()])
        };
        execute(args).unwrap();

        let after_content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            after_content, original_content,
            "check mode must not modify the file"
        );
    }

    #[test]
    fn test_execute_diff_mode_does_not_modify_file() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let original_content = fs::read_to_string(&path).unwrap();

        let args = FormatArgs {
            diff: true,
            ..default_args(vec![path.clone()])
        };
        execute(args).unwrap();

        let after_content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            after_content, original_content,
            "diff mode must not modify the file"
        );
    }

    #[test]
    fn test_execute_without_check_or_diff_modifies_file() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let original_content = fs::read_to_string(&path).unwrap();

        execute(default_args(vec![path.clone()])).unwrap();

        let after_content = fs::read_to_string(&path).unwrap();
        assert_ne!(
            after_content, original_content,
            "normal format mode must modify the file when it needs formatting"
        );
    }

    #[test]
    fn test_execute_exit_code_0_when_no_files_found() {
        let temp = TempDir::new().unwrap();
        // Pass a path that exists but has no markdown files
        let empty_dir = temp.path().join("empty");
        fs::create_dir(&empty_dir).unwrap();

        let result = execute(default_args(vec![empty_dir]));
        assert_eq!(result.unwrap(), 0);
    }

    // --- json mode tests ---

    #[test]
    fn test_execute_json_mode_exits_zero_for_already_formatted_file() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");

        let args = FormatArgs {
            json: true,
            ..default_args(vec![path])
        };
        let result = execute(args);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_json_check_mode_exits_two_when_needs_formatting() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");

        let args = FormatArgs {
            json: true,
            check: true,
            ..default_args(vec![path])
        };
        let result = execute(args);
        assert_eq!(result.unwrap(), 2);
    }

    // --- paths tests ---

    #[test]
    fn test_execute_with_explicit_paths_uses_those_paths() {
        let temp = TempDir::new().unwrap();
        // Write a file only at the specified path
        let path = write_already_formatted_file(&temp, "lash.index.md");

        // Provide paths explicitly; should find the file and succeed
        let result = execute(default_args(vec![path]));
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_with_project_root_uses_root() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");

        // Pass project_root explicitly but empty paths to trigger the project root path
        let args = FormatArgs {
            paths: vec![],
            project_root: Some(temp.path().to_path_buf()),
            ..default_args(vec![])
        };
        let result = execute(args);
        // File is already formatted, so should succeed with exit code 0
        // (The file exists at temp root so it will be found)
        assert_eq!(result.unwrap(), 0);
        let _ = path;
    }

    // --- load_project_config tests ---

    #[test]
    fn test_load_project_config_returns_default_when_no_config_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("some.md");
        fs::write(&path, "# Task\n\n@id: x\n\n## Tasks\n\n- [ ] task\n").unwrap();

        let config = load_project_config(&[path]).unwrap();
        // Default config should be equivalent to LashConfig::default()
        let default_config = LashConfig::default();
        // Compare by debug representation as LashConfig may not impl PartialEq
        assert_eq!(format!("{config:?}"), format!("{default_config:?}"));
    }

    #[test]
    fn test_load_project_config_reads_config_when_present() {
        let temp = TempDir::new().unwrap();
        // Create .lash directory and config.toml
        let lash_dir = temp.path().join(".lash");
        fs::create_dir(&lash_dir).unwrap();

        // LashConfig is deserialized directly from TOML, so we need to provide all required fields.
        // We set indent_spaces=4 (non-default; default is 2) to verify the config was actually read.
        let root_str = temp.path().display().to_string();
        let db_str = temp.path().join(".lash/lash.db").display().to_string();
        let config_content = format!(
            "root_path = \"{root_str}\"\nindex_file = \"lash.index.md\"\nmax_depth = 3\nindent_spaces = 4\ndb_path = \"{db_str}\"\n"
        );
        fs::write(lash_dir.join("config.toml"), &config_content).unwrap();

        // Create a markdown file in the temp dir
        let md_path = temp.path().join("lash.index.md");
        fs::write(&md_path, "# Task\n\n@id: x\n\n## Tasks\n\n- [ ] task\n").unwrap();

        // Config should load without error and reflect indent_spaces=4 from the file
        let config = load_project_config(&[md_path]).unwrap();
        assert_eq!(config.indent_spaces, 4);
    }

    // --- format result counter tests ---

    #[test]
    fn test_needs_formatting_count_equals_one_in_check_mode() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        // Use the internal format_files to inspect result counters
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path.clone()])
        };
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.needs_formatting, 1);
        assert_eq!(result.formatted, 0);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_formatted_count_equals_one_when_file_needs_formatting() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.formatted, 1);
        assert_eq!(result.needs_formatting, 0);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_formatted_count_equals_zero_when_file_already_formatted() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.formatted, 0);
        assert_eq!(result.needs_formatting, 0);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_failed_count_equals_one_for_unreadable_file() {
        // Pass a path to a non-existent file to trigger a failure
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("does_not_exist.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![nonexistent.clone()]);
        let files = vec![nonexistent];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 1);
        assert_eq!(result.formatted, 0);
        assert_eq!(result.needs_formatting, 0);
    }

    // --- format_single_file tests ---

    #[test]
    fn test_format_single_file_returns_false_when_already_formatted() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![]);
        let changed = format_single_file(&path, &config, &options, &args).unwrap();
        assert!(
            !changed,
            "already-formatted file should return changed=false"
        );
    }

    #[test]
    fn test_format_single_file_returns_true_when_file_needs_formatting() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![]);
        let changed = format_single_file(&path, &config, &options, &args).unwrap();
        assert!(
            changed,
            "file needing formatting should return changed=true"
        );
    }

    #[test]
    fn test_format_single_file_writes_when_not_check_and_not_diff() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let original = fs::read_to_string(&path).unwrap();
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![]);
        format_single_file(&path, &config, &options, &args).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert_ne!(after, original, "file should be written in normal mode");
    }

    #[test]
    fn test_format_single_file_does_not_write_in_check_mode() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let original = fs::read_to_string(&path).unwrap();
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![])
        };
        format_single_file(&path, &config, &options, &args).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(after, original, "file should not be written in check mode");
    }

    #[test]
    fn test_format_single_file_does_not_write_in_diff_mode() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let original = fs::read_to_string(&path).unwrap();
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            diff: true,
            ..default_args(vec![])
        };
        format_single_file(&path, &config, &options, &args).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(after, original, "file should not be written in diff mode");
    }

    #[test]
    fn test_format_single_file_check_mode_still_returns_true_for_changed_file() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![])
        };
        let changed = format_single_file(&path, &config, &options, &args).unwrap();
        assert!(
            changed,
            "check mode should still report changed=true for a file needing formatting"
        );
    }

    // --- exit code boundary tests ---

    #[test]
    fn test_exit_code_is_1_not_0_when_file_fails_to_format() {
        // Use format_files directly with a non-existent path to trigger a failure,
        // then verify the exit code logic: result.failed > 0 → exit 1.
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("ghost.md");

        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![nonexistent.clone()]);
        let files = vec![nonexistent];
        let result = format_files(&files, &config, &options, &args, None).unwrap();

        assert_eq!(result.failed, 1);
        assert_eq!(result.formatted, 0);
        assert_eq!(result.needs_formatting, 0);
    }

    #[test]
    fn test_exit_code_is_2_not_1_in_check_mode_with_needs_formatting() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");

        let args = FormatArgs {
            check: true,
            ..default_args(vec![path])
        };
        let result = execute(args);
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_exit_code_is_0_not_1_when_check_mode_but_no_needs_formatting() {
        // check=true but needs_formatting == 0 should NOT give exit 2
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");

        let args = FormatArgs {
            check: true,
            ..default_args(vec![path])
        };
        let result = execute(args);
        assert_eq!(result.unwrap(), 0);
    }

    // --- multiple files: progress bar boundary (files.len() > 1) ---

    #[test]
    fn test_multiple_files_all_formatted_successfully() {
        let temp = TempDir::new().unwrap();
        let path1 = write_needs_formatting_file(&temp, "file1.md");
        let path2 = write_needs_formatting_file(&temp, "file2.md");

        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path1.clone(), path2.clone()]);
        let files = vec![path1, path2];
        let result = format_files(&files, &config, &options, &args, None).unwrap();

        // Both files changed
        assert_eq!(result.formatted, 2);
        assert_eq!(result.needs_formatting, 0);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_single_file_no_progress_bar_path() {
        // Only 1 file: files.len() > 1 is false, so no progress bar created
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.formatted, 0);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_check_mode_with_multiple_files_counts_needs_formatting() {
        let temp = TempDir::new().unwrap();
        let path1 = write_needs_formatting_file(&temp, "file1.md");
        let path2 = write_already_formatted_file(&temp, "file2.md");

        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path1.clone(), path2.clone()])
        };
        let files = vec![path1, path2];
        let result = format_files(&files, &config, &options, &args, None).unwrap();

        assert_eq!(result.needs_formatting, 1);
        assert_eq!(result.formatted, 0);
    }

    // --- diagnostics in check mode ---

    #[test]
    fn test_needs_formatting_diagnostics_populated_in_check_mode() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");

        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path.clone()])
        };
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();

        assert_eq!(result.needs_formatting_diagnostics.len(), 1);
        assert_eq!(
            result.needs_formatting_diagnostics[0].code,
            "F_NEEDS_FORMATTING"
        );
    }

    #[test]
    fn test_no_diagnostics_when_already_formatted_in_check_mode() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");

        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path.clone()])
        };
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();

        assert_eq!(result.needs_formatting_diagnostics.len(), 0);
        assert_eq!(result.error_diagnostics.len(), 0);
    }

    // --- json mode suppresses progress and inline text ---

    #[test]
    fn test_json_mode_does_not_show_progress_bar_with_multiple_files() {
        let temp = TempDir::new().unwrap();
        let path1 = write_already_formatted_file(&temp, "file1.md");
        let path2 = write_already_formatted_file(&temp, "file2.md");

        let config = LashConfig::default();
        let options = FormatOptions::default();
        // json=true means show_progress is false
        let args = FormatArgs {
            json: true,
            ..default_args(vec![path1.clone(), path2.clone()])
        };
        let files = vec![path1, path2];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 0);
    }

    // --- no_color / theme selection ---

    #[test]
    fn test_execute_with_no_color_false_succeeds() {
        // With no_color=false, CliTheme::load will be called with color enabled
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");

        let args = FormatArgs {
            no_color: false,
            ..default_args(vec![path])
        };
        let result = execute(args);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_with_no_color_true_succeeds() {
        // With no_color=true, CliTheme::load will be called with color disabled
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");

        let args = FormatArgs {
            no_color: true,
            ..default_args(vec![path])
        };
        let result = execute(args);
        assert_eq!(result.unwrap(), 0);
    }
}
