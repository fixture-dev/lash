//! Integration tests for FixApplicator
//!
//! These tests verify the FixApplicator's behavior in realistic scenarios
//! including edge cases and error conditions.

use lash_core::linter::{Fix, FixApplicator, LintDiagnostic};
use std::path::PathBuf;

/// Test applying multiple fixes to a complex file
#[test]
fn test_apply_fixes_to_complex_file() {
    let content = r#"# Complex Task File

@id: test.complex
@status: in-progress
@created: 2024-01-15

## Tasks

- [*] First invalid checkbox
- [ ] Valid task
- [*] Second invalid checkbox
  - [*] Nested invalid
- [x] Completed task
- [*] Another invalid
"#;

    let applicator = FixApplicator::new(content);

    // Create fixes for each invalid checkbox
    let fix1 = Fix::replace(
        "fix first",
        "- [*] First invalid checkbox",
        "- [ ] First invalid checkbox",
    );
    let fix2 = Fix::replace(
        "fix second",
        "- [*] Second invalid checkbox",
        "- [ ] Second invalid checkbox",
    );
    let fix3 = Fix::replace(
        "fix nested",
        "  - [*] Nested invalid",
        "  - [ ] Nested invalid",
    );
    let fix4 = Fix::replace(
        "fix another",
        "- [*] Another invalid",
        "- [ ] Another invalid",
    );

    let diag1 = LintDiagnostic::error(
        "E_INVALID_CHECKBOX",
        "Invalid checkbox",
        PathBuf::from("test.md"),
        9,
        3,
    )
    .with_fix(fix1);
    let diag2 = LintDiagnostic::error(
        "E_INVALID_CHECKBOX",
        "Invalid checkbox",
        PathBuf::from("test.md"),
        11,
        3,
    )
    .with_fix(fix2);
    let diag3 = LintDiagnostic::error(
        "E_INVALID_CHECKBOX",
        "Invalid checkbox",
        PathBuf::from("test.md"),
        12,
        5,
    )
    .with_fix(fix3);
    let diag4 = LintDiagnostic::error(
        "E_INVALID_CHECKBOX",
        "Invalid checkbox",
        PathBuf::from("test.md"),
        14,
        3,
    )
    .with_fix(fix4);

    let result = applicator.apply_fixes(&[diag1, diag2, diag3, diag4]);

    // All fixes should be applied
    assert_eq!(result.applied_count, 4, "Should apply all 4 fixes");
    assert_eq!(result.skipped_fixes.len(), 0, "Should not skip any fixes");

    // Verify no invalid checkboxes remain
    assert!(
        !result.fixed_content.contains("[*]"),
        "No invalid checkboxes should remain"
    );

    // Verify structure is preserved
    assert!(result.fixed_content.contains("## Tasks"));
    assert!(result.fixed_content.contains("- [x] Completed task"));
    assert!(result.fixed_content.contains("@id: test.complex"));
}

/// Test fix application preserves file structure
#[test]
fn test_fix_preserves_structure() {
    let content = r#"# Title

@id: test
@labels: one, two, three
@status: in-progress

## Section 1

- [*] Invalid

## Section 2

- [ ] Valid
  - [*] Nested invalid
"#;

    let applicator = FixApplicator::new(content);

    let fix1 = Fix::replace("fix 1", "- [*] Invalid", "- [ ] Invalid");
    let fix2 = Fix::replace("fix 2", "  - [*] Nested invalid", "  - [ ] Nested invalid");

    let diag1 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 9, 3).with_fix(fix1);
    let diag2 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 14, 5).with_fix(fix2);

    let result = applicator.apply_fixes(&[diag1, diag2]);

    assert_eq!(result.applied_count, 2);

    // Verify structure is intact
    assert!(result.fixed_content.contains("# Title"));
    assert!(result.fixed_content.contains("@id: test"));
    assert!(result.fixed_content.contains("@labels: one, two, three"));
    assert!(result.fixed_content.contains("## Section 1"));
    assert!(result.fixed_content.contains("## Section 2"));

    // Verify fixes were applied
    assert!(!result.fixed_content.contains("[*]"));
}

/// Test handling of fixes with unicode content
#[test]
fn test_fix_with_unicode() {
    let content = r#"# Tasks with Unicode

@id: test.unicode
@status: in-progress

## Tasks

- [*] Task with emoji 🚀
- [*] Task with accents: café, naïve
- [*] Task with CJK: 日本語 中文
"#;

    let applicator = FixApplicator::new(content);

    let fix1 = Fix::replace(
        "fix emoji",
        "- [*] Task with emoji 🚀",
        "- [ ] Task with emoji 🚀",
    );
    let fix2 = Fix::replace(
        "fix accents",
        "- [*] Task with accents: café, naïve",
        "- [ ] Task with accents: café, naïve",
    );
    let fix3 = Fix::replace(
        "fix cjk",
        "- [*] Task with CJK: 日本語 中文",
        "- [ ] Task with CJK: 日本語 中文",
    );

    let diag1 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 7, 3).with_fix(fix1);
    let diag2 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 8, 3).with_fix(fix2);
    let diag3 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 9, 3).with_fix(fix3);

    let result = applicator.apply_fixes(&[diag1, diag2, diag3]);

    assert_eq!(result.applied_count, 3);
    assert!(!result.fixed_content.contains("[*]"));

    // Verify unicode is preserved
    assert!(result.fixed_content.contains("🚀"));
    assert!(result.fixed_content.contains("café"));
    assert!(result.fixed_content.contains("日本語"));
}

