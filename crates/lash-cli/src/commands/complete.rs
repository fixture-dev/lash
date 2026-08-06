//! Complete command implementation
//!
//! The `lash complete` command marks one or more tasks as complete by updating
//! the checkbox in the source markdown file from `[ ]` or `[!]` to `[x]`.

use anyhow::{Context, Result};
use lash_cli::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash_cli::formatter::{OutputFormat, Verbosity};
use lash_cli::theme::CliTheme;
use lash_core::dependency::reference::resolve_reference;
use lash_core::linter::LintContext;
use lash_db::{open_database, FileRepository, TaskRepository};
use lash_types::config::LashConfig;
use lash_types::dependency::DependencyKind;
use lash_types::error::LashError;
use lash_types::{TaskFile, TaskStatus};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::status_mutation::{
    self, find_similar_task_ids, preview_cascade_children, update_markdown_task_status,
    CascadeOutcome,
};
use crate::utils::file_discovery::find_project_root;
use crate::utils::project_loader::{find_task_by_full_id, load_project};
use crate::utils::task_target::TargetError;

/// Arguments for the complete command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct CompleteArgs {
    /// Task ID(s) to mark as complete
    pub task_ids: Vec<String>,
    /// Preview what would be changed without modifying files
    pub dry_run: bool,
    /// Also mark unchecked plain-bullet children (without their own @id)
    /// as complete. Plain-bullet children are sub-step breakdowns of the
    /// parent rather than independently tracked tasks; cascading prevents
    /// the silent footgun where the parent flips to `[x]` while its visible
    /// sub-checkboxes stay `[ ]`.
    pub cascade: bool,
    /// Complete even when a resolvable `@depends-on` target is still open.
    /// When false (the default) completion is refused until every dependency
    /// is done or waived.
    pub force: bool,
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Verbosity level for output
    pub verbosity: Verbosity,
}

