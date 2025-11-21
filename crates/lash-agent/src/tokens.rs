//! Token counting and minimization utilities
//!
//! This module provides utilities for estimating token usage and minimizing
//! the amount of context needed for AI agents.

/// Estimate the number of tokens in a string
///
/// Uses a simple heuristic: words * 1.3 to approximate tokenization.
/// This is intentionally conservative and doesn't require loading a full tokenizer.
///
/// # Examples
///
/// ```
/// use lash_agent::tokens::estimate_tokens;
///
/// let text = "This is a test string with several words";
/// let tokens = estimate_tokens(text);
/// assert!(tokens > 0);
/// ```
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    // Count words (split on whitespace)
    let word_count = text.split_whitespace().count();

    // Apply 1.3x multiplier for conservative estimate
    // This accounts for:
    // - Punctuation becoming separate tokens
    // - Some words splitting into multiple tokens
    // - Markdown syntax tokens
    ((word_count as f64) * 1.3).ceil() as usize
}

/// Summarize a task file for minimal context
///
/// Generates a brief summary of a task file showing key statistics
/// without including the full content.
///
/// # Examples
///
/// ```
/// use lash_agent::tokens::summarize_task_file;
///
/// let summary = summarize_task_file("features/auth.md", 10, 7, 2, 1);
/// assert!(summary.contains("features/auth.md"));
/// assert!(summary.contains("10 tasks"));
/// ```
pub fn summarize_task_file(
    path: &str,
    total_tasks: usize,
    completed_tasks: usize,
    open_tasks: usize,
    blocked_tasks: usize,
) -> String {
    let completion_pct = if total_tasks > 0 {
        (completed_tasks as f64 / total_tasks as f64 * 100.0).round() as usize
    } else {
        0
    };

    let mut parts = vec![
        format!("{}", path),
        format!("{} tasks", total_tasks),
        format!("{}% complete", completion_pct),
    ];

    if open_tasks > 0 {
        parts.push(format!("{open_tasks} open"));
    }
    if blocked_tasks > 0 {
        parts.push(format!("{blocked_tasks} blocked"));
    }

    parts.join(", ")
}

/// Summarize task dependencies for minimal context
///
/// Creates a compact summary of a task's dependencies without including
/// full task details.
///
/// # Examples
///
/// ```
/// use lash_agent::tokens::summarize_dependencies;
///
/// let summary = summarize_dependencies(3, "auth.md", 2, 1, 0);
/// assert!(summary.contains("3 tasks"));
/// assert!(summary.contains("auth.md"));
/// ```
#[must_use]
pub fn summarize_dependencies(
    total_deps: usize,
    file: &str,
    done: usize,
    open: usize,
    blocked: usize,
) -> String {
    let mut parts = vec![format!("{} tasks in {}", total_deps, file)];

    if done > 0 {
        parts.push(format!("{done} done"));
    }
    if open > 0 {
        parts.push(format!("{open} open"));
    }
    if blocked > 0 {
        parts.push(format!("{blocked} blocked"));
    }

    parts.join(", ")
}

/// Truncate text to fit within a token budget
///
/// Truncates the text to approximately fit within the specified token budget,
/// adding an ellipsis if truncation occurs.
///
/// # Examples
///
/// ```
/// use lash_agent::tokens::truncate_to_budget;
///
/// let text = "This is a very long text that needs to be truncated";
/// let truncated = truncate_to_budget(text, 5);
/// assert!(truncated.len() < text.len());
/// assert!(truncated.ends_with("..."));
/// ```
pub fn truncate_to_budget(text: &str, token_budget: usize) -> String {
    let current_tokens = estimate_tokens(text);

    if current_tokens <= token_budget {
        return text.to_string();
    }

    // Calculate approximate character budget
    // Rough heuristic: 1 token ≈ 4 characters
    let char_budget = token_budget * 4;

    if char_budget < 10 {
        return "...".to_string();
    }

    // Truncate to character budget, accounting for ellipsis
    let truncate_at = char_budget.saturating_sub(3).min(text.len());

    // Try to truncate at a word boundary
    let truncated = if let Some(last_space) = text[..truncate_at].rfind(char::is_whitespace) {
        &text[..last_space]
    } else {
        &text[..truncate_at]
    };

    format!("{}...", truncated.trim_end())
}

