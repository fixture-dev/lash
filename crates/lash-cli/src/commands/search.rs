//! Search command implementation
//!
//! The `lash search` command provides full-text search across all tasks and files in the index.
//!
//! ## Implementation
//!
//! This command uses the FTS5 (Full-Text Search) infrastructure from `lash-db` to:
//! - Search across task titles, bodies, labels, and file paths
//! - Rank results by relevance score
//! - Display matching terms and context snippets
//! - Limit results with configurable page size
//!
//! The search uses `SQLite`'s FTS5 virtual table for efficient full-text indexing and retrieval.

use anyhow::{Context, Result};
use lash::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash::formatter::{OutputFormat, TextFormatter, Verbosity};
use lash::theme::{self, CliTheme};
use lash::tree_formatter::TreeFormatter;
use lash_db::{open_database, search, SearchQuery};
use lash_types::error::LashError;
use lash_types::tree::TreeNode;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

// Re-export SearchResult from lash_db for consistency
pub use lash_db::SearchResult;

/// Arguments for the search command
#[derive(Debug, Clone)]
pub struct SearchArgs {
    /// Search query string
    pub query: String,
    /// Maximum number of results to return
    pub limit: usize,
    /// Output in JSON format
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Filter by labels (can specify multiple)
    pub labels: Vec<String>,
    /// Filter by status
    pub status: Option<lash_types::TaskStatus>,
    /// Filter by owner
    pub owner: Option<String>,
    /// Filter by path scope
    pub path: Option<PathBuf>,
    /// Optional color scheme name to use for styling
    pub color_scheme: Option<String>,
    /// Enable tree view (None = use config default)
    pub tree_view: Option<bool>,
    /// Maximum tree depth
    pub max_depth: Option<usize>,
    /// Use ASCII characters for tree
    pub ascii: bool,
    /// Verbosity level for output
    pub verbosity: Verbosity,
}

// SearchResult is re-exported from lash_db above

