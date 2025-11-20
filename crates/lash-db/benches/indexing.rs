//! Benchmarks for the indexing engine
//!
//! This benchmark suite measures indexing performance across different project sizes
//! and indexing scenarios to ensure we meet performance targets:
//!
//! - Small project (10 files, ~50 tasks): <50ms
//! - Medium project (100 files, ~500 tasks): <500ms
//! - Large project (1000 files, ~5000 tasks): <5s
//!
//! Run benchmarks with:
//! ```bash
//! cargo bench --package lash-db --bench indexing
//! ```
//!
//! Generate HTML reports in target/criterion/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lash_db::{
    indexer::{Indexer, IndexerConfig},
    init_database, open_database,
};
use lash_types::LashConfig;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Project size configuration for benchmarks
#[derive(Debug, Clone, Copy)]
struct ProjectSize {
    name: &'static str,
    file_count: usize,
    tasks_per_file: usize,
}

const SIZES: &[ProjectSize] = &[
    ProjectSize {
        name: "small",
        file_count: 10,
        tasks_per_file: 5,
    },
    ProjectSize {
        name: "medium",
        file_count: 100,
        tasks_per_file: 5,
    },
    ProjectSize {
        name: "large",
        file_count: 1000,
        tasks_per_file: 5,
    },
];

/// Generate a markdown task file with the specified number of tasks
fn generate_task_file(file_id: &str, task_count: usize) -> String {
    let mut content =
        format!("# Task File {file_id}\n\n@id: {file_id}\n@status: in-progress\n\n## Tasks\n\n");

    for i in 0..task_count {
        content.push_str(&format!("- [ ] Task {} in file {}\n", i + 1, file_id));
        if i % 3 == 0 && i > 0 {
            // Add some child tasks for hierarchy
            content.push_str(&format!("  - [ ] Subtask {}.1\n", i + 1));
            content.push_str(&format!("  - [ ] Subtask {}.2\n", i + 1));
        }
    }

    content
}

/// Create a test project with the specified size
fn create_test_project(size: ProjectSize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create subdirectories for better realism
    let subdirs = ["backend", "frontend", "docs", "tests"];
    for subdir in &subdirs {
        fs::create_dir_all(root.join(subdir)).unwrap();
    }

    // Generate files distributed across subdirectories
    for i in 0..size.file_count {
        let subdir = subdirs[i % subdirs.len()];
        let file_path = root.join(subdir).join(format!("task_{i:04}.md"));
        let file_id = format!("task_{i:04}");
        let content = generate_task_file(&file_id, size.tasks_per_file);

        fs::write(&file_path, content).unwrap();
    }

    // Create index file
    let index_content = "# Project Index\n\n@id: index\n\n## Tasks\n\n- [ ] Root task\n";
    fs::write(root.join("lash.index.md"), index_content).unwrap();

    temp_dir
}

/// Modify a percentage of files in the project to simulate changes
fn modify_files(root: &Path, percentage: f64) {
    let all_files: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().is_some_and(|ext| {
                ext == "md" && e.path().file_name() != Some("lash.index.md".as_ref())
            })
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    let files_to_modify = (all_files.len() as f64 * percentage) as usize;

    for file_path in all_files.iter().take(files_to_modify) {
        let mut content = fs::read_to_string(file_path).unwrap();
        content.push_str("\n- [ ] New task added\n");
        fs::write(file_path, content).unwrap();
    }
}

/// Delete a percentage of files in the project
fn delete_files(root: &Path, percentage: f64) {
    let all_files: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().is_some_and(|ext| {
                ext == "md" && e.path().file_name() != Some("lash.index.md".as_ref())
            })
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    let files_to_delete = (all_files.len() as f64 * percentage) as usize;

    for file_path in all_files.iter().take(files_to_delete) {
        fs::remove_file(file_path).unwrap();
    }
}

/// Add new files to the project
fn add_files(root: &Path, count: usize) {
    for i in 0..count {
        let file_path = root.join(format!("new_task_{i:04}.md"));
        let file_id = format!("new_task_{i:04}");
        let content = generate_task_file(&file_id, 5);
        fs::write(&file_path, content).unwrap();
    }
}

