//! Benchmarks for indexing contextual notes
//!
//! This benchmark suite measures the impact of contextual notes on indexing performance.
//! We test indexing with varying note densities and sizes to ensure notes don't significantly
//! impact indexing speed.
//!
//! Performance targets:
//! - Indexing overhead with notes: <10% increase over baseline
//! - Incremental indexing should efficiently handle note changes
//!
//! Run benchmarks with:
//! ```bash
//! cargo bench --package lash-db --bench notes_indexing_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lash_db::{
    indexer::{Indexer, IndexerConfig},
    init_database, open_database,
};
use lash_types::LashConfig;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Project configuration for benchmarks
#[derive(Debug, Clone, Copy)]
struct ProjectConfig {
    name: &'static str,
    file_count: usize,
    tasks_per_file: usize,
    note_density: f64, // 0.0 to 1.0
    notes_per_task: usize,
}

const CONFIGS: &[ProjectConfig] = &[
    ProjectConfig {
        name: "small_no_notes",
        file_count: 10,
        tasks_per_file: 5,
        note_density: 0.0,
        notes_per_task: 0,
    },
    ProjectConfig {
        name: "small_with_notes",
        file_count: 10,
        tasks_per_file: 5,
        note_density: 0.5,
        notes_per_task: 3,
    },
    ProjectConfig {
        name: "medium_no_notes",
        file_count: 100,
        tasks_per_file: 5,
        note_density: 0.0,
        notes_per_task: 0,
    },
    ProjectConfig {
        name: "medium_with_notes",
        file_count: 100,
        tasks_per_file: 5,
        note_density: 0.5,
        notes_per_task: 3,
    },
];

/// Generate a task file with contextual notes
fn generate_task_file_with_notes(
    file_id: &str,
    task_count: usize,
    note_density: f64,
    notes_per_task: usize,
) -> String {
    let mut content =
        format!("# Task File {file_id}\n\n@id: {file_id}\n@status: in-progress\n\n## Tasks\n\n");

    for i in 0..task_count {
        content.push_str(&format!("- [ ] Task {} in file {}\n", i + 1, file_id));

        // Add notes based on density
        let should_add_notes = (i as f64) / (task_count as f64) < note_density;
        if should_add_notes {
            for j in 1..=notes_per_task {
                content.push_str(&format!(
                    "  - Contextual note {j}: Details about implementation approach for task {}\n",
                    i + 1
                ));
            }
        }

        // Add some child tasks for hierarchy (every 3rd task)
        if i % 3 == 0 && i > 0 {
            content.push_str(&format!("  - [ ] Subtask {}.1\n", i + 1));

            // Add note to child task sometimes
            if should_add_notes && notes_per_task > 1 {
                content.push_str("    - Implementation detail for subtask\n");
            }

            content.push_str(&format!("  - [ ] Subtask {}.2\n", i + 1));
        }
    }

    content
}

/// Create a test project with specified configuration
fn create_test_project(config: ProjectConfig) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create subdirectories
    let subdirs = ["backend", "frontend", "docs", "tests"];
    for subdir in &subdirs {
        fs::create_dir_all(root.join(subdir)).unwrap();
    }

    // Generate files
    for i in 0..config.file_count {
        let subdir = subdirs[i % subdirs.len()];
        let file_path = root.join(subdir).join(format!("task_{i:04}.md"));
        let file_id = format!("task_{i:04}");
        let content = generate_task_file_with_notes(
            &file_id,
            config.tasks_per_file,
            config.note_density,
            config.notes_per_task,
        );

        fs::write(&file_path, content).unwrap();
    }

    // Create index file
    let index_content = "# Project Index\n\n@id: index\n\n## Tasks\n\n- [ ] Root task\n";
    fs::write(root.join("lash.index.md"), index_content).unwrap();

    temp_dir
}

/// Modify files to add more notes
fn add_notes_to_files(root: &std::path::Path, percentage: f64, notes_to_add: usize) {
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
        content.push_str("\n- [ ] New task with notes\n");
        for i in 1..=notes_to_add {
            content.push_str(&format!("  - Additional contextual note {i}\n"));
        }
        fs::write(file_path, content).unwrap();
    }
}

