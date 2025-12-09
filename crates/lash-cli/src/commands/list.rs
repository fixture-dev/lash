//! List command implementation
//!
//! The `lash list` command queries and filters tasks from the `SQLite` database.

use anyhow::{Context, Result};
use lash_cli::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash_cli::formatter::{OutputFormat as OutputFormatTrait, Verbosity};
use lash_cli::theme::CliTheme;
use lash_cli::tree_formatter::TreeFormatter;
use lash_db::repository::files::FileRecord;
use lash_db::{open_database, DocRefRepository, FileRepository};
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
#[allow(dead_code)]
pub struct ListArgs {
    /// Filter by label (can be specified multiple times) - currently unused in file view
    pub labels: Vec<String>,
    /// Filter by status - currently unused in file view
    pub status: Option<lash_types::TaskStatus>,
    /// Filter by path prefix - currently unused in file view
    pub path: Option<PathBuf>,
    /// Only show blocked tasks - currently unused in file view
    pub blocked: bool,
    /// Filter by owner - currently unused in file view
    pub owner: Option<String>,
    /// Filter by files/tasks that reference a specific document
    pub docs: Option<String>,
    /// Show file descriptions (truncated to 100 chars)
    pub show_descriptions: bool,
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
    let doc_repo = DocRefRepository::new(&conn);

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
            if args.format == OutputFormat::Json || args.format == OutputFormat::JsonPretty {
                output_json_error(&error)?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormatTrait::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: args.theme.clone(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_error(&error);
            }
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
                if args.format == OutputFormat::Json || args.format == OutputFormat::JsonPretty {
                    output_json_error(&error)?;
                } else {
                    let reporter_config = ErrorReporterConfig {
                        verbosity: args.verbosity,
                        output_format: OutputFormatTrait::Text,
                        display_mode: ErrorDisplayMode::Streaming,
                        theme: args.theme.clone(),
                        show_summary: false,
                    };
                    let mut reporter = ErrorReporter::new(reporter_config);
                    reporter.report_error(&error);
                }
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

    // Output results
    match args.format {
        OutputFormat::Json => output_json_files(&files, args.show_descriptions)?,
        OutputFormat::JsonPretty => output_json_pretty_files(&files, args.show_descriptions)?,
        OutputFormat::Text => {
            if use_tree_view {
                output_text_tree(&files, &args);
            } else {
                output_text_flat(&files, args.theme.as_ref(), args.show_descriptions);
            }
        }
    }

    Ok(0)
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

/// Represents a directory or file node in the file tree
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DirectoryNode {
    /// Display name (directory name or file name)
    name: String,
    /// Full path
    path: PathBuf,
    /// true for directory, false for file
    is_directory: bool,
    /// For files, the underlying `FileRecord`
    file_record: Option<FileRecord>,
}

/// Build file tree from flat file list
fn build_file_tree(
    files: &[FileRecord],
    max_depth: usize,
    default_expanded: bool,
) -> Vec<TreeNode<DirectoryNode>> {
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
    let mut roots: Vec<TreeNode<DirectoryNode>> = Vec::new();
    let mut dir_nodes: HashMap<PathBuf, TreeNode<DirectoryNode>> = HashMap::new();

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
                DirectoryNode {
                    name: dir_name,
                    path: dir_path.clone(),
                    is_directory: true,
                    file_record: None,
                },
                depth,
            );

            if default_expanded && depth < max_depth {
                dir_node.expand();
            }

            dir_nodes.insert(dir_path.clone(), dir_node);
        }

        // Add file nodes to directory
        for file in dir_files {
            let file_name = file
                .path
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(""))
                .to_string_lossy()
                .to_string();

            let file_node = TreeNode::new(
                DirectoryNode {
                    name: file_name,
                    path: file.path.clone(),
                    is_directory: false,
                    file_record: Some(file.clone()),
                },
                depth + 1,
            );

