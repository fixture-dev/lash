//! Benchmarks for searching contextual notes
//!
//! This benchmark suite measures the impact of contextual notes on FTS5 search performance.
//! We test search queries that match tasks, notes, or both to understand the performance
//! characteristics when notes are included in the search index.
//!
//! Performance targets:
//! - Search overhead with notes: <10% increase over baseline
//! - Note-specific searches should be efficient
//!
//! Run benchmarks with:
//! ```bash
//! cargo bench --package lash-db --bench notes_search_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lash_db::{
    indexer::{Indexer, IndexerConfig},
    init_database, open_database,
    search::{search, SearchQuery},
};
use lash_types::LashConfig;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Project configuration for search benchmarks
#[derive(Debug, Clone, Copy)]
struct ProjectConfig {
    name: &'static str,
    file_count: usize,
    tasks_per_file: usize,
    note_density: f64,
    notes_per_task: usize,
}

const SEARCH_CONFIGS: &[ProjectConfig] = &[
    ProjectConfig {
        name: "small_no_notes",
        file_count: 20,
        tasks_per_file: 5,
        note_density: 0.0,
        notes_per_task: 0,
    },
    ProjectConfig {
        name: "small_with_notes",
        file_count: 20,
        tasks_per_file: 5,
        note_density: 0.5,
        notes_per_task: 3,
    },
    ProjectConfig {
        name: "medium_no_notes",
        file_count: 200,
        tasks_per_file: 5,
        note_density: 0.0,
        notes_per_task: 0,
    },
    ProjectConfig {
        name: "medium_with_notes",
        file_count: 200,
        tasks_per_file: 5,
        note_density: 0.5,
        notes_per_task: 3,
    },
];

/// Search patterns that target different content types
const SEARCH_PATTERNS: &[(&str, &str)] = &[
    ("task_word", "implement"),                   // Common in task titles
    ("note_word", "requirement"),                 // Common in notes
    ("both_words", "feature details"),            // Appears in both
    ("specific_note", "implementation approach"), // Very specific to notes
];

/// Generate a task file with searchable content in notes
fn generate_searchable_task_file(
    file_id: &str,
    task_count: usize,
    note_density: f64,
    notes_per_task: usize,
) -> String {
    let mut content =
        format!("# Task File {file_id}\n\n@id: {file_id}\n@status: in-progress\n\n## Tasks\n\n");

    let task_verbs = ["Implement", "Fix", "Add", "Update", "Refactor", "Optimize"];
    let task_objects = [
        "feature",
        "bug",
        "functionality",
        "tests",
        "documentation",
        "performance",
    ];
    let note_contexts = [
        "Requirement: Use library X for implementation approach",
        "Details: Consider edge cases and validation rules",
        "Implementation approach: Break into smaller components",
        "Testing strategy: Include unit and integration tests",
        "Performance consideration: Optimize for large datasets",
        "Security note: Validate all user inputs thoroughly",
    ];

    for i in 0..task_count {
        let verb = task_verbs[i % task_verbs.len()];
        let object = task_objects[i % task_objects.len()];

        content.push_str(&format!("- [ ] {verb} {object} for component {file_id}\n"));

        // Add notes based on density
        let should_add_notes = (i as f64) / (task_count as f64) < note_density;
        if should_add_notes {
            for j in 0..notes_per_task {
                let note = note_contexts[j % note_contexts.len()];
                content.push_str(&format!("  - {note}\n"));
            }
        }

        // Add some child tasks
        if i % 3 == 0 && i > 0 {
            content.push_str(&format!("  - [ ] Subtask {}: Design phase\n", i + 1));

            if should_add_notes && notes_per_task > 2 {
                content.push_str("    - Follow established design patterns\n");
            }

            content.push_str(&format!("  - [ ] Subtask {}: Implementation\n", i + 1));
        }
    }

    content
}

/// Create a test project optimized for search benchmarks
fn create_search_test_project(config: ProjectConfig) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let subdirs = ["backend", "frontend", "docs", "tests", "core"];
    for subdir in &subdirs {
        fs::create_dir_all(root.join(subdir)).unwrap();
    }

    for i in 0..config.file_count {
        let subdir = subdirs[i % subdirs.len()];
        let file_path = root.join(subdir).join(format!("task_{i:04}.md"));
        let file_id = format!("task_{i:04}");
        let content = generate_searchable_task_file(
            &file_id,
            config.tasks_per_file,
            config.note_density,
            config.notes_per_task,
        );

        fs::write(&file_path, content).unwrap();
    }

    let index_content = "# Project Index\n\n@id: index\n\n## Tasks\n\n- [ ] Root task\n";
    fs::write(root.join("lash.index.md"), index_content).unwrap();

    temp_dir
}

