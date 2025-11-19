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

use std::collections::HashMap;

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
}

impl Default for AnnotationBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a single annotation line in the format `@key: value`
///
/// This function extracts the key and value from an annotation line.
/// It does NOT validate the key or value - that's done later based on
/// the configuration.
///
/// # Arguments
///
/// * `line` - The line to parse (must start with '@')
///
/// # Returns
///
/// Returns `Some((key, value))` if the line is a valid annotation format,
/// or `None` if it's not an annotation line or is malformed.
///
/// The key is returned without the '@' prefix and both key and value
/// are trimmed of whitespace.
///
/// # Example
///
/// ```rust,ignore
/// let (key, value) = parse_annotation("@id: task-123").unwrap();
/// assert_eq!(key, "id");
/// assert_eq!(value, "task-123");
/// ```
#[must_use]
pub fn parse_annotation(_line: &str) -> Option<(String, String)> {
    // TODO: Implement in Task #3
    // This function will be implemented in the "Implement Annotation Parser" task
    None
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
/// # Example
///
/// ```rust,ignore
/// let text = "Task description [@owner: alice, @estimate: 2h]";
/// let block = parse_inline_annotations(text).unwrap();
/// assert_eq!(block.get_single("owner"), Some("alice"));
/// ```
#[must_use]
pub fn parse_inline_annotations(_text: &str) -> Option<AnnotationBlock> {
    // TODO: Implement in Task #3
    // This function will be implemented in the "Implement Annotation Parser" task
    None
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
}
