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
