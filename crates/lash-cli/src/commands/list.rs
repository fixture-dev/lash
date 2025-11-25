//! List command implementation
//!
//! The `lash list` command queries and filters tasks from the `SQLite` database.

use anyhow::{Context, Result};
use lash_cli::theme::CliTheme;
use lash_db::repository::tasks::{TaskFilter, TaskRecord};
use lash_db::{open_database, TaskRepository};
use lash_types::TaskStatus;
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

/// Output format for list command
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
    JsonPretty,
}

/// Arguments for the list command
#[derive(Debug, Clone)]
pub struct ListArgs {
    /// Filter by label (can be specified multiple times)
    pub labels: Vec<String>,
    /// Filter by status
    pub status: Option<TaskStatus>,
    /// Filter by path prefix
    pub path: Option<PathBuf>,
    /// Only show blocked tasks
    pub blocked: bool,
    /// Filter by owner
    pub owner: Option<String>,
    /// Output format
    pub format: OutputFormat,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Optional CLI theme for styling
    pub theme: Option<CliTheme>,
}

/// Execute the list command
///
/// # Arguments
///
/// * `args` - List command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (general error), 3 (DB error)
pub fn execute(args: ListArgs) -> Result<i32> {
    // Determine project root
    let project_root = if let Some(root) = args.project_root {
        root
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        labels = ?args.labels,
        status = ?args.status,
        "Starting list operation"
    );

    // Determine database path
    let db_path = get_database_path(&project_root);

    // Check if database exists
    if !db_path.exists() {
        if args.format == OutputFormat::Text {
            eprintln!("Database not found at {}", db_path.display());
            eprintln!("Run `lash index` to create the database.");
        } else {
            output_json_no_db()?;
        }
        return Ok(3); // Exit code 3 for DB error
    }

    // Open database
    let conn = open_database(&db_path).context("Failed to open database")?;

    // Create task repository
    let task_repo = TaskRepository::new(&conn);

    // Build filter
    let filter = TaskFilter {
        status: args.status,
        labels: args.labels.clone(),
        owner: args.owner.clone(),
        file_path: args.path.as_ref().map(|p| p.display().to_string()),
        blocked: if args.blocked { Some(true) } else { None },
    };

    // Execute query
    let tasks = task_repo.find(&filter).context("Failed to query tasks")?;

    tracing::debug!(task_count = tasks.len(), "Retrieved tasks");

    // Output results
    match args.format {
        OutputFormat::Json => output_json(&tasks)?,
        OutputFormat::JsonPretty => output_json_pretty(&tasks)?,
        OutputFormat::Text => output_text(&tasks, args.theme.as_ref()),
    }

    Ok(0)
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/lash.db")
}

