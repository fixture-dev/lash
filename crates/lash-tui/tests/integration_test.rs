//! Integration tests for TUI

use lash_db::{init_database, Indexer, IndexerConfig};
use lash_tui::{TuiApp, TuiResult};
use lash_types::LashConfig;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to set up a test database with sample data
fn setup_test_db() -> TuiResult<(TempDir, PathBuf)> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path().to_path_buf();
    let db_path = project_root.join(".lash").join("db.sqlite");

    // Create .lash directory
    std::fs::create_dir_all(db_path.parent().unwrap())?;

    // Initialize database
    let conn = init_database(&db_path)?;

    // Create a minimal test file in temp directory WITH description
    let test_file = project_root.join("test.md");
    std::fs::write(
        &test_file,
        r#"# Test File

@id: test-file

## Description

This is a test file with a description section to verify TUI display.

@agent-note: This note should be highlighted in the TUI.

## Tasks

- [ ] Task 1
  - [ ] Subtask 1.1
  - [x] Subtask 1.2
- [x] Task 2
- [-] Task 3
"#,
    )?;

    // Create index file
    let index_file = project_root.join("lash.index.md");
    std::fs::write(
        &index_file,
        r#"# Project Index

@id: index

## Tasks

- [ ] Root task
"#,
    )?;

    // Index the project
    let indexer_config = IndexerConfig::new(project_root)
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
    indexer.index_project()?;

    Ok((temp_dir, db_path))
}

#[test]
#[ignore] // Ignore by default because it requires a terminal
fn test_tui_app_creation() -> TuiResult<()> {
    let (_temp_dir, db_path) = setup_test_db()?;

    // Create TUI app (don't run it, just verify it initializes)
    // Note: We can't actually run the TUI in tests because it requires a terminal
    let _app = TuiApp::new(&db_path)?;

    Ok(())
}

#[test]
fn test_database_loads_files() -> TuiResult<()> {
    let (_temp_dir, db_path) = setup_test_db()?;
    let conn = lash_db::open_database(&db_path)?;

    // Verify files were indexed
    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;

    assert!(!files.is_empty(), "Database should contain indexed files");

    // Files are sorted by path, so we should have:
    // 1. lash.index.md (Project Index)
    // 2. test.md (Test File)
    assert!(files.len() >= 2, "Should have at least 2 files");

    // Find the test file
    let test_file = files.iter().find(|f| f.title == "Test File");
    assert!(test_file.is_some(), "Should find Test File");

    Ok(())
}

#[test]
fn test_database_loads_description() -> TuiResult<()> {
    let (_temp_dir, db_path) = setup_test_db()?;
    let conn = lash_db::open_database(&db_path)?;

    // Verify files were indexed with descriptions
    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;

    // Find the test file
    let test_file = files
        .iter()
        .find(|f| f.title == "Test File")
        .expect("Should find Test File");

    // Verify description was loaded
    assert!(
        !test_file.description.is_empty(),
        "Test file should have a description"
    );
    assert!(
        test_file
            .description
            .contains("test file with a description"),
        "Description should contain expected text"
    );
    assert!(
        test_file.description.contains("@agent-note"),
        "Description should contain agent-note annotation"
    );

    Ok(())
}

