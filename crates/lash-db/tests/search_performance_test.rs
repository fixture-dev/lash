//! Performance validation tests for search
//!
//! These tests verify that search meets basic performance targets.
//! They are less rigorous than the benchmarks in benches/search_bench.rs
//! but provide a quick sanity check during development.

use lash_db::{
    indexer::{Indexer, IndexerConfig},
    init_database, search_with_profiling, SearchQuery,
};
use lash_types::LashConfig;
use std::fs;
use std::time::Instant;
use tempfile::TempDir;

/// Generate a markdown task file
fn generate_task_file(file_id: &str, task_count: usize) -> String {
    let mut content =
        format!("# Task File {file_id}\n\n@id: {file_id}\n@status: in-progress\n\n## Tasks\n\n");

    for i in 0..task_count {
        content.push_str(&format!(
            "- [ ] Implement task {} for component {}\n",
            i + 1,
            file_id
        ));
    }

    content
}

/// Create a test project with the specified number of files
fn create_test_project(file_count: usize, tasks_per_file: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    for i in 0..file_count {
        let file_path = root.join(format!("task_{i:04}.md"));
        let file_id = format!("task_{i:04}");
        let content = generate_task_file(&file_id, tasks_per_file);
        fs::write(&file_path, content).unwrap();
    }

    // Create index file
    let index_content = "# Project Index\n\n@id: index\n\n## Tasks\n\n- [ ] Root task\n";
    fs::write(root.join("lash.index.md"), index_content).unwrap();

    temp_dir
}

/// Setup and index a test project
fn setup_project(file_count: usize, tasks_per_file: usize) -> (TempDir, rusqlite::Connection) {
    let temp_dir = create_test_project(file_count, tasks_per_file);
    let project_root = temp_dir.path().to_path_buf();
    let db_path = project_root.join(".lash").join("db.sqlite");
    fs::create_dir_all(db_path.parent().unwrap()).unwrap();

    let conn = init_database(&db_path).unwrap();
    let config = IndexerConfig::new(project_root.clone())
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, config, &parser_config);
    indexer.index_project().unwrap();

    (temp_dir, conn)
}

#[test]
fn test_small_project_performance() {
    // Small project: ~100 tasks
    let (_temp, conn) = setup_project(20, 5);

    let query = SearchQuery::new("implement");

    // Warm up
    let _ = search_with_profiling(&conn, &query, false).unwrap();

    // Measure with profiling
    let start = Instant::now();
    let results = search_with_profiling(&conn, &query, true).unwrap();
    let elapsed = start.elapsed();

    println!(
        "\nSmall project search (100 tasks): {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );

    if let Some(metrics) = &results.metrics {
        println!("  Query execution: {:.2}ms", metrics.query_execution_ms);
        println!("  Scoring: {:.2}ms", metrics.scoring_ms);
        println!(
            "  Snippet generation: {:.2}ms",
            metrics.snippet_generation_ms
        );
        println!("  Total (instrumented): {:.2}ms", metrics.total_ms);
    }

    // Target: <50ms for small projects
    assert!(
        elapsed.as_millis() < 50,
        "Small project search took {:.2}ms, expected <50ms",
        elapsed.as_secs_f64() * 1000.0
    );
}

#[test]
#[ignore] // Ignored by default as it's slower
fn test_medium_project_performance() {
    // Medium project: ~1000 tasks
    let (_temp, conn) = setup_project(200, 5);

    let query = SearchQuery::new("implement");

    // Warm up
    let _ = search_with_profiling(&conn, &query, false).unwrap();

    // Measure with profiling
    let start = Instant::now();
    let results = search_with_profiling(&conn, &query, true).unwrap();
    let elapsed = start.elapsed();

    println!(
        "\nMedium project search (1000 tasks): {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );

    if let Some(metrics) = &results.metrics {
        println!("  Query execution: {:.2}ms", metrics.query_execution_ms);
        println!("  Scoring: {:.2}ms", metrics.scoring_ms);
        println!(
            "  Snippet generation: {:.2}ms",
            metrics.snippet_generation_ms
        );
        println!("  Total (instrumented): {:.2}ms", metrics.total_ms);
    }

    // Target: <150ms for medium projects
    assert!(
        elapsed.as_millis() < 150,
        "Medium project search took {:.2}ms, expected <150ms",
        elapsed.as_secs_f64() * 1000.0
    );
}

#[test]
fn test_search_metrics_accuracy() {
    let (_temp, conn) = setup_project(20, 5);

    let query = SearchQuery::new("implement");
    let results = search_with_profiling(&conn, &query, true).unwrap();

    assert!(results.metrics.is_some(), "Metrics should be present");

    let metrics = results.metrics.unwrap();

    // Verify that metrics are reasonable
    assert!(
        metrics.total_ms > 0.0,
        "Total time should be greater than 0"
    );
    assert!(
        metrics.query_execution_ms >= 0.0,
        "Query execution time should be non-negative"
    );
    assert!(
        metrics.scoring_ms >= 0.0,
        "Scoring time should be non-negative"
    );
    assert!(
        metrics.snippet_generation_ms >= 0.0,
        "Snippet generation time should be non-negative"
    );

    // The sum of components should be roughly equal to total (within measurement error)
    let component_sum =
        metrics.query_execution_ms + metrics.scoring_ms + metrics.snippet_generation_ms;

    // Allow for some overhead in total time
    assert!(
        component_sum <= metrics.total_ms,
        "Component times should not exceed total time"
    );

    println!("\nSearch metrics breakdown:");
    println!("  Total: {:.2}ms", metrics.total_ms);
    println!("  Query: {:.2}ms", metrics.query_execution_ms);
    println!("  Scoring: {:.2}ms", metrics.scoring_ms);
    println!("  Snippets: {:.2}ms", metrics.snippet_generation_ms);
    println!("  Overhead: {:.2}ms", metrics.total_ms - component_sum);
}