/// Setup and index a project for search benchmarks
fn setup_indexed_project(config: ProjectConfig) -> (TempDir, PathBuf) {
    let test_project = create_search_test_project(config);
    let project_root = test_project.path().to_path_buf();
    let db_path = project_root.join(".lash").join("db.sqlite");
    fs::create_dir_all(db_path.parent().unwrap()).unwrap();

    let conn = init_database(&db_path).unwrap();
    let indexer_config = IndexerConfig::new(project_root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
    indexer.index_project().unwrap();

    (test_project, db_path)
}

/// Benchmark search queries with and without notes
fn bench_search_with_notes(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_with_notes");

    for config in SEARCH_CONFIGS {
        let (_temp, db_path) = setup_indexed_project(*config);
        let total_tasks = config.file_count * config.tasks_per_file;

        group.throughput(Throughput::Elements(total_tasks as u64));

        for (pattern_name, query_text) in SEARCH_PATTERNS {
            let benchmark_name = format!("{}_{}", config.name, pattern_name);

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

/// Benchmark note-specific searches
fn bench_note_specific_searches(c: &mut Criterion) {
    let mut group = c.benchmark_group("note_specific_searches");

    let config = ProjectConfig {
        name: "medium_dense_notes",
        file_count: 200,
        tasks_per_file: 5,
        note_density: 0.75,
        notes_per_task: 5,
    };

    let (_temp, db_path) = setup_indexed_project(config);

    // Queries that should only match notes
    let note_queries = [
        "requirement",
        "implementation approach",
        "testing strategy",
        "performance consideration",
        "security note",
    ];

    for query_text in &note_queries {
        group.bench_with_input(
            BenchmarkId::from_parameter(query_text),
            query_text,
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

    group.finish();
}

/// Benchmark baseline vs notes search performance
fn bench_baseline_vs_notes_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_vs_notes_search");

    // Baseline: no notes
    let baseline_config = ProjectConfig {
        name: "baseline",
        file_count: 200,
        tasks_per_file: 5,
        note_density: 0.0,
        notes_per_task: 0,
    };

    let (_temp_baseline, db_path_baseline) = setup_indexed_project(baseline_config);

    group.bench_function("baseline_no_notes", |b| {
        b.iter(|| {
            let conn = open_database(&db_path_baseline).unwrap();
            let query = SearchQuery::new("implement feature");
            let results = search(&conn, &query).unwrap();
            black_box(results);
        });
    });

    // With notes (50% density)
    let notes_config = ProjectConfig {
        name: "with_notes",
        file_count: 200,
        tasks_per_file: 5,
        note_density: 0.5,
        notes_per_task: 3,
    };

    let (_temp_notes, db_path_notes) = setup_indexed_project(notes_config);

    group.bench_function("with_notes_50pct", |b| {
        b.iter(|| {
            let conn = open_database(&db_path_notes).unwrap();
            let query = SearchQuery::new("implement feature");
            let results = search(&conn, &query).unwrap();
            black_box(results);
        });
    });

    // Dense notes (100% density)
    let dense_config = ProjectConfig {
        name: "dense_notes",
        file_count: 200,
        tasks_per_file: 5,
        note_density: 1.0,
        notes_per_task: 10,
    };

    let (_temp_dense, db_path_dense) = setup_indexed_project(dense_config);

    group.bench_function("dense_notes_100pct", |b| {
        b.iter(|| {
            let conn = open_database(&db_path_dense).unwrap();
            let query = SearchQuery::new("implement feature");
            let results = search(&conn, &query).unwrap();
            black_box(results);
        });
    });

    group.finish();
}

/// Benchmark search result ranking with notes
fn bench_search_ranking_with_notes(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_ranking_with_notes");

    let config = ProjectConfig {
        name: "ranking_test",
        file_count: 100,
        tasks_per_file: 10,
        note_density: 0.5,
        notes_per_task: 5,
    };

    let (_temp, db_path) = setup_indexed_project(config);

    // Test queries that match many results (ranking matters)
    let ranking_queries = [
        "implement",
        "feature implementation",
        "testing",
        "requirement details",
    ];

    for query_text in &ranking_queries {
        group.bench_with_input(
            BenchmarkId::from_parameter(query_text),
            query_text,
            |b, query_str| {
                let query_text = query_str.to_string();
                b.iter(|| {
                    let conn = open_database(&db_path).unwrap();
                    let search_query = SearchQuery::new(query_text.clone()).with_limit(50);
                    let results = search(&conn, &search_query).unwrap();
                    // Ensure we access ranking scores
                    for result in &results.results {
                        black_box(&result.score);
                    }
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark search with pagination on note-heavy database
fn bench_search_pagination_with_notes(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_pagination_with_notes");

    let config = ProjectConfig {
        name: "pagination_test",
        file_count: 500,
        tasks_per_file: 5,
        note_density: 0.6,
        notes_per_task: 4,
    };

    let (_temp, db_path) = setup_indexed_project(config);

    for limit in [10, 20, 50, 100] {
        group.bench_with_input(BenchmarkId::new("limit", limit), &limit, |b, &limit| {
            b.iter(|| {
                let conn = open_database(&db_path).unwrap();
                let query = SearchQuery::new("implementation").with_limit(limit);
                let results = search(&conn, &query).unwrap();
                black_box(results);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_search_with_notes,
    bench_note_specific_searches,
    bench_baseline_vs_notes_search,
    bench_search_ranking_with_notes,
    bench_search_pagination_with_notes,
);
criterion_main!(benches);
