//! Tests for navigation bounds checking in TUI
//!
//! These tests verify that navigating to the end of a list and pressing down
//! multiple times doesn't cause the selection to go beyond valid indices.

use lash_tui::testing::{keys, TestAppBuilder};
use tempfile::TempDir;

/// Helper to create a test database with multiple files for files pane testing
fn setup_test_db_with_multiple_files() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    let lash_dir = project_root.join(".lash");
    std::fs::create_dir(&lash_dir).unwrap();
    let db_path = lash_dir.join("lash.db");

    // Create and initialize database
    let conn = lash_db::init_database(&db_path).unwrap();

    // Create multiple task files
    for i in 1..=5 {
        let file_path = project_root.join(format!("tasks{i}.md"));
        std::fs::write(
            &file_path,
            format!(
                r#"# Task File {i}

@id: file{i}

## Tasks

- [ ] Task in file {i}
"#
            ),
        )
        .unwrap();
    }

    // Index the project
    use lash_db::{Indexer, IndexerConfig};
    use lash_types::LashConfig;

    let config = LashConfig::default();
    let indexer_config = IndexerConfig::new(project_root.to_path_buf());
    let mut indexer = Indexer::new(&conn, indexer_config, &config);
    indexer.index_project().unwrap();

    (temp_dir, db_path)
}

