//! Integration tests for the full file parser
//!
//! These tests verify that the parser correctly integrates all phases:
//! - Header parsing
//! - Task tree construction
//! - Metadata extraction
//! - Hash computation
//! - Error collection

use lash_core::parser::parse_file_from_string;
use lash_types::{LashConfig, TaskStatus};

// Helper to create default config
fn default_config() -> LashConfig {
    LashConfig::default()
}

// ==================== Valid File Tests ====================

#[test]
fn test_parse_complete_valid_file() {
    let content = r"# Backend API Tasks

@id: backend-api
@owner: alice
@labels: backend, api
@status: in-progress
@created: 2025-01-15

This file tracks all backend API development tasks.
Focus areas include authentication, data models, and endpoints.

## Tasks

- [ ] Design API schema
  - [ ] Define user model
  - [ ] Define task model
- [x] Set up project structure
- [ ] Implement authentication #security
  - [x] Add JWT library
  - [ ] Create login endpoint

## References

- See also: frontend-tasks.md
- Documentation: https://example.com/api-docs
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok(), "Expected successful parse, got: {result:?}");
    let file = result.unwrap();

    // Verify file metadata
    assert_eq!(file.id, "backend-api");
    assert_eq!(file.title, "Backend API Tasks");
    assert_eq!(file.metadata.owner, Some("alice".to_string()));
    assert_eq!(file.metadata.status, Some("in-progress".to_string()));
    assert_eq!(file.metadata.created, Some("2025-01-15".to_string()));
    // Check labels without caring about order
    assert_eq!(file.metadata.labels.len(), 2);
    assert!(file.metadata.labels.contains(&"backend".to_string()));
    assert!(file.metadata.labels.contains(&"api".to_string()));

    // Verify tasks were parsed
    let all_tasks = file.tasks.tasks();
    assert_eq!(
        all_tasks.len(),
        7,
        "Expected 7 tasks, got {}",
        all_tasks.len()
    );

    // Verify task hierarchy
    let top_level: Vec<_> = all_tasks.iter().filter(|t| t.depth == 0).collect();
    assert_eq!(top_level.len(), 3, "Expected 3 top-level tasks");

    // Verify task content
    assert_eq!(top_level[0].title, "Design API schema");
    assert_eq!(top_level[0].status, TaskStatus::Open);

    assert_eq!(top_level[1].title, "Set up project structure");
    assert_eq!(top_level[1].status, TaskStatus::Done);

    assert_eq!(top_level[2].title, "Implement authentication #security");
    assert_eq!(top_level[2].status, TaskStatus::Open);

    // Verify hash was computed
    assert!(!file.hash.is_empty());
}

#[test]
fn test_parse_minimal_valid_file() {
    let content = r"# Simple Tasks

## Tasks

- [ ] Task 1
- [ ] Task 2
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    assert_eq!(file.title, "Simple Tasks");
    assert_eq!(file.tasks.tasks().len(), 2);
    assert_eq!(file.metadata.labels.len(), 0);
}

#[test]
fn test_parse_file_with_only_h1_and_tasks() {
    let content = r"# Title

- [ ] Task without Tasks section
- [ ] Another task
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    assert_eq!(file.title, "Title");
    assert_eq!(file.tasks.tasks().len(), 2);
}

#[test]
fn test_parse_file_without_h1() {
    let content = r"## Tasks

- [ ] Task 1
- [ ] Task 2
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    // Title should be synthesized from filename
    assert_eq!(file.title, "<string>"); // From "<string>" path
    assert_eq!(file.tasks.tasks().len(), 2);
}

#[test]
fn test_parse_file_with_nested_tasks() {
    let content = r"# Nested Tasks

## Tasks

- [ ] Level 0 task 1
  - [ ] Level 1 task 1.1
    - [ ] Level 2 task 1.1.1
    - [ ] Level 2 task 1.1.2
  - [ ] Level 1 task 1.2
- [ ] Level 0 task 2
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    let all_tasks = file.tasks.tasks();
    assert_eq!(all_tasks.len(), 6);

    // Verify depths
    let depths: Vec<u8> = all_tasks.iter().map(|t| t.depth).collect();
    assert_eq!(depths, vec![0, 1, 2, 2, 1, 0]);
}

#[test]
fn test_parse_file_with_all_statuses() {
    let content = r"# Task Statuses

## Tasks

- [ ] Open task
- [x] Done task
- [-] Waived task
- [!] Blocked task
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    let all_tasks = file.tasks.tasks();
    assert_eq!(all_tasks.len(), 4);

    assert_eq!(all_tasks[0].status, TaskStatus::Open);
    assert_eq!(all_tasks[1].status, TaskStatus::Done);
    assert_eq!(all_tasks[2].status, TaskStatus::Waived);
    assert_eq!(all_tasks[3].status, TaskStatus::Blocked);
}

