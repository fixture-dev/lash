//! Completion status computation for task dependencies
//!
//! This module computes the effective completion status of each task based on its
//! own status and its dependencies. The computation follows the completion semantics
//! defined in the design document (section 5.4):
//!
//! - A task is **complete** if:
//!   - Own status is `done`, AND
//!   - All children (hierarchy dependencies) are `done` or `waived`, AND
//!   - All explicit dependencies are complete or waived
//!
//! - A task is **blocked** if:
//!   - Any dependency is `open` or `blocked` (not waived), OR
//!   - Depends on a broken link
//!
//! - A task is **inconsistent** if:
//!   - Marked `done` but has incomplete dependencies
//!   - Parent marked `done` but children are not complete
//!
//! # Algorithm
//!
//! The status computer uses a topological traversal with memoization to compute
//! statuses efficiently in O(V+E) time. It processes tasks in dependency order,
//! ensuring all dependencies are evaluated before their dependents.
//!
//! # Example
//!
//! ```
//! use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, StatusComputer};
//! use lash_types::{TaskStatus, DependencyKind};
//!
//! // Create a simple graph: task1 depends on task2
//! let mut graph = DependencyGraph::new();
//! graph.add_node(
//!     "test#task1".to_string(),
//!     NodeData::new("Task 1".to_string(), TaskStatus::Done, "test".to_string(), 0)
//! );
//! graph.add_node(
//!     "test#task2".to_string(),
//!     NodeData::new("Task 2".to_string(), TaskStatus::Done, "test".to_string(), 0)
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
//! // Both tasks should be complete
//! assert!(statuses.get("test#task1").unwrap().is_complete());
//! assert!(statuses.get("test#task2").unwrap().is_complete());
//! ```

use lash_types::{DependencyKind, TaskStatus};
use std::collections::{HashMap, HashSet};

use super::graph::DependencyGraph;

/// Computed completion status for a task
///
/// This represents the effective status after analyzing all dependencies,
/// not just the task's own status. It can identify blockers, inconsistencies,
/// and provide detailed reasons for the computed status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputedStatus {
    /// Task is complete (done/waived with all dependencies satisfied)
    Complete,

    /// Task is incomplete (not done, but dependencies allow progress)
    Incomplete,

    /// Task is blocked by incomplete dependencies
    Blocked(Vec<BlockerReason>),

    /// Task status is inconsistent with dependencies
    Inconsistent(Vec<InconsistencyKind>),
}

impl ComputedStatus {
    /// Check if this status represents completion
    #[must_use]
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Check if this status represents being blocked
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked(_))
    }

    /// Check if this status represents an inconsistency
    #[must_use]
    pub fn is_inconsistent(&self) -> bool {
        matches!(self, Self::Inconsistent(_))
    }

    /// Get blocker reasons if blocked
    #[must_use]
    pub fn blocker_reasons(&self) -> Option<&[BlockerReason]> {
        match self {
            Self::Blocked(reasons) => Some(reasons),
            _ => None,
        }
    }

    /// Get inconsistencies if inconsistent
    #[must_use]
    pub fn inconsistencies(&self) -> Option<&[InconsistencyKind]> {
        match self {
            Self::Inconsistent(kinds) => Some(kinds),
            _ => None,
        }
    }
}

/// Reason why a task is blocked
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockerReason {
    /// Dependency is not complete
    IncompleteDependency {
        /// Task ID of the blocking dependency
        task_id: String,
        /// Type of dependency
        kind: DependencyKind,
    },

    /// Dependency is itself blocked
    BlockedDependency {
        /// Task ID of the blocked dependency
        task_id: String,
    },

    /// Dependency reference is broken (target doesn't exist)
    BrokenLink {
        /// The broken reference
        reference: String,
    },

    /// Circular dependency detected
    CircularDependency {
        /// Task IDs involved in the cycle
        cycle: Vec<String>,
    },
}

