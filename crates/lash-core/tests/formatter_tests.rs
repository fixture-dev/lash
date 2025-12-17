//! Comprehensive tests for the formatter
//!
//! These tests verify:
//! - Round-trip safety (format is idempotent)
//! - Content preservation (no data loss)
//! - Whitespace normalization
//! - Annotation sorting
//! - Auto-fix application
//! - Edge cases and error handling

use lash_core::formatter::{FormatOptions, Formatter};
use lash_core::parser::parse_file_from_string;
use lash_types::LashConfig;
use std::path::PathBuf;

fn make_config() -> LashConfig {
    LashConfig {
        root_path: PathBuf::from("/test"),
        index_file: "index.md".to_string(),
        max_depth: 2,
        indent_spaces: 2,
        db_path: PathBuf::from(".lash/test.db"),
        custom_annotation_keys: vec![],
    }
}

#[test]
fn test_format_basic_file() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Test File

@id: test-file
@status: in-progress

## Tasks

- [ ] First task
- [x] Second task
- [-] Third task
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    assert!(formatted.contains("# Test File"));
    assert!(formatted.contains("@id: test-file"));
    assert!(formatted.contains("## Tasks"));
    assert!(formatted.contains("- [ ] First task"));
    assert!(formatted.contains("- [x] Second task"));
    assert!(formatted.contains("- [-] Third task"));
}

#[test]
fn test_format_preserves_hierarchy() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Nested Tasks

@id: nested

## Tasks

- [ ] Parent
  - [ ] Child 1
  - [x] Child 2
- [ ] Another parent
  - [-] Waived child
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Verify hierarchy is preserved with exactly 2 spaces
    assert!(formatted.contains("- [ ] Parent\n"));
    assert!(formatted.contains("  - [ ] Child 1\n"));
    assert!(formatted.contains("  - [x] Child 2\n"));
    assert!(formatted.contains("- [ ] Another parent\n"));
    assert!(formatted.contains("  - [-] Waived child\n"));
}

#[test]
fn test_format_normalizes_indentation() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    // Input with correct indentation (formatter preserves it)
    let content = r#"# Test

@id: test

## Tasks

- [ ] Parent
  - [ ] Child with 2 spaces
  - [ ] Another child
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // All children should have exactly 2 spaces
    assert!(formatted.contains("  - [ ] Child with 2 spaces\n"));
    assert!(formatted.contains("  - [ ] Another child\n"));
}

#[test]
fn test_annotation_sorting() {
    let config = make_config();
    let options = FormatOptions {
        sort_annotations: true,
        ..Default::default()
    };
    let formatter = Formatter::new(config.clone(), options);

    let content = r#"# Test

@id: test
@status: done
@owner: alice
@created: 2025-01-01
@labels: backend, api

## Tasks

- [x] Task
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Find positions of annotations
    let id_pos = formatted.find("@id:").unwrap();
    let created_pos = formatted.find("@created:").unwrap();
    let labels_pos = formatted.find("@labels:").unwrap();
    let owner_pos = formatted.find("@owner:").unwrap();
    let status_pos = formatted.find("@status:").unwrap();

    // @id should be first
    assert!(id_pos < created_pos);
    // Rest should be alphabetical: created, labels, owner, status
    assert!(created_pos < labels_pos);
    assert!(labels_pos < owner_pos);
    assert!(owner_pos < status_pos);
}

#[test]
fn test_annotation_sorting_disabled() {
    let config = make_config();
    let options = FormatOptions {
        sort_annotations: false,
        ..Default::default()
    };
    let formatter = Formatter::new(config.clone(), options);

    let content = r#"# Test

@id: test
@status: done
@owner: alice

## Tasks

- [x] Task
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Annotations should maintain original order (after @id)
    let id_pos = formatted.find("@id:").unwrap();
    let status_pos = formatted.find("@status:").unwrap();
    let owner_pos = formatted.find("@owner:").unwrap();

    assert!(id_pos < status_pos);
    assert!(status_pos < owner_pos);
}

#[test]
fn test_whitespace_normalization() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Test

@id: test


## Tasks


