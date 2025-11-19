//! Benchmarks for the markdown parser
//!
//! These benchmarks measure parser performance on various file sizes and complexities.
//! Target: <100ms for typical files (10-100 tasks).

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lash_core::parser::parse_file_from_string;
use lash_types::LashConfig;

/// Generate a simple task file with N tasks
fn generate_simple_file(num_tasks: usize) -> String {
    let mut content = String::from("# Benchmark Tasks\n\n");
    content.push_str("@id: benchmark\n");
    content.push_str("@labels: test, benchmark\n\n");
    content.push_str("## Tasks\n\n");

    for i in 1..=num_tasks {
        content.push_str(&format!("- [ ] Task {i}\n"));
    }

    content
}

/// Generate a nested task file with N top-level tasks, each with 2 children
fn generate_nested_file(num_top_level: usize) -> String {
    let mut content = String::from("# Nested Benchmark\n\n");
    content.push_str("@id: nested-benchmark\n\n");
    content.push_str("## Tasks\n\n");

    for i in 1..=num_top_level {
        content.push_str(&format!("- [ ] Parent task {i}\n"));
        content.push_str(&format!("  - [ ] Child {i}.1\n"));
        content.push_str(&format!("  - [ ] Child {i}.2\n"));
    }

    content
}

/// Generate a complex file with annotations, labels, and mixed nesting
fn generate_complex_file(num_tasks: usize) -> String {
    let mut content = String::from("# Complex Benchmark Tasks\n\n");
    content.push_str("@id: complex-benchmark\n");
    content.push_str("@owner: benchmark-runner\n");
    content.push_str("@labels: backend, api, security, performance\n");
    content.push_str("@status: in-progress\n");
    content.push_str("@created: 2025-01-15\n\n");
    content.push_str("This is a complex file with multiple annotations and nested structure.\n");
    content.push_str("It simulates a real-world task file.\n\n");
    content.push_str("## Tasks\n\n");

    let mut task_num = 1;
    while task_num <= num_tasks {
        // Top-level task with labels
        content.push_str(&format!("- [ ] Task {task_num} #backend #api\n"));
        task_num += 1;
        if task_num > num_tasks {
            break;
        }

        // Child task
        content.push_str(&format!("  - [ ] Subtask {task_num} #implementation\n"));
        task_num += 1;
        if task_num > num_tasks {
            break;
        }

        // Another top-level task
        content.push_str(&format!("- [x] Done task {task_num}\n"));
        task_num += 1;
        if task_num > num_tasks {
            break;
        }

        // Waived task
        content.push_str(&format!("- [-] Waived task {task_num} #deprecated\n"));
        task_num += 1;
    }

    content.push_str("\n## References\n\n");
    content.push_str("- Related: other-tasks.md\n");
    content.push_str("- Documentation: https://example.com\n");

    content
}

/// Benchmark parsing simple files of various sizes
fn bench_simple_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_files");
    let config = LashConfig::default();

    for size in [10, 50, 100, 500, 1000] {
        let content = generate_simple_file(size);
        let bytes = content.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("tasks", size), &content, |b, content| {
            b.iter(|| {
                let result = parse_file_from_string(black_box(content), &config);
                black_box(result).unwrap()
            });
        });
    }

    group.finish();
}

/// Benchmark parsing nested files
fn bench_nested_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_files");
    let config = LashConfig::default();

    for size in [10, 25, 50, 100, 200] {
        let content = generate_nested_file(size);
        let bytes = content.len();
        let total_tasks = size * 3; // Each top-level has 2 children

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("tasks", total_tasks),
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

/// Benchmark parsing complex files with annotations and mixed structure
fn bench_complex_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_files");
    let config = LashConfig::default();

    for size in [10, 50, 100, 500] {
        let content = generate_complex_file(size);
        let bytes = content.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("tasks", size), &content, |b, content| {
            b.iter(|| {
                let result = parse_file_from_string(black_box(content), &config);
                black_box(result).unwrap()
            });
        });
    }

    group.finish();
}

/// Benchmark header parsing specifically
fn bench_header_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_parsing");
    let config = LashConfig::default();

    // Minimal header
    let minimal = r"# Title

## Tasks

- [ ] Task 1
";
    group.bench_function("minimal", |b| {
        b.iter(|| {
            let result = parse_file_from_string(black_box(minimal), &config);
            black_box(result).unwrap()
        });
    });

    // Header with annotations
    let with_annotations = r"# Title

@id: test
@owner: alice
@labels: backend, api, security
@status: in-progress
@created: 2025-01-15

## Tasks

- [ ] Task 1
";
    group.bench_function("with_annotations", |b| {
        b.iter(|| {
            let result = parse_file_from_string(black_box(with_annotations), &config);
            black_box(result).unwrap()
        });
    });

    // Header with overview
    let with_overview = r"# Title

@id: test
@labels: backend

This is a long overview section that provides context about the file.
It can span multiple paragraphs and contain detailed information.

The parser needs to handle this efficiently.

## Tasks

- [ ] Task 1
";
    group.bench_function("with_overview", |b| {
        b.iter(|| {
            let result = parse_file_from_string(black_box(with_overview), &config);
            black_box(result).unwrap()
        });
    });

    group.finish();
}

/// Benchmark hash computation (should be fast)
fn bench_hash_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_computation");

    for size in [100, 1000, 10000] {
        let content = "x".repeat(size);

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("bytes", size), &content, |b, content| {
            b.iter(|| {
                use lash_types::file::compute_hash;
                black_box(compute_hash(black_box(content)))
            });
        });
    }

    group.finish();
}

/// Realistic workload: typical task file
fn bench_realistic_file(c: &mut Criterion) {
    let content = r"# Backend API Development

@id: backend-api
@owner: alice
@labels: backend, api, database
@status: in-progress
@created: 2025-01-10
@depends-on: auth-service.md#task:setup

This file tracks the backend API development tasks for the project.
We're building a RESTful API with authentication, data models, and CRUD operations.

## Tasks

- [x] Set up project structure
  - [x] Initialize Cargo project
  - [x] Add dependencies
  - [x] Configure project settings
- [ ] Design database schema #database
  - [x] Define user model
  - [ ] Define task model
  - [ ] Define project model #important
  - [ ] Add migrations
- [ ] Implement authentication #security
  - [x] Add JWT library
  - [x] Create login endpoint
  - [ ] Create registration endpoint
  - [ ] Add password hashing
  - [ ] Implement token refresh #important
- [ ] Build CRUD endpoints #api
  - [ ] Users endpoint
  - [ ] Tasks endpoint
  - [ ] Projects endpoint
- [ ] Add validation layer #quality
  - [ ] Input validation
  - [ ] Error handling
  - [ ] Response formatting
- [ ] Write tests #testing
  - [ ] Unit tests
  - [ ] Integration tests
  - [ ] E2E tests
- [ ] Documentation #docs
  - [ ] API documentation
  - [ ] Setup guide
  - [ ] Deployment guide

## References

- Architecture doc: docs/architecture.md
- API spec: docs/api-spec.md
- Database schema: docs/schema.sql
";

    let config = LashConfig::default();

    c.bench_function("realistic_file", |b| {
        b.iter(|| {
            let result = parse_file_from_string(black_box(content), &config);
            black_box(result).unwrap()
        });
    });
}

criterion_group!(
    benches,
    bench_simple_files,
    bench_nested_files,
    bench_complex_files,
    bench_header_parsing,
    bench_hash_computation,
    bench_realistic_file
);
criterion_main!(benches);
