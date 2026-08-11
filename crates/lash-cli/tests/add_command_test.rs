//! Integration tests for the `lash add` command
//!
//! Covers GitHub issues #24 (`--id` silently discarded) and #27
//! (`--depends-on` accepts dangling references silently).

mod common;

use common::{run_lash_command, TestProject};
use predicates::prelude::*;
use std::fs;

/// A minimal project with one existing task file containing one task.
fn project_with_tasks_file() -> TestProject {
    TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Existing task
  @id: existing
"#,
        )
        .build()
}

fn index(project: &TestProject) {
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();
}

// ---------------------------------------------------------------------
// Issue #24: `lash add --id` is silently discarded
// ---------------------------------------------------------------------

#[test]
fn test_add_with_id_writes_annotation_and_resolves_via_show() {
    let project = project_with_tasks_file();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Short e task")
        .arg("--file")
        .arg("tasks.md")
        .arg("--id")
        .arg("short-e")
        .assert()
        .success()
        .stdout(predicate::str::contains("short-e"));

    // The @id annotation must actually be in the markdown, not just echoed
    // in the success message.
    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(
        content.contains("@id: short-e"),
        "expected @id: short-e in file, got:\n{content}"
    );

    // `add` re-indexes, so `tasks#short-e` must resolve immediately.
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("show")
        .arg("tasks#short-e")
        .assert()
        .success()
        .stdout(predicate::str::contains("Short e task"));
}

#[test]
fn test_add_without_id_still_works_and_is_not_persisted() {
    // Regression guard: omitting --id must keep working exactly as before
    // (auto-synthesized id, not written to markdown).
    let project = project_with_tasks_file();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Untitled id task")
        .arg("--file")
        .arg("tasks.md")
        .assert()
        .success();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("- [ ] Untitled id task"));
    assert!(!content.contains("@id: untitled-id-task"));
}

#[test]
fn test_add_with_invalid_id_is_rejected_loudly() {
    let project = project_with_tasks_file();
    index(&project);

    let before = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Bad id task")
        .arg("--file")
        .arg("tasks.md")
        .arg("--id")
        .arg("Not_A_Valid_Slug")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_CREATE_INVALID_ID_FORMAT"));

    let after = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert_eq!(
        before, after,
        "file must be untouched on validation failure"
    );
}

#[test]
fn test_add_with_duplicate_id_is_rejected() {
    let project = project_with_tasks_file();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Duplicate id task")
        .arg("--file")
        .arg("tasks.md")
        .arg("--id")
        .arg("existing") // already used by "Existing task" in the fixture
        .assert()
        .failure();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    // Only one @id: existing must remain — nothing was appended.
    assert_eq!(content.matches("@id: existing").count(), 1);
    assert!(!content.contains("Duplicate id task"));
}

// ---------------------------------------------------------------------
// Issue #27: `lash add --depends-on` accepts dangling references silently
// ---------------------------------------------------------------------

#[test]
fn test_add_depends_on_dangling_ref_is_hard_error_and_writes_nothing() {
    let project = project_with_tasks_file();
    index(&project);

    let before = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Blocked task")
        .arg("--file")
        .arg("tasks.md")
        .arg("--depends-on")
        .arg("does-not-exist")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("E_CREATE_DEPENDENCY_NOT_FOUND"))
        .stderr(predicate::str::contains("does-not-exist"));

    let after = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert_eq!(
        before, after,
        "file must be untouched when --depends-on fails to resolve"
    );
}

#[test]
fn test_add_depends_on_allow_forward_ref_warns_and_writes() {
    let project = project_with_tasks_file();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Forward task")
        .arg("--file")
        .arg("tasks.md")
        .arg("--depends-on")
        .arg("not-yet-created")
        .arg("--allow-forward-ref")
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning"))
        .stderr(predicate::str::contains("not-yet-created"));

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("- [ ] Forward task"));
    assert!(content.contains("@depends-on: not-yet-created"));
}

