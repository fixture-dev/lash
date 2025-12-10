//! Tree-based task selection component

use lash_core::fuzzy::FuzzyMatcher;

/// Maximum number of suggestions to show in filtered results
const MAX_SUGGESTIONS: usize = 15;

/// An item in the tree select
///
/// Represents a task that can be selected as a parent.
///
/// # Examples
///
/// ```
/// use lash_tui::components::TreeSelectItem;
///
/// let item = TreeSelectItem {
///     id: "task-1".to_string(),
///     title: "Parent task".to_string(),
///     depth: 0,
///     status_indicator: ' ',
/// };
/// assert_eq!(item.depth, 0);
/// ```
#[derive(Debug, Clone)]
pub struct TreeSelectItem {
    /// Task ID
    pub id: String,
    /// Task title
    pub title: String,
    /// Depth level in the hierarchy
    pub depth: u8,
    /// Status indicator character (' ', 'x', '-', '!')
    pub status_indicator: char,
}

/// State for tree-based task selection
///
/// Provides a searchable dropdown for selecting a parent task from a hierarchical list.
/// Supports filtering and keyboard navigation.
///
/// # Examples
///
/// ```
/// use lash_tui::components::{TreeSelectState, TreeSelectItem};
///
/// let items = vec![
///     TreeSelectItem {
///         id: "task-1".to_string(),
///         title: "Parent task".to_string(),
///         depth: 0,
///         status_indicator: ' ',
///     },
/// ];
/// let mut tree_select = TreeSelectState::new(items);
/// assert!(!tree_select.is_expanded);
/// tree_select.toggle_expand();
/// assert!(tree_select.is_expanded);
/// ```
#[derive(Debug, Clone)]
pub struct TreeSelectState {
    /// Search/filter input
    pub input: String,
    /// All available items
    pub all_items: Vec<TreeSelectItem>,
    /// Filtered items (indices into `all_items`)
    pub filtered_indices: Vec<usize>,
    /// Currently highlighted index in filtered list
    pub selected_index: usize,
    /// Committed selection (None for top-level)
    pub selected_item: Option<TreeSelectItem>,
    /// Whether dropdown is expanded
    pub is_expanded: bool,
}

impl TreeSelectState {
    /// Create a new tree select with the given items
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "1".to_string(),
    ///         title: "Task 1".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let tree_select = TreeSelectState::new(items);
    /// assert_eq!(tree_select.all_items.len(), 1);
    /// ```
    #[must_use]
    pub fn new(items: Vec<TreeSelectItem>) -> Self {
        let filtered_indices: Vec<usize> = (0..items.len()).collect();
        Self {
            input: String::new(),
            all_items: items,
            filtered_indices,
            selected_index: 0,
            selected_item: None,
            is_expanded: false,
        }
    }

    /// Filter items based on current input
    ///
    /// Updates `filtered_indices` to match items whose titles contain
    /// the input string (case-insensitive).
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "1".to_string(),
    ///         title: "Backend task".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    ///     TreeSelectItem {
    ///         id: "2".to_string(),
    ///         title: "Frontend task".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let mut tree_select = TreeSelectState::new(items);
    /// tree_select.input = "back".to_string();
    /// tree_select.filter();
    /// assert_eq!(tree_select.filtered_indices.len(), 1);
    /// ```
    pub fn filter(&mut self) {
        if self.input.is_empty() {
            // Show all items when input is empty (no limit - let UI handle scrolling)
            self.filtered_indices = (0..self.all_items.len()).collect();
        } else {
            // Hybrid approach: substring matching + fuzzy matching for better results
            let input_lower = self.input.to_lowercase();

            // First, collect substring matches (these get priority)
            let mut substring_matches: Vec<(usize, f64)> = self
                .all_items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    if item.title.to_lowercase().contains(&input_lower) {
                        // Boost score based on match position (earlier = better)
                        let pos = item.title.to_lowercase().find(&input_lower).unwrap_or(0);
                        #[allow(clippy::cast_precision_loss)]
                        let score = 1.0 - (pos as f64 / item.title.len() as f64) * 0.2;
                        Some((idx, score))
                    } else {
                        None
                    }
                })
                .collect();

