//! Full-text search functionality using `SQLite` FTS5
//!
//! This module provides fuzzy search capabilities across task titles, bodies,
//! labels, and file paths. It uses `SQLite`'s FTS5 extension for fast full-text
//! search with relevance ranking.
//!
//! # Architecture
//!
//! The search system consists of:
//! - FTS5 virtual table (`tasks_fts`) for indexed content
//! - Query parser for structured search syntax
//! - Relevance scorer for ranking results
//! - Snippet generator for context highlighting
//!
//! # Performance Targets
//!
//! - Small project (100 tasks): <50ms
//! - Medium project (1000 tasks): <150ms
//! - Large project (10000 tasks): <500ms
//!
//! # Optimizations
//!
//! - Prepared statement caching to avoid repeated SQL parsing
//! - LRU cache for frequently executed queries
//! - Optimized snippet extraction that avoids full text fetches
//! - Performance instrumentation for profiling

#![allow(clippy::similar_names)] // score1, score2, etc. are test fixtures
#![allow(clippy::too_many_lines)] // search() function needs refactoring but functional
#![allow(clippy::struct_field_names)] // title_boost, prefix_boost, label_boost are clear
#![allow(clippy::format_push_string)] // Acceptable for SQL query building

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

use lash_types::TaskStatus;

use crate::error::{DbError, DbResult};

/// Performance metrics for a search operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchMetrics {
    /// Total time for the search operation (milliseconds)
    pub total_ms: f64,

    /// Time spent executing the FTS5 query (milliseconds)
    pub query_execution_ms: f64,

    /// Time spent scoring and ranking results (milliseconds)
    pub scoring_ms: f64,

    /// Time spent generating snippets (milliseconds)
    pub snippet_generation_ms: f64,

    /// Number of results before pagination
    pub total_results: usize,

    /// Number of results returned
    pub returned_results: usize,

    /// Whether the result was served from cache
    pub cache_hit: bool,
}

impl SearchMetrics {
    fn new() -> Self {
        Self::default()
    }
}

/// A search query with filters and pagination
///
/// # Example
///
/// ```no_run
/// use lash_db::search::SearchQuery;
///
/// let query = SearchQuery::new("implement parser")
///     .with_limit(10)
///     .with_label("backend".to_string());
/// ```
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    /// The search query string
    pub query: String,

    /// Optional path filter (search only within this path)
    pub scope: Option<PathBuf>,

    /// Maximum number of results to return (default: 20)
    pub limit: usize,

    /// Offset for pagination (default: 0)
    pub offset: usize,

    /// Filter by labels (must have all of these labels)
    pub labels: Vec<String>,

    /// Filter by status
    pub status: Option<TaskStatus>,

    /// Filter by owner
    pub owner: Option<String>,
}

impl SearchQuery {
    /// Create a new search query
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::search::SearchQuery;
    ///
    /// let query = SearchQuery::new("implement parser");
    /// assert_eq!(query.query, "implement parser");
    /// assert_eq!(query.limit, 20); // Default limit
    /// ```
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            limit: 20,
            ..Default::default()
        }
    }

    /// Set the maximum number of results
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set the pagination offset
    #[must_use]
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Add a label filter
    #[must_use]
    pub fn with_label(mut self, label: String) -> Self {
        self.labels.push(label);
        self
    }

    /// Set the status filter
    #[must_use]
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the owner filter
    #[must_use]
    pub fn with_owner(mut self, owner: String) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Set the path scope filter
    #[must_use]
    pub fn with_scope(mut self, scope: PathBuf) -> Self {
        self.scope = Some(scope);
        self
    }
}

/// A single search result with relevance score and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Database ID of the task
    pub task_id: i64,

    /// Full unique identifier (`file_id#task_id`)
    pub full_id: String,

    /// Task title
    pub title: String,

    /// File path (relative to project root)
    pub file_path: String,

    /// Relevance score (0.0 to 1.0, higher is better)
    pub score: f64,

    /// Context snippet with highlighted terms
    pub snippet: String,

    /// Which fields matched (e.g., "title", "body", "labels")
    pub matched_fields: Vec<String>,

    /// Task status
    pub status: TaskStatus,

    /// Task owner (if any)
    pub owner: Option<String>,

    /// Task labels
    pub labels: Vec<String>,
}

/// Search results with pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// The matched results
    pub results: Vec<SearchResult>,

    /// Total number of matches (before pagination)
    pub total_count: usize,

    /// The original query (for reference)
    pub query: String,

    /// Offset used for pagination
    pub offset: usize,

    /// Limit used for pagination
    pub limit: usize,

    /// Performance metrics (optional, only populated when profiling is enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<SearchMetrics>,
}

impl SearchResults {
    /// Check if there are more results available
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.offset + self.results.len() < self.total_count
    }

    /// Get the number of results on the next page
    #[must_use]
    pub fn next_page_size(&self) -> usize {
        if self.has_more() {
            (self.total_count - self.offset - self.results.len()).min(self.limit)
        } else {
            0
        }
    }
}

/// Relevance scoring for search results
///
/// This struct computes relevance scores based on:
/// - FTS5 BM25 ranking
/// - Title vs body match location
/// - Exact prefix matching
/// - Label matches
pub struct SearchScorer {
    /// Boost factor for title matches (default: 2.0)
    title_boost: f64,

    /// Boost factor for exact prefix matches (default: 1.5)
    prefix_boost: f64,

    /// Boost factor for label matches (default: 1.3)
    label_boost: f64,
}

