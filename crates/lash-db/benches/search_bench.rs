//! Benchmarks for search performance
//!
//! This benchmark suite measures search query performance across different project sizes
//! to ensure we meet performance targets:
//!
//! - Small project (100 tasks): <50ms
//! - Medium project (1000 tasks): <150ms
//! - Large project (10000 tasks): <500ms
//!
//! Run benchmarks with:
//! ```bash
//! cargo bench --package lash-db --bench search_bench
//! ```
//!
//! Generate HTML reports in target/criterion/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lash_db::{
    indexer::{Indexer, IndexerConfig},
    init_database, open_database,
    search::{search, SearchQuery},
};
use lash_types::{LashConfig, TaskStatus};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Project size configuration for search benchmarks
#[derive(Debug, Clone, Copy)]
struct ProjectSize {
    name: &'static str,
    file_count: usize,
    tasks_per_file: usize,
}

const SIZES: &[ProjectSize] = &[
    ProjectSize {
        name: "small",
        file_count: 20,
        tasks_per_file: 5, // 100 tasks total
    },
    ProjectSize {
        name: "medium",
        file_count: 200,
        tasks_per_file: 5, // 1000 tasks total
    },
    ProjectSize {
        name: "large",
        file_count: 2000,
        tasks_per_file: 5, // 10000 tasks total
    },
];

/// Search query patterns to test
const QUERY_PATTERNS: &[(&str, &str)] = &[
    ("single_word", "implement"),
    ("two_words", "parser backend"),
    ("common_word", "task"),
    ("rare_word", "xyzabc"),
    ("with_label", "label:backend"),
    ("with_status", "status:open"),
    ("complex", "implement parser label:backend"),
];

/// Generate a markdown task file with realistic content
fn generate_task_file(file_id: &str, task_count: usize, labels: &[&str]) -> String {
    let mut content =
        format!("# Task File {file_id}\n\n@id: {file_id}\n@status: in-progress\n\n## Tasks\n\n");

    let task_templates = [
        ("Implement", "functionality", "backend"),
        ("Fix", "bug", "bugfix"),
        ("Add", "tests", "testing"),
        ("Update", "documentation", "docs"),
        ("Refactor", "code", "refactoring"),
        ("Optimize", "performance", "optimization"),
        ("Design", "architecture", "design"),
        ("Research", "approach", "research"),
    ];

    for i in 0..task_count {
        let template = &task_templates[i % task_templates.len()];
        let label = labels[i % labels.len()];

        content.push_str(&format!(
            "- [ ] {} {} for component {} #{}\n",
            template.0, template.1, file_id, label
        ));

        // Add some child tasks for hierarchy
        if i % 3 == 0 && i > 0 {
            content.push_str(&format!(
                "  - [ ] Subtask {}.1: {} phase\n",
                i + 1,
                template.2
            ));
            content.push_str(&format!(
                "  - [ ] Subtask {}.2: Testing and validation\n",
                i + 1
            ));
        }
    }

    content
}

/// Create a test project with realistic content for search
fn create_search_test_project(size: ProjectSize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create subdirectories
    let subdirs = ["backend", "frontend", "docs", "tests", "core"];
    for subdir in &subdirs {
        fs::create_dir_all(root.join(subdir)).unwrap();
    }

    // Labels to distribute across tasks
    let labels = ["backend", "frontend", "parser", "api", "database", "ui"];

    // Generate files distributed across subdirectories
    for i in 0..size.file_count {
        let subdir = subdirs[i % subdirs.len()];
        let file_path = root.join(subdir).join(format!("task_{i:04}.md"));
        let file_id = format!("task_{i:04}");
        let content = generate_task_file(&file_id, size.tasks_per_file, &labels);

        fs::write(&file_path, content).unwrap();
    }

    // Create index file
    let index_content = "# Project Index\n\n@id: index\n\n## Tasks\n\n- [ ] Root task\n";
    fs::write(root.join("lash.index.md"), index_content).unwrap();

    temp_dir
}

