//! Header block parsing
//!
//! This module handles parsing of the header block in Lash task files.
//! The header consists of:
//! - H1 title (required)
//! - Metadata annotations (@id, @labels, etc.)
//! - Optional overview/description text
//! - Boundary marker (## Tasks section)
//!
//! The parser uses pulldown-cmark to find headings and then processes the
//! content between them to extract annotations and overview text.
//!
//! # Example Header
//!
//! ```markdown
//! # Project Tasks
//!
//! @id: project-abc
//! @labels: backend, api
//! @owner: alice
//! @created: 2025-01-15
//!
//! This is the project overview text that describes what this file is about.
//! It can span multiple paragraphs.
//!
//! ## Tasks
//!
//! - [ ] First task
//! ```
//!
//! # Graceful Degradation
//!
//! The parser handles missing or malformed headers gracefully:
//! - Missing H1: Synthesize title from filename
//! - Missing Tasks section: Treat entire file as tasks
//! - Missing annotations: Use defaults
//! - Multiple H1s: Use first, emit warning

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;

use super::annotations::{parse_annotation_block, AnnotationBlock};
use super::{ParseContext, ParsedHeader, Section};
use lash_types::{Diagnostic, Location, Severity};

/// Parse the header block from Markdown content
///
/// This function extracts:
/// - H1 title
/// - File-level annotations
/// - Optional overview text
/// - Detects the ## Tasks section boundary
///
/// # Arguments
///
/// * `content` - Raw Markdown content
/// * `ctx` - Parse context for error reporting
///
/// # Returns
///
/// Returns `ParsedHeader` with extracted components. Emits warnings/errors
/// to the context for issues like missing H1 or multiple H1s.
///
/// # Errors
///
/// This function adds diagnostics to the context but does not fail - it
/// always returns a header (potentially synthesized from filename if needed).
pub fn parse_header(content: &str, ctx: &mut ParseContext) -> ParsedHeader {
    let parser = Parser::new(content);
    let lines: Vec<&str> = content.lines().collect();

    // Find H1 title
    let (title, h1_line_num) = find_h1_title(parser, &lines, ctx);

    // Find the Description section (## Description)
    let description_section_line = find_description_section(content);

    // Find the Tasks section (## Tasks)
    let tasks_section_line = find_tasks_section(content, ctx);

    // Determine the end of the header section (either Description or Tasks, whichever comes first)
    let header_end_line = match (description_section_line, tasks_section_line) {
        (Some(desc), Some(tasks)) => Some(desc.min(tasks)),
        (Some(desc), None) => Some(desc),
        (None, Some(tasks)) => Some(tasks),
        (None, None) => None,
    };

    // Extract annotations and overview between H1 and first H2 section
    let (annotations, overview) = if let Some(h1_line) = h1_line_num {
        let start_line = h1_line + 1; // Line after H1
        let end_line = header_end_line.unwrap_or(lines.len());

        extract_annotations_and_overview(&lines, start_line, end_line, ctx)
    } else {
        // No H1 found, look for annotations at start of file
        let end_line = header_end_line.unwrap_or(lines.len());
        extract_annotations_and_overview(&lines, 0, end_line, ctx)
    };

    // Extract description section if present
    let (description, description_agent_notes) = if let Some(desc_line) = description_section_line {
        // Check that Description comes before Tasks
        if let Some(tasks_line) = tasks_section_line {
            if desc_line > tasks_line {
                ctx.add_diagnostic(Diagnostic {
                    severity: Severity::Error,
                    code: "E_PARSE_DESCRIPTION_AFTER_TASKS",
                    message: "Description section must come before Tasks section".to_string(),
                    location: Some(Location::new(ctx.file_path.to_path_buf(), desc_line + 1, 1)),
                    snippet: Some(lines.get(desc_line).copied().unwrap_or("").to_string()),
                    help: Some(
                        "Move the ## Description section before the ## Tasks section".to_string(),
                    ),
                    labels: None,
                    recovery_command: None,
                    fix_steps: None,
                    explanation: None,
                    docs_url: None,
                });
                (None, Vec::new())
            } else {
                // Description is correctly placed before Tasks
                let desc_end_line = tasks_line;
                let (desc_text, agent_notes) =
                    extract_description_and_agent_notes(&lines, desc_line, desc_end_line);
                if desc_text.is_empty() {
                    (None, agent_notes)
                } else {
                    (Some(desc_text), agent_notes)
                }
            }
        } else {
            // No Tasks section, extract until end of file or next ## heading
            let desc_end_line = lines
                .iter()
                .enumerate()
                .skip(desc_line + 1)
                .find(|(_, line)| {
                    let trimmed = line.trim();
                    trimmed.starts_with("## ")
                })
                .map_or(lines.len(), |(idx, _)| idx);

            let (desc_text, agent_notes) =
                extract_description_and_agent_notes(&lines, desc_line, desc_end_line);
            if desc_text.is_empty() {
                (None, agent_notes)
            } else {
                (Some(desc_text), agent_notes)
            }
        }
    } else {
        (None, Vec::new())
    };

    // Update parse context section state
    if tasks_section_line.is_some() {
        ctx.current_section = Section::Tasks;
    }

    ParsedHeader {
        title,
        annotations,
        overview,
        description,
        description_agent_notes,
    }
}

