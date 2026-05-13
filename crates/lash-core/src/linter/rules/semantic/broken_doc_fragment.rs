//! Broken documentation fragment validation rule
//!
//! Validates that `@doc` annotations with fragment identifiers reference
//! headings that actually exist in the target document.

use lash_types::{dependency::DocRef, Severity, Task, TaskFile};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::fs;
use std::path::Path;

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates documentation fragment references
///
/// This rule ensures that when a `@doc` annotation includes a fragment
/// identifier (e.g., `design.md#section-7`), the target document contains
/// a heading that matches the fragment.
///
/// # Heading-Slug Matching Algorithm
///
/// Matching is **case-insensitive and punctuation-insensitive**. The fragment
/// and each heading are reduced to the same canonical form before being
/// compared:
///
/// 1. Lowercase the text.
/// 2. Replace `-` with a space.
/// 3. Drop every character that is not alphanumeric or whitespace
///    (so `<`, `>`, `/`, `.`, `(`, `)`, backticks, underscores, etc.
///    are stripped — **no word boundary is inserted in their place**).
/// 4. Collapse runs of whitespace into single spaces.
///
/// Examples (heading → an accepted fragment):
///
/// - `Section One` → `section-one`
/// - `1. Three-Runtime Separation` → `1-three-runtime-separation`
/// - `Validation rules (must pass at index time)` →
///   `validation-rules-must-pass-at-index-time`
/// - `` Pack manifest (`<pack>/SKILL.md`) `` → `pack-manifest-packskillmd`
///   (slashes, dots, and angle brackets collapse to nothing — they do
///   *not* create a hyphen boundary).
/// - `` `allowed_tools` vocabulary (launch) `` →
///   `allowed_tools-vocabulary-launch` (underscores are stripped on both
///   sides, so writing them or omitting them both work).
///
/// **Code:** `W_SEM_DOC_FRAGMENT`
/// **Severity:** Warning
///
/// # Examples
///
/// Valid (assuming headings exist):
/// ```markdown
/// @doc: ../docs/design.md#overview
/// @doc: ./README.md#getting-started
/// ```
///
/// Invalid (`W_SEM_DOC_FRAGMENT`):
/// ```markdown
/// @doc: design.md#nonexistent-section
/// @doc: readme.md#
/// ```
pub struct BrokenDocFragmentRule;

impl BrokenDocFragmentRule {
    /// Create a new broken doc fragment rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Validate a single documentation reference fragment
    ///
    /// Returns a diagnostic if the fragment is invalid, None otherwise.
    /// Skips validation if there's no fragment or if the file doesn't exist.
    fn validate_fragment(&self, doc: &DocRef, ctx: &LintContext) -> Option<LintDiagnostic> {
        // Only validate if there's a fragment
        let fragment = doc.fragment.as_ref()?;

        // Look up where the @doc: line lives in the source file (best-effort).
        // Falls back to (0, 0) if the source can't be read.
        let (src_line, src_col) = Self::locate_doc_annotation(ctx, doc);

        // Empty fragments are invalid
        if fragment.is_empty() {
            return Some(
                LintDiagnostic::warning(
                    self.code(),
                    format!("Empty fragment identifier in doc reference: '{}'", doc.path),
                    ctx.file_path.clone(),
                    src_line,
                    src_col,
                )
                .with_help("Remove the '#' or specify a valid heading fragment"),
            );
        }

        // Resolve the doc path
        let doc_path = Path::new(&doc.path);
        let current_dir = ctx.file_path.parent().unwrap_or(Path::new(""));
        let resolved_path = current_dir.join(doc_path);
        let absolute_path = ctx.config.root_path.join(&resolved_path);

        // If file doesn't exist, ValidDocReferenceRule will catch it
        if !absolute_path.exists() {
            return None;
        }

        // Read the target file
        let Ok(content) = fs::read_to_string(&absolute_path) else {
            return Some(
                LintDiagnostic::warning(
                    self.code(),
                    format!(
                        "Could not read doc file '{}' to validate fragment",
                        doc.path
                    ),
                    ctx.file_path.clone(),
                    src_line,
                    src_col,
                )
                .with_help("Check file permissions or encoding"),
            );
        };

        // Extract all headings from the file
        let headings = Self::extract_headings(&content);

        // Check if any heading matches the fragment
        let normalized_fragment = Self::normalize_fragment(fragment);
        let found = headings
            .iter()
            .any(|h| Self::normalize_heading(h) == normalized_fragment);

        if found {
            None
        } else {
            let help = Self::build_help_text(fragment, &headings);

            Some(
                LintDiagnostic::warning(
                    self.code(),
                    format!("Fragment '{}' not found in '{}'", fragment, doc.path),
                    ctx.file_path.clone(),
                    src_line,
                    src_col,
                )
                .with_help(help),
            )
        }
    }

