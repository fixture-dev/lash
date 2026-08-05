//! Waive command implementation
//!
//! The `lash waive` command marks one or more tasks as waived (not
//! applicable) by updating the checkbox in the source markdown file to
//! `[-]`. It mirrors `lash complete` (see `commands::complete` and the
//! shared machinery in `commands::status_mutation`), but never gates on
//! `@depends-on` — abandoning a task doesn't require its dependencies to be
//! resolved first.

use anyhow::{Context, Result};
use lash_cli::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash_cli::formatter::{OutputFormat, Verbosity};
use lash_cli::theme::CliTheme;
use lash_db::{open_database, FileRepository, TaskRepository};
use lash_types::error::LashError;
use lash_types::TaskStatus;
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::status_mutation::{
    self, find_similar_task_ids, preview_cascade_children, update_markdown_task_status,
    CascadeOutcome,
};
use crate::utils::file_discovery::find_project_root;
use crate::utils::task_target::TargetError;

/// Arguments for the waive command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct WaiveArgs {
    /// Task ID(s) to mark as waived
    pub task_ids: Vec<String>,
    /// Preview what would be changed without modifying files
    pub dry_run: bool,
    /// Also mark unchecked plain-bullet children (without their own @id)
    /// as waived. Mirrors `complete --cascade`.
    pub cascade: bool,
    /// One-line rationale recorded as a contextual note under the task.
    pub reason: Option<String>,
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Verbosity level for output
    pub verbosity: Verbosity,
}

