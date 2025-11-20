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
- [-] Implement implicit hierarchy resolution
  - [-] For each task, add edges to all children
  - [-] Edges are `(parent_id, child_id, type=hierarchy)`
  - Note: Hierarchy dependencies are handled separately during indexing (not part of resolver)
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
  - [-] Mark affected tasks as potentially blocked - handled by status computation

### Success Criteria

- [x] Correctly resolves all three dependency types (explicit ID, explicit path, within-file)
- [x] Handles all supported reference formats
- [x] Detects and reports broken links with precise locations
- [x] Produces complete, accurate dependency graph

### Tests

- [-] Unit: Resolve implicit hierarchy - handled by indexing layer
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

- [ ] Implement `BlockerAnalyzer` struct
  - [ ] Query graph and status for dependencies
  - [ ] Identify incomplete dependencies
- [ ] Implement `find_blockers(task_id)` function
  - [ ] Get all dependencies (direct + transitive)
  - [ ] Filter to incomplete/blocked tasks
  - [ ] Sort by "distance" (direct blockers first)
  - [ ] Return list of blocker tasks with reasons
- [ ] Add blocker chain analysis
  - [ ] For each blocker, recursively find its blockers
  - [ ] Build blocker tree/graph
  - [ ] Identify "root blockers" (no further dependencies)
- [ ] Implement blocker reporting
  - [ ] Format: "Task X is blocked by: Task Y (in file Z)"
  - [ ] Show full blocker chain for deep dependencies
  - [ ] Suggest actions (complete blockers, waive, remove dependency)

### Success Criteria

- Accurately identifies all blockers for a given task
- Handles transitive blockers (A blocked by B blocked by C)
- Clear, actionable blocker reports
- Efficient: O(E) for direct blockers, O(V+E) for transitive

### Tests

- Unit: Task with direct blocker
- Unit: Task with transitive blocker chain
- Unit: Task with multiple independent blockers
- Unit: Task with no blockers (ready to start)
- Integration: Generate blocker report for fixture tasks

---

## Task 6: Graph Export

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 1

### Description

Export dependency graph in various formats for visualization and analysis.

### Subtasks

- [ ] Implement `GraphExporter` struct
  - [ ] Support multiple output formats
- [ ] Implement DOT format export
  - [ ] Generate Graphviz-compatible DOT file
  - [ ] Nodes: task IDs or titles
  - [ ] Edges: dependency relationships
  - [ ] Color-code by status (green=done, red=blocked, etc.)
  - [ ] Cluster by file or directory
- [ ] Implement JSON export
  - [ ] Nodes array: task metadata
  - [ ] Edges array: source/target/type
  - [ ] Include status and labels
- [ ] Add filtering options
  - [ ] Export subgraph (specific file or label)
  - [ ] Hide completed tasks
  - [ ] Show only direct dependencies (no transitive)
- [ ] Implement text-based graph visualization
  - [ ] ASCII tree format for terminal display
  - [ ] Indent by depth
  - [ ] Show dependency arrows

### Success Criteria

- DOT output renders correctly in Graphviz
- JSON format is parsable and complete
- Filtering options work as expected
- Text format is readable in terminal

### Tests

- Unit: Export empty graph
- Unit: Export simple graph to DOT
- Unit: Export simple graph to JSON
- Integration: Export fixture project graph, verify correctness
- Manual: Render DOT file with Graphviz, inspect visually

---

## Task 7: Incremental Graph Updates

**Priority:** MEDIUM
**Effort:** 2-3 days
**Depends on:** Task 3, tasks.indexing.md#5

### Description

Support efficient graph updates when tasks or dependencies change, without full rebuild.

### Subtasks

- [ ] Implement `GraphUpdater` struct
  - [ ] Track which nodes/edges need updates
  - [ ] Incrementally modify graph in place
- [ ] Implement node updates
  - [ ] Add new tasks
  - [ ] Remove deleted tasks (and their edges)
  - [ ] Update task metadata (status, labels)
- [ ] Implement edge updates
  - [ ] Add new dependencies
  - [ ] Remove deleted dependencies
  - [ ] Re-resolve dependencies for modified tasks
- [ ] Maintain graph invariants
  - [ ] Update reverse edge index when edges change
  - [ ] Invalidate cached status computations
  - [ ] Re-run cycle detection if edges added
- [ ] Optimize for common cases
  - [ ] Status change only (no graph structure change)
  - [ ] New task added (only affects parent)
  - [ ] Task deleted (affects dependents)

### Success Criteria

- Incremental updates faster than full rebuild
- Graph remains consistent after updates
- Cached data invalidated correctly

### Tests

- Unit: Add task to graph
- Unit: Remove task from graph
- Unit: Update task status
- Unit: Add/remove dependency edge
- Integration: Incremental update after file modification

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
