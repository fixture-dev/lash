//! Single-line text input component state

/// State for a single-line text input field
///
/// Provides basic text editing capabilities including cursor movement,
/// character insertion, and deletion. Supports placeholders and validation.
///
/// # Examples
///
/// ```
/// use lash_tui::components::TextInputState;
///
/// let mut input = TextInputState::new();
/// input.input_char('h');
/// input.input_char('i');
/// assert_eq!(input.value(), "hi");
/// assert_eq!(input.cursor_position, 2);
/// ```
#[derive(Debug, Clone)]
pub struct TextInputState {
    /// Current input value
    pub value: String,
    /// Cursor position (character index, not byte index)
    pub cursor_position: usize,
    /// Placeholder text when empty
    pub placeholder: String,
    /// Whether this field is required
    pub required: bool,
    /// Max length (0 = unlimited)
    pub max_length: usize,
}

impl TextInputState {
    /// Create a new empty text input
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let input = TextInputState::new();
    /// assert!(input.is_empty());
    /// assert_eq!(input.cursor_position, 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor_position: 0,
            placeholder: String::new(),
            required: false,
            max_length: 0,
        }
    }

    /// Create a text input with a placeholder
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let input = TextInputState::with_placeholder("Enter name...");
    /// assert_eq!(input.placeholder, "Enter name...");
    /// ```
    #[must_use]
    pub fn with_placeholder(placeholder: impl Into<String>) -> Self {
        Self {
            value: String::new(),
            cursor_position: 0,
            placeholder: placeholder.into(),
            required: false,
            max_length: 0,
        }
    }

    /// Set the maximum length for this input
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let input = TextInputState::new().with_max_length(10);
    /// assert_eq!(input.max_length, 10);
    /// ```
    #[must_use]
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = max_length;
        self
    }

    /// Insert a character at the cursor position
    ///
    /// Respects `max_length` if set. Does nothing if max length is reached.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let mut input = TextInputState::new();
    /// input.input_char('a');
    /// input.input_char('b');
    /// assert_eq!(input.value(), "ab");
    /// ```
    pub fn input_char(&mut self, c: char) {
        // Check max length
        if self.max_length > 0 && self.value.chars().count() >= self.max_length {
            return;
        }

        // Convert cursor position to byte index
        let byte_pos = self.char_to_byte_index(self.cursor_position);
        self.value.insert(byte_pos, c);
        self.cursor_position += 1;
    }

    /// Delete character before cursor (backspace)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let mut input = TextInputState::new();
    /// input.set_value("hello");
    /// input.end();
    /// input.backspace();
    /// assert_eq!(input.value(), "hell");
    /// ```
    pub fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            let byte_pos = self.char_to_byte_index(self.cursor_position);
            self.value.remove(byte_pos);
        }
    }

    /// Delete character at cursor (delete key)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let mut input = TextInputState::new();
    /// input.set_value("hello");
    /// input.home();
    /// input.delete();
    /// assert_eq!(input.value(), "ello");
    /// ```
    pub fn delete(&mut self) {
        if self.cursor_position < self.value.chars().count() {
            let byte_pos = self.char_to_byte_index(self.cursor_position);
            self.value.remove(byte_pos);
        }
    }

    /// Move cursor left one character
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let mut input = TextInputState::new();
    /// input.set_value("hi");
    /// input.end();
    /// input.cursor_left();
    /// assert_eq!(input.cursor_position, 1);
    /// ```
    pub fn cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right one character
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let mut input = TextInputState::new();
    /// input.set_value("hi");
    /// input.home();
    /// input.cursor_right();
    /// assert_eq!(input.cursor_position, 1);
    /// ```
    pub fn cursor_right(&mut self) {
        let char_count = self.value.chars().count();
        if self.cursor_position < char_count {
            self.cursor_position += 1;
        }
    }

    /// Move cursor to beginning of input
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let mut input = TextInputState::new();
    /// input.set_value("hello");
    /// input.end();
    /// input.home();
    /// assert_eq!(input.cursor_position, 0);
    /// ```
    pub fn home(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to end of input
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let mut input = TextInputState::new();
    /// input.set_value("hello");
    /// input.end();
    /// assert_eq!(input.cursor_position, 5);
    /// ```
    pub fn end(&mut self) {
        self.cursor_position = self.value.chars().count();
    }

    /// Get the current value
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let mut input = TextInputState::new();
    /// input.set_value("test");
    /// assert_eq!(input.value(), "test");
    /// ```
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set the value directly
    ///
    /// Moves cursor to the end of the new value.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let mut input = TextInputState::new();
    /// input.set_value("hello");
    /// assert_eq!(input.value(), "hello");
    /// assert_eq!(input.cursor_position, 5);
    /// ```
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor_position = self.value.chars().count();
    }

    /// Clear the input value
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let mut input = TextInputState::new();
    /// input.set_value("hello");
    /// input.clear();
    /// assert!(input.is_empty());
    /// assert_eq!(input.cursor_position, 0);
    /// ```
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
    }

    /// Check if the input is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::TextInputState;
    ///
    /// let input = TextInputState::new();
    /// assert!(input.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Convert character position to byte index
    ///
    /// Helper for working with Unicode strings where character count
    /// differs from byte count.
    fn char_to_byte_index(&self, char_pos: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_pos)
            .map_or(self.value.len(), |(idx, _)| idx)
    }
}

impl Default for TextInputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_input() {
        let mut input = TextInputState::new();
        input.input_char('h');
        input.input_char('i');
        assert_eq!(input.value(), "hi");
        assert_eq!(input.cursor_position, 2);
    }

    #[test]
    fn test_backspace() {
        let mut input = TextInputState::new();
        input.set_value("hello");
        input.backspace();
        assert_eq!(input.value(), "hell");
        assert_eq!(input.cursor_position, 4);
    }

    #[test]
    fn test_delete() {
        let mut input = TextInputState::new();
        input.set_value("hello");
        input.home();
        input.delete();
        assert_eq!(input.value(), "ello");
        assert_eq!(input.cursor_position, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let mut input = TextInputState::new();
        input.set_value("hello");
        input.home();
        assert_eq!(input.cursor_position, 0);

        input.cursor_right();
        assert_eq!(input.cursor_position, 1);

        input.end();
        assert_eq!(input.cursor_position, 5);

        input.cursor_left();
        assert_eq!(input.cursor_position, 4);
    }

    #[test]
    fn test_max_length() {
        let mut input = TextInputState::new().with_max_length(3);
        input.input_char('a');
        input.input_char('b');
        input.input_char('c');
        input.input_char('d'); // Should be ignored
        assert_eq!(input.value(), "abc");
    }

    #[test]
    fn test_empty_backspace() {
        let mut input = TextInputState::new();
        input.backspace(); // Should not panic
        assert!(input.is_empty());
    }

    #[test]
    fn test_cursor_at_boundaries() {
        let mut input = TextInputState::new();
        input.set_value("hi");
        input.home();
        input.cursor_left(); // Should stay at 0
        assert_eq!(input.cursor_position, 0);

        input.end();
        input.cursor_right(); // Should stay at end
        assert_eq!(input.cursor_position, 2);
    }

    #[test]
    fn test_unicode_handling() {
        let mut input = TextInputState::new();
        input.input_char('你');
        input.input_char('好');
        assert_eq!(input.value(), "你好");
        assert_eq!(input.cursor_position, 2);

        input.backspace();
        assert_eq!(input.value(), "你");
        assert_eq!(input.cursor_position, 1);
    }

    #[test]
    fn test_clear() {
        let mut input = TextInputState::new();
        input.set_value("test");
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor_position, 0);
    }
}
