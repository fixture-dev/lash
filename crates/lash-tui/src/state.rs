//! Application state management

#![allow(dead_code)] // Some fields/variants reserved for future features

use crate::colors::Theme;
use lash_db::repository::files::FileRecord;
use lash_db::repository::tasks::TaskRecord;

/// Which pane is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    /// Navigation pane (left)
    Navigation,
    /// Detail pane (right)
    Detail,
}

/// View mode for navigation pane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavMode {
    /// File tree view
    Files,
    /// Label list view
    Labels,
    /// Search results view
    SearchResults,
}

/// Application state
#[derive(Debug)]
pub struct AppState {
    /// Which pane is focused
    pub focused_pane: FocusedPane,

    /// Navigation pane mode
    pub nav_mode: NavMode,

    /// Currently loaded files
    pub files: Vec<FileRecord>,

    /// Currently selected file (index into files vec)
    pub selected_file_index: usize,

    /// Currently loaded tasks for selected file
    pub tasks: Vec<TaskRecord>,

    /// Currently selected task (index into tasks vec)
    pub selected_task_index: usize,

    /// Navigation pane scroll offset
    pub nav_scroll: usize,

    /// Detail pane scroll offset
    pub detail_scroll: usize,

    /// Whether help overlay is shown
    pub show_help: bool,

    /// Whether to quit
    pub should_quit: bool,

    /// Color theme
    pub theme: Theme,

    /// Theme selector state (None = closed, Some = open with selection index)
    pub theme_selector_state: Option<ThemeSelectorState>,
}

/// State for the theme selector modal
#[derive(Debug)]
pub struct ThemeSelectorState {
    /// Index of selected scheme in the list
    pub selected_index: usize,

    /// Scroll offset in the list
    pub scroll_offset: usize,

    /// All available scheme names (cached)
    pub scheme_names: Vec<String>,
}

impl AppState {
    /// Create new application state with default theme
    ///
    /// Loads the theme from user config. If that fails, uses the default `Base2Tone Desert` theme.
    #[must_use]
    pub fn new() -> Self {
        Self::with_theme(Self::load_theme_from_config())
    }

    /// Create new application state with a specific theme
    #[must_use]
    pub fn with_theme(theme: Theme) -> Self {
        Self {
            focused_pane: FocusedPane::Navigation,
            nav_mode: NavMode::Files,
            files: Vec::new(),
            selected_file_index: 0,
            tasks: Vec::new(),
            selected_task_index: 0,
            nav_scroll: 0,
            detail_scroll: 0,
            show_help: false,
            should_quit: false,
            theme,
            theme_selector_state: None,
        }
    }

    /// Create new application state with a specific color scheme by name
    ///
    /// # Errors
    ///
    /// Returns error if the color scheme name is not found. The error message
    /// includes "did you mean?" suggestions if similar schemes exist.
    pub fn with_color_scheme(scheme_name: &str) -> Result<Self, String> {
        use crate::colors::REGISTRY;

        let scheme = REGISTRY.get_scheme(scheme_name);

        if let Some(s) = scheme {
            return Ok(Self::with_theme(Theme::new(s.clone())));
        }

        // Scheme not found - provide helpful suggestions
        let suggestions = REGISTRY.fuzzy_search(scheme_name);

        if suggestions.is_empty() {
            Err(format!("Color scheme not found: '{scheme_name}'"))
        } else if suggestions.len() <= 3 {
            Err(format!(
                "Color scheme not found: '{scheme_name}'\nDid you mean: {}?",
                suggestions.join(", ")
            ))
        } else {
            Err(format!(
                "Color scheme not found: '{scheme_name}'\nDid you mean one of: {}, {}?",
                suggestions[..3].join(", "),
                "..."
            ))
        }
    }

    /// Load theme from user config
    ///
    /// If loading fails, returns the default theme (`Base2Tone Desert`).
    #[must_use]
    fn load_theme_from_config() -> Theme {
        use crate::colors::REGISTRY;
        use lash_types::UserConfig;

        let scheme_name = UserConfig::load().ok().map_or_else(
            || "Base2Tone Desert".to_string(),
            |config| config.color_scheme,
        );

        let scheme = REGISTRY.get_scheme_or_default(&scheme_name);
        Theme::new(scheme.clone())
    }

