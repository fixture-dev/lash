# Dependency Graph Architecture Design

**Document Version:** 1.0
**Date:** 2025-11-20
**Status:** Design Specification

---

## 1. Executive Summary

This document specifies the architecture for Lash's in-memory dependency graph system. The graph is the core data structure for modeling task dependencies, computing completion status, detecting cycles, and supporting efficient queries. This design integrates with the existing DB layer (`lash-db`) and type system (`lash-types`) while providing a high-performance in-memory representation optimized for 1000+ task graphs.

**Key Design Principles:**
- **Memory efficiency:** Optimized for 1000+ tasks with compact representations
- **Query performance:** O(1) direct lookups, O(E+V) transitive queries
- **Integration:** Seamless with existing DB layer (which already has cycle detection and transitive closure)
- **Safety:** Type-safe Rust implementation with clear invariants
- **Extensibility:** Supports incremental updates and graph evolution

---

## 2. Context & Existing Architecture

### 2.1 Current State

**lash-types/src/dependency.rs:**
- Defines `DependencyKind` (Hierarchy, ExplicitId, ExplicitPath, Directory)
- Provides `DependencyRef` (unresolved) and `Dependency` (resolved)
- Includes parsing and validation utilities

**lash-db/src/repository/dependencies.rs:**
- Stores dependencies in SQLite `dependencies` table
- Maintains `dependency_closure` table for transitive queries
- Provides:
  - `would_create_cycle()` - single-edge cycle check using recursive CTE
  - `rebuild_closure()` - transitive closure computation
  - `get_all_dependencies()` / `get_all_dependents()` - transitive queries

**lash-types/src/task.rs:**
- `TaskTree` maintains flat task list with HashMap index
- Provides `get_children()` and `get_descendants()` for within-file hierarchy

### 2.2 Design Goals for In-Memory Graph

The in-memory graph complements the DB layer by providing:

1. **Fast graph operations** without SQL overhead
2. **Unified view** of all dependency types (hierarchy, explicit, directory)
3. **Rich graph algorithms** (comprehensive cycle detection, topological sort, blocker analysis)
4. **Incremental updates** for status changes and structural modifications
5. **Export capabilities** for visualization (DOT, JSON)

The DB layer remains the source of truth for persistence, while the in-memory graph is rebuilt on demand for analysis and query operations.

---

## 3. Core Data Structures

### 3.1 Graph Representation Strategy

**Selected Approach:** **Adjacency list with bidirectional edges and unified node index**

**Rationale:**
- Most operations need forward edges (dependencies) and reverse edges (dependents)
- Task lookups by full_id are frequent (O(1) with HashMap)
- Memory overhead is acceptable for 1000s of tasks
- Supports efficient DFS/BFS traversal

**Alternative Considered:** Adjacency matrix
- **Rejected:** O(V²) space unsuitable for sparse graphs with 1000+ tasks

### 3.2 Primary Types

```rust
// File: lash-core/src/graph/types.rs

use std::collections::{HashMap, HashSet};
use lash_types::{DependencyKind, TaskStatus};

/// Node ID in the dependency graph
///
/// Represented as a full task ID: "{file_id}#{task_id}"
/// Examples: "core.api#setup", "tasks#task-1"
pub type NodeId = String;

/// Graph node representing a task
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// Full task ID (file_id#task_id)
    pub full_id: NodeId,

    /// Database row ID (for linking back to DB)
    pub db_id: i64,

    /// File ID this task belongs to
    pub file_id: String,

    /// Task ID within the file
    pub task_id: String,

    /// Task title (for display)
    pub title: String,

    /// Current status
    pub status: TaskStatus,

    /// Task depth (for hierarchy)
    pub depth: u8,

    /// Parent task ID (if any) - for within-file hierarchy
    pub parent_id: Option<NodeId>,
}

/// Edge in the dependency graph
///
/// Represents "from depends ON to" (from → to)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphEdge {
    /// Source task (dependent)
    pub from: NodeId,

    /// Target task (dependency)
    pub to: NodeId,

    /// Type of dependency
    pub kind: DependencyKind,

    /// Original reference string (for error reporting)
    pub raw_ref: Option<String>,
}

impl GraphEdge {
    /// Create a new graph edge
    pub fn new(from: NodeId, to: NodeId, kind: DependencyKind) -> Self {
        Self {
            from,
            to,
            kind,
            raw_ref: None,
        }
    }

    /// Create edge with raw reference string
    pub fn with_ref(mut self, raw_ref: String) -> Self {
        self.raw_ref = Some(raw_ref);
        self
    }
}

/// Main dependency graph structure
pub struct DependencyGraph {
    /// All nodes indexed by full_id
    nodes: HashMap<NodeId, GraphNode>,

    /// Forward edges: task → dependencies (what this task depends ON)
    /// Adjacency list representation
    forward_edges: HashMap<NodeId, Vec<GraphEdge>>,

    /// Reverse edges: task → dependents (tasks that depend on this one)
    /// Enables efficient reverse lookups
    reverse_edges: HashMap<NodeId, Vec<GraphEdge>>,

    /// Index by file_id for file-level queries
    file_index: HashMap<String, Vec<NodeId>>,

    /// Index by DB ID for fast DB-to-graph lookups
    db_id_index: HashMap<i64, NodeId>,

    /// Cache for computed status (invalidated on updates)
    status_cache: HashMap<NodeId, EffectiveStatus>,
}

/// Effective status computed from dependencies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveStatus {
    /// All dependencies complete
    Complete,

    /// Own status is open, but ready to work (no blockers)
    Ready,

    /// Blocked by one or more incomplete dependencies
    Blocked,

    /// Task is waived (treated as complete)
    Waived,
}
```

### 3.3 Memory Characteristics

**Size Estimates (for 1000 tasks):**

- `GraphNode`: ~100 bytes each = 100 KB
- `HashMap<NodeId, GraphNode>`: ~130 KB (with 1.3x load factor)
- Forward/reverse edges (avg 3 edges/task): ~72 KB each
- Indexes: ~50 KB
- **Total:** ~400-500 KB for 1000 tasks

**Scaling:** Linear O(V + E) memory complexity. For 10,000 tasks: ~4-5 MB.

---

## 4. Core API Design

### 4.1 Graph Construction

