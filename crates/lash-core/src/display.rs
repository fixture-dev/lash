//! Display formatting utilities for task rendering
//!
//! This module provides functions for formatting task titles and annotations
//! for display in CLI and TUI interfaces.

use std::path::Path;

/// Extract the link path from a Markdown link
///
/// Extracts the path from `[link text](path)` syntax.
/// Returns `None` if no link is found.
///
/// # Examples
///
/// ```
/// use lash_core::display::extract_link_path;
///
/// assert_eq!(extract_link_path("[Core API](core/api.md)"), Some("core/api.md".to_string()));
/// assert_eq!(extract_link_path("Plain text"), None);
/// assert_eq!(extract_link_path("[Link](path/to/file.md#task-id)"), Some("path/to/file.md#task-id".to_string()));
/// ```
#[must_use]
pub fn extract_link_path(text: &str) -> Option<String> {
    extract_link_paths(text).into_iter().next()
}

/// Extract every Markdown link destination on a line, in order
///
/// Unlike [`extract_link_path`], this collects all links rather than just the
/// first, and each destination ends at the parenthesis that closes *its* link
/// rather than at the last parenthesis on the line. Parentheses nested inside a
/// destination are balanced; angle-bracketed destinations (`[a](<b c.md>)`) are
/// unwrapped.
///
/// # Examples
///
/// ```
/// use lash_core::display::extract_link_paths;
///
/// assert_eq!(
///     extract_link_paths("[A](a.md) and [B](b.md) (see also)"),
///     vec!["a.md".to_string(), "b.md".to_string()]
/// );
/// assert_eq!(extract_link_paths("Plain text"), Vec::<String>::new());
/// ```
#[must_use]
pub fn extract_link_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut cursor = 0;

    while let Some(offset) = text[cursor..].find("](") {
        let close_bracket = cursor + offset;
        let dest_start = close_bracket + 2; // Skip "]("

        // A destination only counts as a link if some `[` opens it.
        if !text[cursor..close_bracket].contains('[') {
            cursor = dest_start;
            continue;
        }

        let Some(dest_end) = find_dest_end(text, dest_start) else {
            break;
        };

        let dest = text[dest_start..dest_end].trim();
        let dest = dest
            .strip_prefix('<')
            .and_then(|inner| inner.strip_suffix('>'))
            .unwrap_or(dest);
        if !dest.is_empty() {
            paths.push(dest.to_string());
        }

        cursor = dest_end + 1; // Skip the closing ')'
    }

    paths
}

/// Find the parenthesis that closes a link destination starting at `start`
///
/// Parentheses inside the destination are balanced, so `[a](f(1).md)` ends at
/// the final `)` rather than the one after `1`.
fn find_dest_end(text: &str, start: usize) -> Option<usize> {
    let mut depth = 0usize;

    for (offset, ch) in text[start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return Some(start + offset);
                }
                depth -= 1;
            }
            _ => {}
        }
    }

    None
}

/// Extract link text from Markdown links
///
/// Converts `[link text](path)` to just `link text`.
/// Returns the original text if no link is found.
///
/// # Examples
///
/// ```
/// use lash_core::display::extract_link_text;
///
/// assert_eq!(extract_link_text("[Core API](core/api.md)"), "Core API");
/// assert_eq!(extract_link_text("Plain text"), "Plain text");
/// ```
#[must_use]
pub fn extract_link_text(text: &str) -> String {
    // Try to match [link text](path) pattern
    if let (Some(open_bracket), Some(close_bracket)) = (text.find('['), text.find("](")) {
        if open_bracket < close_bracket {
            if let Some(close_paren) = text.rfind(')') {
                if close_bracket < close_paren {
                    // Extract the link text between [ and ](
                    let link_text = &text[open_bracket + 1..close_bracket];
                    // Return just the link text, preserving any prefix
                    let prefix = &text[..open_bracket];
                    // Check for suffix after the link
                    let suffix = if close_paren + 1 < text.len() {
                        &text[close_paren + 1..]
                    } else {
                        ""
                    };
                    return format!("{prefix}{link_text}{suffix}");
                }
            }
        }
    }

    // Return original if no link pattern found
    text.to_string()
}

/// Format index task annotations for display
///
/// Strips `@id:` annotations and converts `@labels:` to hashtag format.
///
/// # Examples
///
/// ```
/// use lash_core::display::format_index_annotations;
///
/// assert_eq!(
///     format_index_annotations("Alpha @id:`milestone.alpha` @labels:`milestone, p0`"),
///     "Alpha #milestone #p0"
/// );
/// assert_eq!(format_index_annotations("Task @id:`some.id`"), "Task");
/// assert_eq!(format_index_annotations("Plain Task"), "Plain Task");
/// ```
#[must_use]
pub fn format_index_annotations(text: &str) -> String {
    let mut result = String::new();
    let mut remaining = text;

    // Process the text, stripping @id: and converting @labels:
    while !remaining.is_empty() {
        // Check for @id: annotation - strip it completely
        if remaining.starts_with("@id:") {
            // Find the end of the backtick-wrapped value
            if let Some(backtick_start) = remaining.find('`') {
                if let Some(backtick_end) = remaining[backtick_start + 1..].find('`') {
                    // Skip past the entire @id:`value` including trailing space
                    let end_pos = backtick_start + 1 + backtick_end + 1;
                    remaining = remaining[end_pos..].trim_start();
                    continue;
                }
            }
            // If no backticks found, skip to end
            break;
        }

        // Check for @labels: annotation - convert to hashtags
        if remaining.starts_with("@labels:") {
            if let Some(backtick_start) = remaining.find('`') {
                if let Some(backtick_end) = remaining[backtick_start + 1..].find('`') {
                    let labels_content =
                        &remaining[backtick_start + 1..backtick_start + 1 + backtick_end];
                    // Convert comma-separated labels to hashtags
                    for label in labels_content.split(',') {
                        let label = label.trim();
                        if !label.is_empty() {
                            result.push('#');
                            result.push_str(label);
                            result.push(' ');
                        }
                    }
                    // Move past the @labels:`value`
                    let end_pos = backtick_start + 1 + backtick_end + 1;
                    remaining = remaining[end_pos..].trim_start();
                    continue;
                }
            }
            // If no backticks found, skip to end
            break;
        }

        // Find the next @ or end of string
        if let Some(next_at) = remaining[1..].find('@') {
            // Add text up to next annotation
            result.push_str(&remaining[..=next_at]);
            remaining = &remaining[next_at + 1..];
        } else {
            // No more annotations, add rest of text
            result.push_str(remaining);
            break;
        }
    }

    result.trim().to_string()
}

