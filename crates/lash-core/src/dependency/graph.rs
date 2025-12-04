//! In-memory dependency graph representation
//!
//! This module provides an efficient in-memory graph structure for representing
//! task dependencies. The graph is built from the database and optimized for:
//!
//! - O(1) direct dependency lookups (forward and reverse)
//! - O(E+V) transitive closure computation
//! - Minimal memory overhead (stores only essential task metadata)
//! - Support for multiple edge types (hierarchy, explicit, directory)
//! - Efficient incremental updates for common operations
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
use std::fmt;

/// Errors that can occur during graph operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// Attempted to operate on a node that doesn't exist
    NodeNotFound(String),

    /// Attempted to operate on an edge that doesn't exist
    EdgeNotFound(String, String),

    /// Attempted to remove a node that still has dependencies
    NodeHasDependents {
        node_id: String,
        dependent_count: usize,
    },
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "Node not found: {id}"),
            Self::EdgeNotFound(from, to) => write!(f, "Edge not found: {from} -> {to}"),
            Self::NodeHasDependents {
                node_id,
                dependent_count,
            } => write!(
                f,
                "Cannot remove node '{node_id}' that has {dependent_count} dependent(s)"
            ),
        }
    }
}

impl std::error::Error for GraphError {}

/// Result type for graph operations
pub type GraphResult<T> = Result<T, GraphError>;

/// Tracks changes made to a graph for incremental updates
///
/// This struct records which nodes and edges have been modified, allowing
/// downstream systems (like status computation and cycle detection) to
/// determine what needs to be recomputed.
///
/// # Example
///
/// ```
/// use lash_core::dependency::{DependencyGraph, NodeData, GraphChanges};
/// use lash_types::TaskStatus;
///
/// let mut graph = DependencyGraph::new();
/// let mut changes = GraphChanges::new();
///
/// // Track a node addition
/// graph.add_node(
///     "test#task1".to_string(),
///     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0)
/// );
/// changes.add_node("test#task1");
///
/// // Check what changed
/// assert_eq!(changes.added_nodes().len(), 1);
/// assert!(changes.has_structural_changes());
/// ```
#[derive(Debug, Clone, Default)]
pub struct GraphChanges {
    /// Nodes that were added
    added_nodes: HashSet<String>,

    /// Nodes that were removed
    removed_nodes: HashSet<String>,

    /// Nodes whose metadata (except status) changed
    modified_nodes: HashSet<String>,

    /// Nodes whose status changed
    status_only_changes: HashSet<String>,

    /// Edges that were added (from, to)
    added_edges: HashSet<(String, String)>,

    /// Edges that were removed (from, to)
    removed_edges: HashSet<(String, String)>,

    /// Edges whose metadata changed (from, to)
    modified_edges: HashSet<(String, String)>,
}

impl GraphChanges {
    /// Create a new empty change tracker
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a node was added
    pub fn add_node(&mut self, task_id: &str) {
        self.added_nodes.insert(task_id.to_string());
    }

    /// Record that a node was removed
    pub fn remove_node(&mut self, task_id: &str) {
        self.removed_nodes.insert(task_id.to_string());
    }

    /// Record that a node's metadata was modified
    pub fn modify_node(&mut self, task_id: &str) {
        self.modified_nodes.insert(task_id.to_string());
    }

    /// Record that a node's status changed
    pub fn change_status(&mut self, task_id: &str) {
        self.status_only_changes.insert(task_id.to_string());
    }

    /// Record that an edge was added
    pub fn add_edge(&mut self, from_id: &str, to_id: &str) {
        self.added_edges
            .insert((from_id.to_string(), to_id.to_string()));
    }

    /// Record that an edge was removed
    pub fn remove_edge(&mut self, from_id: &str, to_id: &str) {
        self.removed_edges
            .insert((from_id.to_string(), to_id.to_string()));
    }

    /// Record that an edge's metadata was modified
    pub fn modify_edge(&mut self, from_id: &str, to_id: &str) {
        self.modified_edges
            .insert((from_id.to_string(), to_id.to_string()));
    }

    /// Get all added nodes
    #[must_use]
    pub fn added_nodes(&self) -> &HashSet<String> {
        &self.added_nodes
    }

    /// Get all removed nodes
    #[must_use]
    pub fn removed_nodes(&self) -> &HashSet<String> {
        &self.removed_nodes
    }

