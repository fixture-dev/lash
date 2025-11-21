//! Search command implementation
//!
//! The `lash search` command provides fuzzy search across all tasks and files in the index.
//!
//! ## Status
//!
//! This command is partially implemented. The command structure and argument parsing are complete,
//! but the underlying search infrastructure in `lash-db` has not been implemented yet (see
//! `tasks/tasks.fuzzy-search.md`).
//!
//! Once the search API is available in `lash-db`, this module will need to be updated to:
//! - Call the search API instead of returning an error
//! - Implement result highlighting and context snippets
//! - Apply scope filtering if provided via arguments
//!
//! ## Implementation Notes
//!
//! The search command will use FTS5 (Full-Text Search) or fuzzy matching from `lash-db` to:
//! - Search across task titles, bodies, labels, and file paths
//! - Rank results by relevance
//! - Highlight matching terms in output
//! - Support filtering by scope (path prefix)

use anyhow::{Context, Result};
use lash_db::{open_database, search, SearchQuery};
use owo_colors::OwoColorize;
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

// Re-export SearchResult from lash_db for consistency
pub use lash_db::SearchResult;

/// Arguments for the search command
#[derive(Debug, Clone)]
pub struct SearchArgs {
    /// Search query string
    pub query: String,
    /// Maximum number of results to return
    pub limit: usize,
    /// Fuzzy matching threshold (0.0 = exact, 1.0 = very fuzzy)
    /// Note: Currently not used by the FTS5 search backend
    #[allow(dead_code)]
    pub threshold: f32,
    /// Output in JSON format
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
}

// SearchResult is re-exported from lash_db above

/// Execute the search command
///
/// # Arguments
///
/// * `args` - Search command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (general error), 3 (DB error), 5 (no results)
///
/// # Errors
///
/// Returns an error if:
/// - Project root cannot be found
/// - Database does not exist or cannot be opened
/// - Search query execution fails
pub fn execute(args: &SearchArgs) -> Result<i32> {
    // Determine project root
    let project_root = if let Some(ref root) = args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        query = %args.query,
        limit = args.limit,
        "Starting search operation"
    );

    // Determine database path
    let db_path = get_database_path(&project_root);

    // Check if database exists
    if !db_path.exists() {
        if args.json {
            output_json_error(
                "Database not found",
                "Run `lash index` to create the database.",
            )?;
        } else {
            eprintln!("Database not found at {}", db_path.display());
            eprintln!("Run `lash index` to create the database.");
        }
        return Ok(3); // Exit code 3 for DB error
    }

    // Open database
    let conn = open_database(&db_path).context("Failed to open database")?;

    // Build search query
    let query = SearchQuery {
        query: args.query.clone(),
        scope: None, // TODO: Add --scope flag to CLI args in future
        limit: args.limit,
        offset: 0,
        labels: vec![],
        status: None,
        owner: None,
    };

    // Execute search
    let results = search(&conn, &query).context("Search query failed")?;

    tracing::debug!(
        result_count = results.results.len(),
        total_matches = results.total_count,
        "Search completed"
    );

    // Output results
    if args.json {
        output_json(&results.results, &args.query)?;
    } else {
        output_text(&results.results, &args.query, args.no_color);
    }

    // Return appropriate exit code
    if results.results.is_empty() {
        Ok(5) // Exit code 5 for "not found"
    } else {
        Ok(0)
    }
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/db.sqlite")
}

