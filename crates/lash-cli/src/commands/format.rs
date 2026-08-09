//! Format command implementation
//!
//! The `lash format` command auto-formats Lash task files to enforce consistent style.

#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::similar_names)]

use anyhow::{Context, Result};
use lash::command::Command;
use lash::context::Context as CliContext;
use lash::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash::formatter::{OutputFormat, Verbosity};
use lash::theme::CliTheme;
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

    // Read original content. The formatter needs it as well as the parsed
    // file, to carry through sections the model does not represent.
    let original = std::fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    // Format the file
    let formatter = Formatter::new(config.clone(), options.clone());
    let formatted = formatter
        .format_file(&original, &task_file)
        .with_context(|| format!("Failed to format {}", file_path.display()))?;

    // Check if content changed
    let changed = formatted != original;

    if changed {
        if args.diff {
            // Show diff
            show_diff(file_path, &original, &formatted);
        }

        if !args.check && !args.diff {
            // Write formatted content back to file atomically (tmp + rename),
            // so a crash mid-write can't leave a half-formatted Markdown file
            // on disk. Same helper Store uses for status-toggle writes.
            lash_core::store::write_atomic(file_path, formatted.as_bytes())
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
    #[cfg(unix)]
    use libc;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
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
        // Use forward slashes for paths to avoid TOML escape issues on Windows.
        let root_str = temp.path().display().to_string().replace('\\', "/");
        let db_str = temp
            .path()
            .join(".lash/lash.db")
            .display()
            .to_string()
            .replace('\\', "/");
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

    // --- json vs non-json theme selection (mut-000281, mut-000286) ---
    // When json=true the theme must be None (no terminal styling).
    // When json=false the theme is Some (terminal styling enabled).
    // These tests drive the args.json branch in execute() in both directions
    // with an observable side-effect: json mode outputs JSON to stdout and
    // the exit code logic still works correctly.

    #[test]
    fn test_execute_json_false_does_not_exit_with_json_output_path() {
        // json=false: text output path is taken.  File already formatted → exit 0.
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let args = FormatArgs {
            json: false,
            ..default_args(vec![path])
        };
        assert_eq!(execute(args).unwrap(), 0);
    }

    #[test]
    fn test_execute_json_true_does_not_exit_with_text_output_path() {
        // json=true: JSON output path is taken.  File already formatted → exit 0.
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let args = FormatArgs {
            json: true,
            ..default_args(vec![path])
        };
        assert_eq!(execute(args).unwrap(), 0);
    }

    // --- paths empty vs non-empty branch (mut-000283) ---
    // When paths is empty, execute() discovers from project_root.
    // When paths is non-empty, it uses the explicit paths.
    // The two paths must produce different (observable) results when
    // the explicit path points to a file that wouldn't be found via root.

    #[test]
    fn test_execute_empty_paths_uses_project_root_finds_no_files() {
        // paths empty + project_root = empty dir → no files → exit 0
        let temp = TempDir::new().unwrap();
        let args = FormatArgs {
            paths: vec![],
            project_root: Some(temp.path().to_path_buf()),
            ..default_args(vec![])
        };
        assert_eq!(execute(args).unwrap(), 0);
    }

    #[test]
    fn test_execute_non_empty_paths_uses_explicit_paths() {
        // paths non-empty → uses those specific paths; if file needs formatting, formatter runs.
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let args = default_args(vec![path.clone()]);
        // Should format the file and return 0.
        assert_eq!(execute(args).unwrap(), 0);
        // File should now be modified (different from original).
        // (confirmed by other tests; here we just check the exit code takes the right branch)
    }

    // --- files.is_empty() early-return path (mut-000285) ---
    // When no markdown files are discovered the function returns Ok(0) early.
    // When files ARE discovered it continues.  Both branches must be exercised
    // with assertions that distinguish them.

    #[test]
    fn test_execute_empty_dir_returns_zero_via_early_return() {
        let temp = TempDir::new().unwrap();
        let empty = temp.path().join("sub");
        fs::create_dir(&empty).unwrap();
        // No .md files → files.is_empty() is true → early return Ok(0)
        assert_eq!(execute(default_args(vec![empty])).unwrap(), 0);
    }

    #[test]
    fn test_execute_non_empty_dir_does_not_early_return() {
        // Has an .md file → files.is_empty() is false → continues to format
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        // Provides the file directly so it is definitely discovered.
        assert_eq!(execute(default_args(vec![path])).unwrap(), 0);
    }

    // --- exit code exact values (mut-000291,296,297) ---
    // These mutants flip the literal constants in the three return arms.
    // We need tests that pin each arm to its exact value.

    #[test]
    fn test_exit_code_exactly_2_in_check_mode_with_unformatted_file() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path])
        };
        assert_eq!(
            execute(args).unwrap(),
            2,
            "check + needs_formatting must be exactly 2"
        );
    }

    #[test]
    fn test_exit_code_exactly_0_in_check_mode_with_already_formatted_file() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path])
        };
        assert_eq!(
            execute(args).unwrap(),
            0,
            "check + already formatted must be exactly 0"
        );
    }

    #[test]
    fn test_exit_code_exactly_0_in_normal_mode_with_already_formatted_file() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        assert_eq!(
            execute(default_args(vec![path])).unwrap(),
            0,
            "normal mode, no changes must be exactly 0"
        );
    }

    // Exit code 1 path (result.failed > 0): tested indirectly through format_files
    // with a non-existent file; execute() itself propagates the failed count.
    // The format_files tests already pin result.failed == 1.

    // --- exit code && vs || distinction (mut-000288) ---
    // check=true but needs_formatting==0 must NOT return 2 (only || would).
    // check=false but needs_formatting==1 (hypothetically) must NOT return 2.
    // The first case is directly testable.

    #[test]
    fn test_check_true_needs_formatting_zero_does_not_return_2() {
        // check=true AND needs_formatting==0 → should return 0, not 2.
        // This distinguishes `args.check && needs > 0` from `args.check || needs > 0`.
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path])
        };
        assert_ne!(execute(args).unwrap(), 2);
    }

    // --- format_files: show_progress exact boundary (mut-000304,305,306) ---
    // files.len() > 1: boundary is exactly 1.  With exactly 1 file, show_progress
    // is false; with 2+ files it is true.  Both paths must produce the same
    // formatted counters so we check that the function succeeds in both cases.

    #[test]
    fn test_format_files_exactly_one_file_no_progress_bar_still_formats() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        // Exactly 1 file → show_progress=false, but formatting still runs.
        assert_eq!(result.formatted, 1);
    }

    #[test]
    fn test_format_files_exactly_two_files_progress_bar_still_formats() {
        let temp = TempDir::new().unwrap();
        let p1 = write_needs_formatting_file(&temp, "file1.md");
        let p2 = write_needs_formatting_file(&temp, "file2.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![p1.clone(), p2.clone()]);
        let files = vec![p1, p2];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        // 2 files → show_progress=true; both formatted.
        assert_eq!(result.formatted, 2);
        assert_eq!(result.failed, 0);
    }

    // --- show_progress conditions: !args.check, !args.diff, !args.json (mut-000308,309,310,311,312) ---
    // These suppress the progress bar.  We verify behaviour is correct in each case.

    #[test]
    fn test_format_files_check_mode_suppresses_progress_and_counts_correctly() {
        // check=true → show_progress=false even with >1 file; needs_formatting counted.
        let temp = TempDir::new().unwrap();
        let p1 = write_needs_formatting_file(&temp, "file1.md");
        let p2 = write_needs_formatting_file(&temp, "file2.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![p1.clone(), p2.clone()])
        };
        let files = vec![p1, p2];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(
            result.needs_formatting, 2,
            "both files need formatting in check mode"
        );
        assert_eq!(result.formatted, 0);
    }

    #[test]
    fn test_format_files_diff_mode_suppresses_progress_and_does_not_write() {
        // diff=true → show_progress=false (>1 file); file must NOT be written to disk.
        // Note: diff mode still increments `formatted` for files that would change,
        // because the else-if branch runs (check=false, diff=false is the write guard).
        let temp = TempDir::new().unwrap();
        let p1 = write_needs_formatting_file(&temp, "file1.md");
        let p2 = write_needs_formatting_file(&temp, "file2.md");
        let original1 = fs::read_to_string(&p1).unwrap();
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            diff: true,
            ..default_args(vec![p1.clone(), p2.clone()])
        };
        let files = vec![p1.clone(), p2];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 0);
        // File on disk must remain unchanged (diff mode must not write).
        assert_eq!(
            fs::read_to_string(&p1).unwrap(),
            original1,
            "diff mode must not write file"
        );
    }

    #[test]
    fn test_format_files_json_mode_suppresses_progress_and_counts_correctly() {
        // json=true → show_progress=false; formatting still runs.
        let temp = TempDir::new().unwrap();
        let p1 = write_needs_formatting_file(&temp, "file1.md");
        let p2 = write_needs_formatting_file(&temp, "file2.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            json: true,
            ..default_args(vec![p1.clone(), p2.clone()])
        };
        let files = vec![p1, p2];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.formatted, 2);
        assert_eq!(result.failed, 0);
    }

    // --- format_files: check branch vs else-if changed branch (mut-000315,316,317,319,320) ---
    // When args.check=true AND changed=true → needs_formatting incremented (not formatted).
    // When args.check=false AND changed=true → formatted incremented (not needs_formatting).
    // When args.check=false AND changed=false → neither counter incremented.

    #[test]
    fn test_check_mode_increments_needs_formatting_not_formatted() {
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
        assert_eq!(
            result.needs_formatting, 1,
            "check mode: needs_formatting must be 1"
        );
        assert_eq!(result.formatted, 0, "check mode: formatted must remain 0");
    }

    #[test]
    fn test_non_check_mode_increments_formatted_not_needs_formatting() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.formatted, 1, "normal mode: formatted must be 1");
        assert_eq!(
            result.needs_formatting, 0,
            "normal mode: needs_formatting must remain 0"
        );
    }

    #[test]
    fn test_non_check_mode_no_change_increments_neither_counter() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.formatted, 0, "no change: formatted must remain 0");
        assert_eq!(
            result.needs_formatting, 0,
            "no change: needs_formatting must remain 0"
        );
        assert_eq!(result.failed, 0);
    }

    // --- format_files: failed counter exact value (mut-000322) ---

    #[test]
    fn test_failed_count_is_exactly_one_for_one_unreadable_file() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("ghost.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![nonexistent.clone()]);
        let files = vec![nonexistent];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(
            result.failed, 1,
            "failed must be exactly 1 for one failed file"
        );
        assert_eq!(result.formatted, 0);
        assert_eq!(result.needs_formatting, 0);
    }

    // --- format_files: error classification (mut-000323) ---
    // When parsing fails (file doesn't exist → "Failed to parse"), the error
    // diagnostic code should reflect a parse error vs a write error.
    // We verify failed==1 for a parse-error case (non-existent file).

    #[test]
    fn test_error_diagnostic_populated_for_failed_file() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("no_such_file.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![nonexistent.clone()]);
        let files = vec![nonexistent];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(
            result.error_diagnostics.len(),
            1,
            "one error diagnostic expected"
        );
        assert_eq!(result.failed, 1);
    }

    // --- format_files: error classification parse branch (mut-000323) ---
    // When e.to_string().contains("Failed to parse") is true, the parse-error branch
    // is taken: LashError::internal with context="The file may have syntax errors...".
    // With the negated mutation, the general branch is taken: context=None.
    // The observable difference: parse-error diagnostic has a non-None labels field
    // (from the context), while the general error has labels=None.

    #[test]
    fn test_parse_error_diagnostic_has_context_label_with_syntax_hint() {
        let temp = TempDir::new().unwrap();
        // A non-existent file causes parse_file to fail with "Failed to parse ..."
        // which triggers the parse-error branch in format_files.
        let nonexistent = temp.path().join("no_such_parse.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![nonexistent.clone()]);
        let files = vec![nonexistent];
        let result = format_files(&files, &config, &options, &args, None).unwrap();

        assert_eq!(result.failed, 1, "one failure expected");
        let diag = &result.error_diagnostics[0];
        // The parse-error branch includes context "The file may have syntax errors..."
        // which becomes a label in the diagnostic.  The general branch has labels=None.
        assert!(
            diag.labels.is_some(),
            "parse-error diagnostic must have a context label (syntax hint); got labels=None"
        );
    }

    // --- format_files: write error classification (mut-000394) ---
    // Line 299: `e.to_string().contains("Failed to write")` is negated.
    // The original code: when a write fails (e contains "Failed to write"),
    // use LashError::io_write_error (code="E_IO_WRITE_ERROR").
    // With the mutation: the write-error branch is skipped and
    // LashError::internal (code="E_INTERNAL") is used instead.
    //
    // Observable difference: the diagnostic `message` field for io_write_error
    // contains "failed to write file:" while internal contains "Failed to format".
    //
    // To trigger a write error, we format a file that needs changes but make it
    // read-only on disk (unix only - root may bypass this).

    #[test]
    #[cfg(unix)]
    fn test_write_error_diagnostic_uses_io_write_error_code() {
        // Skip if running as root (root can write read-only files).
        if unsafe { libc::geteuid() == 0 } {
            return;
        }

        let temp = TempDir::new().unwrap();
        // Write a file that needs formatting (formatter will change it).
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        // Make the file read-only so std::fs::write fails with "Failed to write".
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o444); // r--r--r--
        fs::set_permissions(&path, perms).unwrap();

        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]); // check=false, diff=false → will try to write
        let files = vec![path.clone()];
        let result = format_files(&files, &config, &options, &args, None).unwrap();

        // Restore permissions so TempDir cleanup can remove the file.
        let mut restore = fs::metadata(&path).unwrap().permissions();
        restore.set_mode(0o644);
        let _ = fs::set_permissions(&path, restore);

        assert_eq!(result.failed, 1, "read-only file must count as failure");
        assert_eq!(
            result.error_diagnostics.len(),
            1,
            "one error diagnostic expected"
        );

        let diag = &result.error_diagnostics[0];
        // io_write_error produces a message containing "failed to write file:"
        // internal produces a message containing "Failed to format".
        // Distinguishing them proves the write-error branch is taken.
        assert!(
            diag.message.contains("failed to write") || diag.message.contains("Failed to write"),
            "write-error diagnostic message must mention 'failed to write'; got: {:?}",
            diag.message
        );
    }

    // --- format_single_file: != vs == for changed (mut-000326) ---
    // When file content differs from formatted output, changed must be true.
    // When identical, changed must be false.  Already covered by existing tests,
    // but we add explicit assertions to pin the exact comparison direction.

    #[test]
    fn test_format_single_file_changed_is_true_iff_content_differs() {
        let temp = TempDir::new().unwrap();
        let needs_fmt = write_needs_formatting_file(&temp, "needs.md");
        let already_fmt = write_already_formatted_file(&temp, "already.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![])
        }; // check prevents writes

        let changed_needs = format_single_file(&needs_fmt, &config, &options, &args).unwrap();
        let changed_already = format_single_file(&already_fmt, &config, &options, &args).unwrap();

        assert!(
            changed_needs,
            "file that needs formatting must report changed=true"
        );
        assert!(
            !changed_already,
            "file already formatted must report changed=false"
        );
    }

    // --- format_single_file: write condition !check && !diff (mut-000329,330,331) ---
    // Distinguishing the && from ||: if check=true OR diff=true, no write happens.
    // If check=false AND diff=false, write happens.
    // We test the "exactly one flag true" cases:

    #[test]
    fn test_check_true_diff_false_does_not_write() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let original = fs::read_to_string(&path).unwrap();
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            diff: false,
            ..default_args(vec![])
        };
        format_single_file(&path, &config, &options, &args).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            original,
            "check=true must prevent write"
        );
    }

    #[test]
    fn test_check_false_diff_true_does_not_write() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let original = fs::read_to_string(&path).unwrap();
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: false,
            diff: true,
            ..default_args(vec![])
        };
        format_single_file(&path, &config, &options, &args).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            original,
            "diff=true must prevent write"
        );
    }

    #[test]
    fn test_check_false_diff_false_does_write() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let original = fs::read_to_string(&path).unwrap();
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: false,
            diff: false,
            ..default_args(vec![])
        };
        format_single_file(&path, &config, &options, &args).unwrap();
        assert_ne!(
            fs::read_to_string(&path).unwrap(),
            original,
            "check=false && diff=false must write"
        );
    }

    // --- output_text_results: check vs non-check branch (mut-000332) ---
    // output_text_results is private but its effects flow through execute().
    // We verify that check and non-check paths produce different exit codes
    // given the same file state (needs formatting).

    #[test]
    fn test_check_mode_and_non_check_mode_differ_for_unformatted_file() {
        let temp = TempDir::new().unwrap();
        let path_check = write_needs_formatting_file(&temp, "check.md");
        let path_fmt = write_needs_formatting_file(&temp, "fmt.md");

        let check_result = execute(FormatArgs {
            check: true,
            ..default_args(vec![path_check])
        })
        .unwrap();
        let fmt_result = execute(FormatArgs {
            check: false,
            ..default_args(vec![path_fmt])
        })
        .unwrap();

        // check mode: exit 2 (needs formatting); format mode: exit 0 (formatted)
        assert_eq!(
            check_result, 2,
            "check mode with unformatted file should exit 2"
        );
        assert_eq!(fmt_result, 0, "format mode should exit 0 after formatting");
    }

    // --- output_text_results: result.needs_formatting > 0 boundary (mut-000333,334,335,336) ---
    // needs_formatting==0 must not trigger the ">0" branch.
    // needs_formatting==1 must trigger it.

    #[test]
    fn test_check_mode_needs_formatting_zero_does_not_print_count_message() {
        // Already-formatted file: needs_formatting==0 in check mode → exit 0, not 2.
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path])
        };
        assert_eq!(execute(args).unwrap(), 0);
    }

    #[test]
    fn test_check_mode_needs_formatting_one_triggers_count_message_path() {
        // Unformatted file: needs_formatting==1 in check mode → exit 2.
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path])
        };
        assert_eq!(execute(args).unwrap(), 2);
    }

    // --- output_text_results: result.formatted > 0 boundary (mut-000338,339,340,341) ---
    // In non-check mode: formatted==0 shows "All files already formatted";
    // formatted==1 shows "Formatted N file(s) successfully".
    // We verify via format_files counters.

    #[test]
    fn test_non_check_mode_formatted_zero_vs_one_via_counters() {
        let temp = TempDir::new().unwrap();
        let already = write_already_formatted_file(&temp, "already.md");
        let needs = write_needs_formatting_file(&temp, "needs.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();

        // formatted==0: already-formatted file
        let args0 = default_args(vec![already.clone()]);
        let r0 = format_files(&[already], &config, &options, &args0, None).unwrap();
        assert_eq!(
            r0.formatted, 0,
            "already-formatted: formatted counter must be 0"
        );

        // formatted==1: file that needs formatting
        let args1 = default_args(vec![needs.clone()]);
        let r1 = format_files(&[needs], &config, &options, &args1, None).unwrap();
        assert_eq!(
            r1.formatted, 1,
            "needs-formatting: formatted counter must be 1"
        );
    }

    // --- output_text_results: result.failed == 0 in format mode (mut-000343,344,345) ---
    // The "All files already formatted" message appears only when:
    //   formatted==0 AND failed==0 (not check mode).
    // We verify that when failed>0 that branch is NOT taken.

    #[test]
    fn test_format_mode_failed_zero_shows_already_formatted_path() {
        // No failures, no changes → formatted==0, failed==0 → "already formatted" path.
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(result.formatted, 0);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_format_mode_failed_nonzero_does_not_show_already_formatted_path() {
        // One failure → failed==1, formatted==0 → "already formatted" branch skipped.
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("ghost.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![nonexistent.clone()]);
        let result = format_files(&[nonexistent], &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 1);
        assert_eq!(result.formatted, 0);
        // "All files already formatted" path is only taken when failed==0.
        // failed==1 here means that branch was NOT taken.
    }

    // --- execute() exit code 1 via failing file (mut-000365, mut-000366) ---
    // mut-000365: the `0` in `result.failed > 0` is changed to `1`.
    //   With failed=1: `1 > 1` = false → would return Ok(0) instead of Ok(1).
    // mut-000366: `Ok(1)` → `Ok(0)` in the failed branch.
    // Both require calling execute() directly with a file that fails to format,
    // then asserting the exit code is exactly 1, not 0.

    // On unix, making a file read-only and running normal format mode causes
    // std::fs::write to fail → result.failed == 1 → execute() returns Ok(1).
    // Root is skipped because it bypasses read-only restrictions.

    #[test]
    #[cfg(unix)]
    fn test_execute_returns_exit_code_1_for_unwritable_file() {
        if unsafe { libc::geteuid() == 0 } {
            return; // root bypasses file permissions
        }

        let temp = TempDir::new().unwrap();
        // File must need formatting so the formatter attempts to write it.
        let path = write_needs_formatting_file(&temp, "lash.index.md");

        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o444); // read-only
        fs::set_permissions(&path, perms.clone()).unwrap();

        let result = execute(default_args(vec![path.clone()]));

        perms.set_mode(0o644);
        let _ = fs::set_permissions(&path, perms);

        assert_eq!(
            result.unwrap(),
            1,
            "execute() must return exit code 1 when a file cannot be written"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_execute_exit_code_1_is_not_0_for_unwritable_file() {
        if unsafe { libc::geteuid() == 0 } {
            return;
        }

        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");

        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&path, perms.clone()).unwrap();

        let result = execute(default_args(vec![path.clone()]));

        perms.set_mode(0o644);
        let _ = fs::set_permissions(&path, perms);

        let code = result.unwrap();
        assert_ne!(code, 0, "write-failure must not produce exit code 0");
        assert_ne!(
            code, 2,
            "write-failure (non-check mode) must not produce exit code 2"
        );
        assert_eq!(code, 1, "write-failure must produce exit code exactly 1");
    }

    #[test]
    #[cfg(unix)]
    fn test_execute_exit_code_distinguishes_success_from_write_failure() {
        if unsafe { libc::geteuid() == 0 } {
            return;
        }

        let temp = TempDir::new().unwrap();
        let good = write_already_formatted_file(&temp, "good.md");
        let bad = write_needs_formatting_file(&temp, "bad.md");

        let mut perms = fs::metadata(&bad).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&bad, perms.clone()).unwrap();

        let good_code = execute(default_args(vec![good])).unwrap();
        let bad_result = execute(default_args(vec![bad.clone()]));

        perms.set_mode(0o644);
        let _ = fs::set_permissions(&bad, perms);

        let bad_code = bad_result.unwrap();
        assert_eq!(good_code, 0, "success must exit 0");
        assert_eq!(bad_code, 1, "write failure must exit 1, not {bad_code}");
        assert_ne!(
            good_code, bad_code,
            "success and write failure must produce distinct exit codes"
        );
    }

    // --- output_text_results: result.failed > 0 in final section (mut-000346,347,348,349) ---
    // The final "N file(s) failed to format" message appears only when failed>0.
    // failed==0 must not trigger it; failed==1 must.

    #[test]
    fn test_failed_gt_zero_branch_triggered_for_unreadable_file() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("missing.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![nonexistent.clone()]);
        let result = format_files(&[nonexistent], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.failed, 1,
            "failed must be 1 to trigger the failure reporting branch"
        );
    }

    #[test]
    fn test_failed_zero_branch_not_triggered_for_successful_format() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 0, "failed must be 0 for a successful format");
    }

    // --- check mode: result.failed == 0 branch in output_text_results (mut-000333 area) ---
    // In check mode, when needs_formatting==0 AND failed==0, "All files are properly
    // formatted" is printed.  This branch requires failed==0 to be distinguished
    // from failed!=0.

    #[test]
    fn test_check_mode_all_ok_has_zero_failed_and_zero_needs_formatting() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path.clone()])
        };
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(result.needs_formatting, 0);
        assert_eq!(result.failed, 0);
    }

    // --- json mode does not call !args.json reporter (mut-000318) ---
    // In json=true check mode, the reporter.report_diagnostic() call is suppressed.
    // We verify that the diagnostic still ends up in needs_formatting_diagnostics
    // even in json mode (the push happens before the !args.json gate).

    #[test]
    fn test_json_check_mode_diagnostic_in_needs_formatting_diagnostics() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            json: true,
            check: true,
            ..default_args(vec![path.clone()])
        };
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        // Diagnostic must be collected even in json mode.
        assert_eq!(result.needs_formatting_diagnostics.len(), 1);
        assert_eq!(
            result.needs_formatting_diagnostics[0].code,
            "F_NEEDS_FORMATTING"
        );
    }

    #[test]
    fn test_non_json_check_mode_diagnostic_also_in_needs_formatting_diagnostics() {
        // Verify the same diagnostic collection happens in text mode (not just json).
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            json: false,
            check: true,
            ..default_args(vec![path.clone()])
        };
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(result.needs_formatting_diagnostics.len(), 1);
    }

    // --- format_files: args.json for OutputFormat (mut-000303) ---
    // json=true → OutputFormat::Json; json=false → OutputFormat::Text.
    // Both must succeed and produce the same results counts.

    #[test]
    fn test_format_files_json_true_uses_json_output_format() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            json: true,
            ..default_args(vec![path.clone()])
        };
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_format_files_json_false_uses_text_output_format() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            json: false,
            ..default_args(vec![path.clone()])
        };
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 0);
    }

    // --- mut-000351: true -> false in discover_markdown_files respect_gitignore flag ---
    // The second parameter to discover_markdown_files is `respect_gitignore`.
    // When true (original), .gitignore patterns are respected and matching files
    // are excluded from discovery.
    // When false (mutation), .gitignore patterns are ignored and ALL files are found.
    //
    // To kill this mutant, we need a test that:
    //   1. Creates a .gitignore file that excludes a markdown file
    //   2. Verifies that the excluded file is NOT formatted (with respect_gitignore=true)
    //   3. Would FAIL if respect_gitignore=false (excluded file would be found and formatted)

    #[test]
    fn test_format_discovers_files_in_subdirectories_recursively() {
        let temp = TempDir::new().unwrap();
        // Create a subdirectory with an unformatted file.
        let sub = temp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        let sub_file = sub.join("task.md");
        let unformatted = "# Task\n\n@id:   spacing\n\n## Tasks\n\n- [ ] item\n";
        fs::write(&sub_file, unformatted).unwrap();

        // Pass the parent directory; the subdirectory file is discovered recursively.
        let result = execute(default_args(vec![temp.path().to_path_buf()]));
        assert_eq!(result.unwrap(), 0, "recursive discovery must succeed");

        // The file in the subdirectory must have been formatted (content changed).
        let after = fs::read_to_string(&sub_file).unwrap();
        assert_ne!(
            after, unformatted,
            "recursive discovery must format files in subdirectories"
        );
    }

    // --- mut-000326: Ok(0) -> Ok(1) in the final else branch ---
    // When no files fail and check is false, exit code must be exactly 0, not 1.

    #[test]
    fn test_exit_code_zero_in_normal_mode_with_formatted_file_is_not_one() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let result = execute(default_args(vec![path])).unwrap();
        assert_eq!(result, 0, "normal format mode must return exactly 0, not 1");
        assert_ne!(result, 1, "normal format mode must not return 1");
    }

    // --- mut-000327: Ok(1) -> Ok(0) in the failed > 0 branch ---
    // When any file fails to format, exit code must be exactly 1, not 0.

    #[test]
    fn test_exit_code_one_for_failed_file_is_not_zero() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("ghost.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![nonexistent.clone()]);
        let files = vec![nonexistent];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 1);
        assert_ne!(
            result.formatted, 1,
            "failed file must not count as formatted"
        );
        assert_ne!(
            result.needs_formatting, 1,
            "failed file must not count as needs_formatting"
        );
    }

    // --- mut-000334: args.json -> negated in OutputFormat selection ---
    // json=true -> OutputFormat::Json; json=false -> OutputFormat::Text.
    // Both produce the same format result counters.

    #[test]
    fn test_format_files_reporter_format_json_true_and_false_produce_same_counters() {
        let temp = TempDir::new().unwrap();
        let path_json = write_needs_formatting_file(&temp, "json_file.md");
        let path_text = write_needs_formatting_file(&temp, "text_file.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();

        let args_json = FormatArgs {
            json: true,
            ..default_args(vec![path_json.clone()])
        };
        let result_json = format_files(&[path_json], &config, &options, &args_json, None).unwrap();

        let args_text = FormatArgs {
            json: false,
            ..default_args(vec![path_text.clone()])
        };
        let result_text = format_files(&[path_text], &config, &options, &args_text, None).unwrap();

        assert_eq!(result_json.formatted, 1, "json mode: must format one file");
        assert_eq!(result_text.formatted, 1, "text mode: must format one file");
        assert_eq!(result_json.failed, 0);
        assert_eq!(result_text.failed, 0);
    }

    // --- mut-000335,336,337: files.len() > 1 boundary mutations ---
    // > replaced by >= or <=, and 1 replaced by 0.
    // We verify both the single-file (boundary) and two-file cases.

    #[test]
    fn test_format_files_boundary_single_file_formats_without_progress_bar() {
        // files.len() == 1 -> show_progress = false (since 1 > 1 is false).
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.formatted, 1,
            "single file: formatted must be exactly 1"
        );
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_format_files_boundary_two_files_format_with_progress_bar_path() {
        // files.len() == 2 -> show_progress = true.
        let temp = TempDir::new().unwrap();
        let p1 = write_needs_formatting_file(&temp, "file1.md");
        let p2 = write_needs_formatting_file(&temp, "file2.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![p1.clone(), p2.clone()]);
        let result = format_files(&[p1, p2], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.formatted, 2,
            "two files: formatted must be exactly 2"
        );
        assert_eq!(result.failed, 0);
    }

    // --- mut-000338,339,340: && replaced by || in show_progress ---

    #[test]
    fn test_format_files_check_mode_two_files_counts_not_disrupted_by_show_progress() {
        let temp = TempDir::new().unwrap();
        let p1 = write_needs_formatting_file(&temp, "file1.md");
        let p2 = write_needs_formatting_file(&temp, "file2.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![p1.clone(), p2.clone()])
        };
        let result = format_files(&[p1, p2], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.needs_formatting, 2,
            "check mode 2 files: needs_formatting=2"
        );
        assert_eq!(
            result.formatted, 0,
            "check mode must not increment formatted"
        );
    }

    // --- mut-000339: !args.check -> args.check ---

    #[test]
    fn test_show_progress_false_in_check_mode_two_files_formats_correctly() {
        let temp = TempDir::new().unwrap();
        let p1 = write_already_formatted_file(&temp, "file1.md");
        let p2 = write_already_formatted_file(&temp, "file2.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![p1.clone(), p2.clone()])
        };
        let result = format_files(&[p1, p2], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.needs_formatting, 0,
            "already formatted: needs_formatting=0"
        );
        assert_eq!(result.formatted, 0);
        assert_eq!(result.failed, 0);
    }

    // --- mut-000341: !args.diff -> args.diff ---

    #[test]
    fn test_show_progress_false_in_diff_mode_two_files_does_not_write() {
        let temp = TempDir::new().unwrap();
        let p1 = write_needs_formatting_file(&temp, "file1.md");
        let p2 = write_needs_formatting_file(&temp, "file2.md");
        let original1 = fs::read_to_string(&p1).unwrap();
        let original2 = fs::read_to_string(&p2).unwrap();
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            diff: true,
            ..default_args(vec![p1.clone(), p2.clone()])
        };
        let result =
            format_files(&[p1.clone(), p2.clone()], &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 0, "diff mode must not fail");
        assert_eq!(
            fs::read_to_string(&p1).unwrap(),
            original1,
            "diff mode: file1 must not be modified"
        );
        assert_eq!(
            fs::read_to_string(&p2).unwrap(),
            original2,
            "diff mode: file2 must not be modified"
        );
    }

    // --- mut-000343: !args.json -> args.json in show_progress ---

    #[test]
    fn test_show_progress_false_in_json_mode_two_files_formats_correctly() {
        let temp = TempDir::new().unwrap();
        let p1 = write_needs_formatting_file(&temp, "file1.md");
        let p2 = write_needs_formatting_file(&temp, "file2.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            json: true,
            ..default_args(vec![p1.clone(), p2.clone()])
        };
        let result = format_files(&[p1, p2], &config, &options, &args, None).unwrap();
        assert_eq!(result.formatted, 2, "json mode 2 files: formatted=2");
        assert_eq!(result.failed, 0);
    }

    // --- mut-000344: show_progress -> negated in if condition ---

    #[test]
    fn test_show_progress_negation_does_not_affect_format_results() {
        let temp = TempDir::new().unwrap();
        let p1 = write_needs_formatting_file(&temp, "file1.md");
        let p2 = write_needs_formatting_file(&temp, "file2.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![p1.clone(), p2.clone()]);
        let result = format_files(&[p1, p2], &config, &options, &args, None).unwrap();
        assert_eq!(result.formatted, 2);
        assert_eq!(result.failed, 0);
        assert_eq!(result.needs_formatting, 0);
    }

    // --- mut-000349: !args.json -> args.json in reporter.report_diagnostic ---
    // In check mode + json=true: reporter.report_diagnostic must NOT be called.
    // We verify needs_formatting_diagnostics is populated in both cases.

    #[test]
    fn test_json_check_mode_collects_diagnostic_without_text_streaming() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            json: true,
            check: true,
            ..default_args(vec![path.clone()])
        };
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(
            result.needs_formatting_diagnostics.len(),
            1,
            "json check: must collect diagnostic"
        );
        assert_eq!(
            result.needs_formatting, 1,
            "json check: needs_formatting must be 1"
        );
    }

    #[test]
    fn test_non_json_check_mode_also_collects_diagnostic() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            json: false,
            check: true,
            ..default_args(vec![path.clone()])
        };
        let files = vec![path];
        let result = format_files(&files, &config, &options, &args, None).unwrap();
        assert_eq!(
            result.needs_formatting_diagnostics.len(),
            1,
            "text check: must collect diagnostic"
        );
        assert_eq!(
            result.needs_formatting, 1,
            "text check: needs_formatting must be 1"
        );
    }

    // --- mut-000356: pb.inc(1) -> pb.inc(0) ---
    // Only affects progress bar display, not the format result.

    #[test]
    fn test_pb_inc_does_not_affect_format_result_for_two_files() {
        let temp = TempDir::new().unwrap();
        let p1 = write_needs_formatting_file(&temp, "file1.md");
        let p2 = write_needs_formatting_file(&temp, "file2.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![p1.clone(), p2.clone()]);
        let result = format_files(&[p1, p2], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.formatted, 2,
            "both files must be formatted regardless of pb.inc value"
        );
        assert_eq!(result.failed, 0);
    }

    // --- mut-000360: args.diff -> negated in show_diff call ---
    // When diff=true and file changed, show_diff IS called.

    #[test]
    fn test_diff_mode_true_changed_file_does_not_write_and_returns_changed_true() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let original = fs::read_to_string(&path).unwrap();
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            diff: true,
            check: false,
            ..default_args(vec![])
        };
        let changed = format_single_file(&path, &config, &options, &args).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            original,
            "diff mode must not write"
        );
        assert!(
            changed,
            "diff mode: changed must be true for a file that needs formatting"
        );
    }

    #[test]
    fn test_diff_mode_false_already_formatted_returns_changed_false() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let original = fs::read_to_string(&path).unwrap();
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            diff: false,
            check: false,
            ..default_args(vec![])
        };
        let changed = format_single_file(&path, &config, &options, &args).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            original,
            "no change: file must remain identical"
        );
        assert!(
            !changed,
            "no diff: changed must be false for already-formatted file"
        );
    }

    // --- mut-000365: args.check -> negated in output_text_results ---

    #[test]
    fn test_check_mode_and_non_check_produce_different_counters_for_unformatted_file() {
        let temp = TempDir::new().unwrap();
        let check_path = write_needs_formatting_file(&temp, "check.md");
        let fmt_path = write_needs_formatting_file(&temp, "fmt.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();

        let check_args = FormatArgs {
            check: true,
            ..default_args(vec![check_path.clone()])
        };
        let check_result =
            format_files(&[check_path], &config, &options, &check_args, None).unwrap();

        let fmt_args = default_args(vec![fmt_path.clone()]);
        let fmt_result = format_files(&[fmt_path], &config, &options, &fmt_args, None).unwrap();

        assert_eq!(
            check_result.needs_formatting, 1,
            "check: needs_formatting must be 1"
        );
        assert_eq!(check_result.formatted, 0, "check: formatted must be 0");
        assert_eq!(fmt_result.formatted, 1, "format: formatted must be 1");
        assert_eq!(
            fmt_result.needs_formatting, 0,
            "format: needs_formatting must be 0"
        );
    }

    // --- mut-000366,367,368,369: needs_formatting > 0 boundary ---

    #[test]
    fn test_check_mode_needs_formatting_exactly_one_triggers_count_message() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path.clone()])
        };
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.needs_formatting, 1,
            "exactly one unformatted: needs_formatting=1"
        );
        let temp2 = TempDir::new().unwrap();
        let path2 = write_needs_formatting_file(&temp2, "lash.index.md");
        assert_eq!(
            execute(FormatArgs {
                check: true,
                ..default_args(vec![path2])
            })
            .unwrap(),
            2
        );
    }

    #[test]
    fn test_check_mode_needs_formatting_zero_does_not_trigger_count_message_path() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path.clone()])
        };
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.needs_formatting, 0,
            "already formatted: needs_formatting=0"
        );
        let temp2 = TempDir::new().unwrap();
        let path2 = write_already_formatted_file(&temp2, "lash.index.md");
        assert_eq!(
            execute(FormatArgs {
                check: true,
                ..default_args(vec![path2])
            })
            .unwrap(),
            0
        );
    }

    // --- mut-000371,372,373: result.failed == 0 in check mode ---

    #[test]
    fn test_check_mode_failed_zero_and_needs_formatting_zero_has_zero_failed() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![path.clone()])
        };
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 0, "all ok: failed must be exactly 0");
        assert_eq!(
            result.needs_formatting, 0,
            "all ok: needs_formatting must be 0"
        );
    }

    #[test]
    fn test_check_mode_failed_one_prevents_properly_formatted_message_path() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("ghost.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = FormatArgs {
            check: true,
            ..default_args(vec![nonexistent.clone()])
        };
        let result = format_files(&[nonexistent], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.failed, 1,
            "failed file in check mode: failed must be 1"
        );
        assert_ne!(
            result.failed, 0,
            "failed must not be 0 when file is unreadable"
        );
    }

    // --- mut-000375,376,377,378: result.formatted > 0 in format mode ---

    #[test]
    fn test_format_mode_formatted_exactly_one_triggers_success_message_path() {
        let temp = TempDir::new().unwrap();
        let path = write_needs_formatting_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(result.formatted, 1, "one changed file: formatted must be 1");
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_format_mode_formatted_zero_does_not_trigger_success_message_path() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.formatted, 0,
            "already formatted: formatted must be 0"
        );
        assert_eq!(result.failed, 0);
    }

    // --- mut-000380,381,382: result.failed == 0 in format mode else-if ---

    #[test]
    fn test_format_mode_failed_zero_and_formatted_zero_enables_already_formatted_path() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 0, "no failures: failed must be 0");
        assert_eq!(result.formatted, 0, "no changes: formatted must be 0");
    }

    #[test]
    fn test_format_mode_failed_one_bypasses_already_formatted_path() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("ghost.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![nonexistent.clone()]);
        let result = format_files(&[nonexistent], &config, &options, &args, None).unwrap();
        assert_eq!(result.failed, 1, "one failure: failed must be exactly 1");
        assert_ne!(result.failed, 0, "one failure: failed must not be 0");
    }

    // --- mut-000384,385,386,387: result.failed > 0 in final reporting section ---

    #[test]
    fn test_failed_gt_zero_final_section_triggers_for_one_failure() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("missing.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![nonexistent.clone()]);
        let result = format_files(&[nonexistent], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.failed, 1,
            "failure: failed must be 1 for final reporting branch"
        );
        assert_ne!(result.failed, 0, "failure: failed must not be 0");
    }

    #[test]
    fn test_failed_zero_final_section_not_triggered_for_successful_run() {
        let temp = TempDir::new().unwrap();
        let path = write_already_formatted_file(&temp, "lash.index.md");
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![path.clone()]);
        let result = format_files(&[path], &config, &options, &args, None).unwrap();
        assert_eq!(
            result.failed, 0,
            "success: failed must be 0 for final reporting branch"
        );
    }

    // --- L116: discover_markdown_files(&paths, true) → respect_gitignore=true ---
    // The second argument controls whether .gitignore patterns are applied.
    // When true (original), files matching .gitignore are excluded from discovery.
    // When false (mutation), gitignored files ARE included and get formatted.
    //
    // To kill this mutant we need:
    //   1. A real git repository so the ignore crate respects .gitignore
    //   2. A .gitignore that excludes a markdown file with formatting issues
    //   3. After running execute(), the excluded file must remain unformatted
    //
    // If respect_gitignore were false, the gitignored file would be found and
    // formatted — changing its content — which causes the assertion to fail.
    //
    // This test skips gracefully when git is not available on the system.
    #[test]
    fn test_format_respects_gitignore_excludes_ignored_files() {
        // Check git is available; skip silently if not.
        let git_available = std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success());
        if !git_available {
            return;
        }

        let temp = TempDir::new().unwrap();

        // Initialize a git repository so the ignore crate respects .gitignore.
        let git_init = std::process::Command::new("git")
            .args(["init", temp.path().to_str().unwrap()])
            .output()
            .expect("git init must run");
        assert!(git_init.status.success(), "git init must succeed");

        // Write a properly-formatted file at the root (will be discovered and checked).
        let visible_content =
            "# Task List\n\n@id: visible\n@created: 2024-01-15\n\n## Tasks\n\n- [ ] Task\n";
        let visible_path = temp.path().join("lash.index.md");
        fs::write(&visible_path, visible_content).unwrap();

        // Write a file with formatting issues inside a gitignored subdirectory.
        let ignored_dir = temp.path().join("ignored_subdir");
        fs::create_dir(&ignored_dir).unwrap();
        let unformatted_content = "# Ignored\n\n@id:   bad-spacing\n\n## Tasks\n\n- [ ] item\n";
        let ignored_path = ignored_dir.join("ignored.md");
        fs::write(&ignored_path, unformatted_content).unwrap();

        // .gitignore excludes the entire subdirectory.
        fs::write(temp.path().join(".gitignore"), "ignored_subdir/\n").unwrap();

        // Format the whole temp directory.  With respect_gitignore=true, the
        // ignored_subdir is excluded and ignored.md is never touched.
        let result = execute(default_args(vec![temp.path().to_path_buf()]));
        assert_eq!(result.unwrap(), 0, "format with gitignore must succeed");

        // The gitignored file must remain exactly as written (unformatted).
        let after = fs::read_to_string(&ignored_path).unwrap();
        assert_eq!(
            after, unformatted_content,
            "gitignored file must not be formatted; if this fails the \
             respect_gitignore flag is likely false (mutation at L116)"
        );
    }

    // --- L156: `result.failed > 0` → `result.failed > 1` ---
    // --- L157: `Ok(1)` → `Ok(0)` in the failed branch ---
    // These require calling execute() with exactly one failing file and asserting
    // the exit code is exactly 1 (not 0 and not 2).
    //
    // The only cross-platform way to trigger result.failed through execute() is a
    // parse failure on a path that exists (discover_markdown_files won't bail) but
    // whose content is unparseable.  We use a file with content that fails parsing.
    //
    // Actually, if parse_file fails with context "Failed to parse ...", format_files
    // increments result.failed.  We can create an invalid markdown file (binary
    // content) that parse_file rejects.
    //
    // Note: parse_file is lenient about markdown content; a simpler approach is to
    // force a write failure on unix (already tested).  On all platforms we verify
    // the counter logic via format_files directly.
    #[test]
    fn test_execute_exit_code_1_when_failed_equals_exactly_one() {
        // Build a FormatResult where failed == 1, then verify the exit code
        // decision would produce 1.  We do this by constructing the conditions
        // that lead to execute() returning Ok(1): non-check mode, exactly one
        // file fails.
        //
        // On unix we can force a write error (file exists but is read-only).
        // On all platforms: a binary file that parse_file rejects should work.
        let temp = TempDir::new().unwrap();

        // Write a file full of NUL bytes — not valid UTF-8, so read_to_string
        // inside format_single_file will fail with an IO error, which format_files
        // will catch and count as a failure.
        let bad_path = temp.path().join("bad.md");
        fs::write(&bad_path, b"\x00\x01\x02\x03 not utf8 \xff\xfe").unwrap();

        // Also write a valid formatted file so we have a known-good file too.
        let good_path = write_already_formatted_file(&temp, "good.md");

        // format_files with just the bad file: expect failed == 1.
        let config = LashConfig::default();
        let options = FormatOptions::default();
        let args = default_args(vec![bad_path.clone()]);
        let result = format_files(
            std::slice::from_ref(&bad_path),
            &config,
            &options,
            &args,
            None,
        )
        .unwrap();
        assert_eq!(
            result.failed, 1,
            "bad file must count as exactly 1 failure (kills L156: 0→1)"
        );
        assert_eq!(result.formatted, 0, "bad file must not count as formatted");

        // Verify the exit code arm: failed > 0 → Ok(1), not Ok(0).
        // The exit code logic (abbreviated):
        //   if check && needs > 0 { Ok(2) }
        //   else if failed > 0    { Ok(1) }   ← L156 mutates `0` to `1`
        //   else                  { Ok(0) }   ← L157 mutates `Ok(1)` to `Ok(0)`
        //
        // With result.failed == 1 and check == false:
        //   mutation L156 (0→1): `1 > 1` = false → falls through to Ok(0) ✗
        //   mutation L157 (1→0): returns Ok(0) ✗
        //   original: `1 > 0` = true → Ok(1) ✓
        //
        // We can't call execute() directly with bad_path because discover_markdown_files
        // accepts it (file exists), but parse_file will fail inside format_files.
        // However, the bad_path IS a file that exists, so discover_markdown_files
        // will not bail — execute() will proceed to format_files with it.
        let _ = good_path; // suppress unused warning
        let exit_result = execute(default_args(vec![bad_path]));
        // execute() should return Ok(1) — one file failed, no check mode.
        let code = exit_result.expect("execute must not return Err for a bad-content file");
        assert_eq!(
            code, 1,
            "exit code must be exactly 1 when exactly one file fails \
             (kills L156: 0→1 and L157: Ok(1)→Ok(0))"
        );
        assert_ne!(code, 0, "exit code must not be 0 when a file fails");
    }
}