/// Result of completing a single task
#[derive(Debug, Clone, Serialize)]
pub struct CompleteResult {
    /// Task full ID
    pub task_id: String,
    /// File path where task was updated
    pub file_path: PathBuf,
    /// Previous status before completion
    pub previous_status: String,
    /// Number of plain-bullet children also marked complete by --cascade
    #[serde(skip_serializing_if = "is_zero")]
    pub cascaded_children: usize,
    /// Plain-bullet children that were left unchecked (because --cascade
    /// was not set). Empty when there were none or when --cascade ran.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unchecked_children: Vec<String>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// Error for a single task that could not be completed
#[derive(Debug, Clone, Serialize)]
pub struct CompleteError {
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

/// A dependency of the task being completed that is not yet done or waived.
#[derive(Debug, Clone)]
struct UnmetDep {
    /// Target task full id (`file-id#task-id`).
    full_id: String,
    /// Target task title, for the refusal message.
    title: String,
    /// Target task status string (e.g. `open`, `in-progress`).
    status: String,
}

/// Compute the dependencies of `source_full_id` that are not yet done or
/// waived, resolving each `@depends-on` reference with the shared resolver so
/// the gate matches what `lash lint`/`check-links` report.
fn find_unmet_dependencies(
    config: &LashConfig,
    project: &HashMap<PathBuf, TaskFile>,
    source_full_id: &str,
) -> Vec<UnmetDep> {
    let Some((src_path, src_file, src_task)) = find_task_by_full_id(project, source_full_id) else {
        return Vec::new();
    };

    let ctx = LintContext::new(config, src_path.clone(), project);
    let mut unmet = Vec::new();

    for dep in &src_task.metadata.depends_on {
        if matches!(
            dep.kind,
            DependencyKind::Hierarchy | DependencyKind::Directory
        ) {
            continue;
        }
        let resolve_path = |rel: &str| ctx.resolve_path(Path::new(rel));
        let Ok(resolution) =
            resolve_reference(&dep.target, src_path, &src_file.id, project, resolve_path)
        else {
            // Broken references are reported by lint/check-links, not here.
            continue;
        };
        for target_full_id in resolution.full_ids() {
            if let Some((_, _, target)) = find_task_by_full_id(project, &target_full_id) {
                if !matches!(target.status, TaskStatus::Done | TaskStatus::Waived) {
                    unmet.push(UnmetDep {
                        full_id: target_full_id,
                        title: target.title.clone(),
                        status: target.status.as_str().to_string(),
                    });
                }
            }
        }
    }

    unmet
}

/// Execute the complete command
///
/// # Arguments
///
/// * `args` - Complete command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (validation error), 3 (DB error), 5 (not found)
#[allow(clippy::too_many_lines)]
pub fn execute(args: &CompleteArgs) -> Result<i32> {
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
        "Starting complete operation"
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

    // Unless --force, reparse the tree once so completion can be gated on
    // unmet @depends-on targets (GitHub issue #17).
    let project = if args.force {
        None
    } else {
        Some(load_project(&project_root))
    };

    // Process each task ID
    let mut results: Vec<CompleteResult> = Vec::new();
    let mut errors: Vec<CompleteError> = Vec::new();

    for task_id in &unique_task_ids {
        match process_task(
            task_id,
            &task_repo,
            &file_repo,
            &project_root,
            args.dry_run,
            args.cascade,
            project.as_ref(),
        ) {
            Ok(result) => results.push(result),
            Err(error) => errors.push(error),
        }
    }

    // Re-index if we made changes and not in dry-run mode
    if !args.dry_run && !results.is_empty() {
        if let Err(e) = status_mutation::reindex_project(&project_root, "task completion") {
            tracing::warn!("Failed to re-index after completion: {e}");
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
#[allow(clippy::too_many_lines)]
fn process_task(
    task_id: &str,
    task_repo: &TaskRepository,
    file_repo: &FileRepository,
    project_root: &Path,
    dry_run: bool,
    cascade: bool,
    project: Option<&(LashConfig, HashMap<PathBuf, TaskFile>)>,
) -> std::result::Result<CompleteResult, CompleteError> {
    // Try to find the task by full id or bare @id.
    let task = match crate::utils::task_target::resolve_task_target(task_repo, task_id) {
        Ok(task) => task,
        Err(TargetError::NotFound) => {
            // Try fuzzy matching
            let all_task_ids = task_repo.get_all_full_ids().unwrap_or_default();
            let suggestions = find_similar_task_ids(task_id, &all_task_ids);

            return Err(CompleteError {
                task_id: task_id.to_string(),
                code: "E_NOT_FOUND".to_string(),
                message: format!("Task not found: {task_id}"),
                suggestions: suggestions.into_iter().map(|(id, _)| id).collect(),
            });
        }
        Err(TargetError::Ambiguous(candidates)) => {
            return Err(CompleteError {
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
            return Err(CompleteError {
                task_id: task_id.to_string(),
                code: "E_DB_ERROR".to_string(),
                message: format!("Database error: {e}"),
                suggestions: vec![],
            });
        }
    };

    // Check if task can be completed
    match task.status {
        TaskStatus::Done => {
            return Err(CompleteError {
                task_id: task_id.to_string(),
                code: "E_ALREADY_COMPLETE".to_string(),
                message: format!("Task '{}' is already complete", task.full_id),
                suggestions: vec![],
            });
        }
        TaskStatus::Waived => {
            return Err(CompleteError {
                task_id: task_id.to_string(),
                code: "E_WAIVED".to_string(),
                message: format!("Task '{}' is waived (not applicable)", task.full_id),
                suggestions: vec![],
            });
        }
        TaskStatus::Open | TaskStatus::InProgress | TaskStatus::Blocked => {
            // Can be completed
        }
    }

    // Refuse completion while a resolvable @depends-on target is still open
    // (GitHub issue #17). Skipped entirely when --force was passed (project
    // is None in that case).
    if let Some((config, proj)) = project {
        let unmet = find_unmet_dependencies(config, proj, &task.full_id);
        if !unmet.is_empty() {
            let list = unmet
                .iter()
                .map(|d| format!("{} '{}' [{}]", d.full_id, d.title, d.status))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(CompleteError {
                task_id: task_id.to_string(),
                code: "E_DEP_UNMET".to_string(),
                message: format!(
                    "Task '{}' has {} unmet dependency(ies): {list}. Pass --force to override.",
                    task.full_id,
                    unmet.len()
                ),
                suggestions: vec![],
            });
        }
    }

    // Get file information
    let file = match file_repo.get_by_db_id(task.file_id) {
        Ok(Some(file)) => file,
        Ok(None) => {
            return Err(CompleteError {
                task_id: task_id.to_string(),
                code: "E_FILE_NOT_FOUND".to_string(),
                message: format!("File not found for task '{}'", task.full_id),
                suggestions: vec![],
            });
        }
        Err(e) => {
            return Err(CompleteError {
                task_id: task_id.to_string(),
                code: "E_DB_ERROR".to_string(),
                message: format!("Database error: {e}"),
                suggestions: vec![],
            });
        }
    };

    // Update the markdown file (unless dry-run)
    let update_result = if dry_run {
        // In dry-run we still want to *report* what would happen with
        // cascading children, so we compute the preview here too.
        match preview_cascade_children(project_root, &file.path, &task.title, task.status) {
            Ok(children) => CascadeOutcome {
                cascaded: 0,
                unchecked: children,
            },
            Err(_) => CascadeOutcome::default(),
        }
    } else {
        match update_markdown_task_status(
            project_root,
            &file.path,
            &task.title,
            task.status,
            TaskStatus::Done,
            cascade,
        ) {
            Ok(outcome) => outcome,
            Err(e) => {
                return Err(CompleteError {
                    task_id: task_id.to_string(),
                    code: "E_FILE_UPDATE".to_string(),
                    message: format!("Failed to update file: {e}"),
                    suggestions: vec![],
                });
            }
        }
    };

    Ok(CompleteResult {
        task_id: task.full_id.clone(),
        file_path: file.path.clone(),
        previous_status: task.status.as_str().to_string(),
        cascaded_children: update_result.cascaded,
        unchecked_children: update_result.unchecked,
    })
}

/// Output results as JSON
fn output_json_results(results: &[CompleteResult], errors: &[CompleteError]) -> Result<()> {
    let json = serde_json::json!({
        "success": errors.is_empty(),
        "completed": results,
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
#[allow(clippy::too_many_lines)]
fn output_text_results(
    results: &[CompleteResult],
    errors: &[CompleteError],
    dry_run: bool,
    theme: Option<&CliTheme>,
) {
    // Print results
    if dry_run && !results.is_empty() {
        println!("Would complete:");
    }

    for result in results {
        if let Some(t) = theme {
            if dry_run {
                println!(
                    "  {} {} ({})",
                    t.style_success("[x]"),
                    t.style_label(&result.task_id),
                    t.style_muted(&result.file_path.display().to_string())
                );
            } else {
                println!(
                    "{} {} -> {}",
                    t.style_success("[x]"),
                    t.style_label(&result.task_id),
                    t.style_muted(&result.file_path.display().to_string())
                );
            }
        } else if dry_run {
            println!("  [x] {} ({})", result.task_id, result.file_path.display());
        } else {
            println!("[x] {} ({})", result.task_id, result.file_path.display());
        }

        if result.cascaded_children > 0 {
            if let Some(t) = theme {
                println!(
                    "  {} cascaded {} plain-bullet child(ren) to [x]",
                    t.style_info("↳"),
                    result.cascaded_children
                );
            } else {
                println!(
                    "  ↳ cascaded {} plain-bullet child(ren) to [x]",
                    result.cascaded_children
                );
            }
        }

        if !result.unchecked_children.is_empty() {
            let preview: Vec<&str> = result
                .unchecked_children
                .iter()
                .take(3)
                .map(String::as_str)
                .collect();
            let more = result
                .unchecked_children
                .len()
                .saturating_sub(preview.len());
            let header = format!(
                "warning: parent completed but {} plain-bullet child(ren) remain unchecked:",
                result.unchecked_children.len()
            );
            if let Some(t) = theme {
                eprintln!("  {}", t.style_warning(&header));
            } else {
                eprintln!("  {header}");
            }
            for line in preview {
                eprintln!("    {line}");
            }
            if more > 0 {
                eprintln!("    … and {more} more");
            }
            let hint = "pass --cascade to also flip these to [x]";
            if let Some(t) = theme {
                eprintln!("  {} {}", t.style_info("hint:"), hint);
            } else {
                eprintln!("  hint: {hint}");
            }
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
                "{}: {} completed, {} failed",
                t.style_info("Summary"),
                results.len(),
                errors.len()
            );
        } else {
            println!(
                "Summary: {} completed, {} failed",
                results.len(),
                errors.len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::Task;

    fn task(id: &str, status: TaskStatus, deps: &[&str]) -> Task {
        use lash_types::dependency::{DependencyKind, DependencyRef};
        use lash_types::TaskMetadata;
        Task {
            id: id.to_string(),
            has_explicit_id: true,
            title: id.to_string(),
            status,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata: TaskMetadata {
                depends_on: deps
                    .iter()
                    .map(|d| DependencyRef::new((*d).to_string(), DependencyKind::ExplicitId))
                    .collect(),
                ..Default::default()
            },
            body: None,
            contextual_notes: Vec::new(),
        }
    }

    fn project_with(tasks: Vec<Task>) -> HashMap<PathBuf, TaskFile> {
        use lash_types::{FileMetadata, TaskTree};
        use std::time::SystemTime;
        let mut tree = TaskTree::new();
        for t in tasks {
            tree.add_task(t).unwrap();
        }
        let file = TaskFile {
            path: PathBuf::from("tasks.md"),
            title: "T".to_string(),
            id: "repro".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: tree,
            hash: "h".to_string(),
            mtime: SystemTime::now(),
        };
        let mut map = HashMap::new();
        map.insert(PathBuf::from("tasks.md"), file);
        map
    }

    // Issue #17: an open dependency must be reported as unmet.
    #[test]
    fn test_find_unmet_dependencies_flags_open_dependency() {
        let config = LashConfig::default();
        let project = project_with(vec![
            task("base-task", TaskStatus::Open, &[]),
            task("dep-task", TaskStatus::Open, &["base-task"]),
        ]);

        let unmet = find_unmet_dependencies(&config, &project, "repro#dep-task");
        assert_eq!(unmet.len(), 1);
        assert_eq!(unmet[0].full_id, "repro#base-task");
    }

    // Issue #17: a done (or waived) dependency is satisfied.
    #[test]
    fn test_find_unmet_dependencies_satisfied_when_done_or_waived() {
        let config = LashConfig::default();

        let done = project_with(vec![
            task("base-task", TaskStatus::Done, &[]),
            task("dep-task", TaskStatus::Open, &["base-task"]),
        ]);
        assert!(find_unmet_dependencies(&config, &done, "repro#dep-task").is_empty());

        let waived = project_with(vec![
            task("base-task", TaskStatus::Waived, &[]),
            task("dep-task", TaskStatus::Open, &["base-task"]),
        ]);
        assert!(find_unmet_dependencies(&config, &waived, "repro#dep-task").is_empty());
    }

    #[test]
    fn test_complete_result_serialization() {
        let result = CompleteResult {
            task_id: "test#task-1".to_string(),
            file_path: PathBuf::from("tasks.md"),
            previous_status: "open".to_string(),
            cascaded_children: 0,
            unchecked_children: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test#task-1"));
        assert!(json.contains("tasks.md"));
        assert!(json.contains("open"));
    }

    #[test]
    fn test_complete_error_serialization() {
        let error = CompleteError {
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
    fn test_complete_error_no_suggestions() {
        let error = CompleteError {
            task_id: "test#task-1".to_string(),
            code: "E_ALREADY_COMPLETE".to_string(),
            message: "Already complete".to_string(),
            suggestions: vec![],
        };
        let json = serde_json::to_string(&error).unwrap();
        // Empty suggestions should not appear in JSON
        assert!(!json.contains("suggestions"));
    }
}
