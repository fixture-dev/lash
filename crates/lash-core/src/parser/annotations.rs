//! Annotation parsing
//!
//! This module handles parsing of metadata annotations in the format `@key: value`.
//! Annotations can appear in two places:
//! - In the file header (before `## Tasks`)
//! - Inline with tasks (in trailing `[@key: value]` blocks)
//!
//! Supported annotation types include:
//! - `@id`: Unique identifier
//! - `@labels`: Comma-separated list of labels
//! - `@status`: Overall file/task status
//! - `@owner`: Assigned owner
//! - `@created`: Creation date (YYYY-MM-DD)
//! - `@estimate`: Time estimate
//! - `@depends-on`: Dependencies
//! - `@agent-note`: Notes for LLM agents
//!
//! Custom annotations can be defined in the Lash configuration.

// Allow large Result errors - our error type is intentionally rich with context
// This is acceptable for a CLI tool where errors are exceptional, not the hot path
#![allow(clippy::result_large_err)]

use chrono::NaiveDate;
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

use lash_types::{
    parse_annotation_labels, parse_dependency_ref, DependencyRef, Label, LashConfig, LashError,
    Result, TaskStatus,
};

#[cfg(test)]
use lash_types::ConfigBuilder;

/// Collection of parsed annotations
///
/// This structure holds all annotations parsed from a header block or
/// inline metadata. It supports:
/// - Single-value annotations (e.g., @id, @owner)
/// - Multi-value annotations (e.g., @labels, @depends-on)
/// - Unknown/custom annotations (when configured)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationBlock {
    /// Map of annotation key to values
    /// Keys are stored without the '@' prefix
    /// Values are stored as Vec to support multiple occurrences (e.g., multiple @depends-on)
    annotations: HashMap<String, Vec<String>>,
}

impl AnnotationBlock {
    /// Create a new empty annotation block
    #[must_use]
    pub fn new() -> Self {
        Self {
            annotations: HashMap::new(),
        }
    }

    /// Add an annotation key-value pair
    ///
    /// If the key already exists, the value is appended to the list.
    /// This supports annotations like `@depends-on` which can appear multiple times.
    ///
    /// # Arguments
    ///
    /// * `key` - Annotation key (without the '@' prefix)
    /// * `value` - Annotation value
    pub fn add(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        self.annotations.entry(key).or_default().push(value);
    }

    /// Get a single value for an annotation key
    ///
    /// Returns `None` if the key doesn't exist, or `Some(&str)` with the first value.
    /// If multiple values exist, only the first is returned.
    ///
    /// # Arguments
    ///
    /// * `key` - Annotation key (without the '@' prefix)
    #[must_use]
    pub fn get_single(&self, key: &str) -> Option<&str> {
        self.annotations.get(key)?.first().map(String::as_str)
    }

    /// Get all values for an annotation key
    ///
    /// Returns an empty slice if the key doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `key` - Annotation key (without the '@' prefix)
    #[must_use]
    pub fn get_list(&self, key: &str) -> &[String] {
        self.annotations.get(key).map_or(&[], Vec::as_slice)
    }

    /// Get a date value from an annotation
    ///
    /// Parses the annotation value as a date in YYYY-MM-DD format.
    ///
    /// # Arguments
    ///
    /// * `key` - Annotation key (without the '@' prefix)
    ///
    /// # Errors
    ///
    /// Returns error if the date format is invalid
    pub fn get_date(&self, key: &str) -> Result<Option<NaiveDate>> {
        if let Some(value) = self.get_single(key) {
            Ok(Some(parse_date(value)?))
        } else {
            Ok(None)
        }
    }

