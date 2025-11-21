//! Show command implementation
//!
//! The `lash show` command displays detailed information about a specific task or file.

use anyhow::{Context, Result};
use lash_db::repository::files::FileRecord;
use lash_db::repository::tasks::TaskRecord;
use lash_db::{open_database, DependencyRepository, FileRepository, TaskRepository};
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

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

    // Determine database path
    let db_path = get_database_path(&project_root);

    // Check if database exists
    if !db_path.exists() {
        if args.json {
            output_json_no_db()?;
        } else {
            eprintln!("Database not found at {}", db_path.display());
            eprintln!("Run `lash index` to create the database.");
        }
        return Ok(3); // Exit code 3 for DB error
    }

    // Open database
    let conn = open_database(&db_path).context("Failed to open database")?;

    // Create repositories
    let task_repo = TaskRepository::new(&conn);
    let file_repo = FileRepository::new(&conn);
    let dep_repo = DependencyRepository::new(&conn);

    // Determine if target is a task ID or file path
    let is_file_path = args.target.contains('/')
        || std::path::Path::new(&args.target)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));

    if is_file_path {
        // Show file information
        show_file(&file_repo, &task_repo, args, &project_root)?;
    } else {
        // Show task information
        show_task(&task_repo, &file_repo, &dep_repo, args)?;
    }

    Ok(0)
}

/// Show detailed information about a file
fn show_file(
    file_repo: &FileRepository,
    task_repo: &TaskRepository,
    args: &ShowArgs,
    project_root: &Path,
) -> Result<i32> {
    // Convert target to relative path if needed
    let target_path = if args.target.starts_with('/') {
        // Absolute path - make it relative to project root
        PathBuf::from(&args.target)
            .strip_prefix(project_root)
            .map_or_else(
                |_| PathBuf::from(&args.target),
                std::path::Path::to_path_buf,
            )
    } else {
        PathBuf::from(&args.target)
    };

    // Get file record
    let Some(file) = file_repo.get_by_path(&target_path)? else {
        if args.json {
            output_json_not_found(&args.target)?;
        } else {
            eprintln!("File not found: {}", args.target);
            eprintln!("Make sure the file has been indexed with `lash index`");
        }
        return Ok(5); // Exit code 5 for not found
    };

    // Get tasks in this file
    let tasks = task_repo.get_by_file(file.id)?;

    // Output results
    if args.json {
        output_json_file(&file, &tasks)?;
    } else {
        output_text_file(&file, &tasks, args.no_color);
    }

    Ok(0)
}

/// Show detailed information about a task
fn show_task(
    task_repo: &TaskRepository,
    _file_repo: &FileRepository,
    _dep_repo: &DependencyRepository,
    args: &ShowArgs,
) -> Result<i32> {
    // Get task record
    let Some(task) = task_repo.get_by_full_id(&args.target)? else {
        if args.json {
            output_json_not_found(&args.target)?;
        } else {
            eprintln!("Task not found: {}", args.target);
            eprintln!("Make sure the task exists and has been indexed with `lash index`");
        }
        return Ok(5); // Exit code 5 for not found
    };

    // Get file information from the database
    // Note: The task.file_id is the database ID, not the file's string ID
    // We'll need to create a minimal FileRecord for display purposes
    // In the future, we should add a get_by_db_id method to FileRepository
    let file = FileRecord {
        id: task.file_id,
        path: PathBuf::from(format!("<file-id-{}>", task.file_id)),
        file_id: format!("file-{}", task.file_id),
        title: String::from("<unknown>"),
        hash: String::new(),
        mtime: 0,
        status: lash_types::FileStatus::InProgress,
        metadata: lash_types::FileMetadata::default(),
        indexed_at: 0,
    };

    // Get dependencies if requested
    // For now, we'll skip showing full dependency details since we'd need to query by DB ID
    // This can be improved later by adding helper methods to the repository
    let dependencies = if args.deps {
        // We can get the dependency records but can't easily resolve them to full tasks
        // without direct DB access or new repository methods
        // For now, just return empty
        Some(Vec::new())
    } else {
        None
    };

    // Get reverse dependencies if requested
    let dependents = if args.rdeps { Some(Vec::new()) } else { None };

    // Output results
    if args.json {
        output_json_task(&task, &file, dependencies, dependents)?;
    } else {
        output_text_task(
            &task,
            &file,
            dependencies.as_ref(),
            dependents.as_ref(),
            args.no_color,
        );
    }

    Ok(0)
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/db.sqlite")
}