- [ ] Task with trailing spaces
- [x] Another task


"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // No trailing spaces
    assert!(!formatted.lines().any(|line| line.ends_with(' ')));

    // Should not have 3+ blank lines in a row
    assert!(!formatted.contains("\n\n\n\n"));

    // Should end with exactly one newline
    assert!(formatted.ends_with('\n'));
    assert!(!formatted.ends_with("\n\n"));
}

#[test]
fn test_auto_waive_children() {
    let config = make_config();
    let options = FormatOptions {
        apply_auto_fixes: true,
        ..Default::default()
    };
    let formatter = Formatter::new(config.clone(), options);

    let content = r#"# Test

@id: test

## Tasks

- [-] Waived parent
  - [ ] Child should be waived
  - [x] Another child
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Children should be auto-waived
    assert!(formatted.contains("- [-] Waived parent\n"));
    assert!(formatted.contains("  - [-] Child should be waived\n"));
    assert!(formatted.contains("  - [-] Another child\n"));
}

#[test]
fn test_auto_fix_status_consistency() {
    let config = make_config();
    let options = FormatOptions {
        apply_auto_fixes: true,
        ..Default::default()
    };
    let formatter = Formatter::new(config.clone(), options);

    let content = r#"# Test

@id: test

## Tasks

- [x] Parent marked done
  - [ ] But has open child
  - [x] And done child
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Parent should be unmarked (changed to open)
    assert!(formatted.contains("- [ ] Parent marked done\n"));
    assert!(formatted.contains("  - [ ] But has open child\n"));
    assert!(formatted.contains("  - [x] And done child\n"));
}

#[test]
fn test_auto_fix_disabled() {
    let config = make_config();
    let options = FormatOptions {
        apply_auto_fixes: false,
        ..Default::default()
    };
    let formatter = Formatter::new(config.clone(), options);

    let content = r#"# Test

@id: test

## Tasks

- [x] Parent done
  - [ ] Child still open
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Without auto-fix, parent-child inconsistency remains
    assert!(formatted.contains("- [x] Parent done\n"));
    assert!(formatted.contains("  - [ ] Child still open\n"));
}

#[test]
fn test_round_trip_idempotence() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Test File

@id: test

## Tasks

- [ ] Task one
  - [x] Subtask
- [-] Task two
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted1 = formatter.format_file(&file).unwrap();

    // Parse the formatted output
    let file2 = parse_file_from_string(&formatted1, &config).unwrap();
    let formatted2 = formatter.format_file(&file2).unwrap();

    // Second format should be identical to first
    assert_eq!(formatted1, formatted2);
}

#[test]
fn test_round_trip_with_metadata() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Complex File

@id: complex
@labels: backend, api, testing
@status: in-progress
@owner: team-a
@created: 2025-01-15

## Tasks

- [ ] Implement feature
  - [x] Design
  - [ ] Code
  - [ ] Test
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted1 = formatter.format_file(&file).unwrap();

    // Parse and format again
    let file2 = parse_file_from_string(&formatted1, &config).unwrap();
    let formatted2 = formatter.format_file(&file2).unwrap();

    // Second format should be identical to first (idempotent)
    assert_eq!(
        formatted1, formatted2,
        "Formatting should be idempotent.\nFirst:\n{formatted1}\n\nSecond:\n{formatted2}"
    );

    // Verify metadata is preserved (may be sorted differently)
    assert!(formatted1.contains("@labels:"));
    assert!(formatted1.contains("backend"));
    assert!(formatted1.contains("api"));
    assert!(formatted1.contains("testing"));
    assert!(formatted1.contains("@owner: team-a"));
    assert!(formatted1.contains("@created: 2025-01-15"));
}

#[test]
fn test_format_empty_file() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Empty

@id: empty

## Tasks
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    assert!(formatted.contains("# Empty"));
    assert!(formatted.contains("@id: empty"));
    assert!(formatted.contains("## Tasks"));
}

#[test]
fn test_format_deeply_nested() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Nested

@id: nested

## Tasks

