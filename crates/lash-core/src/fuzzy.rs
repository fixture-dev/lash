//! Fuzzy matching for finding similar task IDs
//!
//! Uses Levenshtein distance to find tasks with similar IDs to broken references.
//! This module is useful for autocomplete, typo correction, and suggesting alternatives.

use strsim::levenshtein;

/// A candidate match for fuzzy searching
///
/// Represents a potential match with a similarity score, useful for ranking
/// and filtering results.
#[derive(Debug, Clone, PartialEq)]
pub struct FuzzyCandidate {
    /// The full task ID that might be the correct target
    pub task_id: String,
    /// Similarity score (0.0 = no match, 1.0 = perfect match)
    pub score: f64,
}

/// Fuzzy matcher for finding similar task IDs
///
/// Uses normalized Levenshtein distance to compute similarity between strings.
/// Configurable threshold and result limits make it suitable for various use cases
/// including autocomplete, typo correction, and search suggestions.
///
/// # Examples
///
/// ```
/// use lash_core::fuzzy::FuzzyMatcher;
///
/// let matcher = FuzzyMatcher::default();
/// let candidates = vec![
///     "tasks#setup-database".to_string(),
///     "tasks#setup-databse".to_string(),  // typo
///     "tasks#setup-network".to_string(),
/// ];
///
/// let matches = matcher.find_matches("tasks#setup-databse", &candidates);
/// assert!(!matches.is_empty());
/// ```
pub struct FuzzyMatcher {
    /// Minimum similarity threshold for candidates (default: 0.6)
    min_threshold: f64,
    /// Maximum number of candidates to return (default: 5)
    max_candidates: usize,
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self {
            min_threshold: 0.6,
            max_candidates: 5,
        }
    }
}

impl FuzzyMatcher {
    /// Create a new fuzzy matcher with custom settings
    ///
    /// # Arguments
    ///
    /// * `min_threshold` - Minimum similarity score to include in results (0.0-1.0)
    /// * `max_candidates` - Maximum number of results to return
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::fuzzy::FuzzyMatcher;
    ///
    /// // Strict matching with fewer results
    /// let matcher = FuzzyMatcher::new(0.8, 3);
    /// ```
    #[must_use]
    pub fn new(min_threshold: f64, max_candidates: usize) -> Self {
        Self {
            min_threshold,
            max_candidates,
        }
    }

    /// Find similar task IDs using Levenshtein distance
    ///
    /// Returns candidates sorted by score (highest first), limited to
    /// `max_candidates` with scores >= `min_threshold`.
    ///
    /// # Arguments
    ///
    /// * `query` - The query string to match against
    /// * `candidates` - List of candidate strings to search through
    ///
    /// # Returns
    ///
    /// A vector of `FuzzyCandidate` sorted by similarity score (descending)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::fuzzy::FuzzyMatcher;
    ///
    /// let matcher = FuzzyMatcher::default();
    /// let candidates = vec![
    ///     "tasks#setup-database".to_string(),
    ///     "tasks#setup-databse".to_string(),  // typo
    ///     "tasks#setup-network".to_string(),
    /// ];
    ///
    /// let matches = matcher.find_matches("tasks#setup-databse", &candidates);
    /// assert!(!matches.is_empty());
    /// assert_eq!(matches[0].task_id, "tasks#setup-databse");
    /// assert!(matches[0].score > 0.9);
    /// ```
    #[must_use]
    pub fn find_matches(&self, query: &str, candidates: &[String]) -> Vec<FuzzyCandidate> {
        let mut scored: Vec<FuzzyCandidate> = candidates
            .iter()
            .map(|candidate| {
                let score = self.compute_similarity(query, candidate);
                FuzzyCandidate {
                    task_id: candidate.clone(),
                    score,
                }
            })
            .filter(|c| c.score >= self.min_threshold)
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit to max candidates
        scored.truncate(self.max_candidates);

        scored
    }

    /// Compute similarity score between two strings
    ///
    /// Uses normalized Levenshtein distance where:
    /// - 1.0 = perfect match
    /// - 0.0 = completely different
    ///
    /// The score is computed as `1.0 - (edit_distance / max_length)`.
    ///
    /// # Arguments
    ///
    /// * `a` - First string to compare
    /// * `b` - Second string to compare
    ///
    /// # Returns
    ///
    /// A similarity score from 0.0 (no match) to 1.0 (perfect match)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::fuzzy::FuzzyMatcher;
    ///
    /// let matcher = FuzzyMatcher::default();
    /// let score1 = matcher.compute_similarity("hello", "hello");
    /// let score2 = matcher.compute_similarity("hello", "helo");
    /// let score3 = matcher.compute_similarity("hello", "world");
    ///
    /// assert_eq!(score1, 1.0);
    /// assert!(score2 > 0.7);
    /// assert!(score3 < 0.5);
    /// ```
    #[must_use]
    #[allow(clippy::unused_self)] // Method belongs to impl block for consistency
    #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for similarity scoring
    pub fn compute_similarity(&self, a: &str, b: &str) -> f64 {
        if a == b {
            return 1.0;
        }

        let distance = levenshtein(a, b);
        let max_len = a.len().max(b.len());

        if max_len == 0 {
            return 1.0;
        }

        // Normalize: 1.0 - (distance / max_length)
        1.0 - (distance as f64 / max_len as f64)
    }