    /// Get a duration value from an annotation
    ///
    /// Parses the annotation value as a duration (e.g., "2h", "3d").
    ///
    /// # Arguments
    ///
    /// * `key` - Annotation key (without the '@' prefix)
    ///
    /// # Errors
    ///
    /// Returns error if the duration format is invalid
    pub fn get_duration(&self, key: &str) -> Result<Option<String>> {
        if let Some(value) = self.get_single(key) {
            validate_duration(value)?;
            Ok(Some(value.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Get labels from an annotation
    ///
    /// Parses the annotation value as a comma-separated list of labels.
    ///
    /// # Arguments
    ///
    /// * `key` - Annotation key (without the '@' prefix)
    #[must_use]
    pub fn get_labels(&self, key: &str) -> Vec<Label> {
        if let Some(value) = self.get_single(key) {
            parse_annotation_labels(value)
        } else {
            Vec::new()
        }
    }

    /// Get dependency references from annotations
    ///
    /// Parses all values for the `depends-on` key as dependency references.
    ///
    /// # Errors
    ///
    /// Returns error if any dependency reference has invalid syntax
    pub fn get_dependencies(&self) -> Result<Vec<DependencyRef>> {
        let mut deps = Vec::new();
        for value in self.get_list(known_keys::DEPENDS_ON) {
            deps.push(parse_dependency_ref(value)?);
        }
        Ok(deps)
    }

    /// Get status from an annotation
    ///
    /// Parses the annotation value as a task status.
    ///
    /// # Arguments
    ///
    /// * `key` - Annotation key (without the '@' prefix)
    ///
    /// # Errors
    ///
    /// Returns error if the status value is invalid
    pub fn get_status(&self, key: &str) -> Result<Option<TaskStatus>> {
        if let Some(value) = self.get_single(key) {
            Ok(Some(value.parse()?))
        } else {
            Ok(None)
        }
    }

    /// Check if an annotation key exists
    ///
    /// # Arguments
    ///
    /// * `key` - Annotation key (without the '@' prefix)
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.annotations.contains_key(key)
    }

    /// Get all annotation keys
    #[must_use]
    pub fn keys(&self) -> Vec<&str> {
        self.annotations.keys().map(String::as_str).collect()
    }

    /// Get the number of distinct annotation keys
    #[must_use]
    pub fn len(&self) -> usize {
        self.annotations.len()
    }

    /// Check if the annotation block is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.annotations.is_empty()
    }

    /// Validate that single-value annotations don't have duplicates
    ///
    /// Some annotations (like @id, @owner, @status) should only appear once.
    /// This checks for duplicates and returns an error if found.
    ///
    /// # Errors
    ///
    /// Returns error if single-value annotations have multiple values
    pub fn validate_single_values(&self) -> Result<()> {
        let single_value_keys = [
            known_keys::ID,
            known_keys::OWNER,
            known_keys::STATUS,
            known_keys::CREATED,
            known_keys::ESTIMATE,
            known_keys::AGENT_NOTE,
        ];

        for key in &single_value_keys {
            if let Some(values) = self.annotations.get(*key) {
                if values.len() > 1 {
                    return Err(LashError::Parse {
                        code: "E_PARSE_DUPLICATE_ANNOTATION",
                        message: format!("Annotation @{key} cannot appear multiple times"),
                        location: None,
                        snippet: Some(format!("@{key} appears {} times", values.len())),
                        help: Some(format!("@{key} should only be specified once")),
                    });
                }
            }
        }

        Ok(())
    }
}

impl Default for AnnotationBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a single annotation line in the format `@key: value`
///
/// This function extracts the key and value from an annotation line.
/// It validates the key format but does NOT validate against known keys -
/// that's done separately based on configuration.
///
/// # Arguments
///
/// * `line` - The line to parse (must start with '@' after trimming)
///
/// # Returns
///
/// Returns `Ok((key, value))` if the line is a valid annotation format,
/// or `Err` if it's not an annotation line or is malformed.
///
/// The key is returned without the '@' prefix and both key and value
/// are trimmed of whitespace.
///
/// # Errors
///
/// Returns error if the line doesn't have valid annotation syntax or the key format is invalid.
///
/// # Example
///
/// ```rust,ignore
/// let (key, value) = parse_annotation("@id: task-123").unwrap();
/// assert_eq!(key, "id");
/// assert_eq!(value, "task-123");
/// ```
pub fn parse_annotation(line: &str) -> Result<(String, String)> {
    let trimmed = line.trim();

    // Must start with @
    if !trimmed.starts_with('@') {
        return Err(LashError::Parse {
            code: "E_PARSE_NOT_ANNOTATION",
            message: "Line does not start with '@'".to_string(),
            location: None,
            snippet: Some(trimmed.to_string()),
            help: Some("annotations must start with '@' (e.g., @id: value)".to_string()),
        });
    }

    // Must contain a colon
    let Some((key_part, value_part)) = trimmed.split_once(':') else {
        return Err(LashError::Parse {
            code: "E_PARSE_MISSING_COLON",
            message: "Annotation missing colon separator".to_string(),
            location: None,
            snippet: Some(trimmed.to_string()),
            help: Some("annotations must be in format: @key: value".to_string()),
        });
    };

    // Extract and validate key
    let key = key_part[1..].trim(); // Remove @ prefix and trim
    if key.is_empty() {
        return Err(LashError::Parse {
            code: "E_PARSE_EMPTY_KEY",
            message: "Annotation key cannot be empty".to_string(),
            location: None,
            snippet: Some(trimmed.to_string()),
            help: Some("annotations must be in format: @key: value".to_string()),
        });
    }

    // Validate key format: alphanumeric, hyphens, underscores only
    if !is_valid_key(key) {
        return Err(LashError::Parse {
            code: "E_PARSE_INVALID_KEY",
            message: format!("Invalid annotation key: '{key}'"),
            location: None,
            snippet: Some(trimmed.to_string()),
            help: Some(
                "annotation keys must be alphanumeric with hyphens or underscores".to_string(),
            ),
        });
    }

    // Extract value (trim leading whitespace only, preserve trailing)
    let value = value_part.trim_start();

    Ok((key.to_string(), value.to_string()))
}

/// Parse an inline annotation block in the format `[@key: value, @key2: value2]`
///
/// Inline annotations appear at the end of checkbox lines in square brackets.
/// Multiple annotations are comma-separated.
///
/// # Arguments
///
/// * `text` - The text containing the inline annotation block
///
/// # Returns
///
/// Returns `Some(AnnotationBlock)` if valid inline annotations are found,
/// or `None` if no inline annotations are present.
///
/// # Errors
///
/// Returns error if the inline annotation block has invalid syntax (e.g., unclosed brackets,
/// invalid key format, missing colon separator).
///
/// # Example
///
/// ```rust,ignore
/// let text = "Task description [@owner: alice, @estimate: 2h]";
/// let block = parse_inline_annotations(text).unwrap();
/// assert_eq!(block.get_single("owner"), Some("alice"));
/// ```
pub fn parse_inline_annotations(text: &str) -> Result<Option<AnnotationBlock>> {
    // Find the last occurrence of [@...] pattern
    let Some(start) = text.rfind("[@") else {
        return Ok(None);
    };

    let Some(end) = text[start..].find(']') else {
        return Err(LashError::Parse {
            code: "E_PARSE_UNCLOSED_BRACKET",
            message: "Inline annotation block missing closing bracket".to_string(),
            location: None,
            snippet: Some(text[start..].to_string()),
            help: Some(
                "inline annotations must be enclosed in brackets: [@key: value]".to_string(),
            ),
        });
    };

    let content = &text[start + 2..start + end]; // Extract content between [@ and ]
    let mut block = AnnotationBlock::new();

    // Split by comma and parse each annotation
    for part in content.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Add @ prefix if not present (for consistency with parse_annotation)
        let line = if trimmed.starts_with('@') {
            trimmed.to_string()
        } else {
            format!("@{trimmed}")
        };

        let (key, value) = parse_annotation(&line)?;
        block.add(key, value);
    }

    Ok(Some(block))
}

/// Parse multiple annotation lines from a block of text
///
/// This handles multiline values by detecting indented continuation lines.
///
/// # Arguments
///
/// * `lines` - Iterator of lines to parse
/// * `config` - Configuration for validation (optional, can be None for no validation)
///
/// # Returns
///
/// Returns an `AnnotationBlock` with all parsed annotations
///
/// # Errors
///
/// Returns error if any annotation has invalid syntax or unknown keys
pub fn parse_annotation_block<'a>(
    lines: impl Iterator<Item = &'a str>,
    config: Option<&LashConfig>,
) -> Result<AnnotationBlock> {
    let mut block = AnnotationBlock::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();

    for line in lines {
        let trimmed = line.trim_end(); // Preserve leading whitespace for indentation check

        // Skip blank lines
        if trimmed.is_empty() {
            continue;
        }

        // Check if this is a new annotation or a continuation
        if trimmed.trim_start().starts_with('@') {
            // Save previous annotation if any
            if let Some(key) = current_key.take() {
                block.add(key, current_value.trim_end().to_string());
                current_value.clear();
            }

            // Parse new annotation
            let (key, value) = parse_annotation(trimmed)?;

            // Validate key if config provided
            if let Some(cfg) = config {
                validate_annotation_key(&key, cfg)?;
            }

            current_key = Some(key);
            current_value = value.to_string();
        } else if current_key.is_some() && trimmed.starts_with(' ') {
            // Continuation line (indented)
            if !current_value.is_empty() {
                current_value.push('\n');
            }
            current_value.push_str(trimmed.trim_start());
        } else {
            // Non-annotation line - stop processing
            break;
        }
    }

    // Save last annotation if any
    if let Some(key) = current_key {
        block.add(key, current_value.trim_end().to_string());
    }

    // Validate no duplicates for single-value annotations
    block.validate_single_values()?;

    Ok(block)
}

