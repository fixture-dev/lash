# Lash Performance Benchmarks

This document describes Lash's performance benchmarking suite, how to run benchmarks, interpret results, and performance targets for all major operations.

## Overview

Lash uses the [criterion](https://github.com/bheisler/criterion.rs) benchmarking framework to measure and track performance across all core operations. The benchmark suite is designed to:

1. **Verify performance targets** - Ensure Lash meets speed requirements
2. **Detect regressions** - Catch performance degradations before they reach production
3. **Guide optimization** - Identify bottlenecks and measure improvement impact
4. **Provide transparency** - Document expected performance characteristics

## Running Benchmarks

### Run All Benchmarks

```bash
cargo bench --workspace
```

This runs all benchmarks across all crates and generates HTML reports in `target/criterion/`.

### Run Specific Benchmark Suites

Run benchmarks for a specific crate:

```bash
# Parser and linter benchmarks (lash-core)
cargo bench --package lash-core

# Indexing and query benchmarks (lash-db)
cargo bench --package lash-db
```

Run a specific benchmark file:

```bash
# Just parser benchmarks
cargo bench --package lash-core --bench parser_bench

# Just indexing benchmarks
cargo bench --package lash-db --bench indexing

# Just search benchmarks
cargo bench --package lash-db --bench search_bench
```

Run a specific benchmark function:

```bash
# Benchmark only simple file parsing
cargo bench --package lash-core --bench parser_bench -- simple_files

# Benchmark only full indexing (not incremental)
cargo bench --package lash-db --bench indexing -- full_indexing
```

### Benchmark Options

Control benchmark execution:

```bash
# Quick run (fewer samples, faster but less precise)
cargo bench --package lash-core -- --quick

# Baseline comparison (save baseline for future comparison)
cargo bench --package lash-core -- --save-baseline main

# Compare against baseline
cargo bench --package lash-core -- --baseline main

# Verbose output
cargo bench --package lash-core -- --verbose
```

### View HTML Reports

After running benchmarks, open the HTML reports:

```bash
# macOS
open target/criterion/report/index.html

# Linux
xdg-open target/criterion/report/index.html

# Windows
start target/criterion/report/index.html
```

The reports include:
- Performance graphs over time
- Statistical analysis (mean, median, std deviation)
- Throughput measurements
- Regression detection

## Benchmark Organization

### lash-core Benchmarks

Located in `crates/lash-core/benches/`:

1. **parser_bench.rs** - Markdown parsing performance (simple/nested/complex files)
2. **linter_bench.rs** - Linting rule execution performance (parse + lint workflow)
3. **graph_bench.rs** - Dependency graph data structure operations (direct/transitive queries)

### lash-db Benchmarks

Located in `crates/lash-db/benches/`:

1. **indexing.rs** - Full and incremental indexing performance
2. **search_bench.rs** - Full-text search query performance

## Performance Targets

### Parsing (parser_bench.rs)

| Operation | Size | Target | Throughput Target |
|-----------|------|--------|-------------------|
| Parse simple file | 10 tasks | <1ms | >10,000 tasks/sec |
| Parse simple file | 100 tasks | <5ms | >20,000 tasks/sec |
| Parse simple file | 1000 tasks | <50ms | >20,000 tasks/sec |
| Parse nested file | 30 tasks (10 parents) | <2ms | >15,000 tasks/sec |
| Parse complex file | 100 tasks | <10ms | >10,000 tasks/sec |
| Parse realistic file | ~30 tasks | <5ms | >6,000 tasks/sec |

**Overall Target:** >1000 tasks/sec for typical workloads

### Linting (linter_bench.rs)

| Operation | Size | Target | Throughput Target |
|-----------|------|--------|-------------------|
| Lint valid file | 10 tasks | <1ms | >10,000 tasks/sec |
| Lint valid file | 100 tasks | <5ms | >20,000 tasks/sec |
| Lint valid file | 1000 tasks | <50ms | >20,000 tasks/sec |
| Lint with errors | 100 tasks | <10ms | >10,000 tasks/sec |
| Lint realistic file | ~30 tasks | <5ms | >6,000 tasks/sec |

**Overall Target:** >500 tasks/sec for typical workloads with errors

### Indexing (indexing.rs)

| Operation | Project Size | Target | Notes |
|-----------|-------------|--------|-------|
| Full index | 10 files, 50 tasks | <100ms | Small project |
| Full index | 100 files, 500 tasks | <1s | Medium project |
| Full index | 1000 files, 5000 tasks | <5s | Large project |
| Incremental (no changes) | 100 files | <50ms | Cache hit scenario |
| Incremental (10% modified) | 100 files | <200ms | ~10 files re-indexed |
| Incremental (10% churn) | 100 files | <300ms | 10 deleted + 10 added |

**Overall Targets:**
- Full index: <5s for 1000 files
- Incremental index: <1s for 100 changed files

### Search (search_bench.rs)

| Operation | Database Size | Target | Notes |
|-----------|--------------|--------|-------|
| Single word query | 100 tasks | <50ms | Common case |
| Single word query | 1000 tasks | <100ms | Medium dataset |
| Single word query | 10000 tasks | <200ms | Large dataset |
| Multi-word query | 1000 tasks | <150ms | More complex |
| With filters (label) | 1000 tasks | <100ms | Label filter |
| With filters (status) | 1000 tasks | <100ms | Status filter |
| Complex query | 1000 tasks | <200ms | Multiple filters |

**Overall Target:** <200ms for typical search queries

### Query Operations

Note: Query benchmarks are covered by the indexing and search benchmarks which include query operations as part of the overall workflow. Database query performance is tested indirectly through the indexing benchmark's verification phase and through the search benchmark's filter operations.

### Dependency Graph (graph_bench.rs)

| Operation | Graph Size | Target | Notes |
|-----------|-----------|--------|-------|
| Direct queries | Any size | <1μs | O(1) lookups |
| Transitive queries | 100 nodes | <100μs | O(E+V) traversal |
| Depth-limited queries | 100 nodes, depth=10 | <50μs | Bounded traversal |
| Graph construction | 500 nodes | <10ms | Node + edge creation |

**Overall Target:** O(1) for direct queries, O(E+V) for graph traversals


## Interpreting Results

### Understanding Criterion Output

When you run benchmarks, criterion displays:

```
simple_files/tasks/10  time:   [145.23 µs 147.89 µs 150.82 µs]
                       thrpt:  [6.6K elem/s 6.8K elem/s 6.9K elem/s]
```

- **time**: [lower bound, mean, upper bound] with 95% confidence interval
- **thrpt**: Throughput (elements/sec) if `Throughput` was set
- **change**: Shows percentage change vs. previous run (if available)

### Performance Regression Detection

Criterion automatically detects regressions:

```
Performance has regressed.
  time:   [145.23 µs 147.89 µs 150.82 µs]
  change: [+15.234% +17.891% +20.582%] (p = 0.00 < 0.05)
```

A regression is flagged when:
- Performance degrades by >5% (configurable)
- Statistical significance p < 0.05

### HTML Reports

The HTML reports provide:
- **Line charts** showing performance over time
- **Violin plots** showing distribution of measurements
- **Change indicators** highlighting regressions/improvements
- **Detailed statistics** (mean, median, std dev, outliers)

Navigate reports by clicking benchmark groups in the left sidebar.

### Flamegraphs (Advanced)

For detailed profiling, use flamegraphs:

```bash
# Install flamegraph support
cargo install flamegraph

# Run specific benchmark with flamegraph
cargo bench --package lash-core --bench parser_bench -- simple_files --profile-time=5

# View flamegraph
open target/criterion/simple_files/profile/flamegraph.svg
```

## Adding New Benchmarks

### 1. Create Benchmark File

Add a new file in the appropriate `benches/` directory:

```rust
// crates/lash-core/benches/my_new_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_my_function(c: &mut Criterion) {
    c.bench_function("my_function", |b| {
        b.iter(|| {
            // Your code to benchmark
            black_box(my_function(black_box(&input)))
        });
    });
}

criterion_group!(benches, bench_my_function);
criterion_main!(benches);
```

### 2. Register in Cargo.toml

Add to the appropriate `Cargo.toml`:

```toml
[[bench]]
name = "my_new_bench"
harness = false
```

### 3. Use Benchmark Groups

Organize related benchmarks:

```rust
use criterion::{BenchmarkGroup, BenchmarkId, Throughput};

fn bench_various_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_operation");

    for size in [10, 100, 1000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("size", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Benchmark code
                });
            },
        );
    }

    group.finish();
}
```

### 4. Follow Best Practices

- **Use `black_box()`** to prevent compiler optimization
- **Set `Throughput`** for operations that process multiple items
- **Use `iter_batched()`** for setup-heavy benchmarks
- **Keep benchmarks focused** - one operation per benchmark
- **Use realistic data** - mirror actual usage patterns
- **Document expectations** - add comments with target performance

## Continuous Integration

### Benchmark Comparison in CI

To track performance over time, save baselines:

```bash
# Save baseline from main branch
git checkout main
cargo bench --workspace -- --save-baseline main

# Compare feature branch
git checkout feature-branch
cargo bench --workspace -- --baseline main
```

### Automated Regression Detection

In CI, fail the build on significant regressions:

```yaml
# .github/workflows/benchmarks.yml
- name: Run benchmarks
  run: |
    cargo bench --workspace -- --save-baseline current

- name: Compare with baseline
  run: |
    cargo bench --workspace -- --baseline main --threshold 10
    # Fails if any benchmark regresses by >10%
```

## Historical Performance Data

Track performance over time by saving benchmark results:

```bash
# Create benchmarks directory
mkdir -p benchmarks/results

# Save dated baseline
cargo bench --workspace -- --save-baseline "$(date +%Y-%m-%d)"

# Save to git
git add target/criterion/
git commit -m "Benchmark results: $(date +%Y-%m-%d)"
```

## Profiling for Optimization

### When to Optimize

1. **Benchmark first** - Measure before optimizing
2. **Identify bottlenecks** - Use profiling tools
3. **Set targets** - Know what "good enough" looks like
4. **Optimize deliberately** - Focus on hot paths
5. **Measure impact** - Verify improvements with benchmarks

### Profiling Tools

**CPU Profiling:**
```bash
# Using perf (Linux)
cargo build --release
perf record --call-graph=dwarf target/release/lash index
perf report

# Using Instruments (macOS)
cargo instruments --release --bench indexing -- --profile-time=10
```

**Memory Profiling:**
```bash
# Using valgrind
cargo build --release
valgrind --tool=massif target/release/lash index
ms_print massif.out.<pid>

# Using heaptrack (Linux)
heaptrack target/release/lash index
heaptrack_gui heaptrack.lash.<pid>.gz
```

**Flamegraphs:**
```bash
cargo flamegraph --bench indexing -- --bench
```

## Troubleshooting

### Benchmarks Take Too Long

Use `--quick` for faster iterations during development:
```bash
cargo bench --package lash-core -- --quick
```

### Noisy Results (High Variance)

Reduce system noise:
- Close other applications
- Disable CPU frequency scaling
- Run on isolated CPU cores (Linux: `taskset`)
- Increase sample size (criterion config)

### Out of Memory

Reduce benchmark dataset sizes or use `iter_batched` with cleanup:

```rust
b.iter_batched(
    || setup_large_data(),
    |data| benchmark_operation(data),
    criterion::BatchSize::LargeInput, // Cleans up between iterations
);
```

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Flamegraph Guide](https://www.brendangregg.com/flamegraphs.html)
- Design doc section 13.5: Performance Optimization Strategy

## Summary

Lash's benchmark suite provides comprehensive performance testing across all major operations:

- **5 benchmark files** covering parsing, linting, graph operations, indexing, and search
- **40+ individual benchmarks** testing various scenarios and project sizes
- **Clear performance targets** for every operation
- **Regression detection** to catch performance degradations
- **HTML reports** for visual analysis and tracking

Run `cargo bench --workspace` regularly to ensure Lash remains fast and responsive.
