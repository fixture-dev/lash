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

/// Helper to create a test database with multiple files in directories
/// This simulates a project structure like the `PixelQuest` example
fn setup_test_db_with_directory_structure() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    let lash_dir = project_root.join(".lash");
    std::fs::create_dir(&lash_dir).unwrap();
    let db_path = lash_dir.join("lash.db");

    // Create and initialize database
    let conn = lash_db::init_database(&db_path).unwrap();

    // Create directory structure similar to PixelQuest
    let features_dir = project_root.join("features");
    std::fs::create_dir(&features_dir).unwrap();

    let content_dir = project_root.join("content");
    std::fs::create_dir(&content_dir).unwrap();

    // Create a root index file
    let index_md = project_root.join("lash.index.md");
    std::fs::write(
        &index_md,
        r#"# Test Project

@id: root

## Tasks

- [ ] Root task
"#,
    )
    .unwrap();

    // Create Enemy AI file (the one we'll select)
    let enemy_ai_md = features_dir.join("enemy-ai.md");
    std::fs::write(
        &enemy_ai_md,
        r#"# Enemy AI & Behavior

@id: enemy-ai

## Tasks

- [ ] Implement basic enemy types
- [ ] Create behavior tree system
- [ ] Add pathfinding
"#,
    )
    .unwrap();

    // Create another file in features
    let player_md = features_dir.join("player.md");
    std::fs::write(
        &player_md,
        r#"# Player Movement

@id: player

## Tasks

- [ ] Basic movement
- [ ] Jump mechanics
"#,
    )
    .unwrap();

    // Create a file in content directory
    let levels_md = content_dir.join("levels.md");
    std::fs::write(
        &levels_md,
        r#"# Level Design

@id: levels

## Tasks

- [ ] Design tutorial level
- [ ] Create boss arena
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

/// Test that adding a new task preserves file selection
///
/// This test verifies the fix for the bug where adding a new task would cause
/// navigation to a different file ("content" directory) instead of staying on
/// the originally selected file ("Enemy AI & Behavior").
#[test]
fn test_task_creation_preserves_file_selection() {
    let (_temp_dir, db_path) = setup_test_db_with_directory_structure();

    // Build events to:
    // 1. Navigate to and expand a directory to select a file
    // 2. Open task creation modal with 'n'
    // 3. Type a task title
    // 4. Submit with Enter
    // 5. Verify selection is preserved

    // Tree structure (initially collapsed directories):
    // 0: lash.index.md (file) <- starts here
    // 1: content (dir, collapsed)
    // 2: features (dir, collapsed)
    //
    // We need to:
    // 1. Navigate down to 'features' directory (2 down presses)
    // 2. Press Enter to expand it
    // 3. Navigate down to select enemy-ai.md (1 down press)

    // Build the events in a vec! macro to avoid repeated push calls
    let title_chars: Vec<_> = "New test task".chars().map(keys::char).collect();
    let mut events = vec![
        keys::down(),    // Move to content dir
        keys::down(),    // Move to features dir
        keys::enter(),   // Expand features dir
        keys::down(),    // Move to enemy-ai.md inside features
        keys::char('n'), // Open task creation modal
    ];
    events.extend(title_chars); // Type task title
    events.push(keys::enter()); // Submit with Enter
    events.push(keys::ctrl_c()); // Don't quit immediately - we want to check the state

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(120, 40)
        .with_events(events)
        .build()
        .unwrap();

    // Debug: print initial tree structure
    eprintln!("Initial state:");
    eprintln!("  file count: {}", app.state().files.len());
    if let Some(selected) = app.state().selected_tree_node() {
        eprintln!(
            "  selected_tree_node: is_dir={}, file={:?}",
            selected.is_directory,
            selected
                .file_record
                .as_ref()
                .map(|f| f.path.to_string_lossy().to_string()),
        );
    }

    // Initial render
    app.tick().unwrap();

    // Navigate down to content dir
    app.tick().unwrap();
    eprintln!("After down #1 (to content):");
    if let Some(selected) = app.state().selected_tree_node() {
        eprintln!("  is_dir={}", selected.is_directory);
    }

    // Navigate down to features dir
    app.tick().unwrap();
    eprintln!("After down #2 (to features):");
    if let Some(selected) = app.state().selected_tree_node() {
        eprintln!(
            "  is_dir={}, is_expanded={}",
            selected.is_directory, selected.is_expanded
        );
    }

    // Expand features dir with Enter
    app.tick().unwrap();
    eprintln!("After Enter (expand features):");
    if let Some(selected) = app.state().selected_tree_node() {
        eprintln!(
            "  is_dir={}, is_expanded={}",
            selected.is_directory, selected.is_expanded
        );
    }

    // Navigate down to first file in features (enemy-ai.md)
    app.tick().unwrap();
    eprintln!("After down #3 (to enemy-ai.md):");
    if let Some(selected) = app.state().selected_tree_node() {
        eprintln!(
            "  is_dir={}, file={:?}",
            selected.is_directory,
            selected
                .file_record
                .as_ref()
                .map(|f| f.path.to_string_lossy().to_string())
        );
    }

    // Record the selected file before task creation using tree node
    let selected_tree_node = app.state().selected_tree_node();
    let selected_file_before = selected_tree_node
        .as_ref()
        .and_then(|n| n.file_record.as_ref())
        .map(|f| f.path.clone());
    let selected_index_before = app.state().selected_file_index;

    eprintln!("Before task creation: index={selected_index_before}, path={selected_file_before:?}");

    // Verify we have a file selected (not a directory)
    let is_directory = selected_tree_node.as_ref().map(|n| n.is_directory);
    assert!(
        selected_file_before.is_some(),
        "Should have a file selected (not a directory) at index {selected_index_before}. \
         Tree node: is_directory={is_directory:?}"
    );

    // Open task creation modal
    app.tick().unwrap();
    assert!(
        app.state().is_task_creation_modal_open(),
        "Task creation modal should be open"
    );

    // Type task title
    for _ in "New test task".chars() {
        app.tick().unwrap();
    }

    // Submit
    app.tick().unwrap();

    // Modal should be closed now
    assert!(
        !app.state().is_task_creation_modal_open(),
        "Task creation modal should be closed after submit"
    );

    // Check the selected file after task creation using tree node
    let selected_tree_node_after = app.state().selected_tree_node();
    let selected_file_after = selected_tree_node_after
        .as_ref()
        .and_then(|n| n.file_record.as_ref())
        .map(|f| f.path.clone());
    let selected_index_after = app.state().selected_file_index;

    eprintln!("After task creation: index={selected_index_after}, path={selected_file_after:?}");

    // The file selection should be preserved (same path, not just same index)
    assert_eq!(
        selected_file_before, selected_file_after,
        "File selection should be preserved after task creation. \
         Before: {selected_file_before:?} (index {selected_index_before}), \
         After: {selected_file_after:?} (index {selected_index_after})"
    );

    // Quit
    app.tick().unwrap();
}
