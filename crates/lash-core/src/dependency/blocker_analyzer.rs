//! Blocker identification and analysis
//!
//! This module identifies which dependencies are blocking a task's completion and
//! provides actionable reports for developers to understand and resolve blockers.
//!
//! The blocker analyzer builds on top of `StatusComputer` to provide:
//! - Direct blocker identification (immediate dependencies blocking a task)
//! - Transitive blocker analysis (chains of blocked dependencies)
//! - Root blocker identification (fundamental blockers with no further dependencies)
//! - Actionable blocker reports with suggestions for resolution
//!
//! # Algorithm
//!
//! The analyzer uses BFS to explore blocker chains, tracking depth to distinguish
//! direct blockers (depth 0) from transitive blockers (depth > 0). Root blockers
//! are identified by finding blockers that have no incomplete dependencies themselves.
//!
//! # Performance
//!
//! - Finding direct blockers: O(D) where D = number of direct dependencies
//! - Finding transitive blockers: O(V+E) worst case (full graph traversal)
//! - Typically much faster in practice since we only follow blocker paths
//!
//! # Example
//!
//! ```
//! use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, StatusComputer, BlockerAnalyzer};
//! use lash_types::{TaskStatus, DependencyKind};
//!
//! // Create a chain: task1 depends on task2 (open)
//! let mut graph = DependencyGraph::new();
//! graph.add_node(
//!     "test#task1".to_string(),
//!     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0)
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
//! // Compute statuses
//! let computer = StatusComputer::new(&graph);
//! let statuses = computer.compute_all();
//!
//! // Analyze blockers
//! let analyzer = BlockerAnalyzer::new(&graph, &statuses);
//! let blockers = analyzer.find_blockers("test#task1");
//!
//! assert_eq!(blockers.len(), 1);
//! assert_eq!(blockers[0].task_id, "test#task2");
//! assert_eq!(blockers[0].depth, 0); // Direct blocker
//! ```

use super::graph::DependencyGraph;
use super::status_computer::ComputedStatus;
use lash_types::DependencyKind;
use std::collections::{HashMap, HashSet, VecDeque};

/// Information about a single blocking task
///
/// Provides details about why a task is blocked, including the blocker's depth
/// in the dependency chain and the reason for the blockage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerInfo {
    /// ID of the blocking task
    pub task_id: String,

    /// Title of the blocking task (for display)
    pub title: String,

    /// File containing the blocking task
    pub file_id: String,

    /// Depth in the blocker chain (0 = direct blocker, 1+ = transitive)
    pub depth: usize,

    /// Type of dependency causing the blockage
    pub dependency_kind: DependencyKind,

    /// Computed status of the blocking task
    pub blocker_status: ComputedStatus,
}

impl BlockerInfo {
    /// Create a new blocker info
    #[must_use]
    pub fn new(
        task_id: String,
        title: String,
        file_id: String,
        depth: usize,
        dependency_kind: DependencyKind,
        blocker_status: ComputedStatus,
    ) -> Self {
        Self {
            task_id,
            title,
            file_id,
            depth,
            dependency_kind,
            blocker_status,
        }
    }

    /// Check if this blocker is a direct dependency
    #[must_use]
    pub fn is_direct(&self) -> bool {
        self.depth == 0
    }

    /// Check if this blocker is itself blocked (transitive blockage)
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.blocker_status.is_blocked()
    }

    /// Check if this blocker is a root blocker (no incomplete dependencies)
    #[must_use]
    pub fn is_root(&self) -> bool {
        matches!(self.blocker_status, ComputedStatus::Incomplete)
    }
}

/// Blocker chain showing recursive blocker relationships
///
/// Represents a path through the dependency graph showing how one task is blocked
/// by another, which may itself be blocked by yet another task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerChain {
    /// The task being blocked
    pub blocked_task_id: String,

    /// Chain of blockers from direct to root
    /// First element is the direct blocker, last is a root blocker
    pub chain: Vec<BlockerInfo>,
}

impl BlockerChain {
    /// Create a new blocker chain
    #[must_use]
    pub fn new(blocked_task_id: String, chain: Vec<BlockerInfo>) -> Self {
        Self {
            blocked_task_id,
            chain,
        }
    }

    /// Get the root blocker (last in chain)
    #[must_use]
    pub fn root_blocker(&self) -> Option<&BlockerInfo> {
        self.chain.last()
    }

