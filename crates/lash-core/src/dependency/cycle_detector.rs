//! Cycle detection for dependency graphs
//!
//! This module provides comprehensive cycle detection using a three-color DFS algorithm.
//! It can detect all cycles in a graph, not just the first one encountered, and provides
//! detailed reporting with actionable suggestions for breaking cycles.
//!
//! # Algorithm
//!
//! Uses depth-first search with three node colors:
//! - **White**: Unvisited node
//! - **Gray**: Currently being explored (in the current DFS path)
//! - **Black**: Fully explored (all descendants visited)
//!
//! A back edge from a gray node to another gray node in the current path indicates a cycle.
//!
//! # Example
//!
//! ```
//! use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, CycleDetector};
//! use lash_types::{TaskStatus, DependencyKind};
//!
//! let mut graph = DependencyGraph::new();
//!
//! // Create a simple cycle: A -> B -> A
//! graph.add_node(
//!     "test#A".to_string(),
//!     NodeData::new("Task A".to_string(), TaskStatus::Open, "test".to_string(), 0)
//! );
//! graph.add_node(
//!     "test#B".to_string(),
//!     NodeData::new("Task B".to_string(), TaskStatus::Open, "test".to_string(), 0)
//! );
//!
//! graph.add_edge(
//!     "test#A".to_string(),
//!     "test#B".to_string(),
//!     EdgeData::new(DependencyKind::ExplicitId, Some("test.md:10".to_string()))
//! );
//! graph.add_edge(
//!     "test#B".to_string(),
//!     "test#A".to_string(),
//!     EdgeData::new(DependencyKind::ExplicitId, Some("test.md:15".to_string()))
//! );
//!
//! let detector = CycleDetector::new(&graph);
//! let report = detector.detect_cycles();
//!
//! assert_eq!(report.cycles.len(), 1);
//! assert!(report.cycles[0].path.contains(&"test#A".to_string()));
//! assert!(report.cycles[0].path.contains(&"test#B".to_string()));
//! ```

use super::graph::{DependencyGraph, EdgeData};
use lash_types::DependencyKind;
use std::collections::{HashMap, HashSet};

/// Node color for three-color DFS algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    /// Unvisited node
    White,
    /// Currently being explored (in current DFS path)
    Gray,
    /// Fully explored
    Black,
}

/// A detected cycle in the dependency graph
///
/// Contains the cycle path and metadata about the cycle for reporting purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    /// The cycle path as a sequence of task IDs
    ///
    /// The path starts and ends with the same task ID to show the cycle closure.
    /// Example: `["test#A", "test#B", "test#C", "test#A"]`
    pub path: Vec<String>,

    /// Whether this is a within-file cycle
    ///
    /// `true` if all tasks in the cycle are in the same file.
    pub is_within_file: bool,

    /// Edge metadata for each edge in the cycle
    ///
    /// Maps `(from_id, to_id)` to the edge metadata.
    pub edge_metadata: HashMap<(String, String), EdgeData>,
}

impl Cycle {
    /// Create a new cycle
    fn new(
        path: Vec<String>,
        is_within_file: bool,
        edge_metadata: HashMap<(String, String), EdgeData>,
    ) -> Self {
        Self {
            path,
            is_within_file,
            edge_metadata,
        }
    }

    /// Get the length of the cycle (number of unique tasks)
    #[must_use]
    pub fn len(&self) -> usize {
        // Subtract 1 because the path includes the starting node twice
        self.path.len().saturating_sub(1)
    }

