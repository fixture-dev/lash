# Dependency Graph Architecture - Executive Summary

**Full Design:** See `docs/dependency-graph-architecture.md`

---

## Key Architectural Decisions

### 1. Data Structure: Adjacency List with Bidirectional Edges

**Why:**
- O(1) node lookups via HashMap
- O(1) direct dependency/dependent queries
- Memory efficient for sparse graphs (typical: 3 edges/node)
- Supports all required operations efficiently

**Memory:** ~400-500 KB for 1000 tasks, scales linearly

### 2. Core Types

```rust
// Node representation
pub struct GraphNode {
    full_id: String,        // "file_id#task_id"
    db_id: i64,             // Link back to database
    status: TaskStatus,
    // ... metadata
}

// Edge representation
pub struct GraphEdge {
    from: NodeId,           // Source (dependent)
    to: NodeId,             // Target (dependency)
    kind: DependencyKind,   // Hierarchy/Explicit/Directory
}

// Main graph
pub struct DependencyGraph {
    nodes: HashMap<NodeId, GraphNode>,
    forward_edges: HashMap<NodeId, Vec<GraphEdge>>,   // task → dependencies
    reverse_edges: HashMap<NodeId, Vec<GraphEdge>>,   // task → dependents
    file_index: HashMap<String, Vec<NodeId>>,
    status_cache: HashMap<NodeId, EffectiveStatus>,
}
```

### 3. Algorithm Choices

| Operation | Algorithm | Complexity |
|-----------|-----------|------------|
| Cycle Detection | DFS with 3-color marking | O(V + E) |
| Transitive Dependencies | DFS with visited set | O(V + E) |
| Status Computation | Topological traversal + memoization | O(V + E) |
| Blocker Analysis | Recursive DFS with path tracking | O(V + E) |

### 4. Integration Strategy

**DB Layer (lash-db):**
- Persistence and source of truth
- Single-edge cycle check (`would_create_cycle`)
- Transitive closure table for fast queries
- Dependency resolution

**In-Memory Graph (lash-core):**
- Built from DB on demand
- Rich algorithms (comprehensive cycles, status, blockers)
- Export capabilities (DOT, JSON, ASCII)
- Incremental updates

**Flow:** DB → GraphBuilder → DependencyGraph → Analysis

---

## API Surface

### Construction

```rust
// Build from database
let conn = open_database(&path)?;
let graph = GraphBuilder::new(&conn).build()?;
```

### Queries

```rust
// Direct relationships
graph.get_dependencies(task_id)  // O(1)
graph.get_dependents(task_id)    // O(1)

// Transitive relationships
graph.get_descendants(task_id)   // O(V + E)
graph.get_ancestors(task_id)     // O(V + E)

// Hierarchy
graph.get_children(task_id)      // O(n) where n = tasks in file
```

### Analysis

```rust
// Cycle detection
let cycles = graph.detect_cycles()?;
for cycle in cycles {
    println!("{}", cycle.format(&graph));
}

// Status computation
graph.compute_all_statuses()?;
let status = graph.get_status(task_id)?;

// Blocker analysis
let report = graph.analyze_blockers(task_id)?;
println!("{}", report.format(&graph));
```

### Updates

```rust
let mut updater = GraphUpdate::new(graph);
updater.update_status(task_id, TaskStatus::Done)?;
updater.add_dependency(from, to, DependencyKind::ExplicitId)?;
let graph = updater.finish();
```

### Export

```rust
let mut output = Vec::new();
graph.export(&mut output, ExportFormat::Dot, &options)?;
```

---

## Performance Characteristics

### Time Complexity

| Operation | Best | Average | Worst |
|-----------|------|---------|-------|
| Build graph | - | O(V + E) | O(V + E) |
| Node lookup | O(1) | O(1) | O(1) |
| Direct deps/dependents | O(1) | O(1) | O(k) |
| Transitive queries | - | O(V + E) | O(V + E) |
| Cycle detection | - | O(V + E) | O(V + E) |
| Status computation (all) | - | O(V + E) | O(V + E) |

### Space Complexity

- Nodes: O(V)
- Edges: O(E) × 2 (forward + reverse)
- Indexes: O(V)
- Cache: O(V)
- **Total: O(V + E)** typically ~4-5 MB for 10,000 tasks

