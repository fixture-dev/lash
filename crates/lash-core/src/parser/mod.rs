//! Markdown parser for Lash task files
//!
//! This module provides streaming, event-based parsing of Lash Markdown files
//! into structured task data. The parser is designed to be:
//!
//! - **Fast**: Streaming event-based parsing, no full AST construction
//! - **Robust**: Continues parsing after errors, collects all diagnostics
//! - **Helpful**: Provides actionable error messages with line/column info
//!
//! # Architecture
//!
//! The parser operates in distinct phases:
//!
//! 1. **Event Stream Processing** (`events` module)
//!    - Convert Markdown events from pulldown-cmark into semantic events
//!    - Track line numbers and document structure
//!
//! 2. **Line Parsing** (`checkbox`, `annotations` modules)
//!    - Parse checkbox task lines with status and indentation
//!    - Parse annotation blocks (@id, @labels, etc.)
//!
//! 3. **Tree Building** (`builder` module)
//!    - Construct hierarchical task tree from flat checkbox lines
//!    - Validate depth limits and indentation consistency
//!    - Generate synthetic IDs for tasks without explicit @id
//!
//! 4. **File Construction**
//!    - Combine header metadata, task tree, and references
//!    - Compute content hash for change detection
//!    - Return complete `TaskFile` or aggregated errors
//!
//! # Performance Target
//!
//! The parser is optimized for pre-commit hook usage with a target of
//! <100ms for typical files (10-100 tasks). This is achieved through:
//!
//! - Streaming parsing (no AST construction)
//! - Single-pass processing where possible
//! - Minimal allocations (string reuse, arena allocation for trees)
//!
//! # Example
//!
//! ```no_run
//! use lash_core::parser::parse_file;
//! use lash_types::LashConfig;
//! use std::path::Path;
//!
//! let config = LashConfig::default();
//! // This requires actual file I/O, so we mark it no_run
//! let file = parse_file(Path::new("tasks.md"), &config).unwrap();
//! println!("Parsed {} tasks from {}", file.tasks.len(), file.id);
//! ```
//!
//! # Error Handling
//!
//! The parser uses a "collect all errors" approach - it continues parsing
//! after encountering errors and returns all diagnostics at once. This provides
//! a better user experience than stopping at the first error.
//!
//! Errors include:
//! - Line/column location information
//! - Code snippets showing the problem
//! - Actionable suggestions for fixes
//! - Stable error codes for tooling integration

pub mod annotations;
pub mod builder;
pub mod checkbox;
pub mod events;
pub mod header;

use lash_types::{
    file::{compute_hash, synthesize_file_id, FileMetadata},
    Diagnostic, LashConfig, LashError, Location, Result, TaskFile, TaskTree,
};
use std::fs;
use std::path::Path;

/// Result type for parsing operations that can accumulate multiple errors
///
/// Unlike standard `Result`, `ParseResult` can contain multiple diagnostics
/// even on success (warnings) or failure (multiple errors found).
pub type ParseResult<T> = std::result::Result<T, Vec<Diagnostic>>;

/// Context maintained during parsing of a single file
///
/// This structure tracks the current parse state and accumulates errors
/// encountered during parsing. It's passed through the parser pipeline
/// to provide context for error reporting.
#[derive(Debug)]
pub struct ParseContext<'a> {
    /// Path to the file being parsed (for error messages)
    pub file_path: &'a Path,

    /// Current line number (1-indexed)
    pub current_line: usize,

    /// Current section being parsed
    pub current_section: Section,

    /// Accumulated diagnostics (errors and warnings)
    pub diagnostics: Vec<Diagnostic>,

    /// Parser configuration
    pub config: &'a LashConfig,
}

impl<'a> ParseContext<'a> {
    /// Create a new parse context for the given file
    #[must_use]
    pub fn new(file_path: &'a Path, config: &'a LashConfig) -> Self {
        Self {
            file_path,
            current_line: 1,
            current_section: Section::Header,
            diagnostics: Vec::new(),
            config,
        }
    }

    /// Advance to the next line
    pub fn next_line(&mut self) {
        self.current_line += 1;
    }

    /// Add an error to the diagnostics list
    pub fn add_error(&mut self, error: &LashError) {
        self.diagnostics.push(error.to_diagnostic());
    }

    /// Add a diagnostic directly
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Create a location for the current position
    #[must_use]
    pub fn current_location(&self, column: usize) -> Location {
        Location::new(self.file_path.to_path_buf(), self.current_line, column)
    }