#[test]
fn test_state_file_tree_contains_description() -> TuiResult<()> {
    use lash_tui::state::AppState;

    let (_temp_dir, db_path) = setup_test_db()?;
    let conn = lash_db::open_database(&db_path)?;

    // Load files like TUI does
    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;

    // Verify files have descriptions before building tree
    let test_file_direct = files.iter().find(|f| f.title == "Test File").unwrap();
    eprintln!(
        "Direct file description len: {} content: '{}'",
        test_file_direct.description.len(),
        test_file_direct.description
    );
    assert!(
        !test_file_direct.description.is_empty(),
        "Files list should have description"
    );

    // Create state and build file tree
    let mut state = AppState::new();
    state.files = files;
    state.build_file_tree();

    // List all tree nodes to find the test file
    eprintln!("Looking for Test File in tree...");
    for idx in 0..10 {
        state.selected_file_index = idx;
        if let Some(node) = state.selected_tree_node() {
            let title = node
                .file_record
                .as_ref()
                .map(|f| f.title.as_str())
                .unwrap_or("(directory)");
            let desc_len = node
                .file_record
                .as_ref()
                .map(|f| f.description.len())
                .unwrap_or(0);
            eprintln!(
                "  idx={}: is_dir={}, title='{}', desc_len={}",
                idx, node.is_directory, title, desc_len
            );
            if title == "Test File" {
                assert!(
                    !node.is_directory,
                    "Test File node should not be a directory"
                );
                assert!(
                    node.file_record.is_some(),
                    "Test File node should have file_record"
                );
                let file = node.file_record.unwrap();
                assert!(
                    !file.description.is_empty(),
                    "File record in tree should have description, got: '{}'",
                    file.description
                );
                eprintln!("Found Test File at idx={idx} with description!");
                return Ok(());
            }
        } else {
            break;
        }
    }

    panic!("Test File not found in tree");
}

#[test]
fn test_database_loads_tasks() -> TuiResult<()> {
    let (_temp_dir, db_path) = setup_test_db()?;
    let conn = lash_db::open_database(&db_path)?;

    // Find the test file
    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;
    let test_file = files
        .iter()
        .find(|f| f.title == "Test File")
        .expect("Test File should exist");

    // Get tasks for that file
    let task_repo = lash_db::repository::TaskRepository::new(&conn);
    let tasks = task_repo.get_by_file(test_file.id)?;

    assert_eq!(tasks.len(), 5, "Should have 5 tasks (including subtasks)");

    // Verify task hierarchy
    let task1 = &tasks[0];
    assert_eq!(task1.title, "Task 1");
    assert_eq!(task1.status, lash_types::TaskStatus::Open);

    // Verify subtasks exist
    let subtask1 = &tasks[1];
    assert_eq!(subtask1.title, "Subtask 1.1");
    assert_eq!(subtask1.depth, 1);

    Ok(())
}

/// Helper to set up a test database with subdirectory structure (like playground)
fn setup_test_db_with_dirs() -> TuiResult<(TempDir, PathBuf)> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path().to_path_buf();
    let db_path = project_root.join(".lash").join("db.sqlite");

    // Create .lash directory and subdirectories
    std::fs::create_dir_all(db_path.parent().unwrap())?;
    std::fs::create_dir_all(project_root.join("systems"))?;
    std::fs::create_dir_all(project_root.join("features"))?;

    // Initialize database
    let conn = init_database(&db_path)?;

    // Create index file (no description)
    let index_file = project_root.join("lash.index.md");
    std::fs::write(
        &index_file,
        r#"# Project Index

@id: index

## Tasks

- [ ] Root task
"#,
    )?;

    // Create audio.md in systems/ subdirectory WITH description
    let audio_file = project_root.join("systems").join("audio.md");
    std::fs::write(
        &audio_file,
        r#"# Audio Engine

@id: systems.audio

## Description

Sound engine for music playback, sound effects, and spatial audio.

@agent-note: Focus on crossfade transitions first.

## Tasks

- [x] Set up audio engine
- [ ] Implement music system
"#,
    )?;

    // Create movement.md in features/ subdirectory WITH description
    let movement_file = project_root.join("features").join("movement.md");
    std::fs::write(
        &movement_file,
        r#"# Player Movement

@id: features.movement

## Description

Core player movement mechanics including physics and controls.

@agent-note: Prioritize responsive input handling.

## Tasks

- [x] Basic movement
- [ ] Advanced movement
"#,
    )?;

    // Index the project
    let indexer_config = IndexerConfig::new(project_root)
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
    indexer.index_project()?;

    Ok((temp_dir, db_path))
}