/// Result of waiving a single task
#[derive(Debug, Clone, Serialize)]
pub struct WaiveResult {
    /// Task full ID
    pub task_id: String,
    /// File path where task was updated
    pub file_path: PathBuf,
    /// Previous status before waiving
    pub previous_status: String,
    /// Number of plain-bullet children also marked waived by --cascade
    #[serde(skip_serializing_if = "is_zero")]
    pub cascaded_children: usize,
    /// Plain-bullet children that were left unchecked (because --cascade
    /// was not set). Empty when there were none or when --cascade ran.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unchecked_children: Vec<String>,
    /// Rationale recorded as a contextual note, if `--reason` was passed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// Error for a single task that could not be waived
#[derive(Debug, Clone, Serialize)]
pub struct WaiveError {
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

/// Execute the waive command
///
/// # Arguments
///
/// * `args` - Waive command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (validation error), 3 (DB error), 5 (not found)
#[allow(clippy::too_many_lines)]
pub fn execute(args: &WaiveArgs) -> Result<i32> {
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
        "Starting waive operation"
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
    let mut results: Vec<WaiveResult> = Vec::new();
    let mut errors: Vec<WaiveError> = Vec::new();

    for task_id in &unique_task_ids {
        match process_task(
            task_id,
            &task_repo,
            &file_repo,
            &project_root,
            args.dry_run,
            args.cascade,
            args.reason.as_deref(),
        ) {
            Ok(result) => results.push(result),
            Err(error) => errors.push(error),
        }
    }

    // Re-index if we made changes and not in dry-run mode
    if !args.dry_run && !results.is_empty() {
        if let Err(e) = status_mutation::reindex_project(&project_root, "waiving task") {
            tracing::warn!("Failed to re-index after waiving: {e}");
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
    reason: Option<&str>,
) -> std::result::Result<WaiveResult, WaiveError> {
    // Try to find the task by full id or bare @id.
    let task = match crate::utils::task_target::resolve_task_target(task_repo, task_id) {
        Ok(task) => task,
        Err(TargetError::NotFound) => {
            // Try fuzzy matching
            let all_task_ids = task_repo.get_all_full_ids().unwrap_or_default();
            let suggestions = find_similar_task_ids(task_id, &all_task_ids);

            return Err(WaiveError {
                task_id: task_id.to_string(),
                code: "E_NOT_FOUND".to_string(),
                message: format!("Task not found: {task_id}"),
                suggestions: suggestions.into_iter().map(|(id, _)| id).collect(),
            });
        }
        Err(TargetError::Ambiguous(candidates)) => {
            return Err(WaiveError {
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
            return Err(WaiveError {
                task_id: task_id.to_string(),
                code: "E_DB_ERROR".to_string(),
                message: format!("Database error: {e}"),
                suggestions: vec![],
            });
        }
    };

    // Check if task can be waived. Unlike `complete`, waiving never gates
    // on unmet `@depends-on` targets — abandoning a task doesn't require
    // its dependencies to be resolved.
    match task.status {
        TaskStatus::Waived => {
            return Err(WaiveError {
                task_id: task_id.to_string(),
                code: "E_ALREADY_WAIVED".to_string(),
                message: format!("Task '{}' is already waived", task.full_id),
                suggestions: vec![],
            });
        }
        TaskStatus::Done => {
            return Err(WaiveError {
                task_id: task_id.to_string(),
                code: "E_DONE".to_string(),
                message: format!(
                    "Task '{}' is already complete; completed work shouldn't be silently \
                     waived. Hand-edit the checkbox to `[-]` and run `lash index` if this is \
                     truly intended.",
                    task.full_id
                ),
                suggestions: vec![],
            });
        }
        TaskStatus::Open | TaskStatus::InProgress | TaskStatus::Blocked => {
            // Can be waived
        }
    }

    // Get file information
    let file = match file_repo.get_by_db_id(task.file_id) {
        Ok(Some(file)) => file,
        Ok(None) => {
            return Err(WaiveError {
                task_id: task_id.to_string(),
                code: "E_FILE_NOT_FOUND".to_string(),
                message: format!("File not found for task '{}'", task.full_id),
                suggestions: vec![],
            });
        }
        Err(e) => {
            return Err(WaiveError {
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
            TaskStatus::Waived,
            cascade,
        ) {
            Ok(outcome) => outcome,
            Err(e) => {
                return Err(WaiveError {
                    task_id: task_id.to_string(),
                    code: "E_FILE_UPDATE".to_string(),
                    message: format!("Failed to update file: {e}"),
                    suggestions: vec![],
                });
            }
        }
    };

    // Record the rationale as a contextual note (unless dry-run).
    if !dry_run {
        if let Some(text) = reason {
            if let Err(e) = insert_reason_note(project_root, &file.path, &task.title, text) {
                return Err(WaiveError {
                    task_id: task_id.to_string(),
                    code: "E_FILE_UPDATE".to_string(),
                    message: format!("Failed to record --reason note: {e}"),
                    suggestions: vec![],
                });
            }
        }
    }

    Ok(WaiveResult {
        task_id: task.full_id.clone(),
        file_path: file.path.clone(),
        previous_status: task.status.as_str().to_string(),
        cascaded_children: update_result.cascaded,
        unchecked_children: update_result.unchecked,
        reason: reason.map(str::to_string),
    })
}

/// Record `--reason` as a contextual note under the now-waived task.
///
/// Contextual notes are plain bullet lines (no checkbox, no `@` marker)
/// indented exactly 2 spaces deeper than their parent task — see
/// `docs/design-doc.md` "Contextual Notes". The note is inserted after any
/// existing `@...` annotation lines belonging to the task (so `@id`,
/// `@depends-on`, etc. keep parsing as an unbroken annotation block — a
/// plain-bullet line interrupting them would knock the rest into
/// "orphaned annotation" handling) and before any child checkbox lines, so
/// it round-trips through the parser and passes `lash lint` unchanged.
///
/// # Errors
///
/// Returns an error if the file cannot be read/written or the (now-waived)
/// task line cannot be found.
fn insert_reason_note(
    project_root: &Path,
    file_path: &Path,
    task_title: &str,
    reason: &str,
) -> Result<()> {
    let full_path = project_root.join(file_path);
    let content = fs::read_to_string(&full_path)
        .with_context(|| format!("Failed to read file: {}", full_path.display()))?;

    let waived_char = status_mutation::status_checkbox_char(TaskStatus::Waived);
    let escaped_title = regex::escape(task_title);
    let pattern = format!(r"^(\s*)- \[{waived_char}\] {escaped_title}\b");
    let re = Regex::new(&pattern).context("Failed to compile regex")?;

    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let Some(task_idx) = lines.iter().position(|l| re.is_match(l)) else {
        anyhow::bail!("Could not find task '{task_title}' in file to attach --reason note");
    };

    let task_indent = status_mutation::leading_space_count(&lines[task_idx]);
    let note_indent = " ".repeat(task_indent + 2);

    // Skip past any existing `@...` annotation lines directly under the
    // task so the note lands after them, not between them.
    let mut insert_at = task_idx + 1;
    while insert_at < lines.len() {
        let indent = status_mutation::leading_space_count(&lines[insert_at]);
        let trimmed = lines[insert_at].trim_start();
        if indent > task_indent && trimmed.starts_with('@') {
            insert_at += 1;
        } else {
            break;
        }
    }

    lines.insert(insert_at, format!("{note_indent}- {reason}"));

    let updated_content = lines.join("\n");
    let final_content = if content.ends_with('\n') && !updated_content.ends_with('\n') {
        format!("{updated_content}\n")
    } else {
        updated_content
    };

    fs::write(&full_path, final_content)
        .with_context(|| format!("Failed to write file: {}", full_path.display()))?;

    Ok(())
}

/// Output results as JSON
fn output_json_results(results: &[WaiveResult], errors: &[WaiveError]) -> Result<()> {
    let json = serde_json::json!({
        "success": errors.is_empty(),
        "waived": results,
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
    results: &[WaiveResult],
    errors: &[WaiveError],
    dry_run: bool,
    theme: Option<&CliTheme>,
) {
    // Print results
    if dry_run && !results.is_empty() {
        println!("Would waive:");
    }

    for result in results {
        if let Some(t) = theme {
            if dry_run {
                println!(
                    "  {} {} ({})",
                    t.style_warning("[-]"),
                    t.style_label(&result.task_id),
                    t.style_muted(&result.file_path.display().to_string())
                );
            } else {
                println!(
                    "{} {} -> {}",
                    t.style_warning("[-]"),
                    t.style_label(&result.task_id),
                    t.style_muted(&result.file_path.display().to_string())
                );
            }
        } else if dry_run {
            println!("  [-] {} ({})", result.task_id, result.file_path.display());
        } else {
            println!("[-] {} ({})", result.task_id, result.file_path.display());
        }

        if let Some(ref reason) = result.reason {
            if let Some(t) = theme {
                println!("  {} {reason}", t.style_info("reason:"));
            } else {
                println!("  reason: {reason}");
            }
        }

        if result.cascaded_children > 0 {
            if let Some(t) = theme {
                println!(
                    "  {} cascaded {} plain-bullet child(ren) to [-]",
                    t.style_info("↳"),
                    result.cascaded_children
                );
            } else {
                println!(
                    "  ↳ cascaded {} plain-bullet child(ren) to [-]",
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
                "warning: parent waived but {} plain-bullet child(ren) remain unchecked:",
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
            let hint = "pass --cascade to also flip these to [-]";
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
                "{}: {} waived, {} failed",
                t.style_info("Summary"),
                results.len(),
                errors.len()
            );
        } else {
            println!("Summary: {} waived, {} failed", results.len(), errors.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waive_result_serialization() {
        let result = WaiveResult {
            task_id: "test#task-1".to_string(),
            file_path: PathBuf::from("tasks.md"),
            previous_status: "open".to_string(),
            cascaded_children: 0,
            unchecked_children: vec![],
            reason: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test#task-1"));
        assert!(json.contains("tasks.md"));
        assert!(json.contains("open"));
        // Absent reason should not appear in JSON
        assert!(!json.contains("reason"));
    }

    #[test]
    fn test_waive_result_serialization_with_reason() {
        let result = WaiveResult {
            task_id: "test#task-1".to_string(),
            file_path: PathBuf::from("tasks.md"),
            previous_status: "open".to_string(),
            cascaded_children: 0,
            unchecked_children: vec![],
            reason: Some("Superseded by task-2".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Superseded by task-2"));
    }

    #[test]
    fn test_waive_error_serialization() {
        let error = WaiveError {
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
    fn test_waive_error_no_suggestions() {
        let error = WaiveError {
            task_id: "test#task-1".to_string(),
            code: "E_ALREADY_WAIVED".to_string(),
            message: "Already waived".to_string(),
            suggestions: vec![],
        };
        let json = serde_json::to_string(&error).unwrap();
        // Empty suggestions should not appear in JSON
        assert!(!json.contains("suggestions"));
    }

    #[test]
    fn test_insert_reason_note_after_annotations() {
        // The note must land after @id/@depends-on so they keep parsing as
        // one contiguous annotation block (see doc comment on
        // insert_reason_note for why order matters).
        let temp = tempfile::TempDir::new().unwrap();
        let path = PathBuf::from("tasks.md");
        let full = temp.path().join(&path);
        let content = "# Tasks\n\
                       \n\
                       ## Tasks\n\
                       \n\
                       - [-] Waived task\n  \
                       @id: task-1\n  \
                       @depends-on: other-task\n";
        std::fs::write(&full, content).unwrap();

        insert_reason_note(temp.path(), &path, "Waived task", "No longer needed").unwrap();

        let updated = std::fs::read_to_string(&full).unwrap();
        let lines: Vec<&str> = updated.lines().collect();
        let id_idx = lines
            .iter()
            .position(|l| l.contains("@id: task-1"))
            .unwrap();
        let dep_idx = lines
            .iter()
            .position(|l| l.contains("@depends-on: other-task"))
            .unwrap();
        let note_idx = lines
            .iter()
            .position(|l| l.contains("No longer needed"))
            .unwrap();
        assert!(id_idx < dep_idx);
        assert!(dep_idx < note_idx, "note must come after annotations");
        // Note must be a plain bullet (no checkbox, no @ marker) indented
        // 2 spaces deeper than the task.
        assert_eq!(lines[note_idx], "  - No longer needed");
    }

    #[test]
    fn test_insert_reason_note_no_annotations() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = PathBuf::from("tasks.md");
        let full = temp.path().join(&path);
        let content = "- [-] Waived task\n";
        std::fs::write(&full, content).unwrap();

        insert_reason_note(temp.path(), &path, "Waived task", "Not applicable").unwrap();

        let updated = std::fs::read_to_string(&full).unwrap();
        assert_eq!(updated, "- [-] Waived task\n  - Not applicable\n");
    }
}