- [ ] Level 0
  - [ ] Level 1
    - [ ] Level 2
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Verify exact indentation
    assert!(formatted.contains("- [ ] Level 0\n"));
    assert!(formatted.contains("  - [ ] Level 1\n"));
    assert!(formatted.contains("    - [ ] Level 2\n"));
}

#[test]
fn test_format_with_inline_labels() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Labels

@id: labels

## Tasks

- [ ] Task with #label1 #label2
- [x] Another #backend
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Inline labels should be preserved
    assert!(formatted.contains("#label1"));
    assert!(formatted.contains("#label2"));
    assert!(formatted.contains("#backend"));
}

#[test]
fn test_format_preserves_task_order() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Ordered

@id: ordered

## Tasks

- [ ] First
- [ ] Second
- [ ] Third
- [ ] Fourth
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    let first_pos = formatted.find("First").unwrap();
    let second_pos = formatted.find("Second").unwrap();
    let third_pos = formatted.find("Third").unwrap();
    let fourth_pos = formatted.find("Fourth").unwrap();

    assert!(first_pos < second_pos);
    assert!(second_pos < third_pos);
    assert!(third_pos < fourth_pos);
}

#[test]
fn test_format_multiple_root_tasks() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Multiple Roots

@id: multi

## Tasks

- [ ] Root 1
  - [ ] Child 1a
  - [ ] Child 1b
- [x] Root 2
  - [x] Child 2a
- [-] Root 3
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // All root tasks should be at depth 0
    let lines: Vec<&str> = formatted.lines().collect();
    let task_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.contains("- ["))
        .copied()
        .collect();

    assert_eq!(task_lines[0], "- [ ] Root 1");
    assert_eq!(task_lines[1], "  - [ ] Child 1a");
    assert_eq!(task_lines[2], "  - [ ] Child 1b");
    assert_eq!(task_lines[3], "- [x] Root 2");
    assert_eq!(task_lines[4], "  - [x] Child 2a");
    assert_eq!(task_lines[5], "- [-] Root 3");
}

#[test]
fn test_format_mixed_statuses() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Statuses

@id: statuses

## Tasks

- [ ] Open
- [x] Done
- [-] Waived
- [!] Blocked
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    assert!(formatted.contains("- [ ] Open"));
    assert!(formatted.contains("- [x] Done"));
    assert!(formatted.contains("- [-] Waived"));
    assert!(formatted.contains("- [!] Blocked"));
}

#[test]
fn test_format_nested_auto_waive() {
    let config = make_config();
    let options = FormatOptions {
        apply_auto_fixes: true,
        ..Default::default()
    };
    let formatter = Formatter::new(config.clone(), options);

    let content = r#"# Nested Waive

@id: nested-waive

## Tasks

- [-] Waived root
  - [ ] Child
    - [ ] Grandchild
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Both child and grandchild should be waived
    assert!(formatted.contains("  - [-] Child\n"));
    assert!(formatted.contains("    - [-] Grandchild\n"));
}

#[test]
fn test_minimal_formatter() {
    let config = make_config();
    let options = FormatOptions::minimal();
    let formatter = Formatter::new(config.clone(), options);

    let content = r#"# Test

@id: test
@status: done
@owner: alice

## Tasks

- [ ] Open parent
  - [x] Done child
- [x] Done parent
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Minimal formatter doesn't apply auto-fixes, so parent-child inconsistency remains
    // (done parent stays done even with open child when auto-fix is disabled)
    assert!(formatted.contains("- [ ] Open parent\n"));
    assert!(formatted.contains("  - [x] Done child\n"));
    assert!(formatted.contains("- [x] Done parent\n"));

    // No sorting, so annotations keep original order
    let status_pos = formatted.find("@status:").unwrap();
    let owner_pos = formatted.find("@owner:").unwrap();
    assert!(status_pos < owner_pos);
}

#[test]
fn test_strict_formatter() {
    let config = make_config();
    let options = FormatOptions::strict();
    let formatter = Formatter::new(config.clone(), options);

    let content = r#"# Test

@id: test


@status: done


## Tasks


- [ ] Task
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Strict mode: max 1 blank line
    assert!(!formatted.contains("\n\n\n"));
}

