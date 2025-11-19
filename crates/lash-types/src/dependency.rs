//! Dependency types and reference parsing

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{codes, LashError, Result};

/// Kind of dependency relationship
///
/// Dependencies can be implicit (hierarchical parent-child) or explicit
/// (via `@depends-on` annotations). This enum categorizes the type of
/// dependency reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    /// Parent depends on child (implicit from nesting)
    Hierarchy,
    /// Explicit ID reference: file-id#task-id
    ExplicitId,
    /// Explicit path reference: path/to/file.md
    ExplicitPath,
    /// Directory-level dependency
    Directory,
}

/// Reference to a dependency (unresolved)
///
/// Represents a dependency reference as written in the Markdown file,
/// before resolution to actual task IDs. Stores both the raw reference
/// string and the parsed dependency kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyRef {
    /// Raw reference string
    pub target: String,
    /// Parsed dependency kind
    pub kind: DependencyKind,
}

impl DependencyRef {
    /// Create a new dependency reference
    #[must_use]
    pub fn new(target: String, kind: DependencyKind) -> Self {
        Self { target, kind }
    }

    /// Validate the dependency reference syntax
    ///
    /// # Errors
    ///
    /// Returns error if the reference has invalid syntax
    pub fn validate(&self) -> Result<()> {
        if self.target.trim().is_empty() {
            return Err(LashError::Dependency {
                code: codes::E_DEP_INVALID_REF,
                message: "Dependency reference cannot be empty".to_string(),
                location: None,
                chain: None,
                help: Some("dependencies must be in format: path/to/file.md#task:id".to_string()),
            });
        }

        // Validate based on kind
        match self.kind {
            DependencyKind::ExplicitPath => {
                if !std::path::Path::new(&self.target)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                {
                    return Err(LashError::Dependency {
                        code: codes::E_DEP_INVALID_REF,
                        message: format!(
                            "ExplicitPath dependency must end with '.md': '{}'",
                            self.target
                        ),
                        location: None,
                        chain: None,
                        help: Some(
                            "dependencies must be in format: path/to/file.md#task:id".to_string(),
                        ),
                    });
                }
            }
            DependencyKind::Directory => {
                if !self.target.ends_with('/') {
                    return Err(LashError::Dependency {
                        code: codes::E_DEP_INVALID_REF,
                        message: format!(
                            "Directory dependency must end with '/': '{}'",
                            self.target
                        ),
                        location: None,
                        chain: None,
                        help: Some(
                            "dependencies must be in format: path/to/file.md#task:id".to_string(),
                        ),
                    });
                }
            }
            DependencyKind::ExplicitId | DependencyKind::Hierarchy => {
                // ExplicitId can be either:
                // 1. Full ID with #: "file-id#task-id"
                // 2. Bare file ID: "file-id" (will be resolved to a file reference)
                // Hierarchy dependencies don't have external references
                // Both are valid at this stage
            }
        }

        Ok(())
    }
}

impl fmt::Display for DependencyRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            DependencyKind::Hierarchy => write!(f, "(implicit)"),
            DependencyKind::ExplicitId | DependencyKind::ExplicitPath => {
                write!(f, "{}", self.target)
            }
            DependencyKind::Directory => write!(f, "dir:{}", self.target),
        }
    }
}

/// Resolved dependency
///
/// Represents a fully resolved dependency between two tasks, with both
/// source and target identified by their full IDs (file-id#task-id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    /// Source task full ID (file-id#task-id)
    pub from_task_id: String,
    /// Target task full ID
    pub to_task_id: String,
    /// Dependency type
    pub kind: DependencyKind,
}

impl Dependency {
    /// Create a new resolved dependency
    #[must_use]
    pub fn new(from_task_id: String, to_task_id: String, kind: DependencyKind) -> Self {
        Self {
            from_task_id,
            to_task_id,
            kind,
        }
    }
}