    /// Get the direct blocker (first in chain)
    #[must_use]
    pub fn direct_blocker(&self) -> Option<&BlockerInfo> {
        self.chain.first()
    }

    /// Get the length of the blocker chain
    #[must_use]
    pub fn len(&self) -> usize {
        self.chain.len()
    }

    /// Check if the chain is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }
}

/// Actionable suggestion for resolving a blocker
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockerSuggestion {
    /// Complete the blocking task
    CompleteTask {
        /// ID of the task that must be completed
        task_id: String,
        /// Human-readable title of the blocking task
        title: String,
    },

    /// Waive the blocking task (mark as not applicable)
    WaiveTask {
        /// ID of the task to waive
        task_id: String,
        /// Human-readable title of the task to waive
        title: String,
    },

    /// Remove the dependency relationship
    RemoveDependency {
        /// ID of the dependent task (the one that is blocked)
        from_task_id: String,
        /// ID of the blocking task (the dependency to remove)
        to_task_id: String,
    },

    /// Waive the dependent task itself
    WaiveDependentTask {
        /// ID of the dependent task to waive
        task_id: String,
        /// Human-readable title of the dependent task
        title: String,
    },
}

/// Formatted blocker report for user display
///
/// Provides human-readable information about blockers with actionable suggestions
/// for resolving them.
#[derive(Debug, Clone)]
pub struct BlockerReport {
    /// Task being analyzed
    pub task_id: String,

    /// Title of the task
    pub task_title: String,

    /// List of all blockers (sorted by depth)
    pub blockers: Vec<BlockerInfo>,

    /// Blocker chains showing transitive relationships
    pub chains: Vec<BlockerChain>,

    /// Root blockers (fundamental blockers to address first)
    pub root_blockers: Vec<BlockerInfo>,

    /// Actionable suggestions
    pub suggestions: Vec<BlockerSuggestion>,
}

impl BlockerReport {
    /// Create a new blocker report
    #[must_use]
    pub fn new(
        task_id: String,
        task_title: String,
        blockers: Vec<BlockerInfo>,
        chains: Vec<BlockerChain>,
        root_blockers: Vec<BlockerInfo>,
        suggestions: Vec<BlockerSuggestion>,
    ) -> Self {
        Self {
            task_id,
            task_title,
            blockers,
            chains,
            root_blockers,
            suggestions,
        }
    }

    /// Format the report as a human-readable string
    ///
    /// Produces a multi-line report with:
    /// - Summary of blockers
    /// - Blocker chains showing dependencies
    /// - Root blockers to address first
    /// - Actionable suggestions
    #[must_use]
    pub fn format(&self) -> String {
        let mut output = Vec::new();

        output.push(format!("Blocker Analysis for: {}", self.task_title));
        output.push(format!("Task ID: {}", self.task_id));
        output.push(String::new());

        if self.blockers.is_empty() {
            output.push("This task has no blockers and is ready to work on.".to_string());
            return output.join("\n");
        }

        // Summary
        output.push(format!("Total blockers: {}", self.blockers.len()));
        let direct_count = self.blockers.iter().filter(|b| b.is_direct()).count();
        output.push(format!("  - Direct blockers: {direct_count}"));
        output.push(format!(
            "  - Transitive blockers: {}",
            self.blockers.len() - direct_count
        ));
        output.push(String::new());

        // Root blockers (most important)
        if !self.root_blockers.is_empty() {
            output.push("Root Blockers (address these first):".to_string());
            for blocker in &self.root_blockers {
                output.push(format!("  - {} (in {})", blocker.title, blocker.file_id));
            }
            output.push(String::new());
        }

        // Blocker chains
        if !self.chains.is_empty() {
            output.push("Blocker Chains:".to_string());
            for chain in &self.chains {
                let chain_str: Vec<String> = chain
                    .chain
                    .iter()
                    .map(|b| format!("{} ({})", b.title, b.file_id))
                    .collect();
                output.push(format!("  {}", chain_str.join(" → ")));
            }
            output.push(String::new());
        }

        // All blockers (detailed)
        output.push("All Blockers:".to_string());
        for blocker in &self.blockers {
            let depth_str = if blocker.is_direct() {
                "direct".to_string()
            } else {
                format!("depth {}", blocker.depth)
            };
            let status_str = if blocker.is_blocked() {
                "blocked"
            } else if blocker.is_root() {
                "incomplete"
            } else {
                "unknown"
            };
            output.push(format!(
                "  - {} (in {}) [{}] [{}]",
                blocker.title, blocker.file_id, depth_str, status_str
            ));
        }
        output.push(String::new());

        // Suggestions
        if !self.suggestions.is_empty() {
            output.push("Suggested Actions:".to_string());
            for suggestion in &self.suggestions {
                match suggestion {
                    BlockerSuggestion::CompleteTask { task_id, title } => {
                        output.push(format!("  - Complete task: {title} ({task_id})"));
                    }
                    BlockerSuggestion::WaiveTask { task_id, title } => {
                        output.push(format!(
                            "  - Waive task if not applicable: {title} ({task_id})"
                        ));
                    }
                    BlockerSuggestion::RemoveDependency {
                        from_task_id,
                        to_task_id,
                    } => {
                        output.push(format!(
                            "  - Remove dependency: {from_task_id} no longer depends on {to_task_id}"
                        ));
                    }
                    BlockerSuggestion::WaiveDependentTask { task_id, title } => {
                        output.push(format!(
                            "  - Waive this task if not needed: {title} ({task_id})"
                        ));
                    }
                }
            }
        }

        output.join("\n")
    }
}

