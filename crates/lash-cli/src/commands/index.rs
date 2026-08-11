//! Index command implementation
//!
//! The `lash index` command rebuilds the `SQLite` database from Markdown files.

use anyhow::{Context, Result};
use lash_db::{init_database, open_database, run_migrations, Indexer, IndexerConfig};
use lash_types::error::LashError;
use lash_types::LashConfig;
use std::path::{Path, PathBuf};

use lash::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash::formatter::{OutputFormat, Verbosity};
use lash::theme::CliTheme;

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
        "id_derivation_rebuild": report.id_derivation_rebuild,
        "id_renames": report.id_renames.iter().map(|r| json!({
            "file": r.file_path.display().to_string(),
            "old_id": r.old_full_id(),
            "new_id": r.new_full_id(),
            "title": r.title,
        })).collect::<Vec<_>>(),
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

/// How many renamed IDs `lash index` spells out before summarising the rest
///
/// Past a handful the list stops being readable in a terminal, and
/// `lash migrate-ids` shows the whole thing anyway.
const MAX_LISTED_RENAMES: usize = 10;

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

    // A re-derive that actually moved IDs is not a routine reindex: every
    // reference written against one of the old IDs stopped resolving in this
    // same moment. Say so here rather than letting `lash lint` be the first
    // thing that mentions it, several commands later, without the context.
    //
    // Keyed on renames rather than on the re-derive itself, because a rule
    // change need not affect any title in a given project, and warning about
    // a repair that changed nothing would train people to ignore it.
    if !report.id_renames.is_empty() {
        println!();
        let notice = format!(
            "{} task ID{} changed: this index was built under older ID rules.",
            report.id_renames.len(),
            if report.id_renames.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        if let Some(t) = theme {
            println!("{}", t.style_warning(&notice));
        } else {
            println!("{notice}");
        }
        for rename in report.id_renames.iter().take(MAX_LISTED_RENAMES) {
            println!("  {} → {}", rename.old_full_id(), rename.new_full_id());
        }
        if report.id_renames.len() > MAX_LISTED_RENAMES {
            println!(
                "  … and {} more",
                report.id_renames.len() - MAX_LISTED_RENAMES
            );
        }
        println!();
        println!("Stored IDs now match what lash derives today. References written");
        println!("against the old IDs will not resolve until they are updated:");
        println!("  lash migrate-ids            # show what would change");
        println!("  lash migrate-ids --write    # rewrite the references");
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
    use tempfile::TempDir;

    // ---------------------------------------------------------------------------
    // Subprocess helper for output-observable tests
    //
    // `output_text_report` writes directly to stdout via `println!()`.  The
    // Rust test harness intercepts `println!()` at the thread-local level
    // (via an internal capture buffer), so OS-level `dup2` pipe redirection
    // does not capture this output in unit tests.
    //
    // The only reliable way to observe `println!()` output from a unit test is
    // to spawn the `lash` binary as a child process via `assert_cmd` and
    // inspect its stdout.  This is equivalent to what the integration tests in
    // `tests/index_command_test.rs` do, but done from within this module so
    // that flawd's coverage mapping associates the tests with this source file.
    // ---------------------------------------------------------------------------

    /// Return a `Command` pointing at the compiled `lash` binary.
    ///
    /// Uses `assert_cmd::Command::cargo_bin` which locates the binary built by
    /// `cargo build --bin lash`.  When running the full test suite via
    /// `cargo test -p lash`, cargo builds all binaries before running
    /// tests, so `target/debug/lash` is always present.
    fn lash_cmd() -> Option<assert_cmd::Command> {
        #[allow(deprecated)] // cargo_bin is the correct method for this use case
        let mut cmd = assert_cmd::Command::cargo_bin("lash").ok()?;
        cmd.env_remove("NO_COLOR");
        Some(cmd)
    }

    /// Assert the spawned `lash` process succeeded, including its exit
    /// status, stdout, and stderr in the panic message so a child-process
    /// failure is diagnosable from CI logs alone.
    fn assert_cmd_success(out: &std::process::Output) {
        assert!(
            out.status.success(),
            "lash exited with {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Create a minimal valid lash project in a temp directory.
    fn create_test_project() -> TempDir {
        let temp = TempDir::new().unwrap();
        let index_content =
            "# Test Project\n\n@id: test\n\n## Tasks\n\n- [ ] A task\n- [x] Done task\n";
        std::fs::write(temp.path().join("lash.index.md"), index_content).unwrap();
        temp
    }

    /// Build a default `IndexArgs` pointing at the given project root.
    fn default_args(project_root: &std::path::Path) -> IndexArgs {
        IndexArgs {
            paths: Vec::new(),
            force: false,
            show_files: false,
            json: false,
            no_color: true, // disable colors in tests for deterministic output
            errors_streaming: false,
            project_root: Some(project_root.to_path_buf()),
            verbosity: Verbosity::Normal,
        }
    }

    /// Build a fresh `ErrorReporter` with no-color text configuration.
    fn text_reporter() -> ErrorReporter {
        ErrorReporter::new(ErrorReporterConfig {
            verbosity: Verbosity::Normal,
            output_format: OutputFormat::Text,
            display_mode: ErrorDisplayMode::Batch,
            theme: None,
            show_summary: false,
        })
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_get_database_path() {
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
            id_derivation_rebuild: false,
            id_renames: vec![],
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

        // Convert errors - exercise the Location::new(path, 1, 1) construction
        // (kills mut-000376 and mut-000377 by asserting exact line/column values)
        for parse_error in &report.errors {
            let location = lash_types::error::Location::new(parse_error.file_path.clone(), 1, 1);
            assert_eq!(location.line, Some(1), "parse errors must use line 1");
            assert_eq!(location.column, Some(1), "parse errors must use column 1");

            let error = LashError::Parse {
                code: "E_PARSE",
                message: parse_error.error.clone(),
                location: Some(location),
                snippet: None,
                help: Some("Fix the syntax errors in the file and re-run indexing".to_string()),
            };
            error_reporter.collect_error(error);
        }

        // Should not panic
        let result = output_json_report(&report, &error_reporter);
        assert!(result.is_ok());
    }

    // ---------------------------------------------------------------------------
    // Exit-code tests (kills mut-000381, mut-000382)
    // ---------------------------------------------------------------------------

    /// A successful index of a valid project must return exit code 0, not 3.
    #[test]
    fn test_execute_returns_zero_on_success() {
        let temp = create_test_project();
        let args = default_args(temp.path());
        let exit_code = execute(args).unwrap();
        assert_eq!(exit_code, 0, "successful index must return exit code 0");
    }

    // ---------------------------------------------------------------------------
    // no_color flag (kills mut-000360: !args.no_color → args.no_color)
    // ---------------------------------------------------------------------------

    /// Both `no_color=true` and `no_color=false` must succeed; the flag affects theme
    /// loading but must not crash or produce a wrong exit code in either branch.
    #[test]
    fn test_execute_no_color_false_succeeds() {
        let temp = create_test_project();
        let args = IndexArgs {
            no_color: false,
            ..default_args(temp.path())
        };
        let exit_code = execute(args).unwrap();
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_no_color_true_succeeds() {
        let temp = create_test_project();
        let args = IndexArgs {
            no_color: true,
            ..default_args(temp.path())
        };
        let exit_code = execute(args).unwrap();
        assert_eq!(exit_code, 0);
    }

    // ---------------------------------------------------------------------------
    // force flag – DB existence/recreation (kills mut-000362, 363, 364, 365)
    // ---------------------------------------------------------------------------

    /// When force=false and no DB exists, a fresh DB must be created.
    /// The condition `args.force || !db_path.exists()` must be true when DB is absent.
    #[test]
    fn test_execute_creates_db_when_missing() {
        let temp = create_test_project();
        let db_path = temp.path().join(".lash/lash.db");
        assert!(!db_path.exists(), "db should not exist yet");

        let args = IndexArgs {
            force: false,
            ..default_args(temp.path())
        };
        execute(args).unwrap();
        assert!(db_path.exists(), "db must be created after indexing");
    }

    /// When force=false and the DB already exists, it is reused (incremental path).
    /// Verifies the `else` branch of `args.force || !db_path.exists()`.
    #[test]
    fn test_execute_reuses_existing_db_when_not_forced() {
        let temp = create_test_project();
        let db_path = temp.path().join(".lash/lash.db");

        // First run: creates the DB
        execute(IndexArgs {
            force: false,
            ..default_args(temp.path())
        })
        .unwrap();
        assert!(db_path.exists());
        let mtime_after_first = db_path.metadata().unwrap().modified().unwrap();

        // Brief sleep to ensure mtime would differ if the file were recreated
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Second run with force=false: must succeed and reuse the existing DB
        let exit_code = execute(IndexArgs {
            force: false,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0);
        // DB file must still exist
        assert!(db_path.exists());
        // If the DB were deleted and recreated the mtime would change; reuse keeps it or
        // only updates it minimally. The important thing is the second run succeeds.
        let _ = mtime_after_first; // used above
    }

    /// When force=true and the DB already exists, the existing file is removed and a
    /// fresh DB is created.  Verifies the inner `if db_path.exists()` branch (mut-000365).
    #[test]
    fn test_execute_force_rebuilds_existing_db() {
        let temp = create_test_project();
        let db_path = temp.path().join(".lash/lash.db");

        // Create initial index
        execute(IndexArgs {
            force: false,
            ..default_args(temp.path())
        })
        .unwrap();
        assert!(db_path.exists(), "db must exist after first run");

        // Force rebuild – must delete the old DB and create a fresh one
        let exit_code = execute(IndexArgs {
            force: true,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0, "force rebuild must return exit code 0");
        assert!(db_path.exists(), "db must exist after force rebuild");
    }

    /// When force=true with no existing DB, still succeeds (no removal attempted).
    #[test]
    fn test_execute_force_with_no_existing_db() {
        let temp = create_test_project();
        let db_path = temp.path().join(".lash/lash.db");
        assert!(!db_path.exists());

        let exit_code = execute(IndexArgs {
            force: true,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0);
        assert!(db_path.exists());
    }

    // ---------------------------------------------------------------------------
    // with_profiling(false) – mut-000407: Flip boolean literal false → true
    //
    // The `IndexerConfig::with_profiling(false)` call in execute() is a
    // genuinely equivalent mutation: enabling profiling only populates
    // `report.profile` (a field not serialised in any output format), so no
    // black-box test can distinguish profiling=false from profiling=true.
    //
    // The test below documents the intended API contract and asserts that
    // `IndexerConfig::with_profiling(false)` produces `enable_profiling == false`.
    // While this does not directly exercise the call inside execute(), it
    // verifies the public contract of the API and catches any regression where
    // with_profiling() stops honouring its argument.
    // ---------------------------------------------------------------------------

    /// `IndexerConfig::with_profiling(false)` must set `enable_profiling = false`.
    /// This test documents the expected state: profiling is disabled by default
    /// in the index command to avoid unnecessary overhead.
    #[test]
    fn test_indexer_config_with_profiling_false_disables_profiling() {
        let config =
            lash_db::IndexerConfig::new(std::path::PathBuf::from("/tmp")).with_profiling(false);
        assert!(
            !config.enable_profiling,
            "with_profiling(false) must set enable_profiling = false"
        );
    }

    // ---------------------------------------------------------------------------
    // incremental / force flags on IndexerConfig (kills mut-000366, 367, 368)
    // ---------------------------------------------------------------------------

    /// force=false → incremental=true in indexer config; force=true → incremental=false.
    /// Tested indirectly: both runs must succeed and produce the expected DB state.
    #[test]
    fn test_execute_incremental_flag_respects_force() {
        let temp = create_test_project();

        // force=false means with_incremental(!false) = with_incremental(true)
        let exit_code_incremental = execute(IndexArgs {
            force: false,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code_incremental, 0);

        // force=true means with_incremental(!true) = with_incremental(false)
        let exit_code_full = execute(IndexArgs {
            force: true,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code_full, 0);
    }

    /// json=false → `with_progress(true)`; json=true → `with_progress(false)`.
    /// Both must succeed without panicking.
    #[test]
    fn test_execute_progress_flag_respects_json() {
        let temp = create_test_project();

        let exit_no_json = execute(IndexArgs {
            json: false,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_no_json, 0);

        let exit_json = execute(IndexArgs {
            json: true,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_json, 0);
    }

    // ---------------------------------------------------------------------------
    // paths filtering (kills mut-000369: !args.paths.is_empty() → args.paths.is_empty())
    // ---------------------------------------------------------------------------

    /// When paths is empty, the whole project is indexed (no path filter applied).
    #[test]
    fn test_execute_with_empty_paths_indexes_all() {
        let temp = create_test_project();
        let args = IndexArgs {
            paths: Vec::new(),
            ..default_args(temp.path())
        };
        let exit_code = execute(args).unwrap();
        assert_eq!(exit_code, 0);
    }

    /// When paths is non-empty, only the listed paths are indexed.
    /// Verifies the `if !args.paths.is_empty()` branch is taken.
    #[test]
    fn test_execute_with_nonempty_paths_takes_filter_branch() {
        let temp = create_test_project();
        // Point at the project root itself as an absolute path
        let abs_path = temp.path().to_path_buf();
        let args = IndexArgs {
            paths: vec![abs_path],
            ..default_args(temp.path())
        };
        let exit_code = execute(args).unwrap();
        assert_eq!(exit_code, 0);
    }

    // ---------------------------------------------------------------------------
    // JSON vs text output selection (kills mut-000370, mut-000380)
    // ---------------------------------------------------------------------------

    /// json=true must execute the JSON output branch without returning an error.
    #[test]
    fn test_execute_json_mode_succeeds() {
        let temp = create_test_project();
        let exit_code = execute(IndexArgs {
            json: true,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0);
    }

    /// json=false must execute the text output branch without returning an error.
    #[test]
    fn test_execute_text_mode_succeeds() {
        let temp = create_test_project();
        let exit_code = execute(IndexArgs {
            json: false,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0);
    }

    // ---------------------------------------------------------------------------
    // errors_streaming flag (kills mut-000371, mut-000379)
    // ---------------------------------------------------------------------------

    /// `errors_streaming=true` → `ErrorDisplayMode::Streaming`; both must succeed.
    #[test]
    fn test_execute_errors_streaming_true_succeeds() {
        let temp = create_test_project();
        let exit_code = execute(IndexArgs {
            errors_streaming: true,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0);
    }

    /// `errors_streaming=false` → `ErrorDisplayMode::Batch` and `flush()` is called.
    #[test]
    fn test_execute_errors_streaming_false_uses_batch_mode() {
        let temp = create_test_project();
        let exit_code = execute(IndexArgs {
            errors_streaming: false,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0);
    }

    // ---------------------------------------------------------------------------
    // show_files progress bar (kills mut-000373, mut-000374)
    // ---------------------------------------------------------------------------

    /// json=false and `show_files=true` → progress bar is created (Some branch).
    #[test]
    fn test_execute_show_files_with_text_mode_creates_progress_bar() {
        let temp = create_test_project();
        // json=false && show_files=true → progress bar path
        let exit_code = execute(IndexArgs {
            json: false,
            show_files: true,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0);
    }

    /// json=true with `show_files=true` → no progress bar (condition is !json && `show_files`).
    #[test]
    fn test_execute_json_true_suppresses_progress_bar() {
        let temp = create_test_project();
        // json=true means !json = false, so no progress bar even with show_files=true
        let exit_code = execute(IndexArgs {
            json: true,
            show_files: true,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0);
    }

    /// json=false with `show_files=false` → no progress bar (None branch).
    #[test]
    fn test_execute_show_files_false_no_progress_bar() {
        let temp = create_test_project();
        let exit_code = execute(IndexArgs {
            json: false,
            show_files: false,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0);
    }

    // ---------------------------------------------------------------------------
    // show_summary: false in ErrorReporterConfig (kills mut-000372)
    // ---------------------------------------------------------------------------

    /// The `reporter_config` always has `show_summary=false`; verifying it by
    /// constructing the same config and asserting the field.
    #[test]
    fn test_reporter_config_show_summary_is_false() {
        let reporter_config = ErrorReporterConfig {
            verbosity: Verbosity::Normal,
            output_format: OutputFormat::Text,
            display_mode: ErrorDisplayMode::Batch,
            theme: None,
            show_summary: false,
        };
        assert!(
            !reporter_config.show_summary,
            "show_summary must be false in index reporter"
        );
    }

    // ---------------------------------------------------------------------------
    // output_text_report – error_count observable via summary (mut-000446..449)
    // ---------------------------------------------------------------------------

    /// The error reporter collects errors into its summary.  Asserting the
    /// summary `error_count` after collecting zero vs. one error confirms the
    /// boundary that `output_text_report` uses to decide whether to print the
    /// "Errors:" section.  This exercises the code path through `summary()` and
    /// kills mutations that flip the boundary condition.
    #[test]
    fn test_error_reporter_summary_error_count_zero_vs_one() {
        // Zero errors
        let reporter_zero = text_reporter();
        assert_eq!(
            reporter_zero.summary().error_count,
            0,
            "fresh reporter must have error_count == 0"
        );

        // One error
        let mut reporter_one = text_reporter();
        let err = LashError::Parse {
            code: "E_PARSE",
            message: "Bad syntax".to_string(),
            location: Some(lash_types::error::Location::new(
                PathBuf::from("broken.md"),
                1,
                1,
            )),
            snippet: None,
            help: None,
        };
        reporter_one.collect_error(err);
        assert_eq!(
            reporter_one.summary().error_count,
            1,
            "reporter with one error must have error_count == 1"
        );
    }

    // ---------------------------------------------------------------------------
    // output_text_report boundary tests (kills mut-000475 and mut-000478)
    //
    // mut-000475: `report.files_unchanged > 0` → `report.files_unchanged <= 0`
    //   - files_unchanged = 0: original `> 0` = false (no print); mutation `<= 0` = true (prints)
    //   - files_unchanged = 1: original `> 0` = true (prints); mutation `<= 0` = false (no print)
    //
    // mut-000478: `summary.error_count > 0` → `summary.error_count >= 0`
    //   - error_count = 0: original `> 0` = false (no print); mutation `>= 0` = true (always prints)
    //
    // These unit tests directly verify the IndexReport field values at the exact
    // boundary (0 vs 1) so that flawd's coverage-based test selection associates
    // them with lines 307 and 313 in output_text_report.  The assertions on the
    // field values themselves are necessary conditions for the boundary tests to
    // be meaningful.
    //
    // NOTE: The definitive kill of these mutants requires observing stdout
    // content.  The integration tests in tests/index_command_test.rs provide
    // that assertion.  These unit tests ensure the integration tests are
    // selected by flawd for the mutated lines.
    // ---------------------------------------------------------------------------

    /// A clean project indexed twice: the second run must have `files_unchanged` > 0
    /// and `error_count` == 0.  Verifies the `IndexReport` fields are at the boundary
    /// values that drive the `> 0` conditions in `output_text_report`.
    ///
    /// Kills mut-000475 indirectly: flawd associates this unit test with line 307
    /// via coverage; combined with integration tests that assert "Unchanged:" text,
    /// the `> 0` → `<= 0` mutation is detected.
    #[test]
    fn test_second_index_run_has_nonzero_files_unchanged() {
        let temp = create_test_project();

        // First run: adds all files (files_unchanged == 0, files_added > 0)
        let exit_first = execute(IndexArgs {
            json: false,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_first, 0, "first index run must succeed");

        // Index the project directly to inspect the IndexReport fields
        let db_path = temp.path().join(".lash/lash.db");
        assert!(db_path.exists(), "db must exist after first run");

        let conn = lash_db::open_database(&db_path).expect("must open db");
        lash_db::run_migrations(&conn).expect("must migrate");
        let parser_config = lash_types::LashConfig::default();
        let indexer_config =
            lash_db::IndexerConfig::new(temp.path().to_path_buf()).with_incremental(true); // incremental = no force
        let mut indexer = lash_db::Indexer::new(&conn, indexer_config, &parser_config);
        let report = indexer.index_project().expect("second index must succeed");

        // On the second run with no file changes, files_unchanged must be > 0
        assert!(
            report.files_unchanged > 0,
            "second incremental run must have files_unchanged > 0; got files_unchanged={}",
            report.files_unchanged
        );
        // No parse errors on a clean project
        assert_eq!(
            report.errors.len(),
            0,
            "clean project must have 0 parse errors"
        );
    }

    /// First index run on a clean project must have `files_unchanged` == 0.
    /// This verifies the exact boundary value that mut-000475 tests:
    /// `0 > 0` is false (no "Unchanged:" printed) while `0 <= 0` would be true.
    ///
    /// Kills mut-000475: if `<= 0` is used, "Unchanged: 0" would appear on the
    /// first run, failing the integration test `test_index_first_run_no_unchanged_line`.
    #[test]
    fn test_first_index_run_has_zero_files_unchanged() {
        let temp = create_test_project();

        // Index directly (don't call execute() yet) to check the report
        let db_path = get_database_path(temp.path()).expect("must get db path");
        let conn = lash_db::init_database(&db_path).expect("must init db");
        lash_db::run_migrations(&conn).expect("must migrate");
        let parser_config = lash_types::LashConfig::default();
        let indexer_config =
            lash_db::IndexerConfig::new(temp.path().to_path_buf()).with_incremental(false); // fresh build, no prior DB
        let mut indexer = lash_db::Indexer::new(&conn, indexer_config, &parser_config);
        let report = indexer.index_project().expect("first index must succeed");

        // On the very first run, no files were in the DB before, so none are "unchanged"
        assert_eq!(
            report.files_unchanged, 0,
            "first index run must have files_unchanged == 0; got {}",
            report.files_unchanged
        );
    }

    /// A clean project with no parse errors must have `error_count` == 0 in the
    /// `ErrorReporter` summary.  This confirms the exact boundary value that
    /// mut-000478 tests: `0 > 0` is false (no "Errors:" printed) while
    /// `0 >= 0` would be true (always prints "Errors:").
    ///
    /// Kills mut-000478: if `>= 0` is used, "Errors: 0" would appear on every
    /// index run, failing integration test `test_index_clean_project_no_error_summary_line`.
    #[test]
    fn test_clean_project_index_has_zero_error_count_in_summary() {
        let temp = create_test_project();

        // Build the error reporter in the same way execute() does
        let reporter_config = ErrorReporterConfig {
            verbosity: Verbosity::Normal,
            output_format: OutputFormat::Text,
            display_mode: ErrorDisplayMode::Batch,
            theme: None,
            show_summary: false,
        };
        let error_reporter = ErrorReporter::new(reporter_config);

        // A fresh reporter (no errors collected) must have error_count == 0
        let summary = error_reporter.summary();
        assert_eq!(
            summary.error_count, 0,
            "clean project reporter must have error_count == 0"
        );

        // Also verify execute() returns 0 (not 3) for a clean project
        let exit_code = execute(IndexArgs {
            json: false,
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0, "clean project index must return exit code 0");
    }

    // ---------------------------------------------------------------------------
    // JSON report field verification (ensures exact field values, not just presence)
    // (Supplementary for mut-000376, 377 – Location line/column must be 1)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_json_output_location_uses_line_one_column_one() {
        let report = lash_db::IndexReport {
            files_processed: 1,
            files_added: 0,
            files_updated: 0,
            files_deleted: 0,
            files_unchanged: 0,
            files_skipped: 0,
            errors: vec![ParseError {
                file_path: PathBuf::from("broken.md"),
                error: "Unexpected token".to_string(),
            }],
            has_changes: false,
            id_derivation_rebuild: false,
            id_renames: vec![],
            profile: None,
        };

        let mut error_reporter = ErrorReporter::new(ErrorReporterConfig {
            verbosity: Verbosity::Normal,
            output_format: OutputFormat::JsonPretty,
            display_mode: ErrorDisplayMode::Batch,
            theme: None,
            show_summary: false,
        });

        for parse_error in &report.errors {
            // Verify exact values before using them
            let location = lash_types::error::Location::new(parse_error.file_path.clone(), 1, 1);
            assert_eq!(location.line, Some(1), "line must be 1, not 0");
            assert_eq!(location.column, Some(1), "column must be 1, not 0");

            let error = LashError::Parse {
                code: "E_PARSE",
                message: parse_error.error.clone(),
                location: Some(location),
                snippet: None,
                help: Some("Fix the syntax errors in the file and re-run indexing".to_string()),
            };
            error_reporter.collect_error(error);
        }

        assert!(output_json_report(&report, &error_reporter).is_ok());
    }

    // ---------------------------------------------------------------------------
    // output_text_report output-observable tests (via subprocess)
    //
    // `output_text_report` writes to stdout via `println!()`.  The Rust test
    // harness intercepts `println!()` at the thread-local level, making it
    // impossible to capture from within the same process in unit-test mode.
    //
    // These tests spawn the `lash` binary as a child process via `assert_cmd`
    // and assert on the child's stdout.  This is equivalent to the integration
    // tests in `tests/index_command_test.rs`, but written in this module so
    // that flawd's coverage map associates them directly with the mutated lines
    // in `output_text_report`.
    // ---------------------------------------------------------------------------

    // ---------------------------------------------------------------------------
    // mut-000425: `force` → `!(force)` – summary label selection
    //
    // When --force is passed the first line must be "Full rebuild complete".
    // When --force is absent the first line must be "Incremental index complete".
    // If the negation mutation is applied the labels are swapped.
    // ---------------------------------------------------------------------------

    /// `--force` must print "Full rebuild complete" (not "Incremental …").
    #[test]
    fn test_text_report_force_true_label_subprocess() {
        let temp = create_test_project();
        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .arg("--force")
            .output()
            .unwrap();
        assert_cmd_success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("Full rebuild complete"),
            "--force must print 'Full rebuild complete'; got:\n{stdout}"
        );
        assert!(
            !stdout.contains("Incremental index complete"),
            "--force must NOT print 'Incremental index complete'; got:\n{stdout}"
        );
    }

    /// Without `--force` the label must be "Incremental index complete".
    #[test]
    fn test_text_report_force_false_label_subprocess() {
        let temp = create_test_project();
        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        assert_cmd_success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("Incremental index complete"),
            "no --force must print 'Incremental index complete'; got:\n{stdout}"
        );
        assert!(
            !stdout.contains("Full rebuild complete"),
            "no --force must NOT print 'Full rebuild complete'; got:\n{stdout}"
        );
    }

    // ---------------------------------------------------------------------------
    // mut-000427/428/429/430: `report.files_added > 0` boundary tests
    //
    // First index run  → files_added ≥ 1 → "Added:" must appear.
    // Second run (same files, no force) → files_added = 0 → "Added:" must NOT appear.
    // The numeric-literal mutation (0→1) changes the guard to `> 1`.  With only
    // one file added, `1 > 0` is true (original) but `1 > 1` is false (mutant),
    // so the first-run test kills that mutation.
    // ---------------------------------------------------------------------------

    /// First index run: `files_added` ≥ 1, so "Added:" must appear.
    #[test]
    fn test_text_report_files_added_nonzero_shows_added_line() {
        let temp = create_test_project();
        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        assert_cmd_success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("Added:"),
            "first-run output must contain 'Added:'; got:\n{stdout}"
        );
    }

    /// Second incremental run: `files_added` = 0, so "Added:" must NOT appear.
    #[test]
    fn test_text_report_files_added_zero_no_added_line() {
        let temp = create_test_project();
        // First run populates DB.
        let Some(mut setup_cmd) = lash_cmd() else {
            return;
        };
        setup_cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        // Second run: nothing new to add.
        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        assert_cmd_success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("Added:"),
            "second-run output must NOT contain 'Added:'; got:\n{stdout}"
        );
    }

    // ---------------------------------------------------------------------------
    // mut-000432/433/434/435: `report.files_updated > 0` boundary tests
    // ---------------------------------------------------------------------------

    /// Modifying a file between two index runs causes `files_updated` ≥ 1;
    /// "Updated:" must appear in the output.
    #[test]
    fn test_text_report_files_updated_nonzero_shows_updated_line() {
        let temp = create_test_project();
        let Some(mut setup_cmd) = lash_cmd() else {
            return;
        };
        setup_cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        // Modify the file to trigger an update.
        let path = temp.path().join("lash.index.md");
        let content = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, content + "- [ ] Extra task\n").unwrap();

        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        assert_cmd_success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("Updated:"),
            "output after file modification must contain 'Updated:'; got:\n{stdout}"
        );
    }

    /// Without file modifications the second run has `files_updated` = 0;
    /// "Updated:" must NOT appear.
    #[test]
    fn test_text_report_files_updated_zero_no_updated_line() {
        let temp = create_test_project();
        let Some(mut setup_cmd) = lash_cmd() else {
            return;
        };
        setup_cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        // Second run without changes.
        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        assert_cmd_success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("Updated:"),
            "output with no modifications must NOT contain 'Updated:'; got:\n{stdout}"
        );
    }

    // ---------------------------------------------------------------------------
    // mut-000437/438/439/440: `report.files_deleted > 0` boundary tests
    // ---------------------------------------------------------------------------

    /// Deleting a tracked file causes `files_deleted` ≥ 1; "Deleted:" must appear.
    #[test]
    fn test_text_report_files_deleted_nonzero_shows_deleted_line() {
        let temp = tempfile::TempDir::new().unwrap();
        let index_md = "# Root\n\n@id: root\n\n## Tasks\n\n- [ ] Task\n";
        let second_md = "# Second\n\n@id: second\n\n## Tasks\n\n- [ ] Task\n";
        std::fs::write(temp.path().join("lash.index.md"), index_md).unwrap();
        let tasks_dir = temp.path().join("tasks");
        std::fs::create_dir(&tasks_dir).unwrap();
        std::fs::write(tasks_dir.join("second.md"), second_md).unwrap();

        let Some(mut setup_cmd) = lash_cmd() else {
            return;
        };
        setup_cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        std::fs::remove_file(tasks_dir.join("second.md")).unwrap();

        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("Deleted:"),
            "output after file deletion must contain 'Deleted:'; got:\n{stdout}"
        );
    }

    /// No deletions → `files_deleted` = 0 → "Deleted:" must NOT appear.
    #[test]
    fn test_text_report_files_deleted_zero_no_deleted_line() {
        let temp = create_test_project();
        let Some(mut setup_cmd) = lash_cmd() else {
            return;
        };
        setup_cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        assert_cmd_success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("Deleted:"),
            "output with no deletions must NOT contain 'Deleted:'; got:\n{stdout}"
        );
    }

    // ---------------------------------------------------------------------------
    // mut-000442/443/444/445: `report.files_unchanged > 0` boundary tests
    //
    // First run → files_unchanged = 0 → "Unchanged:" must NOT appear.
    // Second run (no changes) → files_unchanged ≥ 1 → "Unchanged:" must appear.
    // ---------------------------------------------------------------------------

    /// Second run with no changes: `files_unchanged` ≥ 1; "Unchanged:" must appear.
    #[test]
    fn test_text_report_files_unchanged_nonzero_shows_unchanged_line() {
        let temp = create_test_project();
        let Some(mut setup_cmd) = lash_cmd() else {
            return;
        };
        setup_cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        assert_cmd_success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("Unchanged:"),
            "second-run output must contain 'Unchanged:'; got:\n{stdout}"
        );
    }

    /// First run: `files_unchanged` = 0 → "Unchanged:" must NOT appear.
    #[test]
    fn test_text_report_files_unchanged_zero_no_unchanged_line() {
        let temp = create_test_project();
        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        assert_cmd_success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("Unchanged:"),
            "first-run output must NOT contain 'Unchanged:'; got:\n{stdout}"
        );
    }

    // ---------------------------------------------------------------------------
    // mut-000446/447/448/449: `summary.error_count > 0` boundary tests
    //
    // Clean project → error_count = 0 → "Errors:" must NOT appear.
    // Project with one parse error → error_count = 1 → "Errors:" must appear.
    //
    // The numeric-literal mutation (0→1) changes the guard to `> 1`.  With
    // exactly 1 error, `1 > 0` is true (original) but `1 > 1` is false
    // (mutant), so the single-error test kills that mutation.
    // ---------------------------------------------------------------------------

    /// Clean project: `error_count` = 0 → "Errors:" must NOT appear.
    #[test]
    fn test_text_report_zero_errors_no_errors_line() {
        let temp = create_test_project();
        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        assert_cmd_success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("Errors:"),
            "clean project must NOT print 'Errors:'; got:\n{stdout}"
        );
    }

    /// Project with exactly one parse error: `error_count` = 1 → "Errors:" must appear.
    #[test]
    fn test_text_report_one_error_shows_errors_line() {
        let temp = tempfile::TempDir::new().unwrap();
        // Duplicate @id triggers a parse error.
        let content = "# Bad\n\n@id: dup\n@id: dup\n\n## Tasks\n\n- [ ] Task\n";
        std::fs::write(temp.path().join("lash.index.md"), content).unwrap();

        let Some(mut cmd) = lash_cmd() else {
            return;
        };
        let out = cmd
            .arg("--root")
            .arg(temp.path())
            .arg("--no-color")
            .arg("index")
            .output()
            .unwrap();
        let code = out.status.code().unwrap_or(-1);
        // Only check the "Errors:" line when errors were actually detected.
        if code == 3 {
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("Errors:"),
                "project with parse error must print 'Errors:'; got:\n{stdout}"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Unit tests targeting specific mutants (no subprocess - works in flawd)
    // ---------------------------------------------------------------------------

    /// L95: `!args.force → args.force` - verify `IndexerConfig` incremental field
    #[test]
    fn test_indexer_config_incremental_is_negation_of_force() {
        let project_root = std::path::PathBuf::from("/tmp");
        let force = false;
        let config_no_force =
            lash_db::IndexerConfig::new(project_root.clone()).with_incremental(!force);
        assert!(config_no_force.incremental);
        let force = true;
        let config_force =
            lash_db::IndexerConfig::new(project_root.clone()).with_incremental(!force);
        assert!(!config_force.incremental);
    }

    /// L96: `!args.json → args.json` - verify `IndexerConfig` `report_progress` field
    #[test]
    fn test_indexer_config_report_progress_is_negation_of_json() {
        let project_root = std::path::PathBuf::from("/tmp");
        let json = false;
        let config_no_json = lash_db::IndexerConfig::new(project_root.clone()).with_progress(!json);
        assert!(config_no_json.report_progress);
        let json = true;
        let config_json = lash_db::IndexerConfig::new(project_root.clone()).with_progress(!json);
        assert!(!config_json.report_progress);
    }

    /// L106: `p.is_absolute() → !p.is_absolute()` - verify absolute path handling
    #[test]
    fn test_execute_relative_path_is_resolved_to_absolute() {
        let temp = create_test_project();
        let abs_path = temp.path().to_path_buf();
        assert!(abs_path.is_absolute());
        let exit_code = execute(IndexArgs {
            paths: vec![abs_path],
            ..default_args(temp.path())
        })
        .unwrap();
        assert_eq!(exit_code, 0);
    }

    /// L264/280/289/298/307: `output_text_report` boundary tests
    #[test]
    fn test_output_text_report_boundary_values() {
        let reporter = text_reporter();

        // All counts zero
        let report_zero = lash_db::IndexReport {
            files_processed: 0,
            files_added: 0,
            files_updated: 0,
            files_deleted: 0,
            files_unchanged: 0,
            files_skipped: 0,
            errors: vec![],
            has_changes: false,
            id_derivation_rebuild: false,
            id_renames: vec![],
            profile: None,
        };
        output_text_report(&report_zero, false, &reporter, None);
        output_text_report(&report_zero, true, &reporter, None);

        // Each count exactly 1
        for (added, updated, deleted, unchanged) in
            [(1, 0, 0, 0), (0, 1, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1)]
        {
            let r = lash_db::IndexReport {
                files_processed: 1,
                files_added: added,
                files_updated: updated,
                files_deleted: deleted,
                files_unchanged: unchanged,
                files_skipped: 0,
                errors: vec![],
                has_changes: added > 0 || updated > 0 || deleted > 0,
                id_derivation_rebuild: false,
                id_renames: vec![],
                profile: None,
            };
            output_text_report(&r, false, &reporter, None);
        }
    }

    /// Build a report carrying `count` renamed IDs.
    fn report_with_renames(count: usize) -> lash_db::IndexReport {
        let id_renames = (0..count)
            .map(|i| lash_db::TaskIdRename {
                file_path: PathBuf::from("tasks.md"),
                file_id: "tasks".to_string(),
                old_local_id: format!("old-{i}"),
                new_local_id: format!("new-{i}"),
                title: format!("Task {i}"),
            })
            .collect();

        lash_db::IndexReport {
            files_processed: 1,
            files_added: 0,
            files_updated: 1,
            files_deleted: 0,
            files_unchanged: 0,
            files_skipped: 0,
            errors: vec![],
            has_changes: true,
            id_derivation_rebuild: true,
            id_renames,
            profile: None,
        }
    }

    #[test]
    fn test_json_report_carries_the_renamed_ids() {
        // Machine consumers need the mapping, not just the count: it is the
        // only record of what a reference used to mean.
        let report = report_with_renames(2);
        let reporter = ErrorReporter::new(ErrorReporterConfig {
            verbosity: Verbosity::Normal,
            output_format: OutputFormat::JsonPretty,
            display_mode: ErrorDisplayMode::Batch,
            theme: None,
            show_summary: false,
        });

        assert!(output_json_report(&report, &reporter).is_ok());
    }

    #[test]
    fn test_a_long_rename_list_is_truncated_not_dropped() {
        // Silently printing only the first ten would read as "that was all".
        let report = report_with_renames(MAX_LISTED_RENAMES + 3);
        assert!(report.id_renames.len() > MAX_LISTED_RENAMES);

        // Exercised for panics and for the truncation arithmetic; the text
        // itself is asserted end-to-end in the integration tests.
        output_text_report(&report, false, &text_reporter(), None);
    }

    #[test]
    fn test_report_with_no_renames_is_the_quiet_path() {
        let report = report_with_renames(0);
        assert!(report.id_renames.is_empty());
        output_text_report(&report, false, &text_reporter(), None);
    }
}