/// Type of status inconsistency
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InconsistencyKind {
    /// Task marked done but has incomplete dependencies
    DoneWithIncompleteDependencies {
        /// IDs of incomplete dependencies
        incomplete_deps: Vec<String>,
    },

    /// Parent task marked done but children are incomplete
    ParentDoneChildrenIncomplete {
        /// IDs of incomplete children
        incomplete_children: Vec<String>,
    },

    /// Task marked done but explicit dependencies are incomplete
    DoneWithIncompleteExplicitDeps {
        /// IDs of incomplete explicit dependencies
        incomplete_deps: Vec<String>,
    },
}

/// Status computer for computing effective completion status
///
/// The `StatusComputer` analyzes a dependency graph and computes the effective
/// completion status for each task, taking into account both the task's own
/// status and the status of all its dependencies.
///
/// # Example
///
/// ```
/// use lash_core::dependency::{DependencyGraph, NodeData, StatusComputer};
/// use lash_types::TaskStatus;
///
/// let mut graph = DependencyGraph::new();
/// graph.add_node(
///     "test#task1".to_string(),
///     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0)
/// );
///
/// let computer = StatusComputer::new(&graph);
/// let statuses = computer.compute_all();
///
/// // Open task with no dependencies should be incomplete
/// assert!(!statuses.get("test#task1").unwrap().is_complete());
/// ```
pub struct StatusComputer<'a> {
    /// Reference to the dependency graph
    graph: &'a DependencyGraph,

    /// Memoized computed statuses
    cache: HashMap<String, ComputedStatus>,

    /// Tasks currently being processed (for cycle detection)
    visiting: HashSet<String>,
}

impl<'a> StatusComputer<'a> {
    /// Create a new status computer for the given graph
    #[must_use]
    pub fn new(graph: &'a DependencyGraph) -> Self {
        Self {
            graph,
            cache: HashMap::new(),
            visiting: HashSet::new(),
        }
    }

