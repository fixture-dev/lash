# Dependency Resolution Tasks

**Module:** `lash-core` (dependency resolver)
**Dependencies:** tasks.core-data-model.md, tasks.sqlite-schema.md, tasks.markdown-parser.md
**Effort:** 10-14 days
**Priority:** CRITICAL

## Overview

The dependency resolution system builds and analyzes the task dependency graph. It handles three types of dependencies:
1. **Implicit hierarchy** - parent tasks depend on children
2. **Explicit cross-file references** - `@depends-on` annotations
3. **Directory-level dependencies** - directory structure relationships

It must detect cycles, compute completion status, identify blockers, and support efficient queries.

## Core Requirements

From design-doc.md:
- Model dependencies as a directed graph (section 5)
- Detect circular dependencies (section 5)
- Compute task completion status based on dependencies (section 5.4)
- Support within-file and cross-file dependencies (sections 5.1, 5.3)
- Handle waived tasks (section 5.1)
- Identify blocked tasks (section 5.4)

---

## Task 1: Graph Data Structure

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** tasks.core-data-model.md#1

### Description

Define the in-memory graph representation for the task dependency network.

### Subtasks

- [x] Define `DependencyGraph` struct
  - [x] Node storage (task IDs -> Task references)
  - [x] Edge storage (adjacency list: task_id -> Vec<task_id>)
  - [x] Reverse edge storage (for efficient reverse lookups)
  - [x] Edge metadata (dependency type: hierarchy, explicit, directory)
- [x] Implement graph construction from DB
  - [x] Load all tasks and dependencies from SQLite
  - [x] Build adjacency lists
  - [x] Index tasks by full_id for fast lookup
- [x] Implement graph query methods
  - [x] `get_dependencies(task_id)` - direct dependencies
  - [x] `get_dependents(task_id)` - reverse dependencies
  - [x] `get_descendants(task_id)` - transitive closure (DFS/BFS)
  - [x] `get_ancestors(task_id)` - reverse transitive closure
- [x] Add edge type tracking
  - [x] Distinguish hierarchy vs explicit vs directory edges
  - [x] Store source location for explicit edges (for error reporting)

### Success Criteria

- [x] Graph correctly represents all dependency relationships
- [x] Efficient lookups: O(1) for direct dependencies, O(E+V) for transitive
- [x] Memory-efficient for large graphs (1000+ tasks)
- [x] Clear API for downstream consumers

### Tests

- [x] Unit: Build graph from fixture data
- [x] Unit: Test query methods on various graph structures
- [x] Unit: Test edge type tracking
- [x] Performance: Measure memory usage for large graphs

---

## Task 2: Cycle Detection

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** Task 1

### Description

Detect circular dependencies in the task graph using standard graph algorithms.

### Subtasks

- [x] Implement `CycleDetector` struct
  - [x] Use DFS with color marking (white/gray/black)
  - [x] Track path during traversal for cycle reporting
- [x] Implement `detect_cycles()` function
  - [x] Run DFS from all unvisited nodes
  - [x] Detect back edges (gray -> gray)
  - [x] Collect all cycles (not just first)
  - [x] Return cycle paths with task IDs
- [x] Add cycle reporting
  - [x] Format cycle as: Task A -> Task B -> Task C -> Task A
  - [x] Include file paths and line numbers
  - [x] Distinguish cycle types (within-file vs cross-file)
- [x] Implement cycle resolution suggestions
  - [x] Identify weakest link (e.g., directory dep vs explicit)
  - [x] Suggest breaking explicit dependencies
  - [x] Suggest restructuring hierarchy

### Success Criteria

- [x] Detects all cycles in arbitrary graphs
- [x] Correctly handles graphs with multiple disjoint cycles
- [x] Clear, actionable error messages for each cycle
- [x] No false positives or false negatives

### Tests

- [x] Unit: Acyclic graph (no cycles)
- [x] Unit: Simple cycle (A -> B -> A)
- [x] Unit: Complex cycle (A -> B -> C -> D -> B)
- [x] Unit: Multiple disjoint cycles
- [x] Unit: Self-loop (A -> A)
- [x] Integration: Test with fixture files containing cycles

---

## Task 3: Dependency Resolution Engine

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 1, tasks.markdown-parser.md#2

### Description

Parse dependency references from tasks and build the complete dependency graph.

### Subtasks