/// Test fix application with very long lines
#[test]
fn test_fix_with_long_lines() {
    let long_task = "a".repeat(1000);
    let content = format!(
        r#"# Tasks

@id: test.long
@status: in-progress

## Tasks

- [*] {long_task}
"#
    );

    let applicator = FixApplicator::new(&content);

    let old_text = format!("- [*] {long_task}");
    let new_text = format!("- [ ] {long_task}");
    let fix = Fix::replace("fix long", &old_text, &new_text);

    let diag =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 7, 3).with_fix(fix);

    let result = applicator.apply_fixes(&[diag]);

    assert_eq!(result.applied_count, 1);
    assert!(!result.fixed_content.contains("[*]"));
    assert!(result.fixed_content.contains(&long_task));
}

/// Test applying fixes when content has CRLF line endings
#[test]
fn test_fix_with_crlf_line_endings() {
    let content = "# Tasks\r\n\r\n@id: test\r\n\r\n## Tasks\r\n\r\n- [*] Invalid\r\n";

    let applicator = FixApplicator::new(content);

    let fix = Fix::replace("fix checkbox", "- [*] Invalid", "- [ ] Invalid");
    let diag =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 7, 3).with_fix(fix);

    let result = applicator.apply_fixes(&[diag]);

    assert_eq!(result.applied_count, 1);
    assert!(!result.fixed_content.contains("[*]"));
}

/// Test fix application to very large file
#[test]
fn test_fix_large_file() {
    // Generate a file with 100 tasks
    let mut content =
        String::from("# Large File\n\n@id: test.large\n@status: in-progress\n\n## Tasks\n\n");

    for i in 0..100 {
        if i % 3 == 0 {
            content.push_str(&format!("- [*] Invalid task {i}\n"));
        } else {
            content.push_str(&format!("- [ ] Valid task {i}\n"));
        }
    }

    let applicator = FixApplicator::new(&content);

    // Create fixes for all invalid checkboxes
    let mut diagnostics = Vec::new();
    for i in (0..100).step_by(3) {
        let old = format!("- [*] Invalid task {i}");
        let new = format!("- [ ] Invalid task {i}");
        let fix = Fix::replace(format!("fix {i}"), &old, &new);
        let diag = LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 7 + i, 3)
            .with_fix(fix);
        diagnostics.push(diag);
    }

    let result = applicator.apply_fixes(&diagnostics);

    // Should fix approximately 34 tasks (100 / 3)
    assert!(
        result.applied_count >= 33 && result.applied_count <= 34,
        "Should fix ~34 tasks, got {}",
        result.applied_count
    );
    assert!(!result.fixed_content.contains("[*]"));
}

/// Test that identical fixes at same location are properly deduplicated
#[test]
fn test_duplicate_fixes_at_same_location() {
    let content = "- [*] Invalid task\n";

    let applicator = FixApplicator::new(content);

    // Create two identical fixes
    let fix1 = Fix::replace("fix 1", "- [*] Invalid task", "- [ ] Invalid task");
    let fix2 = Fix::replace("fix 2", "- [*] Invalid task", "- [ ] Invalid task");

    let diag1 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 3).with_fix(fix1);
    let diag2 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 3).with_fix(fix2);

    let result = applicator.apply_fixes(&[diag1, diag2]);

    // First fix should succeed, second should be skipped (content changed)
    assert_eq!(result.applied_count, 1, "Should apply first fix");
    assert_eq!(result.skipped_fixes.len(), 1, "Should skip second fix");
    assert!(!result.fixed_content.contains("[*]"));
}

/// Test fix with insert at beginning of file
#[test]
fn test_fix_insert_at_beginning() {
    let content = "## Tasks\n\n- [ ] Task\n";

    let applicator = FixApplicator::new(content);

    let fix = Fix::insert("add title", 0, "# Title\n\n@id: test\n\n");
    let diag =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix);

    let result = applicator.apply_fixes(&[diag]);

    assert_eq!(result.applied_count, 1);
    assert!(result.fixed_content.starts_with("# Title"));
    assert!(result.fixed_content.contains("@id: test"));
}

