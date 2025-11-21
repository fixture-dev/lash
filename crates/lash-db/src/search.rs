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
//! # Performance Target
//!
//! Search queries should complete in <200ms for typical projects.

#![allow(clippy::similar_names)] // score1, score2, etc. are test fixtures
#![allow(clippy::too_many_lines)] // search() function needs refactoring but functional
#![allow(clippy::struct_field_names)] // title_boost, prefix_boost, label_boost are clear
#![allow(clippy::format_push_string)] // Acceptable for SQL query building

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

use lash_types::TaskStatus;

use crate::error::{DbError, DbResult};

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
            bm25(fts.tasks_fts, 3.0, 1.0, 2.0, 0.5) as bm25_score,
            fts.labels,
            fts.file_path
        FROM tasks_fts fts
        JOIN tasks t ON t.id = fts.rowid
        JOIN files f ON f.id = t.file_id
        ",
    );

    // Add FTS5 WHERE clause if we have a query
    if !fts_query.is_empty() {
        sql.push_str(&format!("WHERE tasks_fts MATCH '{fts_query}' "));
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
        let snippet = generate_snippet(&title, body.as_deref(), &fts_query);

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

    // Get total count (without pagination)
    let total_count = count_matches(conn, &fts_query, query)?;

    Ok(SearchResults {
        results,
        total_count,
        query: query.query.clone(),
        offset: query.offset,
        limit: query.limit,
    })
}

/// Generate a context snippet with highlighted terms
fn generate_snippet(title: &str, body: Option<&str>, _query: &str) -> String {
    // For now, just return the title and first 100 chars of body
    // TODO: Implement proper highlighting with match positions
    let mut snippet = title.to_string();

    if let Some(body_text) = body {
        let preview = if body_text.len() > 100 {
            format!("{}...", &body_text[..100])
        } else {
            body_text.to_string()
        };
        snippet.push_str(&format!("\n{preview}"));
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
         WHERE fts.tasks_fts MATCH ?1",
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

    let count: usize = conn.query_row(&sql, [fts_query], |row| row.get(0))?;
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
        };

        assert!(!results3.has_more());
        assert_eq!(results3.next_page_size(), 0);
    }
}