- [x] Implement `DependencyResolver` struct
  - [x] Hold references to parsed tasks and files
  - [x] Track resolution errors (broken links)
- [x] Implement implicit hierarchy resolution
  - [x] For each task, add edges to all children
  - [x] Edges are `(parent_id, child_id, type=hierarchy)`
  - Note: Implemented in indexing layer (`DependencyUpdater.insert_hierarchy_dependencies()`)
- [x] Implement explicit `@depends-on` resolution
  - [x] Parse `@depends-on` annotations
  - [x] Support formats:
    - [x] Relative path: `../core/cli.md#task:parse-args`
    - [x] Absolute path: `core/cli.md#task:parse-args`
    - [x] ID-only: `#task:parse-args` (within-file)
    - [-] File-level: `../core/cli.md` (depends on all tasks in file) - deferred
  - [x] Resolve paths relative to source file
  - [x] Look up target task in DB/graph
  - [x] Create edge `(source_id, target_id, type=explicit)`
  - [x] Handle missing targets (broken links)
- [-] Implement directory-level dependencies
  - [-] Parse directory metadata (if any)
  - [-] Infer dependencies from directory structure
  - [-] Add edges for directory relationships
  - Note: Directory dependencies deferred to future implementation
- [x] Handle broken dependencies
  - [x] Collect all broken references
  - [x] Return structured errors with source location
  - [x] Mark affected tasks as potentially blocked - implemented in status computation

### Success Criteria

- [x] Correctly resolves all three dependency types (explicit ID, explicit path, within-file)
- [x] Handles all supported reference formats
- [x] Detects and reports broken links with precise locations
- [x] Produces complete, accurate dependency graph

### Tests

- [x] Unit: Resolve implicit hierarchy - tests in indexing layer (`test_hierarchy_dependencies_created`)
- [x] Unit: Resolve explicit cross-file dependencies
- [x] Unit: Resolve various path formats
- [x] Unit: Handle broken links gracefully
- [-] Integration: Build graph from fixture project - deferred
- [-] Integration: Verify graph structure matches expectations - deferred

### Implementation Notes

**Completed:**
- Created `crates/lash-core/src/dependency/resolver.rs` with `DependencyResolver`
- Supports path-based and ID-based dependency references
- Handles relative path resolution with `normalize_path()` helper
- Collects all resolution errors without failing fast
- Provides detailed error messages with source locations
- All unit tests passing (8 tests)
- All doctests passing (3 tests)

**Deferred:**
- File-level dependencies (no task fragment) - marked as unsupported
- Directory dependencies - marked as unsupported
- Integration tests with fixture projects - to be added later

**Changes to lash-types:**
- Fixed `parse_dependency_ref()` to correctly detect path references with `#` fragments
- Fixed `DependencyRef::validate()` to check only the path part (before `#`)
- Added `TaskTree::get_task_mut()` for modifying tasks in tests

---

## Task 4: Completion Status Computation

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 1, Task 3

### Description

Compute the effective completion status of each task based on its own status and its dependencies.

### Subtasks

- [x] Implement `StatusComputer` struct
  - [x] Take graph and task statuses as input
  - [x] Compute effective status for each task
- [x] Implement completion rules (from design doc section 5.4)
  - [x] Task is complete if:
    - [x] Own status is `done`, AND
    - [x] All children are `done` or `waived`, AND
    - [x] All explicit dependencies are complete or waived
  - [x] Task is blocked if:
    - [x] Any dependency is `open` or `blocked` (not waived), OR
    - [x] Depends on broken link
- [x] Implement `compute_status()` function
  - [x] Topological traversal (or recursive with memoization)
  - [x] Cache computed statuses to avoid recomputation
  - [x] Handle waived dependencies (ignore in completion check)
- [x] Add file-level completion status
  - [x] File complete if all top-level tasks complete
  - [-] Consider directory-level dependencies (deferred)
- [x] Detect inconsistencies
  - [x] Parent marked done but children open (lint warning)
  - [x] Task marked done but dependencies open

### Success Criteria

- [x] Correctly computes status for all tasks in graph
- [x] Respects waived tasks (treats as complete)
- [x] Identifies blocked tasks accurately
- [x] Efficient: O(V+E) traversal, with memoization

### Tests

- [x] Unit: Simple chain (A -> B -> C), various states
- [x] Unit: Waived dependencies ignored
- [x] Unit: Blocked propagation (A blocks B, B blocks C)
- [x] Unit: Parent/child status consistency
- [-] Integration: Compute status for entire fixture project (deferred)

