//! Search command implementation
//!
//! The `lash search` command provides full-text search across all tasks and files in the index.
//!
//! ## Implementation
//!
//! This command uses the FTS5 (Full-Text Search) infrastructure from `lash-db` to:
//! - Search across task titles, bodies, labels, and file paths
//! - Rank results by relevance score
//! - Display matching terms and context snippets
//! - Limit results with configurable page size
//!
//! The search uses `SQLite`'s FTS5 virtual table for efficient full-text indexing and retrieval.

use anyhow::{Context, Result};
use lash_cli::formatter::{TextFormatter, Verbosity};
use lash_cli::theme::{self, CliTheme};
use lash_db::{open_database, search, SearchQuery};
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
    /// Output in JSON format
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Filter by labels (can specify multiple)
    pub labels: Vec<String>,
    /// Filter by status
    pub status: Option<lash_types::TaskStatus>,
    /// Filter by owner
    pub owner: Option<String>,
    /// Filter by path scope
    pub path: Option<PathBuf>,
    /// Optional color scheme name to use for styling
    pub color_scheme: Option<String>,
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
/// Exit code: 0 (success), 1 (general error), 3 (DB error)
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

    // Build search query with filters
    let mut query = SearchQuery::new(&args.query)
        .with_limit(args.limit)
        .with_offset(0);

    // Apply label filters
    for label in &args.labels {
        query = query.with_label(label.clone());
    }

    // Apply status filter
    if let Some(status) = args.status {
        query = query.with_status(status);
    }

    // Apply owner filter
    if let Some(ref owner) = args.owner {
        query = query.with_owner(owner.clone());
    }

    // Apply path scope filter
    if let Some(ref path) = args.path {
        query = query.with_scope(path.clone());
    }

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
        // Load theme for colored output
        let colors_enabled = !args.no_color && theme::supports_color();
        let theme = CliTheme::load(args.color_scheme.as_deref(), colors_enabled)?;
        let formatter = TextFormatter::with_theme(theme, Verbosity::Normal);

        output_text(&results.results, &args.query, &formatter);
    }

    // Return success - "no results" is a successful search, not an error
    Ok(0)
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/lash.db")
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
fn output_text(results: &[SearchResult], query: &str, formatter: &TextFormatter) {
    let has_theme = formatter.has_color();

    if results.is_empty() {
        let no_results = format!("No results found for '{query}'");
        if has_theme {
            if let Some(theme) = formatter.theme() {
                println!("{}", theme.style_warning(&no_results));
            } else {
                println!("{no_results}");
            }
        } else {
            println!("{no_results}");
        }
        println!();
        println!("Suggestions:");
        println!("  - Try a different query");
        println!("  - Check that your files are indexed with `lash index`");
        return;
    }

    // Print header
    if has_theme {
        if let Some(theme) = formatter.theme() {
            println!(
                "{} {} {}",
                theme.style_info("Found"),
                theme.style_info(&results.len().to_string()),
                theme.style_info(&format!("result(s) for '{query}'"))
            );
        } else {
            println!("Found {} result(s) for '{}'", results.len(), query);
        }
    } else {
        println!("Found {} result(s) for '{}'", results.len(), query);
    }
    println!();

    // Print each result
    for (i, result) in results.iter().enumerate() {
        if has_theme {
            if let Some(theme) = formatter.theme() {
                // ID and score
                println!(
                    "{}. {} {} {}",
                    formatter.format_muted(&(i + 1).to_string()),
                    theme.style_info(&result.full_id),
                    formatter.format_muted(&format!("(score: {:.2})", result.score)),
                    format_matched_fields(&result.matched_fields, formatter)
                );

                // Title - use success color to emphasize it
                println!("   {}", theme.style_success(&result.title));

                // File location - use info color for paths
                println!(
                    "   {} {}",
                    formatter.format_muted("└─"),
                    theme.style_info(&result.file_path)
                );

                // Snippet (if present and different from title)
                if !result.snippet.is_empty() && result.snippet != result.title {
                    println!("   {}", formatter.format_muted(&result.snippet));
                }

                // Labels (if present)
                if !result.labels.is_empty() {
                    println!("      {}", format_labels(&result.labels, formatter));
                }
            }
        } else {
            // No color version
            println!(
                "{}. {} (score: {:.2}) {}",
                i + 1,
                result.full_id,
                result.score,
                format_matched_fields(&result.matched_fields, formatter)
            );
            println!("   {}", result.title);
            println!("   └─ {}", result.file_path);

            if !result.snippet.is_empty() && result.snippet != result.title {
                println!("   {}", result.snippet);
            }

            if !result.labels.is_empty() {
                println!("      {}", format_labels(&result.labels, formatter));
            }
        }

        println!();
    }
}

/// Format matched fields for display
fn format_matched_fields(fields: &[String], formatter: &TextFormatter) -> String {
    if fields.is_empty() {
        return String::new();
    }

    let fields_str = format!("[{}]", fields.join(", "));
    formatter.format_muted(&fields_str)
}

/// Format labels for display
fn format_labels(labels: &[String], formatter: &TextFormatter) -> String {
    if labels.is_empty() {
        return String::new();
    }

    let labels_str = labels
        .iter()
        .map(|l| {
            let label_text = format!("#{l}");
            formatter.format_label(&label_text)
        })
        .collect::<Vec<_>>()
        .join(" ");

    labels_str
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_color_formatter() -> TextFormatter {
        TextFormatter::with_theme(None, Verbosity::Normal)
    }

    #[test]
    fn test_format_matched_fields() {
        let formatter = no_color_formatter();
        assert_eq!(format_matched_fields(&[], &formatter), "");
        assert_eq!(
            format_matched_fields(&["title".to_string()], &formatter),
            "[title]"
        );
        assert_eq!(
            format_matched_fields(&["title".to_string(), "body".to_string()], &formatter),
            "[title, body]"
        );
    }

    #[test]
    fn test_format_labels() {
        let formatter = no_color_formatter();
        assert_eq!(format_labels(&[], &formatter), "");
        assert_eq!(
            format_labels(&["backend".to_string()], &formatter),
            "#backend"
        );
        assert_eq!(
            format_labels(&["backend".to_string(), "api".to_string()], &formatter),
            "#backend #api"
        );
    }
}