#[test]
fn test_add_depends_on_resolvable_ref_succeeds_without_warning() {
    let project = project_with_tasks_file();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Dependent task")
        .arg("--file")
        .arg("tasks.md")
        .arg("--depends-on")
        .arg("existing")
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning").not());

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("@depends-on: existing"));
}

#[test]
fn test_add_depends_on_resolves_task_added_earlier_via_explicit_id() {
    // Interaction between #24 and #27: a --depends-on ref can target a task
    // created moments earlier by an explicit --id, since validation runs
    // against on-disk state at add time.
    let project = project_with_tasks_file();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Base step")
        .arg("--file")
        .arg("tasks.md")
        .arg("--id")
        .arg("base-step")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Follow-up step")
        .arg("--file")
        .arg("tasks.md")
        .arg("--depends-on")
        .arg("base-step")
        .assert()
        .success()
        .stderr(predicate::str::contains("E_CREATE_DEPENDENCY_NOT_FOUND").not());

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(content.contains("@depends-on: base-step"));
}

// ---------------------------------------------------------------------
// Multi-line @agent-note preservation
//
// `lash add` counted a multi-line `@agent-note` as a single line when
// computing the append point, so the new task was spliced between the
// note's first line and its continuations. The orphaned continuation
// lines were then destroyed on reindex — silent data loss, exit code 0.
// ---------------------------------------------------------------------

/// A project whose single task carries a three-line `@agent-note`.
fn project_with_multiline_note() -> TestProject {
    TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Existing task
  @id: existing
  @agent-note: line one of the note
  line two of the note
  line three of the note
"#,
        )
        .build()
}

#[test]
fn test_add_preserves_preceding_multiline_agent_note_in_file() {
    let project = project_with_multiline_note();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Second task")
        .arg("--file")
        .arg("tasks.md")
        .assert()
        .success();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    // The note must remain contiguous: all three lines still adjacent, with
    // no task line spliced between them.
    assert!(
        content.contains(
            "  @agent-note: line one of the note\n  line two of the note\n  line three of the note"
        ),
        "multi-line agent note was split apart:\n{content}"
    );

    // And the new task must land after the whole note, not inside it.
    let note_end = content.find("line three of the note").unwrap();
    let new_task = content
        .find("- [ ] Second task")
        .expect("new task missing from file");
    assert!(
        new_task > note_end,
        "new task was inserted inside the preceding note:\n{content}"
    );
}

#[test]
fn test_add_preserves_multiline_agent_note_through_reindex() {
    // The destructive step: even when the on-disk splice looks survivable,
    // reindexing used to drop the orphaned continuation lines entirely.
    let project = project_with_multiline_note();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Second task")
        .arg("--file")
        .arg("tasks.md")
        .assert()
        .success();

    index(&project);

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    for line in [
        "line one of the note",
        "line two of the note",
        "line three of the note",
    ] {
        assert!(
            content.contains(line),
            "note content '{line}' lost after reindex:\n{content}"
        );
    }
}

// ---------------------------------------------------------------------
// Appending to a file whose `## Tasks` section is empty
//
// `resolve_append` used to return the magic line number 0 whenever the
// target file held no tasks, meaning "this is a brand-new file, write at
// offset 0". For a file that already existed with an empty `## Tasks`
// section that was catastrophically wrong: the checkbox was prepended
// above the H1, so the parser never saw it as a task, `lash index` found
// nothing, and `lash lint` still passed. The task was on disk and
// invisible.
// ---------------------------------------------------------------------

/// Line the task line lands on, 1-indexed.
fn task_line_number(content: &str, needle: &str) -> usize {
    content
        .lines()
        .position(|line| line.contains(needle))
        .map(|idx| idx + 1)
        .unwrap_or_else(|| panic!("'{needle}' not found in:\n{content}"))
}

/// Run `lash add`, returning the command's stdout.
fn add_task(project: &TestProject, title: &str, file: &str) -> String {
    let output = run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg(title)
        .arg("--file")
        .arg(file)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(output).unwrap()
}