/// Validate annotation key against known and custom keys
///
/// # Arguments
///
/// * `key` - The annotation key to validate
/// * `config` - Configuration containing custom annotation keys
///
/// # Errors
///
/// Returns error if the key is unknown and not in custom config
fn validate_annotation_key(key: &str, config: &LashConfig) -> Result<()> {
    // Check if it's a known built-in key
    if known_keys::ALL.contains(&key) {
        return Ok(());
    }

    // Check if it's in custom keys
    if config.custom_annotation_keys.contains(&key.to_string()) {
        return Ok(());
    }

    // Unknown key - provide helpful error
    Err(LashError::Lint {
        code: "E_LINT_UNKNOWN_ANNOTATION",
        message: format!("Unknown annotation key: '@{key}'"),
        location: None,
        snippet: Some(format!("@{key}")),
        help: Some(format!(
            "known keys: {}. To use custom annotations, add to .lash/config.toml: custom_annotation_keys = [\"{}\"]",
            known_keys::ALL.join(", "),
            key
        )),
    })
}

/// Check if a string is a valid annotation key
///
/// Valid keys:
/// - Alphanumeric characters
/// - Hyphens (-)
/// - Underscores (_)
/// - Cannot be empty
#[must_use]
fn is_valid_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }

    key.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Parse a date value in YYYY-MM-DD format
