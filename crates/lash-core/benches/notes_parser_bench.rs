//! Benchmarks for parsing contextual notes
//!
//! These benchmarks measure the impact of contextual notes on parser performance.
//! We test parsing with varying note densities (0%, 25%, 50%, 75%, 100%) to understand
//! the performance characteristics and ensure notes don't significantly impact parsing speed.
//!
//! Performance targets:
//! - Parsing overhead with notes: <10% increase over baseline
//! - Parser should handle dense note files efficiently
//!
//! Run benchmarks with:
//! ```bash
//! cargo bench --package lash-core --bench notes_parser_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lash_core::parser::parse_file_from_string;
use lash_types::LashConfig;

/// Generate a task file with specified number of tasks and note density
///
/// # Arguments
/// * `num_tasks` - Number of tasks to generate
/// * `note_density` - Percentage of tasks that should have notes (0.0 to 1.0)
/// * `notes_per_task` - Average number of notes per task (when notes are added)
fn generate_file_with_notes(num_tasks: usize, note_density: f64, notes_per_task: usize) -> String {
    let mut content = String::from("# Benchmark Tasks with Notes\n\n");
    content.push_str("@id: notes-benchmark\n");
    content.push_str("@labels: test, benchmark\n\n");
    content.push_str("## Tasks\n\n");

    for i in 1..=num_tasks {
        content.push_str(&format!("- [ ] Task {i}\n"));

        // Add notes based on density
        if (i as f64 / num_tasks as f64) < note_density {
            for j in 1..=notes_per_task {
                content.push_str(&format!(
                    "  - Contextual note {j} for task {i} with some details\n"
                ));
            }
        }
    }

    content
}

/// Generate a nested task file with notes at various levels
fn generate_nested_with_notes(num_top_level: usize, note_density: f64) -> String {
    let mut content = String::from("# Nested Tasks with Notes\n\n");
    content.push_str("@id: nested-notes-benchmark\n\n");
    content.push_str("## Tasks\n\n");

    for i in 1..=num_top_level {
        content.push_str(&format!("- [ ] Parent task {i}\n"));

        // Add notes to parent
        if i % 2 == 0 && note_density > 0.0 {
            content.push_str(&format!("  - Context for parent task {i}\n"));
            content.push_str("  - Additional requirement details\n");
        }

        // Add child tasks
        content.push_str(&format!("  - [ ] Child {i}.1\n"));

        // Add notes to child
        if i % 3 == 0 && note_density > 0.5 {
            content.push_str("    - Implementation note for child\n");
        }

        content.push_str(&format!("  - [ ] Child {i}.2\n"));
    }

    content
}

/// Generate a realistic task file with mixed notes
fn generate_realistic_with_notes() -> String {
    r#"# Backend API Development

@id: backend-api-notes
@owner: alice
@labels: backend, api, database
@created: 2025-01-10

Implementation plan for backend API with detailed notes.

## Tasks

- [x] Set up project structure
  - Initialize Cargo workspace
  - Configure dependencies and features
  - Set up directory structure
  - [x] Initialize Cargo project
  - [x] Add dependencies
  - [x] Configure project settings
- [ ] Design database schema #database
  - Use PostgreSQL for primary storage
  - Ensure proper indexing for performance
  - Consider partitioning for large tables
  - [x] Define user model
  - [ ] Define task model
  - [ ] Define project model #important
    - Include timestamps for auditing
    - Add soft delete support
  - [ ] Add migrations
- [ ] Implement authentication #security
  - Use JWT with RS256 algorithm
  - Token expiry: 1 hour for access, 7 days for refresh
  - Store refresh tokens in secure HTTP-only cookies
  - [x] Add JWT library
  - [x] Create login endpoint
  - [ ] Create registration endpoint
    - Validate email format
    - Check password strength (min 8 chars, mixed case, numbers)
    - Send verification email
  - [ ] Add password hashing
  - [ ] Implement token refresh #important
- [ ] Build CRUD endpoints #api
  - [ ] Users endpoint
    - Support pagination with cursor-based approach
    - Add filtering by role, status, creation date
  - [ ] Tasks endpoint
  - [ ] Projects endpoint
- [ ] Add validation layer #quality
  - Use serde for deserialization validation
  - Add custom validators for business rules
  - [ ] Input validation
  - [ ] Error handling
  - [ ] Response formatting
- [ ] Write tests #testing
  - Aim for >80% coverage
  - [ ] Unit tests
  - [ ] Integration tests
  - [ ] E2E tests
"#
    .to_string()
}

