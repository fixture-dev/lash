//! Benchmarks for the linter
//!
//! These benchmarks measure linter performance on various file sizes and error scenarios.
//! Target: >500 tasks/sec (equivalent to <2ms per task file with 10 tasks).
//!
//! Note: These benchmarks test end-to-end parse + lint performance since
//! the linter operates on parsed task files.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lash_core::{
    linter::{LintConfig, LintDiagnostic, Linter},
    parser::parse_file,
};
use lash_types::LashConfig;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Generate a valid task file for linting
fn generate_valid_file(num_tasks: usize) -> String {
    let mut content = String::from("# Valid Tasks\n\n");
    content.push_str("@id: valid-file\n");
    content.push_str("@labels: test, benchmark\n\n");
    content.push_str("## Tasks\n\n");

    for i in 1..=num_tasks {
        content.push_str(&format!("- [ ] Task {i}\n"));
    }

    content
}

/// Generate a file with depth violations (nesting beyond max depth)
fn generate_deeply_nested_file(num_top_level: usize) -> String {
    let mut content = String::from("# Deeply Nested Tasks\n\n");
    content.push_str("@id: nested-file\n\n");
    content.push_str("## Tasks\n\n");

    for i in 1..=num_top_level {
        content.push_str(&format!("- [ ] Level 1 Task {i}\n"));
        content.push_str(&format!("  - [ ] Level 2 Task {i}.1\n"));
        content.push_str(&format!("    - [ ] Level 3 Task {i}.1.1\n"));
        content.push_str(&format!("      - [ ] Level 4 Task {i}.1.1.1\n"));
        content.push_str(&format!(
            "        - [ ] Level 5 Task {i}.1.1.1.1 (too deep)\n"
        ));
    }

    content
}

/// Generate a complex file with mixed content
fn generate_complex_file(num_tasks: usize) -> String {
    let mut content = String::from("# Complex File\n\n");
    content.push_str("@id: complex\n");
    content.push_str("@labels: backend, api\n\n");
    content.push_str("This file has complex structure with various task types.\n\n");
    content.push_str("## Tasks\n\n");

    for i in 1..=num_tasks {
        if i % 4 == 0 {
            content.push_str(&format!("- [x] Completed task {i}\n"));
        } else if i % 4 == 1 {
            content.push_str(&format!("- [ ] Open task {i}\n"));
            content.push_str(&format!("  - [ ] Subtask {i}.1\n"));
        } else if i % 4 == 2 {
            content.push_str(&format!("- [-] Waived task {i}\n"));
        } else {
            content.push_str(&format!("- [ ] Task {i} #label\n"));
        }
    }

    content
}

/// Helper to parse and lint a file
fn parse_and_lint(content: &str, lint_config: &LintConfig) -> Vec<LintDiagnostic> {
    // Create temp file
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Parse the file
    let path = PathBuf::from(temp_file.path());
    let parser_config = LashConfig::default();

    // Parse may fail for files with depth violations - handle gracefully
    match parse_file(&path, &parser_config) {
        Ok(parsed) => {
            // Lint the parsed file
            let linter = Linter::new(lint_config.clone());
            linter.lint_file(&parsed, &parser_config)
        }
        Err(_e) => {
            // Parse failed (e.g., depth violations) - return empty diagnostics
            // since we're measuring linting performance, not parse error handling
            vec![]
        }
    }
}

/// Benchmark linting valid files (no errors)
fn bench_lint_valid_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("lint_valid_files");
    let config = LintConfig::default();

    for size in [10, 50, 100, 500] {
        let content = generate_valid_file(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("tasks", size), &size, |b, _| {
            b.iter(|| {
                let diagnostics = parse_and_lint(black_box(&content), black_box(&config));
                black_box(diagnostics)
            });
        });
    }

    group.finish();
}

/// Benchmark linting files with depth violations
fn bench_lint_depth_violations(c: &mut Criterion) {
    let mut group = c.benchmark_group("lint_depth_violations");
    let config = LintConfig::default();

    for size in [10, 25, 50, 100] {
        let content = generate_deeply_nested_file(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("tasks", size), &size, |b, _| {
            b.iter(|| {
                let diagnostics = parse_and_lint(black_box(&content), black_box(&config));
                black_box(diagnostics)
            });
        });
    }

    group.finish();
}

/// Benchmark linting complex files
fn bench_lint_complex_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("lint_complex_files");
    let config = LintConfig::default();

    for size in [10, 50, 100, 500] {
        let content = generate_complex_file(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("tasks", size), &size, |b, _| {
            b.iter(|| {
                let diagnostics = parse_and_lint(black_box(&content), black_box(&config));
                black_box(diagnostics)
            });
        });
    }

    group.finish();
}

/// Benchmark realistic linting workload
fn bench_realistic_lint(c: &mut Criterion) {
    let content = r"# Backend API Development

@id: backend-api
@owner: alice
@labels: backend, api, database
@status: in-progress
@created: 2025-01-10

This file tracks the backend API development tasks for the project.

## Tasks

- [ ] Set up project structure
  - [ ] Initialize Cargo project
  - [ ] Add dependencies
  - [ ] Configure project settings
- [ ] Design database schema #database
  - [ ] Define user model
  - [ ] Define task model
  - [ ] Define project model #important
  - [ ] Add migrations
- [ ] Implement authentication #security
  - [ ] Add JWT library
  - [ ] Create login endpoint
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
";

    let config = LintConfig::default();

    c.bench_function("realistic_lint", |b| {
        b.iter(|| {
            let diagnostics = parse_and_lint(black_box(content), black_box(&config));
            black_box(diagnostics)
        });
    });
}

criterion_group!(
    benches,
    bench_lint_valid_files,
    bench_lint_depth_violations,
    bench_lint_complex_files,
    bench_realistic_lint
);
criterion_main!(benches);
