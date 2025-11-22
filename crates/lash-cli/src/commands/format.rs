//! Format command implementation
//!
//! The `lash format` command auto-formats Lash task files to enforce consistent style.

#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::similar_names)]

use anyhow::{Context, Result};
use lash_cli::command::Command;
use lash_cli::context::Context as CliContext;
use lash_core::formatter::{FormatOptions, Formatter};
use lash_core::parser::parse_file;
use lash_types::{error::Result as LashResult, LashConfig};
use similar::{ChangeTag, TextDiff};
use std::path::{Path, PathBuf};
use tracing::instrument;

use crate::utils::file_discovery::{discover_markdown_files, find_project_root};
use crate::utils::output::create_progress_bar;

/// Arguments for the format command
#[derive(Debug, Clone)]
pub struct FormatArgs {
    /// Paths to format (files or directories)
    pub paths: Vec<PathBuf>,
    /// Check formatting without modifying (dry-run)
    pub check: bool,
    /// Show diff of changes
    pub diff: bool,
    /// Only normalize formatting, don't apply lint fixes
    pub no_fix: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
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
        eprintln!("No markdown files found to format");
        return Ok(0);
    }

    // Load project configuration
    let project_config = load_project_config(&files)?;

    // Configure formatter
    let format_options = configure_formatter(&args);

    // Format files
    let result = format_files(&files, &project_config, &format_options, &args)?;

    // Determine exit code
    if args.check && result.needs_formatting > 0 {
        eprintln!("\n{} file(s) need formatting", result.needs_formatting);
        Ok(2)
    } else if result.formatted > 0 {
        eprintln!("\nFormatted {} file(s) successfully", result.formatted);
        Ok(0)
    } else {
        if !args.check {
            eprintln!("All files already formatted");
        }
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
#[instrument(skip(files, config, options, args), fields(file_count = files.len()))]
fn format_files(
    files: &[PathBuf],
    config: &LashConfig,
    options: &FormatOptions,
    args: &FormatArgs,
) -> anyhow::Result<FormatResult> {
    let mut result = FormatResult::default();

    let show_progress = files.len() > 1 && !args.check && !args.diff;
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
                        println!("{}", file_path.display());
                    }
                } else if changed {
                    result.formatted += 1;
                }
            }
            Err(e) => {
                if let Some(ref pb) = pb {
                    pb.finish_and_clear();
                }
                eprintln!("Error formatting {}: {}", file_path.display(), e);
                result.failed += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configure_formatter_default() {
        let args = FormatArgs {
            paths: vec![],
            check: false,
            diff: false,
            no_fix: false,
            project_root: None,
        };

        let options = configure_formatter(&args);
        assert!(options.apply_auto_fixes);
    }

    #[test]
    fn test_configure_formatter_no_fix() {
        let args = FormatArgs {
            paths: vec![],
            check: false,
            diff: false,
            no_fix: true,
            project_root: None,
        };

        let options = configure_formatter(&args);
        assert!(!options.apply_auto_fixes);
    }
}