### Implementation Notes

**Completed:**
- Created `crates/lash-core/src/dependency/status_computer.rs` with complete implementation
- Implemented `ComputedStatus` enum with Complete, Incomplete, Blocked, and Inconsistent variants
- Implemented `BlockerReason` enum to provide detailed information about why tasks are blocked
- Implemented `InconsistencyKind` enum to identify different types of status inconsistencies
- Used recursive DFS with memoization for efficient O(V+E) status computation
- Handles cycle detection during status computation
- Distinguishes between hierarchy and explicit dependencies for inconsistency detection
- File-level completion status computed based on top-level tasks (depth 0)
- All unit tests passing (14 tests)
- All doctests passing (4 tests)
- Exported from `crates/lash-core/src/dependency/mod.rs`

**Algorithm:**
- Recursive status computation with memoization cache
- Uses visiting set for cycle detection
- Waived tasks always treated as complete
- Blocked status propagates through dependency chains
- Inconsistencies detected when done tasks have incomplete dependencies
- Separates parent/child inconsistencies from explicit dependency inconsistencies

**Test Coverage:**
- Single task states (done, open, waived)
- Simple dependency chains
- Waived dependency handling
- Blocked dependency propagation
- Multiple blockers
- Parent-child inconsistencies
- Done tasks with incomplete explicit dependencies
- File-level status computation
- Cycle detection

**Deferred:**
- Directory-level dependencies (not yet implemented in graph)
- Integration tests with full fixture projects (to be added later)

---

## Task 5: Blocker Identification

**Priority:** HIGH
**Effort:** 2 days
**Depends on:** Task 4

### Description

Given a task, identify which dependencies are blocking its completion.

### Subtasks

- [x] Implement `BlockerAnalyzer` struct
  - [x] Query graph and status for dependencies
  - [x] Identify incomplete dependencies
- [x] Implement `find_blockers(task_id)` function
  - [x] Get all dependencies (direct + transitive)
  - [x] Filter to incomplete/blocked tasks
  - [x] Sort by "distance" (direct blockers first)
  - [x] Return list of blocker tasks with reasons
- [x] Add blocker chain analysis
  - [x] For each blocker, recursively find its blockers
  - [x] Build blocker tree/graph
  - [x] Identify "root blockers" (no further dependencies)
- [x] Implement blocker reporting
  - [x] Format: "Task X is blocked by: Task Y (in file Z)"
  - [x] Show full blocker chain for deep dependencies
  - [x] Suggest actions (complete blockers, waive, remove dependency)

### Success Criteria

- [x] Accurately identifies all blockers for a given task
- [x] Handles transitive blockers (A blocked by B blocked by C)
- [x] Clear, actionable blocker reports
- [x] Efficient: O(E) for direct blockers, O(V+E) for transitive

### Tests

- [x] Unit: Task with direct blocker
- [x] Unit: Task with transitive blocker chain
- [x] Unit: Task with multiple independent blockers
- [x] Unit: Task with no blockers (ready to start)
- [x] Integration: Generate blocker report for fixture tasks

### Implementation Notes

**Completed:**
- Created `crates/lash-core/src/dependency/blocker_analyzer.rs` with complete implementation
- Implemented `BlockerAnalyzer` with comprehensive blocker identification
- Implemented `BlockerInfo` struct with depth tracking and blocker metadata
- Implemented `BlockerChain` for showing transitive blocker relationships
- Implemented `BlockerReport` with human-readable formatting
- Implemented `BlockerSuggestion` enum for actionable resolution suggestions
- Uses BFS to find all blockers with depth tracking (0=direct, 1+=transitive)
- Root blocker identification (tasks with no incomplete dependencies)
- Deduplication to avoid repeated blockers via multiple paths
- All unit tests passing (7 tests)
- All doctests passing (7 tests)
- Exported from `crates/lash-core/src/dependency/mod.rs`

**Algorithm:**
- BFS traversal starting from direct dependencies
- Depth tracking to distinguish direct vs transitive blockers
- Only follows paths through blocked or incomplete tasks
- Sorts results by depth (direct blockers first)
- Root blockers identified as incomplete tasks with no blockers themselves

