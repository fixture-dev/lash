//! Show command implementation
//!
//! The `lash show` command displays detailed information about a specific task or file.

use anyhow::{Context, Result};
use lash_cli::theme::CliTheme;
use lash_cli::tree_formatter::TreeFormatter;
use lash_db::repository::files::FileRecord;
use lash_db::repository::tasks::TaskRecord;
use lash_db::{open_database, DependencyRepository, FileRepository, TaskRepository};
use lash_types::tree::TreeNode;
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
    /// Enable tree view (None = use config default)
    pub tree_view: Option<bool>,
    /// Maximum tree depth
    pub max_depth: Option<usize>,
    /// Use ASCII characters for tree
    pub ascii: bool,
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
    // Check for path separators (both forward and back slashes) or .md extension
    let is_file_path = args.target.contains('/')
        || args.target.contains('\\')
        || std::path::Path::new(&args.target)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));

    // Show file or task information and return appropriate exit code
    let exit_code = if is_file_path {
        // Show file information
        show_file(&file_repo, &task_repo, args, &project_root, theme.as_ref())?
    } else {
        // Show task information
        show_task(&task_repo, &file_repo, &dep_repo, args, theme.as_ref())?
    };

    Ok(exit_code)
}

/// Show detailed information about a file
fn show_file(
    file_repo: &FileRepository,
    task_repo: &TaskRepository,
    args: &ShowArgs,
    project_root: &Path,
    theme: Option<&CliTheme>,
) -> Result<i32> {
    // Convert target to relative path if needed
    // Normalize path separators - convert forward slashes to native separators
    let normalized_target = args.target.replace('/', std::path::MAIN_SEPARATOR_STR);

    let target_path = if normalized_target.starts_with(std::path::MAIN_SEPARATOR)
        || normalized_target.starts_with('/')
    {
        // Absolute path - make it relative to project root
        PathBuf::from(&normalized_target)
            .strip_prefix(project_root)
            .map_or_else(
                |_| PathBuf::from(&normalized_target),
                std::path::Path::to_path_buf,
            )
    } else {
        PathBuf::from(&normalized_target)
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
        // Determine if tree view is enabled
        let use_tree_view = determine_tree_view_enabled(args);
        output_text_file(&file, &tasks, theme, use_tree_view, args);
    }

    Ok(0)
}

/// Determine if tree view should be enabled
///
/// Priority: CLI flag > config > default (true)
fn determine_tree_view_enabled(args: &ShowArgs) -> bool {
    // Check CLI flag first
    if let Some(tree_view) = args.tree_view {
        return tree_view;
    }

    // Fall back to config
    match lash_types::UserConfig::load() {
        Ok(config) => config.tree_view.enabled,
        Err(_) => true, // Default to true if config can't be loaded
    }
}

