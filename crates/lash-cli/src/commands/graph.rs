//! Graph command implementation
//!
//! The `lash graph` command exports dependency graphs in various formats.

use anyhow::{Context, Result};
use lash_cli::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash_cli::formatter::{OutputFormat, Verbosity};
use lash_cli::theme::CliTheme;
use lash_core::dependency::{FilterOptions, GraphExporter};
use lash_db::{graph_builder::GraphBuilder, open_database};
use lash_types::error::LashError;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

/// Output format for graph export
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphFormat {
    Dot,
    Mermaid,
    Json,
}

/// Arguments for the graph command
#[derive(Debug, Clone)]
pub struct GraphArgs {
    /// Output format
    pub format: GraphFormat,
    /// Scope to specific path or label
    pub scope: Option<String>,
    /// Hide completed tasks
    pub hide_completed: bool,
    /// Output file (defaults to stdout)
    pub output: Option<PathBuf>,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Optional CLI theme for styling
    pub theme: Option<CliTheme>,
    /// Verbosity level for output
    pub verbosity: Verbosity,
}

/// Execute the graph command
///
/// # Arguments
///
/// * `args` - Graph command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (general error), 3 (DB error)
pub fn execute(args: &GraphArgs) -> Result<i32> {
    // Determine project root
    let project_root = if let Some(ref root) = args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        format = ?args.format,
        scope = ?args.scope,
        "Starting graph export operation"
    );

    // Determine database path
    let db_path = get_database_path(&project_root);

    // Create error reporter for consistent error reporting
    let reporter_config = ErrorReporterConfig {
        verbosity: args.verbosity,
        output_format: OutputFormat::Text,
        display_mode: ErrorDisplayMode::Streaming,
        theme: args.theme.clone(),
        show_summary: false,
    };
    let mut reporter = ErrorReporter::new(reporter_config);

    // Check if database exists
    if !db_path.exists() {
        let error = LashError::index_out_of_sync(0)
            .to_diagnostic()
            .with_help(format!(
                "Database not found at {}. Run `lash index` to create the database.",
                db_path.display()
            ));
        reporter.report_diagnostic(&error);
        return Ok(3); // Exit code 3 for DB error
    }

    // Open database
    let conn = match open_database(&db_path) {
        Ok(conn) => conn,
        Err(e) => {
            let error = LashError::index_corrupted(format!("Failed to open database: {e}"));
            reporter.report_error(&error);
            return Ok(3);
        }
    };

    // Build dependency graph from database
    let builder = GraphBuilder::new(&conn);
    let graph = match builder.build() {
        Ok(graph) => graph,
        Err(e) => {
            let error = LashError::internal(format!("Failed to build dependency graph: {e}"), None);
            reporter.report_error(&error);
            return Ok(1);
        }
    };

    tracing::debug!(
        node_count = graph.node_count(),
        edge_count = graph.edge_count(),
        "Built dependency graph"
    );

    // Build filter options
    let filter_options = build_filter_options(args);

    // Export graph in requested format
    let exporter = GraphExporter::new(&graph);
    let output_text = match args.format {
        GraphFormat::Dot => exporter.to_dot(&filter_options),
        GraphFormat::Mermaid => export_mermaid(&exporter, &filter_options),
        GraphFormat::Json => exporter.to_json(&filter_options),
    };

    // Write output to file or stdout
    if let Some(output_path) = &args.output {
        if let Err(e) = fs::write(output_path, &output_text) {
            let error = LashError::io_write_error(
                output_path.clone(),
                format!("Failed to write graph: {e}"),
            );
            reporter.report_error(&error);
            return Ok(1);
        }
        tracing::info!(path = %output_path.display(), "Graph exported to file");
    } else {
        print!("{output_text}");
    }

    Ok(0)
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/lash.db")
}

/// Build filter options from command arguments
fn build_filter_options(args: &GraphArgs) -> FilterOptions {
    let mut options = FilterOptions {
        files: None,
        labels: None,
        hide_completed: args.hide_completed,
        max_depth: None,
    };

    // Parse scope option
    if let Some(ref scope) = args.scope {
        // Try to determine if scope is a file path or a label
        if scope.contains('/')
            || std::path::Path::new(scope)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            // Treat as file path
            options.files = Some(vec![scope.clone()]);
        } else {
            // Treat as label
            options.labels = Some(vec![scope.clone()]);
        }
    }

    options
}