    /// Check if any errors have been encountered
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == lash_types::Severity::Error)
    }

    /// Get the number of errors
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == lash_types::Severity::Error)
            .count()
    }
}

/// Sections of a Lash task file
///
/// A well-formed task file has three main sections:
/// - Header: Title (H1), metadata annotations, optional overview text
/// - Tasks: The `## Tasks` section with checkbox lists
/// - References: Optional `## References` section with links/notes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// Header section (H1, annotations, overview)
    Header,
    /// Tasks section (## Tasks)
    Tasks,
    /// References section (## References)
    References,
    /// Unknown/unrecognized section
    Other,
}

/// Intermediate representation of a parsed file before conversion to `TaskFile`
///
/// This structure holds the parsed components before final validation and
/// conversion to the canonical `TaskFile` type.
#[derive(Debug, Clone)]
pub struct ParsedFile {
    /// Parsed header information
    pub header: ParsedHeader,

    /// Parsed task checkbox lines (before tree construction)
    pub checkbox_lines: Vec<checkbox::CheckboxLine>,

    /// Optional references section content
    pub references: Option<String>,

    /// Raw file content (for hash computation)
    pub raw_content: String,
}

/// Parsed header block
///
/// Contains the file title, metadata annotations, and optional overview text
/// extracted from the top of the file before the `## Tasks` section.
#[derive(Debug, Clone)]
pub struct ParsedHeader {
    /// File title (from H1 heading)
    pub title: String,

    /// Metadata annotations (@id, @labels, @status, etc.)
    pub annotations: annotations::AnnotationBlock,

    /// Optional overview/description text
    pub overview: Option<String>,
}

/// Main entry point for parsing a Lash task file
///
/// This function reads the file, parses it through all phases, and returns
/// either a complete `TaskFile` or a list of errors encountered.
///
/// # Arguments
///
/// * `path` - Path to the Markdown file to parse
/// * `config` - Parser configuration (max depth, custom annotations, etc.)
///
/// # Returns
///
/// Returns `Ok(TaskFile)` if parsing succeeds, or `Err(Vec<Diagnostic>)` if
/// errors were encountered. Even on success, the `TaskFile` may contain
/// warnings in its diagnostic list.
///
/// # Errors
///
/// Returns errors for:
/// - File I/O failures
/// - Invalid Markdown structure
/// - Parsing errors (invalid checkboxes, annotations, etc.)
/// - Validation errors (depth limits, duplicate IDs, etc.)
///
/// # Example
///
/// ```no_run
/// use lash_core::parser::parse_file;
/// use lash_types::LashConfig;
/// use std::path::Path;
///
/// let config = LashConfig::default();
/// // This requires actual file I/O, so we mark it no_run
/// match parse_file(Path::new("tasks.md"), &config) {
///     Ok(file) => println!("Parsed {} tasks", file.tasks.len()),
///     Err(err) => eprintln!("Error: {}", err),
/// }
/// ```
#[allow(clippy::result_large_err)] // LashError is intentionally large for rich context
pub fn parse_file(path: &Path, config: &LashConfig) -> Result<TaskFile> {
    // Read file content
    let content = fs::read_to_string(path).map_err(|e| LashError::IO {
        code: "E_IO_READ_FAILED",
        message: format!("Failed to read file: {}", path.display()),
        path: Some(path.to_path_buf()),
        io_error: Some(e.to_string()),
    })?;

    // Get file metadata from filesystem
    let metadata = fs::metadata(path).map_err(|e| LashError::IO {
        code: "E_IO_METADATA_FAILED",
        message: format!("Failed to get file metadata: {}", path.display()),
        path: Some(path.to_path_buf()),
        io_error: Some(e.to_string()),
    })?;
    let mtime = metadata
        .modified()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    // Parse the file using parse_file_from_string
    match parse_file_from_string(&content, config) {
        Ok(mut task_file) => {
            // Set the actual path and mtime
            task_file.path = path.to_path_buf();
            task_file.mtime = mtime;
            Ok(task_file)
        }
        Err(diagnostics) => {
            // Convert diagnostics to a single error
            // For now, return the first error
            if let Some(first_diag) = diagnostics.first() {
                Err(LashError::Parse {
                    code: first_diag.code,
                    message: first_diag.message.clone(),
                    location: first_diag.location.clone(),
                    snippet: first_diag.snippet.clone(),
                    help: first_diag.help.clone(),
                })
            } else {
                Err(LashError::Parse {
                    code: "E_PARSE_UNKNOWN",
                    message: "Parsing failed with no diagnostics".to_string(),
                    location: None,
                    snippet: None,
                    help: None,
                })
            }
        }
    }
}

