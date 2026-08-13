//! Explanations for the legacy generic lint codes (`E_LINT_*`)
//!
//! These predate the per-rule codes in [`super::syntax`], [`super::semantic`]
//! and [`super::crossfile`]. They are still accepted by `lash explain` so that
//! older diagnostics, scripts and docs keep resolving.

use super::ErrorExplanation;
use crate::error::codes;

/// Codes explained by this module
pub(super) const CODES: &[&str] = &[
    codes::E_LINT_DUPLICATE_ID,
    codes::E_LINT_UNKNOWN_ANNOTATION,
    codes::E_LINT_DEPTH_EXCEEDED,
    codes::E_LINT_STATUS_INCONSISTENCY,
    codes::E_LINT_INVALID_LABEL,
    codes::E_LINT_MISSING_ANNOTATION,
    codes::E_LINT_BAD_INDENTATION,
];

/// Look up a legacy lint error explanation
pub(super) fn explain(code: &str) -> Option<ErrorExplanation> {
    match code {
        // ===== Lint Errors =====
        codes::E_LINT_DUPLICATE_ID => Some(ErrorExplanation {
            code: codes::E_LINT_DUPLICATE_ID,
            summary: "Duplicate task ID",
            description: "Two or more tasks in the same file have the same @id annotation. Task IDs must be unique within a file.",
            why_it_matters: "Duplicate IDs make it impossible to reference specific tasks unambiguously, breaking dependency links and task lookups.",
            how_to_fix: "Rename one of the duplicate IDs to a unique value. Choose descriptive, unique identifiers for each task.",
            example_bad: Some("- [ ] First task\n  @id: setup\n\n- [ ] Second task\n  @id: setup"),
            example_good: Some("- [ ] First task\n  @id: setup-database\n\n- [ ] Second task\n  @id: setup-server"),
        }),

        codes::E_LINT_UNKNOWN_ANNOTATION => Some(ErrorExplanation {
            code: codes::E_LINT_UNKNOWN_ANNOTATION,
            summary: "Unknown annotation",
            description: "The annotation used is not recognized by Lash. Valid annotations are: @id, @labels, @owner, @estimate, @depends-on, @created, @doc, and @agent-note.",
            why_it_matters: "Unknown annotations are ignored by Lash, which may indicate a typo or misunderstanding of the annotation system.",
            how_to_fix: "Check the annotation name for typos, or remove it if it's not needed. Refer to the documentation for the list of valid annotations.",
            example_bad: Some("@task-id: my-task\n@priority: high"),
            example_good: Some("@id: my-task\n@labels: priority-high"),
        }),

        codes::E_LINT_DEPTH_EXCEEDED => Some(ErrorExplanation {
            code: codes::E_LINT_DEPTH_EXCEEDED,
            summary: "Task nesting exceeds maximum depth",
            description: "Tasks are nested too deeply. The maximum recommended depth is 4 levels (parent, child, grandchild, great-grandchild).",
            why_it_matters: "Excessive nesting makes task files hard to read and maintain. It often indicates that tasks should be broken into separate files.",
            how_to_fix: "Flatten the task hierarchy by moving deeply nested tasks to a separate file or restructuring the task breakdown.",
            example_bad: Some("- [ ] Level 1\n  - [ ] Level 2\n    - [ ] Level 3\n      - [ ] Level 4\n        - [ ] Level 5 (too deep)"),
            example_good: Some("- [ ] High-level task\n  @depends-on: detailed-tasks.md#task:setup\n\n(Move details to detailed-tasks.md)"),
        }),

        codes::E_LINT_STATUS_INCONSISTENCY => Some(ErrorExplanation {
            code: codes::E_LINT_STATUS_INCONSISTENCY,
            summary: "Parent task marked done with incomplete children",
            description: "A parent task is marked as done [x] but has child tasks that are still open [ ].",
            why_it_matters: "This creates logical inconsistency in your task hierarchy. A parent can only be complete when all its children are complete or waived.",
            how_to_fix: "Either mark all child tasks as done/waived, or change the parent status to open or blocked.",
            example_bad: Some("- [x] Complete feature\n  - [ ] Write code\n  - [ ] Write tests"),
            example_good: Some("- [ ] Complete feature\n  - [x] Write code\n  - [x] Write tests\n\nOR\n\n- [x] Complete feature\n  - [x] Write code\n  - [x] Write tests"),
        }),

        codes::E_LINT_INVALID_LABEL => Some(ErrorExplanation {
            code: codes::E_LINT_INVALID_LABEL,
            summary: "Invalid label format",
            description: "A label contains invalid characters. Labels must be alphanumeric with hyphens, and multiple labels should be comma-separated.",
            why_it_matters: "Invalid labels can't be used for filtering and may cause parsing errors.",
            how_to_fix: "Use only letters, numbers, and hyphens in labels. Separate multiple labels with commas.",
            example_bad: Some("@labels: front-end!, back_end\n@labels: high priority"),
            example_good: Some("@labels: front-end, back-end\n@labels: high-priority"),
        }),

        codes::E_LINT_MISSING_ANNOTATION => Some(ErrorExplanation {
            code: codes::E_LINT_MISSING_ANNOTATION,
            summary: "Missing required annotation",
            description: "A task is missing a required annotation, typically @id. Some projects require certain annotations for proper tracking.",
            why_it_matters: "Missing required annotations prevent proper task identification and cross-referencing.",
            how_to_fix: "Add the required annotation to the task. For @id, choose a unique, descriptive identifier.",
            example_bad: Some("- [ ] My task without ID"),
            example_good: Some("- [ ] My task\n  @id: my-task-id"),
        }),

        codes::E_LINT_BAD_INDENTATION => Some(ErrorExplanation {
            code: codes::E_LINT_BAD_INDENTATION,
            summary: "Incorrect indentation",
            description: "The indentation doesn't match the expected 2-space increments for task nesting.",
            why_it_matters: "Inconsistent indentation breaks the task hierarchy and makes files hard to read.",
            how_to_fix: "Run `lash format` to automatically fix indentation to the standard 2-space increments.",
            example_bad: Some("- [ ] Parent\n   - [ ] Child (3 spaces)\n    - [ ] Another (4 spaces)"),
            example_good: Some("- [ ] Parent\n  - [ ] Child (2 spaces)\n  - [ ] Another (2 spaces)"),
        }),
        _ => None,
    }
}