```rust
// File: lash-core/src/graph/builder.rs

use rusqlite::Connection;
use crate::error::Result;

pub struct GraphBuilder<'conn> {
    conn: &'conn Connection,
}

impl<'conn> GraphBuilder<'conn> {
    /// Create a new graph builder from a database connection
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Build the complete dependency graph from the database
    ///
    /// Loads all tasks and dependencies, constructs adjacency lists,
    /// and builds all indexes.
    ///
    /// # Complexity
    /// O(V + E) where V = tasks, E = dependencies
    ///
    /// # Errors
    /// Returns error if database queries fail
    pub fn build(&self) -> Result<DependencyGraph> {
        let mut graph = DependencyGraph::new();

        // 1. Load all tasks and create nodes
        self.load_nodes(&mut graph)?;

        // 2. Load all dependencies and create edges
        self.load_edges(&mut graph)?;

        // 3. Build indexes
        graph.build_indexes();

        // 4. Validate graph invariants
        graph.validate()?;

        Ok(graph)
    }

    /// Load all task nodes from the database
    fn load_nodes(&self, graph: &mut DependencyGraph) -> Result<()> {
        // Query: SELECT id, file_id, task_id, full_id, title, status, depth, parent_id
        // FROM tasks
        // ...implementation...
    }

    /// Load all dependency edges from the database
    fn load_edges(&self, graph: &mut DependencyGraph) -> Result<()> {
        // Query: SELECT from_task_id, to_task_id, kind, raw_ref
        // FROM dependencies
        // Join with tasks to get full_ids
        // ...implementation...
    }
}

impl DependencyGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            forward_edges: HashMap::new(),
            reverse_edges: HashMap::new(),
            file_index: HashMap::new(),
            db_id_index: HashMap::new(),
            status_cache: HashMap::new(),
        }
    }

    /// Add a node to the graph
    ///
    /// # Errors
    /// Returns error if node with same full_id already exists
    pub fn add_node(&mut self, node: GraphNode) -> Result<()> {
        if self.nodes.contains_key(&node.full_id) {
            return Err(/* duplicate node error */);
        }

        let full_id = node.full_id.clone();
        let db_id = node.db_id;
        let file_id = node.file_id.clone();

        self.nodes.insert(full_id.clone(), node);
        self.db_id_index.insert(db_id, full_id.clone());
        self.file_index.entry(file_id)
            .or_default()
            .push(full_id);

        Ok(())
    }

    /// Add an edge to the graph
    ///
    /// # Errors
    /// Returns error if either endpoint doesn't exist
    pub fn add_edge(&mut self, edge: GraphEdge) -> Result<()> {
        // Validate endpoints exist
        if !self.nodes.contains_key(&edge.from) {
            return Err(/* from node not found */);
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(/* to node not found */);
        }

        // Add to forward edges
        self.forward_edges
            .entry(edge.from.clone())
            .or_default()
            .push(edge.clone());

        // Add to reverse edges
        self.reverse_edges
            .entry(edge.to.clone())
            .or_default()
            .push(edge);

        // Invalidate status cache for affected nodes
        self.invalidate_status_cache(&edge.from);

        Ok(())
    }

    /// Build secondary indexes after bulk node/edge insertion
    fn build_indexes(&mut self) {
        // Already built incrementally in add_node/add_edge
        // This is a no-op, but kept for API clarity
    }

    /// Validate graph invariants
    ///
    /// Checks:
    /// - All edge endpoints exist as nodes
    /// - Parent references are valid
    /// - No duplicate edges
    fn validate(&self) -> Result<()> {
        // ...validation logic...
    }
}
```

### 4.2 Query Operations

```rust
// File: lash-core/src/graph/queries.rs

impl DependencyGraph {
    /// Get a node by full_id
    ///
    /// # Complexity
    /// O(1) - HashMap lookup
    pub fn get_node(&self, full_id: &str) -> Option<&GraphNode> {
        self.nodes.get(full_id)
    }

    /// Get a node by database ID
    ///
    /// # Complexity
    /// O(1) - HashMap lookup
    pub fn get_node_by_db_id(&self, db_id: i64) -> Option<&GraphNode> {
        self.db_id_index.get(&db_id)
            .and_then(|id| self.nodes.get(id))
    }

    /// Get all nodes in a file
    ///
    /// # Complexity
    /// O(1) to get list, O(k) to clone where k = tasks in file
    pub fn get_nodes_in_file(&self, file_id: &str) -> Vec<&GraphNode> {
        self.file_index.get(file_id)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get direct dependencies (outgoing edges)
    ///
    /// Returns tasks that the given task depends ON.
    ///
    /// # Complexity
    /// O(1) for lookup, O(k) for result where k = number of dependencies
    pub fn get_dependencies(&self, full_id: &str) -> Vec<&GraphEdge> {
        self.forward_edges.get(full_id)
            .map(|edges| edges.iter().collect())
            .unwrap_or_default()
    }

    /// Get direct dependents (incoming edges)
    ///
    /// Returns tasks that depend on the given task.
    ///
    /// # Complexity
    /// O(1) for lookup, O(k) for result where k = number of dependents
    pub fn get_dependents(&self, full_id: &str) -> Vec<&GraphEdge> {
        self.reverse_edges.get(full_id)
            .map(|edges| edges.iter().collect())
            .unwrap_or_default()
    }

    /// Get all descendants (transitive dependencies) using DFS
    ///
    /// Returns all tasks that the given task depends on, directly or indirectly.
    /// Uses depth-first search to avoid revisiting nodes.
    ///
    /// # Complexity
    /// O(E + V) - visits each reachable node and edge once
    ///
    /// # Returns
    /// Vec of (NodeId, depth) tuples sorted by depth (closest first)
    pub fn get_descendants(&self, full_id: &str) -> Vec<(NodeId, usize)> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();

        self.dfs_descendants(full_id, 0, &mut visited, &mut result);

        result.sort_by_key(|(_, depth)| *depth);
        result
    }

    /// DFS helper for descendants
    fn dfs_descendants(
        &self,
        node_id: &str,
        depth: usize,
        visited: &mut HashSet<NodeId>,
        result: &mut Vec<(NodeId, usize)>,
    ) {
        if !visited.insert(node_id.to_string()) {
            return; // Already visited
        }

        if let Some(edges) = self.forward_edges.get(node_id) {
            for edge in edges {
                result.push((edge.to.clone(), depth + 1));
                self.dfs_descendants(&edge.to, depth + 1, visited, result);
            }
        }
    }

    /// Get all ancestors (transitive dependents) using reverse DFS
    ///
    /// Returns all tasks that depend on the given task, directly or indirectly.
    ///
    /// # Complexity
    /// O(E + V)
    pub fn get_ancestors(&self, full_id: &str) -> Vec<(NodeId, usize)> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();

        self.dfs_ancestors(full_id, 0, &mut visited, &mut result);

        result.sort_by_key(|(_, depth)| *depth);
        result
    }

    /// DFS helper for ancestors
    fn dfs_ancestors(
        &self,
        node_id: &str,
        depth: usize,
        visited: &mut HashSet<NodeId>,
        result: &mut Vec<(NodeId, usize)>,
    ) {
        if !visited.insert(node_id.to_string()) {
            return;
        }

        if let Some(edges) = self.reverse_edges.get(node_id) {
            for edge in edges {
                result.push((edge.from.clone(), depth + 1));
                self.dfs_ancestors(&edge.from, depth + 1, visited, result);
            }
        }
    }

    /// Get children (immediate subtasks) within the same file
    ///
    /// Uses the parent_id relationship, not dependency edges.
    ///
    /// # Complexity
    /// O(n) where n = tasks in the same file
    pub fn get_children(&self, full_id: &str) -> Vec<&GraphNode> {
        if let Some(node) = self.nodes.get(full_id) {
            self.file_index.get(&node.file_id)
                .map(|ids| {
                    ids.iter()
                        .filter_map(|id| self.nodes.get(id))
                        .filter(|n| n.parent_id.as_deref() == Some(full_id))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Count total nodes in graph
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Count total edges in graph
    pub fn edge_count(&self) -> usize {
        self.forward_edges.values().map(Vec::len).sum()
    }
}
```

