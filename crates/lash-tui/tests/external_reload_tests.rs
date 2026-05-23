//! Integration tests for live-update reload from external file changes.
//!
//! These tests bypass the OS file watcher and drive the reload pipeline
//! directly via `process_external_change`, so they're deterministic and
//! don't depend on inotify timing.

use lash_tui::testing::TestAppBuilder;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_tasks(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
}

fn setup_project(initial_body: &str) -> (TempDir, PathBuf, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();
    let lash_dir = project_root.join(".lash");
    std::fs::create_dir(&lash_dir).unwrap();
    let db_path = lash_dir.join("lash.db");

    let conn = lash_db::init_database(&db_path).unwrap();
    let tasks_md = project_root.join("tasks.md");
    write_tasks(&tasks_md, initial_body);

    use lash_db::{Indexer, IndexerConfig};
    use lash_types::LashConfig;
    let parser_config = LashConfig::default();
    let indexer_config = IndexerConfig::new(project_root.clone()).with_progress(false);
    let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
    indexer.index_project().unwrap();

    (temp_dir, db_path, tasks_md)
}

const INITIAL_BODY: &str = r"# Sample Tasks

@id: sample

## Tasks

- [ ] Alpha task
- [ ] Beta task
- [ ] Gamma task
";

const BODY_WITH_INSERT_ABOVE: &str = r"# Sample Tasks

@id: sample

## Tasks

- [ ] Brand new task
- [ ] Alpha task
- [ ] Beta task
- [ ] Gamma task
";

const BODY_WITH_TWO_DONE: &str = r"# Sample Tasks

@id: sample

## Tasks

- [x] Already done task
- [x] Another done task
- [ ] Open task
";

#[test]
fn external_edit_to_one_file_does_not_wipe_other_files_from_db() {
    // Regression for the data-loss bug that crashed soak's progress bar
    // from 76% to 0% on every external edit. The TUI's handle_file_reloaded
    // ran an incremental reindex scoped to just the changed file; the
    // diff layer (before the fix) interpreted "every other file in the DB
    // wasn't observed this walk" as "every other file was deleted" and
    // CASCADE-removed all their tasks.
    //
    // After the fix, sibling files survive an external edit to one.
    let temp = TempDir::new().unwrap();
    let project_root = temp.path().to_path_buf();
    let lash_dir = project_root.join(".lash");
    std::fs::create_dir(&lash_dir).unwrap();
    let db_path = lash_dir.join("lash.db");
    let conn = lash_db::init_database(&db_path).unwrap();

    // Three real files, like a multi-phase project.
    let phase0 = project_root.join("phase0.md");
    let phase1 = project_root.join("phase1.md");
    let phase2 = project_root.join("phase2.md");
    std::fs::write(
        &phase0,
        "# Phase 0\n\n@id: phase0\n\n## Tasks\n\n- [x] p0-a\n- [x] p0-b\n",
    )
    .unwrap();
    std::fs::write(
        &phase1,
        "# Phase 1\n\n@id: phase1\n\n## Tasks\n\n- [ ] p1-a\n- [ ] p1-b\n",
    )
    .unwrap();
    std::fs::write(
        &phase2,
        "# Phase 2\n\n@id: phase2\n\n## Tasks\n\n- [ ] p2-a\n",
    )
    .unwrap();

    use lash_db::{Indexer, IndexerConfig};
    use lash_types::LashConfig;
    let parser_config = LashConfig::default();
    let indexer_config = IndexerConfig::new(project_root.clone()).with_progress(false);
    let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
    indexer.index_project().unwrap();
    drop(conn);

    // Baseline: 5 tasks across 3 files.
    {
        let conn = lash_db::open_database(&db_path).unwrap();
        let task_repo = lash_db::TaskRepository::new(&conn);
        let (total, _completed) = task_repo.get_project_counts().unwrap();
        assert_eq!(total, 5, "baseline total tasks");
    }

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();
    app.tick().unwrap();

    // External edit to phase1 only.
    std::fs::write(
        &phase1,
        "# Phase 1\n\n@id: phase1\n\n## Tasks\n\n- [x] p1-a\n- [ ] p1-b\n",
    )
    .unwrap();
    app.process_external_change(&phase1).unwrap();

    // Phase0 and Phase2 must still be in the DB with their tasks intact.
    // Pre-fix this would have dropped to ~2 tasks (just phase1's).
    let task_repo = lash_db::TaskRepository::new(app.conn_for_tests());
    let (total, _completed) = task_repo.get_project_counts().unwrap();
    assert_eq!(
        total, 5,
        "external edit to phase1 must NOT wipe phase0/phase2 tasks; expected 5 total, got {total}"
    );

    // Spot-check by id: a known phase0 task still exists.
    let p0_a = task_repo.get_by_full_id("phase0#p0-a").unwrap();
    assert!(
        p0_a.is_some(),
        "phase0#p0-a must still be in DB after reload of phase1"
    );
}