    /// Get all modified nodes (metadata changes, not including status)
    #[must_use]
    pub fn modified_nodes(&self) -> &HashSet<String> {
        &self.modified_nodes
    }

    /// Get all nodes with status-only changes
    #[must_use]
    pub fn status_only_changes(&self) -> &HashSet<String> {
        &self.status_only_changes
    }

    /// Get all added edges
    #[must_use]
    pub fn added_edges(&self) -> &HashSet<(String, String)> {
        &self.added_edges
    }

    /// Get all removed edges
    #[must_use]
    pub fn removed_edges(&self) -> &HashSet<(String, String)> {
        &self.removed_edges
    }

    /// Get all modified edges
    #[must_use]
    pub fn modified_edges(&self) -> &HashSet<(String, String)> {
        &self.modified_edges
    }

    /// Check if there were any structural changes (nodes/edges added/removed)
    ///
    /// Structural changes require cycle detection and full status recomputation.
    #[must_use]
    pub fn has_structural_changes(&self) -> bool {
        !self.added_nodes.is_empty()
            || !self.removed_nodes.is_empty()
            || !self.added_edges.is_empty()
            || !self.removed_edges.is_empty()
    }

    /// Check if only status changes occurred (no structural changes, no metadata changes)
    ///
    /// Status-only changes can be handled with faster incremental updates.
    #[must_use]
    pub fn is_status_only(&self) -> bool {
        !self.has_structural_changes()
            && self.modified_nodes.is_empty()
            && self.modified_edges.is_empty()
    }

