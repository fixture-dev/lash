//! Multi-select list component for choosing dependencies

use std::collections::HashSet;

/// A selectable option in a multi-select list
///
/// # Examples
///
/// ```
/// use lash_tui::components::MultiSelectOption;
///
/// let option = MultiSelectOption {
///     id: "task-1".to_string(),
///     label: "Implement auth".to_string(),
///     description: Some("tasks/backend.md".to_string()),
/// };
/// assert_eq!(option.id, "task-1");
/// ```
#[derive(Debug, Clone)]
pub struct MultiSelectOption {
    /// Unique identifier for this option
    pub id: String,
    /// Display label
    pub label: String,
    /// Optional description (e.g., file path)
    pub description: Option<String>,
}

/// State for multi-select list
///
/// Allows selecting multiple items from a searchable list.
/// Commonly used for selecting task dependencies.
///
/// # Examples
///
/// ```
/// use lash_tui::components::{MultiSelectState, MultiSelectOption};
///
/// let options = vec![
///     MultiSelectOption {
///         id: "1".to_string(),
///         label: "Task 1".to_string(),
///         description: None,
///     },
///     MultiSelectOption {
///         id: "2".to_string(),
///         label: "Task 2".to_string(),
///         description: None,
///     },
/// ];
/// let mut multi_select = MultiSelectState::new(options);
/// multi_select.toggle_highlighted();
/// assert_eq!(multi_select.get_selected().len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct MultiSelectState {
    /// Search filter input
    pub input: String,
    /// All options
    pub all_options: Vec<MultiSelectOption>,
    /// Filtered indices (into `all_options`)
    pub filtered_indices: Vec<usize>,
    /// Selected indices (in `all_options`, not filtered list)
    pub selected_indices: HashSet<usize>,
    /// Currently highlighted index in filtered list
    pub highlighted_index: usize,
}

impl MultiSelectState {
    /// Create a new multi-select with the given options
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{MultiSelectState, MultiSelectOption};
    ///
    /// let options = vec![
    ///     MultiSelectOption {
    ///         id: "1".to_string(),
    ///         label: "Option 1".to_string(),
    ///         description: None,
    ///     },
    /// ];
    /// let multi_select = MultiSelectState::new(options);
    /// assert_eq!(multi_select.all_options.len(), 1);
    /// assert!(multi_select.selected_indices.is_empty());
    /// ```
    #[must_use]
    pub fn new(options: Vec<MultiSelectOption>) -> Self {
        let filtered_indices: Vec<usize> = (0..options.len()).collect();
        Self {
            input: String::new(),
            all_options: options,
            filtered_indices,
            selected_indices: HashSet::new(),
            highlighted_index: 0,
        }
    }

    /// Filter options based on current input
    ///
    /// Updates `filtered_indices` to match options whose labels contain
    /// the input string (case-insensitive).
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{MultiSelectState, MultiSelectOption};
    ///
    /// let options = vec![
    ///     MultiSelectOption {
    ///         id: "1".to_string(),
    ///         label: "Backend task".to_string(),
    ///         description: None,
    ///     },
    ///     MultiSelectOption {
    ///         id: "2".to_string(),
    ///         label: "Frontend task".to_string(),
    ///         description: None,
    ///     },
    /// ];
    /// let mut multi_select = MultiSelectState::new(options);
    /// multi_select.input = "back".to_string();
    /// multi_select.filter();
    /// assert_eq!(multi_select.filtered_indices.len(), 1);
    /// ```
    pub fn filter(&mut self) {
        if self.input.is_empty() {
            self.filtered_indices = (0..self.all_options.len()).collect();
        } else {
            let input_lower = self.input.to_lowercase();
            self.filtered_indices = self
                .all_options
                .iter()
                .enumerate()
                .filter(|(_, opt)| opt.label.to_lowercase().contains(&input_lower))
                .map(|(idx, _)| idx)
                .collect();
        }

        // Reset highlighted index if out of bounds
        if self.highlighted_index >= self.filtered_indices.len() {
            self.highlighted_index = 0;
        }
    }

    /// Highlight the next item in the filtered list
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{MultiSelectState, MultiSelectOption};
    ///
    /// let options = vec![
    ///     MultiSelectOption {
    ///         id: "1".to_string(),
    ///         label: "Task 1".to_string(),
    ///         description: None,
    ///     },
    ///     MultiSelectOption {
    ///         id: "2".to_string(),
    ///         label: "Task 2".to_string(),
    ///         description: None,
    ///     },
    /// ];
    /// let mut multi_select = MultiSelectState::new(options);
    /// multi_select.highlight_next();
    /// assert_eq!(multi_select.highlighted_index, 1);
    /// ```
    pub fn highlight_next(&mut self) {
        if !self.filtered_indices.is_empty()
            && self.highlighted_index + 1 < self.filtered_indices.len()
        {
            self.highlighted_index += 1;
        }
    }