            // Then, use fuzzy matching for additional results
            let search_engine = FuzzyMatcher::new(0.4, MAX_SUGGESTIONS);
            let titles: Vec<String> = self
                .all_items
                .iter()
                .map(|item| item.title.clone())
                .collect();
            let search_results = search_engine.find_matches(&self.input, &titles);

            // Add fuzzy matches that aren't already in substring matches
            let substring_indices: std::collections::HashSet<usize> =
                substring_matches.iter().map(|(idx, _)| *idx).collect();

            for candidate in search_results {
                if let Some(idx) = self
                    .all_items
                    .iter()
                    .position(|item| item.title == candidate.task_id)
                {
                    if !substring_indices.contains(&idx) {
                        substring_matches.push((idx, candidate.score));
                    }
                }
            }

            // Sort by score (descending) and limit to MAX_SUGGESTIONS
            substring_matches
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            self.filtered_indices = substring_matches
                .into_iter()
                .take(MAX_SUGGESTIONS)
                .map(|(idx, _)| idx)
                .collect();
        }

        // Reset selection if current selection is out of bounds
        if self.selected_index >= self.filtered_indices.len() {
            self.selected_index = 0;
        }
    }

    /// Select the next item in the filtered list
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "1".to_string(),
    ///         title: "Task 1".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    ///     TreeSelectItem {
    ///         id: "2".to_string(),
    ///         title: "Task 2".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let mut tree_select = TreeSelectState::new(items);
    /// tree_select.select_next();
    /// assert_eq!(tree_select.selected_index, 1);
    /// ```
    pub fn select_next(&mut self) {
        if !self.filtered_indices.is_empty()
            && self.selected_index + 1 < self.filtered_indices.len()
        {
            self.selected_index += 1;
        }
    }

    /// Select the previous item in the filtered list
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "1".to_string(),
    ///         title: "Task 1".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    ///     TreeSelectItem {
    ///         id: "2".to_string(),
    ///         title: "Task 2".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let mut tree_select = TreeSelectState::new(items);
    /// tree_select.selected_index = 1;
    /// tree_select.select_prev();
    /// assert_eq!(tree_select.selected_index, 0);
    /// ```
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Confirm the currently highlighted item as the selection
    ///
    /// Closes the dropdown and commits the selection.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "1".to_string(),
    ///         title: "Task 1".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let mut tree_select = TreeSelectState::new(items);
    /// tree_select.toggle_expand();
    /// tree_select.confirm_selection();
    /// assert!(tree_select.selected_item.is_some());
    /// assert!(!tree_select.is_expanded);
    /// ```
    pub fn confirm_selection(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected_index < self.filtered_indices.len() {
            let item_index = self.filtered_indices[self.selected_index];
            self.selected_item = Some(self.all_items[item_index].clone());
            self.is_expanded = false;
        }
    }

    /// Clear the selection (select "None" / top-level)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "1".to_string(),
    ///         title: "Task 1".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let mut tree_select = TreeSelectState::new(items);
    /// tree_select.confirm_selection();
    /// tree_select.clear_selection();
    /// assert!(tree_select.selected_item.is_none());
    /// ```
    pub fn clear_selection(&mut self) {
        self.selected_item = None;
        self.is_expanded = false;
    }

    /// Toggle dropdown expansion
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "1".to_string(),
    ///         title: "Task 1".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let mut tree_select = TreeSelectState::new(items);
    /// assert!(!tree_select.is_expanded);
    /// tree_select.toggle_expand();
    /// assert!(tree_select.is_expanded);
    /// tree_select.toggle_expand();
    /// assert!(!tree_select.is_expanded);
    /// ```
    pub fn toggle_expand(&mut self) {
        self.is_expanded = !self.is_expanded;
    }

    /// Select an item by its ID
    ///
    /// Finds the item with the given ID and sets it as the selected item.
    /// Returns true if an item was found and selected, false otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "task-1".to_string(),
    ///         title: "First task".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    ///     TreeSelectItem {
    ///         id: "task-2".to_string(),
    ///         title: "Second task".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let mut tree_select = TreeSelectState::new(items);
    /// assert!(tree_select.select_by_id("task-2"));
    /// assert_eq!(tree_select.selected_item.as_ref().unwrap().id, "task-2");
    /// assert!(!tree_select.select_by_id("nonexistent"));
    /// ```
    pub fn select_by_id(&mut self, id: &str) -> bool {
        if let Some(idx) = self.all_items.iter().position(|item| item.id == id) {
            self.selected_item = Some(self.all_items[idx].clone());
            // Also update selected_index to match if it's in filtered list
            if let Some(filtered_pos) = self.filtered_indices.iter().position(|&i| i == idx) {
                self.selected_index = filtered_pos;
            }
            true
        } else {
            false
        }
    }

    /// Get the currently highlighted item (before confirmation)
    ///
    /// Returns the item at the current `selected_index` in the filtered list.
    /// This is useful for showing what item would be selected if the user confirms.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "1".to_string(),
    ///         title: "Task 1".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    ///     TreeSelectItem {
    ///         id: "2".to_string(),
    ///         title: "Task 2".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let mut tree_select = TreeSelectState::new(items);
    /// tree_select.select_next();
    /// let highlighted = tree_select.highlighted_item();
    /// assert!(highlighted.is_some());
    /// assert_eq!(highlighted.unwrap().id, "2");
    /// ```
    #[must_use]
    pub fn highlighted_item(&self) -> Option<&TreeSelectItem> {
        if self.filtered_indices.is_empty() {
            return None;
        }
        let idx = self.filtered_indices.get(self.selected_index)?;
        self.all_items.get(*idx)
    }

    /// Insert a character into the input field
    ///
    /// Automatically filters items after inserting.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "1".to_string(),
    ///         title: "Task 1".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let mut tree_select = TreeSelectState::new(items);
    /// tree_select.input_char('t');
    /// tree_select.input_char('a');
    /// assert_eq!(tree_select.input, "ta");
    /// ```
    pub fn input_char(&mut self, c: char) {
        self.input.push(c);
        self.filter();
    }

    /// Delete character before cursor (backspace)
    ///
    /// Automatically filters items after deleting.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{TreeSelectState, TreeSelectItem};
    ///
    /// let items = vec![
    ///     TreeSelectItem {
    ///         id: "1".to_string(),
    ///         title: "Task 1".to_string(),
    ///         depth: 0,
    ///         status_indicator: ' ',
    ///     },
    /// ];
    /// let mut tree_select = TreeSelectState::new(items);
    /// tree_select.input_char('a');
    /// tree_select.input_char('b');
    /// tree_select.backspace();
    /// assert_eq!(tree_select.input, "a");
    /// ```
    pub fn backspace(&mut self) {
        if !self.input.is_empty() {
            self.input.pop();
            self.filter();
        }
    }
}