/// Benchmark parsing files with different note densities
fn bench_note_density(c: &mut Criterion) {
    let mut group = c.benchmark_group("note_density");
    let config = LashConfig::default();

    let num_tasks = 100;
    let notes_per_task = 3;

    // Test 0%, 25%, 50%, 75%, 100% note density
    for density_pct in [0, 25, 50, 75, 100] {
        let density = density_pct as f64 / 100.0;
        let content = generate_file_with_notes(num_tasks, density, notes_per_task);
        let bytes = content.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("density_pct", density_pct),
            &content,
            |b, content| {
                b.iter(|| {
                    let result = parse_file_from_string(black_box(content), &config);
                    black_box(result).unwrap()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parsing with varying numbers of notes per task
fn bench_notes_per_task(c: &mut Criterion) {
    let mut group = c.benchmark_group("notes_per_task");
    let config = LashConfig::default();

    let num_tasks = 50;
    let density = 1.0; // 100% of tasks have notes

    for notes_per_task in [1, 3, 5, 10, 20] {
        let content = generate_file_with_notes(num_tasks, density, notes_per_task);
        let bytes = content.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("notes", notes_per_task),
            &content,
            |b, content| {
                b.iter(|| {
                    let result = parse_file_from_string(black_box(content), &config);
                    black_box(result).unwrap()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parsing nested tasks with notes
fn bench_nested_with_notes(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_with_notes");
    let config = LashConfig::default();

    for size in [10, 25, 50, 100] {
        // Baseline: no notes
        let content_no_notes = generate_nested_with_notes(size, 0.0);
        group.bench_with_input(
            BenchmarkId::new("no_notes", size),
            &content_no_notes,
            |b, content| {
                b.iter(|| {
                    let result = parse_file_from_string(black_box(content), &config);
                    black_box(result).unwrap()
                });
            },
        );

        // With notes
        let content_with_notes = generate_nested_with_notes(size, 1.0);
        group.bench_with_input(
            BenchmarkId::new("with_notes", size),
            &content_with_notes,
            |b, content| {
                b.iter(|| {
                    let result = parse_file_from_string(black_box(content), &config);
                    black_box(result).unwrap()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parsing realistic file with notes
fn bench_realistic_with_notes(c: &mut Criterion) {
    let config = LashConfig::default();
    let content = generate_realistic_with_notes();

    c.bench_function("realistic_with_notes", |b| {
        b.iter(|| {
            let result = parse_file_from_string(black_box(&content), &config);
            black_box(result).unwrap()
        });
    });
}

/// Benchmark comparison: baseline vs notes
fn bench_baseline_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_comparison");
    let config = LashConfig::default();

    let num_tasks = 100;

    // Baseline: no notes
    let baseline = generate_file_with_notes(num_tasks, 0.0, 0);
    group.bench_function("baseline_no_notes", |b| {
        b.iter(|| {
            let result = parse_file_from_string(black_box(&baseline), &config);
            black_box(result).unwrap()
        });
    });

    // With notes (50% density, 3 notes per task)
    let with_notes = generate_file_with_notes(num_tasks, 0.5, 3);
    group.bench_function("with_notes_50pct", |b| {
        b.iter(|| {
            let result = parse_file_from_string(black_box(&with_notes), &config);
            black_box(result).unwrap()
        });
    });

    // Heavy notes (100% density, 10 notes per task)
    let heavy_notes = generate_file_with_notes(num_tasks, 1.0, 10);
    group.bench_function("heavy_notes_100pct", |b| {
        b.iter(|| {
            let result = parse_file_from_string(black_box(&heavy_notes), &config);
            black_box(result).unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_note_density,
    bench_notes_per_task,
    bench_nested_with_notes,
    bench_realistic_with_notes,
    bench_baseline_comparison,
);
criterion_main!(benches);