    /// Highlight the previous item in the filtered list
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{MultiSelectState, MultiSelectOption};
    ///
    /// let options = vec![
    ///     MultiSelectOption {
    ///         id: "1".to_string(),
    ///         label: "Task 1".to_string(),
    ///         description: None,
    ///     },
    ///     MultiSelectOption {
    ///         id: "2".to_string(),
    ///         label: "Task 2".to_string(),
    ///         description: None,
    ///     },
    /// ];
    /// let mut multi_select = MultiSelectState::new(options);
    /// multi_select.highlighted_index = 1;
    /// multi_select.highlight_prev();
    /// assert_eq!(multi_select.highlighted_index, 0);
    /// ```
    pub fn highlight_prev(&mut self) {
        if self.highlighted_index > 0 {
            self.highlighted_index -= 1;
        }
    }

    /// Toggle selection of the currently highlighted item
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{MultiSelectState, MultiSelectOption};
    ///
    /// let options = vec![
    ///     MultiSelectOption {
    ///         id: "1".to_string(),
    ///         label: "Task 1".to_string(),
    ///         description: None,
    ///     },
    /// ];
    /// let mut multi_select = MultiSelectState::new(options);
    /// multi_select.toggle_highlighted();
    /// assert_eq!(multi_select.selected_indices.len(), 1);
    /// multi_select.toggle_highlighted();
    /// assert_eq!(multi_select.selected_indices.len(), 0);
    /// ```
    pub fn toggle_highlighted(&mut self) {
        if !self.filtered_indices.is_empty() && self.highlighted_index < self.filtered_indices.len()
        {
            let option_index = self.filtered_indices[self.highlighted_index];
            if self.selected_indices.contains(&option_index) {
                self.selected_indices.remove(&option_index);
            } else {
                self.selected_indices.insert(option_index);
            }
        }
    }

    /// Get all selected options
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{MultiSelectState, MultiSelectOption};
    ///
    /// let options = vec![
    ///     MultiSelectOption {
    ///         id: "1".to_string(),
    ///         label: "Task 1".to_string(),
    ///         description: None,
    ///     },
    ///     MultiSelectOption {
    ///         id: "2".to_string(),
    ///         label: "Task 2".to_string(),
    ///         description: None,
    ///     },
    /// ];
    /// let mut multi_select = MultiSelectState::new(options);
    /// multi_select.toggle_highlighted();
    /// let selected = multi_select.get_selected();
    /// assert_eq!(selected.len(), 1);
    /// assert_eq!(selected[0].id, "1");
    /// ```
    #[must_use]
    pub fn get_selected(&self) -> Vec<&MultiSelectOption> {
        let mut selected: Vec<&MultiSelectOption> = self
            .selected_indices
            .iter()
            .filter_map(|&idx| self.all_options.get(idx))
            .collect();
        // Sort by original index for consistent ordering
        selected.sort_by_key(|opt| {
            self.all_options
                .iter()
                .position(|o| o.id == opt.id)
                .unwrap_or(0)
        });
        selected
    }

    /// Insert a character into the input field
    ///
    /// Automatically filters options after inserting.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{MultiSelectState, MultiSelectOption};
    ///
    /// let options = vec![
    ///     MultiSelectOption {
    ///         id: "1".to_string(),
    ///         label: "Task 1".to_string(),
    ///         description: None,
    ///     },
    /// ];
    /// let mut multi_select = MultiSelectState::new(options);
    /// multi_select.input_char('t');
    /// multi_select.input_char('a');
    /// assert_eq!(multi_select.input, "ta");
    /// ```
    pub fn input_char(&mut self, c: char) {
        self.input.push(c);
        self.filter();
    }

    /// Delete character before cursor (backspace)
    ///
    /// Automatically filters options after deleting.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{MultiSelectState, MultiSelectOption};
    ///
    /// let options = vec![
    ///     MultiSelectOption {
    ///         id: "1".to_string(),
    ///         label: "Task 1".to_string(),
    ///         description: None,
    ///     },
    /// ];
    /// let mut multi_select = MultiSelectState::new(options);
    /// multi_select.input_char('a');
    /// multi_select.input_char('b');
    /// multi_select.backspace();
    /// assert_eq!(multi_select.input, "a");
    /// ```
    pub fn backspace(&mut self) {
        if !self.input.is_empty() {
            self.input.pop();
            self.filter();
        }
    }
}

