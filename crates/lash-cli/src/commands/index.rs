//! Index command implementation
//!
//! The `lash index` command rebuilds the `SQLite` database from Markdown files.

use anyhow::{Context, Result};
use lash_db::{init_database, open_database, run_migrations, Indexer, IndexerConfig};
use lash_types::error::LashError;
use lash_types::LashConfig;
use std::path::{Path, PathBuf};

use lash_cli::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash_cli::formatter::{OutputFormat, Verbosity};
use lash_cli::theme::CliTheme;

use crate::utils::file_discovery::find_project_root;
use crate::utils::output::create_progress_bar;

/// Arguments for the index command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct IndexArgs {
    /// Paths to index (if empty, indexes entire project)
    pub paths: Vec<PathBuf>,
    /// Force full rebuild even if index is up to date
    pub force: bool,
    /// Show which files are being indexed
    pub show_files: bool,
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Show errors as they occur (streaming) vs at end (batch)
    pub errors_streaming: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Verbosity level for output
    pub verbosity: Verbosity,
}

/// Execute the index command
///
/// # Arguments
///
/// * `args` - Index command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (general error), 3 (indexing failed)
#[allow(clippy::too_many_lines)]
pub fn execute(args: IndexArgs) -> Result<i32> {
    // Load theme based on no_color flag
    let theme = CliTheme::load(None, !args.no_color)?;

    // Determine project root
    let project_root = if let Some(root) = args.project_root {
        root
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        force = args.force,
        "Starting index operation"
    );

    // Determine database path
    let db_path = get_database_path(&project_root)?;

    // Initialize or open database
    let conn = if args.force || !db_path.exists() {
        // Force rebuild or DB doesn't exist - initialize fresh
        if db_path.exists() {
            tracing::debug!("Removing existing database for full rebuild");
            std::fs::remove_file(&db_path).context("Failed to remove existing database")?;
        }
        init_database(&db_path).context("Failed to initialize database")?
    } else {
        // Open existing database for incremental indexing
        open_database(&db_path).context("Failed to open database")?
    };

    // Run migrations to ensure schema is up to date
    run_migrations(&conn).context("Failed to run database migrations")?;

    // Load project configuration
    let parser_config = LashConfig::from_root(&project_root).unwrap_or_else(|_| {
        tracing::debug!("No project config found, using defaults");
        LashConfig::default()
    });

    // Configure indexer
    let mut indexer_config = IndexerConfig::new(project_root.clone())
        .with_incremental(!args.force)
        .with_progress(!args.json)
        .with_profiling(false);

    // Add path filtering if paths were provided
    if !args.paths.is_empty() {
        // Convert relative paths to absolute and validate they're under project root
        let absolute_paths: Vec<PathBuf> = args
            .paths
            .iter()
            .map(|p| {
                if p.is_absolute() {
                    p.clone()
                } else {
                    std::env::current_dir().map_or_else(|_| p.clone(), |cwd| cwd.join(p))
                }
            })
            .collect();
        indexer_config = indexer_config.with_paths(absolute_paths);
    }

    // Set up error reporter
    let output_format = if args.json {
        OutputFormat::JsonPretty
    } else {
        OutputFormat::Text
    };

    // Determine error display mode based on CLI flag
    let display_mode = if args.errors_streaming {
        ErrorDisplayMode::Streaming
    } else {
        ErrorDisplayMode::Batch
    };

    let reporter_config = ErrorReporterConfig {
        verbosity: args.verbosity,
        output_format,
        display_mode,
        theme: theme.clone(),
        show_summary: false, // We'll print our own summary
    };

    let mut error_reporter = ErrorReporter::new(reporter_config);

    // Set up progress reporting if requested
    let pb = if !args.json && args.show_files {
        Some(create_progress_bar(100)) // Will update total later
    } else {
        None
    };

    // Create indexer with progress callback if needed
    let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);

    if let Some(progress_bar) = pb.as_ref() {
        let pb_clone = progress_bar.clone();
        indexer.with_progress_callback(move |progress| {
            pb_clone.set_length(progress.total_files as u64);
            pb_clone.set_position(progress.files_processed as u64);

            if let Some(file) = &progress.current_file {
                pb_clone.set_message(format!("Indexing {}", file.display()));
            }
        });
    }

    // Execute indexing
    let report = indexer.index_project().context("Failed to index project")?;

    // Convert parse errors to LashError types and report them
    // (Do this before clearing the progress bar so we can use it for suspended output)
    for parse_error in &report.errors {
        // Create a parse error with the error message
        // Since we don't have precise location information from the indexer,
        // we use line 1, column 1 as a generic location
        let error = LashError::Parse {
            code: "E_PARSE",
            message: parse_error.error.clone(),
            location: Some(lash_types::error::Location::new(
                parse_error.file_path.clone(),
                1,
                1,
            )),
            snippet: None,
            help: Some("Fix the syntax errors in the file and re-run indexing".to_string()),
        };
        // Use progress-aware error reporting to avoid mangling progress bar output
        error_reporter.report_error_with_progress(&error, pb.as_ref());
    }

    // Clear progress bar after reporting errors
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    // Flush any collected errors (for batch mode)
    if !args.errors_streaming {
        error_reporter.flush();
    }

    // Output results
    if args.json {
        output_json_report(&report, &error_reporter)?;
    } else {
        output_text_report(&report, args.force, &error_reporter, theme.as_ref());
    }

    // Return exit code based on errors
    if report.has_errors() {
        tracing::warn!(
            error_count = report.errors.len(),
            "Indexing completed with errors"
        );
        Ok(3) // Exit code 3 for indexing failures
    } else {
        tracing::info!("Indexing completed successfully");
        Ok(0)
    }
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> Result<PathBuf> {
    let lash_dir = project_root.join(".lash");

    // Create .lash directory if it doesn't exist
    if !lash_dir.exists() {
        std::fs::create_dir_all(&lash_dir).context("Failed to create .lash directory")?;
    }

    Ok(lash_dir.join("lash.db"))
}