/// Execute the search command
///
/// # Arguments
///
/// * `args` - Search command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (general error), 3 (DB error)
///
/// # Errors
///
/// Returns an error if:
/// - Project root cannot be found
/// - Database does not exist or cannot be opened
/// - Search query execution fails
#[allow(clippy::too_many_lines)]
pub fn execute(args: &SearchArgs) -> Result<i32> {
    // Determine project root
    let project_root = if let Some(ref root) = args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        query = %args.query,
        limit = args.limit,
        "Starting search operation"
    );

    // Determine database path
    let db_path = get_database_path(&project_root);

    // Load theme for colored output
    let colors_enabled = !args.no_color && theme::supports_color();
    let theme = CliTheme::load(args.color_scheme.as_deref(), colors_enabled)?;

    // Check if database exists
    if !db_path.exists() {
        let error = LashError::io_file_not_found(db_path);
        let mut diag = error.to_diagnostic();
        diag.help = Some("Run `lash index` to create the database".to_string());

        if args.json {
            output_json_diagnostic(&diag)?;
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
                output_json_diagnostic(&diag)?;
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

    // Build search query with filters
    let mut query = SearchQuery::new(&args.query)
        .with_limit(args.limit)
        .with_offset(0);

    // Apply label filters
    for label in &args.labels {
        query = query.with_label(label.clone());
    }

    // Apply status filter
    if let Some(status) = args.status {
        query = query.with_status(status);
    }

    // Apply owner filter
    if let Some(ref owner) = args.owner {
        query = query.with_owner(owner.clone());
    }

    // Apply path scope filter
    if let Some(ref path) = args.path {
        query = query.with_scope(path.clone());
    }

    // Execute search
    let results = match search(&conn, &query) {
        Ok(results) => results,
        Err(e) => {
            let error = LashError::internal(
                format!("Search query failed: {e}"),
                Some("search".to_string()),
            );
            if args.json {
                output_json_error_structured(&error)?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormat::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: theme.clone(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_error(&error);
            }
            return Ok(3); // Exit code 3 for DB error
        }
    };

    tracing::debug!(
        result_count = results.results.len(),
        total_matches = results.total_count,
        "Search completed"
    );

    // Output results
    if args.json {
        output_json(&results.results, &args.query)?;
    } else {
        let formatter = TextFormatter::with_theme(theme.clone(), args.verbosity);

        // Determine if tree view is enabled
        let use_tree_view = determine_tree_view_enabled(args);

        if use_tree_view {
            output_text_tree(&results.results, &args.query, args, theme);
        } else {
            output_text(&results.results, &args.query, &formatter);
        }
    }

    // Return success - "no results" is a successful search, not an error
    Ok(0)
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/lash.db")
}

/// Determine if tree view should be enabled
///
/// Priority: CLI flag > config > default (true)
fn determine_tree_view_enabled(args: &SearchArgs) -> bool {
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

/// Represents a directory or result node in the search tree
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SearchTreeNode {
    /// Display name (directory name or result title)
    name: String,
    /// Full path for directories
    path: PathBuf,
    /// true for directory, false for result
    is_directory: bool,
    /// For results, the underlying `SearchResult`
    result: Option<SearchResult>,
}

/// Build search tree from flat results list
///
/// Groups search results by their file path in a hierarchical tree structure.
#[allow(clippy::too_many_lines)]
fn build_search_tree(
    results: &[SearchResult],
    max_depth: usize,
    default_expanded: bool,
) -> Vec<TreeNode<SearchTreeNode>> {
    if results.is_empty() {
        return Vec::new();
    }

    // Group results by directory
    let mut dir_map: HashMap<PathBuf, Vec<&SearchResult>> = HashMap::new();
    for result in results {
        let path = PathBuf::from(&result.file_path);
        let dir = path.parent().unwrap_or_else(|| Path::new(""));
        dir_map.entry(dir.to_path_buf()).or_default().push(result);
    }

    // Build tree structure
    let mut roots: Vec<TreeNode<SearchTreeNode>> = Vec::new();
    let mut dir_nodes: HashMap<PathBuf, TreeNode<SearchTreeNode>> = HashMap::new();

    // Sort directories by path for consistent ordering
    let mut sorted_dirs: Vec<_> = dir_map.keys().collect();
    sorted_dirs.sort();

    for dir_path in sorted_dirs {
        let dir_results = dir_map.get(dir_path).unwrap();
        let depth = dir_path.components().count();

        // Create directory node if it doesn't exist
        if !dir_nodes.contains_key(dir_path) && depth > 0 {
            let dir_name = dir_path
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(""))
                .to_string_lossy()
                .to_string();

            let mut dir_node = TreeNode::new(
                SearchTreeNode {
                    name: dir_name,
                    path: dir_path.clone(),
                    is_directory: true,
                    result: None,
                },
                depth,
            );

            if default_expanded && depth < max_depth {
                dir_node.expand();
            }

            dir_nodes.insert(dir_path.clone(), dir_node);
        }

        // Group results by file path within this directory
        let mut file_results: HashMap<PathBuf, Vec<&SearchResult>> = HashMap::new();
        for result in dir_results {
            let file_path = PathBuf::from(&result.file_path);
            file_results.entry(file_path).or_default().push(result);
        }

        // Sort file paths for deterministic order
        let mut sorted_files: Vec<_> = file_results.keys().collect();
        sorted_files.sort();

        for file_path in sorted_files {
            let results_for_file = file_results.get(file_path).unwrap();
            let file_name = file_path
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(""))
                .to_string_lossy()
                .to_string();

            // Create file node as container for results
            let mut file_node = TreeNode::new(
                SearchTreeNode {
                    name: file_name,
                    path: file_path.clone(),
                    is_directory: true, // Treat files as containers
                    result: None,
                },
                depth + 1,
            );

            if default_expanded && (depth + 1) < max_depth {
                file_node.expand();
            }

            // Add result nodes as children
            for result in results_for_file {
                let result_node = TreeNode::new(
                    SearchTreeNode {
                        name: result.title.clone(),
                        path: file_path.clone(),
                        is_directory: false,
                        result: Some((*result).clone()),
                    },
                    depth + 2,
                );
                file_node.children.push(result_node);
            }

            // Add file node to parent directory or root
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

    // Sort roots by name for deterministic order
    roots.sort_by(|a, b| a.data.name.cmp(&b.data.name));
    roots
}

/// Output search results as tree view
#[allow(clippy::needless_pass_by_value)]
fn output_text_tree(
    results: &[SearchResult],
    query: &str,
    args: &SearchArgs,
    theme: Option<CliTheme>,
) {
    let has_theme = theme.is_some();

    if results.is_empty() {
        let no_results = format!("No results found for '{query}'");
        if let Some(ref theme) = theme {
            println!("{}", theme.style_warning(&no_results));
        } else {
            println!("{no_results}");
        }
        println!();
        println!("Suggestions:");
        println!("  - Try a different query");
        println!("  - Check that your files are indexed with `lash index`");
        return;
    }

    // Print header
    if has_theme {
        if let Some(ref theme) = theme {
            println!(
                "{} {} {}",
                theme.style_info("Found"),
                theme.style_info(&results.len().to_string()),
                theme.style_info(&format!("result(s) for '{query}'"))
            );
        } else {
            println!("Found {} result(s) for '{}'", results.len(), query);
        }
    } else {
        println!("Found {} result(s) for '{}'", results.len(), query);
    }
    println!();

    // Get config for tree settings
    let config = lash_types::UserConfig::load().unwrap_or_default();
    let max_depth = args.max_depth.unwrap_or(config.tree_view.max_depth);
    // For search results, always expand nodes by default so results are visible
    let default_expanded = true;

    // Build tree
    let trees = build_search_tree(results, max_depth, default_expanded);

    // Create formatter
    let formatter = TreeFormatter::new(args.ascii, max_depth, theme.clone());

    // Format tree
    let lines = formatter.format_tree(&trees, |node, fmt| {
        if node.is_directory {
            // Directory or file node
            if let Some(theme) = fmt.theme() {
                format!("{}/", theme.style_info(&node.name))
            } else {
                format!("{}/", node.name)
            }
        } else if let Some(ref result) = node.result {
            // Search result node
            let status_indicator = match result.status {
                lash_types::TaskStatus::Open => "[ ]",
                lash_types::TaskStatus::InProgress => "[>]",
                lash_types::TaskStatus::Done => "[x]",
                lash_types::TaskStatus::Waived => "[-]",
                lash_types::TaskStatus::Blocked => "[!]",
            };

            if let Some(theme) = fmt.theme() {
                let status_styled = match result.status {
                    lash_types::TaskStatus::Done => theme.style_success(status_indicator),
                    lash_types::TaskStatus::Blocked => theme.style_error(status_indicator),
                    lash_types::TaskStatus::InProgress => {
                        theme.style_task_status(status_indicator, result.status)
                    }
                    lash_types::TaskStatus::Open | lash_types::TaskStatus::Waived => {
                        theme.style_muted(status_indicator)
                    }
                };
                let score_str = format!("({:.2})", result.score);
                format!(
                    "{} {} {}",
                    status_styled,
                    theme.style_label(&node.name),
                    theme.style_muted(&score_str)
                )
            } else {
                format!("{} {} ({:.2})", status_indicator, node.name, result.score)
            }
        } else {
            // Fallback
            node.name.clone()
        }
    });

    // Print lines
    for line in lines {
        println!("{line}");
    }
}

/// Output error as JSON using `LashError`
fn output_json_error_structured(error: &LashError) -> Result<()> {
    let diagnostic = error.to_diagnostic();
    output_json_diagnostic(&diagnostic)
}

/// Output diagnostic as JSON
fn output_json_diagnostic(diagnostic: &lash_types::error::Diagnostic) -> Result<()> {
    let output = json!({
        "error": diagnostic.message,
        "code": diagnostic.code,
        "suggestion": diagnostic.help.clone().unwrap_or_else(|| "Run `lash index` to create the database".to_string()),
        "results": []
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output search results as JSON
fn output_json(results: &[SearchResult], query: &str) -> Result<()> {
    let output = json!({
        "query": query,
        "count": results.len(),
        "results": results
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output search results as human-readable text
fn output_text(results: &[SearchResult], query: &str, formatter: &TextFormatter) {
    let has_theme = formatter.has_color();

    if results.is_empty() {
        let no_results = format!("No results found for '{query}'");
        if has_theme {
            if let Some(theme) = formatter.theme() {
                println!("{}", theme.style_warning(&no_results));
            } else {
                println!("{no_results}");
            }
        } else {
            println!("{no_results}");
        }
        println!();
        println!("Suggestions:");
        println!("  - Try a different query");
        println!("  - Check that your files are indexed with `lash index`");
        return;
    }

    // Print header
    if has_theme {
        if let Some(theme) = formatter.theme() {
            println!(
                "{} {} {}",
                theme.style_info("Found"),
                theme.style_info(&results.len().to_string()),
                theme.style_info(&format!("result(s) for '{query}'"))
            );
        } else {
            println!("Found {} result(s) for '{}'", results.len(), query);
        }
    } else {
        println!("Found {} result(s) for '{}'", results.len(), query);
    }
    println!();

    // Print each result
    for (i, result) in results.iter().enumerate() {
        if has_theme {
            if let Some(theme) = formatter.theme() {
                // ID and score
                println!(
                    "{}. {} {} {}",
                    formatter.format_muted(&(i + 1).to_string()),
                    theme.style_info(&result.full_id),
                    formatter.format_muted(&format!("(score: {:.2})", result.score)),
                    format_matched_fields(&result.matched_fields, formatter)
                );

                // Title - use success color to emphasize it
                println!("   {}", theme.style_success(&result.title));

                // File location - use info color for paths
                println!(
                    "   {} {}",
                    formatter.format_muted("└─"),
                    theme.style_info(&result.file_path)
                );

                // Snippet (if present and different from title)
                if !result.snippet.is_empty() && result.snippet != result.title {
                    println!("   {}", formatter.format_muted(&result.snippet));
                }

                // Labels (if present)
                if !result.labels.is_empty() {
                    println!("      {}", format_labels(&result.labels, formatter));
                }
            }
        } else {
            // No color version
            println!(
                "{}. {} (score: {:.2}) {}",
                i + 1,
                result.full_id,
                result.score,
                format_matched_fields(&result.matched_fields, formatter)
            );
            println!("   {}", result.title);
            println!("   └─ {}", result.file_path);

            if !result.snippet.is_empty() && result.snippet != result.title {
                println!("   {}", result.snippet);
            }

            if !result.labels.is_empty() {
                println!("      {}", format_labels(&result.labels, formatter));
            }
        }

        println!();
    }
}

/// Format matched fields for display
fn format_matched_fields(fields: &[String], formatter: &TextFormatter) -> String {
    if fields.is_empty() {
        return String::new();
    }

    // Replace "contextual_notes" with a more user-friendly label
    let display_fields: Vec<String> = fields
        .iter()
        .map(|f| {
            if f == "contextual_notes" {
                "note".to_string()
            } else {
                f.clone()
            }
        })
        .collect();

    let fields_str = format!("[{}]", display_fields.join(", "));
    formatter.format_muted(&fields_str)
}

/// Format labels for display
fn format_labels(labels: &[String], formatter: &TextFormatter) -> String {
    if labels.is_empty() {
        return String::new();
    }

    let labels_str = labels
        .iter()
        .map(|l| {
            let label_text = format!("#{l}");
            formatter.format_label(&label_text)
        })
        .collect::<Vec<_>>()
        .join(" ");

    labels_str
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_color_formatter() -> TextFormatter {
        TextFormatter::with_theme(None, Verbosity::Normal)
    }

    #[test]
    fn test_format_matched_fields() {
        let formatter = no_color_formatter();
        assert_eq!(format_matched_fields(&[], &formatter), "");
        assert_eq!(
            format_matched_fields(&["title".to_string()], &formatter),
            "[title]"
        );
        assert_eq!(
            format_matched_fields(&["title".to_string(), "body".to_string()], &formatter),
            "[title, body]"
        );
        // Test that contextual_notes is displayed as "note"
        assert_eq!(
            format_matched_fields(&["contextual_notes".to_string()], &formatter),
            "[note]"
        );
        assert_eq!(
            format_matched_fields(
                &["title".to_string(), "contextual_notes".to_string()],
                &formatter
            ),
            "[title, note]"
        );
    }

    #[test]
    fn test_format_labels() {
        let formatter = no_color_formatter();
        assert_eq!(format_labels(&[], &formatter), "");
        assert_eq!(
            format_labels(&["backend".to_string()], &formatter),
            "#backend"
        );
        assert_eq!(
            format_labels(&["backend".to_string(), "api".to_string()], &formatter),
            "#backend #api"
        );
    }
}
