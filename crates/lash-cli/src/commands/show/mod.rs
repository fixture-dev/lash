//! Show command implementation
//!
//! The `lash show` command displays detailed information about a specific task or file.
//!
//! Rendering is split across sibling modules to keep any one file under the
//! project's ~500-line guideline:
//! - `file_view` — `lash show <file>` (text, JSON, tree view)
//! - `task_view` — `lash show <task>` (text, JSON)
//! - `detail` — task-record extras added for GitHub issue #26 (agent notes,
//!   `@depends-on` status, children)
//! - `format` — status-formatting helpers shared by the two views

use anyhow::{Context, Result};
use lash::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash::formatter::{OutputFormat, Verbosity};
use lash::theme::CliTheme;
use lash_core::fuzzy::FuzzyMatcher;
use lash_db::{
    open_database, DependencyRepository, DocRefRepository, FileRepository, TaskRepository,
};
use lash_types::error::LashError;
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

mod detail;
mod file_view;
mod format;
mod task_view;

/// Arguments for the show command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ShowArgs {
    /// Task ID or file path
    pub target: String,
    /// Show dependency tree
    pub deps: bool,
    /// Show reverse dependencies (tasks that depend on this)
    pub rdeps: bool,
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Enable tree view (None = use config default)
    pub tree_view: Option<bool>,
    /// Maximum tree depth
    pub max_depth: Option<usize>,
    /// Use ASCII characters for tree
    pub ascii: bool,
    /// Verbosity level for output
    pub verbosity: Verbosity,
    /// Restrict task output to the terse ID/Title/Status/File/Labels view,
    /// omitting agent notes, dependency status, and children (GitHub #26).
    pub short: bool,
}

/// Execute the show command
///
/// # Arguments
///
/// * `args` - Show command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (general error), 3 (DB error), 5 (not found)
pub fn execute(args: &ShowArgs) -> Result<i32> {
    // Determine project root
    let project_root = if let Some(ref root) = args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        target = %args.target,
        "Starting show operation"
    );

    // Load theme for colored output
    let theme = CliTheme::load(None, !args.no_color)?;

    // Determine database path
    let db_path = get_database_path(&project_root);

    // Check if database exists
    if !db_path.exists() {
        let error = LashError::io_file_not_found(db_path);
        let mut diag = error.to_diagnostic();
        diag.help = Some("Run `lash index` to create the database".to_string());

        if args.json {
            output_json_diagnostic(&diag, &[])?;
        } else {
            let reporter_config = ErrorReporterConfig {
                verbosity: args.verbosity,
                output_format: OutputFormat::Text,
                display_mode: ErrorDisplayMode::Streaming,
                theme: theme.clone(),
                show_summary: false,
            };
            let mut reporter = ErrorReporter::new(reporter_config);
            reporter.report_diagnostic(&diag);
        }
        return Ok(3); // Exit code 3 for DB error
    }

    // Open database
    let conn = match open_database(&db_path) {
        Ok(conn) => conn,
        Err(e) => {
            let error = LashError::index_corrupted(format!("Failed to open database: {e}"));
            let mut diag = error.to_diagnostic();
            diag.help = Some("Try running `lash index` to rebuild the database".to_string());

            if args.json {
                output_json_diagnostic(&diag, &[])?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormat::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: theme.clone(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_diagnostic(&diag);
            }
            return Ok(3); // Exit code 3 for DB error
        }
    };

    // Create repositories
    let task_repo = TaskRepository::new(&conn);
    let file_repo = FileRepository::new(&conn);
    let dep_repo = DependencyRepository::new(&conn);
    let doc_repo = DocRefRepository::new(&conn);

    // Determine if target is a task ID or file path
    // Check for path separators (both forward and back slashes) or .md extension
    let is_file_path = args.target.contains('/')
        || args.target.contains('\\')
        || std::path::Path::new(&args.target)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));

    // Show file or task information and return appropriate exit code
    let exit_code = if is_file_path {
        // Show file information
        file_view::show_file(
            &file_repo,
            &task_repo,
            &doc_repo,
            args,
            &project_root,
            theme.as_ref(),
        )?
    } else {
        // Show task information
        task_view::show_task(
            &task_repo,
            &file_repo,
            &dep_repo,
            &doc_repo,
            args,
            &project_root,
            theme.as_ref(),
        )?
    };

    Ok(exit_code)
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/lash.db")
}

/// Output error as JSON
fn output_json_error(error: &LashError) -> Result<()> {
    let diagnostic = error.to_diagnostic();
    output_json_diagnostic(&diagnostic, &[])
}

/// Output diagnostic as JSON
fn output_json_diagnostic(
    diagnostic: &lash_types::error::Diagnostic,
    suggestions: &[(String, f64)],
) -> Result<()> {
    use serde_json::json;

    let mut output = json!({
        "error": diagnostic.message,
        "code": diagnostic.code,
        "suggestion": diagnostic.help.clone().unwrap_or_else(|| "Run `lash index` to ensure the database is up to date".to_string()),
    });

    // Include suggestions if available
    if !suggestions.is_empty() {
        let suggestion_list: Vec<_> = suggestions
            .iter()
            .map(|(id, score)| {
                json!({
                    "id": id,
                    "score": score
                })
            })
            .collect();
        output["similar_ids"] = json!(suggestion_list);
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Find similar task IDs using fuzzy matching
///
/// Returns a list of (`task_id`, score) pairs sorted by score descending.
fn find_similar_task_ids(query: &str, candidates: &[String]) -> Vec<(String, f64)> {
    let fuzzy_matcher = FuzzyMatcher::new(0.5, 5); // Lower threshold for more suggestions
    let results = fuzzy_matcher.find_matches(query, candidates);
    results.into_iter().map(|c| (c.task_id, c.score)).collect()
}