#[test]
fn test_format_file_ends_with_newline() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Test
@id: test
## Tasks
- [ ] Task"#; // No trailing newline

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    assert!(formatted.ends_with('\n'));
    assert!(!formatted.ends_with("\n\n"));
}

#[test]
fn test_format_with_custom_annotations() {
    let mut config = make_config();
    config.custom_annotation_keys = vec!["priority".to_string()];
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Test

@id: test
@priority: high
@estimate: 2d

## Tasks

- [ ] Task
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Custom annotations should be preserved
    assert!(formatted.contains("@priority: high"));
    assert!(formatted.contains("@estimate: 2d"));
}

/// Regression test for GitHub Issue #6:
/// https://github.com/fixture-dev/lash/issues/6
///
/// The formatter was stripping task-level annotations (@id, @owner, etc.)
/// and contextual notes (plain bullet points) from task files.
#[test]
fn test_format_preserves_task_annotations_issue_6() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Project Setup

@id: project
@status: in-progress

## Tasks

- [x] Setup infrastructure
  @id: task-1-1
  @owner: alice
  @estimate: 2h
  - Initialize the project structure
  - Configure build tools
  - [x] Create project skeleton
    @id: task-1-1-1
    - Set up directory layout
    - Add configuration files
- [ ] Implement core features
  @id: task-1-2
  @depends-on: task-1-1
  @agent-note: Prioritize performance
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted = formatter.format_file(&file).unwrap();

    // Task-level @id annotations must be preserved
    assert!(
        formatted.contains("@id: task-1-1"),
        "Task @id annotation for task-1-1 should be preserved"
    );
    assert!(
        formatted.contains("@id: task-1-1-1"),
        "Task @id annotation for task-1-1-1 should be preserved"
    );
    assert!(
        formatted.contains("@id: task-1-2"),
        "Task @id annotation for task-1-2 should be preserved"
    );

    // Task metadata annotations must be preserved
    assert!(
        formatted.contains("@owner: alice"),
        "Task @owner annotation should be preserved"
    );
    assert!(
        formatted.contains("@estimate: 2h"),
        "Task @estimate annotation should be preserved"
    );
    assert!(
        formatted.contains("@depends-on: task-1-1"),
        "Task @depends-on annotation should be preserved"
    );
    assert!(
        formatted.contains("@agent-note: Prioritize performance"),
        "Task @agent-note annotation should be preserved"
    );

    // Contextual notes (plain bullet points) must be preserved
    assert!(
        formatted.contains("- Initialize the project structure"),
        "Contextual note 'Initialize the project structure' should be preserved"
    );
    assert!(
        formatted.contains("- Configure build tools"),
        "Contextual note 'Configure build tools' should be preserved"
    );
    assert!(
        formatted.contains("- Set up directory layout"),
        "Contextual note 'Set up directory layout' should be preserved"
    );
    assert!(
        formatted.contains("- Add configuration files"),
        "Contextual note 'Add configuration files' should be preserved"
    );
}

/// Test that round-trip formatting is idempotent for task annotations
#[test]
fn test_round_trip_preserves_task_annotations() {
    let config = make_config();
    let formatter = Formatter::new(config.clone(), FormatOptions::default());

    let content = r#"# Test

@id: test

## Tasks

- [ ] Task with metadata
  @id: explicit-task-id
  @owner: bob
  - This is a contextual note
  - Another note here
"#;

    let file = parse_file_from_string(content, &config).unwrap();
    let formatted1 = formatter.format_file(&file).unwrap();

    // Parse and format again - should be idempotent
    let file2 = parse_file_from_string(&formatted1, &config).unwrap();
    let formatted2 = formatter.format_file(&file2).unwrap();

    assert_eq!(
        formatted1, formatted2,
        "Formatting with task annotations should be idempotent.\nFirst:\n{formatted1}\n\nSecond:\n{formatted2}"
    );

    // Verify annotations survived both passes
    assert!(formatted2.contains("@id: explicit-task-id"));
    assert!(formatted2.contains("@owner: bob"));
    assert!(formatted2.contains("- This is a contextual note"));
    assert!(formatted2.contains("- Another note here"));
}
