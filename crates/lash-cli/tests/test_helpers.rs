//! Test to validate test helper functions work correctly

mod common;

use common::{assert_error_contains, fixture_path, load_fixture, temp_test_dir};

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