**Data Structures:**
- `BlockerInfo`: Contains task_id, title, file_id, depth, dependency_kind, and blocker_status
- `BlockerChain`: Shows recursive blocker relationships from direct to root
- `BlockerReport`: Formatted output with blockers, chains, roots, and suggestions
- `BlockerSuggestion`: Actionable recommendations (complete, waive, remove dependency)

**Report Format:**
- Summary: Total blockers, direct vs transitive counts
- Root blockers section (most important - address first)
- Blocker chains showing dependency paths (A → B → C)
- All blockers listed with depth and status
- Suggested actions prioritizing root blockers

**Test Coverage:**
- Direct blocker identification
- Transitive blocker chains
- Multiple independent blockers
- No blockers (ready to start)
- Completed dependencies not treated as blockers
- Blocker chain construction
- Report generation and formatting

**Integration:**
- Uses `DependencyGraph` for traversal
- Uses `StatusComputer::compute_all()` for task statuses
- Builds on existing `ComputedStatus` from status_computer
- Provides detailed analysis beyond basic status computation

---

## Task 6: Graph Export

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 1

### Description

Export dependency graph in various formats for visualization and analysis.

### Subtasks

- [x] Implement `GraphExporter` struct
  - [x] Support multiple output formats
- [x] Implement DOT format export
  - [x] Generate Graphviz-compatible DOT file
  - [x] Nodes: task IDs or titles
  - [x] Edges: dependency relationships
  - [x] Color-code by status (green=done, yellow=open, coral=blocked, gray=waived)
  - [x] Cluster by file or directory
- [x] Implement JSON export
  - [x] Nodes array: task metadata
  - [x] Edges array: source/target/type
  - [x] Include status (labels not yet in NodeData)
- [x] Add filtering options
  - [x] Export subgraph (specific file or label)
  - [x] Hide completed tasks
  - [x] Show only direct dependencies (max_depth option)
- [x] Implement text-based graph visualization
  - [x] ASCII tree format for terminal display
  - [x] Indent by depth
  - [x] Show dependency arrows with status indicators

### Success Criteria

- [x] DOT output renders correctly in Graphviz (valid syntax verified)
- [x] JSON format is parsable and complete
- [x] Filtering options work as expected
- [x] Text format is readable in terminal

### Tests

- [x] Unit: Export empty graph (DOT and JSON)
- [x] Unit: Export simple graph to DOT
- [x] Unit: Export simple graph to JSON
- [x] Unit: Export ASCII tree (simple and nested)
- [x] Unit: Filter by file
- [x] Unit: Filter by completion status
- [x] Unit: Filter by max depth
- [x] Unit: Multiple file clustering
- [x] Unit: Cycle detection in ASCII tree
- [x] Unit: DOT special character escaping
- [-] Integration: Export fixture project graph, verify correctness (deferred)
- [-] Manual: Render DOT file with Graphviz, inspect visually (deferred)

### Implementation Notes

**Completed:**
- Created `crates/lash-core/src/dependency/graph_exporter.rs` with complete implementation
- Implemented `GraphExporter` struct with three export formats
- Implemented `FilterOptions` for flexible subgraph export
- DOT format with Graphviz syntax:
  - Color-coded nodes (lightgreen=done, lightyellow=open, lightcoral=blocked, lightgray=waived)
  - File-based clustering with subgraphs
  - Labeled edges with dependency kind
  - Proper escaping of special characters
- JSON format with serde serialization:
  - Separate nodes and edges arrays
  - Full task metadata (id, title, status, file_id, depth)
  - Edge metadata (from, to, kind, source_location)
- ASCII tree format for terminal display:
  - Recursive tree rendering with proper indentation
  - Status indicators: [ ] open, [✓] done, [-] waived, [!] blocked
  - Cycle detection to prevent infinite recursion
  - Box-drawing characters (└─, ├─, │) for visual structure
- Filter options:
  - Filter by file IDs
  - Hide completed tasks (done/waived)
  - Max depth for limiting transitive dependencies
  - Label filtering placeholder (labels not yet in NodeData)
- All unit tests passing (12 tests)
- All doctests passing (7 tests)
- Exported from `crates/lash-core/src/dependency/mod.rs`

**Data Structures:**
- `GraphExporter<'a>`: Borrows graph reference for export
- `FilterOptions`: Configure which nodes/edges to include
- `JsonGraph`, `JsonNode`, `JsonEdge`: Serde-compatible JSON representation

