//! Multi-line text input component

/// State for multi-line text input
///
/// Provides multi-line text editing with cursor movement, line management,
/// and scrolling support.
///
/// # Examples
///
/// ```
/// use lash_tui::components::TextAreaState;
///
/// let mut text_area = TextAreaState::new();
/// text_area.input_char('H');
/// text_area.input_char('i');
/// text_area.newline();
/// text_area.input_char('!');
/// assert_eq!(text_area.get_text(), "Hi\n!");
/// ```
#[derive(Debug, Clone)]
pub struct TextAreaState {
    /// Lines of text
    pub lines: Vec<String>,
    /// Cursor row (line index)
    pub cursor_row: usize,
    /// Cursor column (character index in current line)
    pub cursor_col: usize,
    /// Scroll offset for display
    pub scroll_offset: usize,
    /// Max visible rows
    pub max_visible_rows: usize,
}

impl TextAreaState {
    /// Create a new empty text area
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let text_area = TextAreaState::new();
    /// assert_eq!(text_area.line_count(), 1);
    /// assert!(text_area.get_text().is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            max_visible_rows: 10,
        }
    }

    /// Create a text area with a specific max visible rows
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let text_area = TextAreaState::with_max_rows(20);
    /// assert_eq!(text_area.max_visible_rows, 20);
    /// ```
    #[must_use]
    pub fn with_max_rows(max_rows: usize) -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            max_visible_rows: max_rows,
        }
    }

    /// Insert a character at the cursor position
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.input_char('a');
    /// text_area.input_char('b');
    /// assert_eq!(text_area.get_text(), "ab");
    /// ```
    pub fn input_char(&mut self, c: char) {
        if self.cursor_row >= self.lines.len() {
            self.lines.push(String::new());
            self.cursor_row = self.lines.len() - 1;
        }

        let byte_pos = self.char_to_byte_index(self.cursor_row, self.cursor_col);
        self.lines[self.cursor_row].insert(byte_pos, c);
        self.cursor_col += 1;
    }

    /// Insert a newline at the cursor position
    ///
    /// Splits the current line at the cursor position.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.input_char('a');
    /// text_area.input_char('b');
    /// text_area.newline();
    /// text_area.input_char('c');
    /// assert_eq!(text_area.get_text(), "ab\nc");
    /// ```
    pub fn newline(&mut self) {
        if self.cursor_row >= self.lines.len() {
            self.lines.push(String::new());
            self.cursor_row = self.lines.len() - 1;
        }

        let byte_pos = self.char_to_byte_index(self.cursor_row, self.cursor_col);
        let current_line = &self.lines[self.cursor_row];
        let rest = current_line[byte_pos..].to_string();
        self.lines[self.cursor_row].truncate(byte_pos);

        // Insert new line after current
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;

        // Adjust scroll if needed
        if self.cursor_row >= self.scroll_offset + self.max_visible_rows {
            self.scroll_offset = self.cursor_row - self.max_visible_rows + 1;
        }
    }

    /// Delete character before cursor (backspace)
    ///
    /// If at the beginning of a line, joins with the previous line.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.input_char('a');
    /// text_area.input_char('b');
    /// text_area.backspace();
    /// assert_eq!(text_area.get_text(), "a");
    /// ```
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            // Delete character in current line
            self.cursor_col -= 1;
            let byte_pos = self.char_to_byte_index(self.cursor_row, self.cursor_col);
            self.lines[self.cursor_row].remove(byte_pos);
        } else if self.cursor_row > 0 {
            // Join with previous line
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].chars().count();
            self.lines[self.cursor_row].push_str(&current_line);

            // Adjust scroll if needed
            if self.cursor_row < self.scroll_offset {
                self.scroll_offset = self.cursor_row;
            }
        }
    }

    /// Delete character at cursor (delete key)
    ///
    /// If at the end of a line, joins with the next line.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("hello");
    /// text_area.home();
    /// text_area.delete();
    /// assert_eq!(text_area.get_text(), "ello");
    /// ```
    pub fn delete(&mut self) {
        if self.cursor_row >= self.lines.len() {
            return;
        }

        let line_len = self.lines[self.cursor_row].chars().count();
        if self.cursor_col < line_len {
            // Delete character in current line
            let byte_pos = self.char_to_byte_index(self.cursor_row, self.cursor_col);
            self.lines[self.cursor_row].remove(byte_pos);
        } else if self.cursor_row + 1 < self.lines.len() {
            // Join with next line
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next_line);
        }
    }

    /// Move cursor up one line
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("line1\nline2");
    /// text_area.end();
    /// text_area.cursor_up();
    /// assert_eq!(text_area.cursor_row, 0);
    /// ```
    pub fn cursor_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            // Clamp cursor column to line length
            let line_len = self.lines[self.cursor_row].chars().count();
            self.cursor_col = self.cursor_col.min(line_len);

            // Adjust scroll if needed
            if self.cursor_row < self.scroll_offset {
                self.scroll_offset = self.cursor_row;
            }
        }
    }

    /// Move cursor down one line
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("line1\nline2");
    /// text_area.home();
    /// text_area.cursor_down();
    /// assert_eq!(text_area.cursor_row, 1);
    /// ```
    pub fn cursor_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            // Clamp cursor column to line length
            let line_len = self.lines[self.cursor_row].chars().count();
            self.cursor_col = self.cursor_col.min(line_len);

            // Adjust scroll if needed
            if self.cursor_row >= self.scroll_offset + self.max_visible_rows {
                self.scroll_offset = self.cursor_row - self.max_visible_rows + 1;
            }
        }
    }

    /// Move cursor left one character
    ///
    /// Wraps to previous line if at beginning of line.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("hi");
    /// text_area.end();
    /// text_area.cursor_left();
    /// assert_eq!(text_area.cursor_col, 1);
    /// ```
    pub fn cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            // Move to end of previous line
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].chars().count();

            // Adjust scroll if needed
            if self.cursor_row < self.scroll_offset {
                self.scroll_offset = self.cursor_row;
            }
        }
    }

    /// Move cursor right one character
    ///
    /// Wraps to next line if at end of line.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("hi");
    /// text_area.home();
    /// text_area.cursor_right();
    /// assert_eq!(text_area.cursor_col, 1);
    /// ```
    pub fn cursor_right(&mut self) {
        if self.cursor_row >= self.lines.len() {
            return;
        }

        let line_len = self.lines[self.cursor_row].chars().count();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            // Move to beginning of next line
            self.cursor_row += 1;
            self.cursor_col = 0;

            // Adjust scroll if needed
            if self.cursor_row >= self.scroll_offset + self.max_visible_rows {
                self.scroll_offset = self.cursor_row - self.max_visible_rows + 1;
            }
        }
    }

    /// Move cursor to beginning of current line
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("hello");
    /// text_area.end();
    /// text_area.home();
    /// assert_eq!(text_area.cursor_col, 0);
    /// ```
    pub fn home(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of current line
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("hello");
    /// text_area.end();
    /// assert_eq!(text_area.cursor_col, 5);
    /// ```
    pub fn end(&mut self) {
        if self.cursor_row < self.lines.len() {
            self.cursor_col = self.lines[self.cursor_row].chars().count();
        }
    }

    /// Get the text as a single string with newlines
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("line1\nline2\nline3");
    /// assert_eq!(text_area.get_text(), "line1\nline2\nline3");
    /// ```
    #[must_use]
    pub fn get_text(&self) -> String {
        self.lines.join("\n")
    }

    /// Set the text from a string
    ///
    /// Splits on newlines and moves cursor to end.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("line1\nline2");
    /// assert_eq!(text_area.line_count(), 2);
    /// ```
    pub fn set_text(&mut self, text: &str) {
        self.lines = text.lines().map(String::from).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = self.lines[self.cursor_row].chars().count();
        self.scroll_offset = 0;
    }

    /// Clear all text
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("hello");
    /// text_area.clear();
    /// assert!(text_area.get_text().is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    /// Get the number of lines
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextAreaState;
    ///
    /// let mut text_area = TextAreaState::new();
    /// text_area.set_text("line1\nline2\nline3");
    /// assert_eq!(text_area.line_count(), 3);
    /// ```
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Convert character position to byte index for a given line
    fn char_to_byte_index(&self, row: usize, char_pos: usize) -> usize {
        if row >= self.lines.len() {
            return 0;
        }
        self.lines[row]
            .char_indices()
            .nth(char_pos)
            .map_or(self.lines[row].len(), |(idx, _)| idx)
    }
}

