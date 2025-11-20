//! Benchmarks for dependency graph operations
//!
//! Measures the performance of key graph operations to ensure they meet
//! the O(1) and O(E+V) complexity requirements.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use lash_core::dependency::{DependencyGraph, EdgeData, NodeData};
use lash_types::{DependencyKind, TaskStatus};

/// Create a graph with a linear chain of dependencies
fn create_chain_graph(size: usize) -> DependencyGraph {
    let mut graph = DependencyGraph::new();

    for i in 0..size {
        let id = format!("test#task{i}");
        let node = NodeData::new(format!("Task {i}"), TaskStatus::Open, "test".to_string(), 0);
        graph.add_node(id, node);
    }

    for i in 0..(size - 1) {
        let from = format!("test#task{i}");
        let to = format!("test#task{}", i + 1);
        graph.add_edge(from, to, EdgeData::new(DependencyKind::ExplicitId, None));
    }

    graph
}

/// Create a diamond-shaped dependency graph
fn create_diamond_graph(layers: usize) -> DependencyGraph {
    let mut graph = DependencyGraph::new();

    // Create layers with exponentially growing width
    for layer in 0..layers {
        let width = 1 << layer; // 1, 2, 4, 8, 16...
        for i in 0..width {
            let id = format!("test#layer{layer}_node{i}");
            let node = NodeData::new(
                format!("Layer {layer} Node {i}"),
                TaskStatus::Open,
                "test".to_string(),
                0,
            );
            graph.add_node(id, node);
        }
    }

    // Connect layers: each node connects to two in the next layer
    for layer in 0..(layers - 1) {
        let width = 1 << layer;
        for i in 0..width {
            let from = format!("test#layer{layer}_node{i}");
            let next_layer = layer + 1;

            let to1 = format!("test#layer{next_layer}_node{}", i * 2);
            let to2 = format!("test#layer{next_layer}_node{}", i * 2 + 1);

            graph.add_edge(
                from.clone(),
                to1,
                EdgeData::new(DependencyKind::ExplicitId, None),
            );
            graph.add_edge(from, to2, EdgeData::new(DependencyKind::ExplicitId, None));
        }
    }

    graph
}

/// Benchmark direct dependency queries (should be O(1))
fn bench_direct_queries(c: &mut Criterion) {
    let sizes = vec![10, 100, 1000];

    let mut group = c.benchmark_group("direct_queries");

    for size in sizes {
        let graph = create_chain_graph(size);
        let mid_task = format!("test#task{}", size / 2);

        group.bench_with_input(BenchmarkId::new("get_dependencies", size), &size, |b, _| {
            b.iter(|| {
                black_box(graph.get_dependencies(black_box(&mid_task)));
            });
        });

        group.bench_with_input(BenchmarkId::new("get_dependents", size), &size, |b, _| {
            b.iter(|| {
                black_box(graph.get_dependents(black_box(&mid_task)));
            });
        });

        group.bench_with_input(
            BenchmarkId::new("get_dependency_ids", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(graph.get_dependency_ids(black_box(&mid_task)));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark transitive dependency queries (should be O(E+V))
fn bench_transitive_queries(c: &mut Criterion) {
    let sizes = vec![10, 50, 100];

    let mut group = c.benchmark_group("transitive_queries");

    for size in sizes {
        let graph = create_chain_graph(size);

        group.bench_with_input(
            BenchmarkId::new("get_descendants_chain", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(graph.get_descendants(black_box("test#task0")).unwrap());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("get_ancestors_chain", size),
            &size,
            |b, _| {
                b.iter(|| {
                    let last_task = format!("test#task{}", size - 1);
                    black_box(graph.get_ancestors(black_box(&last_task)).unwrap());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark depth-limited queries
fn bench_depth_limited_queries(c: &mut Criterion) {
    let graph = create_chain_graph(100);

    let mut group = c.benchmark_group("depth_limited_queries");

    for depth in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("get_descendants_with_depth", depth),
            &depth,
            |b, &depth| {
                b.iter(|| {
                    black_box(
                        graph
                            .get_descendants_with_depth(black_box("test#task0"), black_box(depth))
                            .unwrap(),
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark graph construction
fn bench_graph_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_construction");

    for size in [10, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::new("create_chain_graph", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    black_box(create_chain_graph(black_box(size)));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark queries on diamond-shaped graphs
fn bench_diamond_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("diamond_queries");

    for layers in [3, 4, 5, 6] {
        let graph = create_diamond_graph(layers);
        let node_count = (1 << layers) - 1; // 2^layers - 1

        group.bench_with_input(
            BenchmarkId::new("get_descendants_diamond", node_count),
            &layers,
            |b, _| {
                b.iter(|| {
                    black_box(
                        graph
                            .get_descendants(black_box("test#layer0_node0"))
                            .unwrap(),
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark filtering operations
fn bench_filtering(c: &mut Criterion) {
    let mut graph = DependencyGraph::new();

    // Create a graph with mixed dependency types
    for i in 0..100 {
        let id = format!("test#task{i}");
        graph.add_node(
            id,
            NodeData::new(format!("Task {i}"), TaskStatus::Open, "test".to_string(), 0),
        );
    }

    // Add hierarchy dependencies (even numbers)
    for i in (0..100).step_by(2) {
        let from = format!("test#task{i}");
        let to = format!("test#task{}", i + 1);
        graph.add_edge(from, to, EdgeData::new(DependencyKind::Hierarchy, None));
    }

    // Add explicit dependencies (odd numbers)
    for i in (1..99).step_by(2) {
        let from = format!("test#task{i}");
        let to = format!("test#task{}", i + 1);
        graph.add_edge(from, to, EdgeData::new(DependencyKind::ExplicitId, None));
    }

    c.bench_function("get_dependencies_by_kind_hierarchy", |b| {
        b.iter(|| {
            black_box(graph.get_dependencies_by_kind(
                black_box("test#task50"),
                black_box(&DependencyKind::Hierarchy),
            ));
        });
    });

    c.bench_function("get_dependencies_by_kind_explicit", |b| {
        b.iter(|| {
            black_box(graph.get_dependencies_by_kind(
                black_box("test#task51"),
                black_box(&DependencyKind::ExplicitId),
            ));
        });
    });
}

criterion_group!(
    benches,
    bench_direct_queries,
    bench_transitive_queries,
    bench_depth_limited_queries,
    bench_graph_construction,
    bench_diamond_queries,
    bench_filtering,
);
criterion_main!(benches);
