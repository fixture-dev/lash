//! `lash show <file>` — file-detail rendering (text, JSON, and tree view).
//!
//! Split out of `show/mod.rs` (alongside `task_view.rs`/`format.rs`) to keep
//! that file under the project's ~500-line guideline. Owns everything
//! specific to showing a whole file's tasks; task-detail rendering (GitHub
//! issue #26) lives in `task_view.rs`.

use anyhow::Result;
use lash::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash::formatter::OutputFormat;
use lash::theme::CliTheme;
use lash::tree_formatter::TreeFormatter;
use lash_db::repository::files::FileRecord;
use lash_db::repository::tasks::TaskRecord;
use lash_db::{DocRefRepository, FileRepository, TaskRepository};
use lash_types::error::LashError;
use lash_types::tree::TreeNode;
use std::path::{Path, PathBuf};

use super::format::{format_file_status, format_task_status_icon};
use super::ShowArgs;

/// Show detailed information about a file
#[allow(clippy::too_many_lines)]
pub(super) fn show_file(
    file_repo: &FileRepository,
    task_repo: &TaskRepository,
    doc_repo: &DocRefRepository,
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
    let file = match file_repo.get_by_path(&target_path) {
        Ok(Some(file)) => file,
        Ok(None) => {
            let error = LashError::io_file_not_found(target_path);
            let mut diag = error.to_diagnostic();
            diag.help = Some("Make sure the file has been indexed with `lash index`".to_string());

            if args.json {
                super::output_json_diagnostic(&diag, &[])?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormat::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: theme.cloned(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_diagnostic(&diag);
            }
            return Ok(5); // Exit code 5 for not found
        }
        Err(e) => {
            let error = LashError::internal(
                format!("Database query failed: {e}"),
                Some("get_by_path".to_string()),
            );
            if args.json {
                super::output_json_error(&error)?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormat::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: theme.cloned(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_error(&error);
            }
            return Ok(3); // Exit code 3 for DB error
        }
    };

    // Get tasks in this file
    let tasks = match task_repo.get_by_file(file.id) {
        Ok(tasks) => tasks,
        Err(e) => {
            let error = LashError::internal(
                format!("Database query failed: {e}"),
                Some("get_by_file".to_string()),
            );
            if args.json {
                super::output_json_error(&error)?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormat::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: theme.cloned(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_error(&error);
            }
            return Ok(3); // Exit code 3 for DB error
        }
    };

    // Get file-level doc references
    let doc_refs = match doc_repo.find_file_level(file.id) {
        Ok(refs) => refs,
        Err(e) => {
            let error = LashError::internal(
                format!("Database query failed: {e}"),
                Some("find_file_level".to_string()),
            );
            if args.json {
                super::output_json_error(&error)?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormat::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: theme.cloned(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_error(&error);
            }
            return Ok(3); // Exit code 3 for DB error
        }
    };

    // Output results
    if args.json {
        output_json_file(&file, &tasks, &doc_refs)?;
    } else {
        // Determine if tree view is enabled
        let use_tree_view = determine_tree_view_enabled(args);
        output_text_file(&file, &tasks, &doc_refs, theme, use_tree_view, args);
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

/// Output file as JSON
fn output_json_file(
    file: &FileRecord,
    tasks: &[TaskRecord],
    doc_refs: &[lash_db::repository::DocRefRow],
) -> Result<()> {
    use serde_json::json;

    let mut file_json = json!({
        "path": file.path,
        "file_id": file.file_id,
        "title": file.title,
        "status": file.status,
    });

    // Include description if present
    if !file.description.is_empty() {
        file_json["description"] = json!(file.description);
    }

    let output = json!({
        "type": "file",
        "file": file_json,
        "task_count": tasks.len(),
        "tasks": tasks,
        "doc_refs": doc_refs,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output file as human-readable text
fn output_text_file(
    file: &FileRecord,
    tasks: &[TaskRecord],
    doc_refs: &[lash_db::repository::DocRefRow],
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

    // Doc references
    if !doc_refs.is_empty() {
        let doc_str = doc_refs
            .iter()
            .map(|d| {
                if let Some(ref frag) = d.fragment {
                    format!("{}#{}", d.target_path, frag)
                } else {
                    d.target_path.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        if let Some(theme) = theme {
            println!("  Docs:     {}", theme.style_muted(&doc_str));
        } else {
            println!("  Docs:     {doc_str}");
        }
    }

    // Description
    if !file.description.is_empty() {
        println!();
        if let Some(theme) = theme {
            println!("{}", theme.style_info("Description:"));
        } else {
            println!("Description:");
        }
        for line in file.description.lines() {
            println!("  {line}");
        }
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

            // Print contextual notes for this task
            if !task.contextual_notes.is_empty() {
                let note_indent = "  ".repeat(task.depth as usize + 2);
                for note in &task.contextual_notes {
                    if let Some(theme) = theme {
                        println!(
                            "{}{}",
                            note_indent,
                            theme.style_muted(&format!("· {}", note.text()))
                        );
                    } else {
                        println!("{}· {}", note_indent, note.text());
                    }
                }
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
    /// Contextual notes
    contextual_notes: Vec<lash_types::task::ContextualNote>,
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
                contextual_notes: task.contextual_notes.clone(),
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
            lash_types::TaskStatus::InProgress => "[>]",
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

    // Print tree lines and contextual notes
    let mut line_index = 0;
    for root in &roots {
        print_tree_with_notes(root, &lines, &mut line_index, theme);
    }
}

/// Recursively print tree nodes with contextual notes
fn print_tree_with_notes(
    node: &TreeNode<TaskTreeNode>,
    lines: &[String],
    line_index: &mut usize,
    theme: Option<&CliTheme>,
) {
    // Print the task line
    if *line_index < lines.len() {
        println!("  {}", lines[*line_index]);
        *line_index += 1;
    }

    // Print contextual notes for this task
    if !node.data.contextual_notes.is_empty() {
        // Calculate indentation based on depth
        // Each level adds one tree character width (typically 4 spaces or "│   ")
        let indent_per_level = 4;
        let base_indent = node.depth * indent_per_level;
        let note_indent = " ".repeat(base_indent + indent_per_level);

        for note in &node.data.contextual_notes {
            if let Some(theme) = theme {
                println!(
                    "  {}{}",
                    note_indent,
                    theme.style_muted(&format!("· {}", note.text()))
                );
            } else {
                println!("  {}· {}", note_indent, note.text());
            }
        }
    }

    // Recursively print children
    for child in &node.children {
        print_tree_with_notes(child, lines, line_index, theme);
    }
}