    /// Check if the changes are empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added_nodes.is_empty()
            && self.removed_nodes.is_empty()
            && self.modified_nodes.is_empty()
            && self.status_only_changes.is_empty()
            && self.added_edges.is_empty()
            && self.removed_edges.is_empty()
            && self.modified_edges.is_empty()
    }

    /// Compute which nodes are affected by the changes
    ///
    /// Returns a set of node IDs that need to have their status or dependencies
    /// recomputed based on the changes. This includes:
    /// - All modified nodes
    /// - All nodes that depend on modified nodes (transitively)
    /// - All nodes involved in edge changes
    ///
    /// # Errors
    ///
    /// Returns error if graph traversal fails (e.g., cycle detection).
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData, EdgeData, GraphChanges};
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
    /// let mut changes = GraphChanges::new();
    /// changes.change_status("test#task2");
    ///
    /// // task1 depends on task2, so both need recomputation
    /// let affected = changes.compute_affected_nodes(&graph).unwrap();
    /// assert!(affected.contains("test#task1"));
    /// assert!(affected.contains("test#task2"));
    /// ```
    pub fn compute_affected_nodes(
        &self,
        graph: &DependencyGraph,
    ) -> Result<HashSet<String>, String> {
        let mut affected = HashSet::new();

        // All directly modified nodes are affected
        affected.extend(self.added_nodes.iter().cloned());
        affected.extend(self.modified_nodes.iter().cloned());
        affected.extend(self.status_only_changes.iter().cloned());

        // For removed nodes, we need to affect their former dependents
        // (but we can't query them since they're gone - caller must track)
        affected.extend(self.removed_nodes.iter().cloned());

        // Nodes involved in edge changes
        for (from_id, to_id) in &self.added_edges {
            affected.insert(from_id.clone());
            affected.insert(to_id.clone());
        }
        for (from_id, to_id) in &self.removed_edges {
            affected.insert(from_id.clone());
            affected.insert(to_id.clone());
        }
        for (from_id, to_id) in &self.modified_edges {
            affected.insert(from_id.clone());
            affected.insert(to_id.clone());
        }

        // For each affected node, also affect all its ancestors (nodes that depend on it)
        // This ensures status changes propagate upward through the dependency graph
        let mut to_process: Vec<String> = affected.iter().cloned().collect();
        while let Some(node_id) = to_process.pop() {
            if let Ok(ancestors) = graph.get_ancestors(&node_id) {
                for ancestor in ancestors {
                    if affected.insert(ancestor.clone()) {
                        to_process.push(ancestor);
                    }
                }
            }
        }

        Ok(affected)
    }

    /// Merge another `GraphChanges` into this one
    ///
    /// Useful for accumulating changes across multiple operations.
    pub fn merge(&mut self, other: &GraphChanges) {
        self.added_nodes.extend(other.added_nodes.iter().cloned());
        self.removed_nodes
            .extend(other.removed_nodes.iter().cloned());
        self.modified_nodes
            .extend(other.modified_nodes.iter().cloned());
        self.status_only_changes
            .extend(other.status_only_changes.iter().cloned());
        self.added_edges.extend(other.added_edges.iter().cloned());
        self.removed_edges
            .extend(other.removed_edges.iter().cloned());
        self.modified_edges
            .extend(other.modified_edges.iter().cloned());
    }

    /// Clear all recorded changes
    pub fn clear(&mut self) {
        self.added_nodes.clear();
        self.removed_nodes.clear();
        self.modified_nodes.clear();
        self.status_only_changes.clear();
        self.added_edges.clear();
        self.removed_edges.clear();
        self.modified_edges.clear();
    }
}

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

    /// Source file path (relative to project root)
    pub source_path: Option<String>,
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
            source_path: None,
        }
    }

    /// Set the source path for this node
    #[must_use]
    pub fn with_source_path(mut self, path: String) -> Self {
        self.source_path = Some(path);
        self
    }

    /// Check if this node is from an index file (lash.index.md or index.lash.md)
    #[must_use]
    pub fn is_from_index_file(&self) -> bool {
        self.source_path.as_ref().is_some_and(|p| {
            let filename = std::path::Path::new(p)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("");
            filename == "lash.index.md" || filename == "index.lash.md"
        })
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

    // ========================================================================
    // Mutation Operations (Phase 1)
    // ========================================================================

    /// Remove a node from the graph
    ///
    /// Removes the node and all its associated edges (both incoming and outgoing).
    /// By default, this operation fails if the node has dependents (other nodes
    /// depend on it), to prevent breaking the graph. Use `force = true` to remove
    /// the node anyway and cascade the removal to dependent edges.
    ///
    /// # Errors
    ///
    /// - Returns `GraphError::NodeNotFound` if the node doesn't exist
    /// - Returns `GraphError::NodeHasDependents` if the node has dependents and `force = false`
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
    ///
    /// // Remove the node
    /// graph.remove_node("test#task1", false).unwrap();
    /// assert_eq!(graph.node_count(), 0);
    /// ```
    pub fn remove_node(&mut self, task_id: &str, force: bool) -> GraphResult<()> {
        // Check if node exists
        if !self.nodes.contains_key(task_id) {
            return Err(GraphError::NodeNotFound(task_id.to_string()));
        }

        // Check if node has dependents (unless force is true)
        if !force {
            if let Some(dependents) = self.reverse.get(task_id) {
                if !dependents.is_empty() {
                    return Err(GraphError::NodeHasDependents {
                        node_id: task_id.to_string(),
                        dependent_count: dependents.len(),
                    });
                }
            }
        }

        // Remove all edges where this node is the source (outgoing edges)
        if let Some(dependencies) = self.adjacency.get(task_id) {
            let dep_ids: Vec<String> = dependencies.iter().map(|e| e.target_id.clone()).collect();
            for dep_id in dep_ids {
                self.remove_edge_internal(task_id, &dep_id);
            }
        }

        // Remove all edges where this node is the target (incoming edges)
        if let Some(dependents) = self.reverse.get(task_id) {
            let dependent_ids: Vec<String> =
                dependents.iter().map(|e| e.target_id.clone()).collect();
            for dependent_id in dependent_ids {
                self.remove_edge_internal(&dependent_id, task_id);
            }
        }

        // Remove the node itself
        self.nodes.remove(task_id);
        self.adjacency.remove(task_id);
        self.reverse.remove(task_id);

        Ok(())
    }

    /// Update node metadata
    ///
    /// Replaces the existing node data with new data. The node must exist.
    ///
    /// # Errors
    ///
    /// Returns `GraphError::NodeNotFound` if the node doesn't exist.
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
    ///
    /// // Update the node with new data
    /// let new_data = NodeData::new(
    ///     "Task 1 (Updated)".to_string(),
    ///     TaskStatus::Done,
    ///     "test".to_string(),
    ///     0
    /// );
    /// graph.update_node("test#task1", new_data).unwrap();
    ///
    /// let node = graph.get_node("test#task1").unwrap();
    /// assert_eq!(node.title, "Task 1 (Updated)");
    /// assert_eq!(node.status, TaskStatus::Done);
    /// ```
    pub fn update_node(&mut self, task_id: &str, node_data: NodeData) -> GraphResult<()> {
        if !self.nodes.contains_key(task_id) {
            return Err(GraphError::NodeNotFound(task_id.to_string()));
        }

        self.nodes.insert(task_id.to_string(), node_data);
        Ok(())
    }

    /// Update only the status of a node
    ///
    /// This is an optimized operation for the common case of status-only updates.
    /// It's more efficient than `update_node` when only the status changes.
    ///
    /// # Errors
    ///
    /// Returns `GraphError::NodeNotFound` if the node doesn't exist.
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
    ///
    /// // Update just the status
    /// graph.update_node_status("test#task1", TaskStatus::Done).unwrap();
    ///
    /// let node = graph.get_node("test#task1").unwrap();
    /// assert_eq!(node.status, TaskStatus::Done);
    /// assert_eq!(node.title, "Task 1"); // Other fields unchanged
    /// ```
    pub fn update_node_status(&mut self, task_id: &str, status: TaskStatus) -> GraphResult<()> {
        let node = self
            .nodes
            .get_mut(task_id)
            .ok_or_else(|| GraphError::NodeNotFound(task_id.to_string()))?;

        node.status = status;
        Ok(())
    }

    /// Remove an edge from the graph
    ///
    /// Removes the dependency relationship from `from_id` to `to_id`. This updates
    /// both the forward adjacency list and the reverse adjacency list to maintain
    /// graph invariants.
    ///
    /// # Errors
    ///
    /// Returns `GraphError::EdgeNotFound` if the edge doesn't exist.
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
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// assert_eq!(graph.edge_count(), 1);
    ///
    /// // Remove the edge
    /// graph.remove_edge("test#task1", "test#task2").unwrap();
    /// assert_eq!(graph.edge_count(), 0);
    /// ```
    pub fn remove_edge(&mut self, from_id: &str, to_id: &str) -> GraphResult<()> {
        let edge_id = (from_id.to_string(), to_id.to_string());

        // Check if edge exists
        if !self.edge_metadata.contains_key(&edge_id) {
            return Err(GraphError::EdgeNotFound(
                from_id.to_string(),
                to_id.to_string(),
            ));
        }

        self.remove_edge_internal(from_id, to_id);
        Ok(())
    }

    /// Internal helper to remove an edge without error checking
    ///
    /// This is used by `remove_node` to efficiently remove multiple edges.
    /// It assumes the edge exists and maintains graph invariants.
    fn remove_edge_internal(&mut self, from_id: &str, to_id: &str) {
        let edge_id = (from_id.to_string(), to_id.to_string());

        // Remove from forward adjacency list
        if let Some(deps) = self.adjacency.get_mut(from_id) {
            deps.retain(|e| e.target_id != to_id);
        }

        // Remove from reverse adjacency list
        if let Some(dependents) = self.reverse.get_mut(to_id) {
            dependents.retain(|e| e.target_id != from_id);
        }

        // Remove edge metadata
        self.edge_metadata.remove(&edge_id);
    }

    /// Update edge metadata
    ///
    /// Replaces the existing edge data with new data. The edge must exist.
    ///
    /// # Errors
    ///
    /// Returns `GraphError::EdgeNotFound` if the edge doesn't exist.
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
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// // Update edge metadata
    /// let new_data = EdgeData::new(
    ///     DependencyKind::Hierarchy,
    ///     Some("updated location".to_string())
    /// );
    /// graph.update_edge("test#task1", "test#task2", new_data).unwrap();
    ///
    /// let edge = graph.get_edge("test#task1", "test#task2").unwrap();
    /// assert_eq!(edge.kind, DependencyKind::Hierarchy);
    /// ```
    pub fn update_edge(
        &mut self,
        from_id: &str,
        to_id: &str,
        edge_data: EdgeData,
    ) -> GraphResult<()> {
        let edge_id = (from_id.to_string(), to_id.to_string());

        // Check if edge exists
        if !self.edge_metadata.contains_key(&edge_id) {
            return Err(GraphError::EdgeNotFound(
                from_id.to_string(),
                to_id.to_string(),
            ));
        }

        self.edge_metadata.insert(edge_id, edge_data);
        Ok(())
    }

    // ========================================================================
    // Batch Operations (Phase 2)
    // ========================================================================

    /// Add multiple nodes to the graph at once
    ///
    /// This is more efficient than calling `add_node` repeatedly, as it pre-allocates
    /// space for all nodes at once.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::dependency::{DependencyGraph, NodeData};
    /// use lash_types::TaskStatus;
    ///
    /// let mut graph = DependencyGraph::new();
    ///
    /// let nodes = vec![
    ///     ("test#task1".to_string(), NodeData::new(
    ///         "Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0
    ///     )),
    ///     ("test#task2".to_string(), NodeData::new(
    ///         "Task 2".to_string(), TaskStatus::Open, "test".to_string(), 0
    ///     )),
    /// ];
    ///
    /// graph.add_nodes(nodes);
    /// assert_eq!(graph.node_count(), 2);
    /// ```
    pub fn add_nodes(&mut self, nodes: Vec<(String, NodeData)>) {
        // Pre-allocate space to minimize reallocations
        self.nodes.reserve(nodes.len());
        self.adjacency.reserve(nodes.len());
        self.reverse.reserve(nodes.len());

        for (task_id, node_data) in nodes {
            self.add_node(task_id, node_data);
        }
    }

    /// Remove multiple nodes from the graph at once
    ///
    /// Removes all specified nodes and their associated edges. By default, this fails
    /// if any node has dependents. Use `force = true` to remove nodes regardless of
    /// dependents.
    ///
    /// # Errors
    ///
    /// - Returns the first error encountered (either `NodeNotFound` or `NodeHasDependents`)
    /// - If an error occurs, some nodes may have been removed already
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
    /// let ids = vec!["test#task1".to_string(), "test#task2".to_string()];
    /// graph.remove_nodes(&ids, false).unwrap();
    /// assert_eq!(graph.node_count(), 0);
    /// ```
    pub fn remove_nodes(&mut self, task_ids: &[String], force: bool) -> GraphResult<()> {
        for task_id in task_ids {
            self.remove_node(task_id, force)?;
        }
        Ok(())
    }

    /// Add multiple edges to the graph at once
    ///
    /// This is more efficient than calling `add_edge` repeatedly, as it pre-allocates
    /// space for all edges at once.
    ///
    /// # Panics
    ///
    /// Panics if any source or target node doesn't exist. Use `contains_node` to check first.
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
    /// let edges = vec![
    ///     ("test#task1".to_string(), "test#task2".to_string(),
    ///      EdgeData::new(DependencyKind::ExplicitId, None)),
    ///     ("test#task1".to_string(), "test#task3".to_string(),
    ///      EdgeData::new(DependencyKind::ExplicitId, None)),
    /// ];
    ///
    /// graph.add_edges(edges);
    /// assert_eq!(graph.edge_count(), 2);
    /// ```
    pub fn add_edges(&mut self, edges: Vec<(String, String, EdgeData)>) {
        // Pre-allocate space to minimize reallocations
        self.edge_metadata.reserve(edges.len());

        for (from_id, to_id, edge_data) in edges {
            self.add_edge(from_id, to_id, edge_data);
        }
    }

    /// Remove multiple edges from the graph at once
    ///
    /// Removes all specified dependency relationships.
    ///
    /// # Errors
    ///
    /// - Returns the first `EdgeNotFound` error encountered
    /// - If an error occurs, some edges may have been removed already
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
    /// graph.add_edge(
    ///     "test#task1".to_string(),
    ///     "test#task2".to_string(),
    ///     EdgeData::new(DependencyKind::ExplicitId, None)
    /// );
    ///
    /// let edges = vec![("test#task1", "test#task2")];
    /// graph.remove_edges(&edges).unwrap();
    /// assert_eq!(graph.edge_count(), 0);
    /// ```
    pub fn remove_edges(&mut self, edges: &[(&str, &str)]) -> GraphResult<()> {
        for (from_id, to_id) in edges {
            self.remove_edge(from_id, to_id)?;
        }
        Ok(())
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

    // ========================================================================
    // Phase 1: Mutation Operations Tests
    // ========================================================================

    #[test]
    fn test_remove_node_simple() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));

        assert_eq!(graph.node_count(), 1);

        let result = graph.remove_node("test#task1", false);
        assert!(result.is_ok());
        assert_eq!(graph.node_count(), 0);
        assert!(!graph.contains_node("test#task1"));
    }

    #[test]
    fn test_remove_node_not_found() {
        let mut graph = DependencyGraph::new();

        let result = graph.remove_node("test#missing", false);
        assert!(matches!(result, Err(GraphError::NodeNotFound(_))));
    }

    #[test]
    fn test_remove_node_with_dependents_fails() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

        // task1 depends on task2
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        // Try to remove task2 (has dependents)
        let result = graph.remove_node("test#task2", false);
        assert!(matches!(result, Err(GraphError::NodeHasDependents { .. })));

        // Task should still exist
        assert!(graph.contains_node("test#task2"));
    }

    #[test]
    fn test_remove_node_with_dependents_force() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

        // task1 depends on task2
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        assert_eq!(graph.edge_count(), 1);

        // Remove task2 with force=true
        let result = graph.remove_node("test#task2", true);
        assert!(result.is_ok());

        // Task should be gone
        assert!(!graph.contains_node("test#task2"));

        // Edge should also be removed
        assert_eq!(graph.edge_count(), 0);
        assert_eq!(graph.get_dependency_ids("test#task1").len(), 0);
    }

    #[test]
    fn test_remove_node_removes_all_edges() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

        // task1 depends on task2 and task3
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

        // Remove task1
        let result = graph.remove_node("test#task1", false);
        assert!(result.is_ok());

        // All edges should be removed
        assert_eq!(graph.edge_count(), 0);

        // task2 and task3 should have no dependents
        assert_eq!(graph.get_dependent_ids("test#task2").len(), 0);
        assert_eq!(graph.get_dependent_ids("test#task3").len(), 0);
    }

    #[test]
    fn test_update_node() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Original Title"));

        let new_data = NodeData::new(
            "Updated Title".to_string(),
            TaskStatus::Done,
            "test".to_string(),
            0,
        );

        let result = graph.update_node("test#task1", new_data);
        assert!(result.is_ok());

        let node = graph.get_node("test#task1").unwrap();
        assert_eq!(node.title, "Updated Title");
        assert_eq!(node.status, TaskStatus::Done);
    }

    #[test]
    fn test_update_node_not_found() {
        let mut graph = DependencyGraph::new();

        let new_data = create_test_node("Test");
        let result = graph.update_node("test#missing", new_data);
        assert!(matches!(result, Err(GraphError::NodeNotFound(_))));
    }

    #[test]
    fn test_update_node_status() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));

        // Initial status is Open
        assert_eq!(
            graph.get_node("test#task1").unwrap().status,
            TaskStatus::Open
        );

        // Update to Done
        let result = graph.update_node_status("test#task1", TaskStatus::Done);
        assert!(result.is_ok());

        let node = graph.get_node("test#task1").unwrap();
        assert_eq!(node.status, TaskStatus::Done);
        assert_eq!(node.title, "Task 1"); // Other fields unchanged
    }

    #[test]
    fn test_update_node_status_not_found() {
        let mut graph = DependencyGraph::new();

        let result = graph.update_node_status("test#missing", TaskStatus::Done);
        assert!(matches!(result, Err(GraphError::NodeNotFound(_))));
    }

    #[test]
    fn test_remove_edge() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        assert_eq!(graph.edge_count(), 1);

        let result = graph.remove_edge("test#task1", "test#task2");
        assert!(result.is_ok());

        assert_eq!(graph.edge_count(), 0);
        assert_eq!(graph.get_dependency_ids("test#task1").len(), 0);
        assert_eq!(graph.get_dependent_ids("test#task2").len(), 0);
    }

    #[test]
    fn test_remove_edge_not_found() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

        let result = graph.remove_edge("test#task1", "test#task2");
        assert!(matches!(result, Err(GraphError::EdgeNotFound(_, _))));
    }

    #[test]
    fn test_remove_edge_maintains_invariants() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

        // task1 depends on task2 and task3
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

        // Remove one edge
        graph.remove_edge("test#task1", "test#task2").unwrap();

        // task1 should still depend on task3
        let deps = graph.get_dependency_ids("test#task1");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "test#task3");

        // task2 should have no dependents
        assert_eq!(graph.get_dependent_ids("test#task2").len(), 0);

        // task3 should still have task1 as dependent
        let dependents = graph.get_dependent_ids("test#task3");
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0], "test#task1");
    }

    #[test]
    fn test_update_edge() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        // Update edge metadata
        let new_data = EdgeData::new(DependencyKind::Hierarchy, Some("updated".to_string()));
        let result = graph.update_edge("test#task1", "test#task2", new_data);
        assert!(result.is_ok());

        let edge = graph.get_edge("test#task1", "test#task2").unwrap();
        assert_eq!(edge.kind, DependencyKind::Hierarchy);
        assert_eq!(edge.source_location.as_deref(), Some("updated"));
    }

    #[test]
    fn test_update_edge_not_found() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

        let new_data = EdgeData::new(DependencyKind::Hierarchy, None);
        let result = graph.update_edge("test#task1", "test#task2", new_data);
        assert!(matches!(result, Err(GraphError::EdgeNotFound(_, _))));
    }

    // ========================================================================
    // Phase 2: Batch Operations Tests
    // ========================================================================

    #[test]
    fn test_add_nodes_batch() {
        let mut graph = DependencyGraph::new();

        let nodes = vec![
            ("test#task1".to_string(), create_test_node("Task 1")),
            ("test#task2".to_string(), create_test_node("Task 2")),
            ("test#task3".to_string(), create_test_node("Task 3")),
        ];

        graph.add_nodes(nodes);

        assert_eq!(graph.node_count(), 3);
        assert!(graph.contains_node("test#task1"));
        assert!(graph.contains_node("test#task2"));
        assert!(graph.contains_node("test#task3"));
    }

    #[test]
    fn test_remove_nodes_batch() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

        let ids = vec![
            "test#task1".to_string(),
            "test#task2".to_string(),
            "test#task3".to_string(),
        ];

        let result = graph.remove_nodes(&ids, false);
        assert!(result.is_ok());
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_remove_nodes_batch_with_error() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

        // task1 depends on task2
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        // Try to remove both (task2 has dependents)
        let ids = vec!["test#task1".to_string(), "test#task2".to_string()];
        let result = graph.remove_nodes(&ids, false);

        // Should succeed for task1, fail for task2
        // (Note: task1 removed first, so task2 no longer has dependents)
        assert!(result.is_ok());
    }

    #[test]
    fn test_remove_nodes_batch_not_found() {
        let mut graph = DependencyGraph::new();

        let ids = vec!["test#missing".to_string()];
        let result = graph.remove_nodes(&ids, false);
        assert!(matches!(result, Err(GraphError::NodeNotFound(_))));
    }

    #[test]
    fn test_add_edges_batch() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_node("test#task3".to_string(), create_test_node("Task 3"));

        let edges = vec![
            (
                "test#task1".to_string(),
                "test#task2".to_string(),
                EdgeData::new(DependencyKind::ExplicitId, None),
            ),
            (
                "test#task1".to_string(),
                "test#task3".to_string(),
                EdgeData::new(DependencyKind::ExplicitId, None),
            ),
        ];

        graph.add_edges(edges);

        assert_eq!(graph.edge_count(), 2);
        assert_eq!(graph.get_dependency_ids("test#task1").len(), 2);
    }

    #[test]
    fn test_remove_edges_batch() {
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

        assert_eq!(graph.edge_count(), 2);

        let edges = vec![("test#task1", "test#task2"), ("test#task1", "test#task3")];
        let result = graph.remove_edges(&edges);
        assert!(result.is_ok());

        assert_eq!(graph.edge_count(), 0);
        assert_eq!(graph.get_dependency_ids("test#task1").len(), 0);
    }

    #[test]
    fn test_remove_edges_batch_not_found() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));

        let edges = vec![("test#task1", "test#task2")];
        let result = graph.remove_edges(&edges);
        assert!(matches!(result, Err(GraphError::EdgeNotFound(_, _))));
    }

    // ========================================================================
    // Phase 3: GraphChanges Tests
    // ========================================================================

    #[test]
    fn test_graph_changes_new() {
        let changes = GraphChanges::new();
        assert!(changes.is_empty());
        assert!(!changes.has_structural_changes());
        assert!(changes.is_status_only());
    }

    #[test]
    fn test_graph_changes_add_node() {
        let mut changes = GraphChanges::new();
        changes.add_node("test#task1");

        assert!(!changes.is_empty());
        assert!(changes.has_structural_changes());
        assert!(!changes.is_status_only());
        assert_eq!(changes.added_nodes().len(), 1);
    }

    #[test]
    fn test_graph_changes_remove_node() {
        let mut changes = GraphChanges::new();
        changes.remove_node("test#task1");

        assert!(!changes.is_empty());
        assert!(changes.has_structural_changes());
        assert_eq!(changes.removed_nodes().len(), 1);
    }

    #[test]
    fn test_graph_changes_modify_node() {
        let mut changes = GraphChanges::new();
        changes.modify_node("test#task1");

        assert!(!changes.is_empty());
        assert!(!changes.has_structural_changes());
        assert!(!changes.is_status_only());
        assert_eq!(changes.modified_nodes().len(), 1);
    }

    #[test]
    fn test_graph_changes_status_only() {
        let mut changes = GraphChanges::new();
        changes.change_status("test#task1");

        assert!(!changes.is_empty());
        assert!(!changes.has_structural_changes());
        assert!(changes.is_status_only());
        assert_eq!(changes.status_only_changes().len(), 1);
    }

    #[test]
    fn test_graph_changes_add_edge() {
        let mut changes = GraphChanges::new();
        changes.add_edge("test#task1", "test#task2");

        assert!(!changes.is_empty());
        assert!(changes.has_structural_changes());
        assert_eq!(changes.added_edges().len(), 1);
    }

    #[test]
    fn test_graph_changes_remove_edge() {
        let mut changes = GraphChanges::new();
        changes.remove_edge("test#task1", "test#task2");

        assert!(!changes.is_empty());
        assert!(changes.has_structural_changes());
        assert_eq!(changes.removed_edges().len(), 1);
    }

    #[test]
    fn test_graph_changes_modify_edge() {
        let mut changes = GraphChanges::new();
        changes.modify_edge("test#task1", "test#task2");

        assert!(!changes.is_empty());
        assert!(!changes.has_structural_changes());
        assert!(!changes.is_status_only());
        assert_eq!(changes.modified_edges().len(), 1);
    }

    #[test]
    fn test_graph_changes_compute_affected_nodes_status_change() {
        let mut graph = DependencyGraph::new();
        graph.add_node("test#task1".to_string(), create_test_node("Task 1"));
        graph.add_node("test#task2".to_string(), create_test_node("Task 2"));
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let mut changes = GraphChanges::new();
        changes.change_status("test#task2");

        let affected = changes.compute_affected_nodes(&graph).unwrap();

        // Both task1 (dependent) and task2 (changed) should be affected
        assert_eq!(affected.len(), 2);
        assert!(affected.contains("test#task1"));
        assert!(affected.contains("test#task2"));
    }

    #[test]
    fn test_graph_changes_compute_affected_nodes_transitive() {
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

        let mut changes = GraphChanges::new();
        changes.change_status("test#task3");

        let affected = changes.compute_affected_nodes(&graph).unwrap();

        // All three tasks should be affected (transitive propagation)
        assert_eq!(affected.len(), 3);
        assert!(affected.contains("test#task1"));
        assert!(affected.contains("test#task2"));
        assert!(affected.contains("test#task3"));
    }

    #[test]
    fn test_graph_changes_merge() {
        let mut changes1 = GraphChanges::new();
        changes1.add_node("test#task1");
        changes1.change_status("test#task2");

        let mut changes2 = GraphChanges::new();
        changes2.add_node("test#task3");
        changes2.remove_edge("test#task1", "test#task2");

        changes1.merge(&changes2);

        assert_eq!(changes1.added_nodes().len(), 2);
        assert_eq!(changes1.status_only_changes().len(), 1);
        assert_eq!(changes1.removed_edges().len(), 1);
    }

    #[test]
    fn test_graph_changes_clear() {
        let mut changes = GraphChanges::new();
        changes.add_node("test#task1");
        changes.change_status("test#task2");
        changes.add_edge("test#task1", "test#task2");

        assert!(!changes.is_empty());

        changes.clear();

        assert!(changes.is_empty());
        assert_eq!(changes.added_nodes().len(), 0);
        assert_eq!(changes.status_only_changes().len(), 0);
        assert_eq!(changes.added_edges().len(), 0);
    }
}
