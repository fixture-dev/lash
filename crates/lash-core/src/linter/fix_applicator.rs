//! Apply multiple fixes to file content safely
//!
//! The `FixApplicator` handles applying multiple fixes to a single file's content
//! while managing position shifts, detecting overlapping fixes, and ensuring
//! content integrity.
//!
//! # Algorithm
//!
//! 1. Sort fixes by position (reverse order for safe application)
//! 2. Apply fixes one at a time from end to start of file
//! 3. Track position shifts after each fix
//! 4. Detect and skip overlapping or invalid fixes
//! 5. Return the fixed content plus metadata about what was applied
//!
//! # Example
//!
//! ```
//! use lash_core::linter::{FixApplicator, LintDiagnostic, Fix};
//! use lash_types::Severity;
//! use std::path::PathBuf;
//!
//! let content = "- [*] task one\n- [*] task two";
//!
//! # let fix1 = Fix::replace("fix first checkbox", "- [*] task one", "- [ ] task one");
//! # let fix2 = Fix::replace("fix second checkbox", "- [*] task two", "- [ ] task two");
//! # let diag1 = LintDiagnostic::error("E_TEST", "bad checkbox", PathBuf::from("test.md"), 1, 1)
//! #     .with_fix(fix1);
//! # let diag2 = LintDiagnostic::error("E_TEST", "bad checkbox", PathBuf::from("test.md"), 2, 1)
//! #     .with_fix(fix2);
//! # let diagnostics = vec![diag1, diag2];
//! let applicator = FixApplicator::new(content);
//! let result = applicator.apply_fixes(&diagnostics);
//!
//! assert_eq!(result.fixed_content, "- [ ] task one\n- [ ] task two");
//! assert_eq!(result.applied_count, 2);
//! assert_eq!(result.skipped_fixes.len(), 0);
//! ```

use crate::linter::{LintDiagnostic, Replacement};

/// Result of applying fixes to content
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    /// The fixed content (or original if all fixes failed)
    pub fixed_content: String,

    /// Number of fixes successfully applied
    pub applied_count: usize,

    /// Fixes that were skipped due to conflicts or errors
    pub skipped_fixes: Vec<SkippedFix>,
}

/// A fix that was skipped during application
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedFix {
    /// Description of the fix that was skipped
    pub description: String,

    /// Reason why it was skipped
    pub reason: String,
}

/// Applies multiple fixes to file content safely
///
/// The applicator handles position tracking, overlap detection, and error
/// handling to ensure that applying multiple fixes doesn't corrupt content.
pub struct FixApplicator {
    /// Original content to apply fixes to
    content: String,
}

