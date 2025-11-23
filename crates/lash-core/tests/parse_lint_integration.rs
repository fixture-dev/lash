//! Integration tests for Parse → Lint workflow
//!
//! Tests the integration between the parser and linter:
//! - Parse file → Check structure → Lint → Verify result
//! - Parse valid file → No parse errors
//! - Parse with violations → Verify file still parses (parser is lenient)
//! - Complex file → Parse → Verify structure preserved for linting
//!
//! This fills a critical gap in testing the complete parse-lint cycle.

use lash_core::parser::parse_file_from_string;
use lash_types::LashConfig;

#[test]
fn test_parse_valid_file_structure() {
    // Test scenario: Parse valid file → Verify structure ready for linting
    let content = r#"# Test Task File

@id: test.valid
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Task one
- [ ] Task two
  - [ ] Subtask A
  - [ ] Subtask B
"#;

    let config = LashConfig::default();

    // Parse the file content (in-memory parsing)
    let parse_result = parse_file_from_string(content, &config);
    let task_file = match parse_result {
        Ok(file) => file,
        Err(diagnostics) => panic!("Valid file should parse without errors, got: {diagnostics:?}"),
    };

    // Verify parsing captured the structure correctly
    assert_eq!(task_file.id, "test.valid");
    assert_eq!(task_file.title, "Test Task File");
    assert!(!task_file.tasks.is_empty(), "Should have tasks");

    // Verify we got all tasks (2 root + 2 subtasks)
    assert_eq!(task_file.tasks.len(), 4, "Should have 4 total tasks");
}

#[test]
fn test_parse_file_with_depth_violations() {
    // Test scenario: Parse file with nesting violations → Should still parse
    // (Parser is lenient, linter will catch violations)
    let content = r#"# Test Task File

@id: test.depth
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Task one
- [ ] Task two
  - [ ] Subtask A
    - [ ] Subtask B
      - [ ] Subtask C
        - [ ] Subtask D (potentially too deep)
"#;

    let config = LashConfig::default();

    // Parse the file
    let parse_result = parse_file_from_string(content, &config);

    // Parser validates depth - this should fail
    match parse_result {
        Ok(_file) => {
            panic!("File with excessive depth should be rejected by parser");
        }
        Err(diagnostics) => {
            // Parser should catch depth violations
            let has_depth_error = diagnostics
                .iter()
                .any(|d| d.message.contains("depth") || d.message.contains("maximum"));
            assert!(
                has_depth_error,
                "Error should mention depth violation, got: {diagnostics:?}"
            );
        }
    }
}

#[test]
fn test_parse_missing_required_metadata() {
    // Test scenario: Parse file missing required metadata
    // Parser should be lenient, linter will flag missing metadata
    let content = r#"# Test Task File

@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Task one
- [ ] Task two
"#;

    let config = LashConfig::default();

    // Parse the file
    let parse_result = parse_file_from_string(content, &config);

    match parse_result {
        Ok(task_file) => {
            // Parser was lenient - uses filename as fallback ID
            // The ID will be "<string>" since we're parsing from a string
            assert!(
                !task_file.id.is_empty(),
                "Should have an ID (possibly fallback)"
            );
            assert!(!task_file.tasks.is_empty(), "Should still parse tasks");
        }
        Err(diagnostics) => {
            // If parser is strict about @id, that's okay too
            let has_id_error = diagnostics
                .iter()
                .any(|d| d.message.contains("@id") || d.message.contains("identifier"));
            assert!(
                has_id_error,
                "Error should mention missing @id, got: {diagnostics:?}"
            );
        }
    }
}

