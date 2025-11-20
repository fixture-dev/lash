//! In-memory dependency graph representation
//!
//! This module provides an efficient in-memory graph structure for representing
//! task dependencies. The graph is built from the database and optimized for:
//!
//! - O(1) direct dependency lookups (forward and reverse)
//! - O(E+V) transitive closure computation
//! - Minimal memory overhead (stores only essential task metadata)
//! - Support for multiple edge types (hierarchy, explicit, directory)
//!
//! # Design
//!
//! The graph uses a HashMap-based adjacency list representation, suitable for
//! sparse graphs typical in task tracking (most tasks have few dependencies).
//!
//! Nodes are identified by their full task ID (`file_id#task_id`), and we maintain:
//! - Forward adjacency list: task → list of dependencies
//! - Reverse adjacency list: task → list of dependents
//! - Edge metadata: (from, to) → edge type and source location
//!
//! # Example
//!
//! ```
//! use lash_core::dependency::{DependencyGraph, NodeData, EdgeData};
//! use lash_types::{TaskStatus, DependencyKind};
//!
//! // Create an empty graph
//! let mut graph = DependencyGraph::new();
//!
//! // Add nodes
//! graph.add_node(
//!     "core.api#setup".to_string(),
//!     NodeData::new("Setup API".to_string(), TaskStatus::Open, "core.api".to_string(), 0)
//! );
//! graph.add_node(
//!     "core.db#init".to_string(),
//!     NodeData::new("Init DB".to_string(), TaskStatus::Open, "core.db".to_string(), 0)
//! );
//!
//! // Add edge (setup depends on init)
//! graph.add_edge(
//!     "core.api#setup".to_string(),
//!     "core.db#init".to_string(),
//!     EdgeData::new(DependencyKind::ExplicitId, None)
//! );
//!
//! // Query the graph
//! assert_eq!(graph.node_count(), 2);
//! assert_eq!(graph.edge_count(), 1);
//! ```

use lash_types::{DependencyKind, TaskStatus};
use std::collections::{HashMap, HashSet, VecDeque};

/// Node metadata stored in the graph
///
/// Contains minimal information needed for graph operations. For full task details,
/// query the database using the task's full ID.
#[derive(Debug, Clone)]
pub struct NodeData {
    /// Task title
    pub title: String,

    /// Current task status
    pub status: TaskStatus,

    /// File ID containing this task
    pub file_id: String,

    /// Nesting depth (0 = top-level)
    pub depth: u8,
}

impl NodeData {
    /// Create new node data
    #[must_use]
    pub fn new(title: String, status: TaskStatus, file_id: String, depth: u8) -> Self {
        Self {
            title,
            status,
            file_id,
            depth,
        }
    }
}

/// Reference to an edge in the graph
///
/// Lightweight reference containing the target node ID and a reference to the edge
/// metadata. Used for efficient edge iteration without copying metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeRef {
    /// Target node ID (for forward edges, this is the dependency; for reverse, the dependent)
    pub target_id: String,

    /// Edge identifier for looking up metadata
    pub edge_id: EdgeId,
}

impl EdgeRef {
    /// Create a new edge reference
    #[must_use]
    pub fn new(target_id: String, edge_id: EdgeId) -> Self {
        Self { target_id, edge_id }
    }
}

/// Unique identifier for an edge
///
/// Edges are uniquely identified by their (from, to) tuple. This allows O(1)
/// lookup of edge metadata.
pub type EdgeId = (String, String);

/// Edge metadata
///
/// Stores the type of dependency relationship and optional source location
/// information for error reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeData {
    /// Type of dependency
    pub kind: DependencyKind,

    /// Optional source location (file path, line number) for explicit dependencies
    pub source_location: Option<String>,
}

impl EdgeData {
    /// Create new edge data
    #[must_use]
    pub fn new(kind: DependencyKind, source_location: Option<String>) -> Self {
        Self {
            kind,
            source_location,
        }
    }
}