/// Parse a dependency reference string
///
/// Detects the format and creates a `DependencyRef`:
/// - Path ending in `.md` → `ExplicitPath`
/// - Contains `#` → `ExplicitId`
/// - Ends with `/` → `Directory`
///
/// # Examples
///
/// ```
/// use lash_types::dependency::{parse_dependency_ref, DependencyKind};
///
/// let ref1 = parse_dependency_ref("core/api.md").unwrap();
/// assert_eq!(ref1.kind, DependencyKind::ExplicitPath);
///
/// let ref2 = parse_dependency_ref("core.api#setup").unwrap();
/// assert_eq!(ref2.kind, DependencyKind::ExplicitId);
///
/// let ref3 = parse_dependency_ref("core/").unwrap();
/// assert_eq!(ref3.kind, DependencyKind::Directory);
/// ```
///
/// # Errors
///
/// Returns error if the reference string is empty or has invalid syntax
pub fn parse_dependency_ref(s: &str) -> Result<DependencyRef> {
    let trimmed = s.trim();

    if trimmed.is_empty() {
        return Err(LashError::Dependency {
            code: codes::E_DEP_INVALID_REF,
            message: "Dependency reference cannot be empty".to_string(),
            location: None,
            chain: None,
            help: Some("dependencies must be in format: path/to/file.md#task:id".to_string()),
        });
    }

    // Detect kind based on format
    let kind = if trimmed.ends_with('/') {
        DependencyKind::Directory
    } else if std::path::Path::new(trimmed)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        DependencyKind::ExplicitPath
    } else if trimmed.contains('#') {
        DependencyKind::ExplicitId
    } else {
        // Default to ExplicitId if no clear indicator
        // This allows bare file IDs like "core.api"
        DependencyKind::ExplicitId
    };

    let dep_ref = DependencyRef::new(trimmed.to_string(), kind);
    dep_ref.validate()?;

    Ok(dep_ref)
}

/// Create a full task ID from file and task IDs
///
/// Combines file ID and task ID into the canonical format: `{file_id}#{task_id}`
///
/// # Examples
///
/// ```
/// use lash_types::dependency::make_full_id;
///
/// assert_eq!(make_full_id("core.api", "setup"), "core.api#setup");
/// assert_eq!(make_full_id("tasks", "task-1"), "tasks#task-1");
/// ```
#[must_use]
pub fn make_full_id(file_id: &str, task_id: &str) -> String {
    format!("{file_id}#{task_id}")
}