    /// Get the configured minimum threshold
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::fuzzy::FuzzyMatcher;
    ///
    /// let matcher = FuzzyMatcher::new(0.75, 10);
    /// assert!((matcher.min_threshold() - 0.75).abs() < 0.01);
    /// ```
    #[must_use]
    pub fn min_threshold(&self) -> f64 {
        self.min_threshold
    }

    /// Get the configured max candidates
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::fuzzy::FuzzyMatcher;
    ///
    /// let matcher = FuzzyMatcher::new(0.6, 10);
    /// assert_eq!(matcher.max_candidates(), 10);
    /// ```
    #[must_use]
    pub fn max_candidates(&self) -> usize {
        self.max_candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_match() {
        let matcher = FuzzyMatcher::default();
        let score = matcher.compute_similarity("tasks#setup", "tasks#setup");
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_single_char_typo() {
        let matcher = FuzzyMatcher::default();
        let score = matcher.compute_similarity("tasks#setup", "tasks#setpu");
        assert!(
            score > 0.8,
            "Score for single transposition should be > 0.8"
        );
    }

    #[test]
    fn test_missing_char() {
        let matcher = FuzzyMatcher::default();
        let score = matcher.compute_similarity("tasks#setup", "tasks#setip");
        assert!(score > 0.8, "Score for single substitution should be > 0.8");
    }

    #[test]
    fn test_completely_different() {
        let matcher = FuzzyMatcher::default();
        let score = matcher.compute_similarity("tasks#setup", "other#teardown");
        assert!(
            score < 0.5,
            "Completely different strings should have low score"
        );
    }

    #[test]
    #[allow(clippy::similar_names)] // matcher vs matches is intentional
    fn test_find_matches_with_typo() {
        let matcher = FuzzyMatcher::default();
        let candidates = vec![
            "tasks#setup-database".to_string(),
            "tasks#setup-databse".to_string(), // contains typo
            "tasks#setup-network".to_string(),
            "other#completely-different".to_string(),
        ];

        let matches = matcher.find_matches("tasks#setup-databse", &candidates);

        // Should find the exact match first
        assert!(!matches.is_empty());
        assert_eq!(matches[0].task_id, "tasks#setup-databse");
        assert!((matches[0].score - 1.0).abs() < 0.01);

        // Should find the similar one second
        assert!(matches.len() >= 2);
        assert_eq!(matches[1].task_id, "tasks#setup-database");
        assert!(matches[1].score > 0.9);
    }

    #[test]
    #[allow(clippy::similar_names)] // matcher vs matches is intentional
    fn test_find_matches_respects_threshold() {
        let matcher = FuzzyMatcher::new(0.9, 10); // High threshold
        let candidates = vec![
            "tasks#setup".to_string(),
            "tasks#teardown".to_string(), // Very different
        ];

        let matches = matcher.find_matches("tasks#setup", &candidates);

        // Should only find the perfect match
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].task_id, "tasks#setup");
    }

    #[test]
    #[allow(clippy::similar_names)] // matcher vs matches is intentional
    fn test_find_matches_limits_results() {
        let matcher = FuzzyMatcher::new(0.5, 2); // Max 2 candidates
        let candidates = vec![
            "a".to_string(),
            "ab".to_string(),
            "abc".to_string(),
            "abcd".to_string(),
            "abcde".to_string(),
        ];

        let matches = matcher.find_matches("abc", &candidates);

        // Should return at most 2 results
        assert!(matches.len() <= 2);
    }

    #[test]
    #[allow(clippy::similar_names)] // matcher vs matches is intentional
    fn test_find_matches_sorted_by_score() {
        let matcher = FuzzyMatcher::default();
        let candidates = vec![
            "tasks#completely-different-thing".to_string(),
            "tasks#setup-database".to_string(),
            "tasks#setup-databse".to_string(), // Closest match
        ];

        let matches = matcher.find_matches("tasks#setup-databse", &candidates);

        // Results should be sorted by score descending
        for i in 0..matches.len().saturating_sub(1) {
            assert!(
                matches[i].score >= matches[i + 1].score,
                "Results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_empty_strings() {
        let matcher = FuzzyMatcher::default();
        let score = matcher.compute_similarity("", "");
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_one_empty_string() {
        let matcher = FuzzyMatcher::default();
        let score = matcher.compute_similarity("", "hello");
        assert!(score.abs() < 0.01);
    }

    #[test]
    fn test_case_sensitive() {
        let matcher = FuzzyMatcher::default();
        let score = matcher.compute_similarity("Tasks#Setup", "tasks#setup");
        assert!(
            score < 1.0,
            "Comparison should be case-sensitive by default"
        );
    }

    #[test]
    fn test_getters() {
        let matcher = FuzzyMatcher::new(0.75, 10);
        assert!((matcher.min_threshold() - 0.75).abs() < 0.01);
        assert_eq!(matcher.max_candidates(), 10);
    }
}