/// In-memory task dependency graph
///
/// Efficient graph representation optimized for dependency queries. The graph is
/// built from the database and provides fast lookups for both direct and transitive
/// dependencies.
///
/// # Performance Characteristics
///
/// - Construction: O(V + E) where V = tasks, E = dependencies
/// - Direct queries: O(1) average case (`HashMap` lookup)
/// - Transitive queries: O(E + V) (BFS/DFS traversal)
/// - Memory: O(V + E) with minimal per-node overhead
///
/// # Example
///
/// ```
/// use lash_core::dependency::{DependencyGraph, NodeData};
/// use lash_types::TaskStatus;
///
/// let mut graph = DependencyGraph::new();
/// graph.add_node(
///     "core.api#setup".to_string(),
///     NodeData::new("Setup API".to_string(), TaskStatus::Open, "core.api".to_string(), 0)
/// );
///
/// // Check if a task exists
/// if graph.contains_node("core.api#setup") {
///     println!("Task exists in graph");
/// }
///
/// // Get node metadata
/// if let Some(node) = graph.get_node("core.api#setup") {
///     println!("Task: {} (status: {:?})", node.title, node.status);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Node storage: `full_id` → task metadata
    nodes: HashMap<String, NodeData>,

    /// Forward adjacency list: task → dependencies (tasks this one depends ON)
    adjacency: HashMap<String, Vec<EdgeRef>>,

    /// Reverse adjacency list: task → dependents (tasks that depend on this one)
    reverse: HashMap<String, Vec<EdgeRef>>,

    /// Edge metadata: (`from_id`, `to_id`) → edge type and source info
    edge_metadata: HashMap<EdgeId, EdgeData>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::DependencyGraph;
    ///
    /// let graph = DependencyGraph::new();
    /// assert_eq!(graph.node_count(), 0);
    /// assert_eq!(graph.edge_count(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            adjacency: HashMap::new(),
            reverse: HashMap::new(),
            edge_metadata: HashMap::new(),
        }
    }

    /// Check if a node exists in the graph
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData};
    /// use lash_types::TaskStatus;
    ///
    /// let mut graph = DependencyGraph::new();
    /// let node = NodeData::new(
    ///     "Test Task".to_string(),
    ///     TaskStatus::Open,
    ///     "test".to_string(),
    ///     0
    /// );
    ///
    /// graph.add_node("test#task1".to_string(), node);
    /// assert!(graph.contains_node("test#task1"));
    /// assert!(!graph.contains_node("test#missing"));
    /// ```
    #[must_use]
    pub fn contains_node(&self, task_id: &str) -> bool {
        self.nodes.contains_key(task_id)
    }

    /// Get node metadata
    ///
    /// Returns `None` if the node doesn't exist.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData};
    /// use lash_types::TaskStatus;
    ///
    /// let mut graph = DependencyGraph::new();
    /// let node = NodeData::new(
    ///     "Test Task".to_string(),
    ///     TaskStatus::Open,
    ///     "test".to_string(),
    ///     0
    /// );
    ///
    /// graph.add_node("test#task1".to_string(), node);
    ///
    /// if let Some(node_data) = graph.get_node("test#task1") {
    ///     assert_eq!(node_data.title, "Test Task");
    ///     assert_eq!(node_data.status, TaskStatus::Open);
    /// }
    /// ```
    #[must_use]
    pub fn get_node(&self, task_id: &str) -> Option<&NodeData> {
        self.nodes.get(task_id)
    }

    /// Get the number of nodes in the graph
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData};
    /// use lash_types::TaskStatus;
    ///
    /// let mut graph = DependencyGraph::new();
    /// assert_eq!(graph.node_count(), 0);
    ///
    /// graph.add_node(
    ///     "test#task1".to_string(),
    ///     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    /// assert_eq!(graph.node_count(), 1);
    /// ```
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges in the graph
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData};
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
    ///
    /// assert_eq!(graph.edge_count(), 0);
    ///
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    /// assert_eq!(graph.edge_count(), 1);
    /// ```
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edge_metadata.len()
    }

    /// Get all node IDs in the graph
    ///
    /// Returns a vector containing all task IDs in the graph. The order is
    /// unspecified.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData};
    /// use lash_types::TaskStatus;
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
    ///
    /// let ids = graph.all_node_ids();
    /// assert_eq!(ids.len(), 2);
    /// assert!(ids.contains(&"test#task1".to_string()));
    /// assert!(ids.contains(&"test#task2".to_string()));
    /// ```
    #[must_use]
    pub fn all_node_ids(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }

    /// Add a node to the graph
    ///
    /// If a node with the same ID already exists, it will be replaced.
    pub fn add_node(&mut self, task_id: String, node_data: NodeData) {
        self.nodes.insert(task_id.clone(), node_data);
        // Initialize adjacency lists if not present
        self.adjacency.entry(task_id.clone()).or_default();
        self.reverse.entry(task_id).or_default();
    }

    /// Add an edge to the graph
    ///
    /// Creates a dependency relationship from `from_id` to `to_id`. Both nodes must
    /// already exist in the graph.
    ///
    /// # Panics
    ///
    /// Panics if either node doesn't exist in the graph. Use [`contains_node`] to check first.
    ///
    /// [`contains_node`]: Self::contains_node
    pub fn add_edge(&mut self, from_id: String, to_id: String, edge_data: EdgeData) {
        assert!(
            self.nodes.contains_key(&from_id),
            "Source node '{from_id}' must exist before adding edge",
        );
        assert!(
            self.nodes.contains_key(&to_id),
            "Target node '{to_id}' must exist before adding edge",
        );

        let edge_id = (from_id.clone(), to_id.clone());

        // Add to forward adjacency list
        self.adjacency
            .entry(from_id.clone())
            .or_default()
            .push(EdgeRef::new(to_id.clone(), edge_id.clone()));

        // Add to reverse adjacency list
        self.reverse
            .entry(to_id)
            .or_default()
            .push(EdgeRef::new(from_id, edge_id.clone()));

        // Store edge metadata
        self.edge_metadata.insert(edge_id, edge_data);
    }

    /// Get edge metadata
    ///
    /// Returns `None` if the edge doesn't exist.
    #[must_use]
    pub fn get_edge(&self, from_id: &str, to_id: &str) -> Option<&EdgeData> {
        let edge_id = (from_id.to_string(), to_id.to_string());
        self.edge_metadata.get(&edge_id)
    }

    /// Get direct dependencies for a task
    ///
    /// Returns the list of tasks that the given task depends on (outgoing edges).
    /// Returns `None` if the task doesn't exist in the graph.
    ///
    /// # Performance
    ///
    /// O(1) average case (`HashMap` lookup returning a reference to the edge list).
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData};
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
    ///
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// // task1 depends on task2
    /// let deps = graph.get_dependencies("test#task1").unwrap();
    /// assert_eq!(deps.len(), 1);
    /// assert_eq!(deps[0].target_id, "test#task2");
    /// ```
    #[must_use]
    pub fn get_dependencies(&self, task_id: &str) -> Option<&Vec<EdgeRef>> {
        self.adjacency.get(task_id)
    }

    /// Get direct dependents for a task
    ///
    /// Returns the list of tasks that depend on the given task (incoming edges).
    /// Returns `None` if the task doesn't exist in the graph.
    ///
    /// # Performance
    ///
    /// O(1) average case (`HashMap` lookup returning a reference to the edge list).
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData};
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
    ///
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// // task2 is depended on by task1
    /// let dependents = graph.get_dependents("test#task2").unwrap();
    /// assert_eq!(dependents.len(), 1);
    /// assert_eq!(dependents[0].target_id, "test#task1");
    /// ```
    #[must_use]
    pub fn get_dependents(&self, task_id: &str) -> Option<&Vec<EdgeRef>> {
        self.reverse.get(task_id)
    }

    /// Get dependency IDs (convenience method)
    ///
    /// Returns just the task IDs of dependencies, without edge metadata.
    /// Returns an empty vector if the task doesn't exist.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData};
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
    /// graph.add_node(
    ///     "test#task3".to_string(),
    ///     NodeData::new("Task 3".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    ///
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task3".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// let dep_ids = graph.get_dependency_ids("test#task1");
    /// assert_eq!(dep_ids.len(), 2);
    /// assert!(dep_ids.contains(&"test#task2".to_string()));
    /// assert!(dep_ids.contains(&"test#task3".to_string()));
    /// ```
    #[must_use]
    pub fn get_dependency_ids(&self, task_id: &str) -> Vec<String> {
        self.get_dependencies(task_id)
            .map(|edges| edges.iter().map(|e| e.target_id.clone()).collect())
            .unwrap_or_default()
    }

    /// Get dependent IDs (convenience method)
    ///
    /// Returns just the task IDs of dependents, without edge metadata.
    /// Returns an empty vector if the task doesn't exist.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData};
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
    /// graph.add_node(
    ///     "test#task3".to_string(),
    ///     NodeData::new("Task 3".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    ///
    /// // Both task1 and task3 depend on task2
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    /// graph.add_edge(
    ///     "test#task3".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// let dependent_ids = graph.get_dependent_ids("test#task2");
    /// assert_eq!(dependent_ids.len(), 2);
    /// assert!(dependent_ids.contains(&"test#task1".to_string()));
    /// assert!(dependent_ids.contains(&"test#task3".to_string()));
    /// ```
    #[must_use]
    pub fn get_dependent_ids(&self, task_id: &str) -> Vec<String> {
        self.get_dependents(task_id)
            .map(|edges| edges.iter().map(|e| e.target_id.clone()).collect())
            .unwrap_or_default()
    }

    /// Filter dependencies by kind
    ///
    /// Returns only the dependencies matching the specified kind.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData};
    /// use lash_types::{TaskStatus, DependencyKind};
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node(
    ///     "test#parent".to_string(),
    ///     NodeData::new("Parent".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    /// graph.add_node(
    ///     "test#child".to_string(),
    ///     NodeData::new("Child".to_string(), TaskStatus::Open, "test".to_string(), 1)
    /// );
    /// graph.add_node(
    ///     "test#other".to_string(),
    ///     NodeData::new("Other".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    ///
    /// // Hierarchy dependency: parent → child
    /// graph.add_edge(
    ///     "test#parent".to_string(),
    ///     "test#child".to_string(),
    ///     EdgeData::new(DependencyKind::Hierarchy, None)
    /// );
    /// // Explicit dependency: parent → other
    /// graph.add_edge(
    ///     "test#parent".to_string(),
    ///     "test#other".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// // Get only hierarchy dependencies
    /// let hierarchy_deps = graph.get_dependencies_by_kind("test#parent", &DependencyKind::Hierarchy);
    /// assert_eq!(hierarchy_deps.len(), 1);
    /// assert_eq!(hierarchy_deps[0], "test#child");
    ///
    /// // Get only explicit dependencies
    /// let explicit_deps = graph.get_dependencies_by_kind("test#parent", &DependencyKind::ExplicitId);
    /// assert_eq!(explicit_deps.len(), 1);
    /// assert_eq!(explicit_deps[0], "test#other");
    /// ```
    #[must_use]
    pub fn get_dependencies_by_kind(&self, task_id: &str, kind: &DependencyKind) -> Vec<String> {
        self.get_dependencies(task_id)
            .map(|edges| {
                edges
                    .iter()
                    .filter(|e| {
                        self.edge_metadata
                            .get(&e.edge_id)
                            .is_some_and(|meta| &meta.kind == kind)
                    })
                    .map(|e| e.target_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all descendants (transitive dependencies)
    ///
    /// Returns all tasks that the given task transitively depends on, using BFS traversal.
    /// Detects cycles during traversal and stops if one is found.
    ///
    /// # Performance
    ///
    /// O(V + E) where V is the number of reachable nodes and E is the number of edges.
    ///
    /// # Errors
    ///
    /// Returns error if a cycle is detected during traversal.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData};
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
    /// graph.add_node(
    ///     "test#task3".to_string(),
    ///     NodeData::new("Task 3".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    ///
    /// // Chain: task1 → task2 → task3
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    /// graph.add_edge(
    ///     "test#task2".to_string(),
    ///     "test#task3".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// // task1 transitively depends on both task2 and task3
    /// let descendants = graph.get_descendants("test#task1").unwrap();
    /// assert_eq!(descendants.len(), 2);
    /// assert!(descendants.contains(&"test#task2".to_string()));
    /// assert!(descendants.contains(&"test#task3".to_string()));
    /// ```
    pub fn get_descendants(&self, task_id: &str) -> Result<Vec<String>, String> {
        if !self.contains_node(task_id) {
            return Ok(Vec::new());
        }

        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(task_id.to_string());
        visited.insert(task_id.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(deps) = self.get_dependencies(&current) {
                for dep in deps {
                    let dep_id = &dep.target_id;

                    // Cycle detection: if we're trying to visit the starting node again
                    if dep_id == task_id && !result.is_empty() {
                        return Err(format!("Cycle detected involving task '{task_id}'"));
                    }

                    if !visited.contains(dep_id) {
                        visited.insert(dep_id.clone());
                        result.push(dep_id.clone());
                        queue.push_back(dep_id.clone());
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get all ancestors (transitive dependents)
    ///
    /// Returns all tasks that transitively depend on the given task, using BFS traversal.
    /// Detects cycles during traversal and stops if one is found.
    ///
    /// # Performance
    ///
    /// O(V + E) where V is the number of reachable nodes and E is the number of edges.
    ///
    /// # Errors
    ///
    /// Returns error if a cycle is detected during traversal.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData};
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
    /// graph.add_node(
    ///     "test#task3".to_string(),
    ///     NodeData::new("Task 3".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    ///
    /// // Chain: task1 → task2 → task3
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    /// graph.add_edge(
    ///     "test#task2".to_string(),
    ///     "test#task3".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// // task3 is transitively depended on by both task2 and task1
    /// let ancestors = graph.get_ancestors("test#task3").unwrap();
    /// assert_eq!(ancestors.len(), 2);
    /// assert!(ancestors.contains(&"test#task2".to_string()));
    /// assert!(ancestors.contains(&"test#task1".to_string()));
    /// ```
    pub fn get_ancestors(&self, task_id: &str) -> Result<Vec<String>, String> {
        if !self.contains_node(task_id) {
            return Ok(Vec::new());
        }

        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(task_id.to_string());
        visited.insert(task_id.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(dependents) = self.get_dependents(&current) {
                for dependent in dependents {
                    let dependent_id = &dependent.target_id;

                    // Cycle detection: if we're trying to visit the starting node again
                    if dependent_id == task_id && !result.is_empty() {
                        return Err(format!("Cycle detected involving task '{task_id}'"));
                    }

                    if !visited.contains(dependent_id) {
                        visited.insert(dependent_id.clone());
                        result.push(dependent_id.clone());
                        queue.push_back(dependent_id.clone());
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get descendants with depth limit
    ///
    /// Like `get_descendants`, but stops after reaching a specified depth.
    /// Useful for limiting the scope of dependency analysis.
    ///
    /// # Errors
    ///
    /// Returns error if a cycle is detected during traversal.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData};
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
    /// graph.add_node(
    ///     "test#task3".to_string(),
    ///     NodeData::new("Task 3".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    ///
    /// // Chain: task1 → task2 → task3
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    /// graph.add_edge(
    ///     "test#task2".to_string(),
    ///     "test#task3".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// // With depth 1, only get direct dependencies
    /// let descendants = graph.get_descendants_with_depth("test#task1", 1).unwrap();
    /// assert_eq!(descendants.len(), 1);
    /// assert_eq!(descendants[0], "test#task2");
    ///
    /// // With depth 2, get both
    /// let descendants = graph.get_descendants_with_depth("test#task1", 2).unwrap();
    /// assert_eq!(descendants.len(), 2);
    /// ```
    pub fn get_descendants_with_depth(
        &self,
        task_id: &str,
        max_depth: usize,
    ) -> Result<Vec<String>, String> {
        if !self.contains_node(task_id) {
            return Ok(Vec::new());
        }

        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back((task_id.to_string(), 0));
        visited.insert(task_id.to_string());

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            if let Some(deps) = self.get_dependencies(&current) {
                for dep in deps {
                    let dep_id = &dep.target_id;

                    if dep_id == task_id && !result.is_empty() {
                        return Err(format!("Cycle detected involving task '{task_id}'"));
                    }

                    if !visited.contains(dep_id) {
                        visited.insert(dep_id.clone());
                        result.push(dep_id.clone());
                        queue.push_back((dep_id.clone(), depth + 1));
                    }
                }
            }
        }

        Ok(result)
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node(title: &str) -> NodeData {
        NodeData::new(title.to_string(), TaskStatus::Open, "test".to_string(), 0)
    }

    #[test]
    fn test_new_graph_is_empty() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));

        assert_eq!(graph.node_count(), 1);
        assert!(graph.contains_node("test#task1"));
        assert!(!graph.contains_node("test#missing"));
    }

    #[test]
    fn test_get_node() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));

        let node = graph.get_node("test#task1");
        assert!(node.is_some());
        assert_eq!(node.unwrap().title, "Task 1");

        let missing = graph.get_node("test#missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_add_edge() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_get_edge() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, Some("test.md:10".to_string())),
        );

        let edge = graph.get_edge("test#task1", "test#task2");
        assert!(edge.is_some());

        let edge_data = edge.unwrap();
        assert!(matches!(edge_data.kind, DependencyKind::Hierarchy));
        assert_eq!(edge_data.source_location.as_deref(), Some("test.md:10"));

        let missing = graph.get_edge("test#task2", "test#task1");
        assert!(missing.is_none());
    }

    #[test]
    #[should_panic(expected = "Source node 'test#missing' must exist")]
    fn test_add_edge_missing_source() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));

        graph.add_edge(
            "test#missing".to_string(),
            "test#task1".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
    }

    #[test]
    #[should_panic(expected = "Target node 'test#missing' must exist")]
    fn test_add_edge_missing_target() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));

        graph.add_edge(
            "test#task1".to_string(),
            "test#missing".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
    }

    #[test]
    fn test_multiple_edges_same_node() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

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

        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn test_get_dependencies() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

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

        let deps = graph.get_dependencies("test#task1");
        assert!(deps.is_some());
        assert_eq!(deps.unwrap().len(), 2);

        let missing = graph.get_dependencies("test#missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_dependents() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

        // task1 and task3 both depend on task2
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#task3".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let dependents = graph.get_dependents("test#task2");
        assert!(dependents.is_some());
        assert_eq!(dependents.unwrap().len(), 2);

        let missing = graph.get_dependents("test#missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_dependency_ids() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

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

        let dep_ids = graph.get_dependency_ids("test#task1");
        assert_eq!(dep_ids.len(), 2);
        assert!(dep_ids.contains(&"test#task2".to_string()));
        assert!(dep_ids.contains(&"test#task3".to_string()));

        let missing = graph.get_dependency_ids("test#missing");
        assert_eq!(missing.len(), 0);
    }

    #[test]
    fn test_get_dependent_ids() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#task3".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let dependent_ids = graph.get_dependent_ids("test#task2");
        assert_eq!(dependent_ids.len(), 2);
        assert!(dependent_ids.contains(&"test#task1".to_string()));
        assert!(dependent_ids.contains(&"test#task3".to_string()));

        let missing = graph.get_dependent_ids("test#missing");
        assert_eq!(missing.len(), 0);
    }

    #[test]
    fn test_get_dependencies_by_kind() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#parent".to_string(), create_test_node("Parent"));
        graph.add_node("test#child".to_string(), create_test_node("Child"));
        graph.add_node("test#other".to_string(), create_test_node("Other"));

        graph.add_edge(
            "test#parent".to_string(),
            "test#child".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#parent".to_string(),
            "test#other".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let hierarchy = graph.get_dependencies_by_kind("test#parent", &DependencyKind::Hierarchy);
        assert_eq!(hierarchy.len(), 1);
        assert_eq!(hierarchy[0], "test#child");

        let explicit = graph.get_dependencies_by_kind("test#parent", &DependencyKind::ExplicitId);
        assert_eq!(explicit.len(), 1);
        assert_eq!(explicit[0], "test#other");

        let directory = graph.get_dependencies_by_kind("test#parent", &DependencyKind::Directory);
        assert_eq!(directory.len(), 0);
    }

    #[test]
    fn test_get_descendants_chain() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

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

        let descendants = graph.get_descendants("test#task1").unwrap();
        assert_eq!(descendants.len(), 2);
        assert!(descendants.contains(&"test#task2".to_string()));
        assert!(descendants.contains(&"test#task3".to_string()));

        let descendants = graph.get_descendants("test#task2").unwrap();
        assert_eq!(descendants.len(), 1);
        assert_eq!(descendants[0], "test#task3");

        let descendants = graph.get_descendants("test#task3").unwrap();
        assert_eq!(descendants.len(), 0);
    }

    #[test]
    fn test_get_ancestors_chain() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

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

        let ancestors = graph.get_ancestors("test#task3").unwrap();
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.contains(&"test#task2".to_string()));
        assert!(ancestors.contains(&"test#task1".to_string()));

        let ancestors = graph.get_ancestors("test#task2").unwrap();
        assert_eq!(ancestors.len(), 1);
        assert_eq!(ancestors[0], "test#task1");

        let ancestors = graph.get_ancestors("test#task1").unwrap();
        assert_eq!(ancestors.len(), 0);
    }

    #[test]
    fn test_get_descendants_diamond() {
        let mut graph = DependencyGraph::new();

        // Diamond: task1 → task2, task1 → task3, task2 → task4, task3 → task4
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));
        graph.add_node("test#task4".to_string(), create_test_node("Task 4"));

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
        graph.add_edge(
            "test#task2".to_string(),
            "test#task4".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#task3".to_string(),
            "test#task4".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let descendants = graph.get_descendants("test#task1").unwrap();
        assert_eq!(descendants.len(), 3);
        assert!(descendants.contains(&"test#task2".to_string()));
        assert!(descendants.contains(&"test#task3".to_string()));
        assert!(descendants.contains(&"test#task4".to_string()));
    }

    #[test]
    fn test_get_descendants_cycle_detection() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

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

        let result = graph.get_descendants("test#task1");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cycle detected"));
    }

    #[test]
    fn test_get_ancestors_cycle_detection() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

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

        let result = graph.get_ancestors("test#task1");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cycle detected"));
    }

    #[test]
    fn test_get_descendants_with_depth() {
        let mut graph = DependencyGraph::new();

        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

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

        // Depth 0: no dependencies
        let descendants = graph.get_descendants_with_depth("test#task1", 0).unwrap();
        assert_eq!(descendants.len(), 0);

        // Depth 1: only direct dependencies
        let descendants = graph.get_descendants_with_depth("test#task1", 1).unwrap();
        assert_eq!(descendants.len(), 1);
        assert_eq!(descendants[0], "test#task2");

        // Depth 2: full chain
        let descendants = graph.get_descendants_with_depth("test#task1", 2).unwrap();
        assert_eq!(descendants.len(), 2);
        assert!(descendants.contains(&"test#task2".to_string()));
        assert!(descendants.contains(&"test#task3".to_string()));

        // Depth 100: same as unlimited
        let descendants = graph.get_descendants_with_depth("test#task1", 100).unwrap();
        assert_eq!(descendants.len(), 2);
    }

    #[test]
    fn test_get_descendants_missing_node() {
        let graph = DependencyGraph::new();

        let descendants = graph.get_descendants("test#missing").unwrap();
        assert_eq!(descendants.len(), 0);
    }

    #[test]
    fn test_get_ancestors_missing_node() {
        let graph = DependencyGraph::new();

        let ancestors = graph.get_ancestors("test#missing").unwrap();
        assert_eq!(ancestors.len(), 0);
    }
}
