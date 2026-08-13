//! Explanations for the linter's cross-file rules
//!
//! These are the codes `lash lint` emits when it compares files against each
//! other: dependency links (`E_LINK_*`) and root-index coverage
//! (`E_INDEX_FILE_MISSING`, `W_INDEX_ORPHAN`).

use super::ErrorExplanation;
use crate::error::codes;

/// Codes explained by this module
pub(super) const CODES: &[&str] = &[
    codes::E_LINK_NOT_FOUND,
    codes::E_LINK_CYCLE,
    codes::E_LINK_INVALID_PATH,
    codes::E_INDEX_FILE_MISSING,
    codes::W_INDEX_ORPHAN,
];

/// Look up a cross-file rule explanation
pub(super) fn explain(code: &str) -> Option<ErrorExplanation> {
    match code {
        codes::E_LINK_NOT_FOUND => Some(ErrorExplanation {
            code: codes::E_LINK_NOT_FOUND,
            summary: "`@depends-on:` target file or task does not exist",
            description: "A dependency reference points at a file that is not in the project, or at a task ID that file does not contain. References take the form `path/to/file.md#task:id`, resolved relative to the file holding the annotation.",
            why_it_matters: "An unresolvable dependency is not tracked: the dependent task is never reported as blocked, so `lash list --blocked` and the graph both understate what is waiting on what.",
            how_to_fix: "Check the path and the task ID — `lash show <id>` prints the qualified form to copy. If the target task still exists but its ID changed, run `lash migrate-ids` to rewrite the references. `lash check-links` lists every broken reference at once.",
            example_bad: Some("@depends-on: tasks/missing.md#task:setup\n@depends-on: tasks/setup.md#task:no-such-id"),
            example_good: Some("@depends-on: tasks/setup.md#task:database-setup"),
        }),

        codes::E_LINK_CYCLE => Some(ErrorExplanation {
            code: codes::E_LINK_CYCLE,
            summary: "Dependency references form a cycle",
            description: "Following `@depends-on:` links leads back to where it started: A waits on B, B waits on C, C waits on A. The diagnostic prints the cycle path.",
            why_it_matters: "Every task in a cycle is permanently blocked by another task in the cycle, so none of them can ever be started. Cycles also make any dependency ordering impossible to compute.",
            how_to_fix: "Drop the one dependency that closes the loop, or split the task that appears twice into the part that comes first and the part that comes later.",
            example_bad: Some("# a.md\n- [ ] A\n  @depends-on: b.md#task:b\n\n# b.md\n- [ ] B\n  @depends-on: a.md#task:a"),
            example_good: Some("# a.md\n- [ ] A\n  @depends-on: b.md#task:b\n\n# b.md\n- [ ] B   # no dependency back on A"),
        }),

        codes::E_LINK_INVALID_PATH => Some(ErrorExplanation {
            code: codes::E_LINK_INVALID_PATH,
            summary: "Dependency path is malformed or escapes the project root",
            description: "The path part of a dependency reference is not well-formed, or it climbs out of the project with `../` segments. Dependencies may only point at files inside the project.",
            why_it_matters: "A reference outside the project cannot be indexed or verified, and it makes the task file non-portable — the link breaks for anyone who checks the project out somewhere else.",
            how_to_fix: "Use a path inside the project, relative to the file containing the annotation. If the target genuinely lives outside the project, link it as documentation with `@doc:` instead of as a dependency.",
            example_bad: Some("@depends-on: ../../other-project/tasks.md#task:setup\n@depends-on: /absolute/path.md#task:setup"),
            example_good: Some("@depends-on: ../shared/tasks.md#task:setup"),
        }),

        codes::E_INDEX_FILE_MISSING => Some(ErrorExplanation {
            code: codes::E_INDEX_FILE_MISSING,
            summary: "Root index references a file that does not exist",
            description: "An entry in the root index (`lash.index.md` or `index.lash.md`) points at a Markdown file that is not on disk. This is the mirror image of W_INDEX_ORPHAN: there the file exists but is not listed, here it is listed but does not exist.",
            why_it_matters: "The root index is the map of the project. An entry pointing at nothing sends readers and agents to a file that is not there, usually after a rename or delete that missed the index.",
            how_to_fix: "Fix the path if the file moved, or remove the entry if the file is gone.",
            example_bad: Some("## Tasks\n\n- [ ] [Backend](tasks/backend.md)   # file was renamed to tasks/api.md"),
            example_good: Some("## Tasks\n\n- [ ] [Backend](tasks/api.md)"),
        }),

        codes::W_INDEX_ORPHAN => Some(ErrorExplanation {
            code: codes::W_INDEX_ORPHAN,
            summary: "Markdown file is not referenced in the root index",
            description: "Lash found a Markdown file in the project that the root index (`lash.index.md` or `index.lash.md`) does not link to. Common documentation names (README.md, CHANGELOG.md, CONTRIBUTING.md, devlog.md and similar) and files under `docs/`, `doc/`, `documentation/` and `.github/` are exempt already.",
            why_it_matters: "The root index is how humans and agents discover task files. A task file missing from it is invisible to anyone starting from the index — and it is easy to create one by adding a file and forgetting the index entry.",
            how_to_fix: "Add a link to the file in the root index. If the file is not a task file at all — prose, notes, generated content — add its path or directory to a `.lashignore` at the project root and Lash will stop walking it entirely (one glob per line, `.gitignore` syntax, so `content/` excludes a whole directory).",
            example_bad: Some("# project layout\nlash.index.md      # links only tasks/backend.md\ntasks/backend.md\ncontent/a-post.md  # warns: prose, never a task file"),
            example_good: Some("# .lashignore\ncontent/\n\n# or, in lash.index.md:\n- [ ] [Notes](content/a-post.md)"),
        }),

        _ => None,
    }
}