    /// Switch focus to the other pane
    pub fn switch_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::Navigation => FocusedPane::Detail,
            FocusedPane::Detail => FocusedPane::Navigation,
        };
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        match self.focused_pane {
            FocusedPane::Navigation => {
                if self.selected_file_index > 0 {
                    self.selected_file_index -= 1;
                }
            }
            FocusedPane::Detail => {
                if self.selected_task_index > 0 {
                    self.selected_task_index -= 1;
                }
            }
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        match self.focused_pane {
            FocusedPane::Navigation => {
                if self.selected_file_index + 1 < self.files.len() {
                    self.selected_file_index += 1;
                }
            }
            FocusedPane::Detail => {
                if self.selected_task_index + 1 < self.tasks.len() {
                    self.selected_task_index += 1;
                }
            }
        }
    }

    /// Go to top of current list
    pub fn go_top(&mut self) {
        match self.focused_pane {
            FocusedPane::Navigation => self.selected_file_index = 0,
            FocusedPane::Detail => self.selected_task_index = 0,
        }
    }

    /// Go to bottom of current list
    pub fn go_bottom(&mut self) {
        match self.focused_pane {
            FocusedPane::Navigation => {
                if !self.files.is_empty() {
                    self.selected_file_index = self.files.len() - 1;
                }
            }
            FocusedPane::Detail => {
                if !self.tasks.is_empty() {
                    self.selected_task_index = self.tasks.len() - 1;
                }
            }
        }
    }

    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Open theme selector
    pub fn open_theme_selector(&mut self) {
        use crate::colors::REGISTRY;

        let scheme_names = REGISTRY.scheme_names();
        let current_name = self.theme.name();

        // Find current theme in the list
        let selected_index = scheme_names
            .iter()
            .position(|name| name == current_name)
            .unwrap_or(0);

        self.theme_selector_state = Some(ThemeSelectorState {
            selected_index,
            scroll_offset: selected_index.saturating_sub(5), // Center selection
            scheme_names,
        });
    }

    /// Close theme selector without applying
    pub fn close_theme_selector(&mut self) {
        self.theme_selector_state = None;
    }

    /// Move selection up in theme selector
    pub fn theme_selector_up(&mut self) {
        if let Some(selector) = &mut self.theme_selector_state {
            if selector.selected_index > 0 {
                selector.selected_index -= 1;
            }
        }
    }

    /// Move selection down in theme selector
    pub fn theme_selector_down(&mut self) {
        if let Some(selector) = &mut self.theme_selector_state {
            if selector.selected_index + 1 < selector.scheme_names.len() {
                selector.selected_index += 1;
            }
        }
    }

    /// Apply selected theme and close selector
    ///
    /// Saves the theme to user config and updates the current theme.
    pub fn apply_selected_theme(&mut self) -> Result<(), String> {
        use crate::colors::REGISTRY;
        use lash_types::UserConfig;

        if let Some(selector) = &self.theme_selector_state {
            let scheme_name = &selector.scheme_names[selector.selected_index];
            let scheme = REGISTRY
                .get_scheme(scheme_name)
                .ok_or_else(|| format!("Scheme not found: {scheme_name}"))?;

            // Update theme
            self.theme = Theme::new(scheme.clone());

            // Save to user config
            let mut user_config = UserConfig::load().unwrap_or_default();
            user_config.color_scheme.clone_from(scheme_name);
            user_config
                .save()
                .map_err(|e| format!("Failed to save theme: {e}"))?;

            // Close selector
            self.theme_selector_state = None;

            Ok(())
        } else {
            Err("Theme selector not open".to_string())
        }
    }

    /// Get currently selected file
    #[must_use]
    pub fn selected_file(&self) -> Option<&FileRecord> {
        self.files.get(self.selected_file_index)
    }

    /// Get currently selected task
    #[must_use]
    pub fn selected_task(&self) -> Option<&TaskRecord> {
        self.tasks.get(self.selected_task_index)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
