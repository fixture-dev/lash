//! Theme with semantic color mappings for Lash TUI

use super::ColorScheme;
use lash_types::{FileStatus, TaskStatus};
use ratatui::style::{Color, Modifier, Style};

/// Theme providing semantic color mappings
///
/// Wraps a `ColorScheme` and maps ANSI colors to semantic meanings in the Lash TUI.
#[derive(Debug, Clone)]
pub struct Theme {
    scheme: ColorScheme,
}

impl Theme {
    /// Create a new theme from a color scheme
    #[must_use]
    pub fn new(scheme: ColorScheme) -> Self {
        Self { scheme }
    }

    /// Get the underlying color scheme
    #[must_use]
    pub fn scheme(&self) -> &ColorScheme {
        &self.scheme
    }

    /// Get the scheme name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.scheme.name
    }

    // ========================================================================
    // Base colors
    // ========================================================================

    /// Background color
    #[must_use]
    pub fn background(&self) -> Color {
        self.scheme.bg_color()
    }

    /// Foreground (text) color
    #[must_use]
    pub fn foreground(&self) -> Color {
        self.scheme.fg_color()
    }

    /// Cursor color
    #[must_use]
    pub fn cursor(&self) -> Color {
        self.scheme.cursor_color()
    }

    // ========================================================================
    // Task status colors (semantic mappings)
    // ========================================================================

    /// Color for completed tasks (ANSI green)
    #[must_use]
    pub fn task_done(&self) -> Color {
        self.scheme.ansi_color(2) // color_03 (green)
    }

    /// Color for blocked tasks (ANSI red)
    #[must_use]
    pub fn task_blocked(&self) -> Color {
        self.scheme.ansi_color(1) // color_02 (red)
    }

    /// Color for open tasks (foreground)
    #[must_use]
    pub fn task_open(&self) -> Color {
        self.foreground()
    }

    /// Color for waived tasks (ANSI bright black / dark gray)
    #[must_use]
    pub fn task_waived(&self) -> Color {
        self.scheme.ansi_color(8) // color_09 (bright black)
    }

    /// Get style for task status
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::colors::{Theme, REGISTRY};
    /// use lash_types::TaskStatus;
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
    /// let theme = Theme::new(scheme.clone());
    ///
    /// let style = theme.task_status_style(TaskStatus::Done);
    /// // style will have green foreground
    /// ```
    #[must_use]
    pub fn task_status_style(&self, status: TaskStatus) -> Style {
        match status {
            TaskStatus::Done => Style::default().fg(self.task_done()),
            TaskStatus::Blocked => Style::default().fg(self.task_blocked()),
            TaskStatus::Open => Style::default().fg(self.task_open()),
            TaskStatus::Waived => Style::default().fg(self.task_waived()),
        }
    }

    // ========================================================================
    // File status colors
    // ========================================================================

    /// Color for complete files (green)
    #[must_use]
    pub fn file_complete(&self) -> Color {
        self.scheme.ansi_color(2) // green
    }

    /// Color for blocked files (red)
    #[must_use]
    pub fn file_blocked(&self) -> Color {
        self.scheme.ansi_color(1) // red
    }

    /// Color for in-progress files (yellow)
    #[must_use]
    pub fn file_in_progress(&self) -> Color {
        self.scheme.ansi_color(3) // color_04 (yellow)
    }

    /// Color for empty files (dark gray)
    #[must_use]
    pub fn file_empty(&self) -> Color {
        self.scheme.ansi_color(8) // bright black
    }

    /// Get style for file status
    #[must_use]
    pub fn file_status_style(&self, status: FileStatus) -> Style {
        match status {
            FileStatus::Complete => Style::default().fg(self.file_complete()),
            FileStatus::Blocked => Style::default().fg(self.file_blocked()),
            FileStatus::InProgress => Style::default().fg(self.file_in_progress()),
            FileStatus::Empty => Style::default().fg(self.file_empty()),
        }
    }

    // ========================================================================
    // UI element colors
    // ========================================================================

    /// Border color for focused pane (cyan)
    #[must_use]
    pub fn border_focused(&self) -> Color {
        self.scheme.ansi_color(6) // color_07 (cyan)
    }

    /// Border color for unfocused pane (dark gray)
    #[must_use]
    pub fn border_unfocused(&self) -> Color {
        self.scheme.ansi_color(8) // bright black
    }

    /// Background for selected items (ANSI color 8 / bright black)
    #[must_use]
    pub fn selected_bg(&self) -> Color {
        self.scheme.ansi_color(8)
    }

    /// Style for selected items
    #[must_use]
    pub fn selected_style(&self) -> Style {
        Style::default()
            .bg(self.selected_bg())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for focused pane border
    #[must_use]
    pub fn focused_border_style(&self) -> Style {
        Style::default().fg(self.border_focused())
    }

    /// Style for unfocused pane border
    #[must_use]
    pub fn unfocused_border_style(&self) -> Style {
        Style::default().fg(self.border_unfocused())
    }

    /// Color for labels/tags (cyan)
    #[must_use]
    pub fn label_color(&self) -> Color {
        self.scheme.ansi_color(6) // cyan
    }

    /// Color for emphasis/highlights (yellow)
    #[must_use]
    pub fn emphasis_color(&self) -> Color {
        self.scheme.ansi_color(3) // yellow
    }

    /// Color for errors (red)
    #[must_use]
    pub fn error_color(&self) -> Color {
        self.scheme.ansi_color(1) // red
    }

    /// Color for success (green)
    #[must_use]
    pub fn success_color(&self) -> Color {
        self.scheme.ansi_color(2) // green
    }

    /// Color for info/hints (blue)
    #[must_use]
    pub fn info_color(&self) -> Color {
        self.scheme.ansi_color(4) // color_05 (blue)
    }

    /// Color for warnings (yellow)
    #[must_use]
    pub fn warning_color(&self) -> Color {
        self.scheme.ansi_color(3) // yellow
    }

    /// Color for muted/secondary text (bright black)
    #[must_use]
    pub fn muted_color(&self) -> Color {
        self.scheme.ansi_color(8) // bright black
    }

    // ========================================================================
    // Checkbox characters (not colors, but related to task status)
    // ========================================================================

    /// Get checkbox character for task status
    #[must_use]
    pub fn checkbox_char(&self, status: TaskStatus) -> &'static str {
        match status {
            TaskStatus::Open => "[ ]",
            TaskStatus::Done => "[x]",
            TaskStatus::Waived => "[-]",
            TaskStatus::Blocked => "[!]",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::REGISTRY;

    #[test]
    fn test_theme_creation() {
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        let theme = Theme::new(scheme.clone());

        assert_eq!(theme.name(), "Base2Tone Desert");
        assert_eq!(theme.scheme().name, "Base2Tone Desert");
    }

    #[test]
    fn test_task_status_styles() {
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        let theme = Theme::new(scheme.clone());

        // Just verify they don't panic
        let _ = theme.task_status_style(TaskStatus::Done);
        let _ = theme.task_status_style(TaskStatus::Blocked);
        let _ = theme.task_status_style(TaskStatus::Open);
        let _ = theme.task_status_style(TaskStatus::Waived);
    }

    #[test]
    fn test_file_status_styles() {
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        let theme = Theme::new(scheme.clone());

        // Just verify they don't panic
        let _ = theme.file_status_style(FileStatus::Complete);
        let _ = theme.file_status_style(FileStatus::Blocked);
        let _ = theme.file_status_style(FileStatus::InProgress);
        let _ = theme.file_status_style(FileStatus::Empty);
    }

    #[test]
    fn test_ui_element_styles() {
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        let theme = Theme::new(scheme.clone());

        // Verify all UI element color methods work
        let _ = theme.border_focused();
        let _ = theme.border_unfocused();
        let _ = theme.selected_bg();
        let _ = theme.selected_style();
        let _ = theme.focused_border_style();
        let _ = theme.unfocused_border_style();
    }

    #[test]
    fn test_checkbox_chars() {
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        let theme = Theme::new(scheme.clone());

        assert_eq!(theme.checkbox_char(TaskStatus::Open), "[ ]");
        assert_eq!(theme.checkbox_char(TaskStatus::Done), "[x]");
        assert_eq!(theme.checkbox_char(TaskStatus::Waived), "[-]");
        assert_eq!(theme.checkbox_char(TaskStatus::Blocked), "[!]");
    }
}