/// Benchmark full indexing from scratch
fn bench_full_indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_indexing");

    for size in SIZES {
        let test_project = create_test_project(*size);
        let project_root = test_project.path().to_path_buf();
        let db_path = project_root.join(".lash").join("db.sqlite");
        fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        group.throughput(Throughput::Elements(size.file_count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size.name), &size, |b, _| {
            b.iter(|| {
                // Clean DB for each iteration
                let _ = fs::remove_file(&db_path);

                let conn = init_database(&db_path).unwrap();
                let config = IndexerConfig::new(project_root.clone())
                    .with_incremental(false)
                    .with_progress(false);
                let parser_config = LashConfig::default();
                let mut indexer = Indexer::new(&conn, config, &parser_config);

                black_box(indexer.index_project().unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark incremental indexing with no changes
fn bench_incremental_no_changes(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_no_changes");

    for size in SIZES {
        let test_project = create_test_project(*size);
        let project_root = test_project.path().to_path_buf();
        let db_path = project_root.join(".lash").join("db.sqlite");
        fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        // Perform initial indexing
        let conn = init_database(&db_path).unwrap();
        let config = IndexerConfig::new(project_root.clone())
            .with_incremental(true)
            .with_progress(false);
        let parser_config = LashConfig::default();
        let mut indexer = Indexer::new(&conn, config, &parser_config);
        indexer.index_project().unwrap();
        drop(indexer);
        drop(conn);

        group.throughput(Throughput::Elements(size.file_count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size.name), &size, |b, _| {
            b.iter(|| {
                let conn = open_database(&db_path).unwrap();
                let config = IndexerConfig::new(project_root.clone())
                    .with_incremental(true)
                    .with_progress(false);
                let parser_config = LashConfig::default();
                let mut indexer = Indexer::new(&conn, config, &parser_config);

                black_box(indexer.index_project().unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark incremental indexing with 10% modified files
fn bench_incremental_10pct_modified(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_10pct_modified");

    for size in SIZES {
        let test_project = create_test_project(*size);
        let project_root = test_project.path().to_path_buf();
        let db_path = project_root.join(".lash").join("db.sqlite");
        fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        group.throughput(Throughput::Elements((size.file_count / 10) as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size.name), &size, |b, _| {
            b.iter_batched(
                || {
                    // Setup: Index project and modify 10% of files
                    let _ = fs::remove_file(&db_path);
                    let conn = init_database(&db_path).unwrap();
                    let config = IndexerConfig::new(project_root.clone())
                        .with_incremental(false)
                        .with_progress(false);
                    let parser_config = LashConfig::default();
                    let mut indexer = Indexer::new(&conn, config, &parser_config);
                    indexer.index_project().unwrap();
                    drop(indexer);
                    drop(conn);

                    modify_files(&project_root, 0.1);

                    (project_root.clone(), db_path.clone())
                },
                |(root, db)| {
                    let conn = open_database(&db).unwrap();
                    let config = IndexerConfig::new(root)
                        .with_incremental(true)
                        .with_progress(false);
                    let parser_config = LashConfig::default();
                    let mut indexer = Indexer::new(&conn, config, &parser_config);

                    black_box(indexer.index_project().unwrap());
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

/// Benchmark incremental indexing with 10% new and 10% deleted files
fn bench_incremental_10pct_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_10pct_churn");

    for size in SIZES {
        let test_project = create_test_project(*size);
        let project_root = test_project.path().to_path_buf();
        let db_path = project_root.join(".lash").join("db.sqlite");
        fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        let files_to_churn = (size.file_count as f64 * 0.1) as usize;
        group.throughput(Throughput::Elements((files_to_churn * 2) as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size.name), &size, |b, _| {
            b.iter_batched(
                || {
                    // Setup: Index project, delete 10%, add 10%
                    let _ = fs::remove_file(&db_path);
                    let conn = init_database(&db_path).unwrap();
                    let config = IndexerConfig::new(project_root.clone())
                        .with_incremental(false)
                        .with_progress(false);
                    let parser_config = LashConfig::default();
                    let mut indexer = Indexer::new(&conn, config, &parser_config);
                    indexer.index_project().unwrap();
                    drop(indexer);
                    drop(conn);

                    delete_files(&project_root, 0.1);
                    add_files(&project_root, files_to_churn);

                    (project_root.clone(), db_path.clone())
                },
                |(root, db)| {
                    let conn = open_database(&db).unwrap();
                    let config = IndexerConfig::new(root)
                        .with_incremental(true)
                        .with_progress(false);
                    let parser_config = LashConfig::default();
                    let mut indexer = Indexer::new(&conn, config, &parser_config);

                    black_box(indexer.index_project().unwrap());
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

/// Benchmark with profiling enabled to measure overhead
fn bench_profiling_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_overhead");

    let size = SIZES[1]; // Use medium size for this test
    let test_project = create_test_project(size);
    let project_root = test_project.path().to_path_buf();
    let db_path = project_root.join(".lash").join("db.sqlite");
    fs::create_dir_all(db_path.parent().unwrap()).unwrap();

    group.bench_function("disabled", |b| {
        b.iter(|| {
            let _ = fs::remove_file(&db_path);
            let conn = init_database(&db_path).unwrap();
            let config = IndexerConfig::new(project_root.clone())
                .with_incremental(false)
                .with_progress(false)
                .with_profiling(false);
            let parser_config = LashConfig::default();
            let mut indexer = Indexer::new(&conn, config, &parser_config);

            black_box(indexer.index_project().unwrap());
        });
    });

    group.bench_function("enabled", |b| {
        b.iter(|| {
            let _ = fs::remove_file(&db_path);
            let conn = init_database(&db_path).unwrap();
            let config = IndexerConfig::new(project_root.clone())
                .with_incremental(false)
                .with_progress(false)
                .with_profiling(true);
            let parser_config = LashConfig::default();
            let mut indexer = Indexer::new(&conn, config, &parser_config);

            let report = indexer.index_project().unwrap();
            black_box(report.profile.unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_full_indexing,
    bench_incremental_no_changes,
    bench_incremental_10pct_modified,
    bench_incremental_10pct_churn,
    bench_profiling_overhead,
);
criterion_main!(benches);