/// Setup a project and index it for search benchmarks
fn setup_indexed_project(size: ProjectSize) -> (TempDir, PathBuf) {
    let test_project = create_search_test_project(size);
    let project_root = test_project.path().to_path_buf();
    let db_path = project_root.join(".lash").join("db.sqlite");
    fs::create_dir_all(db_path.parent().unwrap()).unwrap();

    // Index the project
    let conn = init_database(&db_path).unwrap();
    let config = IndexerConfig::new(project_root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().unwrap();

    (test_project, db_path)
}

/// Benchmark basic search queries
fn bench_search_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_queries");

    for size in SIZES {
        let (_temp, db_path) = setup_indexed_project(*size);
        let total_tasks = size.file_count * size.tasks_per_file;

        group.throughput(Throughput::Elements(total_tasks as u64));

        for (pattern_name, query_text) in QUERY_PATTERNS {
            let benchmark_name = format!("{}_{}", size.name, pattern_name);

            group.bench_with_input(
                BenchmarkId::from_parameter(&benchmark_name),
                &query_text,
                |b, query_str| {
                    let query_text = query_str.to_string();
                    b.iter(|| {
                        let conn = open_database(&db_path).unwrap();
                        let search_query = SearchQuery::new(query_text.clone());
                        let results = search(&conn, &search_query).unwrap();
                        black_box(results);
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark search with pagination
fn bench_search_pagination(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_pagination");

    for size in SIZES {
        let (_temp, db_path) = setup_indexed_project(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size.name), &size, |b, _| {
            b.iter(|| {
                let conn = open_database(&db_path).unwrap();
                let query = SearchQuery::new("implement").with_limit(20);
                let results = search(&conn, &query).unwrap();
                black_box(results);
            });
        });
    }

    group.finish();
}

/// Benchmark search with filters
fn bench_search_with_filters(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_with_filters");

    for size in SIZES {
        let (_temp, db_path) = setup_indexed_project(*size);

        // Test different filter combinations
        group.bench_with_input(
            BenchmarkId::new("label_filter", size.name),
            &size,
            |b, _| {
                b.iter(|| {
                    let conn = open_database(&db_path).unwrap();
                    let query = SearchQuery::new("task").with_label("backend".to_string());
                    let results = search(&conn, &query).unwrap();
                    black_box(results);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("status_filter", size.name),
            &size,
            |b, _| {
                b.iter(|| {
                    let conn = open_database(&db_path).unwrap();
                    let query = SearchQuery::new("task").with_status(TaskStatus::Open);
                    let results = search(&conn, &query).unwrap();
                    black_box(results);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("multiple_filters", size.name),
            &size,
            |b, _| {
                b.iter(|| {
                    let conn = open_database(&db_path).unwrap();
                    let query = SearchQuery::new("implement")
                        .with_label("backend".to_string())
                        .with_status(TaskStatus::Open);
                    let results = search(&conn, &query).unwrap();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark repeated queries (cache effectiveness)
fn bench_repeated_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("repeated_queries");

    let size = SIZES[1]; // Use medium size
    let (_temp, db_path) = setup_indexed_project(size);

    group.bench_function("first_query", |b| {
        b.iter(|| {
            let conn = open_database(&db_path).unwrap();
            let query = SearchQuery::new("implement parser");
            let results = search(&conn, &query).unwrap();
            black_box(results);
        });
    });

    // This will show cache effectiveness once caching is implemented
    group.bench_function("repeated_query", |b| {
        let conn = open_database(&db_path).unwrap();
        b.iter(|| {
            let query = SearchQuery::new("implement parser");
            let results = search(&conn, &query).unwrap();
            black_box(results);
        });
    });

    group.finish();
}

/// Benchmark search result snippet generation
fn bench_snippet_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("snippet_generation");

    for size in SIZES {
        let (_temp, db_path) = setup_indexed_project(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size.name), &size, |b, _| {
            b.iter(|| {
                let conn = open_database(&db_path).unwrap();
                let query = SearchQuery::new("implement functionality");
                let results = search(&conn, &query).unwrap();

                // Force snippet access to ensure it's computed
                for result in &results.results {
                    black_box(&result.snippet);
                }

                black_box(results);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_search_queries,
    bench_search_pagination,
    bench_search_with_filters,
    bench_repeated_queries,
    bench_snippet_generation,
);
criterion_main!(benches);
