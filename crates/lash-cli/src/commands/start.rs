//! Start command implementation
//!
//! The `lash start` command marks one or more tasks as in-progress by updating
//! the checkbox in the source markdown file from `[ ]` or `[!]` to `[>]`.

use anyhow::{Context, Result};
use lash_cli::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash_cli::formatter::{OutputFormat, Verbosity};
use lash_cli::theme::CliTheme;
use lash_core::fuzzy::FuzzyMatcher;
use lash_db::{open_database, FileRepository, Indexer, IndexerConfig, TaskRepository};
use lash_types::config::LashConfig;
use lash_types::error::LashError;
use lash_types::TaskStatus;
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;
use crate::utils::task_target::TargetError;

/// Arguments for the start command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct StartArgs {
    /// Task ID(s) to mark as in-progress
    pub task_ids: Vec<String>,
    /// Preview what would be changed without modifying files
    pub dry_run: bool,
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Verbosity level for output
    pub verbosity: Verbosity,
}

/// Result of starting a single task
#[derive(Debug, Clone, Serialize)]
pub struct StartResult {
    /// Task full ID
    pub task_id: String,
    /// File path where task was updated
    pub file_path: PathBuf,
    /// Previous status before starting
    pub previous_status: String,
}

/// Error for a single task that could not be started
#[derive(Debug, Clone, Serialize)]
pub struct StartError {
    /// The task ID that was requested
    pub task_id: String,
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Fuzzy match suggestions (if task not found)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
}