/// Find the first H1 heading in the document
///
/// Returns the title text and the line number where it appears.
/// Emits warnings for multiple H1s or missing H1.
fn find_h1_title(
    parser: Parser,
    lines: &[&str],
    ctx: &mut ParseContext,
) -> (String, Option<usize>) {
    let mut title = String::new();
    let mut in_h1 = false;
    let mut h1_count = 0;
    let mut h1_line_num: Option<usize> = None;

    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                in_h1 = true;
                h1_count += 1;

                if h1_count == 1 {
                    // Find line number where this H1 appears
                    h1_line_num = find_line_with_heading(lines, 1);
                } else if h1_count == 2 {
                    // Emit warning for multiple H1s
                    ctx.add_diagnostic(Diagnostic {
                        severity: Severity::Warning,
                        code: "W_PARSE_MULTIPLE_H1",
                        message: "Multiple H1 headings found, using first".to_string(),
                        location: None,
                        snippet: None,
                        help: Some("Lash files should have exactly one H1 heading".to_string()),
                        labels: None,
                        recovery_command: None,
                        fix_steps: None,
                        explanation: None,
                        docs_url: None,
                    });
                }
            }
            Event::Text(text) if in_h1 && h1_count == 1 => {
                title.push_str(&text);
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) if h1_count == 1 => {
                in_h1 = false;
            }
            _ => {}
        }
    }

    // If no H1 found, synthesize from filename
    if title.is_empty() {
        let synthesized_title = synthesize_title_from_path(ctx.file_path);
        ctx.add_diagnostic(Diagnostic {
            severity: Severity::Warning,
            code: "W_PARSE_MISSING_H1",
            message: "No H1 heading found, synthesized from filename".to_string(),
            location: Some(Location::new(ctx.file_path.to_path_buf(), 1, 1)),
            snippet: None,
            help: Some(format!(
                "Add an H1 heading at the start: # {synthesized_title}"
            )),
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        });
        (synthesized_title, None)
    } else {
        (title, h1_line_num)
    }
}

/// Find the ## Tasks section heading
///
/// Returns the line number where "## Tasks" appears (0-indexed).
/// Case-insensitive comparison.
fn find_tasks_section(content: &str, ctx: &mut ParseContext) -> Option<usize> {
    let parser = Parser::new(content);
    let lines: Vec<&str> = content.lines().collect();

    let mut in_h2 = false;
    let mut h2_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            }) => {
                in_h2 = true;
                h2_text.clear();
            }
            Event::Text(text) if in_h2 => {
                h2_text.push_str(&text);
            }
            Event::End(TagEnd::Heading(HeadingLevel::H2)) => {
                if h2_text.trim().eq_ignore_ascii_case("tasks") {
                    // Found it! Find the line number
                    return find_heading_with_text(&lines, 2, "tasks");
                }
                in_h2 = false;
            }
            _ => {}
        }
    }

    // No Tasks section found - emit warning
    ctx.add_diagnostic(Diagnostic {
        severity: Severity::Warning,
        code: "W_PARSE_MISSING_TASKS_SECTION",
        message: "No '## Tasks' section found, treating entire file as tasks".to_string(),
        location: None,
        snippet: None,
        help: Some("Add a '## Tasks' section to separate metadata from tasks".to_string()),
        labels: None,
        recovery_command: None,
        fix_steps: None,
        explanation: None,
        docs_url: None,
    });

    None
}