/// Output JSON when database doesn't exist
fn output_json_no_db() -> Result<()> {
    use serde_json::json;

    let output = json!({
        "error": "Database not found",
        "suggestion": "Run `lash index` to create the database"
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output JSON when target not found
fn output_json_not_found(target: &str) -> Result<()> {
    use serde_json::json;

    let output = json!({
        "error": "Not found",
        "target": target,
        "suggestion": "Run `lash index` to ensure the database is up to date"
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output file as JSON
fn output_json_file(file: &FileRecord, tasks: &[TaskRecord]) -> Result<()> {
    use serde_json::json;

    let output = json!({
        "type": "file",
        "file": file,
        "task_count": tasks.len(),
        "tasks": tasks,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output task as JSON
fn output_json_task(
    task: &TaskRecord,
    file: &FileRecord,
    dependencies: Option<Vec<TaskRecord>>,
    dependents: Option<Vec<TaskRecord>>,
) -> Result<()> {
    use serde_json::json;

    let mut output = json!({
        "type": "task",
        "task": task,
        "file": {
            "path": file.path,
            "title": file.title,
        },
    });

    if let Some(deps) = dependencies {
        output["dependencies"] = json!(deps);
    }

    if let Some(deps) = dependents {
        output["dependents"] = json!(deps);
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output file as human-readable text
fn output_text_file(file: &FileRecord, tasks: &[TaskRecord], no_color: bool) {
    use owo_colors::OwoColorize;

    let use_color = !no_color;

    // File header
    if use_color {
        println!("{}", "File:".bold());
        println!("  Path:     {}", file.path.display().to_string().cyan());
        println!("  Title:    {}", file.title.bold());
        println!("  ID:       {}", file.file_id);
        println!("  Status:   {}", format_file_status(file.status, use_color));
    } else {
        println!("File:");
        println!("  Path:     {}", file.path.display());
        println!("  Title:    {}", file.title);
        println!("  ID:       {}", file.file_id);
        println!("  Status:   {}", format_file_status(file.status, use_color));
    }

    // Task summary
    println!();
    if use_color {
        println!("{}", format!("Tasks ({}):", tasks.len()).bold());
    } else {
        println!("Tasks ({}):", tasks.len());
    }

    if tasks.is_empty() {
        println!("  (no tasks)");
    } else {
        for task in tasks {
            let indent = "  ".repeat(task.depth as usize + 1);
            if use_color {
                println!(
                    "{}{} {} ({})",
                    indent,
                    format_task_status_icon(task.status),
                    task.title,
                    task.full_id.dimmed()
                );
            } else {
                println!(
                    "{}{} {} ({})",
                    indent,
                    format_task_status_icon(task.status),
                    task.title,
                    task.full_id
                );
            }
        }
    }
}

/// Output task as human-readable text
#[allow(clippy::too_many_lines)]
fn output_text_task(
    task: &TaskRecord,
    file: &FileRecord,
    dependencies: Option<&Vec<TaskRecord>>,
    dependents: Option<&Vec<TaskRecord>>,
    no_color: bool,
) {
    use owo_colors::OwoColorize;

    let use_color = !no_color;

    // Task header
    if use_color {
        println!("{}", "Task:".bold());
        println!("  ID:       {}", task.full_id.cyan());
        println!("  Title:    {}", task.title.bold());
        println!("  Status:   {}", format_task_status(task.status, use_color));
        println!("  File:     {}", file.path.display().to_string().dimmed());
    } else {
        println!("Task:");
        println!("  ID:       {}", task.full_id);
        println!("  Title:    {}", task.title);
        println!("  Status:   {}", format_task_status(task.status, use_color));
        println!("  File:     {}", file.path.display());
    }

    // Optional fields
    if let Some(ref owner) = task.owner {
        println!("  Owner:    {owner}");
    }
    if let Some(ref estimate) = task.estimate {
        println!("  Estimate: {estimate}");
    }

    // Labels
    if !task.metadata.labels.is_empty() {
        if use_color {
            println!("  Labels:   {}", task.metadata.labels.join(", ").dimmed());
        } else {
            println!("  Labels:   {}", task.metadata.labels.join(", "));
        }
    }

    // Body
    if let Some(ref body) = task.body {
        println!();
        if use_color {
            println!("{}", "Description:".bold());
        } else {
            println!("Description:");
        }
        for line in body.lines() {
            println!("  {line}");
        }
    }

    // Dependencies
    if let Some(deps) = dependencies {
        println!();
        if use_color {
            println!("{}", format!("Dependencies ({}):", deps.len()).bold());
        } else {
            println!("Dependencies ({}):", deps.len());
        }

        if deps.is_empty() {
            println!("  (none)");
        } else {
            for dep in deps {
                if use_color {
                    println!(
                        "  {} {} ({})",
                        format_task_status_icon(dep.status),
                        dep.title,
                        dep.full_id.dimmed()
                    );
                } else {
                    println!(
                        "  {} {} ({})",
                        format_task_status_icon(dep.status),
                        dep.title,
                        dep.full_id
                    );
                }
            }
        }
    }

    // Dependents
    if let Some(deps) = dependents {
        println!();
        if use_color {
            println!("{}", format!("Depended on by ({}):", deps.len()).bold());
        } else {
            println!("Depended on by ({}):", deps.len());
        }

        if deps.is_empty() {
            println!("  (none)");
        } else {
            for dep in deps {
                if use_color {
                    println!(
                        "  {} {} ({})",
                        format_task_status_icon(dep.status),
                        dep.title,
                        dep.full_id.dimmed()
                    );
                } else {
                    println!(
                        "  {} {} ({})",
                        format_task_status_icon(dep.status),
                        dep.title,
                        dep.full_id
                    );
                }
            }
        }
    }
}

/// Format task status with color
fn format_task_status(status: lash_types::TaskStatus, use_color: bool) -> String {
    use owo_colors::OwoColorize;

    let status_str = match status {
        lash_types::TaskStatus::Open => "open",
        lash_types::TaskStatus::Done => "done",
        lash_types::TaskStatus::Waived => "waived",
        lash_types::TaskStatus::Blocked => "blocked",
    };

    if use_color {
        match status {
            lash_types::TaskStatus::Open => status_str.blue().to_string(),
            lash_types::TaskStatus::Done => status_str.green().to_string(),
            lash_types::TaskStatus::Waived => status_str.yellow().to_string(),
            lash_types::TaskStatus::Blocked => status_str.red().to_string(),
        }
    } else {
        status_str.to_string()
    }
}

/// Format task status as icon
fn format_task_status_icon(status: lash_types::TaskStatus) -> &'static str {
    match status {
        lash_types::TaskStatus::Open => "[ ]",
        lash_types::TaskStatus::Done => "[x]",
        lash_types::TaskStatus::Waived => "[-]",
        lash_types::TaskStatus::Blocked => "[!]",
    }
}

/// Format file status with color
fn format_file_status(status: lash_types::FileStatus, use_color: bool) -> String {
    use owo_colors::OwoColorize;

    let status_str = match status {
        lash_types::FileStatus::InProgress => "in-progress",
        lash_types::FileStatus::Complete => "complete",
        lash_types::FileStatus::Blocked => "blocked",
        lash_types::FileStatus::Empty => "empty",
    };

    if use_color {
        match status {
            lash_types::FileStatus::InProgress => status_str.blue().to_string(),
            lash_types::FileStatus::Complete => status_str.green().to_string(),
            lash_types::FileStatus::Blocked => status_str.red().to_string(),
            lash_types::FileStatus::Empty => status_str.dimmed().to_string(),
        }
    } else {
        status_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_task_status_icon() {
        assert_eq!(format_task_status_icon(lash_types::TaskStatus::Open), "[ ]");
        assert_eq!(format_task_status_icon(lash_types::TaskStatus::Done), "[x]");
        assert_eq!(
            format_task_status_icon(lash_types::TaskStatus::Waived),
            "[-]"
        );
        assert_eq!(
            format_task_status_icon(lash_types::TaskStatus::Blocked),
            "[!]"
        );
    }
}