    /// Best-effort lookup of the source line that authored a `@doc:` reference.
    ///
    /// The `DocRef` struct doesn't carry its origin line, so we reopen the
    /// source markdown and scan for an `@doc:` annotation whose value matches
    /// the reference we're validating. Returns `(line, col)` (1-indexed) when
    /// the source file is readable; falls back to `(0, 0)` otherwise so the
    /// caller can still construct a diagnostic.
    fn locate_doc_annotation(ctx: &LintContext, doc: &DocRef) -> (usize, usize) {
        let absolute_source = ctx.config.root_path.join(&ctx.file_path);
        let Ok(content) = fs::read_to_string(&absolute_source) else {
            return (0, 0);
        };

        let target = doc
            .fragment
            .as_ref()
            .map_or_else(|| doc.path.clone(), |frag| format!("{}#{frag}", doc.path));

        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("@doc:") {
                continue;
            }
            let value = trimmed.trim_start_matches("@doc:").trim();
            if value == target {
                let col = line.find('@').unwrap_or(0).saturating_add(1);
                return (idx.saturating_add(1), col);
            }
        }

        (0, 0)
    }

    /// Build the `help:` text shown alongside a `W_SEM_DOC_FRAGMENT` warning.
    ///
    /// Always lists available headings so the author can pick the right slug.
    /// When a close match is found by normalized-form comparison, it is
    /// surfaced first as a `did you mean:` suggestion.
    fn build_help_text(fragment: &str, headings: &[String]) -> String {
        if headings.is_empty() {
            return "No headings found in the document".to_string();
        }

        let suggestion = Self::closest_heading_slug(fragment, headings);

        let listed: String = headings
            .iter()
            .take(8)
            .map(|h| format!("'{}' (#{})", h, Self::canonical_slug(h)))
            .collect::<Vec<_>>()
            .join(", ");

        match suggestion {
            Some(slug) => format!("did you mean: '{slug}'? Available headings: {listed}"),
            None => format!("Available headings: {listed}"),
        }
    }

    /// Suggest the slug of the heading whose normalized form is closest to
    /// the supplied (mis-typed) fragment.
    ///
    /// We score each heading two ways and take the lowest:
    /// - **Substring score:** 0 if the normalized fragment is a substring of
    ///   the normalized heading (or vice versa). Catches "user typed a prefix
    ///   of the real slug" — by far the most common authoring mistake.
    /// - **Edit-distance score:** Levenshtein distance otherwise.
    ///
    /// We then only suggest if the score is below a loose threshold (half the
    /// shorter string's length, minimum 3) so we stay silent when the user is
    /// just lost instead of mistyping.
    fn closest_heading_slug(fragment: &str, headings: &[String]) -> Option<String> {
        let normalized_fragment = Self::normalize_fragment(fragment);
        if normalized_fragment.is_empty() {
            return None;
        }

        let scored: Vec<(&String, usize)> = headings
            .iter()
            .map(|h| {
                let normalized = Self::normalize_heading(h);
                let score = if normalized.contains(&normalized_fragment)
                    || normalized_fragment.contains(&normalized)
                {
                    0
                } else {
                    levenshtein(&normalized_fragment, &normalized)
                };
                (h, score)
            })
            .collect();

        let (best_heading, best_score) = scored.into_iter().min_by_key(|(_, s)| *s)?;

        let threshold = normalized_fragment.len().clamp(3, 8);
        if best_score <= threshold {
            Some(Self::canonical_slug(best_heading))
        } else {
            None
        }
    }

    /// Produce the canonical (dash-separated) slug recommendation for a
    /// heading. This is the form most users expect to write as a fragment
    /// (lowercase, hyphenated, punctuation stripped without word
    /// boundaries — exactly matching the normalize-and-compare algorithm).
    fn canonical_slug(heading: &str) -> String {
        Self::normalize_for_matching(heading).replace(' ', "-")
    }

    /// Extract all headings from markdown content
    fn extract_headings(content: &str) -> Vec<String> {
        let parser = Parser::new(content);
        let mut headings = Vec::new();
        let mut in_heading = false;
        let mut current_heading = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    // Accept all heading levels (H1-H6)
                    if matches!(
                        level,
                        HeadingLevel::H1
                            | HeadingLevel::H2
                            | HeadingLevel::H3
                            | HeadingLevel::H4
                            | HeadingLevel::H5
                            | HeadingLevel::H6
                    ) {
                        in_heading = true;
                        current_heading.clear();
                    }
                }
                Event::Text(text) if in_heading => {
                    current_heading.push_str(&text);
                }
                Event::Code(code) if in_heading => {
                    current_heading.push_str(&code);
                }
                Event::End(TagEnd::Heading(_)) if in_heading => {
                    if !current_heading.is_empty() {
                        headings.push(current_heading.clone());
                    }
                    in_heading = false;
                }
                _ => {}
            }
        }

        headings
    }

    /// Normalize a fragment for matching
    ///
    /// Converts to lowercase, replaces hyphens with spaces, and removes
    /// non-alphanumeric characters for fuzzy matching with headings.
    fn normalize_fragment(fragment: &str) -> String {
        Self::normalize_for_matching(fragment)
    }

    /// Normalize a heading for matching
    ///
    /// Converts to lowercase, removes non-alphanumeric characters, and
    /// normalizes whitespace for comparison with fragments.
    fn normalize_heading(heading: &str) -> String {
        Self::normalize_for_matching(heading)
    }

    /// Common normalization for both fragments and headings
    ///
    /// This creates a canonical form for matching by:
    /// 1. Converting to lowercase
    /// 2. Replacing hyphens with spaces
    /// 3. Removing non-alphanumeric characters (except spaces)
    /// 4. Normalizing whitespace
    fn normalize_for_matching(s: &str) -> String {
        s.to_lowercase()
            .replace('-', " ")
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Default for BrokenDocFragmentRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard Levenshtein edit distance between two strings.
///
/// Used to score "did you mean?" suggestions against the misspelled fragment.
/// Kept private to this rule because no other rule currently needs it.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();

    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];

    for (i, &ac) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &bc) in b.iter().enumerate() {
            let cost = usize::from(ac != bc);
            curr[j + 1] = (curr[j] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b.len()]
}

