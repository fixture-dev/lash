//! Common test utilities shared across integration tests
//!
//! This module is not a test itself, but provides shared functionality
//! that can be used by integration tests.

#![allow(dead_code)] // Test helpers may not be used by all test files

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

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

/// Assert that a file contains the expected content
///
/// # Arguments
///
/// * `path` - Path to the file to check
/// * `expected` - Expected content substring
///
/// # Panics
///
/// Panics if the file doesn't exist or doesn't contain the expected content
pub fn assert_file_contains<P: AsRef<Path>>(path: P, expected: &str) {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read file {}: {}", path.display(), e));
    assert!(
        content.contains(expected),
        "Expected file {} to contain '{expected}', but content was:\n{}",
        path.display(),
        content
    );
}

/// Assert that a file's content exactly matches the expected content
///
/// # Arguments
///
/// * `path` - Path to the file to check
/// * `expected` - Expected exact content
///
/// # Panics
///
/// Panics if the file doesn't exist or content doesn't match
pub fn assert_file_contents<P: AsRef<Path>>(path: P, expected: &str) {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read file {}: {}", path.display(), e));
    assert_eq!(
        content,
        expected,
        "File content mismatch for {}",
        path.display()
    );
}

/// Run a lash CLI command
///
/// Returns a `Command` instance ready to execute the lash binary
///
/// # Example
///
/// ```no_run
/// # use common::run_lash_command;
/// let output = run_lash_command()
///     .arg("lint")
///     .arg("test.md")
///     .output()
///     .expect("Failed to run lash");
/// ```
#[allow(deprecated)]
#[must_use]
pub fn run_lash_command() -> Command {
    Command::cargo_bin("lash").expect("Failed to find lash binary")
}

/// Parse JSON output from a command
///
/// # Arguments
///
/// * `json_str` - JSON string to parse
///
/// # Returns
///
/// Parsed JSON value
///
/// # Panics
///
/// Panics if the JSON is invalid
#[must_use]
pub fn parse_json_output(json_str: &str) -> serde_json::Value {
    serde_json::from_str(json_str)
        .unwrap_or_else(|e| panic!("Failed to parse JSON: {e}\nInput: {json_str}"))
}

/// Builder for creating temporary test projects
///
/// # Example
///
/// ```no_run
/// # use common::TestProject;
/// let project = TestProject::builder()
///     .with_index("test-project", "My test project")
///     .with_file("tasks.md", "# Tasks\n@id: tasks\n")
///     .build();
///
/// // Use project.path() to access the project directory
/// ```
pub struct TestProject {
    temp_dir: TempDir,
}

impl TestProject {
    /// Create a new empty test project builder
    ///
    /// # Panics
    ///
    /// Panics if the temporary directory cannot be created
    #[must_use]
    pub fn builder() -> TestProjectBuilder {
        TestProjectBuilder::new()
    }

    /// Get the path to the test project directory
    #[must_use]
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Get the path to a file within the project
    #[must_use]
    pub fn file_path(&self, relative_path: &str) -> PathBuf {
        self.temp_dir.path().join(relative_path)
    }

    /// Load a fixture project (small, medium, or large)
    ///
    /// # Arguments
    ///
    /// * `size` - Project size: "small", "medium", or "large"
    ///
    /// # Panics
    ///
    /// Panics if the fixture project cannot be copied
    #[must_use]
    pub fn from_fixture(size: &str) -> Self {
        let temp_dir = temp_test_dir();
        let fixture_dir = fixture_path(&format!("repos/{size}-project"));

        copy_dir_recursive(&fixture_dir, temp_dir.path())
            .unwrap_or_else(|e| panic!("Failed to copy fixture project: {e}"));

        Self { temp_dir }
    }
}

impl Default for TestProject {
    fn default() -> Self {
        TestProjectBuilder::new().build()
    }
}

/// Builder for `TestProject`
pub struct TestProjectBuilder {
    temp_dir: TempDir,
    files: Vec<(PathBuf, String)>,
}

impl TestProjectBuilder {
    /// Create a new test project builder
    #[must_use]
    fn new() -> Self {
        Self {
            temp_dir: temp_test_dir(),
            files: Vec::new(),
        }
    }

    /// Add an index file to the project
    ///
    /// # Arguments
    ///
    /// * `id` - Project ID
    /// * `title` - Project title
    #[must_use]
    pub fn with_index(mut self, id: &str, title: &str) -> Self {
        let content = format!(
            "# {title}\n\n@id: {id}\n@status: in-progress\n@created: 2024-01-15\n\n## Tasks\n\n- [ ] Example task\n"
        );
        self.files.push((PathBuf::from("lash.index.md"), content));
        self
    }

    /// Add a file to the project
    ///
    /// # Arguments
    ///
    /// * `path` - Relative path within the project
    /// * `content` - File content
    #[must_use]
    pub fn with_file(mut self, path: &str, content: &str) -> Self {
        self.files.push((PathBuf::from(path), content.to_string()));
        self
    }

    /// Add a task file with basic structure
    ///
    /// # Arguments
    ///
    /// * `path` - Relative path within the project
    /// * `id` - Task file ID
    /// * `title` - Task file title
    #[must_use]
    pub fn with_task_file(mut self, path: &str, id: &str, title: &str) -> Self {
        let content = format!(
            "# {title}\n\n@id: {id}\n@status: in-progress\n@created: 2024-01-15\n\n## Tasks\n\n- [ ] Example task\n"
        );
        self.files.push((PathBuf::from(path), content));
        self
    }

    /// Build the test project
    ///
    /// # Panics
    ///
    /// Panics if files cannot be written
    #[must_use]
    pub fn build(self) -> TestProject {
        for (path, content) in &self.files {
            let full_path = self.temp_dir.path().join(path);

            // Create parent directories if needed
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|e| {
                    panic!("Failed to create directory {}: {}", parent.display(), e)
                });
            }

            fs::write(&full_path, content)
                .unwrap_or_else(|e| panic!("Failed to write file {}: {}", full_path.display(), e));
        }

        TestProject {
            temp_dir: self.temp_dir,
        }
    }
}

/// Copy a directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
