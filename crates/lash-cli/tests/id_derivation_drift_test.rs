//! Integration tests for task-ID derivation drift (GitHub issue #54)
//!
//! When the rules that derive a task ID from its title change, every unpinned
//! ID moves. Nothing in the Markdown records the old value, and incremental
//! indexing keys off content hashes — so a file nobody has touched keeps
//! serving IDs derived under rules that are no longer in force, and
//! `check-index` calls that in sync.
//!
//! These tests stand in for that state the only way it can honestly be
//! reached: by writing a stored ID that the current rules would not derive,
//! and clearing the version stamp that says which rules built the index.

mod common;

use common::{run_lash_command, TestProject};
use predicates::prelude::*;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

/// A project whose index file depends on a task by its derived ID.
///
/// The title slugs to `founder-add-releases-mirror-token-secret` under the
/// current rules and to `founder-add-releasesmirrortoken-secret-t` under the
/// pre-0.3.0 ones — underscores used to vanish rather than become separators.
fn project_with_a_reference() -> TestProject {
    TestProject::builder()
        .with_file(
            "lash.index.md",
            r"# Test Project

@id: index
@created: 2024-01-15

## Tasks

- [ ] Founder: add RELEASES_MIRROR_TOKEN secret
- [ ] Merge the mirror PR
  @depends-on: index#founder-add-releasesmirrortoken-secret-t
",
        )
        .build()
}

const LEGACY_ID: &str = "founder-add-releasesmirrortoken-secret-t";
const CURRENT_ID: &str = "founder-add-releases-mirror-token-secret";

fn index(project: &TestProject) {
    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();
}

fn db(project: &TestProject) -> Connection {
    Connection::open(project.path().join(".lash/lash.db")).expect("index should be readable")
}

/// Rewrite a stored ID to the value the old rules produced, and forget which
/// rules built the index — the state an upgrade leaves behind.
fn simulate_pre_upgrade_index(project: &TestProject) {
    let conn = db(project);
    conn.execute(
        "UPDATE tasks SET local_id = ?1, full_id = 'index#' || ?1 WHERE local_id = ?2",
        [LEGACY_ID, CURRENT_ID],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM metadata WHERE key = 'id_derivation_version'",
        [],
    )
    .unwrap();
}

fn stored_ids(project: &TestProject) -> Vec<String> {
    let conn = db(project);
    let mut stmt = conn.prepare("SELECT local_id FROM tasks").unwrap();
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    ids
}

fn index_file(project: &TestProject) -> String {
    fs::read_to_string(project.file_path("lash.index.md")).unwrap()
}

// ---------------------------------------------------------------------
// The index repairs itself
// ---------------------------------------------------------------------

#[test]
fn test_incremental_index_re_derives_ids_when_the_rules_changed() {
    // The file has not changed, so hash comparison finds nothing to do —
    // which is exactly why keying re-derivation on hashes alone leaves the
    // stale ID in place indefinitely.
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);

    assert!(stored_ids(&project).contains(&LEGACY_ID.to_string()));

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success();

    let ids = stored_ids(&project);
    assert!(
        ids.contains(&CURRENT_ID.to_string()),
        "expected the current ID to be stored, got: {ids:?}"
    );
    assert!(
        !ids.contains(&LEGACY_ID.to_string()),
        "the stale ID should be gone, got: {ids:?}"
    );
}

#[test]
fn test_index_reports_the_ids_it_moved() {
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 task ID changed"))
        .stdout(predicate::str::contains(LEGACY_ID))
        .stdout(predicate::str::contains(CURRENT_ID))
        .stdout(predicate::str::contains("lash migrate-ids"));
}

#[test]
fn test_an_ordinary_index_says_nothing_about_derivation() {
    // Warning when nothing moved would train people to ignore the warning.
    let project = project_with_a_reference();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("task ID changed").not())
        .stdout(predicate::str::contains("older ID rules").not());

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("older ID rules").not());
}