---

## 5. Cycle Detection

### 5.1 Algorithm Selection

**Chosen Algorithm:** DFS with three-color marking (White/Gray/Black)

**Rationale:**
- Standard, well-tested algorithm for directed graphs
- O(V + E) time complexity
- Natural path tracking for cycle reporting
- Detects ALL cycles, not just one

**Alternative Considered:** Tarjan's strongly connected components
- **Not chosen:** More complex, and we need actual cycle paths for error reporting

### 5.2 Implementation

```rust
// File: lash-core/src/graph/cycles.rs

use std::collections::{HashMap, HashSet};
use crate::error::{LashError, Result};

/// Node color for DFS cycle detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    /// Not yet visited
    White,
    /// Currently being explored (on DFS stack)
    Gray,
    /// Fully explored
    Black,
}

/// A detected cycle in the dependency graph
#[derive(Debug, Clone)]
pub struct Cycle {
    /// Ordered list of node IDs forming the cycle
    /// Example: ["A", "B", "C", "A"] for A→B→C→A
    pub path: Vec<NodeId>,

    /// Edge kinds in the cycle (parallel to path)
    pub edge_kinds: Vec<DependencyKind>,
}

impl Cycle {
    /// Check if cycle contains any explicit dependencies
    pub fn has_explicit_deps(&self) -> bool {
        self.edge_kinds.iter().any(|k| {
            matches!(k, DependencyKind::ExplicitId | DependencyKind::ExplicitPath)
        })
    }

    /// Get a human-readable description of the cycle
    pub fn format(&self, graph: &DependencyGraph) -> String {
        let mut parts = Vec::new();
        for (i, node_id) in self.path.iter().enumerate() {
            if let Some(node) = graph.get_node(node_id) {
                parts.push(format!("{}:{}", node.file_id, node.task_id));
            } else {
                parts.push(node_id.clone());
            }

            if i < self.edge_kinds.len() {
                let arrow = match self.edge_kinds[i] {
                    DependencyKind::Hierarchy => " ⊃ ",
                    DependencyKind::ExplicitId | DependencyKind::ExplicitPath => " → ",
                    DependencyKind::Directory => " ⊇ ",
                };
                parts.push(arrow.to_string());
            }
        }
        parts.join("")
    }
}

/// Cycle detector using DFS with color marking
pub struct CycleDetector<'g> {
    graph: &'g DependencyGraph,
    colors: HashMap<NodeId, Color>,
    stack: Vec<NodeId>,
    cycles: Vec<Cycle>,
}

impl<'g> CycleDetector<'g> {
    /// Create a new cycle detector for the given graph
    pub fn new(graph: &'g DependencyGraph) -> Self {
        Self {
            graph,
            colors: HashMap::new(),
            stack: Vec::new(),
            cycles: Vec::new(),
        }
    }

    /// Detect all cycles in the graph
    ///
    /// Uses DFS with three-color marking:
    /// - White: not visited
    /// - Gray: on current DFS path (if we see a gray node, we found a cycle)
    /// - Black: fully explored
    ///
    /// # Complexity
    /// O(V + E)
    ///
    /// # Returns
    /// Vec of all detected cycles, or error if detection fails
    pub fn detect_cycles(mut self) -> Result<Vec<Cycle>> {
        // Initialize all nodes as White
        for node_id in self.graph.nodes.keys() {
            self.colors.insert(node_id.clone(), Color::White);
        }

        // Run DFS from each unvisited node
        for node_id in self.graph.nodes.keys().cloned().collect::<Vec<_>>() {
            if self.colors[&node_id] == Color::White {
                self.dfs_visit(&node_id)?;
            }
        }

        Ok(self.cycles)
    }

    /// DFS visit for cycle detection
    fn dfs_visit(&mut self, node_id: &str) -> Result<()> {
        // Mark as Gray (on current path)
        self.colors.insert(node_id.to_string(), Color::Gray);
        self.stack.push(node_id.to_string());

        // Explore dependencies
        if let Some(edges) = self.graph.forward_edges.get(node_id) {
            for edge in edges {
                let color = self.colors[&edge.to];

                match color {
                    Color::White => {
                        // Not yet visited, recurse
                        self.dfs_visit(&edge.to)?;
                    }
                    Color::Gray => {
                        // Back edge detected - we found a cycle!
                        self.extract_cycle(&edge.to, edge.kind);
                    }
                    Color::Black => {
                        // Already fully explored, skip
                    }
                }
            }
        }

        // Mark as Black (fully explored)
        self.stack.pop();
        self.colors.insert(node_id.to_string(), Color::Black);

        Ok(())
    }

    /// Extract cycle from stack when back edge is found
    fn extract_cycle(&mut self, back_to: &str, last_edge_kind: DependencyKind) {
        // Find where the cycle starts in the stack
        if let Some(start_idx) = self.stack.iter().position(|id| id == back_to) {
            let mut path: Vec<NodeId> = self.stack[start_idx..].to_vec();
            path.push(back_to.to_string()); // Close the cycle

            // Collect edge kinds for the cycle
            let mut edge_kinds = Vec::new();
            for i in 0..path.len() - 1 {
                // Look up edge kind between path[i] and path[i+1]
                if let Some(edges) = self.graph.forward_edges.get(&path[i]) {
                    if let Some(edge) = edges.iter().find(|e| e.to == path[i + 1]) {
                        edge_kinds.push(edge.kind);
                    }
                }
            }
            edge_kinds.push(last_edge_kind); // Last edge back to start

            self.cycles.push(Cycle { path, edge_kinds });
        }
    }
}

impl DependencyGraph {
    /// Detect all cycles in the graph
    ///
    /// # Complexity
    /// O(V + E)
    pub fn detect_cycles(&self) -> Result<Vec<Cycle>> {
        let detector = CycleDetector::new(self);
        detector.detect_cycles()
    }

    /// Check if adding an edge would create a cycle
    ///
    /// Does NOT mutate the graph. Tests hypothetical edge addition.
    ///
    /// # Complexity
    /// O(V + E) in worst case (full DFS from target to source)
    pub fn would_create_cycle(&self, from: &str, to: &str) -> Result<bool> {
        // Check if there's already a path from `to` back to `from`
        // If yes, adding from→to would create a cycle

        let descendants = self.get_descendants(to);
        Ok(descendants.iter().any(|(id, _)| id == from))
    }
}
```

### 5.3 Cycle Resolution Suggestions