impl Default for TreeSelectState {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_items() -> Vec<TreeSelectItem> {
        vec![
            TreeSelectItem {
                id: "1".to_string(),
                title: "Backend tasks".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
            TreeSelectItem {
                id: "2".to_string(),
                title: "Frontend tasks".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
            TreeSelectItem {
                id: "3".to_string(),
                title: "Database migration".to_string(),
                depth: 1,
                status_indicator: 'x',
            },
        ]
    }

    #[test]
    fn test_new() {
        let items = create_test_items();
        let tree_select = TreeSelectState::new(items.clone());
        assert_eq!(tree_select.all_items.len(), 3);
        assert_eq!(tree_select.filtered_indices.len(), 3);
        assert!(!tree_select.is_expanded);
        assert!(tree_select.selected_item.is_none());
    }

    #[test]
    fn test_filter() {
        let items = create_test_items();
        let mut tree_select = TreeSelectState::new(items);

        tree_select.input = "backend".to_string();
        tree_select.filter();
        assert_eq!(tree_select.filtered_indices.len(), 1);

        tree_select.input.clear();
        tree_select.filter();
        assert_eq!(tree_select.filtered_indices.len(), 3);
    }

    #[test]
    fn test_select_navigation() {
        let items = create_test_items();
        let mut tree_select = TreeSelectState::new(items);

        assert_eq!(tree_select.selected_index, 0);
        tree_select.select_next();
        assert_eq!(tree_select.selected_index, 1);
        tree_select.select_next();
        assert_eq!(tree_select.selected_index, 2);
        tree_select.select_next(); // Should stay at 2
        assert_eq!(tree_select.selected_index, 2);

        tree_select.select_prev();
        assert_eq!(tree_select.selected_index, 1);
    }

    #[test]
    fn test_confirm_selection() {
        let items = create_test_items();
        let mut tree_select = TreeSelectState::new(items);
        tree_select.is_expanded = true;
        tree_select.selected_index = 1;

        tree_select.confirm_selection();
        assert!(tree_select.selected_item.is_some());
        assert_eq!(tree_select.selected_item.as_ref().unwrap().id, "2");
        assert!(!tree_select.is_expanded);
    }

    #[test]
    fn test_clear_selection() {
        let items = create_test_items();
        let mut tree_select = TreeSelectState::new(items);
        tree_select.selected_index = 1;
        tree_select.confirm_selection();

        tree_select.clear_selection();
        assert!(tree_select.selected_item.is_none());
    }

    #[test]
    fn test_toggle_expand() {
        let items = create_test_items();
        let mut tree_select = TreeSelectState::new(items);

        assert!(!tree_select.is_expanded);
        tree_select.toggle_expand();
        assert!(tree_select.is_expanded);
        tree_select.toggle_expand();
        assert!(!tree_select.is_expanded);
    }

    #[test]
    fn test_input_char() {
        let items = create_test_items();
        let mut tree_select = TreeSelectState::new(items);

        tree_select.input_char('b');
        tree_select.input_char('a');
        tree_select.input_char('c');
        tree_select.input_char('k');
        assert_eq!(tree_select.input, "back");
        assert_eq!(tree_select.filtered_indices.len(), 1);
    }

    #[test]
    fn test_backspace() {
        let items = create_test_items();
        let mut tree_select = TreeSelectState::new(items);

        tree_select.input_char('a');
        tree_select.input_char('b');
        tree_select.backspace();
        assert_eq!(tree_select.input, "a");
    }

    #[test]
    fn test_filter_resets_selection() {
        let items = create_test_items();
        let mut tree_select = TreeSelectState::new(items);

        tree_select.selected_index = 2;
        tree_select.input = "backend".to_string();
        tree_select.filter();
        // Only 1 item matches, so selected_index should be reset to 0
        assert_eq!(tree_select.selected_index, 0);
    }

    #[test]
    fn test_fuzzy_matching_with_typo() {
        let items = vec![
            TreeSelectItem {
                id: "1".to_string(),
                title: "Database migration".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
            TreeSelectItem {
                id: "2".to_string(),
                title: "Frontend refactor".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
            TreeSelectItem {
                id: "3".to_string(),
                title: "Backend API".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
        ];
        let mut tree_select = TreeSelectState::new(items);

        // Search for "migrat" - should match "Database migration"
        tree_select.input = "migrat".to_string();
        tree_select.filter();

        // Should find "Database migration"
        assert!(
            !tree_select.filtered_indices.is_empty(),
            "Should find items containing the search term"
        );
        let first_match = &tree_select.all_items[tree_select.filtered_indices[0]];
        assert_eq!(first_match.title, "Database migration");
    }

    #[test]
    fn test_fuzzy_matching_sorted_by_score() {
        let items = vec![
            TreeSelectItem {
                id: "1".to_string(),
                title: "Completely different thing".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
            TreeSelectItem {
                id: "2".to_string(),
                title: "Backend tasks".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
            TreeSelectItem {
                id: "3".to_string(),
                title: "Backend API work".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
        ];
        let mut tree_select = TreeSelectState::new(items);

        tree_select.input = "backend".to_string();
        tree_select.filter();

        // Should find both backend items, sorted by score
        assert!(tree_select.filtered_indices.len() >= 2);
        let first = &tree_select.all_items[tree_select.filtered_indices[0]];
        let second = &tree_select.all_items[tree_select.filtered_indices[1]];

        // Both should contain "backend"
        assert!(first.title.to_lowercase().contains("backend"));
        assert!(second.title.to_lowercase().contains("backend"));
    }

    #[test]
    fn test_max_suggestions_limit_with_typed_input() {
        // Create more than MAX_SUGGESTIONS items
        let items: Vec<TreeSelectItem> = (0..20)
            .map(|i| TreeSelectItem {
                id: format!("task-{i}"),
                title: format!("Task {i}"),
                depth: 0,
                status_indicator: ' ',
            })
            .collect();
        let mut tree_select = TreeSelectState::new(items);

        // Empty input should show ALL results (UI handles scrolling/pagination)
        tree_select.input.clear();
        tree_select.filter();

        assert_eq!(
            tree_select.filtered_indices.len(),
            20,
            "Empty input should show all items"
        );

        // Partial match that would match many items - still limited
        tree_select.input = "Task".to_string();
        tree_select.filter();

        assert!(
            tree_select.filtered_indices.len() <= MAX_SUGGESTIONS,
            "Typed input should limit to MAX_SUGGESTIONS"
        );
    }

    #[test]
    fn test_substring_matches_score_higher_than_fuzzy() {
        let items = vec![
            TreeSelectItem {
                id: "1".to_string(),
                title: "Setup database connection".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
            TreeSelectItem {
                id: "2".to_string(),
                title: "Databse backup script".to_string(), // typo
                depth: 0,
                status_indicator: ' ',
            },
            TreeSelectItem {
                id: "3".to_string(),
                title: "Something else entirely".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
        ];
        let mut tree_select = TreeSelectState::new(items);

        tree_select.input = "database".to_string();
        tree_select.filter();

        assert!(!tree_select.filtered_indices.is_empty());

        // First result should be the exact substring match
        let first_match = &tree_select.all_items[tree_select.filtered_indices[0]];
        assert_eq!(
            first_match.title, "Setup database connection",
            "Exact substring match should rank higher than fuzzy match"
        );
    }

    #[test]
    fn test_fuzzy_matching_case_insensitive() {
        let items = vec![
            TreeSelectItem {
                id: "1".to_string(),
                title: "BACKEND TASKS".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
            TreeSelectItem {
                id: "2".to_string(),
                title: "frontend work".to_string(),
                depth: 0,
                status_indicator: ' ',
            },
        ];
        let mut tree_select = TreeSelectState::new(items);

        // Lowercase query should match uppercase title
        tree_select.input = "backend".to_string();
        tree_select.filter();

        assert_eq!(tree_select.filtered_indices.len(), 1);
        let match_item = &tree_select.all_items[tree_select.filtered_indices[0]];
        assert_eq!(match_item.title, "BACKEND TASKS");
    }

    #[test]
    fn test_empty_filter_shows_all_results() {
        let items: Vec<TreeSelectItem> = (0..20)
            .map(|i| TreeSelectItem {
                id: format!("{i}"),
                title: format!("Task {i}"),
                depth: 0,
                status_indicator: ' ',
            })
            .collect();
        let mut tree_select = TreeSelectState::new(items);

        tree_select.filter();

        assert_eq!(
            tree_select.filtered_indices.len(),
            20,
            "Empty filter should show all items"
        );
    }
}
