//! Explanations for the linter's syntax rules
//!
//! These are the codes `lash lint` emits for structural problems it can see in
//! a single file without interpreting the content: `E_SYNTAX_*`,
//! `W_SYNTAX_HEADER` and `I_SYNTAX_ORDER`.

use super::ErrorExplanation;
use crate::error::codes;

/// Codes explained by this module
pub(super) const CODES: &[&str] = &[
    codes::E_SYNTAX_CHECKBOX,
    codes::E_SYNTAX_INDENT,
    codes::E_SYNTAX_DEPTH,
    codes::E_SYNTAX_ANNOTATION,
    codes::E_SYNTAX_UNKNOWN_KEY,
    codes::E_SYNTAX_DUPLICATE_DESCRIPTION,
    codes::W_SYNTAX_HEADER,
    codes::I_SYNTAX_ORDER,
];

/// Look up a syntax rule explanation
pub(super) fn explain(code: &str) -> Option<ErrorExplanation> {
    match code {
        codes::E_SYNTAX_CHECKBOX => Some(ErrorExplanation {
            code: codes::E_SYNTAX_CHECKBOX,
            summary: "Checkbox marker is not one of the four valid states",
            description: "A task line uses a checkbox marker Lash does not recognize. The only valid markers are `[ ]` (open), `[x]` (done), `[-]` (waived) and `[!]` (blocked), written as `- [x] Title` with a single space on either side of the marker.",
            why_it_matters: "A line whose checkbox Lash cannot read is not a task: it will not be indexed, listed, searched, or considered when resolving dependencies. The text stays in the file and silently drops out of every query.",
            how_to_fix: "Replace the marker with one of `[ ]`, `[x]`, `[-]` or `[!]`. `lash format` rewrites near-miss markers (such as `[X]`) automatically.",
            example_bad: Some("- [*] Invalid marker\n- [] Missing space\n-[ ] Missing space after the dash"),
            example_good: Some("- [ ] Open task\n- [x] Completed task\n- [-] Waived task\n- [!] Blocked task"),
        }),

        codes::E_SYNTAX_INDENT => Some(ErrorExplanation {
            code: codes::E_SYNTAX_INDENT,
            summary: "Checkbox indentation is not a multiple of 2 spaces",
            description: "Every nesting level in a Lash task list is exactly 2 spaces. A checkbox line indented by 3 spaces, or with a tab, does not land on a level.",
            why_it_matters: "Indentation is the only thing that encodes parent/child structure. An off-by-one indent silently reparents a task, which changes which parent it blocks and where it shows up in the tree.",
            how_to_fix: "Indent by 2 spaces per level and use spaces, not tabs. `lash format` normalizes indentation for you.",
            example_bad: Some("- [ ] Parent\n   - [ ] Child (3 spaces)\n\t- [ ] Child (tab)"),
            example_good: Some("- [ ] Parent\n  - [ ] Child\n    - [ ] Grandchild"),
        }),

        codes::E_SYNTAX_DEPTH => Some(ErrorExplanation {
            code: codes::E_SYNTAX_DEPTH,
            summary: "Task nesting exceeds the configured depth limit",
            description: "A task is nested deeper than the project's `max_depth` allows (3 levels by default, settable to 2-5 via `max_depth` in `.lash/config.toml`).",
            why_it_matters: "Deep hierarchies are hard to read in the terminal and usually mean one file is carrying work that belongs in its own file. The limit keeps files scannable.",
            how_to_fix: "Flatten the hierarchy, move the deep branch into its own task file and link it with `@depends-on`, or raise `max_depth` in `.lash/config.toml` if your project genuinely needs more levels.",
            example_bad: Some("- [ ] Level 1\n  - [ ] Level 2\n    - [ ] Level 3\n      - [ ] Level 4 (beyond the default limit)"),
            example_good: Some("- [ ] Level 1\n  - [ ] Level 2\n    - [ ] Level 3\n      @depends-on: tasks/details.md#task:level-4-work"),
        }),

        codes::E_SYNTAX_ANNOTATION => Some(ErrorExplanation {
            code: codes::E_SYNTAX_ANNOTATION,
            summary: "Annotation line does not match `@key: value`",
            description: "A line starting with `@` is not a well-formed annotation. Annotations are `@key: value` — the key immediately after the `@`, then a colon, then a space, then the value.",
            why_it_matters: "A malformed annotation carries no metadata: the ID, labels, owner or dependency it was meant to declare simply do not exist as far as Lash is concerned.",
            how_to_fix: "Write the annotation as `@key: value`, one per line, indented to the same level as the task it belongs to. `lash format` fixes spacing around the colon.",
            example_bad: Some("@id task-1\n@owner:Alice\n@ labels: backend"),
            example_good: Some("@id: task-1\n@owner: Alice\n@labels: backend"),
        }),

        codes::E_SYNTAX_UNKNOWN_KEY => Some(ErrorExplanation {
            code: codes::E_SYNTAX_UNKNOWN_KEY,
            summary: "Annotation key is neither built-in nor explicitly allowed",
            description: "The annotation key is not one Lash knows (`@id`, `@labels`, `@status`, `@owner`, `@estimate`, `@created`, `@depends-on`, `@doc`, `@agent-note`) and it is not listed in `custom_annotation_keys` in `.lash/config.toml`.",
            why_it_matters: "Unknown keys are almost always typos, and a typo means the metadata is silently absent. Requiring custom keys to be declared keeps that failure loud instead of quiet.",
            how_to_fix: "Fix the typo — the diagnostic suggests the closest built-in key — or add the key to `custom_annotation_keys` in `.lash/config.toml` if it is intentional.",
            example_bad: Some("@labls: backend\n@priority: high   # not built-in and not declared"),
            example_good: Some("@labels: backend\n\n# .lash/config.toml\n# custom_annotation_keys = [\"priority\"]"),
        }),

        codes::E_SYNTAX_DUPLICATE_DESCRIPTION => Some(ErrorExplanation {
            code: codes::E_SYNTAX_DUPLICATE_DESCRIPTION,
            summary: "File has more than one `## Description` section",
            description: "A task file may contain at most one `## Description` section. The diagnostic lists the line of every duplicate it found.",
            why_it_matters: "With two description sections there is no defined answer to \"what is this file's description\" — Lash indexes one of them, and the other silently disappears from `lash list --show-descriptions` and agent prompts.",
            how_to_fix: "Merge the sections into a single `## Description` block and delete the extras.",
            example_bad: Some("# Tasks\n\n## Description\n\nFirst.\n\n## Description\n\nSecond."),
            example_good: Some("# Tasks\n\n## Description\n\nFirst. Second."),
        }),

        codes::W_SYNTAX_HEADER => Some(ErrorExplanation {
            code: codes::W_SYNTAX_HEADER,
            summary: "File is missing its H1 title or `## Tasks` section",
            description: "Every Lash task file starts with a single `# Title` heading and holds its checkboxes under a `## Tasks` heading. This file is missing one of them.",
            why_it_matters: "The H1 is the file's display name in listings and the TUI, and `## Tasks` marks where the task list begins. Without them the file renders as untitled and its structure is ambiguous to both readers and agents.",
            how_to_fix: "Add the missing heading. `lash init` and `lash add --file-title` generate the standard skeleton.",
            example_bad: Some("@id: tasks\n\n- [ ] A task with no headings above it"),
            example_good: Some("# Backend Tasks\n\n@id: backend\n\n## Tasks\n\n- [ ] A task"),
        }),

        codes::I_SYNTAX_ORDER => Some(ErrorExplanation {
            code: codes::I_SYNTAX_ORDER,
            summary: "Annotations are not in the conventional order",
            description: "Annotations on a task read most predictably in a consistent order. This is an informational suggestion, never an error, and it never blocks a lint run.",
            why_it_matters: "Consistent ordering makes diffs smaller and lets readers find `@id` or `@depends-on` in the same place in every file.",
            how_to_fix: "Run `lash format`, which orders annotations for you, or reorder them by hand. Filter the suggestion out with `lash lint --min-severity warning` if you do not want to see it.",
            example_bad: Some("- [ ] Task\n  @owner: alice\n  @id: task-1\n  @labels: backend"),
            example_good: Some("- [ ] Task\n  @id: task-1\n  @labels: backend\n  @owner: alice"),
        }),

        _ => None,
    }
}