impl LintRule for BrokenDocFragmentRule {
    fn code(&self) -> &'static str {
        "W_SEM_DOC_FRAGMENT"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> String {
        "Documentation fragment validation".to_string()
    }

    fn description(&self) -> &'static str {
        "Validates that @doc annotation fragments reference existing headings in the target document"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check file-level doc references
        for doc in &file.metadata.docs {
            if let Some(diag) = self.validate_fragment(doc, ctx) {
                diagnostics.push(diag);
            }
        }

        diagnostics
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check task-level doc references
        for doc in &task.metadata.docs {
            if let Some(diag) = self.validate_fragment(doc, ctx) {
                diagnostics.push(diag);
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{
        dependency::DocRef, task::TaskBuilder, FileMetadata, LashConfig, TaskStatus, TaskTree,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tempfile::TempDir;

    fn make_config_with_root(root: PathBuf) -> LashConfig {
        LashConfig {
            root_path: root,
            index_file: "index.md".to_string(),
            max_depth: 3,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/test.db"),
            custom_annotation_keys: vec![],
        }
    }

    fn make_context_with_config<'a>(
        config: &'a LashConfig,
        current_file: PathBuf,
        files: &'a HashMap<PathBuf, TaskFile>,
    ) -> LintContext<'a> {
        LintContext::new(config, current_file, files)
    }

    #[test]
    fn test_extract_headings() {
        let content = r"# Main Title

Some text here.

## Section One

More text.

### Subsection

#### Deep Section

##### Very Deep

###### Deepest

Regular paragraph.
";
        let headings = BrokenDocFragmentRule::extract_headings(content);
        assert_eq!(headings.len(), 6);
        assert_eq!(headings[0], "Main Title");
        assert_eq!(headings[1], "Section One");
        assert_eq!(headings[2], "Subsection");
        assert_eq!(headings[3], "Deep Section");
        assert_eq!(headings[4], "Very Deep");
        assert_eq!(headings[5], "Deepest");
    }

    #[test]
    fn test_extract_headings_with_code() {
        let content = "# Using `code` in Heading\n\n## Another `example`";
        let headings = BrokenDocFragmentRule::extract_headings(content);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0], "Using code in Heading");
        assert_eq!(headings[1], "Another example");
    }

    #[test]
    fn test_normalize_fragment() {
        assert_eq!(
            BrokenDocFragmentRule::normalize_fragment("section-one"),
            "section one"
        );
        assert_eq!(
            BrokenDocFragmentRule::normalize_fragment("Getting-Started"),
            "getting started"
        );
        assert_eq!(
            BrokenDocFragmentRule::normalize_fragment("API-Reference"),
            "api reference"
        );
        assert_eq!(
            BrokenDocFragmentRule::normalize_fragment("simple"),
            "simple"
        );
    }

    #[test]
    fn test_normalize_heading() {
        assert_eq!(
            BrokenDocFragmentRule::normalize_heading("Section One"),
            "section one"
        );
        assert_eq!(
            BrokenDocFragmentRule::normalize_heading("  Extra   Spaces  "),
            "extra spaces"
        );
        assert_eq!(
            BrokenDocFragmentRule::normalize_heading("UPPERCASE"),
            "uppercase"
        );
    }

    #[test]
    fn test_no_fragment_passes() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a doc file
        fs::write(temp_dir.path().join("design.md"), "# Design\n\nContent").unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();
        let doc = DocRef::new("design.md", None);

        let result = rule.validate_fragment(&doc, &ctx);
        assert!(result.is_none(), "Doc without fragment should pass");
    }

    #[test]
    fn test_empty_fragment_fails() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a doc file
        fs::write(temp_dir.path().join("design.md"), "# Design").unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();
        let doc = DocRef::new("design.md", Some(String::new()));

        let result = rule.validate_fragment(&doc, &ctx);
        assert!(result.is_some(), "Empty fragment should fail");
        let diag = result.unwrap();
        assert_eq!(diag.code, "W_SEM_DOC_FRAGMENT");
        assert!(diag.message.contains("Empty fragment"));
    }

    #[test]
    fn test_valid_fragment_passes() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a doc file with headings
        fs::write(
            temp_dir.path().join("design.md"),
            "# Design\n\n## Getting Started\n\nContent here.\n\n## API Reference\n\nMore content.",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();
        let doc = DocRef::new("design.md", Some("getting-started".to_string()));

        let result = rule.validate_fragment(&doc, &ctx);
        assert!(result.is_none(), "Valid fragment should pass");
    }

    #[test]
    fn test_case_insensitive_matching() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a doc file
        fs::write(
            temp_dir.path().join("design.md"),
            "# Design\n\n## Getting Started\n\nContent.",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();

        // Test various case combinations
        let doc1 = DocRef::new("design.md", Some("GETTING-STARTED".to_string()));
        assert!(rule.validate_fragment(&doc1, &ctx).is_none());

        let doc2 = DocRef::new("design.md", Some("Getting-Started".to_string()));
        assert!(rule.validate_fragment(&doc2, &ctx).is_none());

        let doc3 = DocRef::new("design.md", Some("getting-started".to_string()));
        assert!(rule.validate_fragment(&doc3, &ctx).is_none());
    }

    #[test]
    fn test_missing_heading_fails() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a doc file
        fs::write(
            temp_dir.path().join("design.md"),
            "# Design\n\n## Overview\n\nContent.",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();
        let doc = DocRef::new("design.md", Some("nonexistent-section".to_string()));

        let result = rule.validate_fragment(&doc, &ctx);
        assert!(result.is_some(), "Missing heading should fail");
        let diag = result.unwrap();
        assert_eq!(diag.code, "W_SEM_DOC_FRAGMENT");
        assert!(diag.message.contains("not found"));
        assert!(diag.message.contains("nonexistent-section"));
    }

    #[test]
    fn test_missing_file_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();
        let doc = DocRef::new("missing.md", Some("section".to_string()));

        // Should not produce diagnostic (ValidDocReferenceRule handles this)
        let result = rule.validate_fragment(&doc, &ctx);
        assert!(result.is_none(), "Missing file should be skipped");
    }

    #[test]
    fn test_file_level_doc_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create doc files
        fs::write(
            temp_dir.path().join("valid.md"),
            "# Valid\n\n## Section One\n\nContent.",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("other.md"),
            "# Other\n\n## Different\n\nContent.",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();

        let mut metadata = FileMetadata::default();
        metadata
            .docs
            .push(DocRef::new("valid.md", Some("section-one".to_string()))); // Valid
        metadata
            .docs
            .push(DocRef::new("other.md", Some("missing".to_string()))); // Invalid

        let file = TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata,
            description: None,
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        };

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_SEM_DOC_FRAGMENT");
        assert!(diagnostics[0].message.contains("missing"));
    }

    #[test]
    fn test_task_level_doc_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a doc file
        fs::write(
            temp_dir.path().join("guide.md"),
            "# Guide\n\n## Setup\n\nContent.",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();

        let mut task = TaskBuilder::new("Test task")
            .id("task-1")
            .status(TaskStatus::Open)
            .build()
            .unwrap();

        task.metadata
            .docs
            .push(DocRef::new("guide.md", Some("setup".to_string()))); // Valid
        task.metadata
            .docs
            .push(DocRef::new("guide.md", Some("invalid".to_string()))); // Invalid

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_SEM_DOC_FRAGMENT");
        assert!(diagnostics[0].message.contains("invalid"));
    }

    #[test]
    fn test_relative_path_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create structure:
        // - tasks/feature.md (current file)
        // - docs/design.md (doc reference)
        let tasks_dir = temp_dir.path().join("tasks");
        let docs_dir = temp_dir.path().join("docs");
        fs::create_dir_all(&tasks_dir).unwrap();
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(
            docs_dir.join("design.md"),
            "# Design\n\n## Architecture\n\nContent.",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("tasks/feature.md"), &files);

        let rule = BrokenDocFragmentRule::new();
        let doc = DocRef::new("../docs/design.md", Some("architecture".to_string()));

        let result = rule.validate_fragment(&doc, &ctx);
        assert!(
            result.is_none(),
            "Valid relative path with fragment should pass"
        );
    }

    #[test]
    fn test_heading_levels() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create doc with various heading levels
        fs::write(
            temp_dir.path().join("design.md"),
            r"# H1 Title
## H2 Section
### H3 Subsection
#### H4 Detail
##### H5 Fine
###### H6 Finest
",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();

        // All heading levels should be found
        for (fragment, expected_heading) in [
            ("h1-title", "H1 Title"),
            ("h2-section", "H2 Section"),
            ("h3-subsection", "H3 Subsection"),
            ("h4-detail", "H4 Detail"),
            ("h5-fine", "H5 Fine"),
            ("h6-finest", "H6 Finest"),
        ] {
            let doc = DocRef::new("design.md", Some(fragment.to_string()));
            let result = rule.validate_fragment(&doc, &ctx);
            assert!(
                result.is_none(),
                "Should find heading '{expected_heading}' for fragment '{fragment}'"
            );
        }
    }

    #[test]
    fn test_rule_metadata() {
        let rule = BrokenDocFragmentRule::new();
        assert_eq!(rule.code(), "W_SEM_DOC_FRAGMENT");
        assert_eq!(rule.severity(), Severity::Warning);
        assert_eq!(rule.name(), "Documentation fragment validation");
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_help_shows_available_headings() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        fs::write(
            temp_dir.path().join("design.md"),
            "# Title\n\n## Overview\n\n## API\n\nContent.",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();
        let doc = DocRef::new("design.md", Some("nonexistent".to_string()));

        let result = rule.validate_fragment(&doc, &ctx);
        assert!(result.is_some());
        let diag = result.unwrap();
        // Help should mention available headings
        assert!(diag.help.as_ref().unwrap().contains("Available headings"));
        assert!(diag.help.as_ref().unwrap().contains("Title"));
    }

    #[test]
    fn test_no_headings_in_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a doc file with no headings
        fs::write(
            temp_dir.path().join("plain.md"),
            "Just some plain text without any headings.",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();
        let doc = DocRef::new("plain.md", Some("section".to_string()));

        let result = rule.validate_fragment(&doc, &ctx);
        assert!(result.is_some());
        let diag = result.unwrap();
        assert!(diag.help.as_ref().unwrap().contains("No headings found"));
    }

    #[test]
    fn test_normalize_strips_special_characters() {
        // Parentheses should be stripped
        assert_eq!(
            BrokenDocFragmentRule::normalize_heading("JSON schema (canonical)"),
            "json schema canonical"
        );
        assert_eq!(
            BrokenDocFragmentRule::normalize_fragment("json-schema-canonical"),
            "json schema canonical"
        );

        // Backticks should be stripped
        assert_eq!(
            BrokenDocFragmentRule::normalize_heading("Configuration (`flawd.toml`)"),
            "configuration flawdtoml"
        );
        assert_eq!(
            BrokenDocFragmentRule::normalize_fragment("configuration-flawdtoml"),
            "configuration flawdtoml"
        );

        // Complex heading with parentheses
        assert_eq!(
            BrokenDocFragmentRule::normalize_heading(
                "Core components (Rust crates/modules inside one workspace)"
            ),
            "core components rust cratesmodules inside one workspace"
        );
    }

    #[test]
    fn test_locate_doc_annotation_reports_line() {
        // The diagnostic should point at the actual @doc: line, not :0:0.
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a target doc with a known heading.
        fs::write(temp_dir.path().join("guide.md"), "# Guide\n\n## Setup\n").unwrap();

        // Create a source task file whose @doc: annotation is on a specific line.
        let source_rel = PathBuf::from("tasks.md");
        let source_abs = temp_dir.path().join(&source_rel);
        let source_content = "# Tasks\n\
                              \n\
                              ## Tasks\n\
                              \n\
                              - [ ] Do the thing\n  \
                              @id: thing\n  \
                              @doc: guide.md#nonexistent\n\
                              ";
        fs::write(&source_abs, source_content).unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, source_rel, &files);

        let rule = BrokenDocFragmentRule::new();
        let doc = DocRef::new("guide.md", Some("nonexistent".to_string()));

        let diag = rule.validate_fragment(&doc, &ctx).expect("expected diag");
        // Line 7 in the source file contains the `@doc:` annotation.
        assert_eq!(
            diag.location.line,
            Some(7),
            "should report the @doc: line, got {:?}",
            diag.location.line
        );
        assert!(
            diag.location.column.unwrap_or(0) > 0,
            "should report a non-zero column for the @ sign"
        );
    }

    #[test]
    fn test_locate_doc_annotation_falls_back_when_unreadable() {
        // If the source file isn't readable (e.g., it doesn't exist on disk
        // because we're only working with in-memory test data), the warning
        // still fires with line/col of 0 — never a panic.
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        fs::write(temp_dir.path().join("guide.md"), "# Guide\n\n## Setup\n").unwrap();

        let files = HashMap::new();
        // Note: tasks.md is *not* created on disk.
        let ctx = make_context_with_config(&config, PathBuf::from("tasks.md"), &files);

        let rule = BrokenDocFragmentRule::new();
        let doc = DocRef::new("guide.md", Some("nonexistent".to_string()));

        let diag = rule.validate_fragment(&doc, &ctx).expect("expected diag");
        assert_eq!(diag.location.line, Some(0));
        assert_eq!(diag.location.column, Some(0));
    }

    #[test]
    fn test_help_suggests_close_match() {
        // For a near-miss slug, the help should propose a concrete "did you
        // mean?" alternative.
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        fs::write(
            temp_dir.path().join("design.md"),
            "# Design\n\n## Validation rules (must pass at index time)\n",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();
        // A typo close to the real slug: missing "-must-pass-..."
        let doc = DocRef::new("design.md", Some("validation-rules".to_string()));

        let diag = rule.validate_fragment(&doc, &ctx).expect("expected diag");
        let help = diag.help.as_deref().unwrap_or_default();
        assert!(
            help.contains("did you mean"),
            "help should suggest a close match, got: {help}"
        );
        assert!(
            help.contains("validation-rules-must-pass-at-index-time"),
            "help should suggest the canonical slug, got: {help}"
        );
    }

    #[test]
    fn test_canonical_slug_format() {
        assert_eq!(
            BrokenDocFragmentRule::canonical_slug("Pack manifest (`<pack>/SKILL.md`)"),
            "pack-manifest-packskillmd"
        );
        assert_eq!(
            BrokenDocFragmentRule::canonical_slug("Validation rules (must pass at index time)"),
            "validation-rules-must-pass-at-index-time"
        );
        assert_eq!(
            BrokenDocFragmentRule::canonical_slug("`allowed_tools` vocabulary (launch)"),
            "allowedtools-vocabulary-launch"
        );
    }

    #[test]
    fn test_levenshtein_basic() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("", "abc"), 3);
    }

    #[test]
    fn test_matching_headings_with_special_chars() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a doc file with headings containing special characters
        fs::write(
            temp_dir.path().join("design.md"),
            "# Design\n\n\
             ## JSON schema (canonical)\n\n\
             Some content.\n\n\
             ## Configuration (`flawd.toml`)\n\n\
             More content.\n\n\
             ## Core components (Rust crates/modules inside one workspace)\n\n\
             Even more content.\n",
        )
        .unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = BrokenDocFragmentRule::new();

        // Test matching json-schema-canonical to "JSON schema (canonical)"
        let doc = DocRef::new("design.md", Some("json-schema-canonical".to_string()));
        let result = rule.validate_fragment(&doc, &ctx);
        assert!(
            result.is_none(),
            "json-schema-canonical should match 'JSON schema (canonical)'"
        );

        // Test matching configuration-flawdtoml to "Configuration (`flawd.toml`)"
        let doc = DocRef::new("design.md", Some("configuration-flawdtoml".to_string()));
        let result = rule.validate_fragment(&doc, &ctx);
        assert!(
            result.is_none(),
            "configuration-flawdtoml should match 'Configuration (`flawd.toml`)'"
        );
    }
}