```rust
// File: lash-core/src/graph/cycles.rs (continued)

impl Cycle {
    /// Suggest how to break this cycle
    ///
    /// Heuristics:
    /// 1. Prefer breaking explicit dependencies over hierarchy
    /// 2. Prefer breaking directory deps over explicit
    /// 3. Suggest the "weakest" link to remove
    pub fn suggest_resolution(&self, graph: &DependencyGraph) -> String {
        // Find weakest link (priority: Directory > Explicit > Hierarchy)
        let mut weakest_idx = 0;
        let mut weakest_priority = 0;

        for (i, kind) in self.edge_kinds.iter().enumerate() {
            let priority = match kind {
                DependencyKind::Directory => 3,
                DependencyKind::ExplicitId | DependencyKind::ExplicitPath => 2,
                DependencyKind::Hierarchy => 1,
            };

            if priority > weakest_priority {
                weakest_priority = priority;
                weakest_idx = i;
            }
        }

        let from = &self.path[weakest_idx];
        let to = &self.path[weakest_idx + 1];

        format!(
            "Consider removing the {} dependency from '{}' to '{}'",
            self.edge_kinds[weakest_idx].as_str(),
            from,
            to
        )
    }
}
```

---

## 6. Status Computation

### 6.1 Completion Rules

From design doc section 5.4:

- Task is **complete** if:
  - Own status is `Done`, AND
  - All children are `Done` or `Waived`, AND
  - All explicit dependencies are complete or waived

- Task is **blocked** if:
  - Any dependency is `Open` or `Blocked` (not waived)

### 6.2 Algorithm: Topological Traversal with Memoization

```rust
// File: lash-core/src/graph/status.rs

use std::collections::HashMap;

impl DependencyGraph {
    /// Compute effective status for all tasks
    ///
    /// Uses topological traversal order (dependencies before dependents)
    /// with memoization to avoid recomputation.
    ///
    /// # Complexity
    /// O(V + E) with memoization
    ///
    /// # Errors
    /// Returns error if graph has cycles (status is undefined)
    pub fn compute_all_statuses(&mut self) -> Result<()> {
        // Check for cycles first
        let cycles = self.detect_cycles()?;
        if !cycles.is_empty() {
            return Err(LashError::Dependency {
                code: "E_DEP_CYCLE",
                message: format!("Cannot compute status: graph has {} cycle(s)", cycles.len()),
                location: None,
                chain: None,
                help: Some("Remove circular dependencies before computing status".to_string()),
            });
        }

        // Clear cache
        self.status_cache.clear();

        // Compute status for each node
        for node_id in self.nodes.keys().cloned().collect::<Vec<_>>() {
            self.compute_status_cached(&node_id)?;
        }

        Ok(())
    }

    /// Compute effective status for a single task (with memoization)
    fn compute_status_cached(&mut self, full_id: &str) -> Result<EffectiveStatus> {
        // Check cache
        if let Some(&status) = self.status_cache.get(full_id) {
            return Ok(status);
        }

        let status = self.compute_status_impl(full_id)?;
        self.status_cache.insert(full_id.to_string(), status);

        Ok(status)
    }

    /// Compute effective status implementation
    fn compute_status_impl(&mut self, full_id: &str) -> Result<EffectiveStatus> {
        let node = self.nodes.get(full_id)
            .ok_or_else(|| /* node not found error */)?;

        // If waived, treat as complete
        if node.status == TaskStatus::Waived {
            return Ok(EffectiveStatus::Waived);
        }

        // If own status is Done, check dependencies
        if node.status == TaskStatus::Done {
            // Check all dependencies (children + explicit deps)
            let blockers = self.find_blockers_impl(full_id)?;

            if blockers.is_empty() {
                return Ok(EffectiveStatus::Complete);
            } else {
                // Marked done but has incomplete deps - inconsistent
                return Ok(EffectiveStatus::Blocked);
            }
        }

        // Status is Open or Blocked - check if actually blocked
        let blockers = self.find_blockers_impl(full_id)?;

        if blockers.is_empty() {
            Ok(EffectiveStatus::Ready)
        } else {
            Ok(EffectiveStatus::Blocked)
        }
    }

    /// Find all blockers for a task
    fn find_blockers_impl(&mut self, full_id: &str) -> Result<Vec<NodeId>> {
        let mut blockers = Vec::new();

        // Check direct children (hierarchy dependencies)
        for child in self.get_children(full_id) {
            let child_status = self.compute_status_cached(&child.full_id)?;
            if !matches!(child_status, EffectiveStatus::Complete | EffectiveStatus::Waived) {
                blockers.push(child.full_id.clone());
            }
        }

        // Check explicit dependencies
        if let Some(edges) = self.forward_edges.get(full_id) {
            for edge in edges {
                // Skip hierarchy edges (already checked via children)
                if edge.kind == DependencyKind::Hierarchy {
                    continue;
                }

                let dep_status = self.compute_status_cached(&edge.to)?;
                if !matches!(dep_status, EffectiveStatus::Complete | EffectiveStatus::Waived) {
                    blockers.push(edge.to.clone());
                }
            }
        }

        Ok(blockers)
    }

    /// Invalidate status cache for a node and all its ancestors
    fn invalidate_status_cache(&mut self, full_id: &str) {
        self.status_cache.remove(full_id);

        // Also invalidate all ancestors (transitively affected)
        let ancestors = self.get_ancestors(full_id);
        for (ancestor_id, _) in ancestors {
            self.status_cache.remove(&ancestor_id);
        }
    }

    /// Get the effective status of a task
    ///
    /// Must call `compute_all_statuses()` first, or this will compute on-demand.
    pub fn get_status(&mut self, full_id: &str) -> Result<EffectiveStatus> {
        self.compute_status_cached(full_id)
    }
}
```

---

## 7. Blocker Analysis