/// Find the ## Description section heading
///
/// Returns the line number where "## Description" appears (0-indexed).
/// Case-insensitive comparison.
fn find_description_section(content: &str) -> Option<usize> {
    let lines: Vec<&str> = content.lines().collect();
    find_heading_with_text(&lines, 2, "description")
}

/// Extract description text and agent notes from Description section
///
/// Finds the content between ## Description and the next ## heading.
/// Extracts inline `@agent-note:` annotations from the description text.
///
/// Returns `(description_text, agent_notes_vec)`
fn extract_description_and_agent_notes(
    lines: &[&str],
    desc_start_line: usize,
    desc_end_line: usize,
) -> (String, Vec<String>) {
    let mut description_lines = Vec::new();
    let mut agent_notes = Vec::new();

    // Regex pattern for @agent-note: inline annotations
    let agent_note_pattern = regex::Regex::new(r"@agent-note:\s*(.+)").unwrap();

    // Calculate number of lines to take, ensuring no underflow
    let num_lines = if desc_end_line > desc_start_line + 1 {
        desc_end_line - desc_start_line - 1
    } else {
        0
    };

    for line in lines.iter().skip(desc_start_line + 1).take(num_lines) {
        // Check for @agent-note: pattern
        if let Some(captures) = agent_note_pattern.captures(line) {
            if let Some(note) = captures.get(1) {
                agent_notes.push(note.as_str().trim().to_string());
            }
        }

        // Add line to description (including agent notes as they appear)
        description_lines.push(*line);
    }

    let description = description_lines.join("\n").trim().to_string();
    let description = if description.is_empty() {
        String::new()
    } else {
        description
    };

    (description, agent_notes)
}

/// Find the line number where a heading of the given level appears
///
/// This scans the lines looking for the heading marker.
fn find_line_with_heading(lines: &[&str], level: usize) -> Option<usize> {
    let marker = "#".repeat(level);
    let marker_with_space = format!("{marker} ");

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(&marker_with_space) || trimmed == marker {
            return Some(idx);
        }
    }
    None
}

/// Find the line number where a heading with specific text appears
///
/// This scans the lines looking for a heading with the given level and text.
/// Case-insensitive comparison.
fn find_heading_with_text(lines: &[&str], level: usize, text: &str) -> Option<usize> {
    let marker = "#".repeat(level);
    let marker_with_space = format!("{marker} ");

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(&marker_with_space) {
            let heading_text = trimmed[marker_with_space.len()..].trim();
            if heading_text.eq_ignore_ascii_case(text) {
                return Some(idx);
            }
        }
    }
    None
}

/// Extract annotations and overview text from the header section
///
/// Processes lines between H1 and ## Tasks:
/// - Lines starting with @ are annotations
/// - Other non-blank lines are overview text
///
/// Returns `(annotations, overview_text)`
fn extract_annotations_and_overview(
    lines: &[&str],
    start_line: usize,
    end_line: usize,
    ctx: &mut ParseContext,
) -> (AnnotationBlock, Option<String>) {
    let mut annotation_lines = Vec::new();
    let mut overview_lines = Vec::new();
    let mut in_annotations = false;

    for line in lines.iter().skip(start_line).take(end_line - start_line) {
        let trimmed = line.trim();

        // Skip blank lines
        if trimmed.is_empty() {
            continue;
        }

        // New annotation line
        if trimmed.starts_with('@') {
            annotation_lines.push(*line);
            in_annotations = true;
        } else if in_annotations && line.starts_with(' ') {
            // Continuation line (indented) for multiline annotation
            annotation_lines.push(*line);
        } else {
            // Regular line - either overview or end of annotations
            in_annotations = false;
            overview_lines.push(*line);
        }
    }

    // Parse annotations
    let annotations = if annotation_lines.is_empty() {
        AnnotationBlock::new()
    } else {
        match parse_annotation_block(annotation_lines.into_iter(), Some(ctx.config)) {
            Ok(block) => block,
            Err(e) => {
                ctx.add_error(&e);
                AnnotationBlock::new()
            }
        }
    };

    // Process overview text
    let overview = if overview_lines.is_empty() {
        None
    } else {
        let text = overview_lines.join("\n").trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    };

    (annotations, overview)
}

