//! Common test utilities shared across integration tests
//!
//! This module is not a test itself, but provides shared functionality
//! that can be used by integration tests.

use std::fs;
use std::path::{Path, PathBuf};

/// Load a test fixture file
///
/// # Arguments
///
/// * `relative_path` - Path relative to tests/fixtures/ (e.g., "valid/simple-task.md")
///
/// # Panics
///
/// Panics if the fixture file cannot be read
#[must_use]
pub fn load_fixture(relative_path: &str) -> String {
    let fixture_path = fixture_path(relative_path);
    fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fixture_path.display(), e))
}

/// Get the absolute path to a fixture file
///
/// # Arguments
///
/// * `relative_path` - Path relative to tests/fixtures/
#[must_use]
pub fn fixture_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative_path)
}

/// Create a temporary test directory
///
/// Returns a path to a temporary directory that will be cleaned up automatically
/// when the returned `TempDir` is dropped.
///
/// # Panics
///
/// Panics if the temporary directory cannot be created
#[must_use]
pub fn temp_test_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temporary directory")
}

/// Assert that a Result contains an error with the given substring
///
/// # Panics
///
/// Panics if the result is Ok or if the error message doesn't contain the expected text
pub fn assert_error_contains<T, E: std::fmt::Display>(result: Result<T, E>, expected: &str) {
    match result {
        Ok(_) => panic!("Expected error containing '{expected}', but got Ok"),
        Err(e) => {
            let error_msg = format!("{e}");
            assert!(
                error_msg.contains(expected),
                "Expected error to contain '{expected}', but got: {error_msg}"
            );
        }
    }
}