impl Default for TextAreaState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let text_area = TextAreaState::new();
        assert_eq!(text_area.line_count(), 1);
        assert!(text_area.get_text().is_empty());
        assert_eq!(text_area.cursor_row, 0);
        assert_eq!(text_area.cursor_col, 0);
    }

    #[test]
    fn test_input_char() {
        let mut text_area = TextAreaState::new();
        text_area.input_char('h');
        text_area.input_char('i');
        assert_eq!(text_area.get_text(), "hi");
    }

    #[test]
    fn test_newline() {
        let mut text_area = TextAreaState::new();
        text_area.input_char('a');
        text_area.newline();
        text_area.input_char('b');
        assert_eq!(text_area.get_text(), "a\nb");
        assert_eq!(text_area.cursor_row, 1);
    }

    #[test]
    fn test_backspace() {
        let mut text_area = TextAreaState::new();
        text_area.input_char('a');
        text_area.input_char('b');
        text_area.backspace();
        assert_eq!(text_area.get_text(), "a");
    }

    #[test]
    fn test_backspace_join_lines() {
        let mut text_area = TextAreaState::new();
        text_area.input_char('a');
        text_area.newline();
        text_area.input_char('b');
        text_area.home();
        text_area.backspace();
        assert_eq!(text_area.get_text(), "ab");
        assert_eq!(text_area.cursor_row, 0);
    }

    #[test]
    fn test_delete() {
        let mut text_area = TextAreaState::new();
        text_area.set_text("hello");
        text_area.home();
        text_area.delete();
        assert_eq!(text_area.get_text(), "ello");
    }

    #[test]
    fn test_delete_join_lines() {
        let mut text_area = TextAreaState::new();
        text_area.set_text("a\nb");
        // Move to end of first line
        text_area.cursor_row = 0;
        text_area.end();
        text_area.delete();
        assert_eq!(text_area.get_text(), "ab");
    }

    #[test]
    fn test_cursor_navigation() {
        let mut text_area = TextAreaState::new();
        text_area.set_text("line1\nline2");

        // End puts us at end of last line
        assert_eq!(text_area.cursor_row, 1);
        assert_eq!(text_area.cursor_col, 5);

        text_area.cursor_up();
        assert_eq!(text_area.cursor_row, 0);

        text_area.home();
        assert_eq!(text_area.cursor_col, 0);

        text_area.cursor_right();
        assert_eq!(text_area.cursor_col, 1);

        text_area.cursor_left();
        assert_eq!(text_area.cursor_col, 0);
    }

    #[test]
    fn test_set_text() {
        let mut text_area = TextAreaState::new();
        text_area.set_text("line1\nline2\nline3");
        assert_eq!(text_area.line_count(), 3);
        assert_eq!(text_area.cursor_row, 2);
    }

    #[test]
    fn test_clear() {
        let mut text_area = TextAreaState::new();
        text_area.set_text("hello");
        text_area.clear();
        assert!(text_area.get_text().is_empty());
        assert_eq!(text_area.cursor_row, 0);
        assert_eq!(text_area.cursor_col, 0);
    }

    #[test]
    fn test_cursor_wrap_at_line_end() {
        let mut text_area = TextAreaState::new();
        text_area.set_text("ab\ncd");
        text_area.home(); // Go to start of last line
        text_area.cursor_row = 0;
        text_area.end();
        text_area.cursor_right(); // Should wrap to next line
        assert_eq!(text_area.cursor_row, 1);
        assert_eq!(text_area.cursor_col, 0);
    }

    #[test]
    fn test_cursor_wrap_at_line_start() {
        let mut text_area = TextAreaState::new();
        text_area.set_text("ab\ncd");
        text_area.cursor_row = 1;
        text_area.home();
        text_area.cursor_left(); // Should wrap to end of previous line
        assert_eq!(text_area.cursor_row, 0);
        assert_eq!(text_area.cursor_col, 2);
    }

    #[test]
    fn test_unicode_handling() {
        let mut text_area = TextAreaState::new();
        text_area.input_char('你');
        text_area.input_char('好');
        assert_eq!(text_area.get_text(), "你好");
        text_area.backspace();
        assert_eq!(text_area.get_text(), "你");
    }
}