/// Synthesize a title from the file path
///
/// Converts the filename (without extension) into a human-readable title.
/// Example: "my-tasks.md" -> "My Tasks"
fn synthesize_title_from_path(path: &Path) -> String {
    path.file_stem().and_then(|s| s.to_str()).map_or_else(
        || "Untitled".to_string(),
        |s| {
            s.replace(['-', '_'], " ")
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        },
    )
}

/// Parse optional references section
///
/// Looks for "## References" section after Tasks and extracts its content.
///
/// # Arguments
///
/// * `content` - Full file content
/// * `tasks_section_line` - Line number where ## Tasks appears
///
/// # Returns
///
/// Returns the raw Markdown content of the References section if found.
#[must_use]
pub fn parse_references_section(
    content: &str,
    tasks_section_line: Option<usize>,
) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();

    // Find ## References line directly by scanning
    let refs_line = find_heading_with_text(&lines, 2, "references")?;

    // Make sure References comes after Tasks
    if let Some(tasks_line) = tasks_section_line {
        if refs_line <= tasks_line {
            return None; // References must come after Tasks
        }
    }

    // Extract everything from References to end of file
    let content_start = refs_line + 1;
    if content_start < lines.len() {
        let refs_content = lines[content_start..].join("\n").trim().to_string();
        return if refs_content.is_empty() {
            None
        } else {
            Some(refs_content)
        };
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::LashConfig;

    // Helper struct to manage test context lifetime
    struct TestContext {
        config: LashConfig,
        path: std::path::PathBuf,
    }

    impl TestContext {
        fn new(path: &str) -> Self {
            Self {
                config: LashConfig::default(),
                path: std::path::PathBuf::from(path),
            }
        }

        fn context(&self) -> ParseContext<'_> {
            ParseContext::new(&self.path, &self.config)
        }
    }

    // ==================== H1 Title Parsing Tests ====================

    #[test]
    fn test_parse_header_with_h1() {
        let content = "# My Tasks\n\n## Tasks\n\n- [ ] Task 1";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert_eq!(header.title, "My Tasks");
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_parse_header_with_complex_h1() {
        let content = "# Project: Backend API Tasks\n\n## Tasks";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert_eq!(header.title, "Project: Backend API Tasks");
    }

    #[test]
    fn test_parse_header_missing_h1_synthesizes_from_filename() {
        let content = "## Tasks\n\n- [ ] Task 1";
        let test_ctx = TestContext::new("my-project-tasks.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert_eq!(header.title, "My Project Tasks");
        assert!(ctx
            .diagnostics
            .iter()
            .any(|d| d.code == "W_PARSE_MISSING_H1"));
    }

    #[test]
    fn test_parse_header_multiple_h1s_uses_first() {
        let content = "# First Title\n\n# Second Title\n\n## Tasks";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert_eq!(header.title, "First Title");
        assert!(ctx
            .diagnostics
            .iter()
            .any(|d| d.code == "W_PARSE_MULTIPLE_H1"));
    }

    // ==================== Annotation Extraction Tests ====================

    #[test]
    fn test_parse_header_with_annotations() {
        let content = r"# My Tasks

@id: project-1
@owner: alice
@labels: backend, api

## Tasks";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert_eq!(header.annotations.get_single("id"), Some("project-1"));
        assert_eq!(header.annotations.get_single("owner"), Some("alice"));
        assert_eq!(
            header.annotations.get_single("labels"),
            Some("backend, api")
        );
    }

    #[test]
    fn test_parse_header_with_multiline_annotation() {
        let content = r"# Tasks

@agent-note: This is a long note
  that continues on the next line
  and another line

## Tasks";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        let note = header.annotations.get_single("agent-note").unwrap();
        assert!(note.contains("This is a long note"));
        assert!(note.contains("that continues"));
    }

    #[test]
    fn test_parse_header_no_annotations() {
        let content = "# Tasks\n\n## Tasks";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert!(header.annotations.is_empty());
    }

    // ==================== Overview Section Tests ====================

    #[test]
    fn test_parse_header_with_overview() {
        let content = r"# Project Tasks

@id: project-1

This is the project overview.
It describes what this file is about.

## Tasks";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert!(header.overview.is_some());
        let overview = header.overview.unwrap();
        assert!(overview.contains("project overview"));
        assert!(overview.contains("what this file is about"));
    }

    #[test]
    fn test_parse_header_no_overview() {
        let content = r"# Tasks

@id: project-1

## Tasks";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert!(header.overview.is_none());
    }

    #[test]
    fn test_parse_header_overview_without_annotations() {
        let content = r"# Tasks

This is just an overview without any annotations.

## Tasks";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert!(header.overview.is_some());
        assert!(header.annotations.is_empty());
    }

    // ==================== Tasks Section Detection Tests ====================

    #[test]
    fn test_parse_header_finds_tasks_section() {
        let content = "# Title\n\n## Tasks\n\n- [ ] Task";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        parse_header(content, &mut ctx);

        assert_eq!(ctx.current_section, Section::Tasks);
    }

    #[test]
    fn test_parse_header_tasks_section_case_insensitive() {
        let content = "# Title\n\n## tasks\n\n- [ ] Task";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        parse_header(content, &mut ctx);

        assert_eq!(ctx.current_section, Section::Tasks);
    }

    #[test]
    fn test_parse_header_missing_tasks_section() {
        let content = "# Title\n\n@id: test\n\nSome content";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        parse_header(content, &mut ctx);

        assert!(ctx
            .diagnostics
            .iter()
            .any(|d| d.code == "W_PARSE_MISSING_TASKS_SECTION"));
    }

    // ==================== References Section Tests ====================

    #[test]
    fn test_parse_references_section_found() {
        let content = r"# Title

## Tasks

- [ ] Task 1

## References

- Link to [documentation](https://example.com)
- Related: project-2.md";

        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let tasks_line = find_tasks_section(content, &mut ctx);
        let refs = parse_references_section(content, tasks_line);

        assert!(refs.is_some());
        let refs_content = refs.unwrap();
        assert!(refs_content.contains("documentation"));
        assert!(refs_content.contains("project-2.md"));
    }

    #[test]
    fn test_parse_references_section_not_found() {
        let content = "# Title\n\n## Tasks\n\n- [ ] Task";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let tasks_line = find_tasks_section(content, &mut ctx);
        let refs = parse_references_section(content, tasks_line);

        assert!(refs.is_none());
    }

    #[test]
    fn test_parse_references_section_case_insensitive() {
        let content = "# Title\n\n## Tasks\n\n- [ ] Task\n\n## references\n\nSome refs";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let tasks_line = find_tasks_section(content, &mut ctx);
        let refs = parse_references_section(content, tasks_line);

        assert!(refs.is_some());
    }

    // ==================== Malformed Header Handling Tests ====================

    #[test]
    fn test_parse_header_minimal_valid() {
        let content = "# Title\n\n## Tasks";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert_eq!(header.title, "Title");
        assert!(header.annotations.is_empty());
        assert!(header.overview.is_none());
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_parse_header_only_h1() {
        let content = "# Title\n\nSome content";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert_eq!(header.title, "Title");
        // Should emit warning about missing Tasks section
        assert!(ctx
            .diagnostics
            .iter()
            .any(|d| d.code == "W_PARSE_MISSING_TASKS_SECTION"));
    }

    #[test]
    fn test_parse_header_no_h1_no_tasks() {
        let content = "Just some random content";
        let test_ctx = TestContext::new("random.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert_eq!(header.title, "Random");
        assert!(ctx
            .diagnostics
            .iter()
            .any(|d| d.code == "W_PARSE_MISSING_H1"));
        assert!(ctx
            .diagnostics
            .iter()
            .any(|d| d.code == "W_PARSE_MISSING_TASKS_SECTION"));
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_synthesize_title_from_path() {
        assert_eq!(
            synthesize_title_from_path(Path::new("my-tasks.md")),
            "My Tasks"
        );
        assert_eq!(
            synthesize_title_from_path(Path::new("backend_api.md")),
            "Backend Api"
        );
        assert_eq!(synthesize_title_from_path(Path::new("simple.md")), "Simple");
        assert_eq!(
            synthesize_title_from_path(Path::new("UPPERCASE.md")),
            "UPPERCASE"
        );
    }

    #[test]
    fn test_find_line_with_heading() {
        let lines = vec!["# Title", "", "## Section", "content"];
        assert_eq!(find_line_with_heading(&lines, 1), Some(0));
        assert_eq!(find_line_with_heading(&lines, 2), Some(2));
        assert_eq!(find_line_with_heading(&lines, 3), None);
    }

    // ==================== Complete Header Examples ====================

    #[test]
    fn test_parse_complete_header() {
        let content = r"# Backend API Tasks

@id: backend-api
@owner: alice
@labels: backend, api, database
@status: in-progress
@created: 2025-01-15

This file tracks all backend API development tasks.
Focus areas include authentication, data models, and endpoints.

## Tasks

- [ ] Design API schema
- [ ] Implement authentication";

        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert_eq!(header.title, "Backend API Tasks");
        assert_eq!(header.annotations.get_single("id"), Some("backend-api"));
        assert_eq!(header.annotations.get_single("owner"), Some("alice"));
        assert!(header.overview.is_some());
        let overview = header.overview.unwrap();
        assert!(overview.contains("backend API development"));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_parse_header_with_all_sections() {
        let content = r"# Project Tasks

@id: project-1
@labels: important, urgent

Overview of the project goes here.

## Tasks

- [ ] First task
- [ ] Second task

## References

- See also: related-project.md
- Documentation: https://example.com";

        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        let tasks_line = find_tasks_section(content, &mut ctx);
        let refs = parse_references_section(content, tasks_line);

        assert_eq!(header.title, "Project Tasks");
        assert!(header.overview.is_some());
        assert!(refs.is_some());
    }

    #[test]
    fn test_parse_header_graceful_degradation() {
        // File with no clear structure - should still parse
        let content = "Some content\n@id: test\nMore content";
        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        // Should synthesize title and find annotation
        assert_eq!(header.title, "Test");
        assert_eq!(header.annotations.get_single("id"), Some("test"));
    }

    // ==================== Description Section Tests ====================

    #[test]
    fn test_parse_header_with_description() {
        let content = r"# Project Tasks

@id: project-1

## Description

This is the project description. It provides context for both humans and agents.

## Tasks

- [ ] First task";

        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert_eq!(header.title, "Project Tasks");
        assert!(header.description.is_some());
        let desc = header.description.unwrap();
        assert!(desc.contains("project description"));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_parse_header_with_description_and_agent_notes() {
        let content = r"# Project Tasks

@id: project-1

## Description

This is the project description. @agent-note: This is important for context.
It provides context for both humans and agents. @agent-note: Focus on the API design.

## Tasks

- [ ] First task";

        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert!(header.description.is_some());
        assert_eq!(header.description_agent_notes.len(), 2);
        assert!(header.description_agent_notes[0].contains("important for context"));
        assert!(header.description_agent_notes[1].contains("Focus on the API design"));
    }

    #[test]
    fn test_parse_header_description_without_tasks() {
        let content = r"# Project Tasks

@id: project-1

## Description

This file has a description but no tasks section yet.";

        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert!(header.description.is_some());
        assert!(header.description.unwrap().contains("no tasks section"));
    }

    #[test]
    fn test_parse_header_empty_description() {
        let content = r"# Project Tasks

@id: project-1

## Description

## Tasks

- [ ] First task";

        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        // Empty description section should return None
        assert!(header.description.is_none());
    }

    #[test]
    fn test_parse_header_description_after_tasks_error() {
        let content = r"# Project Tasks

@id: project-1

## Tasks

- [ ] First task

## Description

This should error because description comes after tasks.";

        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        // Should have an error diagnostic
        assert!(ctx.has_errors());
        assert!(ctx
            .diagnostics
            .iter()
            .any(|d| d.code == "E_PARSE_DESCRIPTION_AFTER_TASKS"));
        // Description should not be extracted
        assert!(header.description.is_none());
    }

    #[test]
    fn test_parse_header_no_description() {
        let content = r"# Project Tasks

@id: project-1

## Tasks

- [ ] First task";

        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert!(header.description.is_none());
        assert!(header.description_agent_notes.is_empty());
    }

    #[test]
    fn test_parse_header_description_with_multiple_paragraphs() {
        let content = r"# Project Tasks

@id: project-1

## Description

This is the first paragraph of the description.

This is the second paragraph with more details.

And a third paragraph for good measure.

## Tasks

- [ ] First task";

        let test_ctx = TestContext::new("test.md");
        let mut ctx = test_ctx.context();
        let header = parse_header(content, &mut ctx);

        assert!(header.description.is_some());
        let desc = header.description.unwrap();
        assert!(desc.contains("first paragraph"));
        assert!(desc.contains("second paragraph"));
        assert!(desc.contains("third paragraph"));
    }
}