/// Output JSON when database doesn't exist
fn output_json_no_db() -> Result<()> {
    use serde_json::json;

    let output = json!({
        "error": "Database not found",
        "suggestion": "Run `lash index` to create the database",
        "tasks": []
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output tasks as compact JSON
fn output_json(tasks: &[TaskRecord]) -> Result<()> {
    use serde_json::json;

    let output = json!({
        "count": tasks.len(),
        "tasks": tasks
    });

    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

/// Output tasks as pretty-printed JSON
fn output_json_pretty(tasks: &[TaskRecord]) -> Result<()> {
    use serde_json::json;

    let output = json!({
        "count": tasks.len(),
        "tasks": tasks
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output tasks as human-readable text
fn output_text(tasks: &[TaskRecord], theme: Option<&CliTheme>) {
    if tasks.is_empty() {
        let msg = "No tasks match your filters";
        if let Some(theme) = theme {
            println!("{}", theme.style_warning(msg));
        } else {
            println!("{msg}");
        }
        println!();
        println!("Try:");
        println!("  - Removing filters to broaden the search");
        println!("  - Running `lash index` to ensure the database is up to date");
        return;
    }

    // Calculate column widths
    let max_id_len = tasks
        .iter()
        .map(|t| t.full_id.len())
        .max()
        .unwrap_or(10)
        .max(10);

    let max_title_len = tasks
        .iter()
        .map(|t| t.title.len())
        .max()
        .unwrap_or(40)
        .min(60); // Cap at 60 chars

    // Print header
    let header_id = "ID";
    let header_status = "STATUS";
    let header_title = "TITLE";
    let header_labels = "LABELS";

    if let Some(_theme) = theme {
        use owo_colors::OwoColorize;
        println!(
            "{:<width_id$}  {:<width_status$}  {:<width_title$}  {}",
            header_id.bold(),
            header_status.bold(),
            header_title.bold(),
            header_labels.bold(),
            width_id = max_id_len,
            width_status = 7,
            width_title = max_title_len,
        );
    } else {
        println!(
            "{:<width_id$}  {:<width_status$}  {:<width_title$}  {header_labels}",
            header_id,
            header_status,
            header_title,
            width_id = max_id_len,
            width_status = 7,
            width_title = max_title_len,
        );
    }

    // Print separator
    println!("{}", "-".repeat(max_id_len + 7 + max_title_len + 20));

    // Print each task
    for task in tasks {
        let status_display = format_status(task.status, theme);
        let title_display = truncate_string(&task.title, max_title_len);
        let labels_display = format_labels(&task.metadata.labels, theme);

        // Format the ID with theme colors if available
        let id_display = if let Some(theme) = theme {
            theme.style_info(&task.full_id)
        } else {
            task.full_id.clone()
        };

        // Note: When using colored output, we need to account for invisible ANSI codes
        // in the field width. However, owo_colors handles this for us in most cases.
        // For precise alignment with colors, we'd need to calculate visual width separately.
        println!(
            "{:<width_id$}  {:<width_status$}  {:<width_title$}  {}",
            id_display,
            status_display,
            title_display,
            labels_display,
            width_id = max_id_len,
            width_status = 7,
            width_title = max_title_len,
        );
    }

    // Print summary
    println!();
    let summary = format!("Total: {} task(s)", tasks.len());
    if let Some(_theme) = theme {
        use owo_colors::OwoColorize;
        println!("{}", summary.bold());
    } else {
        println!("{summary}");
    }
}

/// Format task status with theme-aware colors
fn format_status(status: TaskStatus, theme: Option<&CliTheme>) -> String {
    let status_str = match status {
        TaskStatus::Open => "open",
        TaskStatus::Done => "done",
        TaskStatus::Waived => "waived",
        TaskStatus::Blocked => "blocked",
    };

    if let Some(theme) = theme {
        theme.style_task_status(status_str, status)
    } else {
        status_str.to_string()
    }
}

/// Format labels for display with theme-aware colors
fn format_labels(labels: &[String], theme: Option<&CliTheme>) -> String {
    if labels.is_empty() {
        return String::new();
    }

    let labels_str = labels.join(", ");

    if let Some(theme) = theme {
        theme.style_muted(&labels_str)
    } else {
        labels_str
    }
}

/// Truncate a string to a maximum length, adding ellipsis if needed
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(
            truncate_string("this is a very long string", 10),
            "this is..."
        );
        assert_eq!(truncate_string("exact", 5), "exact");
    }

    #[test]
    fn test_format_status() {
        assert_eq!(format_status(TaskStatus::Open, None), "open");
        assert_eq!(format_status(TaskStatus::Done, None), "done");
        assert_eq!(format_status(TaskStatus::Waived, None), "waived");
        assert_eq!(format_status(TaskStatus::Blocked, None), "blocked");
    }

    #[test]
    fn test_format_labels() {
        assert_eq!(format_labels(&[], None), "");
        assert_eq!(format_labels(&["backend".to_string()], None), "backend");
        assert_eq!(
            format_labels(&["backend".to_string(), "api".to_string()], None),
            "backend, api"
        );
    }
}