---

## Implementation Plan

### Phase 1: Core Data Structures (2-3 days)
- GraphNode, GraphEdge, DependencyGraph types
- Builder pattern for construction from DB
- Query operations (dependencies, descendants, etc.)

### Phase 2: Cycle Detection (2-3 days)
- DFS-based cycle detector
- Path tracking and reporting
- Resolution suggestions

### Phase 3: Status Computation (2-3 days)
- EffectiveStatus enum
- Topological traversal with memoization
- Cache invalidation on updates

### Phase 4: Blocker Analysis (2 days)
- Direct and transitive blocker identification
- Root blocker detection
- Report formatting

### Phase 5: Graph Export (1-2 days)
- DOT format (Graphviz)
- JSON format
- ASCII tree for terminal

### Phase 6: Incremental Updates (2-3 days)
- GraphUpdate API
- Add/remove nodes and edges
- Status updates with cache invalidation

**Total: 10-16 days**

---

## Key Design Principles

1. **Memory Efficiency**: Optimized for 1000+ tasks, scales to 10,000+
2. **Query Performance**: O(1) direct lookups, O(V+E) transitive
3. **Type Safety**: Leverages Rust's type system for correctness
4. **Integration**: Seamless with existing DB layer and type system
5. **Extensibility**: Supports incremental updates and future enhancements

---

## Concerns & Mitigations

### Concern 1: Memory Usage for Large Graphs
**Impact:** 10,000 tasks = ~4-5 MB
**Mitigation:** Acceptable for CLI tool; can optimize if needed (integer IDs, edge compression)

### Concern 2: Rebuild Cost on File Changes
**Impact:** O(V + E) to rebuild entire graph
**Mitigation:** Fast enough for typical project sizes (<1s for 1000s of tasks); incremental updates for status-only changes

### Concern 3: Cycle Detection Completeness
**Impact:** Must find ALL cycles, not just one
**Mitigation:** DFS naturally finds all cycles; tested against multiple cycle patterns

### Concern 4: Status Cache Invalidation
**Impact:** Must invalidate all ancestors when node status changes
**Mitigation:** Reverse edges enable efficient ancestor traversal; complexity is O(A) where A = ancestor count

---

## Testing Strategy

### Unit Tests (Per Module)
- `types_test.rs`: Node/edge creation and validation
- `queries_test.rs`: All query operations
- `cycles_test.rs`: Various cycle patterns (simple, complex, disjoint, self-loop)
- `status_test.rs`: Status rules and memoization
- `blockers_test.rs`: Blocker identification
- `update_test.rs`: Incremental updates
- `export_test.rs`: Output validation

### Integration Tests
- Build from fixture database
- End-to-end cycle detection
- Status computation on real data
- Export and verify output

### Benchmarks
- Construction time vs graph size
- Query performance
- Cycle detection on various topologies
- Incremental update cost

---

## Next Steps

1. **Review this design** with the team
2. **Create feature branch** for graph implementation
3. **Implement Phase 1** (core data structures)
4. **Test thoroughly** at each phase
5. **Document APIs** with rustdoc examples
6. **Benchmark** at 1k, 5k, 10k tasks
7. **Integrate with CLI** commands (lash graph, lash check-links)

---

## Open Questions

1. **Incremental vs full rebuild:** When to rebuild entire graph?
   - Proposal: Rebuild on file changes, incremental on status changes

2. **Persist transitive closure in DB?**
   - Current: DB has closure table, graph computes on-demand
   - Recommendation: Keep both (DB for persistence, graph for analysis)

3. **Parallel algorithms:** Worth implementing for large graphs?
   - Recommendation: Only if graphs exceed 10,000 tasks

4. **Internal node ID optimization:** String vs numeric?
   - Recommendation: Start with strings, optimize later if needed

---

## Reference Files

- **Full Design:** `docs/dependency-graph-architecture.md`
- **Task Requirements:** `tasks/tasks.dependency-resolution.md`
- **Existing Types:** `crates/lash-types/src/dependency.rs`
- **DB Layer:** `crates/lash-db/src/repository/dependencies.rs`
- **Design Doc:** `docs/design-doc.md` (Section 5)
