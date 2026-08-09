//! Integration tests for the status-bar activity sections.
//!
//! These cover the two cases `tasks/tasks.status-bar-activity.md` left
//! deferred for want of a TUI harness that did not exist yet: the ordering
//! `seed_from_db` uses to pick between several in-progress tasks, and a
//! toggle driven through the real key path rather than through
//! `ActivityState::record_transition` directly.

use lash_tui::activity::ActivityState;
use lash_tui::testing::{keys, TestAppBuilder};
use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;

/// Build a project containing `body` and index it, returning the temp dir
/// (which must outlive the test), the db path, and the task file path.
fn setup_project(body: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();
    std::fs::create_dir(project_root.join(".lash")).unwrap();
    let db_path = project_root.join(".lash").join("lash.db");

    let conn = lash_db::init_database(&db_path).unwrap();
    std::fs::write(project_root.join("tasks.md"), body).unwrap();

    use lash_db::{Indexer, IndexerConfig};
    let parser_config = lash_types::LashConfig::default();
    let indexer_config = IndexerConfig::new(project_root).with_progress(false);
    let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
    indexer.index_project().unwrap();

    (temp_dir, db_path)
}

#[test]
fn seeding_picks_the_smaller_full_id_when_several_tasks_are_in_progress() {
    // Two in-progress tasks, written in the order that would give the wrong
    // answer if seeding took whichever the DB happened to return first:
    // `zulu` is above `alpha` in the file, so insertion order and full_id
    // order disagree.
    let (_temp_dir, db_path) = setup_project(
        r"# Sample

@id: sample

## Tasks

- [>] Zulu task
  @id: zulu
- [>] Alpha task
  @id: alpha
",
    );
    let conn = lash_db::open_database(&db_path).unwrap();

    let mut activity = ActivityState::default();
    activity.seed_from_db(&conn, Instant::now());

    let seeded = activity
        .in_progress
        .as_ref()
        .expect("an in-progress task should have been seeded");
    assert_eq!(
        seeded.full_id, "sample#alpha",
        "seeding must be deterministic on full_id, not on row order"
    );
}

#[test]
fn seeding_leaves_in_progress_empty_when_nothing_is_in_progress() {
    let (_temp_dir, db_path) = setup_project(
        r"# Sample

@id: sample

## Tasks

- [ ] Open task
- [x] Done task
",
    );
    let conn = lash_db::open_database(&db_path).unwrap();

    let mut activity = ActivityState::default();
    activity.seed_from_db(&conn, Instant::now());

    assert!(activity.in_progress.is_none());
}

/// Drive the app to the Tasks pane with the first task selected, then press
/// Space `toggles` times. Returns the app so the caller can inspect state.
fn app_after_toggles(db_path: &PathBuf, toggles: usize) -> lash_tui::testing::TestTuiApp {
    // Tab twice to move focus Navigation -> Description -> Detail (the tasks
    // pane), which is where Space toggles rather than warning.
    let mut events = vec![keys::tab(), keys::tab()];
    for _ in 0..toggles {
        events.push(keys::char(' '));
    }

    let mut app = TestAppBuilder::new()
        .with_db(db_path)
        .with_size(80, 24)
        .with_events(events)
        .build()
        .unwrap();

    // One tick per queued event, plus the initial render.
    for _ in 0..=(2 + toggles) {
        app.tick().unwrap();
    }
    app
}

#[test]
fn toggling_open_to_in_progress_fills_the_in_progress_slot() {
    let (_temp_dir, db_path) = setup_project(
        r"# Sample

@id: sample

## Tasks

- [ ] Alpha task
  @id: alpha
",
    );

    let app = app_after_toggles(&db_path, 1);

    let in_progress = app
        .state()
        .activity
        .in_progress
        .as_ref()
        .expect("toggling Open -> InProgress should fill the in-progress slot");
    assert_eq!(in_progress.title, "Alpha task");
    assert!(
        app.state().activity.recently_completed.is_empty(),
        "nothing has completed yet"
    );
}

#[test]
fn toggling_in_progress_to_done_clears_the_slot_and_records_the_completion() {
    let (_temp_dir, db_path) = setup_project(
        r"# Sample

@id: sample

## Tasks

- [ ] Alpha task
  @id: alpha
",
    );

    // Open -> InProgress -> Done.
    let app = app_after_toggles(&db_path, 2);

    assert!(
        app.state().activity.in_progress.is_none(),
        "completing the in-progress task must clear the slot"
    );
    let recent = &app.state().activity.recently_completed;
    assert_eq!(recent.len(), 1, "the completion should have been recorded");
    assert_eq!(recent[0].title, "Alpha task");
}