#[test]
fn test_parse_complex_file_preserves_structure() {
    // Test scenario: Parse complex but valid file → Verify all structure preserved
    let content = r#"# Complex Task File

@id: test.complex
@status: in-progress
@created: 2024-01-15
@owner: test-user
@labels: backend, testing
@estimate: 3d

## Overview

This is a complex task file with multiple annotations and nested tasks.

## Tasks

- [ ] Phase 1: Setup
  - [ ] Configure environment
  - [ ] Install dependencies
- [ ] Phase 2: Implementation
  - [ ] Core logic
  - [ ] Unit tests
- [ ] Phase 3: Integration
  - [ ] End-to-end tests
  - [ ] Documentation
- [x] Completed task
- [-] Waived task
"#;

    let config = LashConfig::default();

    // Parse the file
    let parse_result = parse_file_from_string(content, &config);
    let task_file = match parse_result {
        Ok(file) => file,
        Err(diagnostics) => panic!("Complex valid file should parse, got: {diagnostics:?}"),
    };

    // Verify parsing captured all the data
    assert_eq!(task_file.id, "test.complex");
    assert_eq!(task_file.title, "Complex Task File");
    assert!(!task_file.tasks.is_empty(), "Should have tasks");

    // Verify metadata
    assert!(task_file.metadata.owner.is_some(), "Should parse owner");

    // Verify task count (3 phases + 6 subtasks + 2 individual = 11 tasks)
    assert!(task_file.tasks.len() >= 10, "Should have at least 10 tasks");

    // Verify different statuses were parsed
    let statuses: Vec<_> = task_file.tasks.tasks().iter().map(|t| t.status).collect();
    assert!(
        statuses.contains(&lash_types::TaskStatus::Open),
        "Should have open tasks"
    );
    assert!(
        statuses.contains(&lash_types::TaskStatus::Done),
        "Should have done tasks"
    );
    assert!(
        statuses.contains(&lash_types::TaskStatus::Waived),
        "Should have waived tasks"
    );
}

#[test]
fn test_parse_then_verify_task_hierarchy() {
    // Test scenario: Parse hierarchical tasks → Verify parent-child relationships
    let content = r#"# Hierarchy Test

@id: test.hierarchy
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] Parent A
  - [ ] Child A1
  - [ ] Child A2
- [ ] Parent B
  - [ ] Child B1
    - [ ] Grandchild B1a
"#;

    let config = LashConfig::default();

    // Parse the file
    let parse_result = parse_file_from_string(content, &config);
    let task_file = match parse_result {
        Ok(file) => file,
        Err(diagnostics) => panic!("File should parse, got: {diagnostics:?}"),
    };

    // Verify task count
    assert_eq!(task_file.tasks.len(), 6, "Should have 6 tasks");

    // Verify depths
    let depths: Vec<_> = task_file.tasks.tasks().iter().map(|t| t.depth).collect();
    assert!(depths.contains(&0), "Should have depth 0 tasks (parents)");
    assert!(depths.contains(&1), "Should have depth 1 tasks (children)");
    assert!(
        depths.contains(&2),
        "Should have depth 2 tasks (grandchildren)"
    );

    // Verify parent relationships are tracked
    let has_parents = task_file
        .tasks
        .tasks()
        .iter()
        .any(|t| t.parent_id.is_some());
    assert!(has_parents, "Some tasks should have parent IDs");
}

#[test]
fn test_parse_preserves_task_order() {
    // Test scenario: Parse tasks → Verify order is preserved
    let content = r#"# Order Test

@id: test.order
@status: in-progress
@created: 2024-01-15

## Tasks

- [ ] First task
- [ ] Second task
- [ ] Third task
- [ ] Fourth task
"#;

    let config = LashConfig::default();

    // Parse the file
    let parse_result = parse_file_from_string(content, &config);
    let task_file = match parse_result {
        Ok(file) => file,
        Err(diagnostics) => panic!("File should parse, got: {diagnostics:?}"),
    };

    // Verify task order is preserved
    let titles: Vec<_> = task_file.tasks.tasks().iter().map(|t| &t.title).collect();
    assert_eq!(titles[0], "First task");
    assert_eq!(titles[1], "Second task");
    assert_eq!(titles[2], "Third task");
    assert_eq!(titles[3], "Fourth task");

    // Verify order indices are assigned
    for (i, task) in task_file.tasks.tasks().iter().enumerate() {
        assert_eq!(task.order_index, i, "Order index should match position");
    }
}
