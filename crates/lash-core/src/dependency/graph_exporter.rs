//! Graph export functionality for dependency visualization
//!
//! This module provides export capabilities for the dependency graph in multiple formats:
//!
//! - **DOT format** - Graphviz-compatible for visual rendering
//! - **JSON format** - Structured data for programmatic consumption
//! - **ASCII tree** - Terminal-friendly text visualization
//!
//! Each format supports filtering options to export subgraphs based on files, labels,
//! or completion status.
//!
//! # Example: DOT Export
//!
//! ```
//! use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, GraphExporter, FilterOptions};
//! use lash_types::{TaskStatus, DependencyKind};
//!
//! let mut graph = DependencyGraph::new();
//! graph.add_node(
//!     "test#task1".to_string(),
//!     NodeData::new("Task 1".to_string(), TaskStatus::Done, "test".to_string(), 0)
//! );
//! graph.add_node(
//!     "test#task2".to_string(),
//!     NodeData::new("Task 2".to_string(), TaskStatus::Open, "test".to_string(), 0)
//! );
//! graph.add_edge(
//!     "test#task1".to_string(),
//!     "test#task2".to_string(),
//!     EdgeData::new(DependencyKind::ExplicitId, None)
//! );
//!
//! let exporter = GraphExporter::new(&graph);
//! let dot = exporter.to_dot(&FilterOptions::default());
//! assert!(dot.contains("digraph"));
//! assert!(dot.contains("test#task1"));
//! ```

use super::graph::DependencyGraph;
use lash_types::TaskStatus;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;

/// Options for filtering graph export
///
/// These options allow exporting subgraphs based on various criteria.
///
/// # Example
///
/// ```
/// use lash_core::dependency::FilterOptions;
///
/// // Export only incomplete tasks from specific files
/// let options = FilterOptions {
///     files: Some(vec!["core/api.md".to_string()]),
///     hide_completed: true,
///     max_depth: Some(1), // Only direct dependencies
///     labels: None,
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct FilterOptions {
    /// Only include tasks from these files (None = all files)
    pub files: Option<Vec<String>>,

    /// Only include tasks with these labels (None = all labels)
    pub labels: Option<Vec<String>>,

    /// Hide completed tasks (done or waived)
    pub hide_completed: bool,

    /// Maximum dependency depth (None = unlimited)
    pub max_depth: Option<usize>,
}

/// Graph exporter for multiple output formats
///
/// Provides methods to export the dependency graph in various formats suitable
/// for different use cases (visualization, analysis, programmatic access).
///
/// # Example
///
/// ```
/// use lash_core::dependency::{DependencyGraph, GraphExporter, FilterOptions};
///
/// let graph = DependencyGraph::new();
/// let exporter = GraphExporter::new(&graph);
///
/// // Export as DOT
/// let dot = exporter.to_dot(&FilterOptions::default());
///
/// // Export as JSON
/// let json = exporter.to_json(&FilterOptions::default());
///
/// // Export as ASCII tree
/// let tree = exporter.to_ascii_tree("test#task1", &FilterOptions::default());
/// ```
pub struct GraphExporter<'a> {
    graph: &'a DependencyGraph,
}