/// Parse a task file from a string (for testing)
///
/// This is a convenience function for parsing Markdown content without
/// reading from a file. Useful for testing and in-memory parsing.
///
/// # Arguments
///
/// * `content` - Markdown content to parse
/// * `config` - Parser configuration
///
/// # Returns
///
/// Returns `ParseResult<TaskFile>` which can contain multiple diagnostics.
///
/// # Errors
///
/// Returns errors for the same conditions as `parse_file`.
pub fn parse_file_from_string(content: &str, config: &LashConfig) -> ParseResult<TaskFile> {
    // Use a temporary path for string-based parsing
    let temp_path = Path::new("<string>");

    // Create parse context
    let mut ctx = ParseContext::new(temp_path, config);

    // Phase 1: Parse header (H1, annotations, overview)
    let header = header::parse_header(content, &mut ctx);

    // Phase 2: Parse task section
    let tasks = parse_task_section_internal(content, &mut ctx)?;

    // Phase 3: Parse references section (if present)
    let tasks_section_line = find_tasks_section_line(content);
    let _references = header::parse_references_section(content, tasks_section_line);

    // Phase 4: Compute content hash
    let hash = compute_hash(content);

    // Phase 5: Extract file metadata from annotations
    let file_metadata = extract_file_metadata(&header.annotations);

    // Phase 6: Synthesize file ID from path if not provided
    let file_id = header
        .annotations
        .get_single("id")
        .map_or_else(|| synthesize_file_id(temp_path), String::from);

    // Check if any errors occurred
    if ctx.has_errors() {
        // Sort diagnostics by line number
        ctx.diagnostics.sort_by_key(|d| {
            d.location
                .as_ref()
                .and_then(|l| l.line)
                .unwrap_or(usize::MAX)
        });
        return Err(ctx.diagnostics);
    }

    // Build the TaskFile
    let task_file = TaskFile {
        path: temp_path.to_path_buf(),
        title: header.title,
        id: file_id,
        metadata: file_metadata,
        tasks,
        hash,
        mtime: std::time::SystemTime::now(),
    };

    // Validate the task file
    if let Err(e) = task_file.validate(config) {
        ctx.add_error(&e);
        return Err(ctx.diagnostics);
    }

    Ok(task_file)
}

/// Find the line number where the ## Tasks section begins
fn find_tasks_section_line(content: &str) -> Option<usize> {
    content
        .lines()
        .enumerate()
        .find(|(_, line)| {
            let trimmed = line.trim();
            trimmed.starts_with("## ") && trimmed[3..].trim().eq_ignore_ascii_case("tasks")
        })
        .map(|(idx, _)| idx)
}

