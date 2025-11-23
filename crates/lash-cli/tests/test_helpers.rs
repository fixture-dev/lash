//! Test to validate test helper functions work correctly

mod common;

use common::{
    assert_error_contains, assert_file_contains, assert_file_contents, fixture_path, load_fixture,
    parse_json_output, run_lash_command, temp_test_dir, TestProject,
};

#[test]
fn test_load_fixture() {
    let content = load_fixture("valid/simple-task.md");
    assert!(content.contains("Simple Task List"));
    assert!(content.contains("@id: simple.example"));
}

#[test]
fn test_fixture_path() {
    let path = fixture_path("valid/simple-task.md");
    assert!(path.exists());
    assert!(path.ends_with("tests/fixtures/valid/simple-task.md"));
}

#[test]
fn test_temp_test_dir() {
    let temp = temp_test_dir();
    assert!(temp.path().exists());
    assert!(temp.path().is_dir());
}

#[test]
fn test_assert_error_contains() {
    let result: Result<(), String> = Err("Something went wrong".to_string());
    assert_error_contains(result, "went wrong");
}

#[test]
#[should_panic(expected = "Expected error to contain")]
fn test_assert_error_contains_fails() {
    let result: Result<(), String> = Err("Something went wrong".to_string());
    assert_error_contains(result, "not found");
}

#[test]
fn test_assert_file_contains() {
    let temp = temp_test_dir();
    let file_path = temp.path().join("test.txt");
    std::fs::write(&file_path, "Hello, world!\nThis is a test.").unwrap();

    assert_file_contains(&file_path, "Hello");
    assert_file_contains(&file_path, "test");
}

#[test]
fn test_assert_file_contents() {
    let temp = temp_test_dir();
    let file_path = temp.path().join("test.txt");
    let content = "Exact content";
    std::fs::write(&file_path, content).unwrap();

    assert_file_contents(&file_path, content);
}

#[test]
fn test_parse_json_output() {
    let json = r#"{"name": "test", "value": 42}"#;
    let parsed = parse_json_output(json);

    assert_eq!(parsed["name"], "test");
    assert_eq!(parsed["value"], 42);
}

#[test]
fn test_test_project_builder() {
    let project = TestProject::builder()
        .with_index("test-project", "Test Project")
        .with_task_file("tasks.md", "tasks", "Tasks")
        .with_file("notes.txt", "Some notes")
        .build();

    // Verify files were created
    assert!(project.path().join("lash.index.md").exists());
    assert!(project.path().join("tasks.md").exists());
    assert!(project.path().join("notes.txt").exists());

    // Verify content
    let index_content = std::fs::read_to_string(project.file_path("lash.index.md")).unwrap();
    assert!(index_content.contains("Test Project"));
    assert!(index_content.contains("@id: test-project"));
}

#[test]
fn test_test_project_from_fixture() {
    let project = TestProject::from_fixture("small");

    // Verify project was copied
    assert!(project.path().join("lash.index.md").exists());
    assert!(project.path().join("tasks.md").exists());
    assert!(project.path().join("bugs.md").exists());

    // Verify content
    let index_content = std::fs::read_to_string(project.file_path("lash.index.md")).unwrap();
    assert!(index_content.contains("Small Test Project"));
}

#[test]
fn test_run_lash_command() {
    // Just verify we can create a command - don't actually run it
    let _cmd = run_lash_command();
}