#[test]
fn test_file_tree_with_directories_preserves_description() -> TuiResult<()> {
    use lash_tui::state::AppState;

    let (_temp_dir, db_path) = setup_test_db_with_dirs()?;
    let conn = lash_db::open_database(&db_path)?;

    // Load files like TUI does
    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;

    eprintln!("Files in database:");
    for f in &files {
        eprintln!(
            "  path={}, title={}, desc_len={}",
            f.path.display(),
            f.title,
            f.description.len()
        );
    }

    // Verify audio file has description before building tree
    let audio_file = files.iter().find(|f| f.title == "Audio Engine").unwrap();
    assert!(
        !audio_file.description.is_empty(),
        "Audio file should have description in file list"
    );

    // Create state and build file tree
    let mut state = AppState::new();
    state.files = files;
    state.build_file_tree();

    // Expand all directories to make all files visible
    if let Some(trees) = &mut state.file_tree {
        for tree in trees {
            tree.expand_all(10); // Expand up to depth 10
        }
    }

    // Print the entire tree structure
    eprintln!("\nFile tree structure (after expand_all):");
    let mut found_audio = false;
    for idx in 0..20 {
        state.selected_file_index = idx;
        if let Some(node) = state.selected_tree_node() {
            let name = if node.is_directory {
                format!("[DIR] {}", node.file_record.as_ref().map_or("?", |_| ""))
            } else {
                node.file_record
                    .as_ref()
                    .map(|f| f.title.as_str())
                    .unwrap_or("(no file_record)")
                    .to_string()
            };
            let desc_len = node
                .file_record
                .as_ref()
                .map(|f| f.description.len())
                .unwrap_or(0);
            eprintln!(
                "  idx={}: is_dir={}, name='{}', desc_len={}, expanded={}",
                idx, node.is_directory, name, desc_len, node.is_expanded
            );

            // Check Audio Engine specifically
            if let Some(file) = &node.file_record {
                if file.title == "Audio Engine" {
                    found_audio = true;
                    assert!(
                        !file.description.is_empty(),
                        "Audio Engine should have description in tree, got: '{}'",
                        file.description
                    );
                    eprintln!(
                        "  -> Found Audio Engine with description len {}",
                        file.description.len()
                    );
                }
            }
        } else {
            break;
        }
    }

    assert!(found_audio, "Should have found Audio Engine in tree");

    // Now test what the detail pane render logic would do
    // Set selection to Audio Engine (idx=4 based on above output)
    state.selected_file_index = 4;
    let node = state
        .selected_tree_node()
        .expect("Should have node at idx 4");
    assert!(!node.is_directory, "Audio Engine should not be a directory");
    let file = node
        .file_record
        .expect("Audio Engine should have file_record");
    assert!(
        !file.description.is_empty(),
        "Audio Engine should have non-empty description"
    );

    // This is what should_show_description checks
    assert!(
        state.current_label_filter.is_none(),
        "No label filter should be active"
    );
    eprintln!(
        "\nDescription check for Audio Engine: desc_len={}, should_show=true",
        file.description.len()
    );

    Ok(())
}