///
/// # Arguments
///
/// * `value` - The date string to parse
///
/// # Errors
///
/// Returns error if the date format is invalid
fn parse_date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|e| LashError::Parse {
        code: "E_PARSE_INVALID_DATE",
        message: format!("Invalid date format: '{value}'"),
        location: None,
        snippet: Some(value.to_string()),
        help: Some(format!("dates must be in YYYY-MM-DD format (error: {e})")),
    })
}

/// Validate duration format
///
/// Valid formats: `\d+[hdwmy]` (hours, days, weeks, months, years)
///
/// # Arguments
///
/// * `value` - The duration string to validate
///
/// # Errors
///
/// Returns error if the duration format is invalid
fn validate_duration(value: &str) -> Result<()> {
    static DURATION_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = DURATION_REGEX.get_or_init(|| Regex::new(r"^\d+[hdwmy]$").unwrap());

    if !regex.is_match(value.trim()) {
        return Err(LashError::Parse {
            code: "E_PARSE_INVALID_DURATION",
            message: format!("Invalid duration format: '{value}'"),
            location: None,
            snippet: Some(value.to_string()),
            help: Some(
                "durations must be in format: <number><unit> where unit is h/d/w/m/y (e.g., 2h, 3d)"
                    .to_string(),
            ),
        });
    }

    Ok(())
}

/// Known built-in annotation keys
///
/// These are the standard annotations supported by Lash.
/// Custom annotations can be added via configuration.
pub mod known_keys {
    /// Task or file identifier
    pub const ID: &str = "id";

    /// Comma-separated labels
    pub const LABELS: &str = "labels";