```rust
// File: lash-core/src/graph/blockers.rs

/// A blocker preventing task completion
#[derive(Debug, Clone)]
pub struct Blocker {
    /// Task that is blocking
    pub task_id: NodeId,

    /// Distance from the original task (0 = direct, 1+ = transitive)
    pub distance: usize,

    /// Dependency edge causing the block
    pub edge_kind: DependencyKind,

    /// Current status of the blocker
    pub status: TaskStatus,
}

/// Blocker analysis result
#[derive(Debug, Clone)]
pub struct BlockerReport {
    /// The task being analyzed
    pub task_id: NodeId,

    /// All direct blockers
    pub direct_blockers: Vec<Blocker>,

    /// All transitive blockers
    pub transitive_blockers: Vec<Blocker>,

    /// Root blockers (no further dependencies)
    pub root_blockers: Vec<NodeId>,
}

impl DependencyGraph {
    /// Analyze blockers for a task
    ///
    /// # Complexity
    /// O(E + V) for full transitive analysis
    pub fn analyze_blockers(&mut self, full_id: &str) -> Result<BlockerReport> {
        let mut direct_blockers = Vec::new();
        let mut transitive_blockers = Vec::new();
        let mut visited = HashSet::new();

        // Find direct blockers
        let direct_blocker_ids = self.find_blockers_impl(full_id)?;

        for blocker_id in &direct_blocker_ids {
            if let Some(node) = self.nodes.get(blocker_id) {
                // Find edge kind
                let edge_kind = self.forward_edges
                    .get(full_id)
                    .and_then(|edges| edges.iter().find(|e| &e.to == blocker_id))
                    .map(|e| e.kind)
                    .unwrap_or(DependencyKind::Hierarchy);

                direct_blockers.push(Blocker {
                    task_id: blocker_id.clone(),
                    distance: 0,
                    edge_kind,
                    status: node.status,
                });

                // Recursively find transitive blockers
                self.find_transitive_blockers(
                    blocker_id,
                    1,
                    &mut visited,
                    &mut transitive_blockers,
                )?;
            }
        }

        // Identify root blockers (no incomplete dependencies themselves)
        let mut root_blockers = Vec::new();
        for blocker in &direct_blockers {
            let blocker_deps = self.find_blockers_impl(&blocker.task_id)?;
            if blocker_deps.is_empty() {
                root_blockers.push(blocker.task_id.clone());
            }
        }

        Ok(BlockerReport {
            task_id: full_id.to_string(),
            direct_blockers,
            transitive_blockers,
            root_blockers,
        })
    }

    /// Find transitive blockers recursively
    fn find_transitive_blockers(
        &mut self,
        blocker_id: &str,
        distance: usize,
        visited: &mut HashSet<NodeId>,
        result: &mut Vec<Blocker>,
    ) -> Result<()> {
        if !visited.insert(blocker_id.to_string()) {
            return Ok(()); // Already visited
        }

        let sub_blockers = self.find_blockers_impl(blocker_id)?;

        for sub_blocker_id in sub_blockers {
            if let Some(node) = self.nodes.get(&sub_blocker_id) {
                let edge_kind = self.forward_edges
                    .get(blocker_id)
                    .and_then(|edges| edges.iter().find(|e| e.to == sub_blocker_id))
                    .map(|e| e.kind)
                    .unwrap_or(DependencyKind::Hierarchy);

                result.push(Blocker {
                    task_id: sub_blocker_id.clone(),
                    distance,
                    edge_kind,
                    status: node.status,
                });

                self.find_transitive_blockers(
                    &sub_blocker_id,
                    distance + 1,
                    visited,
                    result,
                )?;
            }
        }

        Ok(())
    }
}

impl BlockerReport {
    /// Format a human-readable blocker report
    pub fn format(&self, graph: &DependencyGraph) -> String {
        let mut output = String::new();

        if self.direct_blockers.is_empty() {
            output.push_str(&format!("Task '{}' is ready (no blockers)\n", self.task_id));
            return output;
        }

        output.push_str(&format!("Task '{}' is blocked by {} task(s):\n\n",
            self.task_id, self.direct_blockers.len()));

        for blocker in &self.direct_blockers {
            if let Some(node) = graph.get_node(&blocker.task_id) {
                output.push_str(&format!("  • {} ({}): {}\n",
                    blocker.task_id,
                    blocker.status.as_str(),
                    node.title));
            }
        }

        if !self.root_blockers.is_empty() {
            output.push_str(&format!("\nRoot blockers (work on these first):\n"));
            for root_id in &self.root_blockers {
                if let Some(node) = graph.get_node(root_id) {
                    output.push_str(&format!("  • {}: {}\n", root_id, node.title));
                }
            }
        }

        output
    }
}
```

---

## 8. Incremental Updates

```rust
// File: lash-core/src/graph/update.rs

pub struct GraphUpdate {
    graph: DependencyGraph,
}

impl GraphUpdate {
    /// Create an updater for the given graph
    pub fn new(graph: DependencyGraph) -> Self {
        Self { graph }
    }

    /// Update a task's status
    ///
    /// # Complexity
    /// O(1) for update + O(A) for cache invalidation where A = ancestors
    pub fn update_status(&mut self, full_id: &str, new_status: TaskStatus) -> Result<()> {
        let node = self.graph.nodes.get_mut(full_id)
            .ok_or_else(|| /* node not found */)?;

        node.status = new_status;
        self.graph.invalidate_status_cache(full_id);

        Ok(())
    }

    /// Add a new task node
    pub fn add_task(&mut self, node: GraphNode) -> Result<()> {
        self.graph.add_node(node)
    }

    /// Remove a task node (and all its edges)
    ///
    /// # Complexity
    /// O(E) to scan all edges
    pub fn remove_task(&mut self, full_id: &str) -> Result<()> {
        // Remove from nodes
        self.graph.nodes.remove(full_id);

        // Remove from forward edges (this task's dependencies)
        self.graph.forward_edges.remove(full_id);

        // Remove from reverse edges (this task's dependents)
        self.graph.reverse_edges.remove(full_id);

        // Remove edges pointing TO this task
        for edges in self.graph.forward_edges.values_mut() {
            edges.retain(|e| e.to != full_id);
        }

        // Remove edges FROM this task in reverse index
        for edges in self.graph.reverse_edges.values_mut() {
            edges.retain(|e| e.from != full_id);
        }

        // Remove from file index
        if let Some(node_ids) = self.graph.file_index.values_mut().find(|ids| ids.contains(&full_id.to_string())) {
            node_ids.retain(|id| id != full_id);
        }

        // Invalidate status cache
        self.graph.status_cache.clear(); // Full clear for simplicity

        Ok(())
    }

    /// Add a dependency edge
    ///
    /// Checks for cycle before adding.
    pub fn add_dependency(&mut self, from: &str, to: &str, kind: DependencyKind) -> Result<()> {
        // Check for cycle
        if self.graph.would_create_cycle(from, to)? {
            return Err(LashError::Dependency {
                code: "E_DEP_CYCLE",
                message: format!("Adding dependency from '{}' to '{}' would create a cycle", from, to),
                location: None,
                chain: None,
                help: Some("Remove conflicting dependencies to break the cycle".to_string()),
            });
        }

        let edge = GraphEdge::new(from.to_string(), to.to_string(), kind);
        self.graph.add_edge(edge)
    }

    /// Remove a dependency edge
    pub fn remove_dependency(&mut self, from: &str, to: &str) -> Result<()> {
        // Remove from forward edges
        if let Some(edges) = self.graph.forward_edges.get_mut(from) {
            edges.retain(|e| e.to != to);
        }

        // Remove from reverse edges
        if let Some(edges) = self.graph.reverse_edges.get_mut(to) {
            edges.retain(|e| e.from != from);
        }

        // Invalidate status cache
        self.graph.invalidate_status_cache(from);

        Ok(())
    }

    /// Consume the updater and return the updated graph
    pub fn finish(self) -> DependencyGraph {
        self.graph
    }
}
```

---

## 9. Graph Export