/// Helper to set up a test database with nested directories where intermediate
/// directories don't contain files directly (like playground/worlds/forest/)
fn setup_test_db_with_intermediate_dirs() -> TuiResult<(TempDir, PathBuf)> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path().to_path_buf();
    let db_path = project_root.join(".lash").join("db.sqlite");

    // Create .lash directory and nested subdirectories
    // Note: "worlds" has no direct .md files, only subdirectories
    std::fs::create_dir_all(db_path.parent().unwrap())?;
    std::fs::create_dir_all(project_root.join("design"))?;
    std::fs::create_dir_all(project_root.join("worlds").join("forest").join("levels"))?;

    // Initialize database
    let conn = init_database(&db_path)?;

    // Create index file
    let index_file = project_root.join("lash.index.md");
    std::fs::write(
        &index_file,
        r#"# Project Index

@id: index

## Tasks

- [ ] Root task
"#,
    )?;

    // Create design/story.md (direct child, like playground)
    let story_file = project_root.join("design").join("story.md");
    std::fs::write(
        &story_file,
        r#"# Story Design

@id: design.story

## Tasks

- [ ] Write story
"#,
    )?;

    // Create worlds/forest/boss.md (nested in intermediate dir)
    let boss_file = project_root.join("worlds").join("forest").join("boss.md");
    std::fs::write(
        &boss_file,
        r#"# Forest Boss

@id: worlds.forest.boss

## Tasks

- [ ] Design boss
"#,
    )?;

    // Create worlds/forest/levels/world-1.md (deeply nested)
    let level_file = project_root
        .join("worlds")
        .join("forest")
        .join("levels")
        .join("world-1.md");
    std::fs::write(
        &level_file,
        r#"# World 1 Levels

@id: worlds.forest.levels.world1

## Tasks

- [ ] Build level 1
"#,
    )?;

    // Index the project
    let indexer_config = IndexerConfig::new(project_root)
        .with_incremental(false)
        .with_progress(false);
    let parser_config = LashConfig::default();
    let mut indexer = Indexer::new(&conn, indexer_config, &parser_config);
    indexer.index_project()?;

    Ok((temp_dir, db_path))
}

#[test]
fn test_file_tree_with_intermediate_directories() -> TuiResult<()> {
    use lash_tui::state::AppState;

    let (_temp_dir, db_path) = setup_test_db_with_intermediate_dirs()?;
    let conn = lash_db::open_database(&db_path)?;

    // Load files like TUI does
    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;

    eprintln!("Files in database:");
    for f in &files {
        eprintln!("  path={}", f.path.display());
    }

    // Create state and build file tree
    let mut state = AppState::new();
    state.files = files;
    state.build_file_tree();

    // Get the tree
    let trees = state.file_tree.as_ref().expect("Should have file tree");

    // Collect all directory names at root level
    let root_names: Vec<&str> = trees.iter().map(|t| t.data.name.as_str()).collect();

    eprintln!("Root level items: {root_names:?}");

    // The key assertion: "worlds" should be at root level, NOT "forest"
    // If intermediate directories aren't created, "forest" would appear at root
    assert!(
        root_names.contains(&"worlds"),
        "Root should contain 'worlds' directory, got: {root_names:?}",
    );
    assert!(
        !root_names.contains(&"forest"),
        "Root should NOT contain 'forest' (it should be under 'worlds'), got: {root_names:?}",
    );
    assert!(
        !root_names.contains(&"levels"),
        "Root should NOT contain 'levels' (it should be under 'worlds/forest'), got: {root_names:?}",
    );

    // Also verify "design" is at root (direct child directory)
    assert!(
        root_names.contains(&"design"),
        "Root should contain 'design' directory, got: {root_names:?}",
    );

    // Now verify the nested structure: worlds -> forest -> levels
    let worlds_node = trees.iter().find(|t| t.data.name == "worlds").unwrap();
    let worlds_children: Vec<&str> = worlds_node
        .children
        .iter()
        .map(|c| c.data.name.as_str())
        .collect();
    eprintln!("worlds children: {worlds_children:?}");
    assert!(
        worlds_children.contains(&"forest"),
        "worlds should contain 'forest', got: {worlds_children:?}",
    );

    let forest_node = worlds_node
        .children
        .iter()
        .find(|c| c.data.name == "forest")
        .unwrap();
    let forest_children: Vec<&str> = forest_node
        .children
        .iter()
        .map(|c| c.data.name.as_str())
        .collect();
    eprintln!("forest children: {forest_children:?}");
    assert!(
        forest_children.contains(&"levels"),
        "forest should contain 'levels', got: {forest_children:?}",
    );
    assert!(
        forest_children.contains(&"boss.md"),
        "forest should contain 'boss.md', got: {forest_children:?}",
    );

    Ok(())
}