/// Helper to create a test database with hierarchical tasks (with tree structure)
fn setup_test_db_with_hierarchical_tasks() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    let lash_dir = project_root.join(".lash");
    std::fs::create_dir(&lash_dir).unwrap();
    let db_path = lash_dir.join("lash.db");

    // Create and initialize database
    let conn = lash_db::init_database(&db_path).unwrap();

    // Create a task file with hierarchical structure
    let tasks_md = project_root.join("tasks.md");
    std::fs::write(
        &tasks_md,
        r#"# Hierarchical Tasks

@id: hierarchical

## Tasks

- [ ] Parent Task 1
  - [ ] Child 1.1
  - [ ] Child 1.2
    - [ ] Grandchild 1.2.1
    - [ ] Grandchild 1.2.2
- [ ] Parent Task 2
  - [ ] Child 2.1
  - [ ] Child 2.2
- [ ] Parent Task 3
- [ ] Parent Task 4
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
fn test_files_pane_navigation_down_at_end() {
    let (_temp_dir, db_path) = setup_test_db_with_multiple_files();

    // Navigate to bottom, press down 10 times, then verify up works immediately
    let mut events = vec![
        keys::char('g'), // Go to bottom
        keys::char('g'), // (gg is top, G or shift-g is bottom, but we use 'G' key combination)
    ];

    // Actually, let's just use down arrow to get to the bottom
    // First, let's navigate down to the last item (there are 5 files)
    events.clear();
    for _ in 0..10 {
        events.push(keys::down()); // Navigate down - should stop at last file
    }
    events.push(keys::ctrl_c()); // Quit

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .with_events(events)
        .build()
        .unwrap();

    // Initial render
    app.tick().unwrap();
    let _initial_index = app.state().selected_file_index;

    // Process all down key events
    for _ in 0..10 {
        app.tick().unwrap();
    }

    let final_index = app.state().selected_file_index;

    // The index should be at the last valid position (4 for 5 files, 0-indexed)
    // It should NOT have incremented beyond that
    assert_eq!(
        final_index, 4,
        "Selected file index should be at last file (index 4), got {final_index}"
    );

    // Verify it's less than the total number of files (bounds check)
    assert!(
        final_index < app.state().files.len(),
        "Selected index {} should be less than file count {}",
        final_index,
        app.state().files.len()
    );

    // Quit
    app.tick().unwrap();
}

#[test]
fn test_files_pane_navigation_up_after_excessive_down() {
    let (_temp_dir, db_path) = setup_test_db_with_multiple_files();

    // Navigate down excessively, then press up once and verify selection moves
    let mut events = vec![];
    for _ in 0..10 {
        events.push(keys::down()); // Navigate down past the end
    }
    events.push(keys::up()); // Should move up immediately
    events.push(keys::ctrl_c()); // Quit

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .with_events(events)
        .build()
        .unwrap();

    // Initial render - this ALSO processes the first down event!
    app.tick().unwrap();
    let index_after_first_down = app.state().selected_file_index;
    eprintln!("After initial tick (includes first down): index = {index_after_first_down}");

    // Process the remaining 9 down key events
    for i in 0..9 {
        app.tick().unwrap();
        let idx = app.state().selected_file_index;
        let down_num = i + 2;
        eprintln!("After down #{down_num}: index = {idx}");
    }

    let index_after_all_downs = app.state().selected_file_index;
    let file_count = app.state().files.len();
    let visible_count = app.state().visible_tree_node_count();
    let has_tree = app.state().file_tree.is_some();

    eprintln!("\nDEBUG: After pressing all 10 down keys:");
    eprintln!("  selected_file_index: {index_after_all_downs}");
    eprintln!("  total files: {file_count}");
    eprintln!("  visible_tree_node_count: {visible_count}");
    eprintln!("  has_file_tree: {has_tree}");
    eprintln!("  focused_pane: {:?}", app.state().focused_pane);
    eprintln!("  nav_mode: {:?}", app.state().nav_mode);

    // Process the up key event
    app.tick().unwrap();

    let index_after_up = app.state().selected_file_index;

    eprintln!("\nDEBUG: After pressing up once:");
    eprintln!("  selected_file_index: {index_after_up}");

    // The up key should have moved the selection immediately
    assert_eq!(
        index_after_up,
        index_after_all_downs.saturating_sub(1),
        "Up arrow should move selection immediately from {} to {}. File count: {}, Visible count: {}",
        index_after_all_downs,
        index_after_all_downs.saturating_sub(1),
        file_count,
        visible_count
    );

    // Quit
    app.tick().unwrap();
}

#[test]
fn test_task_detail_pane_navigation_down_at_end() {
    let (_temp_dir, db_path) = setup_test_db_with_hierarchical_tasks();

    // Switch to detail pane, navigate to bottom, press down 10 times
    let mut events = vec![
        keys::tab(), // Switch to description pane
        keys::tab(), // Switch to detail pane
    ];

    // Navigate down excessively
    for _ in 0..15 {
        events.push(keys::down());
    }
    events.push(keys::ctrl_c()); // Quit

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .with_events(events)
        .build()
        .unwrap();

    // Initial render
    app.tick().unwrap();

    // Switch to description pane
    app.tick().unwrap();

    // Switch to detail pane
    app.tick().unwrap();

    let _initial_task_index = app.state().selected_task_index;

    // Process all down key events
    for _ in 0..15 {
        app.tick().unwrap();
    }

    let final_task_index = app.state().selected_task_index;

    // The index should not exceed the total tasks
    // This is a basic sanity check
    assert!(
        final_task_index < app.state().tasks.len(),
        "Selected task index {} should be less than total task count {}",
        final_task_index,
        app.state().tasks.len()
    );

    // Verify we can still get the selected task (bounds are valid)
    assert!(
        app.state().selected_task().is_some(),
        "Should be able to get selected task at index {final_task_index}"
    );

    // Quit
    app.tick().unwrap();
}

#[test]
fn test_task_detail_pane_navigation_up_after_excessive_down() {
    let (_temp_dir, db_path) = setup_test_db_with_hierarchical_tasks();

    // Switch to detail pane, navigate down excessively, then press up once
    let mut events = vec![
        keys::tab(), // Switch to description pane
        keys::tab(), // Switch to detail pane
    ];

    // Navigate down excessively
    for _ in 0..15 {
        events.push(keys::down());
    }
    events.push(keys::up()); // Should move up immediately
    events.push(keys::ctrl_c()); // Quit

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .with_events(events)
        .build()
        .unwrap();

    // Process first tab (to description pane)
    app.tick().unwrap();

    // Process second tab (to detail pane)
    app.tick().unwrap();

    // Process all down key events
    for _ in 0..15 {
        app.tick().unwrap();
    }

    let index_after_down = app.state().selected_task_index;

    // Process one up key event
    app.tick().unwrap();

    let index_after_up = app.state().selected_task_index;

    // The up key should have moved the selection immediately
    // This is the key test - if the index went beyond visible items,
    // pressing up might not do anything (the bug behavior)
    assert_eq!(
        index_after_up,
        index_after_down.saturating_sub(1),
        "Up arrow should move selection immediately from {} to {}, indicating no invisible items were selected",
        index_after_down,
        index_after_down.saturating_sub(1)
    );

    // Quit
    app.tick().unwrap();
}

#[test]
fn test_task_detail_pane_with_collapsed_nodes() {
    let (_temp_dir, db_path) = setup_test_db_with_hierarchical_tasks();

    // This test specifically checks navigation when there are collapsed nodes
    // The bug manifests when using self.tasks.len() instead of counting visible nodes

    let mut events = vec![
        keys::tab(), // Switch to description pane
        keys::tab(), // Switch to detail pane
    ];

    // Navigate down excessively (more than visible items)
    for _ in 0..20 {
        events.push(keys::down());
    }

    // Try to go up - this should work immediately if bounds checking is correct
    events.push(keys::up());

    events.push(keys::ctrl_c()); // Quit

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .with_events(events)
        .build()
        .unwrap();

    // Process first tab (to description pane) - initial render also processes this event
    app.tick().unwrap();

    // Process second tab (to detail pane)
    app.tick().unwrap();

    let total_tasks = app.state().tasks.len();
    let has_task_tree = app.state().task_tree.is_some();
    let visible_task_count = app.state().visible_task_count();
    let selected_task = app.state().selected_task();

    eprintln!("\nDEBUG task detail pane state:");
    eprintln!("  total tasks in flat list: {total_tasks}");
    eprintln!("  visible task count: {visible_task_count}");
    eprintln!("  has_task_tree: {has_task_tree}");
    eprintln!("  focused_pane: {:?}", app.state().focused_pane);
    let initial_idx = app.state().selected_task_index;
    let task_title = selected_task.map(|t| &t.title);
    eprintln!("  initial selected_task_index: {initial_idx}");
    eprintln!("  selected task title: {task_title:?}");
    if total_tasks > 0 {
        eprintln!("  first few tasks:");
        for (i, task) in app.state().tasks.iter().take(5).enumerate() {
            eprintln!("    [{}] depth={} {}", i, task.depth, task.title);
        }
    }

    // Process all 20 down key events
    for i in 0..20 {
        app.tick().unwrap();
        if i % 5 == 4 {
            let idx = app.state().selected_task_index;
            let down_num = i + 1;
            eprintln!("  After down #{down_num}: index = {idx}");
        }
    }

    let index_after_downs = app.state().selected_task_index;
    eprintln!("  After all 20 downs: index = {index_after_downs}");

    // Process the up key event
    app.tick().unwrap();

    let index_after_up = app.state().selected_task_index;
    eprintln!("  After one up: index = {index_after_up}");

    // The up key should have moved the selection immediately
    assert_eq!(
        index_after_up,
        index_after_downs.saturating_sub(1),
        "Up arrow should move selection immediately from {} to {}. Visible tasks: {}",
        index_after_downs,
        index_after_downs.saturating_sub(1),
        visible_task_count
    );

    // Quit
    app.tick().unwrap();
}

#[test]
fn test_task_detail_pane_bounds_checking_edge_case() {
    let (_temp_dir, db_path) = setup_test_db_with_hierarchical_tasks();

    // Edge case: Press down exactly once beyond the last item
    // Then press up and verify immediate response

    let mut events = vec![
        keys::tab(), // Switch to description pane
        keys::tab(), // Switch to detail pane
    ];

    // First, go to the bottom using a large number of downs
    for _ in 0..10 {
        events.push(keys::down());
    }

    // Now try to go one more down (should do nothing if bounds checking works)
    events.push(keys::down());

    // Now try to go up (should work immediately)
    events.push(keys::up());

    events.push(keys::ctrl_c()); // Quit

    let mut app = TestAppBuilder::new()
        .with_db(&db_path)
        .with_size(80, 24)
        .with_events(events)
        .build()
        .unwrap();

    // Process first tab (to description pane)
    app.tick().unwrap();

    // Process second tab (to detail pane)
    app.tick().unwrap();

    // Navigate to bottom
    for _ in 0..10 {
        app.tick().unwrap();
    }

    let index_at_bottom = app.state().selected_task_index;

    // Try to go one more down
    app.tick().unwrap();

    let index_after_extra_down = app.state().selected_task_index;

    // Index should not have changed
    assert_eq!(
        index_at_bottom, index_after_extra_down,
        "Index should not increase beyond the last visible item"
    );

    // Now go up
    app.tick().unwrap();

    let index_after_up = app.state().selected_task_index;

    // Index should have decreased by 1
    assert_eq!(
        index_after_up,
        index_after_extra_down.saturating_sub(1),
        "Up should work immediately without requiring multiple presses"
    );

    // Quit
    app.tick().unwrap();
}