#[test]
fn test_add_to_empty_tasks_section_lands_below_the_header() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "empty.md",
            r#"# Empty

@id: empty

## Tasks
"#,
        )
        .build();
    index(&project);

    let stdout = add_task(&project, "Ship it", "empty.md");

    let content = fs::read_to_string(project.file_path("empty.md")).unwrap();

    assert_eq!(
        content.lines().next(),
        Some("# Empty"),
        "the H1 must still be the first line:\n{content}"
    );
    // The old sentinel surfaced here too, as a literal `empty.md:0`.
    assert!(
        stdout.contains(&format!(
            "empty.md:{}",
            task_line_number(&content, "- [ ] Ship it")
        )),
        "reported line does not match where the task was written:\n{stdout}"
    );
    assert!(
        task_line_number(&content, "- [ ] Ship it") > task_line_number(&content, "## Tasks"),
        "task must land inside the Tasks section:\n{content}"
    );
}

#[test]
fn test_add_to_empty_tasks_section_is_indexed() {
    // The destructive part of the bug: the task was written to a place the
    // parser does not read, so it existed on disk and nowhere else.
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "empty.md",
            r#"# Empty

@id: empty

## Tasks
"#,
        )
        .build();
    index(&project);

    add_task(&project, "Ship it", "empty.md");
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Ship it"));
}

#[test]
fn test_add_to_empty_tasks_section_keeps_following_sections_intact() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "sections.md",
            r#"# Sections

@id: sections

## Tasks

## Notes

Prose that must survive.
"#,
        )
        .build();
    index(&project);

    add_task(&project, "Ship it", "sections.md");

    let content = fs::read_to_string(project.file_path("sections.md")).unwrap();

    assert_eq!(
        content.lines().next(),
        Some("# Sections"),
        "the H1 must still be the first line:\n{content}"
    );
    assert!(
        content.contains("Prose that must survive."),
        "trailing section content was destroyed:\n{content}"
    );

    let task = task_line_number(&content, "- [ ] Ship it");
    assert!(
        task > task_line_number(&content, "## Tasks")
            && task < task_line_number(&content, "## Notes"),
        "task escaped the Tasks section:\n{content}"
    );
}

#[test]
fn test_add_creating_a_new_file_still_writes_a_well_formed_file() {
    // Guards the other half of the split: the old code used one sentinel for
    // both "new file" and "existing file with an empty section".
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .build();
    index(&project);

    add_task(&project, "Ship it", "fresh.md");

    let content = fs::read_to_string(project.file_path("fresh.md")).unwrap();

    assert!(
        task_line_number(&content, "- [ ] Ship it") > task_line_number(&content, "## Tasks"),
        "task must land inside the Tasks section:\n{content}"
    );

    index(&project);
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Ship it"));
}

#[test]
fn test_add_leaves_the_file_ending_in_a_newline() {
    let project = project_with_tasks_file();
    index(&project);

    add_task(&project, "Second task", "tasks.md");

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(
        content.ends_with('\n'),
        "file ends mid-line, every later diff shows '\\ No newline at end of file':\n{content}"
    );
}

// ---------------------------------------------------------------------
// `--agent-note` with an embedded newline
//
// The emitter built the note with a single `format!`, so a value holding
// a newline was written as one `@agent-note:` line followed by a bare
// unindented line. The parser treats an unindented line as the end of
// the annotation block, so everything after the first line was silently
// dropped, with exit code 0.
// ---------------------------------------------------------------------

#[test]
fn test_add_multiline_agent_note_survives_a_round_trip() {
    let project = project_with_tasks_file();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Noted task")
        .arg("--file")
        .arg("tasks.md")
        .arg("--id")
        .arg("noted")
        .arg("--agent-note")
        .arg("first line\nsecond line")
        .assert()
        .success();

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(
        content.contains("  @agent-note: first line\n  second line"),
        "continuation line was written without indentation:\n{content}"
    );

    // The parser has to read both lines back, not just the first.
    index(&project);
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("show")
        .arg("tasks#noted")
        .assert()
        .success()
        .stdout(predicate::str::contains("first line"))
        .stdout(predicate::str::contains("second line"));
}

