//! Explanations for the linter's semantic rules
//!
//! These are the codes `lash lint` emits once a file parses and its content is
//! interpreted: `E_SEM_*`, `W_SEM_*`, `I_SEM_*`, and the contextual-note rules
//! (`E_NOTE_*`, `W_NOTE_*`).

use super::ErrorExplanation;
use crate::error::codes;

/// Codes explained by this module
pub(super) const CODES: &[&str] = &[
    codes::E_SEM_DUPLICATE_ID,
    codes::E_SEM_EMPTY_TITLE,
    codes::E_SEM_INVALID_DATE,
    codes::E_SEM_INVALID_DOC,
    codes::E_SEM_INVALID_ESTIMATE,
    codes::E_SEM_INVALID_LABEL,
    codes::E_SEM_DESC_TOO_LONG,
    codes::W_SEM_DESC_TOO_LONG,
    codes::W_SEM_DOC_FRAGMENT,
    codes::W_SEM_OWNER_FORMAT,
    codes::W_SEM_STATUS_INCONSISTENT,
    codes::I_SEM_AUTO_WAIVE,
    codes::E_NOTE_INVALID_INDENT,
    codes::E_NOTE_HAS_CHILDREN,
    codes::E_NOTE_EXCESSIVE_LENGTH,
    codes::W_NOTE_TOO_LONG,
    codes::W_NOTE_AFTER_CHILD_TASKS,
];