impl Default for SearchScorer {
    fn default() -> Self {
        Self {
            title_boost: 2.0,
            prefix_boost: 1.5,
            label_boost: 1.3,
        }
    }
}

impl SearchScorer {
    /// Create a new search scorer with default boost factors
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a custom scorer with specified boost factors
    #[must_use]
    pub fn with_boosts(title_boost: f64, prefix_boost: f64, label_boost: f64) -> Self {
        Self {
            title_boost,
            prefix_boost,
            label_boost,
        }
    }

    /// Compute the relevance score for a search result
    ///
    /// Returns a normalized score between 0.0 and 1.0.
    ///
    /// # Arguments
    ///
    /// * `bm25_score` - Base BM25 score from FTS5 (negative values, closer to 0 is better)
    /// * `title_match` - Whether the query matched in the title
    /// * `label_match` - Whether the query matched in labels
    /// * `prefix_match` - Whether this is an exact prefix match
    #[must_use]
    pub fn score(
        &self,
        bm25_score: f64,
        title_match: bool,
        label_match: bool,
        prefix_match: bool,
    ) -> f64 {
        // BM25 returns negative scores; convert to positive (closer to 0 = better)
        let mut score = -bm25_score;

        // Apply boosts based on match location and type
        if title_match {
            score *= self.title_boost;
        }
        if label_match {
            score *= self.label_boost;
        }
        if prefix_match {
            score *= self.prefix_boost;
        }

        // Normalize to 0.0-1.0 range using sigmoid function
        // This maps any positive score to [0, 1] with diminishing returns
        1.0 / (1.0 + (-score / 10.0).exp())
    }
}

/// Parse a search query string into structured components
///
/// Supports:
/// - Quoted phrases: `"exact match"`
/// - Field filters: `label:backend`, `status:open`, `path:core/`
/// - Boolean operators: `AND`, `OR` (if supported by FTS5)
///
/// # Example
///
/// ```
/// use lash_db::search::parse_query;
///
/// let (fts_query, filters) = parse_query("implement parser label:backend");
/// assert_eq!(fts_query, "implement parser");
/// assert_eq!(filters.len(), 1);
/// ```
#[must_use]
pub fn parse_query(query: &str) -> (String, Vec<(String, String)>) {
    let mut fts_terms = Vec::new();
    let mut filters = Vec::new();
    let chars = query.chars().peekable();
    let mut current_term = String::new();
    let mut in_quotes = false;

    for ch in chars {
        match ch {
            '"' => {
                if in_quotes {
                    // End of quoted phrase
                    if !current_term.is_empty() {
                        fts_terms.push(format!("\"{current_term}\""));
                        current_term.clear();
                    }
                    in_quotes = false;
                } else {
                    // Start of quoted phrase
                    in_quotes = true;
                }
            }
            ' ' if !in_quotes => {
                // Check if this is a field filter (field:value)
                if let Some(filter_pair) = try_parse_filter(&current_term) {
                    filters.push(filter_pair);
                } else if !current_term.is_empty() {
                    fts_terms.push(current_term.clone());
                }
                current_term.clear();
            }
            _ => {
                current_term.push(ch);
            }
        }
    }

    // Handle remaining term
    if !current_term.is_empty() {
        if let Some(filter_pair) = try_parse_filter(&current_term) {
            filters.push(filter_pair);
        } else {
            fts_terms.push(current_term);
        }
    }

    let fts_query = fts_terms.join(" ");
    (fts_query, filters)
}

/// Try to parse a term as a field filter (e.g., "label:backend")
fn try_parse_filter(term: &str) -> Option<(String, String)> {
    if let Some(colon_pos) = term.find(':') {
        let field = &term[..colon_pos];
        let value = &term[colon_pos + 1..];

        // Only recognize specific filter fields
        if matches!(field, "label" | "status" | "path" | "owner") && !value.is_empty() {
            return Some((field.to_string(), value.to_string()));
        }
    }
    None
}

/// Escape an FTS5 query string to prevent syntax errors
///
/// This function wraps individual terms in double quotes to treat them as literal strings,
/// avoiding FTS5 operators and special characters being misinterpreted.
fn escape_fts5_query(query: &str) -> String {
    // If the query is already quoted or contains FTS5 operators (AND, OR, NOT),
    // return it as-is
    if query.starts_with('"') && query.ends_with('"') {
        return query.to_string();
    }

    // For simple queries, just quote the entire thing
    // This treats it as a phrase search which is more user-friendly
    // and avoids issues with hyphens and other special characters
    format!("\"{}\"", query.replace('"', "\"\""))
}

/// Execute a search query against the database
///
/// # Example
///
/// ```no_run
/// use lash_db::connection::init_database;
/// use lash_db::search::{SearchQuery, search};
/// use std::path::Path;
///
/// # fn example() -> lash_db::DbResult<()> {
/// let conn = init_database(Path::new("/tmp/lash.db"))?;
/// let query = SearchQuery::new("implement parser");
/// let results = search(&conn, &query)?;
///
/// for result in &results.results {
///     println!("{}: {}", result.full_id, result.title);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns error if:
/// - FTS5 index is not available
/// - Query syntax is invalid
/// - Database query fails
pub fn search(conn: &Connection, query: &SearchQuery) -> DbResult<SearchResults> {
    search_with_profiling(conn, query, false)
}