#[test]
fn test_add_rejects_an_agent_note_with_a_blank_line() {
    let project = project_with_tasks_file();
    index(&project);

    let before = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Blank note task")
        .arg("--file")
        .arg("tasks.md")
        .arg("--agent-note")
        .arg("first line\n\nthird line")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_CREATE_INVALID_AGENT_NOTE"));

    assert_eq!(
        before,
        fs::read_to_string(project.file_path("tasks.md")).unwrap(),
        "file must be untouched when the note is rejected"
    );
}

#[test]
fn test_add_rejects_an_agent_note_line_starting_with_an_annotation() {
    let project = project_with_tasks_file();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("At note task")
        .arg("--file")
        .arg("tasks.md")
        .arg("--agent-note")
        .arg("first line\n@owner: someone")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_CREATE_INVALID_AGENT_NOTE"));

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();
    assert!(!content.contains("At note task"));
}

// ---------------------------------------------------------------------
// `@depends-on` written in comma form
//
// `count_annotation_lines` added one line per parsed dependency, which
// holds for what the emitter writes but not for a hand-written
// `@depends-on: a, b, c`. The parser splits that single line into three
// references, so the count ran two lines long and the insertion point
// escaped the task block, landing the new task outside `## Tasks`.
// ---------------------------------------------------------------------

#[test]
fn test_add_after_comma_form_depends_on_stays_inside_the_tasks_section() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Alpha
- [ ] Beta
- [ ] Gamma
- [ ] Last task
  @depends-on: alpha, beta, gamma

## Notes

Prose that must stay under its heading.
"#,
        )
        .build();
    index(&project);

    add_task(&project, "Second task", "tasks.md");

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    let task = task_line_number(&content, "- [ ] Second task");
    assert!(
        task > task_line_number(&content, "@depends-on: alpha, beta, gamma"),
        "task must land after the dependency line:\n{content}"
    );
    assert!(
        task < task_line_number(&content, "## Notes"),
        "task escaped the Tasks section:\n{content}"
    );
    assert!(
        content.contains("## Notes\n\nProse that must stay under its heading."),
        "the Notes heading was split from its prose:\n{content}"
    );
}

#[test]
fn test_add_after_one_dependency_per_line_still_clears_the_block() {
    // The other half of the ambiguity: the same three references written the
    // way the emitter writes them occupy three lines, not one.
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Alpha
- [ ] Beta
- [ ] Gamma
- [ ] Last task
  @depends-on: alpha
  @depends-on: beta
  @depends-on: gamma

## Notes

Prose that must stay under its heading.
"#,
        )
        .build();
    index(&project);

    add_task(&project, "Second task", "tasks.md");

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    let task = task_line_number(&content, "- [ ] Second task");
    assert!(
        task > task_line_number(&content, "@depends-on: gamma"),
        "task was spliced into the dependency block:\n{content}"
    );
    assert!(
        task < task_line_number(&content, "## Notes"),
        "task escaped the Tasks section:\n{content}"
    );
}

#[test]
fn test_add_after_a_task_whose_labels_use_the_block_form() {
    // `@labels:` used to be uncountable for the same reason, since labels can
    // also live inline on the checkbox line.
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Last task
  @labels: backend, infra

## Notes

Prose that must stay under its heading.
"#,
        )
        .build();
    index(&project);

    add_task(&project, "Second task", "tasks.md");

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    let task = task_line_number(&content, "- [ ] Second task");
    assert!(
        task > task_line_number(&content, "@labels: backend, infra"),
        "task was spliced above the labels line:\n{content}"
    );
    assert!(
        task < task_line_number(&content, "## Notes"),
        "task escaped the Tasks section:\n{content}"
    );
}

