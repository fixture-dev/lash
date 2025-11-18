# Lash Dependency Graph Architecture Analysis

## Executive Summary

Lash requires a sophisticated dependency graph system that manages three distinct dependency types while maintaining high performance for both queries and updates. The system must handle hundreds to thousands of tasks efficiently, detect cycles reliably, and propagate completion status correctly across a DAG structure.

---

## 1. Graph Representation Strategy

### 1.1 Recommended Core Data Structure

**Primary Recommendation: Adjacency List with Dual Indexing**

```rust
struct DependencyGraph {
    // Forward edges: task -> dependencies it needs
    forward_edges: HashMap<TaskId, HashSet<TaskId>>,

    // Reverse edges: task -> tasks that depend on it
    reverse_edges: HashMap<TaskId, HashSet<TaskId>>,

    // Task metadata for quick access
    task_metadata: HashMap<TaskId, TaskMetadata>,

    // File-to-tasks mapping for directory dependencies
    file_tasks: HashMap<FilePath, Vec<TaskId>>,

    // Cached topological ordering (invalidated on mutation)
    topo_cache: Option<Vec<TaskId>>,
}
```

**Rationale:**
- **Adjacency lists** are optimal for sparse graphs (typical in task systems)
- **Dual indexing** (forward + reverse) enables O(1) lookups in both directions
- **Space complexity:** O(V + E) where V = tasks, E = dependencies
- **Cache-friendly** for traversal operations

### 1.2 Dependency Type Representation

```rust
enum DependencyKind {
    // Parent checkbox depends on child (implicit, hierarchical)
    WithinFileHierarchy {
        parent: TaskId,
        child: TaskId,
        depth_delta: u8  // For validation
    },

    // Explicit cross-file reference via @depends-on
    CrossFileExplicit {
        from: TaskId,
        to: TaskId,
        reference: String  // Original reference for diagnostics
    },

    // Directory-level: file depends on subdirectory completion
    DirectoryLevel {
        file: FilePath,
        subdirectory: FilePath
    },
}

struct DependencyEdge {
    kind: DependencyKind,
    is_waivable: bool,  // Can be marked as [-] to bypass
    created_at: Timestamp,
}
```

### 1.3 Alternative Considered: Adjacency Matrix

Not recommended for Lash because:
- **Space inefficient:** O(V²) storage for sparse graphs
- **Poor cache locality** for typical traversal patterns
- Only beneficial if graph density > 10% (unlikely for task graphs)

---

## 2. Key Algorithms Needed

### 2.1 Dependency Resolution (Topological Sort)

**Algorithm: Modified Kahn's Algorithm with Priority Support**

```rust
fn topological_sort_with_priority(&self) -> Result<Vec<TaskId>, CycleError> {
    // Use Kahn's algorithm (BFS-based) for:
    // - Better incremental update support
    // - Natural level-by-level processing
    // - Easy priority injection

    // Time: O(V + E)
    // Space: O(V)
}
```

**Why Kahn's over DFS:**
- Produces level-order traversal naturally
- Easier to add priority/weight considerations
- Better for incremental updates (can resume from partial state)

### 2.2 Cycle Detection

**Primary: Incremental DFS with Path Tracking**

```rust
fn detect_cycles_incremental(&self, new_edge: &Edge) -> Option<Vec<TaskId>> {
    // Three-color DFS starting from new_edge.to
    // Check if we can reach new_edge.from
    // Time: O(V + E) worst case, often O(k) for local changes

    enum Color { White, Gray, Black }
    // White: unvisited
    // Gray: currently in DFS stack (cycle if revisited)
    // Black: completely processed
}
```

**Secondary: Tarjan's SCC for Batch Processing**

```rust
fn find_all_cycles(&self) -> Vec<Vec<TaskId>> {
    // Tarjan's strongly connected components
    // Time: O(V + E), single pass
    // Used during full reindex or validation
}
```

### 2.3 Completion Status Propagation

**Algorithm: Reverse Topological Traversal with Memoization**

```rust
fn propagate_completion_status(&mut self) -> StatusUpdate {
    // Process in reverse topological order
    // For each task:
    //   1. Check all dependencies' status
    //   2. Apply completion rules:
    //      - All deps done/waived -> can be done
    //      - Any dep blocked -> blocked
    //      - Any dep open -> cannot complete
    //   3. Memoize result

    // Time: O(V + E)
    // Can be optimized to O(affected_subgraph) for local changes
}
```

### 2.4 Finding All Blockers

**Algorithm: BFS with Filtering**

```rust
fn find_blockers(&self, task: TaskId) -> HashSet<TaskId> {
    // BFS through forward_edges
    // Collect all reachable tasks with status != done/waived
    // Time: O(V + E) worst case, typically O(local_graph)
}
```

### 2.5 Impact Analysis

**Algorithm: Reverse BFS from Modified Node**

```rust
fn analyze_impact(&self, changed_task: TaskId) -> ImpactReport {
    // BFS through reverse_edges
    // Categorize affected tasks:
    //   - Direct dependents
    //   - Transitive dependents
    //   - Status changes required
    //   - Potential unblocking

    // Time: O(affected_subgraph)
}
```