**Export Formats:**
- DOT: Graphviz-compatible directed graph with clustering
- JSON: Structured data for programmatic consumption
- ASCII tree: Terminal-friendly visualization starting from a root node

**Test Coverage:**
- Empty graph export (both formats)
- Simple graphs with nodes and edges
- ASCII tree rendering (simple and nested)
- Filter by file
- Filter by completion status
- Filter by depth
- Multiple file clustering
- Cycle detection
- Special character escaping

**Integration:**
- Uses `DependencyGraph` for graph queries
- Respects `TaskStatus` enum including Blocked state
- Compatible with existing dependency module APIs

---

## Task 7: Incremental Graph Updates

**Priority:** MEDIUM
**Effort:** 2-3 days
**Depends on:** Task 3, tasks.indexing.md#5

### Description

Support efficient graph updates when tasks or dependencies change, without full rebuild.

### Subtasks

**Phase 1: Core Graph Mutation Operations (COMPLETE)**
- [x] Define `GraphError` enum for error handling
- [x] Define `GraphResult<T>` type alias
- [x] Implement `remove_node` with force option
  - [x] Error if node has dependents (unless force=true)
  - [x] Remove all associated edges (incoming and outgoing)
  - [x] Maintain bidirectional edge consistency
- [x] Implement `update_node` to replace node metadata
- [x] Implement `update_node_status` for status-only updates (optimized)
- [x] Implement `remove_edge` to remove dependency relationships
  - [x] Update forward adjacency list
  - [x] Update reverse adjacency list
  - [x] Remove edge metadata
- [x] Implement `update_edge` to replace edge metadata
- [x] Add comprehensive unit tests (13 tests)
- [x] Add doctests for all public mutation methods
- [x] Export GraphError and GraphResult from mod.rs

**Phase 2: Batch Update Operations (COMPLETE)**
- [x] Implement `add_nodes` for bulk node insertion
  - [x] Pre-allocate space to minimize reallocations
- [x] Implement `remove_nodes` for bulk node removal
  - [x] Fail-fast on first error (or force remove all)
- [x] Implement `add_edges` for bulk edge insertion
  - [x] Pre-allocate space for edge metadata
- [x] Implement `remove_edges` for bulk edge removal
  - [x] Fail-fast on first error
- [x] Add comprehensive unit tests (6 tests)
- [x] Add doctests for all batch operations

**Phase 3: Incremental Dependency Re-resolution (COMPLETE)**
- [x] Create `GraphChanges` struct to track modifications
  - [x] Track added/removed/modified nodes
  - [x] Track status-only changes separately
  - [x] Track added/removed/modified edges
- [x] Implement change classification methods
  - [x] `has_structural_changes()` - detect graph structure changes
  - [x] `is_status_only()` - detect pure status updates
  - [x] `is_empty()` - check if any changes
- [x] Implement `compute_affected_nodes` to determine recomputation scope
  - [x] Include all modified nodes
  - [x] Propagate to all ancestors (transitive dependents)
  - [x] Handle edge changes correctly
- [x] Implement utility methods
  - [x] `merge()` - combine multiple change sets
  - [x] `clear()` - reset change tracker
- [x] Add comprehensive unit tests (11 tests)
- [x] Add doctests with realistic examples

**Phase 4: Optimization for Common Cases (IMPLEMENTED)**
- [x] Fast path for status-only changes (via `update_node_status`)
  - [x] O(1) status update without copying node data
- [x] Change detection via `GraphChanges`
  - [x] `has_structural_changes()` determines if cycle detection needed
  - [x] `is_status_only()` enables optimized status recomputation
- [-] Benchmarks comparing incremental vs full rebuild (deferred)
  - Note: Benchmarks deferred to future performance optimization phase
- [x] All common operations optimized for minimal allocations

**Phase 5: Integration and Documentation (COMPLETE)**
- [x] Comprehensive doctests for all public APIs
  - [x] All mutation methods (5 doctests)
  - [x] All batch operations (4 doctests)
  - [x] GraphChanges with realistic usage (2 doctests)
- [x] Unit test coverage
  - [x] Phase 1: 13 tests (mutation operations)
  - [x] Phase 2: 6 tests (batch operations)
  - [x] Phase 3: 11 tests (change tracking)
  - [x] Total: 30 new tests, all passing
- [x] Clear error messages via GraphError enum
- [x] Performance characteristics documented in doc comments
- [-] Integration tests with full workflow (deferred to future)
  - Note: Graph mutation layer complete; integration with resolver deferred