    /// Status (open, done, waived, blocked)
    pub const STATUS: &str = "status";

    /// Assigned owner
    pub const OWNER: &str = "owner";

    /// Creation date (YYYY-MM-DD)
    pub const CREATED: &str = "created";

    /// Time estimate
    pub const ESTIMATE: &str = "estimate";

    /// Dependency reference
    pub const DEPENDS_ON: &str = "depends-on";

    /// Note for LLM agents
    pub const AGENT_NOTE: &str = "agent-note";

    /// All known annotation keys
    pub const ALL: &[&str] = &[
        ID, LABELS, STATUS, OWNER, CREATED, ESTIMATE, DEPENDS_ON, AGENT_NOTE,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== AnnotationBlock Tests ====================

    #[test]
    fn test_annotation_block_creation() {
        let block = AnnotationBlock::new();
        assert!(block.is_empty());
        assert_eq!(block.len(), 0);
    }

    #[test]
    fn test_annotation_block_add_single() {
        let mut block = AnnotationBlock::new();
        block.add("id", "task-123");

        assert!(!block.is_empty());
        assert_eq!(block.len(), 1);
        assert_eq!(block.get_single("id"), Some("task-123"));
        assert!(block.contains("id"));
    }

    #[test]
    fn test_annotation_block_add_multiple() {
        let mut block = AnnotationBlock::new();
        block.add("depends-on", "task-1");
        block.add("depends-on", "task-2");

        assert_eq!(block.len(), 1); // One key with multiple values
        let values = block.get_list("depends-on");
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], "task-1");
        assert_eq!(values[1], "task-2");
    }

    #[test]
    fn test_annotation_block_get_single_returns_first() {
        let mut block = AnnotationBlock::new();
        block.add("test", "first");
        block.add("test", "second");

        assert_eq!(block.get_single("test"), Some("first"));
    }

    #[test]
    fn test_annotation_block_missing_key() {
        let block = AnnotationBlock::new();
        assert_eq!(block.get_single("missing"), None);
        assert_eq!(block.get_list("missing").len(), 0);
        assert!(!block.contains("missing"));
    }

    #[test]
    fn test_annotation_block_keys() {
        let mut block = AnnotationBlock::new();
        block.add("id", "123");
        block.add("owner", "alice");
        block.add("labels", "tag1,tag2");

        let keys = block.keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"id"));
        assert!(keys.contains(&"owner"));
        assert!(keys.contains(&"labels"));
    }

    #[test]
    fn test_annotation_block_get_date() {
        let mut block = AnnotationBlock::new();
        block.add("created", "2025-01-15");

        let date = block.get_date("created").unwrap().unwrap();
        assert_eq!(date.to_string(), "2025-01-15");
    }

    #[test]
    fn test_annotation_block_get_date_invalid() {
        let mut block = AnnotationBlock::new();
        block.add("created", "invalid-date");

        assert!(block.get_date("created").is_err());
    }

    #[test]
    fn test_annotation_block_get_duration() {
        let mut block = AnnotationBlock::new();
        block.add("estimate", "2h");

        let duration = block.get_duration("estimate").unwrap().unwrap();
        assert_eq!(duration, "2h");
    }

    #[test]
    fn test_annotation_block_get_duration_invalid() {
        let mut block = AnnotationBlock::new();
        block.add("estimate", "invalid");

        assert!(block.get_duration("estimate").is_err());
    }

    #[test]
    fn test_annotation_block_get_labels() {
        let mut block = AnnotationBlock::new();
        block.add("labels", "backend, api, database");

        let labels = block.get_labels("labels");
        assert_eq!(labels.len(), 3);
        assert!(labels.iter().any(|l| l.name == "api"));
        assert!(labels.iter().any(|l| l.name == "backend"));
        assert!(labels.iter().any(|l| l.name == "database"));
    }

    #[test]
    fn test_annotation_block_get_status() {
        let mut block = AnnotationBlock::new();
        block.add("status", "done");

        let status = block.get_status("status").unwrap().unwrap();
        assert_eq!(status, TaskStatus::Done);
    }

    #[test]
    fn test_annotation_block_get_dependencies() {
        let mut block = AnnotationBlock::new();
        block.add("depends-on", "core/api.md");
        block.add("depends-on", "db#init");

        let deps = block.get_dependencies().unwrap();
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_annotation_block_validate_single_values_ok() {
        let mut block = AnnotationBlock::new();
        block.add("id", "task-1");
        block.add("owner", "alice");
        block.add("depends-on", "task-2");
        block.add("depends-on", "task-3");

        assert!(block.validate_single_values().is_ok());
    }

    #[test]
    fn test_annotation_block_validate_single_values_duplicate_id() {
        let mut block = AnnotationBlock::new();
        block.add("id", "task-1");
        block.add("id", "task-2");

        assert!(block.validate_single_values().is_err());
    }

    // ==================== parse_annotation() Tests ====================

    #[test]
    fn test_parse_annotation_basic() {
        let (key, value) = parse_annotation("@id: task-123").unwrap();
        assert_eq!(key, "id");
        assert_eq!(value, "task-123");
    }

    #[test]
    fn test_parse_annotation_with_whitespace() {
        let (key, value) = parse_annotation("  @owner:   alice  ").unwrap();
        assert_eq!(key, "owner");
        // The whole line is trimmed first, then value gets trim_start
        // So "  @owner:   alice  " -> "@owner:   alice" -> "alice"
        assert_eq!(value, "alice");
    }

    #[test]
    fn test_parse_annotation_with_hyphens() {
        let (key, value) = parse_annotation("@depends-on: core/api.md").unwrap();
        assert_eq!(key, "depends-on");
        assert_eq!(value, "core/api.md");
    }

    #[test]
    fn test_parse_annotation_with_underscores() {
        let (key, value) = parse_annotation("@custom_key: value").unwrap();
        assert_eq!(key, "custom_key");
        assert_eq!(value, "value");
    }

    #[test]
    fn test_parse_annotation_multiword_value() {
        let (key, value) = parse_annotation("@agent-note: This is a long note").unwrap();
        assert_eq!(key, "agent-note");
        assert_eq!(value, "This is a long note");
    }

    #[test]
    fn test_parse_annotation_colon_in_value() {
        let (key, value) = parse_annotation("@url: https://example.com").unwrap();
        assert_eq!(key, "url");
        assert_eq!(value, "https://example.com");
    }

    #[test]
    fn test_parse_annotation_no_at_prefix() {
        assert!(parse_annotation("id: value").is_err());
    }

    #[test]
    fn test_parse_annotation_no_colon() {
        assert!(parse_annotation("@id value").is_err());
    }

    #[test]
    fn test_parse_annotation_empty_key() {
        assert!(parse_annotation("@: value").is_err());
    }

    #[test]
    fn test_parse_annotation_empty_value() {
        let (key, value) = parse_annotation("@id:").unwrap();
        assert_eq!(key, "id");
        assert_eq!(value, "");
    }

    #[test]
    fn test_parse_annotation_invalid_key_characters() {
        assert!(parse_annotation("@invalid.key: value").is_err());
        assert!(parse_annotation("@invalid!key: value").is_err());
        assert!(parse_annotation("@invalid key: value").is_err());
    }

    #[test]
    fn test_parse_annotation_numeric_key() {
        // Keys can contain numbers, just can't start with them
        let (key, value) = parse_annotation("@key123: value").unwrap();
        assert_eq!(key, "key123");
        assert_eq!(value, "value");
    }

    // ==================== parse_inline_annotations() Tests ====================

    #[test]
    fn test_parse_inline_annotations_single() {
        let text = "Task description [@owner: alice]";
        let block = parse_inline_annotations(text).unwrap().unwrap();
        assert_eq!(block.get_single("owner"), Some("alice"));
    }

    #[test]
    fn test_parse_inline_annotations_multiple() {
        let text = "Task [@owner: alice, @estimate: 2h]";
        let block = parse_inline_annotations(text).unwrap().unwrap();
        assert_eq!(block.get_single("owner"), Some("alice"));
        assert_eq!(block.get_single("estimate"), Some("2h"));
    }

    #[test]
    fn test_parse_inline_annotations_without_at_prefix() {
        let text = "Task [@owner: alice, estimate: 2h]";
        let block = parse_inline_annotations(text).unwrap().unwrap();
        assert_eq!(block.get_single("owner"), Some("alice"));
        assert_eq!(block.get_single("estimate"), Some("2h"));
    }

    #[test]
    fn test_parse_inline_annotations_no_annotations() {
        let text = "Task description without annotations";
        let result = parse_inline_annotations(text).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_inline_annotations_unclosed_bracket() {
        let text = "Task [@owner: alice";
        assert!(parse_inline_annotations(text).is_err());
    }

    #[test]
    fn test_parse_inline_annotations_last_occurrence() {
        let text = "Task [not annotation] and [@owner: alice]";
        let block = parse_inline_annotations(text).unwrap().unwrap();
        assert_eq!(block.get_single("owner"), Some("alice"));
    }

    #[test]
    fn test_parse_inline_annotations_with_whitespace() {
        let text = "Task [@  owner:  alice  ,  estimate: 2h  ]";
        let block = parse_inline_annotations(text).unwrap().unwrap();
        // After trimming the annotation and using parse_annotation, trailing spaces are removed
        assert_eq!(block.get_single("owner"), Some("alice"));
        assert_eq!(block.get_single("estimate"), Some("2h"));
    }

    // ==================== parse_annotation_block() Tests ====================

    #[test]
    fn test_parse_annotation_block_single() {
        let lines = vec!["@id: task-123"];
        let block = parse_annotation_block(lines.into_iter(), None).unwrap();
        assert_eq!(block.get_single("id"), Some("task-123"));
    }

    #[test]
    fn test_parse_annotation_block_multiple() {
        let lines = vec!["@id: task-123", "@owner: alice", "@estimate: 2h"];
        let block = parse_annotation_block(lines.into_iter(), None).unwrap();
        assert_eq!(block.get_single("id"), Some("task-123"));
        assert_eq!(block.get_single("owner"), Some("alice"));
        assert_eq!(block.get_single("estimate"), Some("2h"));
    }

    #[test]
    fn test_parse_annotation_block_multiline_value() {
        let lines = vec![
            "@agent-note: This is a long note",
            "  that continues on the next line",
            "  and another line",
        ];
        let block = parse_annotation_block(lines.into_iter(), None).unwrap();
        assert_eq!(
            block.get_single("agent-note"),
            Some("This is a long note\nthat continues on the next line\nand another line")
        );
    }

    #[test]
    fn test_parse_annotation_block_multiple_depends_on() {
        let lines = vec!["@depends-on: task-1", "@depends-on: task-2"];
        let block = parse_annotation_block(lines.into_iter(), None).unwrap();
        let values = block.get_list("depends-on");
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], "task-1");
        assert_eq!(values[1], "task-2");
    }

    #[test]
    fn test_parse_annotation_block_skip_blank_lines() {
        let lines = vec!["@id: task-123", "", "@owner: alice"];
        let block = parse_annotation_block(lines.into_iter(), None).unwrap();
        assert_eq!(block.get_single("id"), Some("task-123"));
        assert_eq!(block.get_single("owner"), Some("alice"));
    }

    #[test]
    fn test_parse_annotation_block_stops_at_non_annotation() {
        let lines = vec!["@id: task-123", "Regular text", "@owner: alice"];
        let block = parse_annotation_block(lines.into_iter(), None).unwrap();
        assert_eq!(block.get_single("id"), Some("task-123"));
        assert_eq!(block.get_single("owner"), None);
    }

    #[test]
    fn test_parse_annotation_block_with_config_known_keys() {
        let config = ConfigBuilder::new().root("/tmp").build().unwrap();
        let lines = vec!["@id: task-123", "@owner: alice"];
        let block = parse_annotation_block(lines.into_iter(), Some(&config)).unwrap();
        assert_eq!(block.get_single("id"), Some("task-123"));
        assert_eq!(block.get_single("owner"), Some("alice"));
    }

    #[test]
    fn test_parse_annotation_block_with_config_unknown_key() {
        let config = ConfigBuilder::new().root("/tmp").build().unwrap();
        let lines = vec!["@unknown: value"];
        let result = parse_annotation_block(lines.into_iter(), Some(&config));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_annotation_block_with_config_custom_key() {
        let config = ConfigBuilder::new()
            .root("/tmp")
            .custom_annotation_keys(vec!["priority".to_string()])
            .build()
            .unwrap();
        let lines = vec!["@priority: high"];
        let block = parse_annotation_block(lines.into_iter(), Some(&config)).unwrap();
        assert_eq!(block.get_single("priority"), Some("high"));
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_is_valid_key() {
        assert!(is_valid_key("id"));
        assert!(is_valid_key("depends-on"));
        assert!(is_valid_key("custom_key"));
        assert!(is_valid_key("key123"));
        assert!(is_valid_key("a"));

        assert!(!is_valid_key(""));
        assert!(!is_valid_key("invalid.key"));
        assert!(!is_valid_key("invalid!key"));
        assert!(!is_valid_key("invalid key"));
    }

    #[test]
    fn test_parse_date_valid() {
        let date = parse_date("2025-01-15").unwrap();
        assert_eq!(date.to_string(), "2025-01-15");
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("2025/01/15").is_err());
        assert!(parse_date("01-15-2025").is_err());
    }

    #[test]
    fn test_validate_duration_valid() {
        assert!(validate_duration("2h").is_ok());
        assert!(validate_duration("3d").is_ok());
        assert!(validate_duration("1w").is_ok());
        assert!(validate_duration("2m").is_ok());
        assert!(validate_duration("1y").is_ok());
        assert!(validate_duration("10h").is_ok());
    }

    #[test]
    fn test_validate_duration_invalid() {
        assert!(validate_duration("invalid").is_err());
        assert!(validate_duration("2").is_err());
        assert!(validate_duration("h2").is_err());
        assert!(validate_duration("2hours").is_err());
        assert!(validate_duration("2 h").is_err());
    }

    // ==================== Known Keys Tests ====================

    #[test]
    fn test_known_annotation_keys() {
        assert_eq!(known_keys::ID, "id");
        assert_eq!(known_keys::LABELS, "labels");
        assert_eq!(known_keys::STATUS, "status");
        assert_eq!(known_keys::OWNER, "owner");
        assert_eq!(known_keys::CREATED, "created");
        assert_eq!(known_keys::ESTIMATE, "estimate");
        assert_eq!(known_keys::DEPENDS_ON, "depends-on");
        assert_eq!(known_keys::AGENT_NOTE, "agent-note");

        assert_eq!(known_keys::ALL.len(), 8);
        assert!(known_keys::ALL.contains(&"id"));
        assert!(known_keys::ALL.contains(&"depends-on"));
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_full_annotation_workflow() {
        let lines = vec![
            "@id: my-task",
            "@labels: backend, api, database",
            "@status: open",
            "@owner: alice",
            "@created: 2025-01-15",
            "@estimate: 2h",
            "@depends-on: core/api.md",
            "@depends-on: db#init",
            "@agent-note: This is important",
            "  and continues here",
        ];

        let config = ConfigBuilder::new().root("/tmp").build().unwrap();
        let block = parse_annotation_block(lines.into_iter(), Some(&config)).unwrap();

        assert_eq!(block.get_single("id"), Some("my-task"));
        assert_eq!(block.get_labels("labels").len(), 3);
        assert_eq!(
            block.get_status("status").unwrap().unwrap(),
            TaskStatus::Open
        );
        assert_eq!(block.get_single("owner"), Some("alice"));
        assert_eq!(
            block.get_date("created").unwrap().unwrap().to_string(),
            "2025-01-15"
        );
        assert_eq!(block.get_duration("estimate").unwrap().unwrap(), "2h");
        assert_eq!(block.get_dependencies().unwrap().len(), 2);
        assert_eq!(
            block.get_single("agent-note"),
            Some("This is important\nand continues here")
        );
    }

    #[test]
    fn test_inline_annotations_with_validation() {
        let text = "- [ ] Task description [@owner: bob, @estimate: 3d]";
        let block = parse_inline_annotations(text).unwrap().unwrap();

        assert_eq!(block.get_single("owner"), Some("bob"));
        assert_eq!(block.get_duration("estimate").unwrap().unwrap(), "3d");
    }
}