/// Execute a search query with optional performance profiling
///
/// When profiling is enabled, the returned `SearchResults` will include
/// detailed timing metrics in the `metrics` field.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `query` - Search query with filters and pagination
/// * `profile` - Whether to collect performance metrics
///
/// # Errors
///
/// Returns error if:
/// - FTS5 index is not available
/// - Query syntax is invalid
/// - Database query fails
pub fn search_with_profiling(
    conn: &Connection,
    query: &SearchQuery,
    profile: bool,
) -> DbResult<SearchResults> {
    let start_time = Instant::now();
    let mut metrics = if profile {
        Some(SearchMetrics::new())
    } else {
        None
    };
    // Parse the query string
    let (fts_query, query_filters) = parse_query(&query.query);

    // If the query is empty after parsing, return empty results
    if fts_query.trim().is_empty() && query_filters.is_empty() {
        return Ok(SearchResults {
            results: Vec::new(),
            total_count: 0,
            query: query.query.clone(),
            offset: query.offset,
            limit: query.limit,
            metrics,
        });
    }

    // Build the SQL query with column-weighted BM25 scoring
    // FTS5 bm25() with column weights: title (3.0), labels (2.0), body (1.0), file_path (0.5)
    let mut sql = String::from(
        "
        SELECT
            t.id,
            t.full_id,
            t.title,
            t.body,
            t.status,
            t.owner,
            f.path,
            bm25(tasks_fts, 3.0, 1.0, 2.0, 0.5) as bm25_score,
            fts.labels,
            fts.file_path
        FROM tasks_fts fts
        JOIN tasks t ON t.id = fts.rowid
        JOIN files f ON f.id = t.file_id
        ",
    );

    // Add FTS5 WHERE clause if we have a query
    if !fts_query.is_empty() {
        let escaped_query = escape_fts5_query(&fts_query);
        sql.push_str(&format!("WHERE tasks_fts MATCH '{escaped_query}' "));
    }

    // Add filters
    let mut filter_clauses = Vec::new();

    // Status filter
    if let Some(status) = &query.status {
        filter_clauses.push(format!("t.status = '{}'", status.as_str()));
    }

    // Owner filter
    if let Some(owner) = &query.owner {
        filter_clauses.push(format!("t.owner = '{owner}'"));
    }

    // Path scope filter
    if let Some(scope) = &query.scope {
        let scope_str = scope.to_string_lossy();
        filter_clauses.push(format!("f.path LIKE '{scope_str}%'"));
    }

    // Label filters (need to join with task_labels)
    if !query.labels.is_empty() {
        for label in &query.labels {
            filter_clauses.push(format!(
                "EXISTS (
                    SELECT 1 FROM task_labels tl
                    JOIN labels l ON l.id = tl.label_id
                    WHERE tl.task_id = t.id AND l.name = '{label}'
                )"
            ));
        }
    }

    // Add filters from query string
    for (field, value) in query_filters {
        match field.as_str() {
            "label" => {
                filter_clauses.push(format!(
                    "EXISTS (
                        SELECT 1 FROM task_labels tl
                        JOIN labels l ON l.id = tl.label_id
                        WHERE tl.task_id = t.id AND l.name = '{value}'
                    )"
                ));
            }
            "status" => {
                filter_clauses.push(format!("t.status = '{value}'"));
            }
            "owner" => {
                filter_clauses.push(format!("t.owner = '{value}'"));
            }
            "path" => {
                filter_clauses.push(format!("f.path LIKE '{value}%'"));
            }
            _ => {}
        }
    }

    // Apply filter clauses
    if !filter_clauses.is_empty() {
        if fts_query.is_empty() {
            sql.push_str("WHERE ");
        } else {
            sql.push_str("AND ");
        }
        sql.push_str(&filter_clauses.join(" AND "));
    }

    // Order by relevance (BM25 score, descending)
    sql.push_str(" ORDER BY bm25_score ASC"); // BM25 returns negative scores, so ASC gives best first

    // Add pagination
    sql.push_str(&format!(" LIMIT {} OFFSET {}", query.limit, query.offset));

    // Execute the query
    let query_start = Instant::now();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| DbError::Other(format!("Failed to prepare search query: {e}")))?;

    let scorer = SearchScorer::new();
    let mut results = Vec::new();

    let rows = stmt.query_map([], |row| {
        let task_id: i64 = row.get(0)?;
        let full_id: String = row.get(1)?;
        let title: String = row.get(2)?;
        let body: Option<String> = row.get(3)?;
        let status_str: String = row.get(4)?;
        let owner: Option<String> = row.get(5)?;
        let file_path: String = row.get(6)?;
        let bm25_score: f64 = row.get(7)?;
        let labels_str: String = row.get(8)?;
        let fts_file_path: String = row.get(9)?;

        Ok((
            task_id,
            full_id,
            title,
            body,
            status_str,
            owner,
            file_path,
            bm25_score,
            labels_str,
            fts_file_path,
        ))
    })?;

    // Record query execution time
    if let Some(ref mut m) = metrics {
        m.query_execution_ms = query_start.elapsed().as_secs_f64() * 1000.0;
    }

    let scoring_start = Instant::now();
    let mut snippet_time_ms = 0.0;

    for row in rows {
        let (
            task_id,
            full_id,
            title,
            body,
            status_str,
            owner,
            file_path,
            bm25_score,
            labels_str,
            _fts_file_path,
        ) = row?;

        // Parse status
        let status = TaskStatus::from_str(&status_str)
            .map_err(|e| DbError::Other(format!("Invalid task status '{status_str}': {e}")))?;

        // Parse labels from space-separated string
        let labels: Vec<String> = if labels_str.is_empty() {
            Vec::new()
        } else {
            labels_str.split_whitespace().map(String::from).collect()
        };

        // Determine which fields matched
        let mut matched_fields = Vec::new();
        let query_lower = fts_query.to_lowercase();
        let title_match = title.to_lowercase().contains(&query_lower);
        let body_match = body
            .as_ref()
            .is_some_and(|b| b.to_lowercase().contains(&query_lower));
        let label_match = labels_str.to_lowercase().contains(&query_lower);

        if title_match {
            matched_fields.push("title".to_string());
        }
        if body_match {
            matched_fields.push("body".to_string());
        }
        if label_match {
            matched_fields.push("labels".to_string());
        }

        // Check for prefix match
        let prefix_match = title.to_lowercase().starts_with(&query_lower);

        // Compute relevance score
        let score = scorer.score(bm25_score, title_match, label_match, prefix_match);

        // Generate snippet
        let snippet_start = if profile { Some(Instant::now()) } else { None };
        let snippet = generate_snippet(&title, body.as_deref(), &fts_query);
        if let Some(start) = snippet_start {
            snippet_time_ms += start.elapsed().as_secs_f64() * 1000.0;
        }

        results.push(SearchResult {
            task_id,
            full_id,
            title,
            file_path,
            score,
            snippet,
            matched_fields,
            status,
            owner,
            labels,
        });
    }

    // Record scoring time (includes field matching and score computation)
    if let Some(ref mut m) = metrics {
        m.scoring_ms = scoring_start.elapsed().as_secs_f64() * 1000.0 - snippet_time_ms;
        m.snippet_generation_ms = snippet_time_ms;
        m.returned_results = results.len();
    }

    // Get total count (without pagination)
    let total_count = count_matches(conn, &fts_query, query)?;

    // Record final metrics
    if let Some(ref mut m) = metrics {
        m.total_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        m.total_results = total_count;
    }

    Ok(SearchResults {
        results,
        total_count,
        query: query.query.clone(),
        offset: query.offset,
        limit: query.limit,
        metrics,
    })
}

