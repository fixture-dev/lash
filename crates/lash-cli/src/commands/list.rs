//! List command implementation
//!
//! The `lash list` command queries and filters tasks from the `SQLite` database.

use anyhow::{Context, Result};
use lash::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash::formatter::{OutputFormat as OutputFormatTrait, Verbosity};
use lash::theme::CliTheme;
use lash::tree_formatter::TreeFormatter;
use lash_core::fuzzy::FuzzyMatcher;
use lash_db::repository::files::FileRecord;
use lash_db::repository::tasks::{TaskFilter, TaskRecord};
use lash_db::{open_database, DocRefRepository, FileRepository, TaskRepository};
use lash_types::error::LashError;
use lash_types::tree::TreeNode;
use std::collections::{HashMap, HashSet};
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
#[allow(clippy::struct_excessive_bools)]
pub struct ListArgs {
    /// Filter by task ID (supports fuzzy matching)
    pub filter: Option<String>,
    /// Filter by label (can be specified multiple times)
    pub labels: Vec<String>,
    /// Filter by status
    pub status: Option<lash_types::TaskStatus>,
    /// Filter by path prefix (relative to project root)
    pub path: Option<PathBuf>,
    /// Only show blocked tasks
    pub blocked: bool,
    /// Filter by owner
    pub owner: Option<String>,
    /// Filter by files/tasks that reference a specific document
    pub docs: Option<String>,
    /// Show file descriptions (truncated to 100 chars)
    pub show_descriptions: bool,
    /// Show contextual notes for tasks
    pub show_notes: bool,
    /// Maximum number of files/tasks to show (None = unlimited)
    pub limit: Option<usize>,
    /// Output format
    pub format: OutputFormat,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Optional CLI theme for styling
    pub theme: Option<CliTheme>,
    /// Enable tree view (None = use config default)
    pub tree_view: Option<bool>,
    /// Maximum tree depth
    pub max_depth: Option<usize>,
    /// Use ASCII characters for tree
    pub ascii: bool,
    /// Verbosity level for output
    pub verbosity: Verbosity,
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
#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_lines)]
pub fn execute(args: ListArgs) -> Result<i32> {
    // Determine project root
    let project_root = if let Some(ref root) = args.project_root {
        root.clone()
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
        let error = LashError::io_file_not_found(db_path);
        // Convert to diagnostic and add help text
        let mut diag = error.to_diagnostic();
        diag.help = Some("Run `lash index` to create the database".to_string());

        if args.format == OutputFormat::Json || args.format == OutputFormat::JsonPretty {
            output_json_diagnostic(&diag)?;
        } else {
            let reporter_config = ErrorReporterConfig {
                verbosity: args.verbosity,
                output_format: OutputFormatTrait::Text,
                display_mode: ErrorDisplayMode::Streaming,
                theme: args.theme.clone(),
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

            if args.format == OutputFormat::Json || args.format == OutputFormat::JsonPretty {
                output_json_diagnostic(&diag)?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormatTrait::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: args.theme.clone(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_diagnostic(&diag);
            }
            return Ok(3); // Exit code 3 for DB error
        }
    };

    // Create repositories
    let file_repo = FileRepository::new(&conn);
    let task_repo = TaskRepository::new(&conn);
    let doc_repo = DocRefRepository::new(&conn);

    // Handle task ID filter if provided
    if let Some(ref filter_id) = args.filter {
        return execute_filtered_list(&args, &task_repo, &file_repo, filter_id);
    }

    // Check if tree view is enabled
    let use_tree_view = determine_tree_view_enabled(&args);

    tracing::debug!(
        use_tree_view = use_tree_view,
        "Determined tree view setting"
    );

    // Get files
    let mut files = match file_repo.list_all() {
        Ok(files) => files,
        Err(e) => {
            let error = LashError::internal(
                format!("Database query failed: {e}"),
                Some("list_all".to_string()),
            );
            report_db_error(&args, &error)?;
            return Ok(3); // Exit code 3 for DB error
        }
    };

    // Filter by doc references if requested
    if let Some(ref doc_path) = args.docs {
        tracing::debug!(doc_path = %doc_path, "Filtering by doc references");

        // Find all files that reference this document
        let doc_sources = match doc_repo.find_by_target_prefix(doc_path) {
            Ok(sources) => sources,
            Err(e) => {
                let error = LashError::internal(
                    format!("Database query failed: {e}"),
                    Some("find_by_target_prefix".to_string()),
                );
                report_db_error(&args, &error)?;
                return Ok(3); // Exit code 3 for DB error
            }
        };

        // Collect unique file IDs
        let file_ids: HashSet<i64> = doc_sources.iter().map(|row| row.source_file_id).collect();

        // Filter files to only those that reference the doc
        files.retain(|f| file_ids.contains(&f.id));

        tracing::debug!(
            filtered_count = files.len(),
            "Filtered files by doc references"
        );
    }

    tracing::debug!(file_count = files.len(), "Retrieved files");

    // Filter by path prefix if requested (paths are stored absolute, so
    // compare against the project-root-relative path as well)
    if let Some(ref path_prefix) = args.path {
        files.retain(|f| {
            f.path
                .strip_prefix(&project_root)
                .unwrap_or(&f.path)
                .starts_with(path_prefix)
                || f.path.starts_with(path_prefix)
        });
        tracing::debug!(
            filtered_count = files.len(),
            "Filtered files by path prefix"
        );
    }

    // Task-level filters switch to a task-centric listing: only matching
    // tasks (and the files containing them) are shown
    let has_task_filters =
        args.status.is_some() || !args.labels.is_empty() || args.owner.is_some() || args.blocked;
    if has_task_filters {
        return execute_task_filtered_list(&args, &task_repo, files);
    }

    // Apply limit if specified
    if let Some(max) = args.limit {
        files.truncate(max);
    }

    // Fetch tasks if tree view or show_notes is enabled
    let file_tasks: HashMap<i64, Vec<TaskRecord>> = if use_tree_view || args.show_notes {
        let task_repo = TaskRepository::new(&conn);
        let mut tasks_map = HashMap::new();
        for file in &files {
            match task_repo.get_by_file(file.id) {
                Ok(tasks) => {
                    tasks_map.insert(file.id, tasks);
                }
                Err(e) => {
                    tracing::warn!(file_id = file.id, error = %e, "Failed to fetch tasks for file");
                    tasks_map.insert(file.id, Vec::new());
                }
            }
        }
        tasks_map
    } else {
        HashMap::new()
    };

    // Output results
    match args.format {
        OutputFormat::Json => {
            output_json_files(&files, &file_tasks, args.show_descriptions, args.show_notes)?;
        }
        OutputFormat::JsonPretty => {
            output_json_pretty_files(&files, &file_tasks, args.show_descriptions, args.show_notes)?;
        }
        OutputFormat::Text => {
            if use_tree_view {
                output_text_tree(&files, &file_tasks, &args);
            } else {
                output_text_flat(
                    &files,
                    &file_tasks,
                    args.theme.as_ref(),
                    args.show_descriptions,
                    args.show_notes,
                );
            }
        }
    }

    Ok(0)
}

/// Execute the list command with task-level filters (status, labels, owner, blocked)
///
/// Queries matching tasks from the database and restricts output to the files
/// containing them. Tree view shows only the matching tasks; flat and JSON
/// output list the matching tasks grouped by file.
fn execute_task_filtered_list(
    args: &ListArgs,
    task_repo: &TaskRepository,
    files: Vec<FileRecord>,
) -> Result<i32> {
    let filter = TaskFilter {
        status: args.status,
        labels: args.labels.clone(),
        owner: args.owner.clone(),
        file_path: None,
        blocked: if args.blocked { Some(true) } else { None },
    };

    let mut tasks = match task_repo.find(&filter) {
        Ok(tasks) => tasks,
        Err(e) => {
            let error = LashError::internal(
                format!("Database query failed: {e}"),
                Some("find".to_string()),
            );
            report_db_error(args, &error)?;
            return Ok(3); // Exit code 3 for DB error
        }
    };

    // Restrict to the already-filtered file set (path/docs filters)
    let file_ids: HashSet<i64> = files.iter().map(|f| f.id).collect();
    tasks.retain(|t| file_ids.contains(&t.file_id));

    if let Some(max) = args.limit {
        tasks.truncate(max);
    }

    // Group tasks by file, in document order within each file
    let mut file_tasks: HashMap<i64, Vec<TaskRecord>> = HashMap::new();
    for task in &tasks {
        file_tasks
            .entry(task.file_id)
            .or_default()
            .push(task.clone());
    }
    for tasks_in_file in file_tasks.values_mut() {
        tasks_in_file.sort_by_key(|t| t.order_index);
    }

    // Keep only files that contain matching tasks
    let files: Vec<FileRecord> = files
        .into_iter()
        .filter(|f| file_tasks.contains_key(&f.id))
        .collect();

    tracing::debug!(
        task_count = tasks.len(),
        file_count = files.len(),
        "Applied task-level filters"
    );

    if tasks.is_empty() {
        output_no_filter_matches(args)?;
        return Ok(0);
    }

    if args.format == OutputFormat::Text && determine_tree_view_enabled(args) {
        output_text_tree(&files, &file_tasks, args);
    } else {
        output_filtered_tasks(args, &tasks, &files);
    }

    Ok(0)
}

/// Report a database error in the format requested by the user
fn report_db_error(args: &ListArgs, error: &LashError) -> Result<()> {
    if args.format == OutputFormat::Json || args.format == OutputFormat::JsonPretty {
        output_json_error(error)?;
    } else {
        let reporter_config = ErrorReporterConfig {
            verbosity: args.verbosity,
            output_format: OutputFormatTrait::Text,
            display_mode: ErrorDisplayMode::Streaming,
            theme: args.theme.clone(),
            show_summary: false,
        };
        let mut reporter = ErrorReporter::new(reporter_config);
        reporter.report_error(error);
    }
    Ok(())
}

/// Output for the case where task-level filters matched no tasks
fn output_no_filter_matches(args: &ListArgs) -> Result<()> {
    match args.format {
        OutputFormat::Json | OutputFormat::JsonPretty => {
            use serde_json::json;
            let output = json!({ "count": 0, "tasks": [], "files": [] });
            if args.format == OutputFormat::JsonPretty {
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("{}", serde_json::to_string(&output)?);
            }
        }
        OutputFormat::Text => {
            let msg = "No tasks found matching the given filters";
            if let Some(theme) = args.theme.as_ref() {
                println!("{}", theme.style_warning(msg));
            } else {
                println!("{msg}");
            }
        }
    }
    Ok(())
}

/// Check whether a task matches the task-level filters (status, blocked, owner, labels)
///
/// Mirrors the semantics of `TaskRepository::find`: labels match if the task
/// has any of the requested labels.
fn task_matches_filters(task: &TaskRecord, args: &ListArgs) -> bool {
    if let Some(status) = args.status {
        if task.status != status {
            return false;
        }
    }
    if args.blocked && task.status != lash_types::TaskStatus::Blocked {
        return false;
    }
    if let Some(ref owner) = args.owner {
        if task.owner.as_deref() != Some(owner.as_str()) {
            return false;
        }
    }
    if !args.labels.is_empty() && !args.labels.iter().any(|l| task.metadata.labels.contains(l)) {
        return false;
    }
    true
}

/// Execute the list command with task ID filtering
///
/// This function handles the --filter flag which allows filtering tasks by ID,
/// with fuzzy matching support when no exact match is found.
fn execute_filtered_list(
    args: &ListArgs,
    task_repo: &TaskRepository,
    file_repo: &FileRepository,
    filter_id: &str,
) -> Result<i32> {
    // Get all task IDs for fuzzy matching
    let all_task_ids = task_repo.get_all_full_ids().unwrap_or_default();

    // Try exact match first
    if let Ok(Some(task)) = task_repo.get_by_full_id(filter_id) {
        // Found exact match - show this task if it passes the other filters
        if !task_matches_filters(&task, args) {
            output_no_filter_matches(args)?;
            return Ok(0);
        }
        let file = file_repo
            .get_by_db_id(task.file_id)?
            .unwrap_or_else(|| FileRecord {
                id: task.file_id,
                path: PathBuf::from(format!("<file-id-{}>", task.file_id)),
                file_id: format!("file-{}", task.file_id),
                title: String::from("<unknown>"),
                description: String::new(),
                hash: String::new(),
                mtime: 0,
                status: lash_types::FileStatus::InProgress,
                metadata: lash_types::FileMetadata::default(),
                indexed_at: 0,
            });

        output_filtered_tasks(args, &[task], &[file]);
        return Ok(0);
    }

    // No exact match - use fuzzy matching
    let id_matcher = FuzzyMatcher::new(0.4, 10); // Lower threshold for more results
    let similar_ids = id_matcher.find_matches(filter_id, &all_task_ids);

    if similar_ids.is_empty() {
        // No matches found - output helpful error message
        let msg = format!("No tasks found matching '{filter_id}'");
        if let Some(theme) = args.theme.as_ref() {
            println!("{}", theme.style_warning(&msg));
        } else {
            println!("{msg}");
        }
        println!();
        println!("Try running `lash search {filter_id}` for full-text search");
        return Ok(5); // Exit code 5 for not found
    }

    // Get the matching tasks
    let mut tasks = Vec::new();
    let mut file_ids = HashSet::new();

    for candidate in &similar_ids {
        if let Ok(Some(task)) = task_repo.get_by_full_id(&candidate.task_id) {
            if task_matches_filters(&task, args) {
                file_ids.insert(task.file_id);
                tasks.push(task);
            }
        }
    }

    if tasks.is_empty() {
        output_no_filter_matches(args)?;
        return Ok(0);
    }

    // Apply limit if specified
    if let Some(max) = args.limit {
        tasks.truncate(max);
    }

    // Get the files for matching tasks
    let files: Vec<FileRecord> = file_ids
        .iter()
        .filter_map(|&id| file_repo.get_by_db_id(id).ok().flatten())
        .collect();

    // Check if we have an exact match
    let has_exact = similar_ids.first().is_some_and(|m| m.score >= 1.0);

    if !has_exact {
        // Show suggestion message
        let match_count = similar_ids.len();
        let msg = if match_count == 1 {
            format!(
                "No exact match for '{}'. Did you mean '{}'?",
                filter_id, similar_ids[0].task_id
            )
        } else {
            format!("No exact match for '{filter_id}'. Found {match_count} similar task IDs:")
        };

        if let Some(theme) = args.theme.as_ref() {
            println!("{}", theme.style_info(&msg));
        } else {
            println!("{msg}");
        }

        if match_count > 1 {
            for candidate in similar_ids.iter().take(5) {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let score_pct = (candidate.score * 100.0) as u32;
                if let Some(theme) = args.theme.as_ref() {
                    println!(
                        "  {} {}",
                        theme.style_label(&candidate.task_id),
                        theme.style_muted(&format!("({score_pct}% match)"))
                    );
                } else {
                    println!("  {} ({}% match)", candidate.task_id, score_pct);
                }
            }
        }
        println!();
    }

    output_filtered_tasks(args, &tasks, &files);
    Ok(0)
}

/// Output filtered tasks in the appropriate format
fn output_filtered_tasks(args: &ListArgs, tasks: &[TaskRecord], files: &[FileRecord]) {
    if tasks.is_empty() {
        return;
    }

    match args.format {
        OutputFormat::Json | OutputFormat::JsonPretty => {
            use serde_json::json;

            let files_json: Vec<serde_json::Value> = files.iter().map(|f| json!(f)).collect();
            let tasks_json: Vec<serde_json::Value> = tasks.iter().map(|t| json!(t)).collect();

            let output = json!({
                "count": tasks.len(),
                "tasks": tasks_json,
                "files": files_json
            });

            if args.format == OutputFormat::JsonPretty {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_default()
                );
            } else {
                println!("{}", serde_json::to_string(&output).unwrap_or_default());
            }
        }
        OutputFormat::Text => {
            // Create file lookup map
            let file_map: HashMap<i64, &FileRecord> = files.iter().map(|f| (f.id, f)).collect();

            // Group tasks by file
            let mut tasks_by_file: HashMap<i64, Vec<&TaskRecord>> = HashMap::new();
            for task in tasks {
                tasks_by_file.entry(task.file_id).or_default().push(task);
            }

            // Print tasks grouped by file
            for (file_id, file_tasks) in &tasks_by_file {
                if let Some(file) = file_map.get(file_id) {
                    // Print file header
                    if let Some(theme) = args.theme.as_ref() {
                        println!("{}", theme.style_label(&file.path.display().to_string()));
                    } else {
                        println!("{}", file.path.display());
                    }

                    // Print tasks
                    for task in file_tasks {
                        let checkbox = match task.status {
                            lash_types::TaskStatus::Open => "[ ]",
                            lash_types::TaskStatus::InProgress => "[>]",
                            lash_types::TaskStatus::Done => "[x]",
                            lash_types::TaskStatus::Waived => "[-]",
                            lash_types::TaskStatus::Blocked => "[!]",
                        };

                        if let Some(theme) = args.theme.as_ref() {
                            let styled_checkbox = theme.styled_checkbox(task.status);
                            let task_id = theme.style_muted(&format!("({})", task.full_id));
                            println!("  {} {} {}", styled_checkbox, task.title, task_id);
                        } else {
                            println!("  {} {} ({})", checkbox, task.title, task.full_id);
                        }
                    }
                    println!();
                }
            }

            // Print summary
            println!("Found {} task(s) in {} file(s)", tasks.len(), files.len());
        }
    }
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/lash.db")
}

/// Truncate a description to a maximum length, adding "..." if truncated
///
/// # Arguments
///
/// * `description` - The description text to truncate
/// * `max_len` - Maximum length (default: 100 characters)
///
/// # Returns
///
/// Truncated string with "..." appended if original was longer than `max_len`
fn truncate_description(description: &str, max_len: usize) -> String {
    if description.is_empty() {
        return String::new();
    }

    // Replace newlines with spaces for single-line display
    let single_line = description.replace('\n', " ").replace('\r', "");

    // Collapse multiple spaces into single spaces
    let collapsed = single_line.split_whitespace().collect::<Vec<_>>().join(" ");

    if collapsed.len() <= max_len {
        collapsed
    } else {
        // Find a valid char boundary at or before max_len
        let mut boundary = max_len;
        while !collapsed.is_char_boundary(boundary) && boundary > 0 {
            boundary -= 1;
        }
        format!("{}...", &collapsed[..boundary])
    }
}

/// Determine if tree view should be enabled
///
/// Priority: CLI flag > config > default (true)
fn determine_tree_view_enabled(args: &ListArgs) -> bool {
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

/// Represents a node in the hierarchical task tree
/// Can be a directory, file, or task
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum TaskTreeNodeData {
    /// Directory containing files
    Directory { name: String, path: PathBuf },
    /// Task file containing tasks
    File { record: FileRecord },
    /// Individual task (`file_path` stored for potential future use)
    Task {
        record: TaskRecord,
        file_path: PathBuf,
    },
}

/// Build complete task tree from files and their tasks
///
/// Creates a hierarchical tree structure that includes directories, files, and their tasks.
/// Tasks are nested under their parent file nodes, preserving task parent-child relationships.
#[allow(clippy::too_many_lines)]
fn build_task_tree(
    files: &[FileRecord],
    file_tasks: &HashMap<i64, Vec<TaskRecord>>,
    max_depth: usize,
    default_expanded: bool,
) -> Vec<TreeNode<TaskTreeNodeData>> {
    if files.is_empty() {
        return Vec::new();
    }

    // Group files by directory
    let mut dir_map: HashMap<PathBuf, Vec<FileRecord>> = HashMap::new();
    for file in files {
        let dir = file
            .path
            .parent()
            .unwrap_or_else(|| std::path::Path::new(""));
        dir_map
            .entry(dir.to_path_buf())
            .or_default()
            .push(file.clone());
    }

    // Build tree structure
    let mut roots: Vec<TreeNode<TaskTreeNodeData>> = Vec::new();
    let mut dir_nodes: HashMap<PathBuf, TreeNode<TaskTreeNodeData>> = HashMap::new();

    // Sort directories by path for consistent ordering
    let mut sorted_dirs: Vec<_> = dir_map.keys().collect();
    sorted_dirs.sort();

    for dir_path in sorted_dirs {
        let dir_files = dir_map.get(dir_path).unwrap();
        let depth = dir_path.components().count();

        // Create directory node if it doesn't exist
        if !dir_nodes.contains_key(dir_path) && depth > 0 {
            let dir_name = dir_path
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(""))
                .to_string_lossy()
                .to_string();

            let mut dir_node = TreeNode::new(
                TaskTreeNodeData::Directory {
                    name: dir_name,
                    path: dir_path.clone(),
                },
                depth,
            );

            if default_expanded && depth < max_depth {
                dir_node.expand();
            }

            dir_nodes.insert(dir_path.clone(), dir_node);
        }

        // Add file nodes with their tasks
        for file in dir_files {
            let file_depth = depth + 1;
            let mut file_node = TreeNode::new(
                TaskTreeNodeData::File {
                    record: file.clone(),
                },
                file_depth,
            );

            if default_expanded && file_depth < max_depth {
                file_node.expand();
            }

            // Add tasks as children of the file
            if let Some(tasks) = file_tasks.get(&file.id) {
                add_tasks_to_node(
                    &mut file_node,
                    tasks,
                    file_depth,
                    max_depth,
                    default_expanded,
                    &file.path,
                );
            }

            // Add file to parent directory or root
            if depth == 0 {
                roots.push(file_node);
            } else if let Some(parent) = dir_nodes.get_mut(dir_path) {
                parent.children.push(file_node);
            }
        }
    }

    // Build parent-child relationships for directories
    let mut dir_paths: Vec<PathBuf> = dir_nodes.keys().cloned().collect();
    dir_paths.sort();
    for dir_path in &dir_paths {
        if let Some(parent_path) = dir_path.parent() {
            if parent_path.as_os_str().is_empty() {
                // This is a root directory
                if let Some(node) = dir_nodes.remove(dir_path) {
                    roots.push(node);
                }
            } else if dir_nodes.contains_key(parent_path) {
                // This directory has a parent in the tree
                if let Some(node) = dir_nodes.remove(dir_path) {
                    if let Some(parent) = dir_nodes.get_mut(parent_path) {
                        parent.children.push(node);
                    }
                }
            } else {
                // Parent doesn't exist, add as root
                if let Some(node) = dir_nodes.remove(dir_path) {
                    roots.push(node);
                }
            }
        }
    }

    // Sort roots by display name for deterministic order
    roots.sort_by(|a, b| {
        let name_a = match &a.data {
            TaskTreeNodeData::Directory { name, .. } => name.clone(),
            TaskTreeNodeData::File { record } => record
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            TaskTreeNodeData::Task { record, .. } => record.title.clone(),
        };
        let name_b = match &b.data {
            TaskTreeNodeData::Directory { name, .. } => name.clone(),
            TaskTreeNodeData::File { record } => record
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            TaskTreeNodeData::Task { record, .. } => record.title.clone(),
        };
        name_a.cmp(&name_b)
    });

    roots
}

/// Add tasks to a file node, preserving task hierarchy
fn add_tasks_to_node(
    file_node: &mut TreeNode<TaskTreeNodeData>,
    tasks: &[TaskRecord],
    file_depth: usize,
    max_depth: usize,
    default_expanded: bool,
    file_path: &Path,
) {
    // Build task hierarchy using parent_id relationships
    // Tasks are already ordered by order_index

    let mut task_nodes: HashMap<i64, TreeNode<TaskTreeNodeData>> = HashMap::new();
    let mut root_task_ids: Vec<i64> = Vec::new();
    let mut child_task_ids: Vec<(i64, i64)> = Vec::new(); // (child_id, parent_id)

    // First pass: create all task nodes and identify relationships
    for task in tasks {
        let task_depth = file_depth + 1 + task.depth as usize;
        let mut node = TreeNode::new(
            TaskTreeNodeData::Task {
                record: task.clone(),
                file_path: file_path.to_path_buf(),
            },
            task_depth,
        );

        if default_expanded && task_depth < max_depth {
            node.expand();
        }

        if let Some(parent_db_id) = task.parent_id {
            child_task_ids.push((task.id, parent_db_id));
        } else {
            root_task_ids.push(task.id);
        }

        task_nodes.insert(task.id, node);
    }

    // Second pass: build parent-child relationships
    for (child_id, parent_id) in child_task_ids {
        if let Some(child_node) = task_nodes.remove(&child_id) {
            if let Some(parent_node) = task_nodes.get_mut(&parent_id) {
                parent_node.children.push(child_node);
            } else {
                // Parent not found, treat as root
                task_nodes.insert(child_id, child_node);
                root_task_ids.push(child_id);
            }
        }
    }

    // Add root tasks to file node (preserving order)
    for task_id in root_task_ids {
        if let Some(node) = task_nodes.remove(&task_id) {
            file_node.children.push(node);
        }
    }
}

/// Output error as JSON
fn output_json_error(error: &LashError) -> Result<()> {
    let diagnostic = error.to_diagnostic();
    output_json_diagnostic(&diagnostic)
}

/// Output diagnostic as JSON
fn output_json_diagnostic(diagnostic: &lash_types::error::Diagnostic) -> Result<()> {
    use serde_json::json;

    let output = json!({
        "error": diagnostic.message,
        "code": diagnostic.code,
        "suggestion": diagnostic.help.clone().unwrap_or_else(|| "Run `lash index` to create the database".to_string()),
        "files": []
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output files as compact JSON
fn output_json_files(
    files: &[FileRecord],
    file_tasks: &HashMap<i64, Vec<TaskRecord>>,
    show_descriptions: bool,
    show_notes: bool,
) -> Result<()> {
    use serde_json::json;

    // Build JSON for each file
    let files_json: Vec<serde_json::Value> = files
        .iter()
        .map(|f| {
            let mut file_json = if show_descriptions {
                json!(f)
            } else {
                json!({
                    "id": f.id,
                    "path": f.path,
                    "file_id": f.file_id,
                    "title": f.title,
                    "hash": f.hash,
                    "mtime": f.mtime,
                    "status": f.status,
                    "metadata": f.metadata,
                    "indexed_at": f.indexed_at,
                })
            };

            // Add tasks with notes if show_notes is enabled
            if show_notes {
                if let Some(tasks) = file_tasks.get(&f.id) {
                    let tasks_with_notes: Vec<serde_json::Value> = tasks
                        .iter()
                        .filter(|t| !t.contextual_notes.is_empty())
                        .map(|t| {
                            json!({
                                "id": t.full_id,
                                "title": t.title,
                                "notes": t.contextual_notes.iter().map(lash_types::ContextualNote::text).collect::<Vec<_>>()
                            })
                        })
                        .collect();

                    if !tasks_with_notes.is_empty() {
                        if let serde_json::Value::Object(ref mut map) = file_json {
                            map.insert("tasks_with_notes".to_string(), json!(tasks_with_notes));
                        }
                    }
                }
            }

            file_json
        })
        .collect();

    let output = json!({
        "count": files.len(),
        "files": files_json
    });

    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

/// Output files as pretty-printed JSON
fn output_json_pretty_files(
    files: &[FileRecord],
    file_tasks: &HashMap<i64, Vec<TaskRecord>>,
    show_descriptions: bool,
    show_notes: bool,
) -> Result<()> {
    use serde_json::json;

    // Build JSON for each file
    let files_json: Vec<serde_json::Value> = files
        .iter()
        .map(|f| {
            let mut file_json = if show_descriptions {
                json!(f)
            } else {
                json!({
                    "id": f.id,
                    "path": f.path,
                    "file_id": f.file_id,
                    "title": f.title,
                    "hash": f.hash,
                    "mtime": f.mtime,
                    "status": f.status,
                    "metadata": f.metadata,
                    "indexed_at": f.indexed_at,
                })
            };

            // Add tasks with notes if show_notes is enabled
            if show_notes {
                if let Some(tasks) = file_tasks.get(&f.id) {
                    let tasks_with_notes: Vec<serde_json::Value> = tasks
                        .iter()
                        .filter(|t| !t.contextual_notes.is_empty())
                        .map(|t| {
                            json!({
                                "id": t.full_id,
                                "title": t.title,
                                "notes": t.contextual_notes.iter().map(lash_types::ContextualNote::text).collect::<Vec<_>>()
                            })
                        })
                        .collect();

                    if !tasks_with_notes.is_empty() {
                        if let serde_json::Value::Object(ref mut map) = file_json {
                            map.insert("tasks_with_notes".to_string(), json!(tasks_with_notes));
                        }
                    }
                }
            }

            file_json
        })
        .collect();

    let output = json!({
        "count": files.len(),
        "files": files_json
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output files as human-readable flat list
fn output_text_flat(
    files: &[FileRecord],
    file_tasks: &HashMap<i64, Vec<TaskRecord>>,
    theme: Option<&CliTheme>,
    show_descriptions: bool,
    show_notes: bool,
) {
    if files.is_empty() {
        let msg = "No files found";
        if let Some(theme) = theme {
            println!("{}", theme.style_warning(msg));
        } else {
            println!("{msg}");
        }
        println!();
        println!("Try running `lash index` to build the file index");
        return;
    }

    // Print each file
    for file in files {
        let status_indicator = match file.status {
            lash_types::FileStatus::Complete => "✓",
            lash_types::FileStatus::Blocked => "!",
            lash_types::FileStatus::InProgress => "○",
            lash_types::FileStatus::Empty => "·",
        };

        if let Some(theme) = theme {
            let status_styled = match file.status {
                lash_types::FileStatus::Complete => theme.style_success(status_indicator),
                lash_types::FileStatus::Blocked => theme.style_error(status_indicator),
                lash_types::FileStatus::InProgress => theme.style_info(status_indicator),
                lash_types::FileStatus::Empty => theme.style_muted(status_indicator),
            };
            let path = theme.style_info(&file.path.display().to_string());
            println!("{status_styled} {path}");
        } else {
            println!("{} {}", status_indicator, file.path.display());
        }

        // Show description if requested and available
        if show_descriptions && !file.description.is_empty() {
            let truncated = truncate_description(&file.description, 100);
            if let Some(theme) = theme {
                println!("  {}", theme.style_muted(&truncated));
            } else {
                println!("  {truncated}");
            }
        }

        // Show notes if requested
        if show_notes {
            if let Some(tasks) = file_tasks.get(&file.id) {
                for task in tasks {
                    if !task.contextual_notes.is_empty() {
                        // Show task title first
                        let task_indent = "  ";
                        if let Some(theme) = theme {
                            println!("{}{}", task_indent, theme.style_label(&task.title));
                        } else {
                            println!("{}{}", task_indent, task.title);
                        }

                        // Show each note with special marker
                        for note in &task.contextual_notes {
                            let note_indent = "    ";
                            let note_marker = "·";
                            if let Some(theme) = theme {
                                println!(
                                    "{}{} {}",
                                    note_indent,
                                    theme.style_muted(note_marker),
                                    theme.style_muted(note.text())
                                );
                            } else {
                                println!("{}{} {}", note_indent, note_marker, note.text());
                            }
                        }
                    }
                }
            }
        }
    }

    // Print summary
    println!();
    println!("Total: {} file(s)", files.len());
}

/// Output files and tasks as human-readable tree view
fn output_text_tree(
    files: &[FileRecord],
    file_tasks: &HashMap<i64, Vec<TaskRecord>>,
    args: &ListArgs,
) {
    if files.is_empty() {
        let msg = "No files found";
        if let Some(theme) = args.theme.as_ref() {
            println!("{}", theme.style_warning(msg));
        } else {
            println!("{msg}");
        }
        println!();
        println!("Try running `lash index` to build the file index");
        return;
    }

    // Get config for tree settings
    let config = lash_types::UserConfig::load().unwrap_or_default();
    let max_depth = args.max_depth.unwrap_or(config.tree_view.max_depth);
    // Always expand by default to show tasks in tree view
    let default_expanded = true;

    // Build task-aware tree
    let trees = build_task_tree(files, file_tasks, max_depth, default_expanded);

    // Create formatter
    let formatter = TreeFormatter::new(args.ascii, max_depth, args.theme.clone());

    // Format tree with task-aware rendering
    let lines = formatter.format_tree(&trees, |node, fmt| format_task_tree_node(node, fmt, args));

    // Print lines
    for line in lines {
        println!("{line}");
    }

    // Print summary
    println!();
    let total_tasks: usize = file_tasks.values().map(Vec::len).sum();
    println!("Total: {} file(s), {} task(s)", files.len(), total_tasks);
}

/// Format a single node in the task tree
#[allow(clippy::too_many_lines)]
fn format_task_tree_node(node: &TaskTreeNodeData, fmt: &TreeFormatter, args: &ListArgs) -> String {
    use std::fmt::Write;

    match node {
        TaskTreeNodeData::Directory { name, .. } => {
            if let Some(theme) = fmt.theme() {
                format!("{}/", theme.style_info(name))
            } else {
                format!("{name}/")
            }
        }
        TaskTreeNodeData::File { record } => {
            let file_name = record.path.file_name().map_or_else(
                || record.path.display().to_string(),
                |n| n.to_string_lossy().to_string(),
            );

            let status_indicator = match record.status {
                lash_types::FileStatus::Complete => "✓",
                lash_types::FileStatus::Blocked => "!",
                lash_types::FileStatus::InProgress => "○",
                lash_types::FileStatus::Empty => "·",
            };

            let mut output = if let Some(theme) = fmt.theme() {
                let status_styled = match record.status {
                    lash_types::FileStatus::Complete => theme.style_success(status_indicator),
                    lash_types::FileStatus::Blocked => theme.style_error(status_indicator),
                    lash_types::FileStatus::InProgress => theme.style_info(status_indicator),
                    lash_types::FileStatus::Empty => theme.style_muted(status_indicator),
                };
                let file_id = theme.style_muted(&format!(" ({})", record.file_id));
                format!(
                    "{} {}{}",
                    status_styled,
                    theme.style_label(&file_name),
                    file_id
                )
            } else {
                format!("{} {} ({})", status_indicator, file_name, record.file_id)
            };

            // Add description if requested and available
            if args.show_descriptions && !record.description.is_empty() {
                let truncated = truncate_description(&record.description, 80);
                if let Some(theme) = fmt.theme() {
                    let _ = write!(output, "\n    {}", theme.style_muted(&truncated));
                } else {
                    let _ = write!(output, "\n    {truncated}");
                }
            }

            output
        }
        TaskTreeNodeData::Task { record, .. } => {
            let checkbox = match record.status {
                lash_types::TaskStatus::Open => "[ ]",
                lash_types::TaskStatus::InProgress => "[>]",
                lash_types::TaskStatus::Done => "[x]",
                lash_types::TaskStatus::Waived => "[-]",
                lash_types::TaskStatus::Blocked => "[!]",
            };

            let mut parts = Vec::new();

            // Status checkbox
            if let Some(theme) = fmt.theme() {
                let styled_checkbox = match record.status {
                    lash_types::TaskStatus::Open => theme.style_muted(checkbox),
                    lash_types::TaskStatus::InProgress => {
                        theme.style_task_status(checkbox, record.status)
                    }
                    lash_types::TaskStatus::Done => theme.style_success(checkbox),
                    lash_types::TaskStatus::Waived => theme.style_warning(checkbox),
                    lash_types::TaskStatus::Blocked => theme.style_error(checkbox),
                };
                parts.push(styled_checkbox);
            } else {
                parts.push(checkbox.to_string());
            }

            // Title
            parts.push(record.title.clone());

            // Task ID (muted)
            let task_id = format!("({})", record.full_id);
            if let Some(theme) = fmt.theme() {
                parts.push(theme.style_muted(&task_id));
            } else {
                parts.push(task_id);
            }

            let mut output = parts.join(" ");

            // Add metadata on same line if present
            let mut metadata_parts = Vec::new();

            if let Some(ref owner) = record.owner {
                metadata_parts.push(format!("@{owner}"));
            }

            if let Some(ref estimate) = record.estimate {
                metadata_parts.push(format!("~{estimate}"));
            }

            if !record.metadata.labels.is_empty() {
                let labels = record
                    .metadata
                    .labels
                    .iter()
                    .map(|l| format!("#{l}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                metadata_parts.push(labels);
            }

            if !metadata_parts.is_empty() {
                let metadata_str = metadata_parts.join(" ");
                if let Some(theme) = fmt.theme() {
                    let _ = write!(output, " {}", theme.style_muted(&metadata_str));
                } else {
                    let _ = write!(output, " {metadata_str}");
                }
            }

            // Add contextual notes if show_notes is enabled
            if args.show_notes && !record.contextual_notes.is_empty() {
                for note in &record.contextual_notes {
                    if let Some(theme) = fmt.theme() {
                        let _ = write!(
                            output,
                            "\n    {}",
                            theme.style_muted(&format!("· {}", note.text()))
                        );
                    } else {
                        let _ = write!(output, "\n    · {}", note.text());
                    }
                }
            }

            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_tree_view_enabled_default() {
        let args = ListArgs {
            filter: None,
            labels: vec![],
            status: None,
            path: None,
            blocked: false,
            owner: None,
            docs: None,
            show_descriptions: false,
            show_notes: false,
            limit: None,
            format: OutputFormat::Text,
            project_root: None,
            theme: None,
            tree_view: None,
            max_depth: None,
            ascii: false,
            verbosity: Verbosity::Normal,
        };

        // Should default to true if no config exists
        let result = determine_tree_view_enabled(&args);
        assert!(result); // Defaults to true
    }

    #[test]
    fn test_determine_tree_view_enabled_with_flag() {
        let args = ListArgs {
            filter: None,
            labels: vec![],
            status: None,
            path: None,
            blocked: false,
            owner: None,
            docs: None,
            show_descriptions: false,
            show_notes: false,
            limit: None,
            format: OutputFormat::Text,
            project_root: None,
            theme: None,
            tree_view: Some(false),
            max_depth: None,
            ascii: false,
            verbosity: Verbosity::Normal,
        };

        assert!(!determine_tree_view_enabled(&args));
    }

    #[test]
    fn test_build_task_tree_empty() {
        let files = vec![];
        let file_tasks = HashMap::new();
        let trees = build_task_tree(&files, &file_tasks, 5, false);
        assert!(trees.is_empty());
    }

    #[test]
    fn test_truncate_description_empty() {
        let result = truncate_description("", 100);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_description_short() {
        let desc = "This is a short description.";
        let result = truncate_description(desc, 100);
        assert_eq!(result, desc);
    }

    #[test]
    fn test_truncate_description_long() {
        let desc = "a".repeat(150);
        let result = truncate_description(&desc, 100);
        assert_eq!(result.len(), 103); // 100 chars + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_description_with_newlines() {
        let desc = "Line one\nLine two\nLine three";
        let result = truncate_description(desc, 100);
        assert_eq!(result, "Line one Line two Line three");
        assert!(!result.contains('\n'));
    }

    #[test]
    fn test_truncate_description_with_multiple_spaces() {
        let desc = "Too    many     spaces";
        let result = truncate_description(desc, 100);
        assert_eq!(result, "Too many spaces");
    }

    #[test]
    fn test_truncate_description_exact_length() {
        let desc = "a".repeat(100);
        let result = truncate_description(&desc, 100);
        assert_eq!(result, desc);
        assert!(!result.ends_with("..."));
    }
}