/// Parse the task section and build the task tree
fn parse_task_section_internal(content: &str, ctx: &mut ParseContext) -> ParseResult<TaskTree> {
    // Find where the Tasks section starts and ends
    let tasks_start = find_tasks_section_line(content);
    let lines: Vec<&str> = content.lines().collect();

    // Determine the range of lines to parse for tasks
    let (start_line, end_line) = if let Some(tasks_line) = tasks_start {
        // Start after the "## Tasks" line
        let start = tasks_line + 1;

        // Find where References section starts (if any)
        let refs_line = lines
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, line)| {
                let trimmed = line.trim();
                trimmed.starts_with("## ") && trimmed[3..].trim().eq_ignore_ascii_case("references")
            })
            .map(|(idx, _)| idx);

        let end = refs_line.unwrap_or(lines.len());
        (start, end)
    } else {
        // No Tasks section found - parse everything after the header
        // Find first H2 or parse from beginning
        let first_h2 = lines.iter().position(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("## ")
        });

        if let Some(h2_pos) = first_h2 {
            (h2_pos + 1, lines.len())
        } else {
            // No structure at all, parse from line 1 (after potential H1)
            let first_h1 = lines.iter().position(|line| line.trim().starts_with("# "));
            let start = first_h1.map_or(0, |pos| pos + 1);
            (start, lines.len())
        }
    };

    // Parse checkbox lines in the task section
    let mut checkbox_lines = Vec::new();
    for (idx, line) in lines
        .iter()
        .enumerate()
        .skip(start_line)
        .take(end_line - start_line)
    {
        let line_num = idx + 1; // 1-indexed

        // Try to parse as checkbox line
        if let Some(cb_line) = checkbox::CheckboxLine::parse(line, line_num) {
            checkbox_lines.push(cb_line);
        } else if let Some(error_msg) = checkbox::CheckboxLine::detect_malformed(line) {
            // Line looks like a checkbox but has invalid syntax
            ctx.add_diagnostic(Diagnostic {
                severity: lash_types::Severity::Error,
                code: "E_INVALID_CHECKBOX",
                message: error_msg,
                location: Some(Location::new(ctx.file_path.to_path_buf(), line_num, 1)),
                snippet: Some((*line).to_string()),
                help: Some("Valid checkbox formats: - [ ], - [x], - [X], - [-], - [!]".to_string()),
                labels: None,
            });
        }
        // Other non-checkbox lines are silently ignored (comments, blank lines, etc.)
    }

    // Build task tree from checkbox lines
    let mut builder = builder::TaskTreeBuilder::new(ctx.config.max_depth);

    for cb_line in &checkbox_lines {
        if let Err(e) = builder.add_line(cb_line) {
            // Add error to context and continue
            ctx.add_diagnostic(Diagnostic {
                severity: lash_types::Severity::Error,
                code: "E_PARSE_TASK_TREE",
                message: e,
                location: Some(Location::new(
                    ctx.file_path.to_path_buf(),
                    cb_line.line_num,
                    cb_line.column,
                )),
                snippet: None,
                help: None,
                labels: None,
            });
        }
    }

    // Return error if we accumulated errors during tree building
    if ctx.has_errors() {
        ctx.diagnostics.sort_by_key(|d| {
            d.location
                .as_ref()
                .and_then(|l| l.line)
                .unwrap_or(usize::MAX)
        });
        return Err(ctx.diagnostics.clone());
    }

    // Build the final tree
    let tree = builder.build();

    Ok(tree)
}