    /// Check if the cycle is empty (should never happen in valid cycles)
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.path.len() <= 1
    }

    /// Check if this is a self-loop (A -> A)
    #[must_use]
    pub fn is_self_loop(&self) -> bool {
        self.len() == 1
    }

    /// Format the cycle as a human-readable string
    ///
    /// Example: `Task A -> Task B -> Task C -> Task A`
    #[must_use]
    pub fn format_path(&self, graph: &DependencyGraph) -> String {
        self.path
            .iter()
            .map(|id| {
                graph
                    .get_node(id)
                    .map_or_else(|| id.clone(), |node| format!("{} ({})", node.title, id))
            })
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    /// Get the weakest edge in the cycle (for suggesting where to break)
    ///
    /// Priority (weakest to strongest):
    /// 1. Directory dependencies
    /// 2. Explicit dependencies
    /// 3. Hierarchy dependencies
    ///
    /// # Panics
    ///
    /// May panic if internal state is inconsistent (should never happen in practice).
    #[must_use]
    pub fn find_weakest_edge(&self) -> Option<(String, String, &EdgeData)> {
        let mut weakest: Option<(String, String, &EdgeData, u8)> = None;

        for i in 0..self.path.len() - 1 {
            let from = &self.path[i];
            let to = &self.path[i + 1];
            let edge_id = (from.clone(), to.clone());

            if let Some(edge_data) = self.edge_metadata.get(&edge_id) {
                let priority = match edge_data.kind {
                    DependencyKind::Directory => 0, // Weakest
                    DependencyKind::ExplicitId | DependencyKind::ExplicitPath => 1,
                    DependencyKind::Hierarchy => 2, // Strongest
                };

                if weakest.is_none() || priority < weakest.as_ref().unwrap().3 {
                    weakest = Some((from.clone(), to.clone(), edge_data, priority));
                }
            }
        }

        weakest.map(|(from, to, edge_data, _)| (from, to, edge_data))
    }
}

/// A suggestion for how to resolve a cycle
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleSuggestion {
    /// The cycle this suggestion applies to
    pub cycle_index: usize,

    /// The edge to break (`from_id`, `to_id`)
    pub edge_to_break: (String, String),

    /// The type of action to take
    pub action: SuggestionAction,

    /// Human-readable description of the suggestion
    pub description: String,
}

/// Type of action to resolve a cycle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestionAction {
    /// Remove an explicit `@depends-on` annotation
    RemoveExplicitDependency,

    /// Restructure task hierarchy
    RestructureHierarchy,

    /// Reorganize directory structure
    ReorganizeDirectories,
}

/// Report containing all detected cycles and suggestions
#[derive(Debug, Clone)]
pub struct CycleReport {
    /// All detected cycles
    pub cycles: Vec<Cycle>,

    /// Suggestions for resolving cycles
    pub suggestions: Vec<CycleSuggestion>,
}

impl CycleReport {
    /// Create a new empty cycle report
    #[allow(dead_code)] // Used internally, may be useful for testing later
    fn new() -> Self {
        Self {
            cycles: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Check if any cycles were detected
    #[must_use]
    pub fn has_cycles(&self) -> bool {
        !self.cycles.is_empty()
    }

    /// Get the number of cycles detected
    #[must_use]
    pub fn cycle_count(&self) -> usize {
        self.cycles.len()
    }

    /// Format the full report as a human-readable string
    #[must_use]
    #[allow(clippy::format_push_string)] // Clearer than write! for building report strings
    pub fn format_report(&self, graph: &DependencyGraph) -> String {
        let mut output = String::new();

        if self.cycles.is_empty() {
            output.push_str("No cycles detected in the dependency graph.\n");
            return output;
        }

        output.push_str(&format!(
            "Found {} cycle{} in the dependency graph:\n\n",
            self.cycles.len(),
            if self.cycles.len() == 1 { "" } else { "s" }
        ));

        for (i, cycle) in self.cycles.iter().enumerate() {
            output.push_str(&format!("Cycle {}:\n", i + 1));
            output.push_str(&format!("  Path: {}\n", cycle.format_path(graph)));
            output.push_str(&format!("  Length: {} tasks\n", cycle.len()));
            output.push_str(&format!(
                "  Type: {}\n",
                if cycle.is_within_file {
                    "within-file"
                } else {
                    "cross-file"
                }
            ));

            // Show edge details
            output.push_str("  Edges:\n");
            for j in 0..cycle.path.len() - 1 {
                let from = &cycle.path[j];
                let to = &cycle.path[j + 1];
                let edge_id = (from.clone(), to.clone());

                if let Some(edge_data) = cycle.edge_metadata.get(&edge_id) {
                    let kind_str = match edge_data.kind {
                        DependencyKind::Hierarchy => "hierarchy",
                        DependencyKind::ExplicitId => "explicit (id)",
                        DependencyKind::ExplicitPath => "explicit (path)",
                        DependencyKind::Directory => "directory",
                    };

                    output.push_str(&format!("    {from} -> {to} [{kind_str}]"));

                    if let Some(location) = &edge_data.source_location {
                        output.push_str(&format!(" at {location}"));
                    }

                    output.push('\n');
                }
            }

            output.push('\n');
        }

        // Add suggestions
        if !self.suggestions.is_empty() {
            output.push_str("Suggestions:\n\n");

            for (i, suggestion) in self.suggestions.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, suggestion.description));
            }
        }