#[test]
fn startup_backfills_recently_completed_even_for_old_files() {
    // Regression: an earlier version of the backfill required file mtime to
    // be within the in-memory TTL (5 min). For any project whose task files
    // hadn't been touched in 5 minutes — e.g. opening lash tui in a
    // quiescent project — the bar showed as empty, looking like a bug.
    // Backfill must use the most-recently-completed-by-file-mtime entries
    // regardless of absolute age.
    let (_dir, db_path, tasks_md) = setup_project(BODY_WITH_TWO_DONE);

    // Backdate the file mtime to a week ago so the "since < 5 min" filter
    // would have hidden it.
    let week_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(7 * 24 * 3600);
    filetime::set_file_mtime(&tasks_md, filetime::FileTime::from_system_time(week_ago)).unwrap();
    // Re-index so the new mtime lands in the DB.
    use lash_db::{Indexer, IndexerConfig};
    use lash_types::LashConfig;
    let conn = lash_db::open_database(&db_path).unwrap();
    let parser_config = LashConfig::default();
    let indexer_config = IndexerConfig::new(tasks_md.parent().unwrap().to_path_buf())
        .with_incremental(false)
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
    indexer.index_project().unwrap();
    drop(conn);

    let app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();

    let titles: Vec<&str> = app
        .state()
        .activity
        .recently_completed
        .iter()
        .map(|e| e.title.as_str())
        .collect();
    assert!(
        titles.contains(&"Already done task"),
        "backfill must include done tasks even from week-old files; got {titles:?}"
    );
    assert!(
        titles.contains(&"Another done task"),
        "backfill must include done tasks even from week-old files; got {titles:?}"
    );
}

#[test]
fn startup_backfills_recently_completed_from_db() {
    // Project has two Done tasks on disk before the TUI ever launches.
    // The build step should populate activity.recently_completed from the
    // DB so the bar isn't empty on first run.
    let (_dir, db_path, _tasks_md) = setup_project(BODY_WITH_TWO_DONE);

    let app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();

    let titles: Vec<&str> = app
        .state()
        .activity
        .recently_completed
        .iter()
        .map(|e| e.title.as_str())
        .collect();
    assert!(
        titles.contains(&"Already done task"),
        "expected 'Already done task' in backfilled recently_completed; got {titles:?}"
    );
    assert!(
        titles.contains(&"Another done task"),
        "expected 'Another done task' in backfilled recently_completed; got {titles:?}"
    );
    // Open tasks must not appear in recently_completed.
    assert!(
        !titles.contains(&"Open task"),
        "open tasks must not be in recently_completed; got {titles:?}"
    );
}

#[test]
fn external_edit_inserting_above_cursor_keeps_selection_on_same_task() {
    let (_dir, db_path, tasks_md) = setup_project(INITIAL_BODY);

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();

    // Drive one tick to load the initial state, then position cursor on
    // "Beta task" (the middle task).
    app.tick().unwrap();
    // The app loads tasks for the selected file on construction; select it
    // explicitly to make the test independent of default selection logic.
    let beta_full_id = app
        .state()
        .tasks
        .iter()
        .find(|t| t.title == "Beta task")
        .expect("Beta task should be present in initial state")
        .full_id
        .clone();
    let beta_index = app
        .state()
        .tasks
        .iter()
        .position(|t| t.title == "Beta task")
        .unwrap();
    app.state_mut().selected_task_index = beta_index;

    assert_eq!(
        app.state().selected_task().map(|t| t.full_id.clone()),
        Some(beta_full_id.clone()),
        "precondition: cursor should sit on Beta task before the external edit",
    );

    // Externally rewrite the file to insert a new task above Beta.
    write_tasks(&tasks_md, BODY_WITH_INSERT_ABOVE);
    app.process_external_change(&tasks_md).unwrap();

    let selected_after = app
        .state()
        .selected_task()
        .map(|t| t.full_id.clone())
        .expect("cursor should still resolve to a task");
    assert_eq!(
        selected_after, beta_full_id,
        "cursor should still be anchored to Beta task after the external insert"
    );

    let titles: Vec<String> = app.state().tasks.iter().map(|t| t.title.clone()).collect();
    assert!(
        titles.iter().any(|t| t == "Brand new task"),
        "external insert should be reflected in reloaded tasks; got {titles:?}"
    );
}

const BODY_WITH_ALPHA_IN_PROGRESS: &str = r"# Sample Tasks

@id: sample

## Tasks

- [>] Alpha task
- [ ] Beta task
- [ ] Gamma task
";

const BODY_WITH_ALPHA_DONE: &str = r"# Sample Tasks