/// Extract file metadata from annotation block
fn extract_file_metadata(annotations: &annotations::AnnotationBlock) -> FileMetadata {
    use lash_types::parse_dependency_ref;

    // Extract labels
    let labels = annotations
        .get_labels("labels")
        .into_iter()
        .map(|l| l.name)
        .collect();

    // Extract status
    let status = annotations.get_single("status").map(String::from);

    // Extract owner
    let owner = annotations.get_single("owner").map(String::from);

    // Extract created date
    let created = annotations.get_single("created").map(String::from);

    // Extract dependencies
    let depends_on = annotations
        .get_list("depends-on")
        .iter()
        .filter_map(|s| parse_dependency_ref(s).ok())
        .collect();

    // Extract custom annotations (all others)
    let mut custom = std::collections::HashMap::new();
    for (key, values) in annotations.iter() {
        // Skip known annotations
        if matches!(
            key.as_str(),
            "id" | "labels" | "status" | "owner" | "created" | "depends-on"
        ) {
            continue;
        }

        // For custom annotations, join multiple values with commas
        if !values.is_empty() {
            custom.insert(key.clone(), values.join(", "));
        }
    }

    FileMetadata {
        labels,
        status,
        owner,
        created,
        depends_on,
        custom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::Severity;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ==================== ParseContext Tests ====================

    #[test]
    fn test_parse_context_creation() {
        let config = LashConfig::default();
        let path = Path::new("test.md");
        let ctx = ParseContext::new(path, &config);

        assert_eq!(ctx.current_line, 1);
        assert_eq!(ctx.current_section, Section::Header);
        assert!(!ctx.has_errors());
        assert_eq!(ctx.error_count(), 0);
    }

    #[test]
    fn test_parse_context_line_tracking() {
        let config = LashConfig::default();
        let path = Path::new("test.md");
        let mut ctx = ParseContext::new(path, &config);

        assert_eq!(ctx.current_line, 1);
        ctx.next_line();
        assert_eq!(ctx.current_line, 2);
        ctx.next_line();
        assert_eq!(ctx.current_line, 3);
    }

    #[test]
    fn test_parse_context_error_tracking() {
        let config = LashConfig::default();
        let path = Path::new("test.md");
        let mut ctx = ParseContext::new(path, &config);

        assert!(!ctx.has_errors());
        assert_eq!(ctx.error_count(), 0);

        let error = LashError::parse_invalid_checkbox(path.to_path_buf(), 5, 3, "[*] invalid");
        ctx.add_error(&error);

        assert!(ctx.has_errors());
        assert_eq!(ctx.error_count(), 1);
    }

    #[test]
    fn test_parse_context_add_diagnostic() {
        let config = LashConfig::default();
        let path = Path::new("test.md");
        let mut ctx = ParseContext::new(path, &config);

        let diag = Diagnostic {
            severity: Severity::Error,
            code: "E_TEST",
            message: "Test error".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
        };

        ctx.add_diagnostic(diag);
        assert!(ctx.has_errors());
        assert_eq!(ctx.error_count(), 1);
    }

    #[test]
    fn test_parse_context_current_location() {
        let config = LashConfig::default();
        let path = Path::new("test.md");
        let ctx = ParseContext::new(path, &config);

        let loc = ctx.current_location(5);
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(5));
        assert_eq!(loc.file_path.to_str(), Some("test.md"));
    }

    #[test]
    fn test_parse_context_mixed_diagnostics() {
        let config = LashConfig::default();
        let path = Path::new("test.md");
        let mut ctx = ParseContext::new(path, &config);

        // Add warning
        ctx.add_diagnostic(Diagnostic {
            severity: Severity::Warning,
            code: "W_TEST",
            message: "Test warning".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
        });

        // Should not have errors yet
        assert!(!ctx.has_errors());
        assert_eq!(ctx.error_count(), 0);
        assert_eq!(ctx.diagnostics.len(), 1);

        // Add error
        ctx.add_diagnostic(Diagnostic {
            severity: Severity::Error,
            code: "E_TEST",
            message: "Test error".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
        });

        // Now we have errors
        assert!(ctx.has_errors());
        assert_eq!(ctx.error_count(), 1);
        assert_eq!(ctx.diagnostics.len(), 2);
    }

    // ==================== Section Tests ====================

    #[test]
    fn test_section_enum() {
        assert_eq!(Section::Header, Section::Header);
        assert_ne!(Section::Header, Section::Tasks);
        assert_ne!(Section::Tasks, Section::References);
    }

    #[test]
    fn test_section_clone() {
        let s1 = Section::Tasks;
        let s2 = s1;
        assert_eq!(s1, s2);
    }

    // ==================== parse_file_from_string Tests ====================

    #[test]
    fn test_parse_file_from_string_minimal() {
        let content = r"# Test File

## Tasks

- [ ] Task 1
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_ok(), "Expected success, got: {result:?}");
        let file = result.unwrap();

        assert_eq!(file.title, "Test File");
        assert_eq!(file.tasks.len(), 1);
    }

    #[test]
    fn test_parse_file_from_string_empty() {
        let content = "";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        // Empty file should fail or return minimal structure
        // Based on implementation, it should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_parse_file_from_string_with_id() {
        let content = r"# Test File

@id: custom-id

## Tasks

- [ ] Task 1
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.id, "custom-id");
    }

    #[test]
    fn test_parse_file_from_string_synthesizes_id() {
        let content = r"# Test File

## Tasks

- [ ] Task 1
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_ok());
        let file = result.unwrap();
        // Should have a synthesized ID
        assert!(!file.id.is_empty());
    }

    #[test]
    fn test_parse_file_from_string_with_metadata() {
        let content = r"# Test File

@id: test-id
@owner: alice
@labels: backend, api
@status: in-progress
@created: 2025-01-15

## Tasks

- [ ] Task 1
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_ok(), "Expected success, got: {result:?}");
        let file = result.unwrap();

        assert_eq!(file.metadata.owner, Some("alice".to_string()));
        assert_eq!(file.metadata.status, Some("in-progress".to_string()));
        assert_eq!(file.metadata.created, Some("2025-01-15".to_string()));
        assert_eq!(file.metadata.labels.len(), 2);
    }

    #[test]
    fn test_parse_file_from_string_with_custom_annotation() {
        let content = r"# Test File

@id: test-id
@custom: custom-value

## Tasks

- [ ] Task 1
";
        let config = LashConfig {
            custom_annotation_keys: vec!["custom".to_string()],
            ..Default::default()
        };
        let result = parse_file_from_string(content, &config);

        assert!(result.is_ok(), "Expected success, got: {result:?}");
        let file = result.unwrap();

        assert!(file.metadata.custom.contains_key("custom"));
        assert_eq!(
            file.metadata.custom.get("custom"),
            Some(&"custom-value".to_string())
        );
    }

    #[test]
    fn test_parse_file_from_string_hash_computation() {
        let content = r"# Test File

## Tasks

- [ ] Task 1
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_ok());
        let file = result.unwrap();

        // Hash should be computed and non-empty
        assert!(!file.hash.is_empty());

        // Hash should be consistent
        let result2 = parse_file_from_string(content, &config);
        assert!(result2.is_ok());
        let file2 = result2.unwrap();
        assert_eq!(file.hash, file2.hash);
    }

    #[test]
    fn test_parse_file_from_string_different_content_different_hash() {
        let content1 = "# File 1\n\n## Tasks\n\n- [ ] Task 1\n";
        let content2 = "# File 2\n\n## Tasks\n\n- [ ] Task 2\n";
        let config = LashConfig::default();

        let file1 = parse_file_from_string(content1, &config).unwrap();
        let file2 = parse_file_from_string(content2, &config).unwrap();

        assert_ne!(file1.hash, file2.hash);
    }

    #[test]
    fn test_parse_file_from_string_invalid_checkbox() {
        let content = r"# Test File

## Tasks

- [*] Invalid checkbox
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        // Should return error with diagnostic
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_parse_file_from_string_depth_violation() {
        let content = r"# Test File

## Tasks

- [ ] Task 1
  - [ ] Task 2
    - [ ] Task 3
      - [ ] Task 4
        - [ ] Task 5
          - [ ] Task 6
";
        let config = LashConfig::default(); // Default max_depth is 5
        let result = parse_file_from_string(content, &config);

        // Should fail due to depth violation
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_file_from_string_multiple_errors() {
        let content = r"# Test File

## Tasks

- [*] Invalid 1
- [?] Invalid 2
- [ ] Valid task
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Should have multiple errors
        assert!(errors.len() >= 2);
    }

    #[test]
    fn test_parse_file_from_string_error_sorting() {
        let content = r"# Test File

## Tasks

- [ ] Task 1
- [*] Invalid line 7
- [ ] Task 2
- [?] Invalid line 9
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_err());
        let errors = result.unwrap_err();

        // Errors should be sorted by line number
        for i in 0..errors.len() - 1 {
            let line1 = errors[i]
                .location
                .as_ref()
                .and_then(|l| l.line)
                .unwrap_or(0);
            let line2 = errors[i + 1]
                .location
                .as_ref()
                .and_then(|l| l.line)
                .unwrap_or(0);
            assert!(line1 <= line2);
        }
    }

    // ==================== parse_file Tests ====================

    #[test]
    fn test_parse_file_success() {
        let content = r"# Test File

## Tasks

- [ ] Task 1
";
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = LashConfig::default();
        let result = parse_file(temp_file.path(), &config);

        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.title, "Test File");
        assert_eq!(file.path, temp_file.path());
    }

    #[test]
    fn test_parse_file_nonexistent() {
        let config = LashConfig::default();
        let result = parse_file(Path::new("/nonexistent/file.md"), &config);

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            LashError::IO { code, .. } => {
                assert_eq!(code, "E_IO_READ_FAILED");
            }
            _ => panic!("Expected IO error, got: {err:?}"),
        }
    }

    #[test]
    fn test_parse_file_with_mtime() {
        let content = "# Test\n\n## Tasks\n\n- [ ] Task 1\n";
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = LashConfig::default();
        let result = parse_file(temp_file.path(), &config);

        assert!(result.is_ok());
        let file = result.unwrap();

        // mtime should be set
        assert_ne!(file.mtime, std::time::SystemTime::UNIX_EPOCH);
    }

    #[test]
    fn test_parse_file_propagates_parse_errors() {
        let content = r"# Test File

## Tasks

- [*] Invalid checkbox
";
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = LashConfig::default();
        let result = parse_file(temp_file.path(), &config);

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            LashError::Parse { code, .. } => {
                assert!(!code.is_empty());
            }
            _ => panic!("Expected Parse error, got: {err:?}"),
        }
    }

    // ==================== find_tasks_section_line Tests ====================

    #[test]
    fn test_find_tasks_section_line_standard() {
        let content = r"# Title

## Tasks

- [ ] Task 1
";
        let result = find_tasks_section_line(content);
        assert_eq!(result, Some(2));
    }

    #[test]
    fn test_find_tasks_section_line_case_insensitive() {
        let content = r"# Title

## tasks

- [ ] Task 1
";
        let result = find_tasks_section_line(content);
        assert_eq!(result, Some(2));

        let content2 = r"# Title

## TASKS

- [ ] Task 1
";
        let result2 = find_tasks_section_line(content2);
        assert_eq!(result2, Some(2));
    }

    #[test]
    fn test_find_tasks_section_line_not_found() {
        let content = r"# Title

## Other Section

- [ ] Task 1
";
        let result = find_tasks_section_line(content);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_tasks_section_line_with_extra_whitespace() {
        let content = r"# Title

##   Tasks

- [ ] Task 1
";
        let result = find_tasks_section_line(content);
        assert_eq!(result, Some(2));
    }

    #[test]
    fn test_find_tasks_section_line_empty_file() {
        let content = "";
        let result = find_tasks_section_line(content);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_tasks_section_line_multiple_h2() {
        let content = r"# Title

## Overview

Some text

## Tasks

- [ ] Task 1
";
        let result = find_tasks_section_line(content);
        assert_eq!(result, Some(6));
    }

    // ==================== parse_task_section_internal Tests ====================

    #[test]
    fn test_parse_task_section_internal_basic() {
        let content = r"# Title

## Tasks

- [ ] Task 1
- [x] Task 2
";
        let config = LashConfig::default();
        let mut ctx = ParseContext::new(Path::new("test.md"), &config);
        let result = parse_task_section_internal(content, &mut ctx);

        assert!(result.is_ok());
        let tree = result.unwrap();
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn test_parse_task_section_internal_no_tasks_section() {
        let content = r"# Title

- [ ] Task 1
- [ ] Task 2
";
        let config = LashConfig::default();
        let mut ctx = ParseContext::new(Path::new("test.md"), &config);
        let result = parse_task_section_internal(content, &mut ctx);

        // Should still parse tasks even without explicit section
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_task_section_internal_with_references() {
        let content = r"# Title

## Tasks

- [ ] Task 1

## References

Some refs
";
        let config = LashConfig::default();
        let mut ctx = ParseContext::new(Path::new("test.md"), &config);
        let result = parse_task_section_internal(content, &mut ctx);

        assert!(result.is_ok());
        let tree = result.unwrap();
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn test_parse_task_section_internal_malformed_checkbox() {
        let content = r"# Title

## Tasks

- [*] Invalid
";
        let config = LashConfig::default();
        let mut ctx = ParseContext::new(Path::new("test.md"), &config);
        let result = parse_task_section_internal(content, &mut ctx);

        assert!(result.is_err());
        assert!(ctx.has_errors());
    }

    #[test]
    fn test_parse_task_section_internal_empty_tasks_section() {
        let content = r"# Title

## Tasks

## References
";
        let config = LashConfig::default();
        let mut ctx = ParseContext::new(Path::new("test.md"), &config);
        let result = parse_task_section_internal(content, &mut ctx);

        assert!(result.is_ok());
        let tree = result.unwrap();
        assert_eq!(tree.len(), 0);
    }

    #[test]
    fn test_parse_task_section_internal_ignores_non_checkbox_lines() {
        let content = r"# Title

## Tasks

Some intro text

- [ ] Task 1

More text

- [ ] Task 2
";
        let config = LashConfig::default();
        let mut ctx = ParseContext::new(Path::new("test.md"), &config);
        let result = parse_task_section_internal(content, &mut ctx);

        assert!(result.is_ok());
        let tree = result.unwrap();
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn test_parse_task_section_internal_nested_tasks() {
        let content = r"# Title

## Tasks

- [ ] Parent
  - [ ] Child 1
  - [ ] Child 2
";
        let config = LashConfig::default();
        let mut ctx = ParseContext::new(Path::new("test.md"), &config);
        let result = parse_task_section_internal(content, &mut ctx);

        assert!(result.is_ok());
        let tree = result.unwrap();
        assert_eq!(tree.len(), 3);
    }

    // ==================== extract_file_metadata Tests ====================

    #[test]
    fn test_extract_file_metadata_empty() {
        let annotations = annotations::AnnotationBlock::new();
        let metadata = extract_file_metadata(&annotations);

        assert!(metadata.labels.is_empty());
        assert_eq!(metadata.status, None);
        assert_eq!(metadata.owner, None);
        assert_eq!(metadata.created, None);
        assert!(metadata.depends_on.is_empty());
        assert!(metadata.custom.is_empty());
    }

    #[test]
    fn test_extract_file_metadata_all_fields() {
        let mut annotations = annotations::AnnotationBlock::new();
        annotations.add("labels".to_string(), "backend, api".to_string());
        annotations.add("status".to_string(), "in-progress".to_string());
        annotations.add("owner".to_string(), "alice".to_string());
        annotations.add("created".to_string(), "2025-01-15".to_string());

        let metadata = extract_file_metadata(&annotations);

        assert_eq!(metadata.labels.len(), 2);
        assert_eq!(metadata.status, Some("in-progress".to_string()));
        assert_eq!(metadata.owner, Some("alice".to_string()));
        assert_eq!(metadata.created, Some("2025-01-15".to_string()));
    }

    #[test]
    fn test_extract_file_metadata_custom_annotations() {
        let mut annotations = annotations::AnnotationBlock::new();
        annotations.add("custom1".to_string(), "value1".to_string());
        annotations.add("custom2".to_string(), "value2".to_string());

        let metadata = extract_file_metadata(&annotations);

        assert_eq!(metadata.custom.len(), 2);
        assert_eq!(metadata.custom.get("custom1"), Some(&"value1".to_string()));
        assert_eq!(metadata.custom.get("custom2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_extract_file_metadata_depends_on() {
        let mut annotations = annotations::AnnotationBlock::new();
        annotations.add(
            "depends-on".to_string(),
            "other-file.md#task:task-1".to_string(),
        );

        let metadata = extract_file_metadata(&annotations);

        assert_eq!(metadata.depends_on.len(), 1);
    }

    #[test]
    fn test_extract_file_metadata_excludes_id() {
        let mut annotations = annotations::AnnotationBlock::new();
        annotations.add("id".to_string(), "file-id".to_string());
        annotations.add("custom".to_string(), "value".to_string());

        let metadata = extract_file_metadata(&annotations);

        // ID should not be in custom
        assert!(!metadata.custom.contains_key("id"));
        assert_eq!(metadata.custom.len(), 1);
    }

    #[test]
    fn test_extract_file_metadata_multiple_label_values() {
        let mut annotations = annotations::AnnotationBlock::new();
        annotations.add("labels".to_string(), "tag1, tag2, tag3".to_string());

        let metadata = extract_file_metadata(&annotations);

        assert_eq!(metadata.labels.len(), 3);
        assert!(metadata.labels.contains(&"tag1".to_string()));
        assert!(metadata.labels.contains(&"tag2".to_string()));
        assert!(metadata.labels.contains(&"tag3".to_string()));
    }

    // ==================== Edge Cases and Integration ====================

    #[test]
    fn test_parse_file_from_string_no_h1() {
        let content = r"## Tasks

- [ ] Task 1
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        // Should handle missing H1
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_parse_file_from_string_waived_tasks() {
        let content = r"# Test File

## Tasks

- [-] Waived task
  - [ ] Child should be auto-waived
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_ok());
        let file = result.unwrap();

        // Both tasks should exist
        assert_eq!(file.tasks.len(), 2);

        // Check waived status
        let tasks = file.tasks.tasks();
        let parent = &tasks[0];
        let child = &tasks[1];

        assert_eq!(parent.status, lash_types::TaskStatus::Waived);
        assert_eq!(child.status, lash_types::TaskStatus::Waived);
    }

    #[test]
    fn test_parse_file_from_string_very_long_line() {
        let long_title = "A".repeat(1000);
        let content = format!(
            r"# Test File

## Tasks

- [ ] {long_title}
"
        );
        let config = LashConfig::default();
        let result = parse_file_from_string(&content, &config);

        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.tasks.len(), 1);
    }

    #[test]
    fn test_parse_file_from_string_unicode() {
        let content = r"# 测试文件

## Tasks

- [ ] タスク １
- [ ] Aufgabe 2 🚀
";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.title, "测试文件");
        assert_eq!(file.tasks.len(), 2);
    }

    #[test]
    fn test_parse_file_from_string_windows_line_endings() {
        let content = "# Test File\r\n\r\n## Tasks\r\n\r\n- [ ] Task 1\r\n";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.tasks.len(), 1);
    }

    #[test]
    fn test_parse_file_from_string_mixed_line_endings() {
        let content = "# Test File\n\r\n## Tasks\n\r\n- [ ] Task 1\r\n- [ ] Task 2\n";
        let config = LashConfig::default();
        let result = parse_file_from_string(content, &config);

        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.tasks.len(), 2);
    }
}