        output
    }
}

/// Cycle detector for dependency graphs
///
/// Uses three-color DFS to detect all cycles in the graph. The detector
/// maintains state during traversal and can generate detailed reports.
pub struct CycleDetector<'a> {
    /// Reference to the graph being analyzed
    graph: &'a DependencyGraph,

    /// Node colors for DFS
    colors: HashMap<String, Color>,

    /// Current DFS path (stack of node IDs)
    path: Vec<String>,

    /// Set of nodes in current path (for fast lookup)
    path_set: HashSet<String>,

    /// Detected cycles
    cycles: Vec<Cycle>,
}

impl<'a> CycleDetector<'a> {
    /// Create a new cycle detector for the given graph
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, CycleDetector};
    ///
    /// let graph = DependencyGraph::new();
    /// let detector = CycleDetector::new(&graph);
    /// ```
    #[must_use]
    pub fn new(graph: &'a DependencyGraph) -> Self {
        Self {
            graph,
            colors: HashMap::new(),
            path: Vec::new(),
            path_set: HashSet::new(),
            cycles: Vec::new(),
        }
    }

    /// Detect all cycles in the graph
    ///
    /// Returns a report containing all detected cycles and suggestions for
    /// resolving them.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, CycleDetector};
    /// use lash_types::{TaskStatus, DependencyKind};
    ///
    /// let mut graph = DependencyGraph::new();
    ///
    /// // Add nodes
    /// graph.add_node(
    ///     "test#A".to_string(),
    ///     NodeData::new("Task A".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    /// graph.add_node(
    ///     "test#B".to_string(),
    ///     NodeData::new("Task B".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    ///
    /// // Create cycle
    /// graph.add_edge(
    ///     "test#A".to_string(),
    ///     "test#B".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    /// graph.add_edge(
    ///     "test#B".to_string(),
    ///     "test#A".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// let detector = CycleDetector::new(&graph);
    /// let report = detector.detect_cycles();
    ///
    /// assert!(report.has_cycles());
    /// assert_eq!(report.cycle_count(), 1);
    /// ```
    #[must_use]
    pub fn detect_cycles(mut self) -> CycleReport {
        // Initialize all nodes as white
        for node_id in self.all_node_ids() {
            self.colors.insert(node_id, Color::White);
        }

        // Run DFS from all unvisited nodes
        let node_ids: Vec<String> = self.all_node_ids();
        for node_id in node_ids {
            if self.colors.get(&node_id) == Some(&Color::White) {
                self.dfs(&node_id);
            }
        }

        // Generate suggestions for each cycle
        let suggestions = self.generate_suggestions();

        CycleReport {
            cycles: self.cycles,
            suggestions,
        }
    }

    /// Get all node IDs from the graph
    fn all_node_ids(&self) -> Vec<String> {
        self.graph.all_node_ids()
    }

    /// Perform DFS from a node
    fn dfs(&mut self, node_id: &str) {
        // Mark as gray (currently exploring)
        self.colors.insert(node_id.to_string(), Color::Gray);
        self.path.push(node_id.to_string());
        self.path_set.insert(node_id.to_string());

        // Explore all dependencies
        if let Some(deps) = self.graph.get_dependencies(node_id) {
            for edge_ref in deps {
                let dep_id = &edge_ref.target_id;

                match self.colors.get(dep_id) {
                    Some(Color::White) => {
                        // Unvisited node - explore it
                        self.dfs(dep_id);
                    }
                    Some(Color::Gray) => {
                        // Back edge detected - we found a cycle!
                        self.extract_cycle(dep_id);
                    }
                    Some(Color::Black) | None => {
                        // Already fully explored or not in graph - skip
                    }
                }
            }
        }

        // Mark as black (fully explored)
        self.colors.insert(node_id.to_string(), Color::Black);
        self.path.pop();
        self.path_set.remove(node_id);
    }

    /// Extract a cycle from the current path
    fn extract_cycle(&mut self, back_edge_target: &str) {
        // Find where the cycle starts in the current path
        let cycle_start = self
            .path
            .iter()
            .position(|id| id == back_edge_target)
            .expect("Back edge target must be in path");

        // Extract the cycle path
        let mut cycle_path: Vec<String> = self.path[cycle_start..].to_vec();
        // Close the cycle by adding the starting node at the end
        cycle_path.push(back_edge_target.to_string());

        // Collect edge metadata for the cycle
        let mut edge_metadata = HashMap::new();
        for i in 0..cycle_path.len() - 1 {
            let from = &cycle_path[i];
            let to = &cycle_path[i + 1];

            if let Some(edge_data) = self.graph.get_edge(from, to) {
                edge_metadata.insert((from.clone(), to.clone()), edge_data.clone());
            }
        }

        // Determine if within-file cycle
        let file_ids: HashSet<_> = cycle_path
            .iter()
            .filter_map(|id| self.graph.get_node(id))
            .map(|node| &node.file_id)
            .collect();
        let is_within_file = file_ids.len() == 1;

        let cycle = Cycle::new(cycle_path, is_within_file, edge_metadata);

        // Check for duplicates (same cycle, possibly different starting point)
        if !self.is_duplicate_cycle(&cycle) {
            self.cycles.push(cycle);
        }
    }

    /// Check if a cycle is a duplicate of one already detected
    fn is_duplicate_cycle(&self, new_cycle: &Cycle) -> bool {
        for existing_cycle in &self.cycles {
            if Self::cycles_equivalent(new_cycle, existing_cycle) {
                return true;
            }
        }
        false
    }

    /// Check if two cycles are equivalent (same nodes, possibly different starting point)
    fn cycles_equivalent(cycle1: &Cycle, cycle2: &Cycle) -> bool {
        if cycle1.len() != cycle2.len() {
            return false;
        }

        // Convert to sets (excluding the duplicate end node)
        let set1: HashSet<_> = cycle1.path.iter().take(cycle1.len()).collect();
        let set2: HashSet<_> = cycle2.path.iter().take(cycle2.len()).collect();

        set1 == set2
    }

    /// Generate suggestions for resolving cycles
    fn generate_suggestions(&self) -> Vec<CycleSuggestion> {
        let mut suggestions = Vec::new();

        for (i, cycle) in self.cycles.iter().enumerate() {
            if let Some((from, to, edge_data)) = cycle.find_weakest_edge() {
                let (action, description) = match edge_data.kind {
                    DependencyKind::Directory => (
                        SuggestionAction::ReorganizeDirectories,
                        format!(
                            "Break cycle {} by reorganizing directory structure (remove dependency from {} to {})",
                            i + 1,
                            from,
                            to
                        ),
                    ),
                    DependencyKind::ExplicitId | DependencyKind::ExplicitPath => {
                        let location = edge_data
                            .source_location
                            .as_ref()
                            .map(|s| format!(" at {s}"))
                            .unwrap_or_default();
                        (
                            SuggestionAction::RemoveExplicitDependency,
                            format!(
                                "Break cycle {} by removing explicit @depends-on from {} to {}{}",
                                i + 1,
                                from,
                                to,
                                location
                            ),
                        )
                    }
                    DependencyKind::Hierarchy => (
                        SuggestionAction::RestructureHierarchy,
                        format!(
                            "Break cycle {} by restructuring task hierarchy (move {} out from under {})",
                            i + 1,
                            to,
                            from
                        ),
                    ),
                };

                suggestions.push(CycleSuggestion {
                    cycle_index: i,
                    edge_to_break: (from, to),
                    action,
                    description,
                });
            }
        }

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency::NodeData;
    use lash_types::TaskStatus;

    fn create_test_node(title: &str, file_id: &str) -> NodeData {
        NodeData::new(title.to_string(), TaskStatus::Open, file_id.to_string(), 0)
    }

    #[test]
    fn test_no_cycles_empty_graph() {
        let graph = DependencyGraph::new();
        let detector = CycleDetector::new(&graph);
        let report = detector.detect_cycles();

        assert!(!report.has_cycles());
        assert_eq!(report.cycle_count(), 0);
    }

    #[test]
    fn test_no_cycles_acyclic_graph() {
        let mut graph = DependencyGraph::new();

        // Linear chain: A -> B -> C
        graph.add_node("test#A".to_string(), create_test_node("A", "test"));
        graph.add_node("test#B".to_string(), create_test_node("B", "test"));
        graph.add_node("test#C".to_string(), create_test_node("C", "test"));

        graph.add_edge(
            "test#A".to_string(),
            "test#B".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#B".to_string(),
            "test#C".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let detector = CycleDetector::new(&graph);
        let report = detector.detect_cycles();

        assert!(!report.has_cycles());
    }

    #[test]
    fn test_simple_cycle() {
        let mut graph = DependencyGraph::new();

        // Simple cycle: A -> B -> A
        graph.add_node("test#A".to_string(), create_test_node("A", "test"));
        graph.add_node("test#B".to_string(), create_test_node("B", "test"));

        graph.add_edge(
            "test#A".to_string(),
            "test#B".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, Some("test.md:10".to_string())),
        );
        graph.add_edge(
            "test#B".to_string(),
            "test#A".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, Some("test.md:15".to_string())),
        );

        let detector = CycleDetector::new(&graph);
        let report = detector.detect_cycles();

        assert!(report.has_cycles());
        assert_eq!(report.cycle_count(), 1);

        let cycle = &report.cycles[0];
        assert_eq!(cycle.len(), 2);
        assert!(cycle.path.contains(&"test#A".to_string()));
        assert!(cycle.path.contains(&"test#B".to_string()));
        assert!(cycle.is_within_file);
    }

    #[test]
    fn test_self_loop() {
        let mut graph = DependencyGraph::new();

        // Self loop: A -> A
        graph.add_node("test#A".to_string(), create_test_node("A", "test"));

        graph.add_edge(
            "test#A".to_string(),
            "test#A".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let detector = CycleDetector::new(&graph);
        let report = detector.detect_cycles();

        assert!(report.has_cycles());
        assert_eq!(report.cycle_count(), 1);

        let cycle = &report.cycles[0];
        assert!(cycle.is_self_loop());
        assert_eq!(cycle.len(), 1);
    }

    #[test]
    fn test_complex_cycle() {
        let mut graph = DependencyGraph::new();

        // Complex cycle: A -> B -> C -> D -> B
        graph.add_node("test#A".to_string(), create_test_node("A", "test"));
        graph.add_node("test#B".to_string(), create_test_node("B", "test"));
        graph.add_node("test#C".to_string(), create_test_node("C", "test"));
        graph.add_node("test#D".to_string(), create_test_node("D", "test"));

        graph.add_edge(
            "test#A".to_string(),
            "test#B".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#B".to_string(),
            "test#C".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#C".to_string(),
            "test#D".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#D".to_string(),
            "test#B".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let detector = CycleDetector::new(&graph);
        let report = detector.detect_cycles();

        assert!(report.has_cycles());
        assert_eq!(report.cycle_count(), 1);

        let cycle = &report.cycles[0];
        assert_eq!(cycle.len(), 3); // B -> C -> D -> B
        assert!(cycle.path.contains(&"test#B".to_string()));
        assert!(cycle.path.contains(&"test#C".to_string()));
        assert!(cycle.path.contains(&"test#D".to_string()));
    }

    #[test]
    fn test_multiple_disjoint_cycles() {
        let mut graph = DependencyGraph::new();

        // Cycle 1: A -> B -> A
        graph.add_node("test#A".to_string(), create_test_node("A", "test"));
        graph.add_node("test#B".to_string(), create_test_node("B", "test"));

        graph.add_edge(
            "test#A".to_string(),
            "test#B".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#B".to_string(),
            "test#A".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        // Cycle 2: C -> D -> C
        graph.add_node("test#C".to_string(), create_test_node("C", "test"));
        graph.add_node("test#D".to_string(), create_test_node("D", "test"));

        graph.add_edge(
            "test#C".to_string(),
            "test#D".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#D".to_string(),
            "test#C".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let detector = CycleDetector::new(&graph);
        let report = detector.detect_cycles();

        assert!(report.has_cycles());
        assert_eq!(report.cycle_count(), 2);
    }

    #[test]
    fn test_cross_file_cycle() {
        let mut graph = DependencyGraph::new();

        // Cross-file cycle
        graph.add_node("file1#A".to_string(), create_test_node("A", "file1"));
        graph.add_node("file2#B".to_string(), create_test_node("B", "file2"));

        graph.add_edge(
            "file1#A".to_string(),
            "file2#B".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, Some("file1.md:10".to_string())),
        );
        graph.add_edge(
            "file2#B".to_string(),
            "file1#A".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, Some("file2.md:15".to_string())),
        );

        let detector = CycleDetector::new(&graph);
        let report = detector.detect_cycles();

        assert!(report.has_cycles());
        assert_eq!(report.cycle_count(), 1);

        let cycle = &report.cycles[0];
        assert!(!cycle.is_within_file);
    }

    #[test]
    fn test_weakest_edge_directory() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#A".to_string(), create_test_node("A", "test"));
        graph.add_node("test#B".to_string(), create_test_node("B", "test"));

        // A -> B (directory), B -> A (explicit)
        graph.add_edge(
            "test#A".to_string(),
            "test#B".to_string(),
            EdgeData::new(DependencyKind::Directory, None),
        );
        graph.add_edge(
            "test#B".to_string(),
            "test#A".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let detector = CycleDetector::new(&graph);
        let report = detector.detect_cycles();

        assert!(report.has_cycles());
        let cycle = &report.cycles[0];
        let (from, to, edge_data) = cycle.find_weakest_edge().unwrap();

        assert_eq!(from, "test#A");
        assert_eq!(to, "test#B");
        assert!(matches!(edge_data.kind, DependencyKind::Directory));
    }

    #[test]
    fn test_suggestions() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#A".to_string(), create_test_node("A", "test"));
        graph.add_node("test#B".to_string(), create_test_node("B", "test"));

        graph.add_edge(
            "test#A".to_string(),
            "test#B".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, Some("test.md:10".to_string())),
        );
        graph.add_edge(
            "test#B".to_string(),
            "test#A".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        let detector = CycleDetector::new(&graph);
        let report = detector.detect_cycles();

        assert!(!report.suggestions.is_empty());
        let suggestion = &report.suggestions[0];
        assert!(matches!(
            suggestion.action,
            SuggestionAction::RemoveExplicitDependency
        ));
    }

    #[test]
    fn test_format_report() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#A".to_string(), create_test_node("A", "test"));
        graph.add_node("test#B".to_string(), create_test_node("B", "test"));

        graph.add_edge(
            "test#A".to_string(),
            "test#B".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#B".to_string(),
            "test#A".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let detector = CycleDetector::new(&graph);
        let report = detector.detect_cycles();

        let formatted = report.format_report(&graph);
        assert!(formatted.contains("Found 1 cycle"));
        assert!(formatted.contains("Path:"));
        assert!(formatted.contains("Suggestions:"));
    }
}