    /// Compute status for all tasks in the graph
    ///
    /// Returns a map from task ID to computed status. Uses memoization to avoid
    /// recomputing statuses for tasks with shared dependencies.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, StatusComputer};
    /// use lash_types::TaskStatus;
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node(
    ///     "test#task1".to_string(),
    ///     NodeData::new("Task 1".to_string(), TaskStatus::Done, "test".to_string(), 0)
    /// );
    ///
    /// let computer = StatusComputer::new(&graph);
    /// let statuses = computer.compute_all();
    ///
    /// assert_eq!(statuses.len(), 1);
    /// assert!(statuses.contains_key("test#task1"));
    /// ```
    #[must_use]
    pub fn compute_all(mut self) -> HashMap<String, ComputedStatus> {
        // Get all task IDs
        let task_ids: Vec<String> = self.graph.all_node_ids();

        // Compute status for each task
        for task_id in task_ids {
            if !self.cache.contains_key(&task_id) {
                self.compute_status(&task_id);
            }
        }

        self.cache
    }

    /// Compute status for a single task
    ///
    /// Uses recursive DFS with memoization. Returns the computed status.
    /// If the status was already computed, returns the cached value.
    fn compute_status(&mut self, task_id: &str) -> ComputedStatus {
        // Check cache first
        if let Some(status) = self.cache.get(task_id) {
            return status.clone();
        }

        // Check for cycles
        if self.visiting.contains(task_id) {
            let status = ComputedStatus::Blocked(vec![BlockerReason::CircularDependency {
                cycle: vec![task_id.to_string()],
            }]);
            return status;
        }

        // Get node data
        let Some(node) = self.graph.get_node(task_id) else {
            // Task doesn't exist in graph - treat as incomplete
            let status = ComputedStatus::Incomplete;
            self.cache.insert(task_id.to_string(), status.clone());
            return status;
        };

        // Mark as visiting
        self.visiting.insert(task_id.to_string());

        // Compute status based on task's own status and dependencies
        let status = self.compute_status_internal(task_id, node.status);

        // Remove from visiting set
        self.visiting.remove(task_id);

        // Cache and return
        self.cache.insert(task_id.to_string(), status.clone());
        status
    }

    /// Internal status computation logic
    fn compute_status_internal(&mut self, task_id: &str, own_status: TaskStatus) -> ComputedStatus {
        // If task is waived, it's complete regardless of dependencies
        if own_status == TaskStatus::Waived {
            return ComputedStatus::Complete;
        }

        // Get all dependencies
        let deps = self
            .graph
            .get_dependencies(task_id)
            .map_or(vec![], |d| d.iter().map(|e| e.target_id.clone()).collect());

        // Check each dependency's status
        let mut incomplete_deps = Vec::new();
        let mut blocked_deps = Vec::new();
        let mut blockers = Vec::new();

        for dep_id in &deps {
            let dep_status = self.compute_status(dep_id);

            match dep_status {
                ComputedStatus::Complete => {
                    // Dependency is satisfied
                }
                ComputedStatus::Incomplete => {
                    incomplete_deps.push(dep_id.clone());
                    blockers.push(BlockerReason::IncompleteDependency {
                        task_id: dep_id.clone(),
                        kind: self.get_edge_kind(task_id, dep_id),
                    });
                }
                ComputedStatus::Blocked(_) => {
                    blocked_deps.push(dep_id.clone());
                    blockers.push(BlockerReason::BlockedDependency {
                        task_id: dep_id.clone(),
                    });
                }
                ComputedStatus::Inconsistent(_) => {
                    // Treat inconsistent dependencies as incomplete
                    incomplete_deps.push(dep_id.clone());
                    blockers.push(BlockerReason::IncompleteDependency {
                        task_id: dep_id.clone(),
                        kind: self.get_edge_kind(task_id, dep_id),
                    });
                }
            }
        }

        // Now determine the computed status based on own status and dependencies
        match own_status {
            TaskStatus::Done => {
                if !incomplete_deps.is_empty() || !blocked_deps.is_empty() {
                    // Inconsistent: marked done but dependencies not complete
                    let mut inconsistencies = Vec::new();

                    // Separate hierarchy deps (children) from explicit deps
                    let hierarchy_deps: Vec<String> = deps
                        .iter()
                        .filter(|dep_id| {
                            matches!(
                                self.get_edge_kind(task_id, dep_id),
                                DependencyKind::Hierarchy
                            )
                        })
                        .cloned()
                        .collect();

                    let explicit_deps: Vec<String> = deps
                        .iter()
                        .filter(|dep_id| !hierarchy_deps.contains(dep_id))
                        .cloned()
                        .collect();

                    // Check for incomplete children
                    let incomplete_children: Vec<String> = hierarchy_deps
                        .iter()
                        .filter(|id| incomplete_deps.contains(id) || blocked_deps.contains(id))
                        .cloned()
                        .collect();

                    if !incomplete_children.is_empty() {
                        inconsistencies.push(InconsistencyKind::ParentDoneChildrenIncomplete {
                            incomplete_children,
                        });
                    }

                    // Check for incomplete explicit deps
                    let incomplete_explicit: Vec<String> = explicit_deps
                        .iter()
                        .filter(|id| incomplete_deps.contains(id) || blocked_deps.contains(id))
                        .cloned()
                        .collect();

                    if !incomplete_explicit.is_empty() {
                        inconsistencies.push(InconsistencyKind::DoneWithIncompleteExplicitDeps {
                            incomplete_deps: incomplete_explicit,
                        });
                    }

                    ComputedStatus::Inconsistent(inconsistencies)
                } else {
                    // Task is done and all dependencies are complete
                    ComputedStatus::Complete
                }
            }
            TaskStatus::Open => {
                if blockers.is_empty() {
                    // Task is open and ready to work on
                    ComputedStatus::Incomplete
                } else {
                    // Task is open but blocked by dependencies
                    ComputedStatus::Blocked(blockers)
                }
            }
            TaskStatus::Blocked => {
                // Task explicitly marked as blocked
                if blockers.is_empty() {
                    // No actual blockers found, just marked as blocked
                    ComputedStatus::Blocked(vec![])
                } else {
                    ComputedStatus::Blocked(blockers)
                }
            }
            TaskStatus::Waived => {
                // Already handled above, but included for completeness
                ComputedStatus::Complete
            }
        }
    }

    /// Get the edge kind between two tasks
    fn get_edge_kind(&self, from_id: &str, to_id: &str) -> DependencyKind {
        self.graph
            .get_edge(from_id, to_id)
            .map_or(DependencyKind::ExplicitId, |e| e.kind.clone())
    }

    /// Compute file-level completion status
    ///
    /// A file is complete if all its top-level tasks (depth 0) are complete.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, StatusComputer};
    /// use lash_types::TaskStatus;
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node(
    ///     "test#task1".to_string(),
    ///     NodeData::new("Task 1".to_string(), TaskStatus::Done, "test".to_string(), 0)
    /// );
    /// graph.add_node(
    ///     "test#task2".to_string(),
    ///     NodeData::new("Task 2".to_string(), TaskStatus::Done, "test".to_string(), 0)
    /// );
    ///
    /// let mut computer = StatusComputer::new(&graph);
    /// assert!(computer.compute_file_status("test"));
    /// ```
    #[must_use]
    pub fn compute_file_status(&mut self, file_id: &str) -> bool {
        // Get all top-level tasks for this file (depth 0)
        let top_level_tasks: Vec<String> = self
            .graph
            .all_node_ids()
            .into_iter()
            .filter(|task_id| {
                self.graph
                    .get_node(task_id)
                    .is_some_and(|node| node.file_id == file_id && node.depth == 0)
            })
            .collect();

        // File is complete if all top-level tasks are complete
        top_level_tasks.iter().all(|task_id| {
            let status = self.compute_status(task_id);
            status.is_complete()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency::{EdgeData, NodeData};

    fn create_node(title: &str, status: TaskStatus, depth: u8) -> NodeData {
        NodeData::new(title.to_string(), status, "test".to_string(), depth)
    }

    #[test]
    fn test_single_task_done() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Done, 0),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();

        let status = statuses.get("test#task1").unwrap();
        assert!(status.is_complete());
    }

    #[test]
    fn test_single_task_open() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, 0),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();

        let status = statuses.get("test#task1").unwrap();
        assert!(!status.is_complete());
        assert_eq!(*status, ComputedStatus::Incomplete);
    }

    #[test]
    fn test_single_task_waived() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Waived, 0),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();