@id: sample

## Tasks

- [x] Alpha task
- [ ] Beta task
- [ ] Gamma task
";

#[test]
fn external_open_to_in_progress_sets_activity_in_progress() {
    let (_dir, db_path, tasks_md) = setup_project(INITIAL_BODY);

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();
    app.tick().unwrap();

    let alpha_full_id = app
        .state()
        .tasks
        .iter()
        .find(|t| t.title == "Alpha task")
        .unwrap()
        .full_id
        .clone();

    // External edit flips Alpha to in-progress.
    write_tasks(&tasks_md, BODY_WITH_ALPHA_IN_PROGRESS);
    app.process_external_change(&tasks_md).unwrap();

    let in_progress = app
        .state()
        .activity
        .in_progress
        .as_ref()
        .expect("activity.in_progress should be populated after external transition");
    assert_eq!(in_progress.full_id, alpha_full_id);
    assert_eq!(in_progress.title, "Alpha task");
}

#[test]
fn external_open_to_done_pushes_to_recently_completed() {
    let (_dir, db_path, tasks_md) = setup_project(INITIAL_BODY);

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();
    app.tick().unwrap();

    write_tasks(&tasks_md, BODY_WITH_ALPHA_DONE);
    app.process_external_change(&tasks_md).unwrap();

    let recent = &app.state().activity.recently_completed;
    assert_eq!(
        recent.len(),
        1,
        "expected exactly one recently-completed entry"
    );
    assert_eq!(recent[0].title, "Alpha task");
}

#[test]
fn external_in_progress_to_done_clears_in_progress_and_pushes_to_recent() {
    let (_dir, db_path, tasks_md) = setup_project(BODY_WITH_ALPHA_IN_PROGRESS);

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();
    app.tick().unwrap();

    // Precondition: startup seed should have picked Alpha as in_progress.
    assert_eq!(
        app.state()
            .activity
            .in_progress
            .as_ref()
            .map(|e| e.title.as_str()),
        Some("Alpha task")
    );

    write_tasks(&tasks_md, BODY_WITH_ALPHA_DONE);
    app.process_external_change(&tasks_md).unwrap();

    assert!(
        app.state().activity.in_progress.is_none(),
        "in_progress slot should be cleared once Alpha moves to Done externally"
    );
    let recent = &app.state().activity.recently_completed;
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].title, "Alpha task");
}

#[test]
fn open_task_creation_modal_goes_stale_on_external_edit_to_target_file() {
    let (_dir, db_path, tasks_md) = setup_project(INITIAL_BODY);
    let project_root = tasks_md.parent().unwrap().to_path_buf();

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();
    app.tick().unwrap();

    // Open a creation modal targeting tasks.md (the project-relative path).
    let relative_target = tasks_md.strip_prefix(&project_root).unwrap().to_path_buf();
    app.state_mut()
        .open_task_creation_modal(relative_target, Vec::new());

    assert!(
        !app.state()
            .task_creation_modal_state
            .as_ref()
            .unwrap()
            .stale,
        "modal should start fresh"
    );

    // External rewrite of the same file the modal is targeting.
    write_tasks(&tasks_md, BODY_WITH_INSERT_ABOVE);
    app.process_external_change(&tasks_md).unwrap();

    let modal = app
        .state()
        .task_creation_modal_state
        .as_ref()
        .expect("modal should remain open");
    assert!(
        modal.stale,
        "modal should be marked stale after external edit to its target file"
    );
}

#[test]
fn stale_modal_refuses_submit_and_does_not_overwrite_external_change() {
    let (_dir, db_path, tasks_md) = setup_project(INITIAL_BODY);
    let project_root = tasks_md.parent().unwrap().to_path_buf();

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();
    app.tick().unwrap();

    let relative_target = tasks_md.strip_prefix(&project_root).unwrap().to_path_buf();
    app.state_mut()
        .open_task_creation_modal(relative_target, Vec::new());
    // Give the form a title so can_submit() would have passed otherwise.
    {
        let modal = app.state_mut().task_creation_modal_state.as_mut().unwrap();
        modal.title.set_value("Form contents");
    }

    // External edit lands while the modal is open.
    write_tasks(&tasks_md, BODY_WITH_INSERT_ABOVE);
    app.process_external_change(&tasks_md).unwrap();
    assert!(
        app.state()
            .task_creation_modal_state
            .as_ref()
            .unwrap()
            .stale
    );

    // Capture the on-disk bytes — submit must not overwrite these.
    let bytes_before = std::fs::read_to_string(&tasks_md).unwrap();

    // Try to submit anyway.
    app.handle_submit_task_creation().unwrap();

    let bytes_after = std::fs::read_to_string(&tasks_md).unwrap();
    assert_eq!(
        bytes_after, bytes_before,
        "stale submit must not write to the target file"
    );
    assert!(
        app.state().task_creation_modal_state.is_some(),
        "modal should remain open after refused submit so user can Esc"
    );
    assert!(
        app.state()
            .status_message
            .as_ref()
            .is_some_and(|m| m.text.contains("changed on disk")),
        "expected a 'changed on disk' warning, got {:?}",
        app.state().status_message
    );
}