```rust
// File: lash-core/src/graph/export.rs

use std::io::Write;

/// Graph export format
pub enum ExportFormat {
    /// Graphviz DOT format
    Dot,
    /// JSON format
    Json,
    /// ASCII tree for terminal
    AsciiTree,
}

/// Export options
pub struct ExportOptions {
    /// Only include tasks with these labels
    pub filter_labels: Vec<String>,

    /// Only include tasks in this file
    pub filter_file: Option<String>,

    /// Hide completed tasks
    pub hide_completed: bool,

    /// Show only direct dependencies (not transitive)
    pub direct_only: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            filter_labels: Vec::new(),
            filter_file: None,
            hide_completed: false,
            direct_only: false,
        }
    }
}

impl DependencyGraph {
    /// Export graph in the specified format
    pub fn export<W: Write>(
        &self,
        writer: &mut W,
        format: ExportFormat,
        options: &ExportOptions,
    ) -> Result<()> {
        match format {
            ExportFormat::Dot => self.export_dot(writer, options),
            ExportFormat::Json => self.export_json(writer, options),
            ExportFormat::AsciiTree => self.export_ascii_tree(writer, options),
        }
    }

    /// Export as Graphviz DOT format
    fn export_dot<W: Write>(&self, writer: &mut W, options: &ExportOptions) -> Result<()> {
        writeln!(writer, "digraph dependencies {{")?;
        writeln!(writer, "  rankdir=LR;")?;
        writeln!(writer, "  node [shape=box];")?;
        writeln!(writer)?;

        // Nodes
        for node in self.filtered_nodes(options) {
            let color = match node.status {
                TaskStatus::Done => "green",
                TaskStatus::Open => "yellow",
                TaskStatus::Blocked => "red",
                TaskStatus::Waived => "gray",
            };

            writeln!(
                writer,
                "  \"{}\" [label=\"{}\\n{}\", color={}, style=filled];",
                node.full_id,
                node.task_id,
                node.status.as_str(),
                color
            )?;
        }

        writeln!(writer)?;

        // Edges
        for (from, edges) in &self.forward_edges {
            if !self.should_include_node(from, options) {
                continue;
            }

            for edge in edges {
                if !self.should_include_node(&edge.to, options) {
                    continue;
                }

                let style = match edge.kind {
                    DependencyKind::Hierarchy => "solid",
                    DependencyKind::ExplicitId | DependencyKind::ExplicitPath => "bold",
                    DependencyKind::Directory => "dashed",
                };

                writeln!(
                    writer,
                    "  \"{}\" -> \"{}\" [style={}];",
                    from, edge.to, style
                )?;
            }
        }

        writeln!(writer, "}}")?;

        Ok(())
    }

    /// Export as JSON
    fn export_json<W: Write>(&self, writer: &mut W, options: &ExportOptions) -> Result<()> {
        use serde_json::json;

        let nodes: Vec<_> = self.filtered_nodes(options)
            .map(|n| json!({
                "id": n.full_id,
                "title": n.title,
                "status": n.status.as_str(),
                "file_id": n.file_id,
                "task_id": n.task_id,
            }))
            .collect();

        let edges: Vec<_> = self.forward_edges.iter()
            .flat_map(|(from, edges)| {
                edges.iter()
                    .filter(|e| {
                        self.should_include_node(from, options)
                            && self.should_include_node(&e.to, options)
                    })
                    .map(|e| json!({
                        "from": from,
                        "to": e.to,
                        "kind": e.kind.as_str(),
                    }))
            })
            .collect();

        let output = json!({
            "nodes": nodes,
            "edges": edges,
        });

        writeln!(writer, "{}", serde_json::to_string_pretty(&output)?)?;

        Ok(())
    }

    /// Export as ASCII tree
    fn export_ascii_tree<W: Write>(&self, writer: &mut W, options: &ExportOptions) -> Result<()> {
        // Find root nodes (no incoming edges from outside current filter)
        let roots: Vec<_> = self.nodes.keys()
            .filter(|id| {
                self.should_include_node(id, options)
                    && self.reverse_edges.get(id.as_str()).map_or(true, |e| e.is_empty())
            })
            .collect();

        for root_id in roots {
            self.export_tree_node(writer, root_id, 0, options)?;
        }

        Ok(())
    }

    /// Recursively export tree node
    fn export_tree_node<W: Write>(
        &self,
        writer: &mut W,
        node_id: &str,
        depth: usize,
        options: &ExportOptions,
    ) -> Result<()> {
        if let Some(node) = self.nodes.get(node_id) {
            let indent = "  ".repeat(depth);
            let status_char = match node.status {
                TaskStatus::Done => "✓",
                TaskStatus::Open => "○",
                TaskStatus::Blocked => "✗",
                TaskStatus::Waived => "−",
            };

            writeln!(
                writer,
                "{}{} {} ({})",
                indent,
                status_char,
                node.title,
                node.task_id
            )?;

            // Print children
            if let Some(edges) = self.forward_edges.get(node_id) {
                for edge in edges {
                    if self.should_include_node(&edge.to, options) {
                        self.export_tree_node(writer, &edge.to, depth + 1, options)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Filter nodes based on options
    fn filtered_nodes<'a>(&'a self, options: &'a ExportOptions) -> impl Iterator<Item = &'a GraphNode> + 'a {
        self.nodes.values().filter(move |node| self.should_include_node(&node.full_id, options))
    }

    /// Check if node should be included based on filters
    fn should_include_node(&self, full_id: &str, options: &ExportOptions) -> bool {
        let node = match self.nodes.get(full_id) {
            Some(n) => n,
            None => return false,
        };

        // Filter by file
        if let Some(ref file_filter) = options.filter_file {
            if &node.file_id != file_filter {
                return false;
            }
        }

        // Filter by completed
        if options.hide_completed && node.status == TaskStatus::Done {
            return false;
        }

        // Filter by labels (would need access to metadata - TODO)

        true
    }
}
```

---

## 10. Integration with Existing Code

### 10.1 Module Structure

```
lash-core/src/graph/
├── mod.rs           # Public API exports
├── types.rs         # GraphNode, GraphEdge, DependencyGraph
├── builder.rs       # GraphBuilder (from DB)
├── queries.rs       # Query operations (impl for DependencyGraph)
├── cycles.rs        # Cycle detection (CycleDetector, Cycle)
├── status.rs        # Status computation (EffectiveStatus)
├── blockers.rs      # Blocker analysis (BlockerReport)
├── update.rs        # Incremental updates (GraphUpdate)
└── export.rs        # Export formats (DOT, JSON, ASCII)
```

### 10.2 Public API (mod.rs)

```rust
// File: lash-core/src/graph/mod.rs

mod types;
mod builder;
mod queries;
mod cycles;
mod status;
mod blockers;
mod update;
mod export;

pub use types::{DependencyGraph, GraphNode, GraphEdge, NodeId, EffectiveStatus};
pub use builder::GraphBuilder;
pub use cycles::{Cycle, CycleDetector};
pub use status::StatusComputer; // If we extract to separate struct
pub use blockers::{Blocker, BlockerReport};
pub use update::GraphUpdate;
pub use export::{ExportFormat, ExportOptions};
```

### 10.3 Integration with lash-db

The graph is built from the DB layer:

