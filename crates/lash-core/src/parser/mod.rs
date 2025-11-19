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
//! ```rust,ignore
//! use lash_core::parser::parse_file;
//! use lash_types::LashConfig;
//! use std::path::Path;
//!
//! let config = LashConfig::default();
//! let file = parse_file(Path::new("tasks.md"), &config)?;
//!
//! println!("Parsed {} tasks from {}", file.tasks.len(), file.metadata.id);
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

use lash_types::{Diagnostic, LashConfig, LashError, Location, Result, TaskFile};
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
/// ```rust,ignore
/// use lash_core::parser::parse_file;
/// use lash_types::LashConfig;
/// use std::path::Path;
///
/// let config = LashConfig::default();
/// match parse_file(Path::new("tasks.md"), &config) {
///     Ok(file) => println!("Parsed {} tasks", file.tasks.len()),
///     Err(diagnostics) => {
///         for diag in diagnostics {
///             eprintln!("{}", diag);
///         }
///     }
/// }
/// ```
#[allow(clippy::result_large_err)] // LashError is intentionally large for rich context
pub fn parse_file(_path: &Path, _config: &LashConfig) -> Result<TaskFile> {
    // TODO: Implement in Task #6
    // This is a placeholder that will be implemented after all component
    // parsers are in place (checkbox, annotations, builder, etc.)
    todo!("parse_file will be implemented in Task #6")
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
pub fn parse_file_from_string(_content: &str, _config: &LashConfig) -> ParseResult<TaskFile> {
    // TODO: Implement in Task #6
    todo!("parse_file_from_string will be implemented in Task #6")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_section_enum() {
        assert_eq!(Section::Header, Section::Header);
        assert_ne!(Section::Header, Section::Tasks);
        assert_ne!(Section::Tasks, Section::References);
    }
}