// ---------------------------------------------------------------------
// A task's body must stay with its task (GitHub issue #48)
//
// The parser records a task's checkbox line and its annotation lines and
// nothing else, so free-text body under a task — prose, numbered steps,
// acceptance criteria — was invisible when the insertion point was
// computed. `lash add` landed the new task between the previous task's
// title and its own body, silently reassigning that body to the new task.
// `lash lint` passes on the result and `lash show` does not print bodies,
// so nothing surfaced the damage.
// ---------------------------------------------------------------------

#[test]
fn test_add_does_not_steal_the_previous_tasks_body() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "polish.md",
            r#"# Repro

@id: polish

## Tasks

- [x] First task @id:rep-first
  A one-line body belonging to the first task.

- [x] Second task @id:rep-second
  This body belongs to the SECOND task and must stay attached to it.
    1. First numbered point.
    2. Second numbered point.
  Acceptance: this paragraph still reads as part of the second task.
"#,
        )
        .build();
    index(&project);

    let stdout = add_task(&project, "New task", "polish.md");

    let content = fs::read_to_string(project.file_path("polish.md")).unwrap();

    let task = task_line_number(&content, "- [ ] New task");
    assert!(
        task > task_line_number(&content, "Acceptance: this paragraph"),
        "the new task split the second task from its body:\n{content}"
    );
    assert!(
        content.contains(
            "- [x] Second task @id:rep-second\n  This body belongs to the SECOND task and must stay attached to it."
        ),
        "the second task lost its body:\n{content}"
    );

    // The reported line must be where the task actually is, not where the
    // resolver first guessed.
    let reported: usize = stdout
        .lines()
        .find_map(|line| line.rsplit_once(':'))
        .and_then(|(_, line_num)| line_num.trim().parse().ok())
        .unwrap_or_else(|| panic!("no line number in:\n{stdout}"));
    assert_eq!(reported, task, "reported line disagrees with the file");
}

#[test]
fn test_add_does_not_steal_a_body_split_into_paragraphs() {
    // A blank line inside a body does not end the block; indented content
    // resumes after it.
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Only task
  First paragraph of the body.

  Second paragraph of the body.

## Notes

Prose that must stay under its heading.
"#,
        )
        .build();
    index(&project);

    add_task(&project, "Second task", "tasks.md");

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    let task = task_line_number(&content, "- [ ] Second task");
    assert!(
        task > task_line_number(&content, "Second paragraph of the body."),
        "the new task split the body's paragraphs:\n{content}"
    );
    assert!(
        task < task_line_number(&content, "## Notes"),
        "task escaped the Tasks section:\n{content}"
    );
}

#[test]
fn test_add_does_not_steal_a_tasks_contextual_notes() {
    // Indented plain bullets are contextual notes on the task above them.
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Only task
  - a contextual note
  - another contextual note
"#,
        )
        .build();
    index(&project);

    add_task(&project, "Second task", "tasks.md");

    let content = fs::read_to_string(project.file_path("tasks.md")).unwrap();

    assert!(
        task_line_number(&content, "- [ ] Second task")
            > task_line_number(&content, "- another contextual note"),
        "the new task was spliced into the note bullets:\n{content}"
    );
}

// ---------------------------------------------------------------------
// The reported task ID must be the indexed one
//
// `lash add` printed an ID derived by the emitter while the index stored
// one derived by the parser, and the two disagreed three ways: `v0.7.0`
// slugged to `v0-7-0` in one and `v070` in the other, the parser folded
// inline labels into the ID, and only the parser truncated at 40
// characters. Anything that copied the printed ID (`lash show`,
// `@depends-on`) failed.
// ---------------------------------------------------------------------

/// The ID out of `Created task <id> ...`.
fn reported_id(stdout: &str) -> String {
    stdout
        .lines()
        .find_map(|line| line.split("Created task ").nth(1))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("no created-task line in:\n{stdout}"))
        .to_string()
}