/// Look up a semantic rule explanation
pub(super) fn explain(code: &str) -> Option<ErrorExplanation> {
    match code {
        codes::E_SEM_DUPLICATE_ID => Some(ErrorExplanation {
            code: codes::E_SEM_DUPLICATE_ID,
            summary: "Two tasks in the same file share an ID",
            description: "Task IDs must be unique within a file. Either two tasks declare the same `@id`, or two titles derive the same implicit ID (IDs are derived from titles when `@id` is absent).",
            why_it_matters: "An ID is how everything else points at a task: `@depends-on`, `lash show`, `lash complete`. When two tasks answer to one ID, references resolve to whichever comes first and the other task becomes unreachable.",
            how_to_fix: "Give one of the tasks an explicit, distinct `@id`, or reword its title so the derived ID differs.",
            example_bad: Some("- [ ] Set up\n  @id: setup\n- [ ] Set up\n  @id: setup"),
            example_good: Some("- [ ] Set up database\n  @id: setup-database\n- [ ] Set up server\n  @id: setup-server"),
        }),

        codes::E_SEM_EMPTY_TITLE => Some(ErrorExplanation {
            code: codes::E_SEM_EMPTY_TITLE,
            summary: "Task has an empty title",
            description: "A checkbox line has no text after the marker, so the task has nothing to identify it.",
            why_it_matters: "A task with no title has no derived ID and no display text — it is invisible in listings and impossible to reference.",
            how_to_fix: "Write a title after the checkbox, or delete the line if it was left over from editing.",
            example_bad: Some("- [ ]\n- [x]   "),
            example_good: Some("- [ ] Write the migration guide"),
        }),

        codes::E_SEM_INVALID_DATE => Some(ErrorExplanation {
            code: codes::E_SEM_INVALID_DATE,
            summary: "Date annotation is not a valid `YYYY-MM-DD` date",
            description: "A date annotation such as `@created:` is either not in `YYYY-MM-DD` form or names a day that does not exist (for example 2024-02-30).",
            why_it_matters: "Dates that do not parse cannot be sorted or compared, so the task drops out of any date-ordered view.",
            how_to_fix: "Write the date as `YYYY-MM-DD` with a real calendar day.",
            example_bad: Some("@created: 01/15/2024\n@created: 2024-02-30"),
            example_good: Some("@created: 2024-01-15"),
        }),

        codes::E_SEM_INVALID_DOC => Some(ErrorExplanation {
            code: codes::E_SEM_INVALID_DOC,
            summary: "`@doc:` annotation points at a file that does not exist",
            description: "The path in a `@doc:` annotation does not resolve to a file inside the project. Paths are resolved relative to the file containing the annotation.",
            why_it_matters: "A `@doc:` link is a promise that the referenced document exists. A broken one sends readers and agents to a file that is not there — usually the sign of a moved or renamed doc.",
            how_to_fix: "Correct the path, or update it to the document's new location. Use `lash check-links` to find every broken reference at once.",
            example_bad: Some("- [ ] Implement auth\n  @doc: ../docs/moved-away.md"),
            example_good: Some("- [ ] Implement auth\n  @doc: ../docs/auth.md#token-refresh"),
        }),

        codes::E_SEM_INVALID_ESTIMATE => Some(ErrorExplanation {
            code: codes::E_SEM_INVALID_ESTIMATE,
            summary: "`@estimate:` value is not a number followed by a unit",
            description: "Estimates are a number followed by a unit character: `h` (hours), `d` (days), `w` (weeks), `m` (months) or `y` (years).",
            why_it_matters: "Estimates that do not parse cannot be summed or compared, so the task contributes nothing to planning views.",
            how_to_fix: "Write the estimate as a number plus a unit, such as `4h`, `2d` or `1w`.",
            example_bad: Some("@estimate: two days\n@estimate: 4 hours"),
            example_good: Some("@estimate: 4h\n@estimate: 2d"),
        }),

        codes::E_SEM_INVALID_LABEL => Some(ErrorExplanation {
            code: codes::E_SEM_INVALID_LABEL,
            summary: "Label is not lowercase alphanumeric with hyphens or underscores",
            description: "Labels must start with a letter or digit and contain only lowercase letters, digits, hyphens and underscores — whether written inline as `#label` or in a `@labels:` list.",
            why_it_matters: "Labels are the main filtering axis (`lash list --label backend`). Case and punctuation variants split one concept into several labels that never match each other.",
            how_to_fix: "Lowercase the label and replace spaces and punctuation with hyphens. `lash lint --fix` normalizes labels automatically.",
            example_bad: Some("@labels: Front-End, back end!\n- [ ] Task #High-Priority"),
            example_good: Some("@labels: front-end, back-end\n- [ ] Task #high-priority"),
        }),

        codes::E_SEM_DESC_TOO_LONG => Some(ErrorExplanation {
            code: codes::E_SEM_DESC_TOO_LONG,
            summary: "Description exceeds the hard length limit",
            description: "The file's `## Description` section is past the hard limit — twice the recommended length, so 2000 characters unless `linter.description_max_length` in `.lash/config.toml` says otherwise. Between the recommended and hard limits the linter warns with W_SEM_DESC_TOO_LONG; past the hard limit it errors.",
            why_it_matters: "Descriptions are included verbatim in agent prompts and file listings. A very long one crowds out the tasks it was meant to introduce and burns context budget on every run.",
            how_to_fix: "Trim the description to a short orientation paragraph and move the detail into a document referenced with `@doc:`.",
            example_bad: None,
            example_good: None,
        }),

        codes::W_SEM_DESC_TOO_LONG => Some(ErrorExplanation {
            code: codes::W_SEM_DESC_TOO_LONG,
            summary: "Description exceeds the recommended length",
            description: "The file's `## Description` section is past the recommended length (1000 characters by default, set by `linter.description_max_length` in `.lash/config.toml`). This is a warning; at twice that length it becomes E_SEM_DESC_TOO_LONG.",
            why_it_matters: "Long descriptions are read on every listing and included in agent prompts, so length here is paid repeatedly.",
            how_to_fix: "Shorten the description, or move the detail into a document referenced with `@doc:`.",
            example_bad: None,
            example_good: None,
        }),

        codes::W_SEM_DOC_FRAGMENT => Some(ErrorExplanation {
            code: codes::W_SEM_DOC_FRAGMENT,
            summary: "@doc: fragment does not match any heading",
            description: "An @doc annotation references a #fragment that does not exist in the target document. Lash matches fragments against headings using case- and punctuation-insensitive normalization: both the fragment and each heading are lowercased, '-' is treated as whitespace, every non-alphanumeric/non-whitespace character (including '<', '>', '/', '.', '_', '(', ')', and backticks) is stripped *without* introducing a hyphen boundary, and runs of whitespace are collapsed. Two strings match when they reduce to the same canonical form.",
            why_it_matters: "Broken @doc: fragments mean readers (humans and agents) following the link cannot land on the intended section. The lint catches them so they fail loudly instead of silently 404-ing in a renderer that ignores anchors.",
            how_to_fix: "Open the target document, find the heading you want, and write the fragment so it normalizes to the same canonical form. The warning's help text lists existing headings in the target — pick one. Convention: lowercase the heading, replace spaces with '-', and drop punctuation entirely (do not turn '/' or '.' into a hyphen).",
            example_bad: Some("# Heading: Pack manifest (`<pack>/SKILL.md`)\n@doc: ../docs/skills.md#pack-manifest-pack-skill-md\n# (`<` `>` `/` are stripped without producing a boundary, so this slug is wrong)"),
            example_good: Some("# Heading: Pack manifest (`<pack>/SKILL.md`)\n@doc: ../docs/skills.md#pack-manifest-packskillmd\n\n# Heading: Validation rules (must pass at index time)\n@doc: ../docs/skills.md#validation-rules-must-pass-at-index-time"),
        }),

        codes::W_SEM_OWNER_FORMAT => Some(ErrorExplanation {
            code: codes::W_SEM_OWNER_FORMAT,
            summary: "`@owner:` value is empty or implausibly long",
            description: "The owner annotation is blank, or longer than 100 characters — long enough that it is probably a sentence that landed in the wrong annotation.",
            why_it_matters: "Owner is a grouping key (`lash list --owner alice`). Blank or prose-length owners produce groups nobody can filter on.",
            how_to_fix: "Use a short handle or name, or remove the annotation if the task is unassigned. Put explanatory text in `@agent-note:` or a contextual note instead.",
            example_bad: Some("@owner:\n@owner: alice, but only after the infra migration lands and she is back from leave"),
            example_good: Some("@owner: alice"),
        }),

        codes::W_SEM_STATUS_INCONSISTENT => Some(ErrorExplanation {
            code: codes::W_SEM_STATUS_INCONSISTENT,
            summary: "Parent is marked done while a child is still open",
            description: "A parent task is `[x]` but at least one of its children is not done or waived. In Lash a parent is complete only when its children are.",
            why_it_matters: "The parent's status is what rolls up into progress counts. A parent that claims completion over unfinished children makes those counts wrong, and the open children stop appearing as work.",
            how_to_fix: "Close or waive the remaining children, or reopen the parent. `lash lint --fix` can reconcile the statuses.",
            example_bad: Some("- [x] Ship feature\n  - [ ] Write tests"),
            example_good: Some("- [x] Ship feature\n  - [x] Write tests\n\n# or\n\n- [ ] Ship feature\n  - [ ] Write tests"),
        }),

        codes::I_SEM_AUTO_WAIVE => Some(ErrorExplanation {
            code: codes::I_SEM_AUTO_WAIVE,
            summary: "Children of a waived parent can be auto-waived",
            description: "A parent task is waived `[-]` but some children are still open. Waiving a parent means the whole branch does not apply, so the children can be waived too. This is informational and never fails a lint run.",
            why_it_matters: "Children left open under a waived parent keep showing up as outstanding work that nobody intends to do.",
            how_to_fix: "Run `lash format` (or `lash lint --fix`) to waive the children, or waive them by hand. Ignore the suggestion if the children really are still in play — in which case the parent probably should not be waived.",
            example_bad: Some("- [-] Legacy migration (not applicable)\n  - [ ] Migrate table A\n  - [ ] Migrate table B"),
            example_good: Some("- [-] Legacy migration (not applicable)\n  - [-] Migrate table A\n  - [-] Migrate table B"),
        }),

        codes::E_NOTE_INVALID_INDENT => Some(ErrorExplanation {
            code: codes::E_NOTE_INVALID_INDENT,
            summary: "Contextual note is not indented 2 spaces past its task",
            description: "A contextual note is a plain bullet (no checkbox) attached to a task. It must be indented exactly 2 spaces deeper than the task line it belongs to.",
            why_it_matters: "Indentation is what binds a note to its task. At the wrong depth the note attaches to a different task, or to none at all, and stops travelling with the work it describes.",
            how_to_fix: "Indent the note 2 spaces past its task line. `lash format` fixes note indentation.",
            example_bad: Some("- [ ] Implement auth\n- Must support SSO"),
            example_good: Some("- [ ] Implement auth\n  - Must support SSO"),
        }),

        codes::E_NOTE_HAS_CHILDREN => Some(ErrorExplanation {
            code: codes::E_NOTE_HAS_CHILDREN,
            summary: "Contextual note has nested children",
            description: "A contextual note is a leaf: it cannot have bullets or tasks nested under it.",
            why_it_matters: "Nesting under a note is ambiguous — the nested lines belong either to the note's task or to nothing. Keeping notes flat keeps the tree unambiguous.",
            how_to_fix: "Flatten the note into a single bullet, or promote the nested items to sibling notes or real subtasks of the parent task.",
            example_bad: Some("- [ ] Implement auth\n  - Must support SSO\n    - and SAML"),
            example_good: Some("- [ ] Implement auth\n  - Must support SSO\n  - Must support SAML"),
        }),

        codes::E_NOTE_EXCESSIVE_LENGTH => Some(ErrorExplanation {
            code: codes::E_NOTE_EXCESSIVE_LENGTH,
            summary: "Contextual note exceeds the hard length limit",
            description: "A contextual note is longer than 500 characters. Between 200 and 500 characters the linter warns with W_NOTE_TOO_LONG; past 500 it errors.",
            why_it_matters: "Notes are inlined next to their task everywhere the task is shown, including agent prompts. A note this long is a document living in a bullet.",
            how_to_fix: "Cut the note to a sentence or two and move the rest into a document referenced with `@doc:`, or into the file's `## Description`.",
            example_bad: None,
            example_good: None,
        }),

        codes::W_NOTE_TOO_LONG => Some(ErrorExplanation {
            code: codes::W_NOTE_TOO_LONG,
            summary: "Contextual note exceeds the recommended length",
            description: "A contextual note is longer than the recommended 200 characters. Past 500 characters it becomes E_NOTE_EXCESSIVE_LENGTH.",
            why_it_matters: "Notes are shown inline with their task, so a long one pushes the task list off the screen and into the scrollback.",
            how_to_fix: "Shorten the note, split it into two notes, or move the detail into a `@doc:` reference.",
            example_bad: None,
            example_good: None,
        }),

        codes::W_NOTE_AFTER_CHILD_TASKS => Some(ErrorExplanation {
            code: codes::W_NOTE_AFTER_CHILD_TASKS,
            summary: "Contextual note appears after child tasks",
            description: "A task's contextual notes come before its child tasks. This note sits after them.",
            why_it_matters: "Notes read as context for the work that follows. Placed after the children, a note looks like it belongs to the last child rather than to the parent.",
            how_to_fix: "Move the note above the first child task. `lash format` reorders notes for you.",
            example_bad: Some("- [ ] Implement auth\n  - [ ] Add login form\n  - Must support SSO"),
            example_good: Some("- [ ] Implement auth\n  - Must support SSO\n  - [ ] Add login form"),
        }),

        _ => None,
    }
}
