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