impl<'a> GraphExporter<'a> {
    /// Create a new graph exporter
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, GraphExporter};
    ///
    /// let graph = DependencyGraph::new();
    /// let exporter = GraphExporter::new(&graph);
    /// ```
    #[must_use]
    pub fn new(graph: &'a DependencyGraph) -> Self {
        Self { graph }
    }

    /// Export the graph in DOT format (Graphviz)
    ///
    /// Generates a DOT file representation of the dependency graph with:
    /// - Color-coded nodes by status (green=done, yellow=open, red=blocked, gray=waived)
    /// - Labeled edges showing dependency types
    /// - Clustering by file
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, GraphExporter, FilterOptions};
    /// use lash_types::TaskStatus;
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node(
    ///     "test#task1".to_string(),
    ///     NodeData::new("Task 1".to_string(), TaskStatus::Done, "test".to_string(), 0)
    /// );
    ///
    /// let exporter = GraphExporter::new(&graph);
    /// let dot = exporter.to_dot(&FilterOptions::default());
    ///
    /// // Verify it's valid DOT syntax
    /// assert!(dot.starts_with("digraph"));
    /// assert!(dot.contains("test#task1"));
    /// assert!(dot.ends_with("}\n"));
    /// ```
    #[must_use]
    pub fn to_dot(&self, options: &FilterOptions) -> String {
        let mut output = String::new();
        output.push_str("digraph dependencies {\n");
        output.push_str("  // Graph settings\n");
        output.push_str("  rankdir=TB;\n");
        output.push_str("  node [shape=box, style=filled];\n");
        output.push_str("  edge [fontsize=10];\n\n");

        // Get filtered nodes
        let nodes = self.filter_nodes(options);

        // Group nodes by file for clustering (using BTreeMap for deterministic ordering)
        let mut file_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for node_id in &nodes {
            if let Some(node) = self.graph.get_node(node_id) {
                file_groups
                    .entry(node.file_id.clone())
                    .or_default()
                    .push(node_id.clone());
            }
        }

        // Sort task_ids within each file for deterministic output
        for task_ids in file_groups.values_mut() {
            task_ids.sort();
        }

        // Output nodes grouped by file (clusters)
        for (file_id, task_ids) in &file_groups {
            writeln!(output, "  // File: {file_id}").ok();
            writeln!(
                output,
                "  subgraph cluster_{} {{",
                file_id.replace(['/', '.', '-'], "_")
            )
            .ok();
            writeln!(output, "    label=\"{file_id}\";").ok();
            output.push_str("    style=dashed;\n");
            output.push_str("    color=gray;\n\n");

            for task_id in task_ids {
                if let Some(node) = self.graph.get_node(task_id) {
                    let color = Self::status_color(node.status);
                    let label = Self::escape_dot_label(&node.title);
                    let escaped_id = Self::escape_dot_id(task_id);
                    writeln!(
                        output,
                        "    {escaped_id} [label=\"{label}\", fillcolor=\"{color}\"];"
                    )
                    .ok();
                }
            }

            output.push_str("  }\n\n");
        }

        // Output edges (sort node IDs for deterministic output)
        output.push_str("  // Dependencies\n");
        let mut sorted_nodes: Vec<_> = nodes.iter().collect();
        sorted_nodes.sort();

        for node_id in sorted_nodes {
            if let Some(deps) = self.graph.get_dependencies(node_id) {
                // Sort dependencies for deterministic output
                let mut sorted_deps: Vec<_> = deps.iter().collect();
                sorted_deps.sort_by(|a, b| a.target_id.cmp(&b.target_id));

                for dep in sorted_deps {
                    let target_id = &dep.target_id;

                    // Only output edge if target is in filtered nodes
                    if nodes.contains(target_id) {
                        let edge_data = self.graph.get_edge(node_id, target_id);
                        let label = if let Some(data) = edge_data {
                            format!(" [label=\"{:?}\"]", data.kind)
                        } else {
                            String::new()
                        };

                        let escaped_from = Self::escape_dot_id(node_id);
                        let escaped_to = Self::escape_dot_id(target_id);
                        writeln!(output, "  {escaped_from} -> {escaped_to}{label};").ok();
                    }
                }
            }
        }

        output.push_str("}\n");
        output
    }

    /// Export the graph in JSON format
    ///
    /// Generates a JSON representation with separate nodes and edges arrays,
    /// including full task metadata and dependency information.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, GraphExporter, FilterOptions};
    /// use lash_types::{TaskStatus, DependencyKind};
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node(
    ///     "test#task1".to_string(),
    ///     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    /// graph.add_node(
    ///     "test#task2".to_string(),
    ///     NodeData::new("Task 2".to_string(), TaskStatus::Done, "test".to_string(), 0)
    /// );
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// let exporter = GraphExporter::new(&graph);
    /// let json = exporter.to_json(&FilterOptions::default());
    ///
    /// // Verify it's valid JSON
    /// let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// assert!(parsed["nodes"].is_array());
    /// assert!(parsed["edges"].is_array());
    /// assert_eq!(parsed["nodes"].as_array().unwrap().len(), 2);
    /// ```
    #[must_use]
    pub fn to_json(&self, options: &FilterOptions) -> String {
        let nodes = self.filter_nodes(options);

        // Build nodes array
        let mut json_nodes = Vec::new();
        for node_id in &nodes {
            if let Some(node) = self.graph.get_node(node_id) {
                json_nodes.push(JsonNode {
                    id: node_id.clone(),
                    title: node.title.clone(),
                    status: format!("{:?}", node.status),
                    file_id: node.file_id.clone(),
                    depth: node.depth,
                });
            }
        }

        // Build edges array
        let mut json_edges = Vec::new();
        for node_id in &nodes {
            if let Some(deps) = self.graph.get_dependencies(node_id) {
                for dep in deps {
                    let target_id = &dep.target_id;

                    // Only include edge if target is in filtered nodes
                    if nodes.contains(target_id) {
                        if let Some(edge_data) = self.graph.get_edge(node_id, target_id) {
                            json_edges.push(JsonEdge {
                                from: node_id.clone(),
                                to: target_id.clone(),
                                kind: format!("{:?}", edge_data.kind),
                                source_location: edge_data.source_location.clone(),
                            });
                        }
                    }
                }
            }
        }

        let graph_json = JsonGraph {
            nodes: json_nodes,
            edges: json_edges,
        };

        serde_json::to_string_pretty(&graph_json)
            .unwrap_or_else(|e| format!("{{\"error\": \"Failed to serialize JSON: {e}\"}}"))
    }

    /// Export the graph as an ASCII tree starting from a root task
    ///
    /// Generates a terminal-friendly tree visualization showing dependency
    /// relationships with indentation and ASCII art.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, GraphExporter, FilterOptions};
    /// use lash_types::{TaskStatus, DependencyKind};
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node(
    ///     "test#task1".to_string(),
    ///     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    /// graph.add_node(
    ///     "test#task2".to_string(),
    ///     NodeData::new("Task 2".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// let exporter = GraphExporter::new(&graph);
    /// let tree = exporter.to_ascii_tree("test#task1", &FilterOptions::default());
    ///
    /// // Should show task1 and its dependency on task2
    /// assert!(tree.contains("Task 1"));
    /// assert!(tree.contains("Task 2"));
    /// assert!(tree.contains("└─")); // Tree branch character
    /// ```
    #[must_use]
    pub fn to_ascii_tree(&self, root_id: &str, options: &FilterOptions) -> String {
        let mut output = String::new();
        let mut visited = HashSet::new();

        self.render_tree_node(root_id, options, 0, "", true, &mut output, &mut visited);

        output
    }

    // Private helper methods

    /// Filter nodes based on options
    fn filter_nodes(&self, options: &FilterOptions) -> HashSet<String> {
        let mut nodes = HashSet::new();

        for node_id in self.graph.all_node_ids() {
            if let Some(node) = self.graph.get_node(&node_id) {
                // Filter by file
                if let Some(files) = &options.files {
                    if !files.contains(&node.file_id) {
                        continue;
                    }
                }

                // Filter by completion status
                if options.hide_completed
                    && (node.status == TaskStatus::Done || node.status == TaskStatus::Waived)
                {
                    continue;
                }

                // TODO: Filter by labels (when labels are available in NodeData)

                nodes.insert(node_id);
            }
        }

        // Apply depth limit if specified
        if let Some(max_depth) = options.max_depth {
            nodes = self.apply_depth_limit(&nodes, max_depth);
        }

        nodes
    }

    /// Apply depth limit to node set
    fn apply_depth_limit(&self, nodes: &HashSet<String>, max_depth: usize) -> HashSet<String> {
        let mut result = HashSet::new();

        // For each node, check if it's reachable within max_depth from any root
        for node_id in nodes {
            // Consider as root if it's in the node set and has no dependents in the set
            let is_root = self.graph.get_dependents(node_id).map_or(true, |deps| {
                !deps.iter().any(|d| nodes.contains(&d.target_id))
            });

            if is_root {
                // Add this root and its descendants up to max_depth
                result.insert(node_id.clone());
                if let Ok(descendants) = self.graph.get_descendants_with_depth(node_id, max_depth) {
                    for desc_id in descendants {
                        if nodes.contains(&desc_id) {
                            result.insert(desc_id);
                        }
                    }
                }
            }
        }

        result
    }

    /// Render a single node in the ASCII tree
    #[allow(clippy::too_many_arguments)]
    fn render_tree_node(
        &self,
        node_id: &str,
        options: &FilterOptions,
        depth: usize,
        prefix: &str,
        is_last: bool,
        output: &mut String,
        visited: &mut HashSet<String>,
    ) {
        // Prevent infinite recursion in case of cycles
        if visited.contains(node_id) {
            writeln!(output, "{prefix}└─ (cycle: {node_id})").ok();
            return;
        }

        if let Some(node) = self.graph.get_node(node_id) {
            // Apply completion filter
            if options.hide_completed
                && (node.status == TaskStatus::Done || node.status == TaskStatus::Waived)
            {
                return;
            }

            // Check depth limit
            if let Some(max_depth) = options.max_depth {
                if depth >= max_depth {
                    return;
                }
            }

            visited.insert(node_id.to_string());

            // Format status indicator
            let status_symbol = match node.status {
                TaskStatus::Open => "[ ]",
                TaskStatus::Done => "[✓]",
                TaskStatus::Waived => "[-]",
                TaskStatus::Blocked => "[!]",
            };

            // Render current node
            let branch = if is_last { "└─" } else { "├─" };
            writeln!(
                output,
                "{prefix}{branch} {status_symbol} {} ({})",
                node.title, node_id
            )
            .ok();

            // Get dependencies
            if let Some(deps) = self.graph.get_dependencies(node_id) {
                let dep_count = deps.len();
                let new_prefix = format!("{prefix}{}", if is_last { "   " } else { "│  " });

                for (idx, dep) in deps.iter().enumerate() {
                    let is_last_dep = idx == dep_count - 1;
                    self.render_tree_node(
                        &dep.target_id,
                        options,
                        depth + 1,
                        &new_prefix,
                        is_last_dep,
                        output,
                        visited,
                    );
                }
            }

            visited.remove(node_id);
        } else {
            // Node doesn't exist
            writeln!(output, "{prefix}└─ (missing: {node_id})").ok();
        }
    }

    /// Get color for task status (for DOT export)
    fn status_color(status: TaskStatus) -> &'static str {
        match status {
            TaskStatus::Done => "lightgreen",
            TaskStatus::Open => "lightyellow",
            TaskStatus::Waived => "lightgray",
            TaskStatus::Blocked => "lightcoral",
        }
    }

    /// Escape special characters in DOT labels
    fn escape_dot_label(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    /// Escape special characters in DOT identifiers
    fn escape_dot_id(s: &str) -> String {
        format!("\"{}\"", s.replace('"', "\\\""))
    }
}

// JSON export data structures

#[derive(Debug, Serialize, Deserialize)]
struct JsonGraph {
    nodes: Vec<JsonNode>,
    edges: Vec<JsonEdge>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonNode {
    id: String,
    title: String,
    status: String,
    file_id: String,
    depth: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonEdge {
    from: String,
    to: String,
    kind: String,
    source_location: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency::{EdgeData, NodeData};
    use lash_types::DependencyKind;

    fn create_test_node(title: &str, status: TaskStatus, file_id: &str) -> NodeData {
        NodeData::new(title.to_string(), status, file_id.to_string(), 0)
    }

    #[test]
    fn test_export_empty_graph_dot() {
        let graph = DependencyGraph::new();
        let exporter = GraphExporter::new(&graph);

        let dot = exporter.to_dot(&FilterOptions::default());

        // Should have valid DOT structure
        assert!(dot.starts_with("digraph"));
        assert!(dot.ends_with("}\n"));
        assert!(dot.contains("rankdir=TB"));
    }

    #[test]
    fn test_export_empty_graph_json() {
        let graph = DependencyGraph::new();
        let exporter = GraphExporter::new(&graph);

        let json = exporter.to_json(&FilterOptions::default());

        // Should be valid JSON with empty arrays
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["nodes"].is_array());
        assert!(parsed["edges"].is_array());
        assert_eq!(parsed["nodes"].as_array().unwrap().len(), 0);
        assert_eq!(parsed["edges"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_export_simple_graph_dot() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Done, "test"),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_test_node("Task 2", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let exporter = GraphExporter::new(&graph);
        let dot = exporter.to_dot(&FilterOptions::default());

        // Should contain both nodes
        assert!(dot.contains("test#task1"));
        assert!(dot.contains("test#task2"));
        assert!(dot.contains("Task 1"));
        assert!(dot.contains("Task 2"));

        // Should contain edge
        assert!(dot.contains("->"));

        // Should have color coding
        assert!(dot.contains("lightgreen")); // task1 is done
        assert!(dot.contains("lightyellow")); // task2 is open

        // Should have file cluster
        assert!(dot.contains("cluster_test"));
        assert!(dot.contains("label=\"test\""));
    }

    #[test]
    fn test_export_simple_graph_json() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Done, "test"),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_test_node("Task 2", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, Some("test.md:10".to_string())),
        );

        let exporter = GraphExporter::new(&graph);
        let json = exporter.to_json(&FilterOptions::default());

        // Parse JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Check nodes
        let json_nodes = parsed["nodes"].as_array().unwrap();
        assert_eq!(json_nodes.len(), 2);

        // Find the node with task1 (order is not guaranteed)
        let task1 = json_nodes
            .iter()
            .find(|n| n["id"] == "test#task1")
            .expect("task1 not found");
        assert_eq!(task1["id"], "test#task1");
        assert_eq!(task1["title"], "Task 1");
        assert_eq!(task1["status"], "Done");
        assert_eq!(task1["file_id"], "test");

        // Check edges
        let edges = parsed["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1);

        let edge = &edges[0];
        assert_eq!(edge["from"], "test#task1");
        assert_eq!(edge["to"], "test#task2");
        assert_eq!(edge["kind"], "Hierarchy");
        assert_eq!(edge["source_location"], "test.md:10");
    }

    #[test]
    fn test_export_ascii_tree_simple() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_test_node("Task 2", TaskStatus::Done, "test"),
        );
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let exporter = GraphExporter::new(&graph);
        let tree = exporter.to_ascii_tree("test#task1", &FilterOptions::default());

        // Should contain both tasks
        assert!(tree.contains("Task 1"));
        assert!(tree.contains("Task 2"));

        // Should have tree structure
        assert!(tree.contains("└─"));

        // Should have status indicators
        assert!(tree.contains("[ ]")); // Open
        assert!(tree.contains("[✓]")); // Done
    }

    #[test]
    fn test_export_ascii_tree_nested() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_test_node("Task 2", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#task3".to_string(),
            create_test_node("Task 3", TaskStatus::Done, "test"),
        );

        // Chain: task1 → task2 → task3
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#task2".to_string(),
            "test#task3".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let exporter = GraphExporter::new(&graph);
        let tree = exporter.to_ascii_tree("test#task1", &FilterOptions::default());

        // Should contain all tasks
        assert!(tree.contains("Task 1"));
        assert!(tree.contains("Task 2"));
        assert!(tree.contains("Task 3"));

        // Should have nested structure
        assert!(tree.contains("└─"));
        assert!(tree.contains("   ")); // Indentation for nested items
    }

    #[test]
    fn test_filter_by_file() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "file1#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Open, "file1"),
        );
        graph.add_node(
            "file2#task2".to_string(),
            create_test_node("Task 2", TaskStatus::Open, "file2"),
        );

        let exporter = GraphExporter::new(&graph);
        let options = FilterOptions {
            files: Some(vec!["file1".to_string()]),
            ..Default::default()
        };

        let json = exporter.to_json(&options);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should only include file1 task
        let json_nodes = parsed["nodes"].as_array().unwrap();
        assert_eq!(json_nodes.len(), 1);
        assert_eq!(json_nodes[0]["file_id"], "file1");
    }

    #[test]
    fn test_filter_hide_completed() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Done, "test"),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_test_node("Task 2", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#task3".to_string(),
            create_test_node("Task 3", TaskStatus::Waived, "test"),
        );

        let exporter = GraphExporter::new(&graph);
        let options = FilterOptions {
            hide_completed: true,
            ..Default::default()
        };

        let json = exporter.to_json(&options);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should only include open task
        let json_nodes = parsed["nodes"].as_array().unwrap();
        assert_eq!(json_nodes.len(), 1);
        assert_eq!(json_nodes[0]["status"], "Open");
    }

    #[test]
    fn test_filter_max_depth() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_test_node("Task 2", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#task3".to_string(),
            create_test_node("Task 3", TaskStatus::Open, "test"),
        );

        // Chain: task1 → task2 → task3
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#task2".to_string(),
            "test#task3".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let exporter = GraphExporter::new(&graph);
        let options = FilterOptions {
            max_depth: Some(1), // Only direct dependencies
            ..Default::default()
        };

        let json = exporter.to_json(&options);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should include task1 and task2, but not task3
        let json_nodes = parsed["nodes"].as_array().unwrap();
        assert_eq!(json_nodes.len(), 2);

        let node_ids: Vec<String> = json_nodes
            .iter()
            .map(|n| n["id"].as_str().unwrap().to_string())
            .collect();
        assert!(node_ids.contains(&"test#task1".to_string()));
        assert!(node_ids.contains(&"test#task2".to_string()));
        assert!(!node_ids.contains(&"test#task3".to_string()));
    }

    #[test]
    fn test_ascii_tree_cycle_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_test_node("Task 2", TaskStatus::Open, "test"),
        );

        // Create cycle: task1 → task2 → task1
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#task2".to_string(),
            "test#task1".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let exporter = GraphExporter::new(&graph);
        let tree = exporter.to_ascii_tree("test#task1", &FilterOptions::default());

        // Should detect cycle
        assert!(tree.contains("cycle"));
    }

    #[test]
    fn test_dot_escape_special_chars() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_test_node(
                "Task with \"quotes\" and \\backslash",
                TaskStatus::Open,
                "test",
            ),
        );

        let exporter = GraphExporter::new(&graph);
        let dot = exporter.to_dot(&FilterOptions::default());

        // Should escape special characters
        assert!(dot.contains("\\\"quotes\\\""));
        assert!(dot.contains("\\\\backslash"));
    }

    #[test]
    fn test_multiple_files_clustering() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "file1#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Open, "file1"),
        );
        graph.add_node(
            "file2#task2".to_string(),
            create_test_node("Task 2", TaskStatus::Open, "file2"),
        );
        graph.add_edge(
            "file1#task1".to_string(),
            "file2#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let exporter = GraphExporter::new(&graph);
        let dot = exporter.to_dot(&FilterOptions::default());

        // Should have separate clusters for each file
        assert!(dot.contains("cluster_file1"));
        assert!(dot.contains("cluster_file2"));
        assert!(dot.contains("label=\"file1\""));
        assert!(dot.contains("label=\"file2\""));
    }
}