impl Default for MultiSelectState {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_options() -> Vec<MultiSelectOption> {
        vec![
            MultiSelectOption {
                id: "1".to_string(),
                label: "Backend task".to_string(),
                description: Some("tasks/backend.md".to_string()),
            },
            MultiSelectOption {
                id: "2".to_string(),
                label: "Frontend task".to_string(),
                description: Some("tasks/frontend.md".to_string()),
            },
            MultiSelectOption {
                id: "3".to_string(),
                label: "Database task".to_string(),
                description: Some("tasks/database.md".to_string()),
            },
        ]
    }

    #[test]
    fn test_new() {
        let options = create_test_options();
        let multi_select = MultiSelectState::new(options.clone());
        assert_eq!(multi_select.all_options.len(), 3);
        assert_eq!(multi_select.filtered_indices.len(), 3);
        assert!(multi_select.selected_indices.is_empty());
        assert_eq!(multi_select.highlighted_index, 0);
    }

    #[test]
    fn test_filter() {
        let options = create_test_options();
        let mut multi_select = MultiSelectState::new(options);

        multi_select.input = "backend".to_string();
        multi_select.filter();
        assert_eq!(multi_select.filtered_indices.len(), 1);

        multi_select.input.clear();
        multi_select.filter();
        assert_eq!(multi_select.filtered_indices.len(), 3);
    }

    #[test]
    fn test_highlight_navigation() {
        let options = create_test_options();
        let mut multi_select = MultiSelectState::new(options);

        assert_eq!(multi_select.highlighted_index, 0);
        multi_select.highlight_next();
        assert_eq!(multi_select.highlighted_index, 1);
        multi_select.highlight_next();
        assert_eq!(multi_select.highlighted_index, 2);
        multi_select.highlight_next(); // Should stay at 2
        assert_eq!(multi_select.highlighted_index, 2);

        multi_select.highlight_prev();
        assert_eq!(multi_select.highlighted_index, 1);
    }

    #[test]
    fn test_toggle_selection() {
        let options = create_test_options();
        let mut multi_select = MultiSelectState::new(options);

        // Select first item
        multi_select.toggle_highlighted();
        assert_eq!(multi_select.selected_indices.len(), 1);
        assert!(multi_select.selected_indices.contains(&0));

        // Deselect first item
        multi_select.toggle_highlighted();
        assert_eq!(multi_select.selected_indices.len(), 0);

        // Select multiple items
        multi_select.toggle_highlighted();
        multi_select.highlight_next();
        multi_select.toggle_highlighted();
        assert_eq!(multi_select.selected_indices.len(), 2);
    }

    #[test]
    fn test_get_selected() {
        let options = create_test_options();
        let mut multi_select = MultiSelectState::new(options);

        multi_select.toggle_highlighted();
        multi_select.highlight_next();
        multi_select.highlight_next();
        multi_select.toggle_highlighted();

        let selected = multi_select.get_selected();
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].id, "1");
        assert_eq!(selected[1].id, "3");
    }

    #[test]
    fn test_input_char() {
        let options = create_test_options();
        let mut multi_select = MultiSelectState::new(options);

        multi_select.input_char('b');
        multi_select.input_char('a');
        multi_select.input_char('c');
        multi_select.input_char('k');
        assert_eq!(multi_select.input, "back");
        assert_eq!(multi_select.filtered_indices.len(), 1);
    }

    #[test]
    fn test_backspace() {
        let options = create_test_options();
        let mut multi_select = MultiSelectState::new(options);

        multi_select.input_char('a');
        multi_select.input_char('b');
        multi_select.backspace();
        assert_eq!(multi_select.input, "a");
    }

    #[test]
    fn test_filter_preserves_selection() {
        let options = create_test_options();
        let mut multi_select = MultiSelectState::new(options);

        // Select first item
        multi_select.toggle_highlighted();
        assert!(multi_select.selected_indices.contains(&0));

        // Filter should preserve selection
        multi_select.input = "frontend".to_string();
        multi_select.filter();
        assert!(multi_select.selected_indices.contains(&0));
    }

    #[test]
    fn test_filter_resets_highlight() {
        let options = create_test_options();
        let mut multi_select = MultiSelectState::new(options);

        multi_select.highlighted_index = 2;
        multi_select.input = "backend".to_string();
        multi_select.filter();
        // Only 1 item matches, so highlighted_index should be reset to 0
        assert_eq!(multi_select.highlighted_index, 0);
    }
}