/// Format an index task title for display
///
/// Combines link text extraction and annotation formatting.
/// This is the main entry point for formatting index file task titles.
///
/// # Examples
///
/// ```
/// use lash_core::display::format_index_title;
///
/// assert_eq!(
///     format_index_title("[Alpha](milestones/alpha.md) @id:`milestone.alpha` @labels:`milestone, p0`"),
///     "Alpha #milestone #p0"
/// );
/// ```
#[must_use]
pub fn format_index_title(title: &str) -> String {
    let extracted = extract_link_text(title);
    format_index_annotations(&extracted)
}

/// Check if a path represents an index file
///
/// Index files are named `lash.index.md` or `index.lash.md`.
///
/// # Examples
///
/// ```
/// use lash_core::display::is_index_file;
/// use std::path::Path;
///
/// assert!(is_index_file(Path::new("project/lash.index.md")));
/// assert!(is_index_file(Path::new("index.lash.md")));
/// assert!(!is_index_file(Path::new("tasks/feature.md")));
/// ```
#[must_use]
pub fn is_index_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|f| f.to_str())
        .is_some_and(|filename| filename == "lash.index.md" || filename == "index.lash.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_link_text() {
        // Standard markdown link
        assert_eq!(extract_link_text("[Core API](core/api.md)"), "Core API");

        // Link with prefix
        assert_eq!(
            extract_link_text("Prefix [Link Text](path.md)"),
            "Prefix Link Text"
        );

        // Link with suffix
        assert_eq!(extract_link_text("[Link](path.md) suffix"), "Link suffix");

        // No link - returns original
        assert_eq!(extract_link_text("Plain text"), "Plain text");

        // Backtick path (no transformation - not a link)
        assert_eq!(extract_link_text("`path/file.md`"), "`path/file.md`");
    }

    #[test]
    fn test_extract_link_paths() {
        // Every link on the line is collected, in order
        assert_eq!(
            extract_link_paths("[A](a.md) and [B](b.md)"),
            vec!["a.md".to_string(), "b.md".to_string()]
        );

        // A later ')' does not extend the destination (GitHub issue #60)
        assert_eq!(
            extract_link_paths("[A](a.md) (see also)"),
            vec!["a.md".to_string()]
        );

        // Parentheses inside the destination are balanced
        assert_eq!(
            extract_link_paths("[Copy](a(1).md)"),
            vec!["a(1).md".to_string()]
        );

        // Angle brackets are unwrapped
        assert_eq!(
            extract_link_paths("[A](<my file.md>)"),
            vec!["my file.md".to_string()]
        );

        // Empty destinations and non-links are skipped
        assert!(extract_link_paths("[A]()").is_empty());
        assert!(extract_link_paths("Plain text").is_empty());
        assert!(extract_link_paths("not a link](a.md)").is_empty());

        // Unterminated destination
        assert!(extract_link_paths("[A](a.md").is_empty());
    }

    #[test]
    fn test_extract_link_path_first_link_only() {
        assert_eq!(
            extract_link_path("[A](a.md) and [B](b.md)"),
            Some("a.md".to_string())
        );
        assert_eq!(
            extract_link_path("[A](a.md) (see also)"),
            Some("a.md".to_string())
        );
        assert_eq!(extract_link_path("Plain text"), None);
    }

    #[test]
    fn test_format_index_annotations() {
        // Full annotation: strip @id and convert @labels to hashtags
        assert_eq!(
            format_index_annotations("Alpha @id:`milestone.alpha` @labels:`milestone, p0`"),
            "Alpha #milestone #p0"
        );

        // Just @id annotation - strip completely
        assert_eq!(format_index_annotations("Task @id:`some.id`"), "Task");

        // Just @labels annotation - convert to hashtags
        assert_eq!(
            format_index_annotations("Task @labels:`foo, bar, baz`"),
            "Task #foo #bar #baz"
        );

        // No annotations - return as-is
        assert_eq!(format_index_annotations("Plain Task"), "Plain Task");

        // With existing hashtags - preserve them
        assert_eq!(
            format_index_annotations("Task #existing @labels:`new`"),
            "Task #existing #new"
        );
    }

    #[test]
    fn test_format_index_title() {
        // Full index task title
        assert_eq!(
            format_index_title(
                "[Alpha](milestones/alpha.md) @id:`milestone.alpha` @labels:`milestone, p0`"
            ),
            "Alpha #milestone #p0"
        );

        // Just a link, no annotations
        assert_eq!(
            format_index_title("[Physics & Collision](systems/physics.md)"),
            "Physics & Collision"
        );

        // Plain text
        assert_eq!(format_index_title("Plain Task"), "Plain Task");
    }
}