/// Benchmark full indexing with different note densities
fn bench_full_indexing_notes(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_indexing_notes");

    for config in CONFIGS {
        let test_project = create_test_project(*config);
        let project_root = test_project.path().to_path_buf();
        let db_path = project_root.join(".lash").join("db.sqlite");
        fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        group.throughput(Throughput::Elements(config.file_count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(config.name), &config, |b, _| {
            b.iter(|| {
                // Clean DB for each iteration
                let _ = fs::remove_file(&db_path);

                let conn = init_database(&db_path).unwrap();
                let indexer_config = IndexerConfig::new(project_root.clone())
                    .with_incremental(false)
                    .with_progress(false);
                let parser_config = LashConfig::default();
                let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);

                black_box(indexer.index_project().unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark incremental indexing with note changes
fn bench_incremental_note_changes(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_note_changes");

    // Use medium project for this test
    let config = ProjectConfig {
        name: "medium_baseline",
        file_count: 100,
        tasks_per_file: 5,
        note_density: 0.25,
        notes_per_task: 2,
    };

    let test_project = create_test_project(config);
    let project_root = test_project.path().to_path_buf();
    let db_path = project_root.join(".lash").join("db.sqlite");
    fs::create_dir_all(db_path.parent().unwrap()).unwrap();

    group.throughput(Throughput::Elements((config.file_count / 10) as u64));

    // Test adding notes to 10% of files
    group.bench_function("add_notes_10pct", |b| {
        b.iter_batched(
            || {
                // Setup: Index project and add notes to 10% of files
                let _ = fs::remove_file(&db_path);
                let conn = init_database(&db_path).unwrap();
                let indexer_config = IndexerConfig::new(project_root.clone())
                    .with_incremental(false)
                    .with_progress(false);
                let parser_config = LashConfig::default();
                let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
                indexer.index_project().unwrap();
                drop(indexer);
                drop(conn);

                // Add notes to 10% of files
                add_notes_to_files(&project_root, 0.1, 5);

                (project_root.clone(), db_path.clone())
            },
            |(root, db)| {
                let conn = open_database(&db).unwrap();
                let indexer_config = IndexerConfig::new(root)
                    .with_incremental(true)
                    .with_progress(false);
                let parser_config = LashConfig::default();
                let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);

                black_box(indexer.index_project().unwrap());
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.finish();
}

/// Benchmark indexing with extreme note density
fn bench_extreme_note_density(c: &mut Criterion) {
    let mut group = c.benchmark_group("extreme_note_density");

    let configs = [
        ProjectConfig {
            name: "dense_notes",
            file_count: 50,
            tasks_per_file: 10,
            note_density: 1.0,
            notes_per_task: 10,
        },
        ProjectConfig {
            name: "very_dense_notes",
            file_count: 50,
            tasks_per_file: 10,
            note_density: 1.0,
            notes_per_task: 20,
        },
    ];

    for config in &configs {
        let test_project = create_test_project(*config);
        let project_root = test_project.path().to_path_buf();
        let db_path = project_root.join(".lash").join("db.sqlite");
        fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(config.name), &config, |b, _| {
            b.iter(|| {
                let _ = fs::remove_file(&db_path);
                let conn = init_database(&db_path).unwrap();
                let indexer_config = IndexerConfig::new(project_root.clone())
                    .with_incremental(false)
                    .with_progress(false);
                let parser_config = LashConfig::default();
                let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);

                black_box(indexer.index_project().unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark baseline vs notes comparison
fn bench_baseline_vs_notes(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_vs_notes");

    let file_count = 100;
    let tasks_per_file = 5;

    // Baseline: no notes
    let baseline_config = ProjectConfig {
        name: "baseline",
        file_count,
        tasks_per_file,
        note_density: 0.0,
        notes_per_task: 0,
    };

    let test_project_baseline = create_test_project(baseline_config);
    let project_root_baseline = test_project_baseline.path().to_path_buf();
    let db_path_baseline = project_root_baseline.join(".lash").join("db.sqlite");
    fs::create_dir_all(db_path_baseline.parent().unwrap()).unwrap();

    group.bench_function("baseline_no_notes", |b| {
        b.iter(|| {
            let _ = fs::remove_file(&db_path_baseline);
            let conn = init_database(&db_path_baseline).unwrap();
            let indexer_config = IndexerConfig::new(project_root_baseline.clone())
                .with_incremental(false)
                .with_progress(false);
            let parser_config = LashConfig::default();
            let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);

            black_box(indexer.index_project().unwrap());
        });
    });

    // With notes (50% density)
    let notes_config = ProjectConfig {
        name: "with_notes",
        file_count,
        tasks_per_file,
        note_density: 0.5,
        notes_per_task: 3,
    };

    let test_project_notes = create_test_project(notes_config);
    let project_root_notes = test_project_notes.path().to_path_buf();
    let db_path_notes = project_root_notes.join(".lash").join("db.sqlite");
    fs::create_dir_all(db_path_notes.parent().unwrap()).unwrap();

    group.bench_function("with_notes_50pct", |b| {
        b.iter(|| {
            let _ = fs::remove_file(&db_path_notes);
            let conn = init_database(&db_path_notes).unwrap();
            let indexer_config = IndexerConfig::new(project_root_notes.clone())
                .with_incremental(false)
                .with_progress(false);
            let parser_config = LashConfig::default();
            let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);

            black_box(indexer.index_project().unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_full_indexing_notes,
    bench_incremental_note_changes,
    bench_extreme_note_density,
    bench_baseline_vs_notes,
);
criterion_main!(benches);