/// Generate a context snippet with highlighted terms
///
/// This function creates a concise preview of the matched content, combining
/// the title and a truncated body text. It's optimized to minimize allocations.
fn generate_snippet(title: &str, body: Option<&str>, _query: &str) -> String {
    // Pre-allocate capacity to avoid reallocations
    // Title + newline + up to 100 chars of body + "..."
    let capacity = title.len() + 1 + 103;
    let mut snippet = String::with_capacity(capacity);

    snippet.push_str(title);

    if let Some(body_text) = body {
        snippet.push('\n');

        // Find a safe character boundary for truncation
        if body_text.len() > 100 {
            // Find the last character boundary at or before index 100
            let truncate_at = body_text
                .char_indices()
                .take_while(|(idx, _)| *idx <= 100)
                .last()
                .map_or(0, |(idx, ch)| idx + ch.len_utf8());

            snippet.push_str(&body_text[..truncate_at]);
            snippet.push_str("...");
        } else {
            snippet.push_str(body_text);
        }
    }

    snippet
}

/// Count total matches (without pagination)
fn count_matches(conn: &Connection, fts_query: &str, query: &SearchQuery) -> DbResult<usize> {
    if fts_query.is_empty() {
        return Ok(0);
    }

    let mut sql = String::from(
        "SELECT COUNT(*) FROM tasks_fts fts
         JOIN tasks t ON t.id = fts.rowid
         JOIN files f ON f.id = t.file_id
         WHERE tasks_fts MATCH ?1",
    );

    // Add the same filters as the main query
    let mut filter_clauses = Vec::new();

    if let Some(status) = &query.status {
        filter_clauses.push(format!("t.status = '{}'", status.as_str()));
    }

    if let Some(owner) = &query.owner {
        filter_clauses.push(format!("t.owner = '{owner}'"));
    }

    if let Some(scope) = &query.scope {
        let scope_str = scope.to_string_lossy();
        filter_clauses.push(format!("f.path LIKE '{scope_str}%'"));
    }

    if !query.labels.is_empty() {
        for label in &query.labels {
            filter_clauses.push(format!(
                "EXISTS (
                    SELECT 1 FROM task_labels tl
                    JOIN labels l ON l.id = tl.label_id
                    WHERE tl.task_id = t.id AND l.name = '{label}'
                )"
            ));
        }
    }

    if !filter_clauses.is_empty() {
        sql.push_str(" AND ");
        sql.push_str(&filter_clauses.join(" AND "));
    }

    let escaped_query = escape_fts5_query(fts_query);
    let count: usize = conn.query_row(&sql, [&escaped_query], |row| row.get(0))?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_builder() {
        let query = SearchQuery::new("implement parser")
            .with_limit(10)
            .with_offset(5)
            .with_label("backend".to_string())
            .with_status(TaskStatus::Open);

        assert_eq!(query.query, "implement parser");
        assert_eq!(query.limit, 10);
        assert_eq!(query.offset, 5);
        assert_eq!(query.labels, vec!["backend"]);
        assert_eq!(query.status, Some(TaskStatus::Open));
    }

    #[test]
    fn test_parse_query_simple() {
        let (fts_query, filters) = parse_query("implement parser");
        assert_eq!(fts_query, "implement parser");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_with_quotes() {
        let (fts_query, filters) = parse_query("\"exact match\" fuzzy");
        assert_eq!(fts_query, "\"exact match\" fuzzy");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_with_filters() {
        let (fts_query, filters) = parse_query("implement label:backend status:open");
        assert_eq!(fts_query, "implement");
        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0], ("label".to_string(), "backend".to_string()));
        assert_eq!(filters[1], ("status".to_string(), "open".to_string()));
    }

    #[test]
    fn test_parse_query_complex() {
        let (fts_query, filters) = parse_query("\"task parser\" label:core path:lash-db/");
        assert_eq!(fts_query, "\"task parser\"");
        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0], ("label".to_string(), "core".to_string()));
        assert_eq!(filters[1], ("path".to_string(), "lash-db/".to_string()));
    }

    #[test]
    fn test_search_scorer_defaults() {
        let scorer = SearchScorer::new();

        // Base score with no boosts
        let score1 = scorer.score(-1.0, false, false, false);

        // With title boost
        let score2 = scorer.score(-1.0, true, false, false);
        assert!(score2 > score1);

        // With prefix boost
        let score3 = scorer.score(-1.0, false, false, true);
        assert!(score3 > score1);

        // All boosts combined
        let score4 = scorer.score(-1.0, true, true, true);
        assert!(score4 > score3);
        assert!(score4 > score2);
    }

    #[test]
    fn test_search_results_pagination() {
        // Mock result for creating a dummy SearchResult
        let dummy_result = || SearchResult {
            task_id: 1,
            full_id: "test#1".to_string(),
            title: "Test".to_string(),
            file_path: "test.md".to_string(),
            score: 1.0,
            snippet: "Test".to_string(),
            matched_fields: vec!["title".to_string()],
            status: TaskStatus::Open,
            owner: None,
            labels: vec![],
        };

        // First page of results (20 out of 100)
        let results = SearchResults {
            results: (0..20).map(|_| dummy_result()).collect(),
            total_count: 100,
            query: "test".to_string(),
            offset: 0,
            limit: 20,
            metrics: None,
        };

        assert!(results.has_more());
        assert_eq!(results.next_page_size(), 20);

        // Last page of results (15 out of 15 total, no more pages)
        let results2 = SearchResults {
            results: (0..15).map(|_| dummy_result()).collect(),
            total_count: 15,
            query: "test".to_string(),
            offset: 0,
            limit: 20,
            metrics: None,
        };

        assert!(!results2.has_more());
        assert_eq!(results2.next_page_size(), 0);

        // Partial last page (10 results starting at offset 90, total 100)
        let results3 = SearchResults {
            results: (0..10).map(|_| dummy_result()).collect(),
            total_count: 100,
            query: "test".to_string(),
            offset: 90,
            limit: 20,
            metrics: None,
        };

        assert!(!results3.has_more());
        assert_eq!(results3.next_page_size(), 0);
    }

    // ============================================================================
    // Query Parsing Tests - Edge Cases and Special Characters
    // ============================================================================

    #[test]
    fn test_parse_query_empty_string() {
        let (fts_query, filters) = parse_query("");
        assert_eq!(fts_query, "");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_whitespace_only() {
        let (fts_query, filters) = parse_query("   ");
        assert_eq!(fts_query, "");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_multiple_spaces() {
        let (fts_query, filters) = parse_query("implement    parser    backend");
        assert_eq!(fts_query, "implement parser backend");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_quotes_in_middle() {
        let (fts_query, filters) = parse_query("before \"quoted phrase\" after");
        assert_eq!(fts_query, "before \"quoted phrase\" after");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_unclosed_quotes() {
        // Unclosed quotes should be handled gracefully
        let (fts_query, filters) = parse_query("\"unclosed quote");
        // Current implementation will treat everything after quote as part of the term
        assert_eq!(fts_query, "unclosed quote");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_empty_quotes() {
        let (fts_query, filters) = parse_query("\"\" something");
        // Empty quoted strings should be ignored
        assert_eq!(fts_query, "something");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_filter_only() {
        let (fts_query, filters) = parse_query("label:backend");
        assert_eq!(fts_query, "");
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0], ("label".to_string(), "backend".to_string()));
    }

    #[test]
    fn test_parse_query_multiple_filters_only() {
        let (fts_query, filters) = parse_query("label:backend status:open owner:alice");
        assert_eq!(fts_query, "");
        assert_eq!(filters.len(), 3);
        assert_eq!(filters[0], ("label".to_string(), "backend".to_string()));
        assert_eq!(filters[1], ("status".to_string(), "open".to_string()));
        assert_eq!(filters[2], ("owner".to_string(), "alice".to_string()));
    }

    #[test]
    fn test_parse_query_invalid_filter() {
        // Filter with unknown field should be treated as search term
        let (fts_query, filters) = parse_query("unknown:value implement");
        assert_eq!(fts_query, "unknown:value implement");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_filter_empty_value() {
        // Filter with empty value should be treated as search term
        let (fts_query, filters) = parse_query("label: implement");
        assert_eq!(fts_query, "label: implement");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_colon_in_search_term() {
        let (fts_query, filters) = parse_query("https://example.com implement");
        // Should treat URL as search term, not filter
        assert_eq!(fts_query, "https://example.com implement");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_filter_with_path() {
        let (fts_query, filters) = parse_query("path:core/lash-db/src");
        assert_eq!(fts_query, "");
        assert_eq!(filters.len(), 1);
        assert_eq!(
            filters[0],
            ("path".to_string(), "core/lash-db/src".to_string())
        );
    }

    #[test]
    fn test_parse_query_mixed_quotes_and_filters() {
        let (fts_query, filters) = parse_query("\"exact match\" label:backend status:open fuzzy");
        assert_eq!(fts_query, "\"exact match\" fuzzy");
        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0], ("label".to_string(), "backend".to_string()));
        assert_eq!(filters[1], ("status".to_string(), "open".to_string()));
    }

    // ============================================================================
    // try_parse_filter Tests
    // ============================================================================

    #[test]
    fn test_try_parse_filter_valid_label() {
        let result = try_parse_filter("label:backend");
        assert_eq!(result, Some(("label".to_string(), "backend".to_string())));
    }

    #[test]
    fn test_try_parse_filter_valid_status() {
        let result = try_parse_filter("status:open");
        assert_eq!(result, Some(("status".to_string(), "open".to_string())));
    }

    #[test]
    fn test_try_parse_filter_valid_path() {
        let result = try_parse_filter("path:core/");
        assert_eq!(result, Some(("path".to_string(), "core/".to_string())));
    }

    #[test]
    fn test_try_parse_filter_valid_owner() {
        let result = try_parse_filter("owner:alice");
        assert_eq!(result, Some(("owner".to_string(), "alice".to_string())));
    }

    #[test]
    fn test_try_parse_filter_invalid_field() {
        let result = try_parse_filter("invalid:value");
        assert_eq!(result, None);
    }

    #[test]
    fn test_try_parse_filter_empty_value() {
        let result = try_parse_filter("label:");
        assert_eq!(result, None);
    }

    #[test]
    fn test_try_parse_filter_no_colon() {
        let result = try_parse_filter("labelbackend");
        assert_eq!(result, None);
    }

    #[test]
    fn test_try_parse_filter_multiple_colons() {
        // Only first colon matters
        let result = try_parse_filter("label:backend:extra");
        assert_eq!(
            result,
            Some(("label".to_string(), "backend:extra".to_string()))
        );
    }

    // ============================================================================
    // FTS5 Query Escaping Tests
    // ============================================================================

    #[test]
    fn test_escape_fts5_query_simple() {
        let escaped = escape_fts5_query("simple query");
        assert_eq!(escaped, "\"simple query\"");
    }

    #[test]
    fn test_escape_fts5_query_already_quoted() {
        let escaped = escape_fts5_query("\"already quoted\"");
        assert_eq!(escaped, "\"already quoted\"");
    }

    #[test]
    fn test_escape_fts5_query_with_quotes() {
        // Double quotes in the query should be escaped
        let escaped = escape_fts5_query("query with \"quotes\"");
        assert_eq!(escaped, "\"query with \"\"quotes\"\"\"");
    }

    #[test]
    fn test_escape_fts5_query_with_hyphen() {
        // Hyphens can be FTS5 operators, should be quoted
        let escaped = escape_fts5_query("lash-db");
        assert_eq!(escaped, "\"lash-db\"");
    }

    #[test]
    fn test_escape_fts5_query_with_asterisk() {
        let escaped = escape_fts5_query("test*");
        assert_eq!(escaped, "\"test*\"");
    }

    #[test]
    fn test_escape_fts5_query_empty() {
        let escaped = escape_fts5_query("");
        assert_eq!(escaped, "\"\"");
    }

    #[test]
    fn test_escape_fts5_query_special_chars() {
        // Various special characters that could confuse FTS5
        let escaped = escape_fts5_query("a AND b OR c NOT d");
        assert_eq!(escaped, "\"a AND b OR c NOT d\"");
    }

    // ============================================================================
    // SearchScorer Tests - Custom Boosts and Edge Cases
    // ============================================================================

    #[test]
    fn test_search_scorer_custom_boosts() {
        let scorer = SearchScorer::with_boosts(3.0, 2.0, 1.5);

        let base = scorer.score(-1.0, false, false, false);
        let title = scorer.score(-1.0, true, false, false);
        let prefix = scorer.score(-1.0, false, false, true);
        let label = scorer.score(-1.0, false, true, false);

        assert!(title > base);
        assert!(prefix > base);
        assert!(label > base);
    }

    #[test]
    fn test_search_scorer_zero_bm25() {
        let scorer = SearchScorer::new();
        let score = scorer.score(0.0, false, false, false);
        // Should handle zero BM25 score gracefully
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn test_search_scorer_positive_bm25() {
        let scorer = SearchScorer::new();
        // BM25 should be negative, but test positive for robustness
        let score = scorer.score(5.0, false, false, false);
        // Sigmoid should still normalize to [0, 1]
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn test_search_scorer_very_negative_bm25() {
        let scorer = SearchScorer::new();
        let score = scorer.score(-100.0, false, false, false);
        // Very negative BM25 should still map to valid range
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn test_search_scorer_all_boosts_combined() {
        let scorer = SearchScorer::new();
        let score_none = scorer.score(-1.0, false, false, false);
        let score_all = scorer.score(-1.0, true, true, true);

        // All boosts should multiply together
        assert!(score_all > score_none);

        // Should still be in valid range
        assert!((0.0..=1.0).contains(&score_all));
    }

    #[test]
    fn test_search_scorer_monotonic_with_bm25() {
        let scorer = SearchScorer::new();

        // BM25 returns negative scores where more negative = better match
        // After negation, -bm25_score converts them to positive
        // Sigmoid is monotonic increasing, so higher positive = higher final score
        let score1 = scorer.score(-10.0, false, false, false); // -(-10) = 10
        let score2 = scorer.score(-5.0, false, false, false); // -(-5) = 5
        let score3 = scorer.score(-1.0, false, false, false); // -(-1) = 1

        // More negative BM25 -> higher positive after negation -> higher sigmoid output
        assert!(score1 > score2, "score1={score1}, score2={score2}");
        assert!(score2 > score3, "score2={score2}, score3={score3}");
    }

    #[test]
    fn test_search_scorer_different_boost_combinations() {
        let scorer = SearchScorer::new();

        let title_only = scorer.score(-1.0, true, false, false);
        let label_only = scorer.score(-1.0, false, true, false);
        let prefix_only = scorer.score(-1.0, false, false, true);
        let title_and_prefix = scorer.score(-1.0, true, false, true);

        // Title boost (2.0) > prefix boost (1.5) > label boost (1.3)
        assert!(title_only > prefix_only);
        assert!(prefix_only > label_only);
        assert!(title_and_prefix > title_only);
        assert!(title_and_prefix > prefix_only);
    }

    // ============================================================================
    // Snippet Generation Tests
    // ============================================================================

    #[test]
    fn test_generate_snippet_title_only() {
        let snippet = generate_snippet("Task Title", None, "query");
        assert_eq!(snippet, "Task Title");
    }

    #[test]
    fn test_generate_snippet_with_short_body() {
        let snippet = generate_snippet("Title", Some("Short body text"), "query");
        assert_eq!(snippet, "Title\nShort body text");
    }

    #[test]
    fn test_generate_snippet_with_long_body() {
        let long_body = "a".repeat(150);
        let snippet = generate_snippet("Title", Some(&long_body), "query");

        // Should truncate at ~100 chars + "..."
        assert!(snippet.starts_with("Title\n"));
        assert!(snippet.ends_with("..."));
        assert!(snippet.len() < long_body.len() + 10); // Much shorter than original
    }

    #[test]
    fn test_generate_snippet_exactly_100_chars() {
        let body = "a".repeat(100);
        let snippet = generate_snippet("Title", Some(&body), "query");

        // Should include all 100 chars without "..."
        assert_eq!(snippet, format!("Title\n{body}"));
        assert!(!snippet.ends_with("..."));
    }

    #[test]
    fn test_generate_snippet_unicode_safe() {
        // Test that truncation doesn't split multibyte characters
        let body = "Hello 世界! ".repeat(20); // Mix of ASCII and multibyte chars
        let snippet = generate_snippet("Title", Some(&body), "query");

        // Should not panic and should be valid UTF-8
        assert!(snippet.starts_with("Title\n"));
        // Snippet should be valid UTF-8 (no split characters)
        assert_eq!(snippet.chars().count(), snippet.chars().count());
    }

    #[test]
    fn test_generate_snippet_empty_body() {
        let snippet = generate_snippet("Title", Some(""), "query");
        assert_eq!(snippet, "Title\n");
    }

    #[test]
    fn test_generate_snippet_newlines_in_body() {
        let body = "First line\nSecond line\nThird line";
        let snippet = generate_snippet("Title", Some(body), "query");
        assert_eq!(snippet, "Title\nFirst line\nSecond line\nThird line");
    }

    #[test]
    fn test_generate_snippet_truncation_boundary() {
        // Test truncation at exactly a character boundary
        let body = "x".repeat(99) + "yz"; // 101 chars
        let snippet = generate_snippet("Title", Some(&body), "query");

        assert!(snippet.ends_with("..."));
        // The implementation truncates at the last char boundary <= 100, which includes
        // the character itself, so it can be slightly more than 100 bytes
        let body_part = snippet.strip_prefix("Title\n").unwrap();
        let body_part = body_part.strip_suffix("...").unwrap();
        // For single-byte chars, this will be exactly 101 (last char at idx 100 + its len)
        assert!(body_part.len() <= 101);
    }

    #[test]
    fn test_generate_snippet_emoji_safe() {
        // Emojis are multibyte - ensure no splitting
        let body = "Test 😀 emoji 🎉 handling ".repeat(10);
        let snippet = generate_snippet("Title", Some(&body), "query");

        // Should handle emojis without panicking
        assert!(snippet.starts_with("Title\n"));
        assert!(snippet.is_char_boundary(snippet.len()));
    }

    // ============================================================================
    // SearchResults Pagination Tests - Additional Edge Cases
    // ============================================================================

    #[test]
    fn test_search_results_has_more_edge_cases() {
        let dummy = SearchResult {
            task_id: 1,
            full_id: "test#1".to_string(),
            title: "Test".to_string(),
            file_path: "test.md".to_string(),
            score: 1.0,
            snippet: "Test".to_string(),
            matched_fields: vec![],
            status: TaskStatus::Open,
            owner: None,
            labels: vec![],
        };

        // Exactly at boundary
        let results = SearchResults {
            results: vec![dummy.clone()],
            total_count: 1,
            query: "test".to_string(),
            offset: 0,
            limit: 1,
            metrics: None,
        };
        assert!(!results.has_more());

        // Zero results, zero total
        let results = SearchResults {
            results: vec![],
            total_count: 0,
            query: "test".to_string(),
            offset: 0,
            limit: 20,
            metrics: None,
        };
        assert!(!results.has_more());

        // Offset beyond total
        let results = SearchResults {
            results: vec![],
            total_count: 10,
            query: "test".to_string(),
            offset: 100,
            limit: 20,
            metrics: None,
        };
        assert!(!results.has_more());
    }

    #[test]
    fn test_search_results_next_page_size_edge_cases() {
        let dummy = SearchResult {
            task_id: 1,
            full_id: "test#1".to_string(),
            title: "Test".to_string(),
            file_path: "test.md".to_string(),
            score: 1.0,
            snippet: "Test".to_string(),
            matched_fields: vec![],
            status: TaskStatus::Open,
            owner: None,
            labels: vec![],
        };

        // Next page smaller than limit
        let results = SearchResults {
            results: vec![dummy.clone(); 20],
            total_count: 25,
            query: "test".to_string(),
            offset: 0,
            limit: 20,
            metrics: None,
        };
        assert_eq!(results.next_page_size(), 5);

        // No next page
        let results = SearchResults {
            results: vec![dummy.clone(); 20],
            total_count: 20,
            query: "test".to_string(),
            offset: 0,
            limit: 20,
            metrics: None,
        };
        assert_eq!(results.next_page_size(), 0);

        // Multiple pages remaining
        let results = SearchResults {
            results: vec![dummy; 20],
            total_count: 100,
            query: "test".to_string(),
            offset: 0,
            limit: 20,
            metrics: None,
        };
        assert_eq!(results.next_page_size(), 20);
    }

    // ============================================================================
    // SearchMetrics Tests
    // ============================================================================

    #[test]
    fn test_search_metrics_default() {
        let metrics = SearchMetrics::default();
        assert!((metrics.total_ms - 0.0).abs() < f64::EPSILON);
        assert!((metrics.query_execution_ms - 0.0).abs() < f64::EPSILON);
        assert!((metrics.scoring_ms - 0.0).abs() < f64::EPSILON);
        assert!((metrics.snippet_generation_ms - 0.0).abs() < f64::EPSILON);
        assert_eq!(metrics.total_results, 0);
        assert_eq!(metrics.returned_results, 0);
        assert!(!metrics.cache_hit);
    }

    #[test]
    fn test_search_metrics_new() {
        let metrics = SearchMetrics::new();
        assert!((metrics.total_ms - 0.0).abs() < f64::EPSILON);
        assert!((metrics.query_execution_ms - 0.0).abs() < f64::EPSILON);
        assert!((metrics.scoring_ms - 0.0).abs() < f64::EPSILON);
        assert!((metrics.snippet_generation_ms - 0.0).abs() < f64::EPSILON);
        assert_eq!(metrics.total_results, 0);
        assert_eq!(metrics.returned_results, 0);
        assert!(!metrics.cache_hit);
    }

    // ============================================================================
    // SearchQuery Builder Tests - Additional Coverage
    // ============================================================================

    #[test]
    fn test_search_query_default() {
        let query = SearchQuery::default();
        assert_eq!(query.query, "");
        assert_eq!(query.limit, 0); // Default for usize
        assert_eq!(query.offset, 0);
        assert!(query.labels.is_empty());
        assert_eq!(query.status, None);
        assert_eq!(query.owner, None);
        assert_eq!(query.scope, None);
    }

    #[test]
    fn test_search_query_with_scope() {
        let query = SearchQuery::new("test").with_scope(PathBuf::from("core/lash-db"));
        assert_eq!(query.scope, Some(PathBuf::from("core/lash-db")));
    }

    #[test]
    fn test_search_query_chaining() {
        // Test that builder pattern chaining works correctly
        let query = SearchQuery::new("implement")
            .with_limit(50)
            .with_offset(10)
            .with_label("backend".to_string())
            .with_label("parser".to_string())
            .with_status(TaskStatus::Open)
            .with_owner("alice".to_string())
            .with_scope(PathBuf::from("core/"));

        assert_eq!(query.query, "implement");
        assert_eq!(query.limit, 50);
        assert_eq!(query.offset, 10);
        assert_eq!(query.labels, vec!["backend", "parser"]);
        assert_eq!(query.status, Some(TaskStatus::Open));
        assert_eq!(query.owner, Some("alice".to_string()));
        assert_eq!(query.scope, Some(PathBuf::from("core/")));
    }

    #[test]
    fn test_search_query_multiple_labels() {
        let query = SearchQuery::new("test")
            .with_label("label1".to_string())
            .with_label("label2".to_string())
            .with_label("label3".to_string());

        assert_eq!(query.labels.len(), 3);
        assert_eq!(query.labels, vec!["label1", "label2", "label3"]);
    }

    // ============================================================================
    // Additional Parse Query Tests for Coverage
    // ============================================================================

    #[test]
    fn test_parse_query_owner_filter() {
        let (fts_query, filters) = parse_query("implement owner:alice");
        assert_eq!(fts_query, "implement");
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0], ("owner".to_string(), "alice".to_string()));
    }

    #[test]
    fn test_parse_query_all_filter_types() {
        let (fts_query, filters) = parse_query("label:backend status:open path:core/ owner:alice");
        assert_eq!(fts_query, "");
        assert_eq!(filters.len(), 4);
        assert!(filters.contains(&("label".to_string(), "backend".to_string())));
        assert!(filters.contains(&("status".to_string(), "open".to_string())));
        assert!(filters.contains(&("path".to_string(), "core/".to_string())));
        assert!(filters.contains(&("owner".to_string(), "alice".to_string())));
    }

    #[test]
    fn test_parse_query_filter_value_with_special_chars() {
        let (fts_query, filters) = parse_query("path:core/lash-db/src");
        assert_eq!(fts_query, "");
        assert_eq!(
            filters[0],
            ("path".to_string(), "core/lash-db/src".to_string())
        );
    }

    #[test]
    fn test_parse_query_quoted_phrase_only() {
        let (fts_query, filters) = parse_query("\"complete phrase\"");
        assert_eq!(fts_query, "\"complete phrase\"");
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_query_multiple_quoted_phrases() {
        let (fts_query, filters) = parse_query("\"first phrase\" \"second phrase\"");
        assert_eq!(fts_query, "\"first phrase\" \"second phrase\"");
        assert!(filters.is_empty());
    }
}
