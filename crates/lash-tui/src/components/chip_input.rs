//! Chip input component for managing tags/labels

/// State for an input field with chips (tags)
///
/// Allows users to enter multiple values as "chips" or "tags". Chips can be
/// added by typing and pressing Enter/comma, and removed by backspacing when
/// the input is empty or by focusing and deleting individual chips.
///
/// # Examples
///
/// ```
/// use lash_tui::components::ChipInputState;
///
/// let mut chip_input = ChipInputState::new();
/// chip_input.input_char('t');
/// chip_input.input_char('e');
/// chip_input.input_char('s');
/// chip_input.input_char('t');
/// chip_input.add_chip();
/// assert_eq!(chip_input.get_chips(), &["test"]);
/// assert!(chip_input.input.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct ChipInputState {
    /// Committed chips (labels)
    pub chips: Vec<String>,
    /// Current partial input
    pub input: String,
    /// Cursor position in input (character index)
    pub cursor_position: usize,
    /// Currently focused chip for deletion (None if input focused)
    pub focused_chip: Option<usize>,
    /// Usage counts for suggestions (label name -> task count)
    pub suggestion_counts: std::collections::HashMap<String, i64>,
}

impl ChipInputState {
    /// Create a new empty chip input
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::ChipInputState;
    ///
    /// let chip_input = ChipInputState::new();
    /// assert!(chip_input.chips.is_empty());
    /// assert!(chip_input.input.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            chips: Vec::new(),
            input: String::new(),
            cursor_position: 0,
            focused_chip: None,
            suggestion_counts: std::collections::HashMap::new(),
        }
    }

    /// Add current input as a chip
    ///
    /// Commits the current input as a chip if it's not empty and not a duplicate.
    /// Clears the input and resets cursor after adding.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::ChipInputState;
    ///
    /// let mut chip_input = ChipInputState::new();
    /// chip_input.input_char('t');
    /// chip_input.input_char('a');
    /// chip_input.input_char('g');
    /// chip_input.add_chip();
    /// assert_eq!(chip_input.get_chips(), &["tag"]);
    /// assert!(chip_input.input.is_empty());
    /// ```
    pub fn add_chip(&mut self) {
        let trimmed = self.input.trim().to_string();
        if !trimmed.is_empty() && !self.chips.contains(&trimmed) {
            self.chips.push(trimmed);
            self.input.clear();
            self.cursor_position = 0;
            self.focused_chip = None;
        }
    }

    /// Remove chip at specific index
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::ChipInputState;
    ///
    /// let mut chip_input = ChipInputState::new();
    /// chip_input.input = "tag1".to_string();
    /// chip_input.add_chip();
    /// chip_input.input = "tag2".to_string();
    /// chip_input.add_chip();
    /// chip_input.remove_chip(0);
    /// assert_eq!(chip_input.get_chips(), &["tag2"]);
    /// ```
    pub fn remove_chip(&mut self, index: usize) {
        if index < self.chips.len() {
            self.chips.remove(index);
            // Adjust focused chip if needed
            if let Some(focused) = self.focused_chip {
                if focused >= self.chips.len() {
                    self.focused_chip = if self.chips.is_empty() {
                        None
                    } else {
                        Some(self.chips.len() - 1)
                    };
                }
            }
        }
    }

    /// Remove the currently focused chip
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::ChipInputState;
    ///
    /// let mut chip_input = ChipInputState::new();
    /// chip_input.input = "tag1".to_string();
    /// chip_input.add_chip();
    /// chip_input.focus_prev_chip();
    /// chip_input.remove_focused_chip();
    /// assert!(chip_input.chips.is_empty());
    /// ```
    pub fn remove_focused_chip(&mut self) {
        if let Some(index) = self.focused_chip {
            self.remove_chip(index);
        }
    }

    /// Focus the previous chip (moving left)
    ///
    /// If input is currently focused and has content, does nothing.
    /// If input is empty, focuses the last chip.
    /// If a chip is focused, focuses the previous chip.
    pub fn focus_prev_chip(&mut self) {
        if self.focused_chip.is_none() {
            // Currently on input, move to last chip if input is empty
            if self.input.is_empty() && !self.chips.is_empty() {
                self.focused_chip = Some(self.chips.len() - 1);
            }
        } else if let Some(focused) = self.focused_chip {
            if focused > 0 {
                self.focused_chip = Some(focused - 1);
            }
        }
    }

    /// Focus the next chip (moving right)
    ///
    /// If a chip is focused, focuses the next chip or returns to input
    /// if at the end of the chip list.
    pub fn focus_next_chip(&mut self) {
        if let Some(focused) = self.focused_chip {
            if focused + 1 < self.chips.len() {
                self.focused_chip = Some(focused + 1);
            } else {
                self.focused_chip = None; // Back to input
            }
        }
    }

    /// Focus the input field
    ///
    /// Moves focus away from any chip to the input field.
    pub fn focus_input(&mut self) {
        self.focused_chip = None;
    }

    /// Insert a character at cursor position in input
    ///
    /// Automatically focuses the input if a chip was focused.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::ChipInputState;
    ///
    /// let mut chip_input = ChipInputState::new();
    /// chip_input.input_char('a');
    /// chip_input.input_char('b');
    /// assert_eq!(chip_input.input, "ab");
    /// ```
    pub fn input_char(&mut self, c: char) {
        self.focused_chip = None; // Focus input when typing
        let byte_pos = self.char_to_byte_index(self.cursor_position);
        self.input.insert(byte_pos, c);
        self.cursor_position += 1;
    }

    /// Delete character before cursor (backspace)
    ///
    /// If input is empty, focuses the last chip instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::ChipInputState;
    ///
    /// let mut chip_input = ChipInputState::new();
    /// chip_input.input_char('a');
    /// chip_input.input_char('b');
    /// chip_input.backspace();
    /// assert_eq!(chip_input.input, "a");
    /// ```
    pub fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            let byte_pos = self.char_to_byte_index(self.cursor_position);
            self.input.remove(byte_pos);
        } else if self.input.is_empty() {
            // Focus last chip when backspacing on empty input
            self.focus_prev_chip();
        }
    }

    /// Get all chips as a slice
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::ChipInputState;
    ///
    /// let mut chip_input = ChipInputState::new();
    /// chip_input.input = "tag1".to_string();
    /// chip_input.add_chip();
    /// chip_input.input = "tag2".to_string();
    /// chip_input.add_chip();
    /// assert_eq!(chip_input.get_chips(), &["tag1", "tag2"]);
    /// ```
    #[must_use]
    pub fn get_chips(&self) -> &[String] {
        &self.chips
    }

    /// Set usage counts for autocomplete suggestions
    ///
    /// This allows displaying how many tasks use each label when showing suggestions.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::ChipInputState;
    /// use std::collections::HashMap;
    ///
    /// let mut chip_input = ChipInputState::new();
    /// let mut counts = HashMap::new();
    /// counts.insert("backend".to_string(), 15);
    /// counts.insert("frontend".to_string(), 8);
    /// chip_input.set_suggestion_counts(counts);
    /// assert_eq!(chip_input.get_suggestion_count("backend"), Some(15));
    /// assert_eq!(chip_input.get_suggestion_count("frontend"), Some(8));
    /// ```
    pub fn set_suggestion_counts(&mut self, counts: std::collections::HashMap<String, i64>) {
        self.suggestion_counts = counts;
    }

    /// Get the usage count for a specific suggestion
    ///
    /// Returns `None` if the label has no associated count.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::ChipInputState;
    /// use std::collections::HashMap;
    ///
    /// let mut chip_input = ChipInputState::new();
    /// let mut counts = HashMap::new();
    /// counts.insert("backend".to_string(), 15);
    /// chip_input.set_suggestion_counts(counts);
    /// assert_eq!(chip_input.get_suggestion_count("backend"), Some(15));
    /// assert_eq!(chip_input.get_suggestion_count("frontend"), None);
    /// ```
    #[must_use]
    pub fn get_suggestion_count(&self, label: &str) -> Option<i64> {
        self.suggestion_counts.get(label).copied()
    }

    /// Get all available suggestions with counts
    ///
    /// Returns a vector of (label, count) tuples, sorted by label name.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::ChipInputState;
    /// use std::collections::HashMap;
    ///
    /// let mut chip_input = ChipInputState::new();
    /// let mut counts = HashMap::new();
    /// counts.insert("backend".to_string(), 15);
    /// counts.insert("frontend".to_string(), 8);
    /// chip_input.set_suggestion_counts(counts);
    ///
    /// let suggestions = chip_input.get_suggestions_with_counts();
    /// assert_eq!(suggestions.len(), 2);
    /// // Results are sorted alphabetically
    /// assert_eq!(suggestions[0].0, "backend");
    /// assert_eq!(suggestions[1].0, "frontend");
    /// ```
    #[must_use]
    pub fn get_suggestions_with_counts(&self) -> Vec<(String, i64)> {
        let mut suggestions: Vec<_> = self
            .suggestion_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        suggestions.sort_by(|a, b| a.0.cmp(&b.0));
        suggestions
    }

    /// Convert character position to byte index
    ///
    /// Helper for working with Unicode strings.
    fn char_to_byte_index(&self, char_pos: usize) -> usize {
        self.input
            .char_indices()
            .nth(char_pos)
            .map_or(self.input.len(), |(idx, _)| idx)
    }
}