#[test]
fn test_parse_file_with_inline_labels() {
    let content = r"# Tasks with Labels

## Tasks

- [ ] Task with #label1
- [ ] Task with #label1 and #label2
- [ ] Task with #backend #api #security
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    let all_tasks = file.tasks.tasks();
    assert_eq!(all_tasks.len(), 3);

    // Check label presence without caring about order
    assert_eq!(all_tasks[0].metadata.labels.len(), 1);
    assert!(all_tasks[0].metadata.labels.contains(&"label1".to_string()));

    assert_eq!(all_tasks[1].metadata.labels.len(), 2);
    assert!(all_tasks[1].metadata.labels.contains(&"label1".to_string()));
    assert!(all_tasks[1].metadata.labels.contains(&"label2".to_string()));

    assert_eq!(all_tasks[2].metadata.labels.len(), 3);
    assert!(all_tasks[2]
        .metadata
        .labels
        .contains(&"backend".to_string()));
    assert!(all_tasks[2].metadata.labels.contains(&"api".to_string()));
    assert!(all_tasks[2]
        .metadata
        .labels
        .contains(&"security".to_string()));
}

#[test]
fn test_parse_file_with_empty_tasks_section() {
    let content = r"# Tasks

@id: empty-file

## Tasks

## References

Some references here.
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    assert_eq!(file.tasks.tasks().len(), 0);
    assert_eq!(file.id, "empty-file");
}

#[test]
fn test_parse_file_with_custom_annotations() {
    let content = r"# Custom Annotations

@id: custom-1
@priority: high
@team: backend
@sprint: 2025-Q1

## Tasks

- [ ] Task 1
";

    let config = LashConfig {
        custom_annotation_keys: vec![
            "priority".to_string(),
            "team".to_string(),
            "sprint".to_string(),
        ],
        ..default_config()
    };

    let result = parse_file_from_string(content, &config);

    if let Err(ref diagnostics) = result {
        eprintln!("Parse failed with errors:");
        for diag in diagnostics {
            eprintln!("  - {}: {}", diag.code, diag.message);
        }
    }

    assert!(result.is_ok());
    let file = result.unwrap();

    assert_eq!(file.id, "custom-1");
    assert_eq!(
        file.metadata.custom.get("priority"),
        Some(&"high".to_string())
    );
    assert_eq!(
        file.metadata.custom.get("team"),
        Some(&"backend".to_string())
    );
    assert_eq!(
        file.metadata.custom.get("sprint"),
        Some(&"2025-Q1".to_string())
    );
}

// ==================== Error Collection Tests ====================

#[test]
fn test_parse_file_with_depth_limit_error() {
    let content = r"# Deep Tasks

## Tasks

- [ ] Level 0
  - [ ] Level 1
    - [ ] Level 2
      - [ ] Level 3 (exceeds default max depth)
";

    let config = LashConfig {
        max_depth: 2, // Only allow 3 levels (0, 1, 2)
        ..Default::default()
    };

    let result = parse_file_from_string(content, &config);

    // Should fail because depth 3 exceeds max
    assert!(result.is_err(), "Expected error for depth limit violation");
}

#[test]
fn test_parse_file_continues_after_errors() {
    // This test verifies that the parser continues collecting tasks
    // even after encountering errors
    let content = r"# Tasks with Issues

## Tasks

- [ ] Valid task 1
  - [ ] Valid nested task
- [ ] Valid task 2
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    // Even if there are warnings, should still succeed
    assert!(result.is_ok());
}

// ==================== Hash Computation Tests ====================

#[test]
fn test_hash_is_deterministic() {
    let content = r"# Test

## Tasks

- [ ] Task 1
";

    let config = default_config();

    let result1 = parse_file_from_string(content, &config);
    let result2 = parse_file_from_string(content, &config);

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let file1 = result1.unwrap();
    let file2 = result2.unwrap();

    assert_eq!(file1.hash, file2.hash);
    assert!(!file1.hash.is_empty());
}

#[test]
fn test_hash_changes_with_content() {
    let content1 = r"# Test

## Tasks

- [ ] Task 1
";

    let content2 = r"# Test

## Tasks

- [ ] Task 2
";

    let config = default_config();

    let file1 = parse_file_from_string(content1, &config).unwrap();
    let file2 = parse_file_from_string(content2, &config).unwrap();

    assert_ne!(file1.hash, file2.hash);
}

// ==================== File ID Synthesis Tests ====================

#[test]
fn test_file_id_from_annotation() {
    let content = r"# Test

@id: custom-id

## Tasks

- [ ] Task 1
";

    let config = default_config();
    let file = parse_file_from_string(content, &config).unwrap();

    assert_eq!(file.id, "custom-id");
}

#[test]
fn test_file_id_synthesized_when_missing() {
    let content = r"# Test

## Tasks

- [ ] Task 1
";

    let config = default_config();
    let file = parse_file_from_string(content, &config).unwrap();

    // Should be synthesized from path
    assert_eq!(file.id, "<string>");
}

// ==================== Large File Tests ====================