/// Blocker analyzer for identifying and analyzing blocking dependencies
///
/// The `BlockerAnalyzer` examines a task's dependencies to identify which ones are
/// preventing the task from being completed. It provides detailed analysis including:
/// - Direct vs transitive blockers
/// - Blocker chains showing dependency paths
/// - Root blockers (fundamental blockers to address first)
/// - Actionable suggestions for resolution
///
/// # Example
///
/// ```
/// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, StatusComputer, BlockerAnalyzer};
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
/// let computer = StatusComputer::new(&graph);
/// let statuses = computer.compute_all();
/// let analyzer = BlockerAnalyzer::new(&graph, &statuses);
///
/// let blockers = analyzer.find_blockers("test#task1");
/// assert_eq!(blockers.len(), 1);
/// ```
pub struct BlockerAnalyzer<'a> {
    /// Reference to the dependency graph
    graph: &'a DependencyGraph,

    /// Computed statuses for all tasks
    statuses: &'a HashMap<String, ComputedStatus>,
}

impl<'a> BlockerAnalyzer<'a> {
    /// Create a new blocker analyzer
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, StatusComputer, BlockerAnalyzer};
    /// use std::collections::HashMap;
    ///
    /// let graph = DependencyGraph::new();
    /// let computer = StatusComputer::new(&graph);
    /// let statuses = computer.compute_all();
    /// let analyzer = BlockerAnalyzer::new(&graph, &statuses);
    /// ```
    #[must_use]
    pub fn new(graph: &'a DependencyGraph, statuses: &'a HashMap<String, ComputedStatus>) -> Self {
        Self { graph, statuses }
    }

    /// Find all blockers for a given task
    ///
    /// Returns a list of blockers sorted by depth (direct blockers first, then transitive).
    /// Returns an empty list if the task has no blockers.
    ///
    /// # Performance
    ///
    /// O(D) for direct blockers only, O(V+E) worst case for full transitive analysis.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, StatusComputer, BlockerAnalyzer};
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
    /// let computer = StatusComputer::new(&graph);
    /// let statuses = computer.compute_all();
    /// let analyzer = BlockerAnalyzer::new(&graph, &statuses);
    ///
    /// let blockers = analyzer.find_blockers("test#task1");
    /// assert_eq!(blockers.len(), 1);
    /// ```
    #[must_use]
    pub fn find_blockers(&self, task_id: &str) -> Vec<BlockerInfo> {
        // Get the task's computed status
        let Some(status) = self.statuses.get(task_id) else {
            return Vec::new();
        };

        // If not blocked, no blockers
        if !status.is_blocked() {
            return Vec::new();
        }

        // BFS to find all blockers with depth tracking
        let mut blockers = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with direct dependencies
        if let Some(deps) = self.graph.get_dependencies(task_id) {
            for dep in deps {
                let dep_id = &dep.target_id;
                if let Some(dep_status) = self.statuses.get(dep_id) {
                    // Only add if it's actually blocking
                    if !dep_status.is_complete() {
                        queue.push_back((dep_id.clone(), 0));
                        visited.insert(dep_id.clone());
                    }
                }
            }
        }

        // BFS to find transitive blockers
        while let Some((current_id, depth)) = queue.pop_front() {
            // Get node data
            let Some(node) = self.graph.get_node(&current_id) else {
                continue;
            };

            // Get status
            let Some(current_status) = self.statuses.get(&current_id) else {
                continue;
            };

            // Get dependency kind from edge
            let dependency_kind = if depth == 0 {
                // Direct dependency - get the actual edge kind
                self.graph
                    .get_edge(task_id, &current_id)
                    .map_or(DependencyKind::ExplicitId, |e| e.kind.clone())
            } else {
                // Transitive - just mark as ExplicitId
                DependencyKind::ExplicitId
            };

            // Add this blocker
            blockers.push(BlockerInfo::new(
                current_id.clone(),
                node.title.clone(),
                node.file_id.clone(),
                depth,
                dependency_kind,
                current_status.clone(),
            ));

            // If this blocker is itself blocked, follow its blockers
            if current_status.is_blocked() {
                if let Some(deps) = self.graph.get_dependencies(&current_id) {
                    for dep in deps {
                        let dep_id = &dep.target_id;
                        if !visited.contains(dep_id) {
                            if let Some(dep_status) = self.statuses.get(dep_id) {
                                if !dep_status.is_complete() {
                                    queue.push_back((dep_id.clone(), depth + 1));
                                    visited.insert(dep_id.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort by depth (direct blockers first)
        blockers.sort_by_key(|b| b.depth);

        blockers
    }

    /// Build blocker chains showing transitive blocker relationships
    ///
    /// For each direct blocker, recursively finds its blockers to build chains
    /// from the dependent task down to root blockers.
    ///
    /// # Panics
    ///
    /// May panic if the graph structure is inconsistent (node exists in edges but not in nodes map).
    /// This should not happen with a properly constructed graph.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, StatusComputer, BlockerAnalyzer};
    /// use lash_types::{TaskStatus, DependencyKind};
    ///
    /// // Create a chain: task1 → task2 → task3
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node("test#task1".to_string(),
    ///     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0));
    /// graph.add_node("test#task2".to_string(),
    ///     NodeData::new("Task 2".to_string(), TaskStatus::Open, "test".to_string(), 0));
    /// graph.add_node("test#task3".to_string(),
    ///     NodeData::new("Task 3".to_string(), TaskStatus::Open, "test".to_string(), 0));
    ///
    /// graph.add_edge("test#task1".to_string(), "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None));
    /// graph.add_edge("test#task2".to_string(), "test#task3".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None));
    ///
    /// let computer = StatusComputer::new(&graph);
    /// let statuses = computer.compute_all();
    /// let analyzer = BlockerAnalyzer::new(&graph, &statuses);
    ///
    /// let chains = analyzer.find_blocker_chains("test#task1");
    /// assert_eq!(chains.len(), 1);
    /// assert_eq!(chains[0].chain.len(), 2); // task2 → task3
    /// ```
    #[must_use]
    pub fn find_blocker_chains(&self, task_id: &str) -> Vec<BlockerChain> {
        let blockers = self.find_blockers(task_id);
        let mut chains = Vec::new();

        // Get direct blockers
        let direct_blockers: Vec<&BlockerInfo> =
            blockers.iter().filter(|b| b.is_direct()).collect();

        // For each direct blocker, build a chain
        for direct_blocker in direct_blockers {
            let mut chain = vec![direct_blocker.clone()];

            // Follow the blocker chain until we hit a root blocker
            let mut current_id = direct_blocker.task_id.clone();
            let mut visited = HashSet::new();
            visited.insert(current_id.clone());

            loop {
                let current_status = self.statuses.get(&current_id);

                // If not blocked or complete, we've reached a root blocker
                if current_status.is_none()
                    || !current_status.unwrap().is_blocked()
                    || current_status.unwrap().is_complete()
                {
                    break;
                }

                // Find the next blocker in the chain
                let mut found_next = false;
                if let Some(deps) = self.graph.get_dependencies(&current_id) {
                    for dep in deps {
                        let dep_id = &dep.target_id;
                        if visited.contains(dep_id) {
                            continue; // Skip cycles
                        }

                        if let Some(dep_status) = self.statuses.get(dep_id) {
                            if !dep_status.is_complete() {
                                // Found next blocker
                                let node = self.graph.get_node(dep_id).unwrap();
                                chain.push(BlockerInfo::new(
                                    dep_id.clone(),
                                    node.title.clone(),
                                    node.file_id.clone(),
                                    chain.len(),
                                    self.graph
                                        .get_edge(&current_id, dep_id)
                                        .map_or(DependencyKind::ExplicitId, |e| e.kind.clone()),
                                    dep_status.clone(),
                                ));
                                current_id = dep_id.clone();
                                visited.insert(current_id.clone());
                                found_next = true;
                                break;
                            }
                        }
                    }
                }

                if !found_next {
                    break;
                }
            }

            chains.push(BlockerChain::new(task_id.to_string(), chain));
        }

        chains
    }

    /// Identify root blockers (blockers with no incomplete dependencies)
    ///
    /// Root blockers are the fundamental tasks that need to be addressed first.
    /// Completing these will unblock other tasks down the chain.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, StatusComputer, BlockerAnalyzer};
    /// use lash_types::{TaskStatus, DependencyKind};
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node("test#task1".to_string(),
    ///     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0));
    /// graph.add_node("test#task2".to_string(),
    ///     NodeData::new("Task 2".to_string(), TaskStatus::Open, "test".to_string(), 0));
    /// graph.add_edge("test#task1".to_string(), "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None));
    ///
    /// let computer = StatusComputer::new(&graph);
    /// let statuses = computer.compute_all();
    /// let analyzer = BlockerAnalyzer::new(&graph, &statuses);
    ///
    /// let roots = analyzer.find_root_blockers("test#task1");
    /// assert_eq!(roots.len(), 1);
    /// assert_eq!(roots[0].task_id, "test#task2");
    /// ```
    #[must_use]
    pub fn find_root_blockers(&self, task_id: &str) -> Vec<BlockerInfo> {
        let blockers = self.find_blockers(task_id);
        blockers.into_iter().filter(BlockerInfo::is_root).collect()
    }

    /// Generate a comprehensive blocker report
    ///
    /// Produces a complete analysis with all blocker information and actionable
    /// suggestions for resolving the blockage.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, StatusComputer, BlockerAnalyzer};
    /// use lash_types::{TaskStatus, DependencyKind};
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node("test#task1".to_string(),
    ///     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0));
    /// graph.add_node("test#task2".to_string(),
    ///     NodeData::new("Task 2".to_string(), TaskStatus::Open, "test".to_string(), 0));
    /// graph.add_edge("test#task1".to_string(), "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None));
    ///
    /// let computer = StatusComputer::new(&graph);
    /// let statuses = computer.compute_all();
    /// let analyzer = BlockerAnalyzer::new(&graph, &statuses);
    ///
    /// let report = analyzer.generate_report("test#task1");
    /// assert!(!report.blockers.is_empty());
    /// println!("{}", report.format());
    /// ```
    #[must_use]
    pub fn generate_report(&self, task_id: &str) -> BlockerReport {
        let task_title = self
            .graph
            .get_node(task_id)
            .map_or_else(|| task_id.to_string(), |n| n.title.clone());

        let blockers = self.find_blockers(task_id);
        let chains = self.find_blocker_chains(task_id);
        let root_blockers = self.find_root_blockers(task_id);

        // Generate suggestions
        let mut suggestions = Vec::new();

        // Suggest completing root blockers first
        for root in &root_blockers {
            suggestions.push(BlockerSuggestion::CompleteTask {
                task_id: root.task_id.clone(),
                title: root.title.clone(),
            });
        }

        // Suggest waiving if applicable
        for blocker in &blockers {
            if blocker.is_direct() {
                suggestions.push(BlockerSuggestion::WaiveTask {
                    task_id: blocker.task_id.clone(),
                    title: blocker.title.clone(),
                });
            }
        }

        // Suggest removing direct dependencies
        for blocker in &blockers {
            if blocker.is_direct() {
                suggestions.push(BlockerSuggestion::RemoveDependency {
                    from_task_id: task_id.to_string(),
                    to_task_id: blocker.task_id.clone(),
                });
            }
        }

        // Suggest waiving the task itself as last resort
        suggestions.push(BlockerSuggestion::WaiveDependentTask {
            task_id: task_id.to_string(),
            title: task_title.clone(),
        });

        BlockerReport::new(
            task_id.to_string(),
            task_title,
            blockers,
            chains,
            root_blockers,
            suggestions,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency::{EdgeData, NodeData, StatusComputer};
    use lash_types::TaskStatus;

    fn create_node(title: &str, status: TaskStatus, file: &str, depth: u8) -> NodeData {
        NodeData::new(title.to_string(), status, file.to_string(), depth)
    }

    #[test]
    fn test_task_with_direct_blocker() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, "test", 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, "test", 0),
        );
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();
        let analyzer = BlockerAnalyzer::new(&graph, &statuses);

        let blockers = analyzer.find_blockers("test#task1");
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].task_id, "test#task2");
        assert_eq!(blockers[0].depth, 0);
        assert!(blockers[0].is_direct());
        assert!(blockers[0].is_root());
    }

    #[test]
    fn test_task_with_transitive_blocker_chain() {
        let mut graph = DependencyGraph::new();
        // Chain: task1 → task2 → task3
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, "test", 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, "test", 0),
        );
        graph.add_node(
            "test#task3".to_string(),
            create_node("Task 3", TaskStatus::Open, "test", 0),
        );

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

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();
        let analyzer = BlockerAnalyzer::new(&graph, &statuses);

        let blockers = analyzer.find_blockers("test#task1");
        assert_eq!(blockers.len(), 2);

        // task2 is a direct blocker
        assert_eq!(blockers[0].task_id, "test#task2");
        assert_eq!(blockers[0].depth, 0);
        assert!(blockers[0].is_direct());

        // task3 is a transitive blocker
        assert_eq!(blockers[1].task_id, "test#task3");
        assert_eq!(blockers[1].depth, 1);
        assert!(!blockers[1].is_direct());

        // task3 is the root blocker
        let roots = analyzer.find_root_blockers("test#task1");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].task_id, "test#task3");
    }

    #[test]
    fn test_task_with_multiple_independent_blockers() {
        let mut graph = DependencyGraph::new();
        // task1 depends on both task2 and task3 (independent)
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, "test", 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, "test", 0),
        );
        graph.add_node(
            "test#task3".to_string(),
            create_node("Task 3", TaskStatus::Open, "test", 0),
        );

        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#task1".to_string(),
            "test#task3".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();
        let analyzer = BlockerAnalyzer::new(&graph, &statuses);

        let blockers = analyzer.find_blockers("test#task1");
        assert_eq!(blockers.len(), 2);

        // Both are direct blockers
        assert!(blockers[0].is_direct());
        assert!(blockers[1].is_direct());

        // Both are root blockers
        let roots = analyzer.find_root_blockers("test#task1");
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn test_task_with_no_blockers_ready_to_start() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, "test", 0),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();
        let analyzer = BlockerAnalyzer::new(&graph, &statuses);

        let blockers = analyzer.find_blockers("test#task1");
        assert_eq!(blockers.len(), 0);

        let report = analyzer.generate_report("test#task1");
        assert!(report.format().contains("no blockers"));
    }

    #[test]
    fn test_blocker_chains() {
        let mut graph = DependencyGraph::new();
        // Chain: task1 → task2 → task3
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, "test", 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, "test", 0),
        );
        graph.add_node(
            "test#task3".to_string(),
            create_node("Task 3", TaskStatus::Open, "test", 0),
        );

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

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();
        let analyzer = BlockerAnalyzer::new(&graph, &statuses);

        let chains = analyzer.find_blocker_chains("test#task1");
        assert_eq!(chains.len(), 1);

        let chain = &chains[0];
        assert_eq!(chain.blocked_task_id, "test#task1");
        assert_eq!(chain.len(), 2);

        // Chain should be task2 → task3
        assert_eq!(chain.direct_blocker().unwrap().task_id, "test#task2");
        assert_eq!(chain.root_blocker().unwrap().task_id, "test#task3");
    }

    #[test]
    fn test_blocker_report_generation() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, "test", 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, "test", 0),
        );
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();
        let analyzer = BlockerAnalyzer::new(&graph, &statuses);

        let report = analyzer.generate_report("test#task1");

        assert_eq!(report.task_id, "test#task1");
        assert_eq!(report.task_title, "Task 1");
        assert_eq!(report.blockers.len(), 1);
        assert_eq!(report.root_blockers.len(), 1);
        assert!(!report.suggestions.is_empty());

        // Verify formatted output contains key information
        let formatted = report.format();
        assert!(formatted.contains("Task 1"));
        assert!(formatted.contains("Task 2"));
        assert!(formatted.contains("Total blockers: 1"));
        assert!(formatted.contains("Root Blockers"));
        assert!(formatted.contains("Suggested Actions"));
    }

    #[test]
    fn test_completed_dependency_not_blocker() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, "test", 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Done, "test", 0),
        );
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();
        let analyzer = BlockerAnalyzer::new(&graph, &statuses);

        // task2 is done, so it shouldn't block task1
        let blockers = analyzer.find_blockers("test#task1");
        assert_eq!(blockers.len(), 0);
    }
}