impl Default for ChipInputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_chip() {
        let mut chip_input = ChipInputState::new();
        chip_input.input = "tag1".to_string();
        chip_input.add_chip();
        assert_eq!(chip_input.get_chips(), &["tag1"]);
        assert!(chip_input.input.is_empty());
    }

    #[test]
    fn test_add_duplicate_chip() {
        let mut chip_input = ChipInputState::new();
        chip_input.input = "tag1".to_string();
        chip_input.add_chip();
        chip_input.input = "tag1".to_string();
        chip_input.add_chip();
        // Should only have one
        assert_eq!(chip_input.get_chips(), &["tag1"]);
    }

    #[test]
    fn test_add_empty_chip() {
        let mut chip_input = ChipInputState::new();
        chip_input.add_chip();
        assert!(chip_input.chips.is_empty());
    }

    #[test]
    fn test_remove_chip() {
        let mut chip_input = ChipInputState::new();
        chip_input.input = "tag1".to_string();
        chip_input.add_chip();
        chip_input.input = "tag2".to_string();
        chip_input.add_chip();
        chip_input.remove_chip(0);
        assert_eq!(chip_input.get_chips(), &["tag2"]);
    }

    #[test]
    fn test_focus_navigation() {
        let mut chip_input = ChipInputState::new();
        chip_input.input = "tag1".to_string();
        chip_input.add_chip();
        chip_input.input = "tag2".to_string();
        chip_input.add_chip();

        // Focus last chip
        chip_input.focus_prev_chip();
        assert_eq!(chip_input.focused_chip, Some(1));

        // Focus previous chip
        chip_input.focus_prev_chip();
        assert_eq!(chip_input.focused_chip, Some(0));

        // Focus next chip
        chip_input.focus_next_chip();
        assert_eq!(chip_input.focused_chip, Some(1));

        // Return to input
        chip_input.focus_next_chip();
        assert_eq!(chip_input.focused_chip, None);
    }

    #[test]
    fn test_backspace_on_empty_focuses_chip() {
        let mut chip_input = ChipInputState::new();
        chip_input.input = "tag1".to_string();
        chip_input.add_chip();
        chip_input.backspace();
        assert_eq!(chip_input.focused_chip, Some(0));
    }

    #[test]
    fn test_input_char_focuses_input() {
        let mut chip_input = ChipInputState::new();
        chip_input.input = "tag1".to_string();
        chip_input.add_chip();
        chip_input.focus_prev_chip();
        assert!(chip_input.focused_chip.is_some());

        chip_input.input_char('a');
        assert_eq!(chip_input.focused_chip, None);
        assert_eq!(chip_input.input, "a");
    }

    #[test]
    fn test_remove_focused_chip() {
        let mut chip_input = ChipInputState::new();
        chip_input.input = "tag1".to_string();
        chip_input.add_chip();
        chip_input.input = "tag2".to_string();
        chip_input.add_chip();

        chip_input.focus_prev_chip();
        chip_input.remove_focused_chip();
        assert_eq!(chip_input.get_chips(), &["tag1"]);
    }

    #[test]
    fn test_trimming_whitespace() {
        let mut chip_input = ChipInputState::new();
        chip_input.input = "  tag1  ".to_string();
        chip_input.add_chip();
        assert_eq!(chip_input.get_chips(), &["tag1"]);
    }

    #[test]
    fn test_get_suggestions_with_counts() {
        let mut chip_input = ChipInputState::new();
        let mut counts = std::collections::HashMap::new();
        counts.insert("backend".to_string(), 15);
        counts.insert("frontend".to_string(), 8);
        counts.insert("database".to_string(), 12);
        chip_input.set_suggestion_counts(counts);

        let suggestions = chip_input.get_suggestions_with_counts();
        assert_eq!(suggestions.len(), 3);

        // Should be sorted alphabetically
        assert_eq!(suggestions[0].0, "backend");
        assert_eq!(suggestions[0].1, 15);
        assert_eq!(suggestions[1].0, "database");
        assert_eq!(suggestions[1].1, 12);
        assert_eq!(suggestions[2].0, "frontend");
        assert_eq!(suggestions[2].1, 8);
    }

    #[test]
    fn test_filter_suggestions_by_prefix() {
        let mut chip_input = ChipInputState::new();
        let mut counts = std::collections::HashMap::new();
        counts.insert("backend".to_string(), 15);
        counts.insert("frontend".to_string(), 8);
        counts.insert("database".to_string(), 12);
        counts.insert("better".to_string(), 3);
        chip_input.set_suggestion_counts(counts);

        chip_input.input = "back".to_string();

        // Get all suggestions and filter by prefix
        let all_suggestions = chip_input.get_suggestions_with_counts();
        let input_lower = chip_input.input.to_lowercase();
        let filtered: Vec<_> = all_suggestions
            .into_iter()
            .filter(|(label, _)| label.to_lowercase().starts_with(&input_lower))
            .collect();

        assert_eq!(
            filtered.len(),
            1,
            "Should find 1 label starting with 'back'"
        );
        assert_eq!(filtered[0].0, "backend");
    }

    #[test]
    fn test_filter_suggestions_case_insensitive() {
        let mut chip_input = ChipInputState::new();
        let mut counts = std::collections::HashMap::new();
        counts.insert("Backend".to_string(), 15);
        counts.insert("FRONTEND".to_string(), 8);
        counts.insert("DataBase".to_string(), 12);
        chip_input.set_suggestion_counts(counts);

        chip_input.input = "back".to_string();

        // Get all suggestions and filter case-insensitively
        let all_suggestions = chip_input.get_suggestions_with_counts();
        let filtered: Vec<_> = all_suggestions
            .into_iter()
            .filter(|(label, _)| {
                label
                    .to_lowercase()
                    .contains(&chip_input.input.to_lowercase())
            })
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "Backend");
    }

    #[test]
    fn test_exclude_added_chips_from_suggestions() {
        let mut chip_input = ChipInputState::new();
        let mut counts = std::collections::HashMap::new();
        counts.insert("backend".to_string(), 15);
        counts.insert("frontend".to_string(), 8);
        counts.insert("database".to_string(), 12);
        chip_input.set_suggestion_counts(counts);

        // Add some chips
        chip_input.input = "backend".to_string();
        chip_input.add_chip();
        chip_input.input = "frontend".to_string();
        chip_input.add_chip();

        // Get suggestions excluding already-added chips
        let all_suggestions = chip_input.get_suggestions_with_counts();
        let filtered: Vec<_> = all_suggestions
            .into_iter()
            .filter(|(label, _)| !chip_input.chips.contains(label))
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "database");
    }

    #[test]
    fn test_suggestion_count_retrieval() {
        let mut chip_input = ChipInputState::new();
        let mut counts = std::collections::HashMap::new();
        counts.insert("backend".to_string(), 42);
        counts.insert("frontend".to_string(), 17);
        chip_input.set_suggestion_counts(counts);

        assert_eq!(chip_input.get_suggestion_count("backend"), Some(42));
        assert_eq!(chip_input.get_suggestion_count("frontend"), Some(17));
        assert_eq!(chip_input.get_suggestion_count("nonexistent"), None);
    }

    #[test]
    fn test_empty_suggestions() {
        let chip_input = ChipInputState::new();
        let suggestions = chip_input.get_suggestions_with_counts();
        assert!(suggestions.is_empty());
    }
}
