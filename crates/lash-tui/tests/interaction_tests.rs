//! Integration tests for TUI interactions
//!
//! These tests demonstrate headless TUI testing with synthetic event injection.

use lash_tui::testing::{keys, TestAppBuilder};
use tempfile::TempDir;

/// Helper to create a test database with sample data
fn setup_test_db() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    let lash_dir = project_root.join(".lash");
    std::fs::create_dir(&lash_dir).unwrap();
    let db_path = lash_dir.join("lash.db");

    // Create and initialize database
    let conn = lash_db::init_database(&db_path).unwrap();

    // Create a sample task file
    let tasks_md = project_root.join("tasks.md");
    std::fs::write(
        &tasks_md,
        r#"# Sample Tasks

@id: sample
@status: in-progress

## Tasks

- [ ] First task
- [ ] Second task
  - [ ] Subtask 1
  - [ ] Subtask 2
- [x] Completed task
"#,
    )
    .unwrap();

    // Index the project
    use lash_db::{Indexer, IndexerConfig};
    use lash_types::LashConfig;

    let config = LashConfig::default();
    let indexer_config = IndexerConfig::new(project_root.to_path_buf());
    let mut indexer = Indexer::new(&conn, indexer_config, &config);
    indexer.index_project().unwrap();

    (temp_dir, db_path)
}

#[test]
fn test_initial_render() {
    let (_temp_dir, db_path) = setup_test_db();

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .build()
        .unwrap();

    // Execute one tick to render initial state
    let should_continue = app.tick().unwrap();
    assert!(should_continue, "App should not quit immediately");

    // Verify state is initialized
    assert!(!app.state().files.is_empty(), "Files should be loaded");
    assert_eq!(app.state().selected_file_index, 0);
}

#[test]
fn test_navigation_down() {
    let (_temp_dir, db_path) = setup_test_db();

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .with_events(vec![
            keys::down(),   // Move down in file list
            keys::ctrl_c(), // Quit
        ])
        .build()
        .unwrap();

    // Initial render
    app.tick().unwrap();
    let _initial_index = app.state().selected_file_index;

    // Process down key
    app.tick().unwrap();

    // Note: Navigation behavior depends on file count
    // This just verifies the app doesn't crash

    // Process quit
    let should_continue = app.tick().unwrap();
    assert!(!should_continue, "App should quit after Ctrl+C");
}

#[test]
fn test_quit_command() {
    let (_temp_dir, db_path) = setup_test_db();

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_events(vec![keys::char('q')])
        .build()
        .unwrap();

    // Initial render
    app.tick().unwrap();

    // Process quit command
    let should_continue = app.tick().unwrap();
    assert!(!should_continue, "App should quit after 'q' key");
}

#[test]
fn test_help_modal() {
    let (_temp_dir, db_path) = setup_test_db();

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_events(vec![
            keys::char('?'), // Open help
            keys::char('?'), // Toggle help (close it)
            keys::ctrl_c(),  // Quit
        ])
        .build()
        .unwrap();

    // Initial render (no events yet)
    // First tick renders, and if there's an event, processes it
    // The '?' event gets processed in this tick
    app.tick().unwrap();
    assert!(app.state().show_help, "Help should be open after '?' key");

    // Second '?' toggles help closed
    app.tick().unwrap();
    assert!(
        !app.state().show_help,
        "Help should be closed after second '?' key"
    );

    // Quit
    let should_continue = app.tick().unwrap();
    assert!(!should_continue, "App should quit after Ctrl+C");
}