#[test]
fn test_add_reports_the_id_the_index_stores() {
    let project = project_with_tasks_file();
    index(&project);

    // Every discrepancy from the original report in one title: a dotted
    // version number, an inline label, and enough characters to truncate.
    let stdout = run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Ship v0.7.0 release notes")
        .arg("--file")
        .arg("tasks.md")
        .arg("--label")
        .arg("docs")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id = reported_id(&String::from_utf8(stdout).unwrap());

    assert_eq!(id, "ship-v0-7-0-release-notes");

    index(&project);
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("show")
        .arg(format!("tasks#{id}"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Ship v0.7.0 release notes"));
}

#[test]
fn test_add_reports_the_truncated_id_for_a_long_title() {
    let project = project_with_tasks_file();
    index(&project);

    let stdout = run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Coverage fail fast when the coverage command fails and its output predates the run")
        .arg("--file")
        .arg("tasks.md")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id = reported_id(&String::from_utf8(stdout).unwrap());

    assert_eq!(
        id.chars().count(),
        40,
        "reported ID was not truncated: {id}"
    );

    index(&project);
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("show")
        .arg(format!("tasks#{id}"))
        .assert()
        .success();
}

#[test]
fn test_add_reports_the_numeric_suffix_used_to_break_a_collision() {
    // Only the parser knows about the suffix, so this is the case the
    // emitter could not get right on its own.
    let project = project_with_tasks_file();
    index(&project);

    add_task(&project, "Repeated title", "tasks.md");
    let stdout = add_task(&project, "Repeated title", "tasks.md");
    let id = reported_id(&stdout);

    assert_eq!(id, "repeated-title-2");

    index(&project);
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("show")
        .arg("tasks#repeated-title-2")
        .assert()
        .success();
}

#[test]
fn test_add_reported_id_resolves_as_a_dependency_target() {
    // The consequence that made this more than an ergonomic problem: a
    // `@depends-on` written against the reported ID used to dangle, and the
    // dangling-reference check rejects it.
    let project = project_with_tasks_file();
    index(&project);

    let stdout = add_task(&project, "Ship v0.7.0 release notes", "tasks.md");
    let id = reported_id(&stdout);
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Follow-up task")
        .arg("--file")
        .arg("tasks.md")
        .arg("--depends-on")
        .arg(&id)
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning").not());
}

// ---------------------------------------------------------------------
// Issue #53: `--before`/`--after` reject the qualified ID lash prints,
// and `--dry-run` does not resolve the position at all
// ---------------------------------------------------------------------

/// A two-task file, so there is something to position against.
fn project_with_two_tasks() -> TestProject {
    TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_file(
            "tasks.md",
            r#"# Tasks

@id: tasks

## Tasks

- [ ] Alpha task
- [ ] Beta task
"#,
        )
        .build()
}

/// The order top-level task titles appear in `tasks.md`.
fn task_titles(project: &TestProject) -> Vec<String> {
    fs::read_to_string(project.file_path("tasks.md"))
        .unwrap()
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("- [ ] ").map(str::to_string))
        .collect()
}

#[test]
fn test_add_before_accepts_the_qualified_id_that_show_prints() {
    // `lash show` reports `tasks#beta-task`; pasting that back into --before
    // used to fail with "task not found" even though the task existed.
    let project = project_with_two_tasks();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Gamma")
        .arg("--file")
        .arg("tasks.md")
        .arg("--before")
        .arg("tasks#beta-task")
        .assert()
        .success();

    assert_eq!(
        task_titles(&project),
        vec!["Alpha task", "Gamma", "Beta task"],
        "Gamma should sit between Alpha and Beta"
    );
}

#[test]
fn test_add_after_accepts_the_qualified_id_that_show_prints() {
    let project = project_with_two_tasks();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Gamma")
        .arg("--file")
        .arg("tasks.md")
        .arg("--after")
        .arg("tasks#alpha-task")
        .assert()
        .success();

    assert_eq!(
        task_titles(&project),
        vec!["Alpha task", "Gamma", "Beta task"]
    );
}

#[test]
fn test_add_before_still_accepts_the_bare_slug() {
    // The form that already worked must keep working.
    let project = project_with_two_tasks();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Gamma")
        .arg("--file")
        .arg("tasks.md")
        .arg("--before")
        .arg("beta-task")
        .assert()
        .success();

    assert_eq!(
        task_titles(&project),
        vec!["Alpha task", "Gamma", "Beta task"]
    );
}

#[test]
fn test_add_before_accepts_the_file_path_as_qualifier() {
    // `tasks.md#beta-task` is the other spelling a caller can have in hand,
    // since `@depends-on` references are written against paths.
    let project = project_with_two_tasks();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Gamma")
        .arg("--file")
        .arg("tasks.md")
        .arg("--before")
        .arg("tasks.md#task:beta-task")
        .assert()
        .success();

    assert_eq!(
        task_titles(&project),
        vec!["Alpha task", "Gamma", "Beta task"]
    );
}

#[test]
fn test_add_before_rejects_a_qualifier_naming_a_different_file() {
    // Accepting the qualifier must not mean ignoring it: a qualifier naming
    // another file means the caller expected the task somewhere it is not,
    // and inserting next to a same-named task here would be wrong.
    let project = project_with_two_tasks();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Gamma")
        .arg("--file")
        .arg("tasks.md")
        .arg("--before")
        .arg("index#beta-task")
        .assert()
        .failure()
        .stderr(predicate::str::contains("names file 'index'"));

    assert_eq!(
        task_titles(&project),
        vec!["Alpha task", "Beta task"],
        "nothing should have been written"
    );
}

#[test]
fn test_add_dry_run_fails_on_a_position_that_does_not_exist() {
    // Dry run used to echo the requested position back and exit 0, so it
    // passed for arguments the real add rejected.
    let project = project_with_two_tasks();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Epsilon")
        .arg("--file")
        .arg("tasks.md")
        .arg("--before")
        .arg("no-such-task-at-all")
        .arg("--dry-run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"))
        .stdout(predicate::str::contains("Validation passed").not());
}