impl FixApplicator {
    /// Create a new fix applicator for the given content
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::linter::FixApplicator;
    ///
    /// let content = "# Title\n\n- [ ] task";
    /// let applicator = FixApplicator::new(content);
    /// ```
    #[must_use]
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
        }
    }

    /// Apply all fixes from the given diagnostics
    ///
    /// Fixes are applied in reverse position order to avoid position shift
    /// complications. Overlapping fixes are detected and skipped.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::linter::{FixApplicator, LintDiagnostic, Fix};
    /// use std::path::PathBuf;
    ///
    /// let content = "- [x] done\n- [ ] todo";
    /// let applicator = FixApplicator::new(content);
    ///
    /// # let fix = Fix::replace("update status", "- [x] done", "- [ ] done");
    /// # let diag = LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1)
    /// #     .with_fix(fix);
    /// # let diagnostics = vec![diag];
    /// let result = applicator.apply_fixes(&diagnostics);
    ///
    /// assert!(result.applied_count > 0 || result.skipped_fixes.len() > 0);
    /// ```
    #[must_use]
    pub fn apply_fixes(mut self, diagnostics: &[LintDiagnostic]) -> ApplyResult {
        let mut applied_count = 0;
        let mut skipped_fixes = Vec::new();

        // Extract fixes with their positions
        let mut positioned_fixes: Vec<(usize, &Replacement, String)> = Vec::new();

        for diag in diagnostics {
            if let Some(fix) = &diag.fix {
                let position = self.estimate_position(&fix.replacement);
                positioned_fixes.push((position, &fix.replacement, fix.description.clone()));
            }
        }

        // Sort by position in reverse order (end to start)
        // This way, applying a fix doesn't affect positions of earlier fixes
        positioned_fixes.sort_by_key(|entry| std::cmp::Reverse(entry.0));

        // Track regions we've modified to detect overlaps
        let mut modified_regions: Vec<(usize, usize)> = Vec::new();

        for (pos, replacement, description) in positioned_fixes {
            // Check for overlaps with already-modified regions
            let (start, end) = self.replacement_bounds(pos, replacement);
            if Self::overlaps_with_regions(start, end, &modified_regions) {
                skipped_fixes.push(SkippedFix {
                    description: description.clone(),
                    reason: format!("Overlaps with previously applied fix at position {pos}"),
                });
                continue;
            }

            // Try to apply the fix
            match replacement.apply(&self.content) {
                Ok(new_content) => {
                    self.content = new_content;
                    applied_count += 1;
                    modified_regions.push((start, end));
                }
                Err(err) => {
                    skipped_fixes.push(SkippedFix {
                        description: description.clone(),
                        reason: err,
                    });
                }
            }
        }

        ApplyResult {
            fixed_content: self.content,
            applied_count,
            skipped_fixes,
        }
    }

    /// Estimate the position of a replacement in the content
    ///
    /// For position-based replacements (Insert, Delete), this is exact.
    /// For `TextReplace`, we find the first occurrence.
    fn estimate_position(&self, replacement: &Replacement) -> usize {
        match replacement {
            Replacement::Insert { position, .. } => *position,
            Replacement::Delete { start, .. } => *start,
            Replacement::TextReplace { old, .. } => {
                // Find first occurrence
                self.content.find(old.as_str()).unwrap_or(0)
            }
            Replacement::Reformat => 0,
        }
    }

    /// Get the start and end bounds of a replacement
    ///
    /// For Insert operations, we return a zero-width range at the position
    /// since inserts don't consume any existing content.
    fn replacement_bounds(&self, pos: usize, replacement: &Replacement) -> (usize, usize) {
        match replacement {
            Replacement::Insert { position, .. } => {
                // Insert has zero width - it doesn't consume existing content
                (*position, *position)
            }
            Replacement::Delete { start, end } => (*start, *end),
            Replacement::TextReplace { old, new: _ } => {
                // For text replace, we need to find the actual position
                if let Some(idx) = self.content.find(old.as_str()) {
                    (idx, idx + old.len())
                } else {
                    (pos, pos)
                }
            }
            Replacement::Reformat => (0, self.content.len()),
        }
    }

    /// Check if the given range overlaps with any modified regions
    ///
    /// For zero-width ranges (point insertions), we only consider them overlapping
    /// if they're at the exact same position as another zero-width range.
    /// Zero-width ranges don't overlap with non-zero-width ranges.
    fn overlaps_with_regions(start: usize, end: usize, regions: &[(usize, usize)]) -> bool {
        let is_zero_width = start == end;

        for (region_start, region_end) in regions {
            let region_is_zero_width = region_start == region_end;

            if is_zero_width && region_is_zero_width {
                // Two point insertions overlap only if at the same position
                if start == *region_start {
                    return true;
                }
            } else if !is_zero_width && !region_is_zero_width {
                // Standard overlap check for ranges
                if start < *region_end && end > *region_start {
                    return true;
                }
            } else {
                // One is zero-width, one is not
                // Zero-width overlaps if it's strictly inside the range
                if is_zero_width {
                    if start > *region_start && start < *region_end {
                        return true;
                    }
                } else if start < *region_start && end > *region_start {
                    // Range overlaps with the point insertion
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::Fix;
    use std::path::PathBuf;

    #[test]
    fn test_single_text_replace() {
        let content = "- [*] task one";
        let applicator = FixApplicator::new(content);

        let fix = Fix::replace("fix checkbox", "- [*]", "- [ ]");
        let diag = LintDiagnostic::error("E_TEST", "bad checkbox", PathBuf::from("test.md"), 1, 1)
            .with_fix(fix);

        let result = applicator.apply_fixes(&[diag]);

        assert_eq!(result.fixed_content, "- [ ] task one");
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.skipped_fixes.len(), 0);
    }

    #[test]
    fn test_multiple_non_overlapping_fixes() {
        let content = "- [*] task one\n- [*] task two";
        let applicator = FixApplicator::new(content);

        let fix1 = Fix::replace("fix first", "- [*] task one", "- [ ] task one");
        let fix2 = Fix::replace("fix second", "- [*] task two", "- [ ] task two");

        let diag1 = LintDiagnostic::error("E_TEST", "bad checkbox", PathBuf::from("test.md"), 1, 1)
            .with_fix(fix1);
        let diag2 = LintDiagnostic::error("E_TEST", "bad checkbox", PathBuf::from("test.md"), 2, 1)
            .with_fix(fix2);

        let result = applicator.apply_fixes(&[diag1, diag2]);

        assert_eq!(result.fixed_content, "- [ ] task one\n- [ ] task two");
        assert_eq!(result.applied_count, 2);
        assert_eq!(result.skipped_fixes.len(), 0);
    }

    #[test]
    fn test_insert_fix() {
        let content = "# Title\n\n## Tasks";
        let applicator = FixApplicator::new(content);

        let fix = Fix::insert("add annotation", 9, "@id: test\n");
        let diag = LintDiagnostic::error("E_TEST", "missing id", PathBuf::from("test.md"), 1, 1)
            .with_fix(fix);

        let result = applicator.apply_fixes(&[diag]);

        assert_eq!(result.fixed_content, "# Title\n\n@id: test\n## Tasks");
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.skipped_fixes.len(), 0);
    }

    #[test]
    fn test_delete_fix() {
        let content = "# Title\nbad line\n## Tasks";
        let applicator = FixApplicator::new(content);

        let fix = Fix::delete("remove line", 8, 17);
        let diag = LintDiagnostic::error("E_TEST", "invalid line", PathBuf::from("test.md"), 2, 1)
            .with_fix(fix);

        let result = applicator.apply_fixes(&[diag]);

        assert_eq!(result.fixed_content, "# Title\n## Tasks");
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.skipped_fixes.len(), 0);
    }

    #[test]
    fn test_overlapping_fixes_detected() {
        let content = "- [*] task";
        let applicator = FixApplicator::new(content);

        // Two fixes that would both modify the same text
        let fix1 = Fix::replace("fix checkbox", "- [*]", "- [ ]");
        let fix2 = Fix::replace("fix entire line", "- [*] task", "- [x] task");

        let diag1 = LintDiagnostic::error("E_TEST", "bad checkbox", PathBuf::from("test.md"), 1, 1)
            .with_fix(fix1);
        let diag2 = LintDiagnostic::error("E_TEST", "bad status", PathBuf::from("test.md"), 1, 1)
            .with_fix(fix2);

        let result = applicator.apply_fixes(&[diag1, diag2]);

        // One should be applied, one should be skipped
        // The second fix fails because the first fix changed the content
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.skipped_fixes.len(), 1);
        assert!(result.skipped_fixes[0].reason.contains("not found"));
    }

    #[test]
    fn test_overlapping_position_based_fixes() {
        let content = "line1\nline2\nline3";
        let applicator = FixApplicator::new(content);

        // Two inserts at overlapping positions
        let fix1 = Fix::insert("insert A", 6, "AAA");
        let fix2 = Fix::insert("insert B", 8, "BBB");

        let diag1 =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix1);
        let diag2 =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix2);

        let result = applicator.apply_fixes(&[diag1, diag2]);

        // Both should be applied as they don't truly overlap (point insertions)
        assert_eq!(result.applied_count, 2);
        assert_eq!(result.skipped_fixes.len(), 0);
    }

    #[test]
    fn test_overlapping_delete_and_insert() {
        let content = "line1\nline2\nline3";
        let applicator = FixApplicator::new(content);

        // Delete from 6 to 12 (covers "line2\n")
        let fix1 = Fix::delete("remove line2", 6, 12);
        // Insert at position 8 (which is inside the deletion range)
        let fix2 = Fix::insert("insert text", 8, "XXX");

        let diag1 =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix1);
        let diag2 =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix2);

        let result = applicator.apply_fixes(&[diag1, diag2]);

        // The delete is applied first (higher position), then insert fails due to overlap
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.skipped_fixes.len(), 1);
        assert!(result.skipped_fixes[0].reason.contains("Overlap"));
    }

    #[test]
    fn test_fix_not_found() {
        let content = "some content";
        let applicator = FixApplicator::new(content);

        let fix = Fix::replace("fix missing", "missing text", "new text");
        let diag =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix);

        let result = applicator.apply_fixes(&[diag]);

        assert_eq!(result.fixed_content, "some content"); // Original unchanged
        assert_eq!(result.applied_count, 0);
        assert_eq!(result.skipped_fixes.len(), 1);
        assert!(result.skipped_fixes[0].reason.contains("not found"));
    }

    #[test]
    fn test_insert_out_of_bounds() {
        let content = "short";
        let applicator = FixApplicator::new(content);

        let fix = Fix::insert("insert text", 1000, "text");
        let diag =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix);

        let result = applicator.apply_fixes(&[diag]);

        assert_eq!(result.fixed_content, "short"); // Original unchanged
        assert_eq!(result.applied_count, 0);
        assert_eq!(result.skipped_fixes.len(), 1);
        assert!(result.skipped_fixes[0].reason.contains("out of bounds"));
    }

    #[test]
    fn test_multiple_inserts_different_positions() {
        let content = "line1\nline2\nline3";
        let applicator = FixApplicator::new(content);

        let fix1 = Fix::insert("add at start", 0, "START\n");
        let fix2 = Fix::insert("add at end", content.len(), "\nEND");

        let diag1 =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix1);
        let diag2 =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 3, 1).with_fix(fix2);

        let result = applicator.apply_fixes(&[diag1, diag2]);

        assert_eq!(result.fixed_content, "START\nline1\nline2\nline3\nEND");
        assert_eq!(result.applied_count, 2);
        assert_eq!(result.skipped_fixes.len(), 0);
    }

    #[test]
    fn test_reformat_fix() {
        let content = "some content";
        let applicator = FixApplicator::new(content);

        let fix = Fix::reformat("reformat file");
        let diag =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix);

        let result = applicator.apply_fixes(&[diag]);

        // Reformat doesn't change content (handled by formatter)
        assert_eq!(result.fixed_content, "some content");
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.skipped_fixes.len(), 0);
    }

    #[test]
    fn test_position_shift_handling() {
        // Test that applying fixes in reverse order handles position shifts correctly
        let content = "ABC\nDEF\nGHI";
        let applicator = FixApplicator::new(content);

        // Insert at position 4 (after ABC\n)
        let fix1 = Fix::insert("add middle", 4, "XXX\n");
        // Insert at position 8 (after DEF\n, but this is BEFORE fix1's position)
        let fix2 = Fix::insert("add later", 8, "YYY\n");

        let diag1 =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix1);
        let diag2 =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 2, 1).with_fix(fix2);

        let result = applicator.apply_fixes(&[diag1, diag2]);

        // Both should be applied without conflict
        assert_eq!(result.fixed_content, "ABC\nXXX\nDEF\nYYY\nGHI");
        assert_eq!(result.applied_count, 2);
        assert_eq!(result.skipped_fixes.len(), 0);
    }

    #[test]
    fn test_no_fixes() {
        let content = "some content";
        let applicator = FixApplicator::new(content);

        // Diagnostic without a fix
        let diag = LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1);

        let result = applicator.apply_fixes(&[diag]);

        assert_eq!(result.fixed_content, "some content");
        assert_eq!(result.applied_count, 0);
        assert_eq!(result.skipped_fixes.len(), 0);
    }

    #[test]
    fn test_empty_content() {
        let content = "";
        let applicator = FixApplicator::new(content);

        let fix = Fix::insert("add content", 0, "new content");
        let diag =
            LintDiagnostic::error("E_TEST", "test", PathBuf::from("test.md"), 1, 1).with_fix(fix);

        let result = applicator.apply_fixes(&[diag]);

        assert_eq!(result.fixed_content, "new content");
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.skipped_fixes.len(), 0);
    }
}