/// Calculate how to distribute a token budget across multiple sections
///
/// Given a total budget and section priorities, returns token allocations
/// for each section. Higher priority sections get allocated first.
///
/// # Examples
///
/// ```
/// use lash_agent::tokens::distribute_budget;
///
/// let sections = vec![
///     ("schema", 500, 10),      // name, estimated_tokens, priority
///     ("examples", 300, 5),
///     ("tasks", 1000, 1),
/// ];
///
/// let allocations = distribute_budget(1000, &sections);
/// assert_eq!(allocations.len(), 3);
/// ```
#[must_use]
pub fn distribute_budget<'a>(
    total_budget: usize,
    sections: &'a [(&'a str, usize, u8)], // (name, estimated_tokens, priority)
) -> Vec<(&'a str, usize)> {
    if sections.is_empty() {
        return Vec::new();
    }

    // Sort by priority (descending)
    let mut sorted_sections: Vec<_> = sections.iter().collect();
    sorted_sections.sort_by(|a, b| b.2.cmp(&a.2));

    let mut allocations = Vec::new();
    let mut remaining_budget = total_budget;

    for (name, estimated_tokens, _priority) in sorted_sections {
        let allocation = (*estimated_tokens).min(remaining_budget);
        allocations.push((*name, allocation));
        remaining_budget = remaining_budget.saturating_sub(allocation);
    }

    // Return in original order
    let mut result = Vec::new();
    for (name, _, _) in sections {
        if let Some((_, alloc)) = allocations.iter().find(|(n, _)| n == name) {
            result.push((*name, *alloc));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_simple() {
        let text = "hello world";
        let tokens = estimate_tokens(text);
        // 2 words * 1.3 = 2.6, rounded up = 3
        assert_eq!(tokens, 3);
    }

    #[test]
    fn test_estimate_tokens_complex() {
        let text = "This is a longer sentence with multiple words.";
        let tokens = estimate_tokens(text);
        // 8 words * 1.3 = 10.4, rounded up = 11
        assert_eq!(tokens, 11);
    }

    #[test]
    fn test_summarize_task_file() {
        let summary = summarize_task_file("auth.md", 10, 7, 2, 1);
        assert!(summary.contains("auth.md"));
        assert!(summary.contains("10 tasks"));
        assert!(summary.contains("70% complete"));
        assert!(summary.contains("2 open"));
        assert!(summary.contains("1 blocked"));
    }

    #[test]
    fn test_summarize_task_file_all_complete() {
        let summary = summarize_task_file("done.md", 5, 5, 0, 0);
        assert!(summary.contains("100% complete"));
        assert!(!summary.contains("open"));
        assert!(!summary.contains("blocked"));
    }

    #[test]
    fn test_summarize_dependencies() {
        let summary = summarize_dependencies(5, "core.md", 3, 2, 0);
        assert!(summary.contains("5 tasks"));
        assert!(summary.contains("core.md"));
        assert!(summary.contains("3 done"));
        assert!(summary.contains("2 open"));
    }

    #[test]
    fn test_truncate_to_budget_no_truncation() {
        let text = "Short text";
        let truncated = truncate_to_budget(text, 100);
        assert_eq!(truncated, text);
    }

    #[test]
    fn test_truncate_to_budget_with_truncation() {
        let text = "This is a very long piece of text that needs truncation";
        let truncated = truncate_to_budget(text, 5);
        assert!(truncated.len() < text.len());
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_to_budget_word_boundary() {
        let text = "One Two Three Four Five";
        let truncated = truncate_to_budget(text, 3);
        // Should truncate at word boundary
        assert!(truncated.ends_with("..."));
        assert!(!truncated.contains("Thr")); // Shouldn't cut mid-word
    }

    #[test]
    fn test_distribute_budget_simple() {
        let sections = vec![("schema", 100, 10), ("examples", 50, 5), ("tasks", 200, 1)];

        let allocations = distribute_budget(350, &sections);
        assert_eq!(allocations.len(), 3);

        // Should get full allocations for all (budget is sufficient)
        assert_eq!(allocations[0], ("schema", 100));
        assert_eq!(allocations[1], ("examples", 50));
        assert_eq!(allocations[2], ("tasks", 200));
    }

    #[test]
    fn test_distribute_budget_limited() {
        let sections = vec![("schema", 100, 10), ("examples", 100, 5), ("tasks", 100, 1)];

        let allocations = distribute_budget(150, &sections);

        // High priority should get full allocation
        assert_eq!(allocations[0], ("schema", 100));
        // Medium priority gets remaining budget
        assert_eq!(allocations[1], ("examples", 50));
        // Low priority gets nothing
        assert_eq!(allocations[2], ("tasks", 0));
    }

    #[test]
    fn test_distribute_budget_empty() {
        let sections = vec![];
        let allocations = distribute_budget(1000, &sections);
        assert!(allocations.is_empty());
    }

    #[test]
    fn test_distribute_budget_zero_budget() {
        let sections = vec![("schema", 100, 10), ("tasks", 50, 1)];

        let allocations = distribute_budget(0, &sections);
        assert_eq!(allocations[0], ("schema", 0));
        assert_eq!(allocations[1], ("tasks", 0));
    }
}
