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

- [ ] Define `DependencyGraph` struct
  - [ ] Node storage (task IDs -> Task references)
  - [ ] Edge storage (adjacency list: task_id -> Vec<task_id>)
  - [ ] Reverse edge storage (for efficient reverse lookups)
  - [ ] Edge metadata (dependency type: hierarchy, explicit, directory)
- [ ] Implement graph construction from DB
  - [ ] Load all tasks and dependencies from SQLite
  - [ ] Build adjacency lists
  - [ ] Index tasks by full_id for fast lookup
- [ ] Implement graph query methods
  - [ ] `get_dependencies(task_id)` - direct dependencies
  - [ ] `get_dependents(task_id)` - reverse dependencies
  - [ ] `get_descendants(task_id)` - transitive closure (DFS/BFS)
  - [ ] `get_ancestors(task_id)` - reverse transitive closure
- [ ] Add edge type tracking
  - [ ] Distinguish hierarchy vs explicit vs directory edges
  - [ ] Store source location for explicit edges (for error reporting)

### Success Criteria

- Graph correctly represents all dependency relationships
- Efficient lookups: O(1) for direct dependencies, O(E+V) for transitive
- Memory-efficient for large graphs (1000+ tasks)
- Clear API for downstream consumers

### Tests

- Unit: Build graph from fixture data
- Unit: Test query methods on various graph structures
- Unit: Test edge type tracking
- Performance: Measure memory usage for large graphs

---

## Task 2: Cycle Detection

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** Task 1

### Description

Detect circular dependencies in the task graph using standard graph algorithms.

### Subtasks

- [ ] Implement `CycleDetector` struct
  - [ ] Use DFS with color marking (white/gray/black)
  - [ ] Track path during traversal for cycle reporting
- [ ] Implement `detect_cycles()` function
  - [ ] Run DFS from all unvisited nodes
  - [ ] Detect back edges (gray -> gray)
  - [ ] Collect all cycles (not just first)
  - [ ] Return cycle paths with task IDs
- [ ] Add cycle reporting
  - [ ] Format cycle as: Task A -> Task B -> Task C -> Task A
  - [ ] Include file paths and line numbers
  - [ ] Distinguish cycle types (within-file vs cross-file)
- [ ] Implement cycle resolution suggestions
  - [ ] Identify weakest link (e.g., directory dep vs explicit)
  - [ ] Suggest breaking explicit dependencies
  - [ ] Suggest restructuring hierarchy

### Success Criteria

- Detects all cycles in arbitrary graphs
- Correctly handles graphs with multiple disjoint cycles
- Clear, actionable error messages for each cycle
- No false positives or false negatives

### Tests

- Unit: Acyclic graph (no cycles)
- Unit: Simple cycle (A -> B -> A)
- Unit: Complex cycle (A -> B -> C -> D -> B)
- Unit: Multiple disjoint cycles
- Unit: Self-loop (A -> A)
- Integration: Test with fixture files containing cycles

---

## Task 3: Dependency Resolution Engine

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 1, tasks.markdown-parser.md#2

### Description

Parse dependency references from tasks and build the complete dependency graph.

### Subtasks

- [ ] Implement `DependencyResolver` struct
  - [ ] Hold references to parsed tasks and files
  - [ ] Track resolution errors (broken links)
- [ ] Implement implicit hierarchy resolution
  - [ ] For each task, add edges to all children
  - [ ] Edges are `(parent_id, child_id, type=hierarchy)`
- [ ] Implement explicit `@depends-on` resolution
  - [ ] Parse `@depends-on` annotations
  - [ ] Support formats:
    - [ ] Relative path: `../core/cli.md#task:parse-args`
    - [ ] Absolute path: `core/cli.md#task:parse-args`
    - [ ] ID-only: `#task:parse-args` (within-file)
    - [ ] File-level: `../core/cli.md` (depends on all tasks in file)
  - [ ] Resolve paths relative to source file
  - [ ] Look up target task in DB/graph
  - [ ] Create edge `(source_id, target_id, type=explicit)`
  - [ ] Handle missing targets (broken links)
- [ ] Implement directory-level dependencies
  - [ ] Parse directory metadata (if any)
  - [ ] Infer dependencies from directory structure
  - [ ] Add edges for directory relationships
- [ ] Handle broken dependencies
  - [ ] Collect all broken references
  - [ ] Return structured errors with source location
  - [ ] Mark affected tasks as potentially blocked

### Success Criteria

- Correctly resolves all three dependency types
- Handles all supported reference formats
- Detects and reports broken links with precise locations
- Produces complete, accurate dependency graph

### Tests

- Unit: Resolve implicit hierarchy
- Unit: Resolve explicit cross-file dependencies
- Unit: Resolve various path formats
- Unit: Handle broken links gracefully
- Integration: Build graph from fixture project
- Integration: Verify graph structure matches expectations

---

## Task 4: Completion Status Computation

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 1, Task 3

### Description

Compute the effective completion status of each task based on its own status and its dependencies.

### Subtasks

- [ ] Implement `StatusComputer` struct
  - [ ] Take graph and task statuses as input
  - [ ] Compute effective status for each task
- [ ] Implement completion rules (from design doc section 5.4)
  - [ ] Task is complete if:
    - [ ] Own status is `done`, AND
    - [ ] All children are `done` or `waived`, AND
    - [ ] All explicit dependencies are complete or waived
  - [ ] Task is blocked if:
    - [ ] Any dependency is `open` or `blocked` (not waived), OR
    - [ ] Depends on broken link
- [ ] Implement `compute_status()` function
  - [ ] Topological traversal (or recursive with memoization)
  - [ ] Cache computed statuses to avoid recomputation
  - [ ] Handle waived dependencies (ignore in completion check)
- [ ] Add file-level completion status
  - [ ] File complete if all top-level tasks complete
  - [ ] Consider directory-level dependencies
- [ ] Detect inconsistencies
  - [ ] Parent marked done but children open (lint warning)
  - [ ] Task marked done but dependencies open

### Success Criteria

- Correctly computes status for all tasks in graph
- Respects waived tasks (treats as complete)
- Identifies blocked tasks accurately
- Efficient: O(V+E) traversal, with memoization

### Tests

- Unit: Simple chain (A -> B -> C), various states
- Unit: Waived dependencies ignored
- Unit: Blocked propagation (A blocks B, B blocks C)
- Unit: Parent/child status consistency
- Integration: Compute status for entire fixture project

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
