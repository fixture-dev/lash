//! Radio button selection component

/// A radio option with a value, label, and shortcut key
///
/// # Examples
///
/// ```
/// use lash_tui::components::RadioOption;
///
/// let option = RadioOption {
///     value: "open",
///     label: "Open".to_string(),
///     key: 'o',
/// };
/// assert_eq!(option.key, 'o');
/// ```
#[derive(Debug, Clone)]
pub struct RadioOption<T> {
    /// The underlying value
    pub value: T,
    /// Display label for the UI
    pub label: String,
    /// Shortcut key (e.g., 'o' for Open)
    pub key: char,
}

/// State for radio button selection
///
/// Allows selecting one option from a list using keyboard navigation
/// or shortcut keys.
///
/// # Examples
///
/// ```
/// use lash_tui::components::{RadioSelectState, RadioOption};
///
/// let options = vec![
///     RadioOption { value: 1, label: "Option 1".to_string(), key: '1' },
///     RadioOption { value: 2, label: "Option 2".to_string(), key: '2' },
/// ];
/// let mut radio = RadioSelectState::new(options);
/// assert_eq!(*radio.selected(), 1);
/// radio.select_next();
/// assert_eq!(*radio.selected(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct RadioSelectState<T: Clone> {
    /// Available options
    pub options: Vec<RadioOption<T>>,
    /// Currently selected index
    pub selected_index: usize,
}

impl<T: Clone> RadioSelectState<T> {
    /// Create a new radio select with the given options
    ///
    /// The first option is selected by default.
    ///
    /// # Panics
    ///
    /// Panics if the options vector is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{RadioSelectState, RadioOption};
    ///
    /// let options = vec![
    ///     RadioOption { value: "a", label: "Option A".to_string(), key: 'a' },
    ///     RadioOption { value: "b", label: "Option B".to_string(), key: 'b' },
    /// ];
    /// let radio = RadioSelectState::new(options);
    /// assert_eq!(*radio.selected(), "a");
    /// ```
    #[must_use]
    pub fn new(options: Vec<RadioOption<T>>) -> Self {
        assert!(
            !options.is_empty(),
            "RadioSelectState requires at least one option"
        );
        Self {
            options,
            selected_index: 0,
        }
    }

    /// Select the next option (cycling)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{RadioSelectState, RadioOption};
    ///
    /// let options = vec![
    ///     RadioOption { value: 1, label: "One".to_string(), key: '1' },
    ///     RadioOption { value: 2, label: "Two".to_string(), key: '2' },
    /// ];
    /// let mut radio = RadioSelectState::new(options);
    /// radio.select_next();
    /// assert_eq!(radio.selected_index, 1);
    /// radio.select_next(); // Cycles back to 0
    /// assert_eq!(radio.selected_index, 0);
    /// ```
    pub fn select_next(&mut self) {
        self.selected_index = (self.selected_index + 1) % self.options.len();
    }