#[test]
fn test_add_dry_run_error_names_the_ids_that_do_exist() {
    let project = project_with_two_tasks();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Epsilon")
        .arg("--file")
        .arg("tasks.md")
        .arg("--before")
        .arg("no-such-task-at-all")
        .arg("--dry-run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("alpha-task"))
        .stderr(predicate::str::contains("beta-task"));
}

#[test]
fn test_add_dry_run_reports_the_resolved_insert_line() {
    // The point of dry run is to check placement, so it has to report the
    // placement it resolved rather than the argument it was handed.
    let project = project_with_two_tasks();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Gamma")
        .arg("--file")
        .arg("tasks.md")
        .arg("--before")
        .arg("tasks#beta-task")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Validation passed"))
        // `- [ ] Beta task` is on line 8 of the fixture.
        .stdout(predicate::str::contains("Insert at: line 8"));

    assert_eq!(
        task_titles(&project),
        vec!["Alpha task", "Beta task"],
        "dry run must not write"
    );
}

#[test]
fn test_add_dry_run_still_fails_on_other_validation_errors() {
    // Position resolution is the new check, but dry run must keep catching
    // everything it caught before it reached the file at all.
    let project = project_with_two_tasks();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Gamma")
        .arg("--file")
        .arg("tasks.md")
        .arg("--estimate")
        .arg("not-a-duration")
        .arg("--dry-run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_CREATE_INVALID_ESTIMATE"));
}

#[test]
fn test_add_dry_run_passes_for_a_plain_append() {
    let project = project_with_two_tasks();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("add")
        .arg("Gamma")
        .arg("--file")
        .arg("tasks.md")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Validation passed"));

    assert_eq!(
        task_titles(&project),
        vec!["Alpha task", "Beta task"],
        "dry run must not write"
    );
}