/// Test fix with delete operation
#[test]
fn test_fix_delete_operation() {
    let content = "# Title\n\nBad line that should be removed\n\n## Tasks\n";

    let applicator = FixApplicator::new(content);

    // Find position of "Bad line"
    let start = content.find("Bad line").unwrap();
    let end = content.find("## Tasks").unwrap();

    let fix = Fix::delete("remove bad line", start, end);
    let diag =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 3, 1).with_fix(fix);

    let result = applicator.apply_fixes(&[diag]);

    assert_eq!(result.applied_count, 1);
    assert!(!result.fixed_content.contains("Bad line"));
    assert!(result.fixed_content.contains("## Tasks"));
}

/// Test applying fixes with mixed insert/delete/replace operations
#[test]
fn test_mixed_fix_operations() {
    let content = r#"# Title

## Tasks

- [*] Invalid
extra line
- [ ] Valid
"#;

    let applicator = FixApplicator::new(content);

    // Replace invalid checkbox
    let fix1 = Fix::replace("fix checkbox", "- [*] Invalid", "- [ ] Invalid");
    // Delete extra line
    let start = content.find("extra line").unwrap();
    let end = start + "extra line\n".len();
    let fix2 = Fix::delete("remove extra", start, end);
    // Insert annotation
    let fix3 = Fix::insert("add annotation", 8, "@id: test\n\n");

    let diag1 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 5, 3).with_fix(fix1);
    let diag2 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 6, 1).with_fix(fix2);
    let diag3 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix3);

    let result = applicator.apply_fixes(&[diag1, diag2, diag3]);

    assert_eq!(result.applied_count, 3, "All three fixes should apply");
    assert!(!result.fixed_content.contains("[*]"));
    assert!(!result.fixed_content.contains("extra line"));
    assert!(result.fixed_content.contains("@id: test"));
}

/// Test that fixes are applied in correct order (reverse position)
#[test]
fn test_fix_application_order() {
    let content = "Line 1\nLine 2\nLine 3\nLine 4\n";

    let applicator = FixApplicator::new(content);

    // Create fixes that would interfere if applied in wrong order
    // Replace Line 2
    let fix1 = Fix::replace("fix line 2", "Line 2", "Modified 2");
    // Replace Line 3
    let fix2 = Fix::replace("fix line 3", "Line 3", "Modified 3");
    // Insert before Line 4
    let pos = content.find("Line 4").unwrap();
    let fix3 = Fix::insert("insert before 4", pos, "Inserted\n");

    let diag1 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 2, 1).with_fix(fix1);
    let diag2 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 3, 1).with_fix(fix2);
    let diag3 =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 4, 1).with_fix(fix3);

    let result = applicator.apply_fixes(&[diag1, diag2, diag3]);

    assert_eq!(result.applied_count, 3);
    assert!(result.fixed_content.contains("Modified 2"));
    assert!(result.fixed_content.contains("Modified 3"));
    assert!(result.fixed_content.contains("Inserted"));
}

/// Test empty file edge case
#[test]
fn test_fix_empty_file() {
    let content = "";
    let applicator = FixApplicator::new(content);

    let fix = Fix::insert("add content", 0, "# New File\n");
    let diag =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix);

    let result = applicator.apply_fixes(&[diag]);

    assert_eq!(result.applied_count, 1);
    assert_eq!(result.fixed_content, "# New File\n");
}

/// Test fix that tries to replace non-existent text
#[test]
fn test_fix_replace_missing_text() {
    let content = "# Title\n\n- [ ] Task\n";

    let applicator = FixApplicator::new(content);

    let fix = Fix::replace("fix missing", "- [*] Invalid", "- [ ] Invalid");
    let diag =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 3, 3).with_fix(fix);

    let result = applicator.apply_fixes(&[diag]);

    assert_eq!(result.applied_count, 0, "Should not apply non-matching fix");
    assert_eq!(result.skipped_fixes.len(), 1, "Should skip the fix");
    assert!(result.skipped_fixes[0].reason.contains("not found"));
}

/// Test fix with whitespace variations
#[test]
fn test_fix_with_whitespace_sensitivity() {
    let content = "- [*] Task with spaces\n";

    let applicator = FixApplicator::new(content);

    // Fix must match whitespace exactly
    let fix = Fix::replace("fix", "- [*] Task with spaces", "- [ ] Task with spaces");
    let diag =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 3).with_fix(fix);

    let result = applicator.apply_fixes(&[diag]);

    assert_eq!(result.applied_count, 1);
    assert!(!result.fixed_content.contains("[*]"));
}

/// Test reformat fix (special case)
#[test]
fn test_reformat_fix() {
    let content = "# Title\n\n- [ ] Task\n";

    let applicator = FixApplicator::new(content);

    let fix = Fix::reformat("reformat file");
    let diag =
        LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix);

    let result = applicator.apply_fixes(&[diag]);

    // Reformat is a special case - it doesn't change content in FixApplicator
    // (actual reformatting happens in formatter)
    assert_eq!(result.applied_count, 1);
    assert_eq!(result.fixed_content, content);
}