#[test]
fn test_parse_large_file_with_100_tasks() {
    let mut content = String::from("# Large File\n\n## Tasks\n\n");

    // Generate 100 tasks
    for i in 1..=100 {
        content.push_str(&format!("- [ ] Task {i}\n"));
    }

    let config = default_config();
    let result = parse_file_from_string(&content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();
    assert_eq!(file.tasks.tasks().len(), 100);
}

#[test]
fn test_parse_deeply_nested_file() {
    let content = r"# Deep Nesting

## Tasks

- [ ] Level 0
  - [ ] Level 1
    - [ ] Level 2
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    let all_tasks = file.tasks.tasks();
    assert_eq!(all_tasks.len(), 3);
    assert_eq!(all_tasks[0].depth, 0);
    assert_eq!(all_tasks[1].depth, 1);
    assert_eq!(all_tasks[2].depth, 2);
}

// ==================== Graceful Degradation Tests ====================

#[test]
fn test_parse_file_with_non_task_content() {
    let content = r"# Test

This is regular markdown content.

Some more text here.

## Tasks

- [ ] Task 1

Some text between tasks.

- [ ] Task 2
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    // Should parse only the checkbox lines
    assert_eq!(file.tasks.tasks().len(), 2);
}

#[test]
fn test_parse_file_with_malformed_checkboxes() {
    let content = r"# Test

## Tasks

- [ ] Valid task
- [X] Invalid (wrong case - should work with uppercase)
- [] Empty checkbox
not a task line
- [ ] Another valid task
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    // Should parse and skip malformed lines
    assert!(result.is_ok());
    let file = result.unwrap();

    // Should have at least the valid tasks
    assert!(file.tasks.tasks().len() >= 2);
}

// ==================== Round-trip Tests ====================

#[test]
fn test_parse_preserves_task_order() {
    let content = r"# Ordered Tasks

## Tasks

- [ ] Task A
- [ ] Task B
- [ ] Task C
- [ ] Task D
- [ ] Task E
";

    let config = default_config();
    let file = parse_file_from_string(content, &config).unwrap();

    let all_tasks = file.tasks.tasks();
    assert_eq!(all_tasks.len(), 5);

    // Verify order is preserved
    assert_eq!(all_tasks[0].title, "Task A");
    assert_eq!(all_tasks[1].title, "Task B");
    assert_eq!(all_tasks[2].title, "Task C");
    assert_eq!(all_tasks[3].title, "Task D");
    assert_eq!(all_tasks[4].title, "Task E");
}

#[test]
fn test_parse_preserves_parent_child_relationships() {
    let content = r"# Hierarchy

## Tasks

- [ ] Parent 1
  - [ ] Child 1.1
  - [ ] Child 1.2
- [ ] Parent 2
  - [ ] Child 2.1
";

    let config = default_config();
    let file = parse_file_from_string(content, &config).unwrap();

    let all_tasks = file.tasks.tasks();
    assert_eq!(all_tasks.len(), 5);

    // Verify parent-child relationships
    assert_eq!(all_tasks[0].title, "Parent 1");
    assert!(all_tasks[0].parent_id.is_none());

    assert_eq!(all_tasks[1].title, "Child 1.1");
    assert!(all_tasks[1].parent_id.is_some());
    assert_eq!(all_tasks[1].parent_id.as_ref().unwrap(), &all_tasks[0].id);

    assert_eq!(all_tasks[2].title, "Child 1.2");
    assert!(all_tasks[2].parent_id.is_some());
    assert_eq!(all_tasks[2].parent_id.as_ref().unwrap(), &all_tasks[0].id);

    assert_eq!(all_tasks[3].title, "Parent 2");
    assert!(all_tasks[3].parent_id.is_none());

    assert_eq!(all_tasks[4].title, "Child 2.1");
    assert!(all_tasks[4].parent_id.is_some());
    assert_eq!(all_tasks[4].parent_id.as_ref().unwrap(), &all_tasks[3].id);
}

// ==================== Metadata Extraction Tests ====================

#[test]
fn test_extract_file_metadata_with_dependencies() {
    let content = r"# Test

@id: test-1
@depends-on: other-file.md#task:123

## Tasks

- [ ] Task 1
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    assert_eq!(file.metadata.depends_on.len(), 1);
}

#[test]
fn test_extract_file_metadata_with_all_fields() {
    let content = r"# Complete Metadata

@id: complete-1
@labels: backend, api, security
@status: in-progress
@owner: alice
@created: 2025-01-15

## Tasks

- [ ] Task 1
";

    let config = default_config();
    let result = parse_file_from_string(content, &config);

    assert!(result.is_ok());
    let file = result.unwrap();

    assert_eq!(file.id, "complete-1");
    // Check labels without caring about order
    assert_eq!(file.metadata.labels.len(), 3);
    assert!(file.metadata.labels.contains(&"backend".to_string()));
    assert!(file.metadata.labels.contains(&"api".to_string()));
    assert!(file.metadata.labels.contains(&"security".to_string()));
    assert_eq!(file.metadata.status, Some("in-progress".to_string()));
    assert_eq!(file.metadata.owner, Some("alice".to_string()));
    assert_eq!(file.metadata.created, Some("2025-01-15".to_string()));
}