#[test]
fn test_load_label_stats_for_autocomplete() -> TuiResult<()> {
    let (_temp_dir, db_path) = setup_test_db()?;
    let conn = lash_db::open_database(&db_path)?;

    // Add some labels to the database
    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;
    let test_file = files
        .iter()
        .find(|f| f.title == "Test File")
        .expect("Test File should exist");

    let task_repo = lash_db::repository::TaskRepository::new(&conn);
    let tasks = task_repo.get_by_file(test_file.id)?;

    let label_repo = lash_db::repository::LabelRepository::new(&conn);

    // Add labels to tasks
    label_repo.set_task_labels(tasks[0].id, &["backend".to_string(), "rust".to_string()])?;
    label_repo.set_task_labels(tasks[1].id, &["backend".to_string()])?;
    label_repo.set_task_labels(tasks[2].id, &["frontend".to_string()])?;

    // Get label stats
    let stats = label_repo.get_label_stats()?;

    assert!(!stats.is_empty(), "Should have label statistics");

    // Find backend label
    let backend_stat = stats.iter().find(|s| s.name == "backend");
    assert!(backend_stat.is_some(), "Should find backend label");
    let backend_stat = backend_stat.unwrap();
    assert_eq!(backend_stat.task_count, 2, "Backend should have 2 tasks");

    // Find rust label
    let rust_stat = stats.iter().find(|s| s.name == "rust");
    assert!(rust_stat.is_some(), "Should find rust label");
    let rust_stat = rust_stat.unwrap();
    assert_eq!(rust_stat.task_count, 1, "Rust should have 1 task");

    // Find frontend label
    let frontend_stat = stats.iter().find(|s| s.name == "frontend");
    assert!(frontend_stat.is_some(), "Should find frontend label");
    let frontend_stat = frontend_stat.unwrap();
    assert_eq!(frontend_stat.task_count, 1, "Frontend should have 1 task");

    Ok(())
}

#[test]
fn test_load_distinct_owners_for_autocomplete() -> TuiResult<()> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("test.db");

    // Initialize database
    let conn = lash_db::init_database(&db_path)?;

    // Manually insert test data with owners
    // (Note: The indexer doesn't currently populate task.owner from file metadata,
    // so we test the database query directly)
    use lash_db::DbError;
    conn.execute(
        "INSERT INTO files (path, file_id, title, hash, mtime, status) VALUES (?, ?, ?, ?, ?, ?)",
        ("test1.md", "test1", "Test 1", "hash1", 0, "empty"),
    )
    .map_err(DbError::from)?;
    let file1_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO files (path, file_id, title, hash, mtime, status) VALUES (?, ?, ?, ?, ?, ?)",
        ("test2.md", "test2", "Test 2", "hash2", 0, "empty"),
    )
    .map_err(DbError::from)?;
    let file2_id = conn.last_insert_rowid();

    // Insert tasks with different owners
    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, owner)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        (
            file1_id,
            "task1",
            "test1#task1",
            "Task 1",
            "open",
            0,
            0,
            "alice",
        ),
    )
    .map_err(DbError::from)?;

    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, owner)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        (
            file1_id,
            "task2",
            "test1#task2",
            "Task 2",
            "open",
            0,
            1,
            "bob",
        ),
    )
    .map_err(DbError::from)?;

    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, owner)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        (
            file2_id,
            "task3",
            "test2#task3",
            "Task 3",
            "open",
            0,
            0,
            "alice",
        ),
    )
    .map_err(DbError::from)?;

    conn.execute(
        "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, owner)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        (
            file2_id,
            "task4",
            "test2#task4",
            "Task 4",
            "open",
            0,
            1,
            "charlie",
        ),
    )
    .map_err(DbError::from)?;

    // Get distinct owners
    let task_repo = lash_db::repository::TaskRepository::new(&conn);
    let owners = task_repo.get_distinct_owners()?;

    assert_eq!(owners.len(), 3, "Should have 3 distinct owners");
    assert!(owners.contains(&"alice".to_string()));
    assert!(owners.contains(&"bob".to_string()));
    assert!(owners.contains(&"charlie".to_string()));

    // Verify they're sorted
    assert_eq!(owners[0], "alice");
    assert_eq!(owners[1], "bob");
    assert_eq!(owners[2], "charlie");

    Ok(())
}