#[test]
fn test_the_repair_does_not_repeat_itself() {
    // The version stamp is what stops the next run from re-deriving
    // everything again, and re-recording the same rename.
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);

    index(&project);
    index(&project);

    let conn = db(&project);
    let pending: i64 = conn
        .query_row("SELECT COUNT(*) FROM id_migrations", [], |row| row.get(0))
        .unwrap();
    assert_eq!(pending, 1, "the same rename must not accumulate");
}

// ---------------------------------------------------------------------
// check-index reports the drift instead of passing
// ---------------------------------------------------------------------

#[test]
fn test_check_index_reports_stale_ids() {
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .failure()
        .stdout(predicate::str::contains("Stale task IDs"))
        .stdout(predicate::str::contains("Index is in sync").not());
}

#[test]
fn test_check_index_names_the_stale_id_with_diff() {
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .arg("--diff")
        .assert()
        .failure()
        .stdout(predicate::str::contains(LEGACY_ID))
        .stdout(predicate::str::contains("lash migrate-ids"));
}

#[test]
fn test_check_index_still_passes_on_a_healthy_index() {
    let project = project_with_a_reference();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Stale task IDs").not());
}

// ---------------------------------------------------------------------
// lint names the cause
// ---------------------------------------------------------------------

#[test]
fn test_lint_explains_a_reference_the_index_still_recognises() {
    // The reporter's confusion: `lash show` prints the ID that lint has just
    // rejected, so lint reads as the thing that is wrong.
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .code(2)
        .stdout(predicate::str::contains("a derivation change moved"))
        .stdout(predicate::str::contains("lash index"));
}

#[test]
fn test_lint_explains_a_reference_with_a_rename_pending() {
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);
    index(&project); // re-derives, records the rename

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .code(2)
        .stdout(predicate::str::contains("a derivation change moved"))
        .stdout(predicate::str::contains("lash migrate-ids --write"));
}

#[test]
fn test_lint_says_nothing_extra_about_an_ordinary_broken_reference() {
    // A genuine typo must not be dressed up as a derivation change.
    let project = TestProject::builder()
        .with_file(
            "lash.index.md",
            r"# Test Project

@id: index
@created: 2024-01-15

## Tasks

- [ ] Real task
- [ ] Dependent task
  @depends-on: index#no-such-task-ever
",
        )
        .build();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .code(2)
        .stdout(predicate::str::contains("a derivation change moved").not());
}

// ---------------------------------------------------------------------
// migrate-ids rewrites the references
// ---------------------------------------------------------------------

#[test]
fn test_migrate_ids_previews_without_writing() {
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);
    index(&project);

    let before = index_file(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .assert()
        .failure() // pending work is unfinished work
        .stdout(predicate::str::contains("Would rewrite 1 reference"))
        .stdout(predicate::str::contains("Nothing has been written"));

    assert_eq!(index_file(&project), before, "preview must not write");
}

#[test]
fn test_migrate_ids_write_rewrites_the_reference() {
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .arg("--write")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rewrote 1 reference"));

    let content = index_file(&project);
    assert!(
        content.contains(&format!("@depends-on: index#{CURRENT_ID}")),
        "expected the rewritten reference, got:\n{content}"
    );
    assert!(
        !content.contains(LEGACY_ID),
        "the old ID should be gone, got:\n{content}"
    );
}

#[test]
fn test_lint_passes_after_migrating() {
    // The whole point: the four commands stop disagreeing.
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .arg("--write")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("lint")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("check-index")
        .assert()
        .success();
}

#[test]
fn test_migrate_ids_is_a_no_op_when_nothing_is_pending() {
    let project = project_with_a_reference();
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .assert()
        .success()
        .stdout(predicate::str::contains("No task IDs are pending"));
}

#[test]
fn test_migrate_ids_clears_the_pending_list_after_writing() {
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .arg("--write")
        .assert()
        .success();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .assert()
        .success()
        .stdout(predicate::str::contains("No task IDs are pending"));
}