/// Parse a full task ID into file and task components
///
/// Splits on the first `#` character to extract file and task IDs.
///
/// # Examples
///
/// ```
/// use lash_types::dependency::parse_full_id;
///
/// let (file_id, task_id) = parse_full_id("core.api#setup").unwrap();
/// assert_eq!(file_id, "core.api");
/// assert_eq!(task_id, "setup");
///
/// let (file_id, task_id) = parse_full_id("tasks#task-1").unwrap();
/// assert_eq!(file_id, "tasks");
/// assert_eq!(task_id, "task-1");
/// ```
///
/// # Errors
///
/// Returns error if the full ID doesn't contain a `#` separator
pub fn parse_full_id(full_id: &str) -> Result<(String, String)> {
    if let Some((file_id, task_id)) = full_id.split_once('#') {
        if file_id.is_empty() || task_id.is_empty() {
            return Err(LashError::Dependency {
                code: codes::E_DEP_INVALID_REF,
                message: format!("Invalid full ID format: '{full_id}'"),
                location: None,
                chain: None,
                help: Some("dependencies must be in format: path/to/file.md#task:id".to_string()),
            });
        }
        Ok((file_id.to_string(), task_id.to_string()))
    } else {
        Err(LashError::Dependency {
            code: codes::E_DEP_INVALID_REF,
            message: format!("Full ID must contain '#': '{full_id}'"),
            location: None,
            chain: None,
            help: Some("dependencies must be in format: path/to/file.md#task:id".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dependency_ref_explicit_path() {
        let dep_ref = parse_dependency_ref("core/api.md").unwrap();
        assert_eq!(dep_ref.kind, DependencyKind::ExplicitPath);
        assert_eq!(dep_ref.target, "core/api.md");
    }

    #[test]
    fn test_parse_dependency_ref_explicit_id() {
        let dep_ref = parse_dependency_ref("core.api#setup").unwrap();
        assert_eq!(dep_ref.kind, DependencyKind::ExplicitId);
        assert_eq!(dep_ref.target, "core.api#setup");
    }

    #[test]
    fn test_parse_dependency_ref_directory() {
        let dep_ref = parse_dependency_ref("core/").unwrap();
        assert_eq!(dep_ref.kind, DependencyKind::Directory);
        assert_eq!(dep_ref.target, "core/");
    }

    #[test]
    fn test_parse_dependency_ref_bare_id() {
        let dep_ref = parse_dependency_ref("core.api").unwrap();
        assert_eq!(dep_ref.kind, DependencyKind::ExplicitId);
        assert_eq!(dep_ref.target, "core.api");
    }

    #[test]
    fn test_parse_dependency_ref_relative_path() {
        let dep_ref = parse_dependency_ref("../sibling.md").unwrap();
        assert_eq!(dep_ref.kind, DependencyKind::ExplicitPath);
        assert_eq!(dep_ref.target, "../sibling.md");
    }

    #[test]
    fn test_parse_dependency_ref_empty() {
        assert!(parse_dependency_ref("").is_err());
        assert!(parse_dependency_ref("   ").is_err());
    }

    #[test]
    fn test_dependency_ref_validate() {
        // Valid references
        assert!(parse_dependency_ref("core/api.md")
            .unwrap()
            .validate()
            .is_ok());
        assert!(parse_dependency_ref("core.api#setup")
            .unwrap()
            .validate()
            .is_ok());
        assert!(parse_dependency_ref("core/").unwrap().validate().is_ok());

        // Valid: ExplicitId without # (bare file ID)
        let bare_id = DependencyRef::new("bare-file-id".to_string(), DependencyKind::ExplicitId);
        assert!(bare_id.validate().is_ok());

        // Invalid: ExplicitPath without .md
        let invalid = DependencyRef::new("no-extension".to_string(), DependencyKind::ExplicitPath);
        assert!(invalid.validate().is_err());

        // Invalid: Directory without /
        let invalid = DependencyRef::new("no-slash".to_string(), DependencyKind::Directory);
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_dependency_ref_display() {
        let hierarchy = DependencyRef::new(String::new(), DependencyKind::Hierarchy);
        assert_eq!(format!("{hierarchy}"), "(implicit)");

        let explicit_id =
            DependencyRef::new("core.api#setup".to_string(), DependencyKind::ExplicitId);
        assert_eq!(format!("{explicit_id}"), "core.api#setup");

        let path = DependencyRef::new("core/api.md".to_string(), DependencyKind::ExplicitPath);
        assert_eq!(format!("{path}"), "core/api.md");

        let dir = DependencyRef::new("core/".to_string(), DependencyKind::Directory);
        assert_eq!(format!("{dir}"), "dir:core/");
    }

    #[test]
    fn test_make_full_id() {
        assert_eq!(make_full_id("core.api", "setup"), "core.api#setup");
        assert_eq!(make_full_id("tasks", "task-1"), "tasks#task-1");
        assert_eq!(make_full_id("file", ""), "file#");
    }

    #[test]
    fn test_parse_full_id() {
        let (file_id, task_id) = parse_full_id("core.api#setup").unwrap();
        assert_eq!(file_id, "core.api");
        assert_eq!(task_id, "setup");

        let (file_id, task_id) = parse_full_id("tasks#task-1").unwrap();
        assert_eq!(file_id, "tasks");
        assert_eq!(task_id, "task-1");
    }

    #[test]
    fn test_parse_full_id_errors() {
        // No hash
        assert!(parse_full_id("no-hash").is_err());

        // Empty components
        assert!(parse_full_id("#task").is_err());
        assert!(parse_full_id("file#").is_err());
    }

    #[test]
    fn test_full_id_round_trip() {
        let file_id = "core.api";
        let task_id = "setup";

        let full_id = make_full_id(file_id, task_id);
        let (parsed_file, parsed_task) = parse_full_id(&full_id).unwrap();

        assert_eq!(parsed_file, file_id);
        assert_eq!(parsed_task, task_id);
    }

    #[test]
    fn test_dependency_creation() {
        let dep = Dependency::new(
            "core.api#setup".to_string(),
            "core.db#init".to_string(),
            DependencyKind::ExplicitId,
        );

        assert_eq!(dep.from_task_id, "core.api#setup");
        assert_eq!(dep.to_task_id, "core.db#init");
        assert_eq!(dep.kind, DependencyKind::ExplicitId);
    }

    #[test]
    fn test_parse_dependency_ref_round_trip() {
        let references = vec!["core/api.md", "core.api#setup", "core/", "../relative.md"];

        for reference in references {
            let dep_ref = parse_dependency_ref(reference).unwrap();
            assert!(dep_ref.validate().is_ok());
            // Display might not match exactly (e.g., dir: prefix), but should be valid
        }
    }
}