```rust
use lash_db::{open_database, repository::DependencyRepository};
use lash_core::graph::{GraphBuilder, DependencyGraph};

// Open database
let conn = open_database(&db_path)?;

// Build graph from database
let graph = GraphBuilder::new(&conn).build()?;

// Now use graph for analysis
let cycles = graph.detect_cycles()?;
if !cycles.is_empty() {
    eprintln!("Found {} cycles:", cycles.len());
    for cycle in cycles {
        eprintln!("  {}", cycle.format(&graph));
    }
}
```

### 10.4 Relationship to DB Layer

**Division of Responsibilities:**

| Capability | DB Layer | In-Memory Graph |
|------------|----------|-----------------|
| Persistence | ✓ (SQLite) | ✗ |
| Single-edge cycle check | ✓ (`would_create_cycle`) | ✓ (comprehensive) |
| Transitive closure | ✓ (`dependency_closure` table) | ✓ (DFS) |
| Comprehensive cycle detection | ✗ | ✓ (DFS with path tracking) |
| Status computation | ✗ | ✓ |
| Blocker analysis | ✗ | ✓ |
| Graph export | ✗ | ✓ |
| Incremental updates | ✓ (via DependencyUpdater) | ✓ (in-memory) |

**Flow:**
1. DB stores dependencies and maintains closure table
2. Graph is built from DB for analysis
3. Graph performs rich algorithms (cycles, status, blockers)
4. Updates go through DB first, then graph is rebuilt or incrementally updated

---

## 11. Performance Analysis

### 11.1 Time Complexity Summary

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Build graph | O(V + E) | Load nodes + edges from DB |
| Get node by ID | O(1) | HashMap lookup |
| Get dependencies | O(1) + O(k) | k = num dependencies |
| Get dependents | O(1) + O(k) | k = num dependents |
| Get descendants (transitive) | O(E + V) | DFS, visits each reachable once |
| Get ancestors (transitive) | O(E + V) | Reverse DFS |
| Detect all cycles | O(E + V) | DFS with color marking |
| Would create cycle | O(E + V) | Worst case: full DFS |
| Compute all statuses | O(E + V) | With memoization |
| Analyze blockers | O(E + V) | Transitive blocker search |
| Add/remove node | O(1) - O(E) | Depends on edge cleanup |
| Add/remove edge | O(1) | Plus cache invalidation |
| Export (all formats) | O(V + E) | Visit all nodes/edges |

### 11.2 Space Complexity

- **Nodes:** O(V) - HashMap + Vec in file_index
- **Edges:** O(E) - Two HashMaps (forward + reverse)
- **Indexes:** O(V) - file_index + db_id_index
- **Cache:** O(V) - status_cache
- **Total:** O(V + E)

For typical graphs (E ≈ 3V): ~4-5x memory overhead compared to minimal node storage.

### 11.3 Optimization Opportunities

**Current Design:**
- Status caching reduces recomputation from O(V²) to O(V)
- Bidirectional edges avoid reverse graph traversal

**Future Optimizations (if needed):**
1. **Lazy index building:** Only build file_index on first query
2. **Compact node representation:** Use integer node IDs internally, map to strings
3. **Edge compression:** Store edges as Vec<(u32, u32, u8)> with separate kind lookup
4. **Transitive closure table:** Precompute all reachability (space/time tradeoff)
5. **Parallel algorithms:** Parallel cycle detection for disconnected components

**When to Optimize:**
- Graph size > 10,000 tasks
- Repeated transitive queries dominate runtime
- Memory pressure on resource-constrained systems

---

## 12. Error Handling

### 12.1 Error Types

```rust
// Extend lash_types::error::LashError with graph-specific variants

pub enum LashError {
    // Existing variants...

    /// Graph construction error
    Graph {
        code: &'static str,
        message: String,
        node_id: Option<String>,
        help: Option<String>,
    },

    /// Cycle detected in dependency graph
    CycleDetected {
        code: &'static str,
        cycles: Vec<Cycle>,
        help: Option<String>,
    },
}
```

### 12.2 Error Codes

| Code | Meaning | Resolution |
|------|---------|------------|
| `E_GRAPH_NODE_NOT_FOUND` | Node ID doesn't exist | Check full_id format |
| `E_GRAPH_DUPLICATE_NODE` | Node already exists | Use unique task IDs |
| `E_GRAPH_INVALID_EDGE` | Edge endpoints missing | Ensure nodes exist first |
| `E_GRAPH_CYCLE` | Circular dependency | Remove conflicting deps |
| `E_GRAPH_INCONSISTENT` | Graph invariants violated | Rebuild from DB |

---

## 13. Testing Strategy

### 13.1 Unit Tests

**tests/graph/types_test.rs:**
- GraphNode creation and validation
- GraphEdge equality and hashing
- DependencyGraph initialization

**tests/graph/queries_test.rs:**
- Direct dependencies/dependents
- Transitive descendants/ancestors
- Children (hierarchy)
- Node lookups (by ID, by DB ID, by file)

**tests/graph/cycles_test.rs:**
- Acyclic graph (no cycles)
- Simple cycle (A → B → A)
- Complex cycle (A → B → C → D → B)
- Multiple disjoint cycles
- Self-loop (A → A)
- Cycle with mixed edge types

**tests/graph/status_test.rs:**
- Simple chain (A → B → C) with various statuses
- Waived dependencies ignored
- Blocked propagation
- Parent/child consistency
- Memoization correctness

**tests/graph/blockers_test.rs:**
- Direct blockers only
- Transitive blocker chain
- Multiple independent blockers
- Root blocker identification

**tests/graph/update_test.rs:**
- Add/remove nodes
- Add/remove edges
- Status updates
- Cache invalidation
- Cycle prevention on edge add

**tests/graph/export_test.rs:**
- DOT output validation
- JSON parsing
- ASCII tree structure
- Filter options

### 13.2 Integration Tests

**tests/graph_integration_test.rs:**
- Build graph from fixture database
- Verify node count, edge count
- Run cycle detection on real data
- Compute statuses for entire graph
- Export and verify output

### 13.3 Performance Benchmarks

**benches/graph_bench.rs:**
- Graph construction (100, 1000, 10000 tasks)
- Cycle detection (various graph structures)
- Status computation (deep vs wide hierarchies)
- Transitive queries (descendants/ancestors)
- Incremental updates

---

## 14. Open Questions & Design Decisions

### 14.1 Resolved Decisions

✅ **Adjacency list vs matrix:** Adjacency list (memory efficient for sparse graphs)
✅ **Cycle detection algorithm:** DFS with three-color marking (standard, path tracking)
✅ **Status caching strategy:** Memoization with ancestor invalidation
✅ **Graph ownership:** Graph owns nodes, builder pattern for construction
✅ **Integration with DB:** Graph built from DB, not parallel data structure

### 14.2 Open Questions

❓ **Incremental vs full rebuild:** When to rebuild entire graph vs incremental update?
- **Proposal:** Rebuild on file changes, incremental on status changes only
- **Rationale:** File changes are infrequent, status changes are frequent