#[test]
fn test_migrate_ids_forget_discards_without_touching_files() {
    // For a project that would rather fix its references by hand, or has
    // already done so.
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);
    index(&project);

    let before = index_file(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .arg("--forget")
        .assert()
        .success()
        .stdout(predicate::str::contains("Discarded 1 pending rename"));

    assert_eq!(index_file(&project), before, "--forget must not write");

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .assert()
        .success()
        .stdout(predicate::str::contains("No task IDs are pending"));
}

#[test]
fn test_migrate_ids_rewrites_a_reference_in_another_file() {
    // The case that makes hand repair expensive: references to a renamed task
    // live wherever anyone wrote them, not next to the task.
    let project = TestProject::builder()
        .with_file(
            "lash.index.md",
            r"# Test Project

@id: index
@created: 2024-01-15

## Tasks

- [ ] Founder: add RELEASES_MIRROR_TOKEN secret
",
        )
        .with_file(
            "other.md",
            &format!(
                r"# Other

@id: other
@created: 2024-01-15

## Tasks

- [ ] Merge the mirror PR
  @depends-on: index#{LEGACY_ID}
"
            ),
        )
        .build();

    index(&project);
    simulate_pre_upgrade_index(&project);
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .arg("--write")
        .assert()
        .success();

    let other = fs::read_to_string(project.file_path("other.md")).unwrap();
    assert!(
        other.contains(&format!("@depends-on: index#{CURRENT_ID}")),
        "expected the cross-file reference rewritten, got:\n{other}"
    );
}

#[test]
fn test_migrate_ids_leaves_prose_mentioning_an_old_id_alone() {
    // Only `@depends-on` lines are references. Rewriting a task's own text
    // would be editing someone's notes.
    let project = TestProject::builder()
        .with_file(
            "lash.index.md",
            &format!(
                r"# Test Project

@id: index
@created: 2024-01-15

## Tasks

- [ ] Founder: add RELEASES_MIRROR_TOKEN secret
- [ ] Merge the mirror PR
  @depends-on: index#{LEGACY_ID}

  The old note here still mentions index#{LEGACY_ID} on purpose.
"
            ),
        )
        .build();

    index(&project);
    simulate_pre_upgrade_index(&project);
    index(&project);

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .arg("--write")
        .assert()
        .success();

    let content = index_file(&project);
    assert!(
        content.contains(&format!(
            "The old note here still mentions index#{LEGACY_ID}"
        )),
        "prose must be untouched, got:\n{content}"
    );
    assert!(
        content.contains(&format!("@depends-on: index#{CURRENT_ID}")),
        "the reference must still be rewritten, got:\n{content}"
    );
}

#[test]
fn test_a_file_edited_since_the_last_index_is_not_guessed_at() {
    // Matching stored rows to parsed tasks is only exact while the file is
    // unchanged. Once it has been edited, the stored rows describe a
    // different arrangement of lines and no rename can be claimed from them —
    // the file is re-indexed on its own hash anyway.
    let project = project_with_a_reference();
    index(&project);
    simulate_pre_upgrade_index(&project);

    let path = project.file_path("lash.index.md");
    let content = fs::read_to_string(&path).unwrap();
    fs::write(&path, format!("{content}- [ ] A task added later\n")).unwrap();

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("task ID changed").not());

    // The stored ID is still corrected — the file was re-parsed either way.
    assert!(stored_ids(&project).contains(&CURRENT_ID.to_string()));
}

#[test]
fn test_migrate_ids_without_an_index_is_not_an_error() {
    let project = project_with_a_reference();
    assert!(!Path::new(&project.file_path(".lash/lash.db")).exists());

    run_lash_command()
        .arg("--root")
        .arg(project.path())
        .arg("migrate-ids")
        .assert()
        .success()
        .stdout(predicate::str::contains("No index found"));
}