/// Export graph in Mermaid format
///
/// Mermaid uses a simple text-based syntax for graphs:
/// - `graph TD` for top-down layout
/// - Nodes: `NodeID[Node Title]`
/// - Edges: `A --> B`
/// - Styling: `style NodeID fill:#color`
fn export_mermaid(exporter: &GraphExporter, options: &FilterOptions) -> String {
    use std::collections::HashSet;
    use std::fmt::Write;

    let mut output = String::new();

    // Mermaid header
    output.push_str("graph TD\n");

    // Get the graph reference from exporter
    // Note: We need access to the graph, but GraphExporter doesn't expose it directly.
    // For now, we'll use the JSON export and parse it to generate Mermaid.
    // This is not the most efficient, but it works without changing the exporter API.

    let json_str = exporter.to_json(options);
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();

    let nodes = json["nodes"].as_array();
    let edges = json["edges"].as_array();

    // Track which nodes we've seen
    let mut seen_nodes = HashSet::new();

    // Output nodes
    if let Some(nodes) = nodes {
        for node in nodes {
            if let (Some(id), Some(title), Some(status)) = (
                node["id"].as_str(),
                node["title"].as_str(),
                node["status"].as_str(),
            ) {
                let escaped_id = escape_mermaid_id(id);
                let escaped_title = escape_mermaid_label(title);

                // Mermaid node syntax: ID[Title]
                writeln!(output, "  {escaped_id}[\"{escaped_title}\"]").ok();
                seen_nodes.insert(id.to_string());

                // Add styling based on status
                let color = match status {
                    "Done" => "#90EE90",    // lightgreen
                    "Open" => "#FFFFE0",    // lightyellow
                    "Waived" => "#D3D3D3",  // lightgray
                    "Blocked" => "#F08080", // lightcoral
                    _ => "#FFFFFF",         // white
                };
                writeln!(output, "  style {escaped_id} fill:{color}").ok();
            }
        }
    }

    // Output edges
    if let Some(edges) = edges {
        for edge in edges {
            if let (Some(from), Some(to)) = (edge["from"].as_str(), edge["to"].as_str()) {
                // Only output edge if both nodes exist
                if seen_nodes.contains(from) && seen_nodes.contains(to) {
                    let escaped_from = escape_mermaid_id(from);
                    let escaped_to = escape_mermaid_id(to);

                    // Mermaid edge syntax: A --> B
                    writeln!(output, "  {escaped_from} --> {escaped_to}").ok();
                }
            }
        }
    }

    output
}

/// Escape special characters in Mermaid node IDs
///
/// Mermaid IDs can't contain special characters like #, /, etc.
/// We'll replace them with underscores.
fn escape_mermaid_id(s: &str) -> String {
    s.replace(['#', '/', '.', '-', ':', ' '], "_")
}

/// Escape special characters in Mermaid labels
fn escape_mermaid_label(s: &str) -> String {
    s.replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_mermaid_id() {
        assert_eq!(escape_mermaid_id("test#task1"), "test_task1");
        assert_eq!(escape_mermaid_id("file/path#task"), "file_path_task");
        assert_eq!(escape_mermaid_id("task-with-dashes"), "task_with_dashes");
    }

    #[test]
    fn test_escape_mermaid_label() {
        assert_eq!(
            escape_mermaid_label("Task with \"quotes\""),
            "Task with &quot;quotes&quot;"
        );
        assert_eq!(escape_mermaid_label("Task <tag>"), "Task &lt;tag&gt;");
    }

    #[test]
    fn test_build_filter_options_file_scope() {
        let args = GraphArgs {
            format: GraphFormat::Dot,
            scope: Some("path/to/file.md".to_string()),
            hide_completed: false,
            output: None,
            project_root: None,
            theme: None,
            verbosity: Verbosity::Normal,
        };

        let options = build_filter_options(&args);
        assert_eq!(options.files, Some(vec!["path/to/file.md".to_string()]));
        assert_eq!(options.labels, None);
    }

    #[test]
    fn test_build_filter_options_label_scope() {
        let args = GraphArgs {
            format: GraphFormat::Dot,
            scope: Some("backend".to_string()),
            hide_completed: false,
            output: None,
            project_root: None,
            theme: None,
            verbosity: Verbosity::Normal,
        };

        let options = build_filter_options(&args);
        assert_eq!(options.files, None);
        assert_eq!(options.labels, Some(vec!["backend".to_string()]));
    }

    #[test]
    fn test_build_filter_options_hide_completed() {
        let args = GraphArgs {
            format: GraphFormat::Json,
            scope: None,
            hide_completed: true,
            output: None,
            project_root: None,
            theme: None,
            verbosity: Verbosity::Normal,
        };

        let options = build_filter_options(&args);
        assert!(options.hide_completed);
    }
}