#[test]
fn test_label_stats_with_file_labels() -> TuiResult<()> {
    let (_temp_dir, db_path) = setup_test_db()?;
    let conn = lash_db::open_database(&db_path)?;

    // Get the test file
    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;
    let test_file = files
        .iter()
        .find(|f| f.title == "Test File")
        .expect("Test File should exist");

    let task_repo = lash_db::repository::TaskRepository::new(&conn);
    let tasks = task_repo.get_by_file(test_file.id)?;

    let label_repo = lash_db::repository::LabelRepository::new(&conn);

    // Add file-level label
    let doc_label_id = label_repo.get_or_create("documentation")?;
    label_repo.add_file_label(test_file.id, doc_label_id)?;

    // Add task-level label
    let urgent_label_id = label_repo.get_or_create("urgent")?;
    if !tasks.is_empty() {
        label_repo.add_task_label(tasks[0].id, urgent_label_id)?;
    }

    // Get label stats
    let stats = label_repo.get_label_stats()?;

    // Find documentation label (file-level, should apply to all tasks in file)
    let doc_stat = stats.iter().find(|s| s.name == "documentation");
    assert!(doc_stat.is_some(), "Should find documentation label");
    let doc_stat = doc_stat.unwrap();
    assert_eq!(doc_stat.file_count, 1, "Documentation should be on 1 file");
    assert!(
        doc_stat.task_count >= 1,
        "Documentation should apply to tasks via file inheritance"
    );

    // Find urgent label (task-level)
    let urgent_stat = stats.iter().find(|s| s.name == "urgent");
    assert!(urgent_stat.is_some(), "Should find urgent label");
    let urgent_stat = urgent_stat.unwrap();
    assert_eq!(urgent_stat.file_count, 0, "Urgent should be on 0 files");
    assert_eq!(urgent_stat.task_count, 1, "Urgent should be on 1 task");

    Ok(())
}

#[test]
fn test_empty_database_returns_empty_options() -> TuiResult<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path().to_path_buf();
    let db_path = project_root.join(".lash").join("db.sqlite");

    std::fs::create_dir_all(db_path.parent().unwrap())?;

    let conn = lash_db::init_database(&db_path)?;

    // Get label stats from empty database
    let label_repo = lash_db::repository::LabelRepository::new(&conn);
    let stats = label_repo.get_label_stats()?;
    assert!(
        stats.is_empty(),
        "Empty database should have no label stats"
    );

    // Get distinct owners from empty database
    let task_repo = lash_db::repository::TaskRepository::new(&conn);
    let owners = task_repo.get_distinct_owners()?;
    assert!(owners.is_empty(), "Empty database should have no owners");

    Ok(())
}

#[test]
fn test_label_stats_usage_counts() -> TuiResult<()> {
    let (_temp_dir, db_path) = setup_test_db()?;
    let conn = lash_db::open_database(&db_path)?;

    let file_repo = lash_db::repository::FileRepository::new(&conn);
    let files = file_repo.list_all()?;
    let test_file = files
        .iter()
        .find(|f| f.title == "Test File")
        .expect("Test File should exist");

    let task_repo = lash_db::repository::TaskRepository::new(&conn);
    let tasks = task_repo.get_by_file(test_file.id)?;

    let label_repo = lash_db::repository::LabelRepository::new(&conn);

    // Add backend label to multiple tasks
    let backend_label_id = label_repo.get_or_create("backend")?;
    for task in &tasks[0..3.min(tasks.len())] {
        label_repo.add_task_label(task.id, backend_label_id)?;
    }

    // Get label stats
    let stats = label_repo.get_label_stats()?;

    let backend_stat = stats.iter().find(|s| s.name == "backend");
    assert!(backend_stat.is_some(), "Should find backend label");
    let backend_stat = backend_stat.unwrap();
    assert_eq!(backend_stat.task_count, 3, "Backend should have 3 tasks");

    Ok(())
}
