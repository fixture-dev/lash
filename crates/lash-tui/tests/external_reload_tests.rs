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