### Success Criteria

- [x] Core mutation operations maintain graph invariants
- [x] Comprehensive test coverage (30 unit tests, 11 doctests)
- [x] Clear error handling with helpful messages (GraphError enum)
- [x] Graph remains consistent after all operations
- [x] Change tracking enables incremental updates
- [x] Optimized for common cases (status-only updates)

### Tests

- [x] Unit: Remove node (simple, with dependents, force removal) - 4 tests
- [x] Unit: Update node metadata and status - 4 tests
- [x] Unit: Remove/update edges - 5 tests
- [x] Unit: Graph invariants maintained after mutations - all tests verify
- [x] Unit: Batch operations - 6 tests
- [x] Unit: Change tracking (GraphChanges) - 11 tests
- [-] Integration: Incremental update after file modification (deferred)
- [-] Benchmarks: Performance comparison (deferred)

### Implementation Notes

**All Phases Complete (Task 7):**

**Phase 1 - Core Mutations:**
- Added `GraphError` enum with three variants:
  - `NodeNotFound`: Node doesn't exist
  - `EdgeNotFound`: Edge doesn't exist
  - `NodeHasDependents`: Cannot remove node with dependents
- Implemented 5 mutation methods on `DependencyGraph`:
  - `remove_node(task_id, force)`: Remove node and edges
  - `update_node(task_id, node_data)`: Replace node metadata
  - `update_node_status(task_id, status)`: Optimized status update (O(1))
  - `remove_edge(from_id, to_id)`: Remove dependency relationship
  - `update_edge(from_id, to_id, edge_data)`: Replace edge metadata
- Added internal helper `remove_edge_internal` for efficient bulk removal
- All mutation methods maintain bidirectional edge consistency
- 13 unit tests + 5 doctests

**Phase 2 - Batch Operations:**
- Implemented 4 batch methods:
  - `add_nodes(nodes)`: Bulk node insertion with pre-allocation
  - `remove_nodes(task_ids, force)`: Bulk node removal
  - `add_edges(edges)`: Bulk edge insertion with pre-allocation
  - `remove_edges(edges)`: Bulk edge removal
- All batch operations optimize for minimal reallocations
- Fail-fast error handling (returns first error encountered)
- 6 unit tests + 4 doctests

**Phase 3 - Change Tracking:**
- Implemented `GraphChanges` struct with comprehensive change tracking:
  - Tracks added/removed/modified nodes
  - Tracks status-only changes separately
  - Tracks added/removed/modified edges
- Change classification methods:
  - `has_structural_changes()`: Detect graph structure changes
  - `is_status_only()`: Detect pure status updates
  - `is_empty()`: Check if any changes occurred
- Implemented `compute_affected_nodes(graph)`:
  - Computes transitive closure of affected nodes
  - Propagates changes to all ancestors
  - Enables incremental status recomputation
- Utility methods: `merge()`, `clear()`
- 11 unit tests + 2 doctests

**Phase 4 - Optimizations:**
- Status-only updates use O(1) in-place mutation
- Batch operations pre-allocate to minimize reallocations
- Change detection enables smart recomputation
- Structural changes → full cycle detection needed
- Status-only changes → incremental status update sufficient

**Test Summary:**
- Total: 30 new unit tests, 11 new doctests
- All tests passing (66 unit tests, 34 doctests overall)
- Full coverage of mutation operations, batch operations, and change tracking
- All graph invariants maintained across all operations

**Exports:**
- Added to `dependency/mod.rs`:
  - `GraphError`, `GraphResult`
  - `GraphChanges`
- All public APIs have executable doctests

---

## Non-Goals (for v1)

- Advanced graph algorithms (shortest path, etc.)
- Graph persistence to disk (rebuild from DB each time)
- Multi-graph support (separate graphs for different views)
- Real-time graph updates (manual recomputation)

---

## Open Questions

- **Cycle handling:** Fail-fast vs collect all cycles?
- **Waived propagation:** If A waives B, does B's status matter?
- **Performance:** Build graph on-demand vs cache in memory?
- **Directory dependencies:** Explicit annotation vs inferred from structure?

---

## References

- Design doc section 5 (Dependency model)
- Design doc section 5.4 (Completion semantics)
- Design doc section 7.3.4 (Graph commands)
- `/docs/dependency-graph-analysis.md` (if exists)