        let status = statuses.get("test#task1").unwrap();
        assert!(status.is_complete());
    }

    #[test]
    fn test_chain_all_done() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Done, 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Done, 0),
        );
        graph.add_node(
            "test#task3".to_string(),
            create_node("Task 3", TaskStatus::Done, 0),
        );

        // Chain: task1 -> task2 -> task3
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

        assert!(statuses.get("test#task1").unwrap().is_complete());
        assert!(statuses.get("test#task2").unwrap().is_complete());
        assert!(statuses.get("test#task3").unwrap().is_complete());
    }

    #[test]
    fn test_chain_with_incomplete_dependency() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, 0),
        );

        // task1 depends on task2 (which is open)
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();

        // task2 is incomplete
        let task2_status = statuses.get("test#task2").unwrap();
        assert_eq!(*task2_status, ComputedStatus::Incomplete);

        // task1 is blocked by task2
        let task1_status = statuses.get("test#task1").unwrap();
        assert!(task1_status.is_blocked());
        assert_eq!(task1_status.blocker_reasons().unwrap().len(), 1);
    }

    #[test]
    fn test_waived_dependency_ignored() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Done, 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Waived, 0),
        );

        // task1 depends on task2 (which is waived)
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();

        // Both should be complete (waived dependencies are satisfied)
        assert!(statuses.get("test#task1").unwrap().is_complete());
        assert!(statuses.get("test#task2").unwrap().is_complete());
    }

    #[test]
    fn test_blocked_propagation() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, 0),
        );
        graph.add_node(
            "test#task3".to_string(),
            create_node("Task 3", TaskStatus::Open, 0),
        );

        // Chain: task1 -> task2 -> task3
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

        // task3 is incomplete (no dependencies)
        assert_eq!(
            *statuses.get("test#task3").unwrap(),
            ComputedStatus::Incomplete
        );

        // task2 is blocked by task3
        assert!(statuses.get("test#task2").unwrap().is_blocked());

        // task1 is blocked by task2 (which is itself blocked)
        assert!(statuses.get("test#task1").unwrap().is_blocked());
    }

    #[test]
    fn test_parent_child_inconsistency() {
        let mut graph = DependencyGraph::new();
        // Parent marked done but child is open
        graph.add_node(
            "test#parent".to_string(),
            create_node("Parent", TaskStatus::Done, 0),
        );
        graph.add_node(
            "test#child".to_string(),
            create_node("Child", TaskStatus::Open, 1),
        );

        // Hierarchy dependency: parent -> child
        graph.add_edge(
            "test#parent".to_string(),
            "test#child".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();

        // Child is incomplete
        assert_eq!(
            *statuses.get("test#child").unwrap(),
            ComputedStatus::Incomplete
        );

        // Parent should be inconsistent
        let parent_status = statuses.get("test#parent").unwrap();
        assert!(parent_status.is_inconsistent());

        if let ComputedStatus::Inconsistent(inconsistencies) = parent_status {
            assert_eq!(inconsistencies.len(), 1);
            assert!(matches!(
                inconsistencies[0],
                InconsistencyKind::ParentDoneChildrenIncomplete { .. }
            ));
        } else {
            panic!("Expected inconsistent status");
        }
    }

    #[test]
    fn test_done_with_incomplete_explicit_deps() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Done, 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, 0),
        );

        // Explicit dependency: task1 -> task2
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();

        // task1 should be inconsistent (marked done but dependency incomplete)
        let task1_status = statuses.get("test#task1").unwrap();
        assert!(task1_status.is_inconsistent());
    }

    #[test]
    fn test_file_status_all_complete() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Done, 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Done, 0),
        );

        let mut computer = StatusComputer::new(&graph);
        assert!(computer.compute_file_status("test"));
    }

    #[test]
    fn test_file_status_some_incomplete() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Done, 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, 0),
        );

        let mut computer = StatusComputer::new(&graph);
        assert!(!computer.compute_file_status("test"));
    }

    #[test]
    fn test_file_status_ignores_nested() {
        let mut graph = DependencyGraph::new();
        // Top-level task done
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Done, 0),
        );
        // Nested task (depth 1) open - should not affect file status
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, 1),
        );

        let mut computer = StatusComputer::new(&graph);
        // File should be complete because only top-level tasks matter
        assert!(computer.compute_file_status("test"));
    }

    #[test]
    fn test_multiple_blockers() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, 0),
        );
        graph.add_node(
            "test#task3".to_string(),
            create_node("Task 3", TaskStatus::Open, 0),
        );

        // task1 depends on both task2 and task3
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

        // task1 should be blocked by both dependencies
        let task1_status = statuses.get("test#task1").unwrap();
        assert!(task1_status.is_blocked());
        assert_eq!(task1_status.blocker_reasons().unwrap().len(), 2);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_node("Task 1", TaskStatus::Open, 0),
        );
        graph.add_node(
            "test#task2".to_string(),
            create_node("Task 2", TaskStatus::Open, 0),
        );

        // Create cycle: task1 -> task2 -> task1
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

        let computer = StatusComputer::new(&graph);
        let statuses = computer.compute_all();

        // Both tasks should be blocked due to cycle
        assert!(statuses.get("test#task1").unwrap().is_blocked());
        assert!(statuses.get("test#task2").unwrap().is_blocked());
    }
}