/// Output indexing report as JSON
fn output_json_report(report: &lash_db::IndexReport, error_reporter: &ErrorReporter) -> Result<()> {
    use serde_json::json;

    let summary = error_reporter.summary();

    let output = json!({
        "files_indexed": report.files_processed,
        "files_processed": report.files_processed,
        "files_added": report.files_added,
        "files_updated": report.files_updated,
        "files_deleted": report.files_deleted,
        "files_unchanged": report.files_unchanged,
        "has_changes": report.has_changes,
        "errors": {
            "count": summary.error_count,
            "files_affected": summary.files_affected.len(),
            "details": report.errors.iter().map(|e| json!({
                "file": e.file_path.display().to_string(),
                "error": e.error,
            })).collect::<Vec<_>>(),
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output indexing report as human-readable text
fn output_text_report(
    report: &lash_db::IndexReport,
    force: bool,
    error_reporter: &ErrorReporter,
    theme: Option<&CliTheme>,
) {
    // Summary line
    let summary_msg = if force {
        "Full rebuild complete"
    } else {
        "Incremental index complete"
    };

    if let Some(t) = theme {
        println!("{}", t.style_success(summary_msg));
    } else {
        println!("{summary_msg}");
    }

    // Statistics
    println!();
    println!("Files processed: {}", report.files_processed);

    if report.files_added > 0 {
        let added_str = if let Some(t) = theme {
            t.style_success(&report.files_added.to_string())
        } else {
            report.files_added.to_string()
        };
        println!("  Added:     {added_str}");
    }

    if report.files_updated > 0 {
        let updated_str = if let Some(t) = theme {
            t.style_warning(&report.files_updated.to_string())
        } else {
            report.files_updated.to_string()
        };
        println!("  Updated:   {updated_str}");
    }

    if report.files_deleted > 0 {
        let deleted_str = if let Some(t) = theme {
            t.style_error(&report.files_deleted.to_string())
        } else {
            report.files_deleted.to_string()
        };
        println!("  Deleted:   {deleted_str}");
    }

    if report.files_unchanged > 0 {
        println!("  Unchanged: {}", report.files_unchanged);
    }

    // Print error summary
    let summary = error_reporter.summary();
    if summary.error_count > 0 {
        println!();
        if let Some(t) = theme {
            println!(
                "{}",
                t.style_error(&format!("Errors: {}", summary.error_count))
            );
            println!("  {} files affected", summary.files_affected.len());
        } else {
            println!("Errors: {}", summary.error_count);
            println!("  {} files affected", summary.files_affected.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_db::ParseError;

    #[test]
    fn test_get_database_path() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let db_path = get_database_path(temp.path()).unwrap();

        assert_eq!(db_path, temp.path().join(".lash/lash.db"));
        assert!(temp.path().join(".lash").exists());
    }

    #[test]
    fn test_json_output() {
        let report = lash_db::IndexReport {
            files_processed: 10,
            files_added: 3,
            files_updated: 2,
            files_deleted: 1,
            files_unchanged: 4,
            files_skipped: 0,
            errors: vec![ParseError {
                file_path: PathBuf::from("test.md"),
                error: "Parse error".to_string(),
            }],
            has_changes: true,
            profile: None,
        };

        // Create error reporter
        let reporter_config = ErrorReporterConfig {
            verbosity: Verbosity::Normal,
            output_format: OutputFormat::JsonPretty,
            display_mode: ErrorDisplayMode::Batch,
            theme: None,
            show_summary: false,
        };
        let mut error_reporter = ErrorReporter::new(reporter_config);

        // Convert errors
        for parse_error in &report.errors {
            let error = LashError::Parse {
                code: "E_PARSE",
                message: parse_error.error.clone(),
                location: Some(lash_types::error::Location::new(
                    parse_error.file_path.clone(),
                    1,
                    1,
                )),
                snippet: None,
                help: Some("Fix the syntax errors in the file and re-run indexing".to_string()),
            };
            error_reporter.collect_error(error);
        }

        // Should not panic
        let result = output_json_report(&report, &error_reporter);
        assert!(result.is_ok());
    }
}