/// Output error message as JSON
fn output_json_error(error: &str, suggestion: &str) -> Result<()> {
    let output = json!({
        "error": error,
        "suggestion": suggestion,
        "results": []
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output search results as JSON
fn output_json(results: &[SearchResult], query: &str) -> Result<()> {
    let output = json!({
        "query": query,
        "count": results.len(),
        "results": results
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output search results as human-readable text
fn output_text(results: &[SearchResult], query: &str, no_color: bool) {
    let use_color = !no_color;

    if results.is_empty() {
        if use_color {
            println!("{}", format!("No results found for '{query}'").yellow());
        } else {
            println!("No results found for '{query}'");
        }
        println!();
        println!("Suggestions:");
        println!("  - Try a different query");
        println!("  - Use a higher --threshold for fuzzier matching");
        println!("  - Check that your files are indexed with `lash index`");
        return;
    }

    // Print header
    if use_color {
        println!(
            "{} {} {}",
            "Found".bold(),
            results.len().to_string().cyan().bold(),
            format!("result(s) for '{query}'").bold()
        );
    } else {
        println!("Found {} result(s) for '{}'", results.len(), query);
    }
    println!();

    // Print each result
    for (i, result) in results.iter().enumerate() {
        if use_color {
            // ID and score
            println!(
                "{}. {} {} {}",
                (i + 1).to_string().dimmed(),
                result.full_id.cyan(),
                format!("(score: {:.2})", result.score).dimmed(),
                format_matched_fields(&result.matched_fields, use_color)
            );

            // Title (potentially with highlighting)
            println!("   {}", result.title.bold());

            // File location
            println!("   {} {}", "└─".dimmed(), result.file_path.blue(),);

            // Snippet (if present and different from title)
            if !result.snippet.is_empty() && result.snippet != result.title {
                println!("   {}", result.snippet.dimmed());
            }

            // Labels (if present)
            if !result.labels.is_empty() {
                println!("      {}", format_labels(&result.labels, use_color));
            }
        } else {
            // No color version
            println!(
                "{}. {} (score: {:.2}) {}",
                i + 1,
                result.full_id,
                result.score,
                format_matched_fields(&result.matched_fields, use_color)
            );
            println!("   {}", result.title);
            println!("   └─ {}", result.file_path);

            if !result.snippet.is_empty() && result.snippet != result.title {
                println!("   {}", result.snippet);
            }

            if !result.labels.is_empty() {
                println!("      {}", format_labels(&result.labels, use_color));
            }
        }

        println!();
    }
}

/// Format matched fields for display
fn format_matched_fields(fields: &[String], use_color: bool) -> String {
    if fields.is_empty() {
        return String::new();
    }

    let fields_str = format!("[{}]", fields.join(", "));

    if use_color {
        fields_str.dimmed().to_string()
    } else {
        fields_str
    }
}

/// Format labels for display
fn format_labels(labels: &[String], use_color: bool) -> String {
    if labels.is_empty() {
        return String::new();
    }

    let labels_str = labels
        .iter()
        .map(|l| format!("#{l}"))
        .collect::<Vec<_>>()
        .join(" ");

    if use_color {
        labels_str.dimmed().to_string()
    } else {
        labels_str
    }
}

/// Highlight matching terms in text (basic implementation)
///
/// This function will be enhanced once we have actual match positions from the search API.
#[allow(dead_code)]
fn highlight_matches(text: &str, query: &str, use_color: bool) -> String {
    if !use_color {
        return text.to_string();
    }

    // Simple case-insensitive highlighting
    // TODO: Use actual match positions from search API
    let query_lower = query.to_lowercase();
    let mut result = String::new();
    let mut remaining = text;

    while let Some(pos) = remaining.to_lowercase().find(&query_lower) {
        result.push_str(&remaining[..pos]);
        let matched = &remaining[pos..pos + query.len()];
        result.push_str(&matched.yellow().bold().to_string());
        remaining = &remaining[pos + query.len()..];
    }
    result.push_str(remaining);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_matched_fields() {
        assert_eq!(format_matched_fields(&[], false), "");
        assert_eq!(
            format_matched_fields(&["title".to_string()], false),
            "[title]"
        );
        assert_eq!(
            format_matched_fields(&["title".to_string(), "body".to_string()], false),
            "[title, body]"
        );
    }

    #[test]
    fn test_highlight_matches() {
        assert_eq!(
            highlight_matches("Hello world", "world", false),
            "Hello world"
        );

        // With color, we'd get ANSI codes, so just verify it runs without panic
        let _ = highlight_matches("Hello world", "world", true);
    }

    #[test]
    fn test_format_labels() {
        assert_eq!(format_labels(&[], false), "");
        assert_eq!(format_labels(&["backend".to_string()], false), "#backend");
        assert_eq!(
            format_labels(&["backend".to_string(), "api".to_string()], false),
            "#backend #api"
        );
    }
}