---

## 3. SQLite Schema Recommendations

### 3.1 Core Tables

```sql
-- Primary dependency storage
CREATE TABLE dependencies (
    id INTEGER PRIMARY KEY,
    from_task_id INTEGER NOT NULL,
    to_task_id INTEGER NOT NULL,
    dependency_kind TEXT NOT NULL CHECK(dependency_kind IN
        ('hierarchy', 'explicit', 'directory')),
    is_waivable BOOLEAN DEFAULT 1,
    reference TEXT,  -- Original @depends-on string
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (from_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (to_task_id) REFERENCES tasks(id) ON DELETE CASCADE,

    -- Prevent duplicate edges
    UNIQUE(from_task_id, to_task_id, dependency_kind)
);

-- Denormalized view for fast queries
CREATE TABLE dependency_closure (
    ancestor_id INTEGER NOT NULL,
    descendant_id INTEGER NOT NULL,
    distance INTEGER NOT NULL,  -- Number of edges in path
    path_count INTEGER DEFAULT 1,  -- Number of distinct paths

    PRIMARY KEY (ancestor_id, descendant_id),
    FOREIGN KEY (ancestor_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (descendant_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Cache for expensive computations
CREATE TABLE graph_cache (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    computed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP
);
```

### 3.2 Critical Indexes

```sql
-- For forward traversal (what does X depend on?)
CREATE INDEX idx_deps_from ON dependencies(from_task_id, dependency_kind);

-- For reverse traversal (what depends on X?)
CREATE INDEX idx_deps_to ON dependencies(to_task_id, dependency_kind);

-- For status propagation queries
CREATE INDEX idx_tasks_status ON tasks(status, file_id);

-- For finding directory dependencies efficiently
CREATE INDEX idx_deps_dir ON dependencies(dependency_kind, reference)
    WHERE dependency_kind = 'directory';

-- For cycle detection (find paths back to origin)
CREATE INDEX idx_closure_cycle ON dependency_closure(descendant_id, ancestor_id);

-- Composite for common query pattern
CREATE INDEX idx_deps_composite ON dependencies(
    from_task_id, to_task_id, dependency_kind, is_waivable
);
```

### 3.3 Special Considerations

**Transitive Closure Maintenance:**
- Maintain `dependency_closure` table for O(1) reachability queries
- Update incrementally using triggers or batch updates
- Trade-off: 2-3x storage for 10-100x query speedup

**Graph Versioning:**
```sql
CREATE TABLE graph_versions (
    version_id INTEGER PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    nodes_hash TEXT NOT NULL,  -- Hash of all nodes
    edges_hash TEXT NOT NULL   -- Hash of all edges
);
```

---

## 4. Performance Considerations

### 4.1 Expected Scale Analysis

**Assumptions:**
- Tasks per project: 500-5,000 (median ~1,000)
- Average dependencies per task: 2-5
- Files in project: 20-200
- Depth of task hierarchies: 3-4 levels max

**Memory Footprint Estimates:**
```
Tasks: 1,000 × 200 bytes = 200 KB
Edges: 3,000 × 50 bytes = 150 KB
Indexes: ~500 KB
Total in-memory: < 1 MB for typical project
```

### 4.2 Query Optimization Priorities

1. **Hot Path: Status Checks** (called constantly)
   - Cache task status in memory
   - Invalidate only affected subgraph on change
   - Target: < 1ms

2. **Frequent: Find Blockers** (UI/CLI operations)
   - Use closure table for O(1) reachability
   - Limit depth for interactive queries
   - Target: < 10ms

3. **Common: List by Label/Status**
   - Compound indexes on (label, status)
   - Materialized view for complex filters
   - Target: < 20ms for 1000 tasks

4. **Occasional: Full Topological Sort**
   - Cache result until graph mutation
   - Background recompute after changes
   - Target: < 100ms for 1000 tasks

### 4.3 Incremental Update Strategies

**Change Propagation Algorithm:**
```rust
struct IncrementalUpdater {
    dirty_set: HashSet<TaskId>,
    affected_closure: HashSet<TaskId>,

    fn on_task_status_change(&mut self, task: TaskId) {
        // 1. Mark task dirty
        // 2. Find reverse dependencies (who depends on this?)
        // 3. Add to affected_closure
        // 4. Process in topological order
        // 5. Stop propagation when no status change
    }
}
```

**Batch Optimization:**
- Accumulate changes for 100ms before propagating
- Process multiple changes in single topological pass
- Maintain generation counter to detect stale caches

---

## 5. Implementation Complexity Assessment

### 5.1 Straightforward Components (Low Risk)

1. **Basic graph structure** (adjacency lists)
   - Well-understood, many examples
   - 2-3 days implementation

2. **Simple cycle detection** (DFS)
   - Standard algorithm
   - 1 day implementation

3. **SQLite schema and basic queries**
   - Standard relational patterns
   - 2-3 days implementation

### 5.2 Moderate Complexity (Medium Risk)

