//! Explanations for parser error codes (`E_PARSE_*`)

use super::ErrorExplanation;
use crate::error::codes;

/// Codes explained by this module
pub(super) const CODES: &[&str] = &[
    codes::E_PARSE,
    codes::E_PARSE_INVALID_CHECKBOX,
    codes::E_PARSE_INVALID_ANNOTATION,
    codes::E_PARSE_INVALID_HEADER,
    codes::E_PARSE_UNEXPECTED_DEPTH,
    codes::E_PARSE_INVALID_DATE,
];

/// Look up a parse error explanation
pub(super) fn explain(code: &str) -> Option<ErrorExplanation> {
    match code {
        codes::E_PARSE => Some(ErrorExplanation {
            code: codes::E_PARSE,
            summary: "File could not be parsed",
            description: "Lash could not read the file as a task file. The diagnostic message carries the specific reason and the line it stopped on; the `E_PARSE_*` codes describe the individual causes.",
            why_it_matters: "A file that does not parse contributes nothing: none of its tasks are linted, indexed, listed or available as dependency targets. Everything downstream behaves as if the file were empty.",
            how_to_fix: "Read the reason in the message and fix that line. The usual causes are an unrecognized checkbox marker, an annotation that is not `@key: value`, or indentation that is not a multiple of 2 spaces — `lash format` fixes all three.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_PARSE_INVALID_CHECKBOX => Some(ErrorExplanation {
            code: codes::E_PARSE_INVALID_CHECKBOX,
            summary: "Invalid checkbox syntax",
            description: "The checkbox marker in your task is not valid. Lash only recognizes four checkbox states: [ ] for open, [x] for done, [-] for waived, and [!] for blocked.",
            why_it_matters: "Invalid checkboxes prevent Lash from parsing your task files correctly, which means the task won't be indexed, tracked, or included in dependency resolution.",
            how_to_fix: "Replace the invalid checkbox with one of the valid formats: [ ], [x], [-], or [!]. Run `lash format` to automatically fix checkbox formatting.",
            example_bad: Some("- [*] Invalid checkbox\n- [v] Also invalid\n- [] Missing space"),
            example_good: Some("- [ ] Open task\n- [x] Completed task\n- [-] Waived task\n- [!] Blocked task"),
        }),

        codes::E_PARSE_INVALID_ANNOTATION => Some(ErrorExplanation {
            code: codes::E_PARSE_INVALID_ANNOTATION,
            summary: "Malformed annotation",
            description: "An annotation in your task file doesn't follow the required @key: value format. Annotations must start with @, followed by the annotation name, a colon, and the value.",
            why_it_matters: "Malformed annotations can't be parsed, so Lash won't be able to read metadata like task IDs, labels, owners, or dependencies.",
            how_to_fix: "Ensure annotations follow the format: @key: value. There must be a space after the colon. Run `lash format` to normalize formatting.",
            example_bad: Some("@id task-1\n@labels: frontend, backend\n@owner:Alice"),
            example_good: Some("@id: task-1\n@labels: frontend, backend\n@owner: Alice"),
        }),

        codes::E_PARSE_INVALID_HEADER => Some(ErrorExplanation {
            code: codes::E_PARSE_INVALID_HEADER,
            summary: "Invalid header format",
            description: "A header in your file doesn't follow proper Markdown syntax. Headers must start with one or more # symbols followed by a space.",
            why_it_matters: "Invalid headers break the document structure, making it impossible for Lash to organize tasks into sections.",
            how_to_fix: "Add a space after the # symbols in your headers. For example, change '##Tasks' to '## Tasks'.",
            example_bad: Some("##Tasks\n###Section 1"),
            example_good: Some("## Tasks\n### Section 1"),
        }),

        codes::E_PARSE_UNEXPECTED_DEPTH => Some(ErrorExplanation {
            code: codes::E_PARSE_UNEXPECTED_DEPTH,
            summary: "Unexpected indentation depth",
            description: "A task or subtask has incorrect indentation. Each level of nesting should be indented by exactly 2 spaces.",
            why_it_matters: "Incorrect indentation breaks the task hierarchy, which affects dependency resolution and task organization.",
            how_to_fix: "Adjust the indentation to use exactly 2 spaces per nesting level. Run `lash format` to automatically fix indentation.",
            example_bad: Some("- [ ] Parent\n   - [ ] Child (3 spaces)\n- [ ] Another (wrong depth)"),
            example_good: Some("- [ ] Parent\n  - [ ] Child (2 spaces)\n    - [ ] Grandchild (4 spaces)"),
        }),

        codes::E_PARSE_INVALID_DATE => Some(ErrorExplanation {
            code: codes::E_PARSE_INVALID_DATE,
            summary: "Invalid date format",
            description: "A date annotation doesn't use the required YYYY-MM-DD format.",
            why_it_matters: "Invalid dates can't be parsed or compared, breaking features like task filtering by date and timeline calculations.",
            how_to_fix: "Change the date to YYYY-MM-DD format. For example, '2024-01-15' for January 15, 2024.",
            example_bad: Some("@created: 01/15/2024\n@created: Jan 15, 2024"),
            example_good: Some("@created: 2024-01-15"),
        }),
        _ => None,
    }
}