    /// Select the previous option (cycling)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{RadioSelectState, RadioOption};
    ///
    /// let options = vec![
    ///     RadioOption { value: 1, label: "One".to_string(), key: '1' },
    ///     RadioOption { value: 2, label: "Two".to_string(), key: '2' },
    /// ];
    /// let mut radio = RadioSelectState::new(options);
    /// radio.select_prev(); // Cycles to last option
    /// assert_eq!(radio.selected_index, 1);
    /// ```
    pub fn select_prev(&mut self) {
        if self.selected_index == 0 {
            self.selected_index = self.options.len() - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    /// Select an option by its shortcut key
    ///
    /// Returns `true` if the key matched an option, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{RadioSelectState, RadioOption};
    ///
    /// let options = vec![
    ///     RadioOption { value: "open", label: "Open".to_string(), key: 'o' },
    ///     RadioOption { value: "done", label: "Done".to_string(), key: 'd' },
    /// ];
    /// let mut radio = RadioSelectState::new(options);
    /// assert!(radio.select_by_key('d'));
    /// assert_eq!(*radio.selected(), "done");
    /// assert!(!radio.select_by_key('x')); // Key not found
    /// ```
    pub fn select_by_key(&mut self, key: char) -> bool {
        if let Some(index) = self.options.iter().position(|opt| opt.key == key) {
            self.selected_index = index;
            true
        } else {
            false
        }
    }

    /// Get the currently selected value
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{RadioSelectState, RadioOption};
    ///
    /// let options = vec![
    ///     RadioOption { value: 42, label: "Answer".to_string(), key: 'a' },
    /// ];
    /// let radio = RadioSelectState::new(options);
    /// assert_eq!(*radio.selected(), 42);
    /// ```
    #[must_use]
    pub fn selected(&self) -> &T {
        &self.options[self.selected_index].value
    }

    /// Get the currently selected index
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::components::{RadioSelectState, RadioOption};
    ///
    /// let options = vec![
    ///     RadioOption { value: "a", label: "A".to_string(), key: 'a' },
    ///     RadioOption { value: "b", label: "B".to_string(), key: 'b' },
    /// ];
    /// let mut radio = RadioSelectState::new(options);
    /// radio.select_next();
    /// assert_eq!(radio.selected_index(), 1);
    /// ```
    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let options = vec![
            RadioOption {
                value: "a",
                label: "A".to_string(),
                key: 'a',
            },
            RadioOption {
                value: "b",
                label: "B".to_string(),
                key: 'b',
            },
        ];
        let radio = RadioSelectState::new(options);
        assert_eq!(*radio.selected(), "a");
        assert_eq!(radio.selected_index, 0);
    }

    #[test]
    #[should_panic(expected = "RadioSelectState requires at least one option")]
    fn test_empty_options_panics() {
        let options: Vec<RadioOption<i32>> = vec![];
        let _radio = RadioSelectState::new(options);
    }

    #[test]
    fn test_select_next() {
        let options = vec![
            RadioOption {
                value: 1,
                label: "One".to_string(),
                key: '1',
            },
            RadioOption {
                value: 2,
                label: "Two".to_string(),
                key: '2',
            },
            RadioOption {
                value: 3,
                label: "Three".to_string(),
                key: '3',
            },
        ];
        let mut radio = RadioSelectState::new(options);

        assert_eq!(*radio.selected(), 1);
        radio.select_next();
        assert_eq!(*radio.selected(), 2);
        radio.select_next();
        assert_eq!(*radio.selected(), 3);
        radio.select_next(); // Cycles back
        assert_eq!(*radio.selected(), 1);
    }

    #[test]
    fn test_select_prev() {
        let options = vec![
            RadioOption {
                value: 1,
                label: "One".to_string(),
                key: '1',
            },
            RadioOption {
                value: 2,
                label: "Two".to_string(),
                key: '2',
            },
        ];
        let mut radio = RadioSelectState::new(options);

        assert_eq!(*radio.selected(), 1);
        radio.select_prev(); // Cycles to last
        assert_eq!(*radio.selected(), 2);
        radio.select_prev();
        assert_eq!(*radio.selected(), 1);
    }

    #[test]
    fn test_select_by_key() {
        let options = vec![
            RadioOption {
                value: "open",
                label: "Open".to_string(),
                key: 'o',
            },
            RadioOption {
                value: "done",
                label: "Done".to_string(),
                key: 'd',
            },
            RadioOption {
                value: "blocked",
                label: "Blocked".to_string(),
                key: 'b',
            },
        ];
        let mut radio = RadioSelectState::new(options);

        assert!(radio.select_by_key('d'));
        assert_eq!(*radio.selected(), "done");

        assert!(radio.select_by_key('b'));
        assert_eq!(*radio.selected(), "blocked");

        assert!(!radio.select_by_key('x')); // Key not found
        assert_eq!(*radio.selected(), "blocked"); // Selection unchanged
    }

    #[test]
    fn test_selected_index() {
        let options = vec![
            RadioOption {
                value: "a",
                label: "A".to_string(),
                key: 'a',
            },
            RadioOption {
                value: "b",
                label: "B".to_string(),
                key: 'b',
            },
        ];
        let mut radio = RadioSelectState::new(options);
        assert_eq!(radio.selected_index(), 0);
        radio.select_next();
        assert_eq!(radio.selected_index(), 1);
    }
}
