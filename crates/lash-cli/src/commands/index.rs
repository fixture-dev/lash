//! Index command implementation
//!
//! The `lash index` command rebuilds the `SQLite` database from Markdown files.

use anyhow::{Context, Result};
use lash_db::{init_database, open_database, Indexer, IndexerConfig};
use lash_types::LashConfig;
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;
use crate::utils::output::create_progress_bar;

/// Arguments for the index command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct IndexArgs {
    /// Force full rebuild even if index is up to date
    pub force: bool,
    /// Show which files are being indexed
    pub show_files: bool,
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
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
pub fn execute(args: IndexArgs) -> Result<i32> {
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

    // Load project configuration
    let parser_config = LashConfig::from_root(&project_root).unwrap_or_else(|_| {
        tracing::debug!("No project config found, using defaults");
        LashConfig::default()
    });

    // Configure indexer
    let indexer_config = IndexerConfig::new(project_root.clone())
        .with_incremental(!args.force)
        .with_progress(!args.json)
        .with_profiling(false);

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

    // Clear progress bar
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    // Output results
    if args.json {
        output_json_report(&report)?;
    } else {
        output_text_report(&report, args.force, args.no_color);
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

    Ok(lash_dir.join("db.sqlite"))
}

/// Output indexing report as JSON
fn output_json_report(report: &lash_db::IndexReport) -> Result<()> {
    use serde_json::json;

    let output = json!({
        "files_processed": report.files_processed,
        "files_added": report.files_added,
        "files_updated": report.files_updated,
        "files_deleted": report.files_deleted,
        "files_unchanged": report.files_unchanged,
        "has_changes": report.has_changes,
        "errors": report.errors.iter().map(|e| json!({
            "file": e.file_path.display().to_string(),
            "error": e.error,
        })).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output indexing report as human-readable text
fn output_text_report(report: &lash_db::IndexReport, force: bool, no_color: bool) {
    use owo_colors::OwoColorize;

    let use_color = !no_color;

    // Summary line
    if force {
        if use_color {
            println!("{}", "Full rebuild complete".green().bold());
        } else {
            println!("Full rebuild complete");
        }
    } else if use_color {
        println!("{}", "Incremental index complete".green().bold());
    } else {
        println!("Incremental index complete");
    }

    // Statistics
    println!();
    println!("Files processed: {}", report.files_processed);

    if report.files_added > 0 {
        if use_color {
            println!("  Added:     {}", report.files_added.to_string().green());
        } else {
            println!("  Added:     {}", report.files_added);
        }
    }

    if report.files_updated > 0 {
        if use_color {
            println!("  Updated:   {}", report.files_updated.to_string().yellow());
        } else {
            println!("  Updated:   {}", report.files_updated);
        }
    }

    if report.files_deleted > 0 {
        if use_color {
            println!("  Deleted:   {}", report.files_deleted.to_string().red());
        } else {
            println!("  Deleted:   {}", report.files_deleted);
        }
    }

    if report.files_unchanged > 0 {
        println!("  Unchanged: {}", report.files_unchanged);
    }

    // Show errors if any
    if !report.errors.is_empty() {
        println!();
        if use_color {
            println!(
                "{}",
                format!("Errors ({})", report.errors.len()).red().bold()
            );
        } else {
            println!("Errors ({})", report.errors.len());
        }

        for error in &report.errors {
            if use_color {
                println!(
                    "  {} {}",
                    error.file_path.display().to_string().yellow(),
                    error.error.red()
                );
            } else {
                println!("  {}: {}", error.file_path.display(), error.error);
            }
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

        assert_eq!(db_path, temp.path().join(".lash/db.sqlite"));
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
            errors: vec![ParseError {
                file_path: PathBuf::from("test.md"),
                error: "Parse error".to_string(),
            }],
            has_changes: true,
            profile: None,
        };

        // Should not panic
        let result = output_json_report(&report);
        assert!(result.is_ok());
    }
}