            // Add to parent directory or root
            if depth == 0 {
                roots.push(file_node);
            } else if let Some(parent) = dir_nodes.get_mut(dir_path) {
                parent.children.push(file_node);
            }
        }
    }

    // Build parent-child relationships for directories
    // Sort for deterministic order
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

    // Sort roots by name for deterministic order
    roots.sort_by(|a, b| a.data.name.cmp(&b.data.name));
    roots
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
fn output_json_files(files: &[FileRecord], show_descriptions: bool) -> Result<()> {
    use serde_json::json;

    // Optionally filter out descriptions from the JSON output
    let files_json: Vec<serde_json::Value> = if show_descriptions {
        // Include descriptions in JSON output
        files.iter().map(|f| json!(f)).collect()
    } else {
        // Exclude descriptions from JSON output - create custom JSON without description field
        files
            .iter()
            .map(|f| {
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
            })
            .collect()
    };

    let output = json!({
        "count": files.len(),
        "files": files_json
    });

    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

/// Output files as pretty-printed JSON
fn output_json_pretty_files(files: &[FileRecord], show_descriptions: bool) -> Result<()> {
    use serde_json::json;

    // Optionally filter out descriptions from the JSON output
    let files_json: Vec<serde_json::Value> = if show_descriptions {
        // Include descriptions in JSON output
        files.iter().map(|f| json!(f)).collect()
    } else {
        // Exclude descriptions from JSON output - create custom JSON without description field
        files
            .iter()
            .map(|f| {
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
            })
            .collect()
    };

    let output = json!({
        "count": files.len(),
        "files": files_json
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output files as human-readable flat list
fn output_text_flat(files: &[FileRecord], theme: Option<&CliTheme>, show_descriptions: bool) {
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
    }

    // Print summary
    println!();
    println!("Total: {} file(s)", files.len());
}

/// Output files as human-readable tree view
fn output_text_tree(files: &[FileRecord], args: &ListArgs) {
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
    let default_expanded = config.tree_view.default_expanded;

    // Build tree
    let trees = build_file_tree(files, max_depth, default_expanded);

    // Create formatter
    let formatter = TreeFormatter::new(args.ascii, max_depth, args.theme.clone());

    // Format tree
    let lines = formatter.format_tree(&trees, |node, fmt| {
        if node.is_directory {
            // Directory node
            if let Some(theme) = fmt.theme() {
                format!("{}/", theme.style_info(&node.name))
            } else {
                format!("{}/", node.name)
            }
        } else if let Some(file) = &node.file_record {
            // File node
            let status_indicator = match file.status {
                lash_types::FileStatus::Complete => "✓",
                lash_types::FileStatus::Blocked => "!",
                lash_types::FileStatus::InProgress => "○",
                lash_types::FileStatus::Empty => "·",
            };

            let mut output = if let Some(theme) = fmt.theme() {
                let status_styled = match file.status {
                    lash_types::FileStatus::Complete => theme.style_success(status_indicator),
                    lash_types::FileStatus::Blocked => theme.style_error(status_indicator),
                    lash_types::FileStatus::InProgress => theme.style_info(status_indicator),
                    lash_types::FileStatus::Empty => theme.style_muted(status_indicator),
                };
                format!("{status_styled} {}", theme.style_label(&node.name))
            } else {
                format!("{status_indicator} {}", node.name)
            };

            // Add description if requested and available
            if args.show_descriptions && !file.description.is_empty() {
                let truncated = truncate_description(&file.description, 100);
                if let Some(theme) = fmt.theme() {
                    output.push('\n');
                    output.push_str("    ");
                    output.push_str(&theme.style_muted(&truncated));
                } else {
                    output.push_str("\n    ");
                    output.push_str(&truncated);
                }
            }

            output
        } else {
            // Fallback for nodes without file records
            node.name.clone()
        }
    });

    // Print lines
    for line in lines {
        println!("{line}");
    }

    // Print summary
    println!();
    println!("Total: {} file(s)", files.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_tree_view_enabled_default() {
        let args = ListArgs {
            labels: vec![],
            status: None,
            path: None,
            blocked: false,
            owner: None,
            docs: None,
            show_descriptions: false,
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
            labels: vec![],
            status: None,
            path: None,
            blocked: false,
            owner: None,
            docs: None,
            show_descriptions: false,
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
    fn test_build_file_tree_empty() {
        let files = vec![];
        let trees = build_file_tree(&files, 5, false);
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