/// Show detailed information about a task
fn show_task(
    task_repo: &TaskRepository,
    file_repo: &FileRepository,
    dep_repo: &DependencyRepository,
    args: &ShowArgs,
    theme: Option<&CliTheme>,
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
    let file = file_repo
        .get_by_db_id(task.file_id)?
        .unwrap_or_else(|| FileRecord {
            id: task.file_id,
            path: PathBuf::from(format!("<file-id-{}>", task.file_id)),
            file_id: format!("file-{}", task.file_id),
            title: String::from("<unknown>"),
            hash: String::new(),
            mtime: 0,
            status: lash_types::FileStatus::InProgress,
            metadata: lash_types::FileMetadata::default(),
            indexed_at: 0,
        });

    // Get dependencies if requested
    let dependencies = if args.deps {
        let dep_records = dep_repo.get_dependencies(task.id)?;
        let mut deps = Vec::new();
        for dep_record in dep_records {
            // Only include dependencies that have been resolved
            if let Some(to_task_id) = dep_record.to_task_id {
                match task_repo.get_by_db_id(to_task_id)? {
                    Some(task_record) => deps.push(task_record),
                    None => {
                        tracing::warn!(
                            "Failed to resolve dependency: task DB ID {} not found",
                            to_task_id
                        );
                    }
                }
            }
        }
        Some(deps)
    } else {
        None
    };

    // Get reverse dependencies if requested
    let dependents = if args.rdeps {
        let dep_records = dep_repo.get_dependents(task.id)?;
        let mut deps = Vec::new();
        for dep_record in dep_records {
            match task_repo.get_by_db_id(dep_record.from_task_id)? {
                Some(task_record) => deps.push(task_record),
                None => {
                    tracing::warn!(
                        "Failed to resolve dependent: task DB ID {} not found",
                        dep_record.from_task_id
                    );
                }
            }
        }
        Some(deps)
    } else {
        None
    };

    // Output results
    if args.json {
        output_json_task(&task, &file, dependencies, dependents)?;
    } else {
        output_text_task(
            &task,
            &file,
            dependencies.as_ref(),
            dependents.as_ref(),
            theme,
        );
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
fn output_text_file(
    file: &FileRecord,
    tasks: &[TaskRecord],
    theme: Option<&CliTheme>,
    use_tree_view: bool,
    args: &ShowArgs,
) {
    // File header
    if let Some(theme) = theme {
        println!("{}", theme.style_info("File:"));
        println!(
            "  Path:     {}",
            theme.style_label(&file.path.display().to_string())
        );
        println!("  Title:    {}", theme.style_info(&file.title));
        println!("  ID:       {}", theme.style_muted(&file.file_id));
        println!(
            "  Status:   {}",
            format_file_status(file.status, Some(theme))
        );
    } else {
        println!("File:");
        println!("  Path:     {}", file.path.display());
        println!("  Title:    {}", file.title);
        println!("  ID:       {}", file.file_id);
        println!("  Status:   {}", format_file_status(file.status, None));
    }

    // Task summary
    println!();
    if let Some(theme) = theme {
        println!("{}", theme.style_info(&format!("Tasks ({}):", tasks.len())));
    } else {
        println!("Tasks ({}):", tasks.len());
    }

    if tasks.is_empty() {
        println!("  (no tasks)");
    } else if use_tree_view {
        output_tasks_as_tree(tasks, theme, args);
    } else {
        // Flat view with indentation
        for task in tasks {
            let indent = "  ".repeat(task.depth as usize + 1);
            if let Some(theme) = theme {
                let checkbox = theme.styled_checkbox(task.status);
                let full_id = theme.style_muted(&task.full_id);
                println!("{}{} {} ({})", indent, checkbox, task.title, full_id);
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

/// Represents a task node in the tree view
#[derive(Debug, Clone)]
struct TaskTreeNode {
    /// Task title
    title: String,
    /// Task full ID
    full_id: String,
    /// Task status
    status: lash_types::TaskStatus,
}

/// Output tasks as a tree view
fn output_tasks_as_tree(tasks: &[TaskRecord], theme: Option<&CliTheme>, args: &ShowArgs) {
    // Build tree structure from flat task list
    // Tasks are already ordered by parent-child relationship with depth info
    let config = lash_types::UserConfig::load().unwrap_or_default();
    let max_depth = args.max_depth.unwrap_or(config.tree_view.max_depth);

    // Build tree by tracking parent indices for each depth level
    let mut roots: Vec<TreeNode<TaskTreeNode>> = Vec::new();
    let mut stack: Vec<*mut TreeNode<TaskTreeNode>> = Vec::new();

    for task in tasks {
        let node = TreeNode::new(
            TaskTreeNode {
                title: task.title.clone(),
                full_id: task.full_id.clone(),
                status: task.status,
            },
            task.depth as usize,
        );

        let depth = task.depth as usize;

        if depth == 0 {
            // Root node - just add and expand
            let mut new_node = node;
            new_node.expand();
            roots.push(new_node);
            // Update stack pointer to point to the new root
            stack.clear();
            stack.push(roots.last_mut().unwrap() as *mut _);
        } else {
            // Child node - find parent at depth-1
            // Truncate stack to parent depth
            stack.truncate(depth);

            if let Some(&parent_ptr) = stack.last() {
                // SAFETY: We maintain the invariant that pointers in stack are valid
                // and point to nodes in roots tree structure
                unsafe {
                    let parent = &mut *parent_ptr;
                    let mut new_node = node;
                    new_node.expand();
                    parent.children.push(new_node);
                    // Add pointer to the new child
                    stack.push(parent.children.last_mut().unwrap() as *mut _);
                }
            }
        }
    }

    // Create formatter and render
    let formatter = TreeFormatter::new(args.ascii, max_depth, theme.cloned());

    let lines = formatter.format_tree(&roots, |node, fmt| {
        let status_indicator = match node.status {
            lash_types::TaskStatus::Open => "[ ]",
            lash_types::TaskStatus::Done => "[x]",
            lash_types::TaskStatus::Waived => "[-]",
            lash_types::TaskStatus::Blocked => "[!]",
        };

        if let Some(theme) = fmt.theme() {
            let checkbox = theme.styled_checkbox(node.status);
            let full_id = theme.style_muted(&node.full_id);
            format!("{} {} ({})", checkbox, node.title, full_id)
        } else {
            format!("{} {} ({})", status_indicator, node.title, node.full_id)
        }
    });

    for line in lines {
        println!("  {line}");
    }
}

/// Output task as human-readable text
#[allow(clippy::too_many_lines)]
fn output_text_task(
    task: &TaskRecord,
    file: &FileRecord,
    dependencies: Option<&Vec<TaskRecord>>,
    dependents: Option<&Vec<TaskRecord>>,
    theme: Option<&CliTheme>,
) {
    // Task header
    if let Some(theme) = theme {
        println!("{}", theme.style_info("Task:"));
        println!("  ID:       {}", theme.style_label(&task.full_id));
        println!("  Title:    {}", theme.style_info(&task.title));
        println!(
            "  Status:   {}",
            format_task_status(task.status, Some(theme))
        );
        println!(
            "  File:     {}",
            theme.style_muted(&file.path.display().to_string())
        );
    } else {
        println!("Task:");
        println!("  ID:       {}", task.full_id);
        println!("  Title:    {}", task.title);
        println!("  Status:   {}", format_task_status(task.status, None));
        println!("  File:     {}", file.path.display());
    }

    // Optional fields
    if let Some(ref owner) = task.owner {
        if let Some(theme) = theme {
            println!("  Owner:    {}", theme.style_info(owner));
        } else {
            println!("  Owner:    {owner}");
        }
    }
    if let Some(ref estimate) = task.estimate {
        if let Some(theme) = theme {
            println!("  Estimate: {}", theme.style_info(estimate));
        } else {
            println!("  Estimate: {estimate}");
        }
    }

    // Labels
    if !task.metadata.labels.is_empty() {
        if let Some(theme) = theme {
            let labels = task
                .metadata
                .labels
                .iter()
                .map(|l| theme.style_label(l))
                .collect::<Vec<_>>()
                .join(", ");
            println!("  Labels:   {labels}");
        } else {
            println!("  Labels:   {}", task.metadata.labels.join(", "));
        }
    }

    // Body
    if let Some(ref body) = task.body {
        println!();
        if let Some(theme) = theme {
            println!("{}", theme.style_info("Description:"));
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
        if let Some(theme) = theme {
            println!(
                "{}",
                theme.style_info(&format!("Dependencies ({}):", deps.len()))
            );
        } else {
            println!("Dependencies ({}):", deps.len());
        }

        if deps.is_empty() {
            println!("  (none)");
        } else {
            for dep in deps {
                if let Some(theme) = theme {
                    let checkbox = theme.styled_checkbox(dep.status);
                    let full_id = theme.style_muted(&dep.full_id);
                    println!("  {} {} ({})", checkbox, dep.title, full_id);
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
        if let Some(theme) = theme {
            println!(
                "{}",
                theme.style_info(&format!("Depended on by ({}):", deps.len()))
            );
        } else {
            println!("Depended on by ({}):", deps.len());
        }

        if deps.is_empty() {
            println!("  (none)");
        } else {
            for dep in deps {
                if let Some(theme) = theme {
                    let checkbox = theme.styled_checkbox(dep.status);
                    let full_id = theme.style_muted(&dep.full_id);
                    println!("  {} {} ({})", checkbox, dep.title, full_id);
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
fn format_task_status(status: lash_types::TaskStatus, theme: Option<&CliTheme>) -> String {
    let status_str = match status {
        lash_types::TaskStatus::Open => "open",
        lash_types::TaskStatus::Done => "done",
        lash_types::TaskStatus::Waived => "waived",
        lash_types::TaskStatus::Blocked => "blocked",
    };

    if let Some(theme) = theme {
        theme.style_task_status(status_str, status)
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
fn format_file_status(status: lash_types::FileStatus, theme: Option<&CliTheme>) -> String {
    let status_str = match status {
        lash_types::FileStatus::InProgress => "in-progress",
        lash_types::FileStatus::Complete => "complete",
        lash_types::FileStatus::Blocked => "blocked",
        lash_types::FileStatus::Empty => "empty",
    };

    if let Some(theme) = theme {
        match status {
            lash_types::FileStatus::InProgress => theme.style_info(status_str),
            lash_types::FileStatus::Complete => theme.style_success(status_str),
            lash_types::FileStatus::Blocked => theme.style_error(status_str),
            lash_types::FileStatus::Empty => theme.style_muted(status_str),
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