1. **Hierarchical dependency management**
   - Need to maintain parent-child invariants
   - Handle depth limits correctly
   - 3-4 days implementation + testing

2. **Status propagation with waiving**
   - Complex business rules
   - Edge cases around partial completion
   - 3-4 days implementation + testing

3. **Incremental updates**
   - Cache invalidation logic
   - Performance tuning required
   - 4-5 days implementation

### 5.3 Challenging Components (High Risk)

1. **Transitive closure maintenance**
   - Complex to implement correctly
   - Performance-critical
   - Consider using library or simplified approach
   - 5-7 days if custom implementation

2. **Directory-level dependencies**
   - Interaction with filesystem
   - Handle moves/renames
   - Recursive dependency resolution
   - 4-5 days implementation

3. **Concurrent access patterns**
   - Reader-writer locks on graph
   - Atomic status updates
   - Transaction boundaries in SQLite
   - 3-4 days implementation + extensive testing

### 5.4 Edge Cases & Tricky Scenarios

1. **Self-loops**: Task depending on itself (should error)
2. **Diamond dependencies**: A→B, A→C, B→D, C→D
3. **Orphaned tasks**: No incoming or outgoing edges
4. **Mass waiving**: Waiving parent with many children
5. **File deletion**: Removing file with active dependencies
6. **Broken references**: Dependencies pointing to non-existent tasks
7. **Depth limit violations**: After edits
8. **Race conditions**: Two agents marking related tasks simultaneously

---

## 6. Phased Implementation Recommendation

### Phase 1: MVP (Week 1-2)
```
✓ Basic adjacency list structure
✓ Within-file hierarchy dependencies only
✓ Simple DFS cycle detection
✓ Basic SQLite schema (tasks + dependencies tables)
✓ Manual status propagation (no optimization)
```

### Phase 2: Core Features (Week 3-4)
```
✓ Cross-file explicit dependencies
✓ Topological sort implementation
✓ Blocker detection
✓ Status propagation with waiving
✓ Basic incremental updates
✓ Comprehensive error handling
```

### Phase 3: Performance (Week 5)
```
✓ Transitive closure table
✓ Query optimization with indexes
✓ Caching layer
✓ Batch update processing
✓ Background reindexing
```

### Phase 4: Advanced (Week 6+)
```
✓ Directory-level dependencies
✓ Concurrent access handling
✓ Graph visualization export
✓ Dependency conflict resolution
✓ Historical tracking
```

---

## 7. Library and Tool Recommendations

### Rust Libraries

1. **petgraph** (Primary recommendation)
   - Mature, well-tested graph library
   - Includes Tarjan's SCC, topological sort, Dijkstra
   - Good performance, extensive API
   - License: MIT/Apache 2.0

2. **graphlib** (Alternative)
   - Lighter weight, simpler API
   - Good for basic operations
   - Less feature-rich than petgraph

3. **daggy** (For DAG-specific operations)
   - Built on petgraph
   - Enforces DAG constraints
   - Good if we guarantee acyclic

### SQLite Extensions

1. **SQLite Recursive CTEs**
   - Built-in support for graph queries
   - Good for reachability queries
   - Example:
   ```sql
   WITH RECURSIVE reach(id) AS (
     SELECT to_task_id FROM dependencies WHERE from_task_id = ?
     UNION
     SELECT d.to_task_id FROM dependencies d
     JOIN reach r ON d.from_task_id = r.id
   )
   SELECT * FROM reach;
   ```

2. **rusqlite** with **r2d2** connection pooling
   - Handle concurrent access efficiently
   - Prepared statement caching

### Testing Tools

1. **quickcheck** or **proptest**
   - Property-based testing for graph invariants
   - Generate random DAGs for stress testing

2. **criterion**
   - Benchmark critical paths
   - Track performance regressions

---

## 8. Critical Success Factors

1. **Maintain DAG invariant**: Never allow cycles to persist
2. **Atomic updates**: Graph mutations must be transactional
3. **Cache aggressively**: But invalidate correctly
4. **Fail fast**: Detect issues early, provide clear errors
5. **Observability**: Log graph operations for debugging
6. **Testing**: Extensive edge case coverage, property-based tests
7. **Documentation**: Clear invariants and operation semantics

---

## Conclusion

The Lash dependency graph system requires careful balance between functionality, performance, and complexity. The recommended approach using adjacency lists with dual indexing, backed by SQLite with strategic denormalization, provides the best trade-offs for the expected use cases.

The phased implementation plan allows for early validation of core concepts while deferring complex optimizations until the basic system proves stable. Using established libraries like `petgraph` reduces implementation risk while maintaining flexibility for Lash-specific requirements.

Key technical decisions:
- **Adjacency list** over matrix (sparse graph assumption)
- **Kahn's algorithm** for topological sort (better incremental behavior)
- **Transitive closure table** in SQL (query performance over storage)
- **Three-tier dependency model** (hierarchy, explicit, directory)
- **Incremental propagation** with dirty tracking (responsiveness)

Total estimated implementation time: 4-6 weeks for full system, 2 weeks for MVP.