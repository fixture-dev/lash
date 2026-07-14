//! Complete command implementation
//!
//! The `lash complete` command marks one or more tasks as complete by updating
//! the checkbox in the source markdown file from `[ ]` or `[!]` to `[x]`.

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
        ) {
            Ok(result) => results.push(result),
            Err(error) => errors.push(error),
        }
    }

    // Re-index if we made changes and not in dry-run mode
    if !args.dry_run && !results.is_empty() {
        if let Err(e) = reindex_project(&project_root) {
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
        match preview_cascade_children(project_root, &file.path, &task.title) {
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

/// Outcome of updating a task line, including cascade information.
#[derive(Debug, Default, Clone)]
struct CascadeOutcome {
    /// Number of plain-bullet children that were also flipped to `[x]`.
    cascaded: usize,
    /// Plain-bullet children that remained unchecked (truncated to a
    /// reasonable size for display).
    unchecked: Vec<String>,
}

/// Scan a markdown file for plain-bullet children of the given parent task
/// without modifying the file. Returns the unchecked child labels so the
/// `--dry-run` path can warn about them.
fn preview_cascade_children(
    project_root: &Path,
    file_path: &Path,
    task_title: &str,
) -> Result<Vec<String>> {
    let full_path = project_root.join(file_path);
    let content = fs::read_to_string(&full_path)
        .with_context(|| format!("Failed to read file: {}", full_path.display()))?;
    let escaped = regex::escape(task_title);
    let parent_re = Regex::new(&format!(r"^(\s*)- \[ \] {escaped}\b"))
        .context("Failed to compile parent regex")?;
    let mut unchecked = Vec::new();
    let line_vec: Vec<&str> = content.lines().collect();
    for (idx, line) in line_vec.iter().enumerate() {
        if let Some(caps) = parent_re.captures(line) {
            let parent_indent = caps[1].len();
            collect_unchecked_plain_children(&line_vec, idx, parent_indent, &mut unchecked);
            break;
        }
    }
    Ok(unchecked)
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

/// Update task status in the markdown file.
///
/// Finds the task line by matching the title and old status, then updates
/// the checkbox character to reflect the new status. When `cascade` is set
/// and the new status is `Done`, also flips unchecked plain-bullet children
/// (children without their own `@id`) of the parent task to `[x]`.
/// Children with an `@id` are independent tasks and are never touched here.
fn update_markdown_task_status(
    project_root: &Path,
    file_path: &Path,
    task_title: &str,
    old_status: TaskStatus,
    new_status: TaskStatus,
    cascade: bool,
) -> Result<CascadeOutcome> {
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

    let lines: Vec<String> = content.lines().map(str::to_string).collect();
    let mut updated_lines = lines.clone();
    let mut parent_idx: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        if re.is_match(line) {
            updated_lines[i] = re
                .replace(line, format!("${{1}}{new_char}${{2}}"))
                .to_string();
            parent_idx = Some(i);
            break;
        }
    }

    let Some(parent_idx) = parent_idx else {
        anyhow::bail!("Could not find task '{task_title}' in file");
    };

    // Cascade only applies when we are flipping a parent to Done.
    let parent_indent = leading_space_count(&lines[parent_idx]);
    let mut cascaded = 0usize;
    let mut unchecked = Vec::new();

    if new_status == TaskStatus::Done {
        let plain_children = find_plain_child_lines(&lines, parent_idx, parent_indent);
        for child_idx in plain_children {
            let child_line = &lines[child_idx];
            if cascade {
                if let Some(new_line) = flip_open_to_done(child_line) {
                    updated_lines[child_idx] = new_line;
                    cascaded += 1;
                }
            } else if child_line.trim_start().strip_prefix("- [ ]").is_some() {
                unchecked.push(child_line.trim().to_string());
            }
        }
    }

    let updated_content = updated_lines.join("\n");
    let final_content = if content.ends_with('\n') && !updated_content.ends_with('\n') {
        format!("{updated_content}\n")
    } else {
        updated_content
    };

    fs::write(&full_path, final_content)
        .with_context(|| format!("Failed to write file: {}", full_path.display()))?;

    Ok(CascadeOutcome {
        cascaded,
        unchecked,
    })
}

/// Count the leading whitespace characters in a line.
fn leading_space_count(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

/// Collect immediate plain-bullet child line indexes under the parent.
///
/// A "plain-bullet child" is a `- [ ]` (or `- [!]`, `- [>]`) checkbox line
/// indented more deeply than the parent, that does NOT carry its own
/// `@id:` annotation in the indented block following it. The walk stops at
/// the first line whose indent drops back to (or below) the parent's
/// indent, since that ends the parent's scope.
fn find_plain_child_lines(lines: &[String], parent_idx: usize, parent_indent: usize) -> Vec<usize> {
    let mut children = Vec::new();
    let mut i = parent_idx + 1;
    while i < lines.len() {
        let line = &lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        let indent = leading_space_count(line);
        if indent <= parent_indent {
            // Out of the parent's scope.
            break;
        }
        if trimmed.starts_with("- [") {
            // Decide whether this child has its own @id by peeking at the
            // following indented annotation lines (deeper than this child).
            let child_indent = indent;
            if !child_has_id_annotation(lines, i, child_indent) {
                children.push(i);
            }
        }
        i += 1;
    }
    children
}

/// Walk lines following a checkbox child to see if it has `@id:` in its
/// own annotation block (lines indented more deeply than the child line).
fn child_has_id_annotation(lines: &[String], child_idx: usize, child_indent: usize) -> bool {
    let mut j = child_idx + 1;
    while j < lines.len() {
        let line = &lines[j];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            j += 1;
            continue;
        }
        let indent = leading_space_count(line);
        // Annotation lines belong to the child only if they are *more*
        // indented than the child itself and are not a new checkbox.
        if indent <= child_indent {
            break;
        }
        if trimmed.starts_with("- [") {
            // A nested checkbox under the child — annotations would have
            // come before this, so we can stop.
            break;
        }
        if trimmed.starts_with("@id:") {
            return true;
        }
        j += 1;
    }
    false
}

/// Flip a `- [ ]` (or `[!]`, `[>]`) checkbox line to `- [x]`. Returns the
/// new line, or `None` if the line isn't a flippable open checkbox.
fn flip_open_to_done(line: &str) -> Option<String> {
    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let rest = &line[indent.len()..];
    for prefix in ["- [ ]", "- [!]", "- [>]"] {
        if let Some(remainder) = rest.strip_prefix(prefix) {
            return Some(format!("{indent}- [x]{remainder}"));
        }
    }
    None
}

/// Same scan as the in-place cascade, but read-only — used by `--dry-run`.
fn collect_unchecked_plain_children(
    lines: &[&str],
    parent_idx: usize,
    parent_indent: usize,
    out: &mut Vec<String>,
) {
    let owned: Vec<String> = lines.iter().map(|s| (*s).to_string()).collect();
    for idx in find_plain_child_lines(&owned, parent_idx, parent_indent) {
        let line = &owned[idx];
        if line.trim_start().starts_with("- [ ]") {
            out.push(line.trim().to_string());
        }
    }
}

/// Re-index the project after completing tasks
fn reindex_project(project_root: &Path) -> Result<()> {
    let db_path = project_root.join(".lash").join("lash.db");
    let conn = open_database(&db_path).context("Failed to open database for re-indexing")?;

    let config = LashConfig::from_root(project_root).unwrap_or_default();
    let indexer_config = IndexerConfig::new(project_root.to_path_buf()).with_incremental(true);
    let mut indexer = Indexer::new(&conn, indexer_config, &config);

    indexer
        .index_project()
        .context("Failed to re-index after task completion")?;

    Ok(())
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

    #[test]
    fn test_status_checkbox_char() {
        assert_eq!(status_checkbox_char(TaskStatus::Open), ' ');
        assert_eq!(status_checkbox_char(TaskStatus::Done), 'x');
        assert_eq!(status_checkbox_char(TaskStatus::Waived), '-');
        assert_eq!(status_checkbox_char(TaskStatus::Blocked), '!');
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
    fn test_flip_open_to_done() {
        assert_eq!(
            flip_open_to_done("  - [ ] Sub task"),
            Some("  - [x] Sub task".to_string())
        );
        assert_eq!(
            flip_open_to_done("    - [!] Blocked"),
            Some("    - [x] Blocked".to_string())
        );
        assert_eq!(
            flip_open_to_done("    - [>] In progress"),
            Some("    - [x] In progress".to_string())
        );
        // Already done — not flippable
        assert_eq!(flip_open_to_done("- [x] Done"), None);
        // Not a checkbox
        assert_eq!(flip_open_to_done("regular line"), None);
    }

    #[test]
    fn test_find_plain_child_lines_skips_id_tagged_children() {
        // Two children: one with @id (independent task, should be skipped),
        // one plain-bullet (should be returned).
        let lines: Vec<String> = vec![
            "- [ ] Parent",
            "  @id: parent-1",
            "  - [ ] Tracked child",
            "    @id: tracked-child",
            "  - [ ] Plain child",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        let plain = find_plain_child_lines(&lines, 0, 0);
        assert_eq!(plain, vec![4]);
    }

    #[test]
    fn test_find_plain_child_lines_stops_at_dedent() {
        // The loop must stop when indent returns to parent level.
        let lines: Vec<String> = vec![
            "- [ ] Parent",
            "  - [ ] Plain child 1",
            "- [ ] Sibling parent",
            "  - [ ] Sibling's child",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        let plain = find_plain_child_lines(&lines, 0, 0);
        // Only the first parent's plain child should be returned.
        assert_eq!(plain, vec![1]);
    }

    #[test]
    fn test_update_markdown_cascade_flips_plain_children() {
        // End-to-end exercise of cascade=true on a parent with mixed children.
        let temp = tempfile::TempDir::new().unwrap();
        let path = PathBuf::from("tasks.md");
        let full = temp.path().join(&path);
        let content = "# Tasks\n\
                       \n\
                       ## Tasks\n\
                       \n\
                       - [ ] Parent task\n  \
                       @id: parent\n  \
                       - [ ] Plain step one\n  \
                       - [ ] Plain step two\n  \
                       - [ ] Tracked child\n    \
                       @id: tracked\n";
        std::fs::write(&full, content).unwrap();

        let outcome = update_markdown_task_status(
            temp.path(),
            &path,
            "Parent task",
            TaskStatus::Open,
            TaskStatus::Done,
            true, // cascade
        )
        .unwrap();

        let updated = std::fs::read_to_string(&full).unwrap();
        assert!(updated.contains("- [x] Parent task"));
        assert!(updated.contains("- [x] Plain step one"));
        assert!(updated.contains("- [x] Plain step two"));
        // Tracked child must NOT be flipped — it has its own @id.
        assert!(updated.contains("- [ ] Tracked child"));

        assert_eq!(outcome.cascaded, 2);
        assert!(outcome.unchecked.is_empty());
    }

    #[test]
    fn test_update_markdown_warns_when_cascade_disabled() {
        // Without --cascade we leave the children alone but report them.
        let temp = tempfile::TempDir::new().unwrap();
        let path = PathBuf::from("tasks.md");
        let full = temp.path().join(&path);
        let content = "# Tasks\n\
                       \n\
                       ## Tasks\n\
                       \n\
                       - [ ] Parent task\n  \
                       @id: parent\n  \
                       - [ ] Plain step one\n  \
                       - [ ] Plain step two\n";
        std::fs::write(&full, content).unwrap();

        let outcome = update_markdown_task_status(
            temp.path(),
            &path,
            "Parent task",
            TaskStatus::Open,
            TaskStatus::Done,
            false, // cascade off
        )
        .unwrap();

        let updated = std::fs::read_to_string(&full).unwrap();
        assert!(updated.contains("- [x] Parent task"));
        assert!(updated.contains("- [ ] Plain step one"));
        assert!(updated.contains("- [ ] Plain step two"));

        assert_eq!(outcome.cascaded, 0);
        assert_eq!(outcome.unchecked.len(), 2);
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