❓ **Persistence of transitive closure:** Should we persist transitive closure in DB?
- **Current:** DB has `dependency_closure` table
- **Graph:** Computes on-demand with DFS
- **Trade-off:** Space (O(V²) worst case) vs Time (O(V+E) per query)
- **Recommendation:** Keep both - DB for persistence, graph for analysis

❓ **Parallel algorithms:** Worth implementing parallel cycle detection?
- **Proposal:** Only if graphs exceed 10,000 tasks
- **Complexity:** Requires thread-safe data structures

❓ **Node ID representation:** String vs numeric IDs internally?
- **Current:** String-based (file_id#task_id)
- **Alternative:** Internal u32/u64 IDs with string lookup table
- **Trade-off:** Memory (strings are 24+ bytes) vs complexity
- **Recommendation:** Start with strings, optimize if memory pressure emerges

---

## 15. Migration Path & Implementation Plan

### 15.1 Phase 1: Core Data Structures (Task 1)

**Files to create:**
- `lash-core/src/graph/mod.rs`
- `lash-core/src/graph/types.rs`
- `lash-core/src/graph/builder.rs`
- `lash-core/src/graph/queries.rs`

**Tests:**
- Unit tests for GraphNode, GraphEdge
- Unit tests for add_node, add_edge
- Unit tests for query operations

**Duration:** 2-3 days

### 15.2 Phase 2: Cycle Detection (Task 2)

**Files to create:**
- `lash-core/src/graph/cycles.rs`

**Tests:**
- Unit tests for various cycle patterns
- Integration test with fixture data

**Duration:** 2-3 days

### 15.3 Phase 3: Dependency Resolution Engine (Task 3)

**Note:** This overlaps with parser/indexer work. Graph assumes dependencies are already resolved in DB.

**Files to modify:**
- `lash-db/src/indexer.rs` (ensure dependencies are fully resolved)
- `lash-db/src/dependency_updater.rs` (already exists)

**Graph changes:** None - graph consumes resolved dependencies from DB

**Duration:** Covered by Task 3 in dependency-resolution tasks

### 15.4 Phase 4: Status Computation (Task 4)

**Files to create:**
- `lash-core/src/graph/status.rs`

**Tests:**
- Unit tests for status computation rules
- Integration test with real graph

**Duration:** 2-3 days

### 15.5 Phase 5: Blocker Analysis (Task 5)

**Files to create:**
- `lash-core/src/graph/blockers.rs`

**Tests:**
- Unit tests for blocker identification
- Report formatting tests

**Duration:** 2 days

### 15.6 Phase 6: Graph Export (Task 6)

**Files to create:**
- `lash-core/src/graph/export.rs`

**Tests:**
- Output validation for each format
- Manual inspection of rendered graphs

**Duration:** 1-2 days

### 15.7 Phase 7: Incremental Updates (Task 7)

**Files to create:**
- `lash-core/src/graph/update.rs`

**Tests:**
- Unit tests for each update operation
- Cache invalidation correctness
- Integration test with DB sync

**Duration:** 2-3 days

### 15.8 Total Estimated Duration

**10-16 days** (matching task breakdown in dependency-resolution.md)

---

## 16. Alternatives Considered

### 16.1 Third-Party Graph Libraries

**Evaluated:**
- **petgraph:** Most popular Rust graph library
  - **Pros:** Battle-tested, rich algorithms, stable API
  - **Cons:** Generic design adds complexity, less control over memory layout
  - **Decision:** Could use as a foundation, but custom implementation gives more control

- **graphlib:** Lightweight, simple API
  - **Pros:** Minimal dependencies
  - **Cons:** Less feature-rich
  - **Decision:** Too minimal for our needs

**Final Decision:** Custom implementation
- **Rationale:**
  - Full control over data structures (optimized for our use case)
  - Tight integration with existing types (DependencyKind, TaskStatus)
  - Learning opportunity for team
  - Avoids dependency on external APIs
  - Can still use petgraph algorithms if needed later

### 16.2 Database-Only Approach

**Alternative:** Keep all graph operations in SQL (recursive CTEs)

**Pros:**
- Single source of truth
- No sync issues
- Leverages SQLite's optimizations

**Cons:**
- SQL complexity for advanced algorithms
- Performance overhead for repeated queries
- Harder to test and debug
- Limited algorithm expressiveness

**Decision:** Hybrid approach (DB for persistence, in-memory for analysis)

---

## 17. Future Enhancements

### 17.1 Post-v1 Features

**Incremental cycle detection:**
- Track only affected subgraph on updates
- Avoid full DFS when small change occurs

**Persistent transitive closure:**
- Sync in-memory graph with DB closure table
- O(1) reachability queries

**Graph visualization server:**
- HTTP API for live graph rendering
- WebSocket for real-time updates
- Integration with D3.js or similar

**Distributed graph:**
- Shard graph across multiple processes
- For extremely large projects (100k+ tasks)

**Machine learning features:**
- Predict task completion time based on historical data
- Suggest optimal task ordering
- Detect anomalous dependency patterns

### 17.2 API Stability

**v1.0 Guarantees:**
- Core types (GraphNode, GraphEdge, DependencyGraph) are stable
- Query API is stable
- Cycle detection API is stable

**Experimental (may change):**
- Export formats (may add more)
- Update API (may optimize for batch operations)
- Internal indexes (may change representation)

---

## 18. References

### 18.1 Literature

- **Introduction to Algorithms (CLRS), Chapter 22:** Graph algorithms (DFS, cycle detection)
- **The Algorithm Design Manual (Skiena), Chapter 5:** Graph traversal and topological sorting
- **Rust By Example - Graphs:** https://doc.rust-lang.org/rust-by-example/std/graph.html

### 18.2 Related Code

- **lash-types/src/dependency.rs:** Core dependency types
- **lash-db/src/repository/dependencies.rs:** DB-level dependency operations
- **lash-types/src/task.rs:** TaskTree (within-file hierarchy)

### 18.3 Design Documents

- **docs/design-doc.md:** Overall Lash design (section 5: Dependency Model)
- **tasks/tasks.dependency-resolution.md:** Task breakdown for this work

---

## 19. Glossary

- **Node:** A task in the dependency graph (GraphNode)
- **Edge:** A dependency relationship between two tasks (GraphEdge)
- **Forward edge:** Task A → Task B means "A depends on B"
- **Reverse edge:** Stored for efficient lookup of "who depends on me?"
- **Descendant:** Task reachable by following forward edges (transitive dependencies)
- **Ancestor:** Task reachable by following reverse edges (transitive dependents)
- **Cycle:** Path from a task back to itself
- **Blocker:** A dependency preventing task completion
- **Transitive closure:** All reachable nodes from a starting node
- **Topological order:** Ordering where dependencies come before dependents
- **DFS:** Depth-first search
- **BFS:** Breadth-first search (not used in this design)

---

**Document Status:** Design Complete, Ready for Implementation
**Next Steps:** Review with team, then proceed to Phase 1 implementation (Task 1)