#[test]
fn external_edit_to_unrelated_file_does_not_mark_modal_stale() {
    let (_dir, db_path, tasks_md) = setup_project(INITIAL_BODY);
    let project_root = tasks_md.parent().unwrap().to_path_buf();
    // Create a second, unrelated task file under the same project root so
    // the watcher path sees a real file change but the modal targets a
    // different one.
    let other_md = project_root.join("other.md");
    write_tasks(
        &other_md,
        "# Other\n\n@id: other\n\n## Tasks\n\n- [ ] something\n",
    );

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();
    app.tick().unwrap();

    let relative_target = tasks_md.strip_prefix(&project_root).unwrap().to_path_buf();
    app.state_mut()
        .open_task_creation_modal(relative_target, Vec::new());

    // Edit a *different* file.
    write_tasks(
        &other_md,
        "# Other\n\n@id: other\n\n## Tasks\n\n- [x] something\n",
    );
    app.process_external_change(&other_md).unwrap();

    let modal = app.state().task_creation_modal_state.as_ref().unwrap();
    assert!(
        !modal.stale,
        "edits to unrelated files must not mark the modal stale"
    );
}

#[test]
fn task_creation_through_store_dedupes_watcher_echo() {
    use lash_types::creation::TaskCreationRequestBuilder;
    use lash_types::LashConfig;

    let (_dir, db_path, tasks_md) = setup_project(INITIAL_BODY);

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();
    app.tick().unwrap();

    let tasks_before = app.state().tasks.len();

    // Drive the same code path the TUI's submit handler uses: Store::apply
    // with a CreateTask mutation. The Store should both perform the write
    // AND record its hash so the subsequent watcher echo is silently dropped.
    let request = TaskCreationRequestBuilder::new("Created via store")
        .file_path(tasks_md.clone())
        .build();
    let deltas = app
        .store_mut()
        .apply(lash_core::store::Mutation::CreateTask(Box::new(
            lash_core::store::CreateTaskMutation {
                request,
                config: LashConfig::default(),
            },
        )))
        .unwrap();
    assert_eq!(deltas.len(), 1);
    assert!(matches!(
        deltas[0],
        lash_core::store::StateDelta::TaskCreated { .. }
    ));

    // Watcher echo: file on disk equals what the Store just wrote.
    // process_external_change must drop it (return no deltas) so the TUI
    // doesn't redundantly reindex a file it just wrote itself.
    app.process_external_change(&tasks_md).unwrap();

    // No reload happened, so state.tasks reflects whatever was there before
    // the creation — the test app doesn't run the submit handler's manual
    // reindex path. (That path is what makes the new task visible in
    // production; here we're isolating the dedupe behaviour.)
    assert_eq!(
        app.state().tasks.len(),
        tasks_before,
        "watcher echo of a Store-mediated create should not have caused a reload"
    );
}

#[test]
fn self_write_echo_is_dropped_without_reload() {
    let (_dir, db_path, tasks_md) = setup_project(INITIAL_BODY);

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();

    app.tick().unwrap();

    // Simulate a TUI-originated toggle by writing the new bytes through the
    // store directly (the same path real toggles take). Then feed the same
    // path back as if the watcher had fired.
    let alpha = app
        .state()
        .tasks
        .iter()
        .find(|t| t.title == "Alpha task")
        .unwrap()
        .full_id
        .clone();

    app.store_mut()
        .apply(lash_core::store::Mutation::SetTaskStatus {
            absolute_path: tasks_md.clone(),
            task_title: "Alpha task".into(),
            old_status: lash_types::TaskStatus::Open,
            new_status: lash_types::TaskStatus::InProgress,
        })
        .unwrap();

    // The watcher echo: file on disk equals what the Store just wrote, so
    // the Store's hash dedupe should suppress any reload.
    let tasks_before_count = app.state().tasks.len();
    app.process_external_change(&tasks_md).unwrap();
    assert_eq!(
        app.state().tasks.len(),
        tasks_before_count,
        "self-write echo should not have caused a reload"
    );

    // Alpha is still tracked by id (the in-memory DB state has not been
    // updated by the dedupe path; that's correct — only handle_external_change
    // for real edits triggers a reindex).
    assert!(
        app.state().tasks.iter().any(|t| t.full_id == alpha),
        "Alpha task should still be present in state"
    );
}