/// Execute the start command
///
/// # Arguments
///
/// * `args` - Start command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (validation error), 3 (DB error), 5 (not found)
pub fn execute(args: &StartArgs) -> Result<i32> {
    // Determine project root
    let project_root = if let Some(ref root) = args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        task_ids = ?args.task_ids,
        dry_run = args.dry_run,
        "Starting start operation"
    );

    // Load theme for colored output
    let theme = CliTheme::load(None, !args.no_color)?;

    // Determine database path
    let db_path = project_root.join(".lash/lash.db");

    // Check if database exists
    if !db_path.exists() {
        let error = LashError::io_file_not_found(db_path.clone());
        let mut diag = error.to_diagnostic();
        diag.help = Some("Run `lash index` to create the database".to_string());

        if args.json {
            output_json_error(&diag.message, "E_DB_NOT_FOUND", None)?;
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
                output_json_error(&diag.message, "E_DB_CORRUPTED", None)?;
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

    // Deduplicate task IDs
    let unique_task_ids: Vec<String> = {
        let mut seen = HashSet::new();
        args.task_ids
            .iter()
            .filter(|id| seen.insert(id.to_lowercase()))
            .cloned()
            .collect()
    };

    // Process each task ID
    let mut results: Vec<StartResult> = Vec::new();
    let mut errors: Vec<StartError> = Vec::new();

    for task_id in &unique_task_ids {
        match process_task(task_id, &task_repo, &file_repo, &project_root, args.dry_run) {
            Ok(result) => results.push(result),
            Err(error) => errors.push(error),
        }
    }

    // Re-index if we made changes and not in dry-run mode
    if !args.dry_run && !results.is_empty() {
        if let Err(e) = reindex_project(&project_root) {
            tracing::warn!("Failed to re-index after starting: {e}");
            // Don't fail the command, just warn
        }
    }

    // Output results
    if args.json {
        output_json_results(&results, &errors)?;
    } else {
        output_text_results(&results, &errors, args.dry_run, theme.as_ref());
    }

    // Determine exit code
    if errors.is_empty() {
        Ok(0) // All succeeded
    } else if results.is_empty() {
        // All failed - check what kind of errors
        if errors.iter().any(|e| e.code == "E_NOT_FOUND") {
            Ok(5) // Not found
        } else {
            Ok(1) // Validation error
        }
    } else {
        Ok(1) // Partial success
    }
}

/// Process a single task ID
fn process_task(
    task_id: &str,
    task_repo: &TaskRepository,
    file_repo: &FileRepository,
    project_root: &Path,
    dry_run: bool,
) -> std::result::Result<StartResult, StartError> {
    // Try to find the task by full id or bare @id.
    let task = match crate::utils::task_target::resolve_task_target(task_repo, task_id) {
        Ok(task) => task,
        Err(TargetError::NotFound) => {
            // Try fuzzy matching
            let all_task_ids = task_repo.get_all_full_ids().unwrap_or_default();
            let suggestions = find_similar_task_ids(task_id, &all_task_ids);

            return Err(StartError {
                task_id: task_id.to_string(),
                code: "E_NOT_FOUND".to_string(),
                message: format!("Task not found: {task_id}"),
                suggestions: suggestions.into_iter().map(|(id, _)| id).collect(),
            });
        }
        Err(TargetError::Ambiguous(candidates)) => {
            return Err(StartError {
                task_id: task_id.to_string(),
                code: "E_AMBIGUOUS".to_string(),
                message: format!(
                    "Task @id '{task_id}' is ambiguous; matches {} tasks",
                    candidates.len()
                ),
                suggestions: candidates,
            });
        }
        Err(TargetError::Db(e)) => {
            return Err(StartError {
                task_id: task_id.to_string(),
                code: "E_DB_ERROR".to_string(),
                message: format!("Database error: {e}"),
                suggestions: vec![],
            });
        }
    };

    // Check if task can be started
    match task.status {
        TaskStatus::InProgress => {
            return Err(StartError {
                task_id: task_id.to_string(),
                code: "E_ALREADY_IN_PROGRESS".to_string(),
                message: format!("Task '{}' is already in progress", task.full_id),
                suggestions: vec![],
            });
        }
        TaskStatus::Done => {
            return Err(StartError {
                task_id: task_id.to_string(),
                code: "E_ALREADY_COMPLETE".to_string(),
                message: format!("Task '{}' is already complete", task.full_id),
                suggestions: vec![],
            });
        }
        TaskStatus::Waived => {
            return Err(StartError {
                task_id: task_id.to_string(),
                code: "E_WAIVED".to_string(),
                message: format!("Task '{}' is waived (not applicable)", task.full_id),
                suggestions: vec![],
            });
        }
        TaskStatus::Open | TaskStatus::Blocked => {
            // Can be started
        }
    }

    // Get file information
    let file = match file_repo.get_by_db_id(task.file_id) {
        Ok(Some(file)) => file,
        Ok(None) => {
            return Err(StartError {
                task_id: task_id.to_string(),
                code: "E_FILE_NOT_FOUND".to_string(),
                message: format!("File not found for task '{}'", task.full_id),
                suggestions: vec![],
            });
        }
        Err(e) => {
            return Err(StartError {
                task_id: task_id.to_string(),
                code: "E_DB_ERROR".to_string(),
                message: format!("Database error: {e}"),
                suggestions: vec![],
            });
        }
    };

    // Update the markdown file (unless dry-run)
    if !dry_run {
        if let Err(e) = update_markdown_task_status(
            project_root,
            &file.path,
            &task.title,
            task.status,
            TaskStatus::InProgress,
        ) {
            return Err(StartError {
                task_id: task_id.to_string(),
                code: "E_FILE_UPDATE".to_string(),
                message: format!("Failed to update file: {e}"),
                suggestions: vec![],
            });
        }
    }

    Ok(StartResult {
        task_id: task.full_id.clone(),
        file_path: file.path.clone(),
        previous_status: task.status.as_str().to_string(),
    })
}

/// Find similar task IDs using fuzzy matching
fn find_similar_task_ids(query: &str, candidates: &[String]) -> Vec<(String, f64)> {
    let fuzzy_matcher = FuzzyMatcher::new(0.5, 5);
    let results = fuzzy_matcher.find_matches(query, candidates);
    results.into_iter().map(|c| (c.task_id, c.score)).collect()
}

/// Get checkbox character for a task status
fn status_checkbox_char(status: TaskStatus) -> char {
    match status {
        TaskStatus::Open => ' ',
        TaskStatus::InProgress => '>',
        TaskStatus::Done => 'x',
        TaskStatus::Waived => '-',
        TaskStatus::Blocked => '!',
    }
}

/// Update task status in the markdown file
///
/// Finds the task line by matching the title and old status, then updates
/// the checkbox character to reflect the new status.
fn update_markdown_task_status(
    project_root: &Path,
    file_path: &Path,
    task_title: &str,
    old_status: TaskStatus,
    new_status: TaskStatus,
) -> Result<()> {
    // Construct full path
    let full_path = project_root.join(file_path);

    // Read file content
    let content = fs::read_to_string(&full_path)
        .with_context(|| format!("Failed to read file: {}", full_path.display()))?;

    // Build pattern to find the task line
    // Task lines look like: "- [ ] Task title" with optional leading whitespace
    let old_char = status_checkbox_char(old_status);
    let new_char = status_checkbox_char(new_status);

    // Escape special regex characters in the title
    let escaped_title = regex::escape(task_title);

    // Pattern: whitespace, dash, space, checkbox with old status, space, title
    // Handle both uppercase and lowercase 'x' for Done status
    let pattern = if old_status == TaskStatus::Done {
        format!(r"^(\s*- \[)[xX](\] {escaped_title})")
    } else {
        format!(r"^(\s*- \[){old_char}(\] {escaped_title})")
    };

    let re = Regex::new(&pattern).context("Failed to compile regex")?;

    // Find and replace the task line
    let mut found = false;
    let updated_content: String = content
        .lines()
        .map(|line| {
            if !found && re.is_match(line) {
                found = true;
                re.replace(line, format!("${{1}}{new_char}${{2}}"))
                    .to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Preserve trailing newline if original had one
    let final_content = if content.ends_with('\n') && !updated_content.ends_with('\n') {
        format!("{updated_content}\n")
    } else {
        updated_content
    };

    if !found {
        anyhow::bail!("Could not find task '{task_title}' in file");
    }

    // Write updated content back to file
    fs::write(&full_path, final_content)
        .with_context(|| format!("Failed to write file: {}", full_path.display()))?;

    Ok(())
}

/// Re-index the project after starting tasks
fn reindex_project(project_root: &Path) -> Result<()> {
    let db_path = project_root.join(".lash").join("lash.db");
    let conn = open_database(&db_path).context("Failed to open database for re-indexing")?;

    let config = LashConfig::from_root(project_root).unwrap_or_default();
    let indexer_config = IndexerConfig::new(project_root.to_path_buf()).with_incremental(true);
    let mut indexer = Indexer::new(&conn, indexer_config, &config);

    indexer
        .index_project()
        .context("Failed to re-index after task start")?;

    Ok(())
}

/// Output results as JSON
fn output_json_results(results: &[StartResult], errors: &[StartError]) -> Result<()> {
    let json = serde_json::json!({
        "success": errors.is_empty(),
        "started": results,
        "errors": errors,
    });
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

/// Output a single JSON error
fn output_json_error(message: &str, code: &str, help: Option<&str>) -> Result<()> {
    let mut json = serde_json::json!({
        "success": false,
        "error": {
            "code": code,
            "message": message,
        },
    });
    if let Some(h) = help {
        json["error"]["help"] = serde_json::Value::String(h.to_string());
    }
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

/// Output results as text
fn output_text_results(
    results: &[StartResult],
    errors: &[StartError],
    dry_run: bool,
    theme: Option<&CliTheme>,
) {
    // Print results
    if dry_run && !results.is_empty() {
        println!("Would start:");
    }

    for result in results {
        if let Some(t) = theme {
            if dry_run {
                println!(
                    "  {} {} ({})",
                    t.style_info("[>]"),
                    t.style_label(&result.task_id),
                    t.style_muted(&result.file_path.display().to_string())
                );
            } else {
                println!(
                    "{} {} -> {}",
                    t.style_info("[>]"),
                    t.style_label(&result.task_id),
                    t.style_muted(&result.file_path.display().to_string())
                );
            }
        } else if dry_run {
            println!("  [>] {} ({})", result.task_id, result.file_path.display());
        } else {
            println!("[>] {} ({})", result.task_id, result.file_path.display());
        }
    }

    // Print errors
    for error in errors {
        if let Some(t) = theme {
            eprintln!(
                "{} [{}]: {}",
                t.style_error("Error"),
                error.code,
                error.message
            );
            if !error.suggestions.is_empty() {
                let suggestions_str = error.suggestions.join(", ");
                eprintln!(
                    "  {} Did you mean: {}",
                    t.style_info("hint:"),
                    suggestions_str
                );
            }
        } else {
            eprintln!("Error [{}]: {}", error.code, error.message);
            if !error.suggestions.is_empty() {
                eprintln!("  hint: Did you mean: {}", error.suggestions.join(", "));
            }
        }
    }

    // Print summary if there were mixed results
    if !results.is_empty() && !errors.is_empty() {
        println!();
        if let Some(t) = theme {
            println!(
                "{}: {} started, {} failed",
                t.style_info("Summary"),
                results.len(),
                errors.len()
            );
        } else {
            println!(
                "Summary: {} started, {} failed",
                results.len(),
                errors.len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_checkbox_char() {
        assert_eq!(status_checkbox_char(TaskStatus::Open), ' ');
        assert_eq!(status_checkbox_char(TaskStatus::InProgress), '>');
        assert_eq!(status_checkbox_char(TaskStatus::Done), 'x');
        assert_eq!(status_checkbox_char(TaskStatus::Waived), '-');
        assert_eq!(status_checkbox_char(TaskStatus::Blocked), '!');
    }

    #[test]
    fn test_start_result_serialization() {
        let result = StartResult {
            task_id: "test#task-1".to_string(),
            file_path: PathBuf::from("tasks.md"),
            previous_status: "open".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test#task-1"));
        assert!(json.contains("tasks.md"));
        assert!(json.contains("open"));
    }

    #[test]
    fn test_start_error_serialization() {
        let error = StartError {
            task_id: "test#task-1".to_string(),
            code: "E_NOT_FOUND".to_string(),
            message: "Task not found".to_string(),
            suggestions: vec!["test#task-2".to_string()],
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("E_NOT_FOUND"));
        assert!(json.contains("test#task-2"));
    }

    #[test]
    fn test_start_error_no_suggestions() {
        let error = StartError {
            task_id: "test#task-1".to_string(),
            code: "E_ALREADY_IN_PROGRESS".to_string(),
            message: "Already in progress".to_string(),
            suggestions: vec![],
        };
        let json = serde_json::to_string(&error).unwrap();
        // Empty suggestions should not appear in JSON
        assert!(!json.contains("suggestions"));
    }
}
