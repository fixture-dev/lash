//! Color schemes and theming
//!
//! This module provides backward-compatible theme functions that now use
//! the Theme struct. All functions now require a theme parameter.

use crate::colors::Theme;
use lash_types::{FileStatus, TaskStatus};
use ratatui::style::Style;

/// Get style for task status
///
/// # Arguments
///
/// * `status` - Task status
/// * `theme` - Color theme to use
pub fn status_style(status: TaskStatus, theme: &Theme) -> Style {
    theme.task_status_style(status)
}

/// Get style for file status
///
/// # Arguments
///
/// * `status` - File status
/// * `theme` - Color theme to use
pub fn file_status_style(status: FileStatus, theme: &Theme) -> Style {
    theme.file_status_style(status)
}

/// Get checkbox character for task status
///
/// # Arguments
///
/// * `status` - Task status
/// * `theme` - Color theme to use
pub fn checkbox_char(status: TaskStatus, theme: &Theme) -> &'static str {
    theme.checkbox_char(status)
}

/// Style for selected items
///
/// # Arguments
///
/// * `theme` - Color theme to use
pub fn selected_style(theme: &Theme) -> Style {
    theme.selected_style()
}

/// Style for focused pane border
///
/// # Arguments
///
/// * `theme` - Color theme to use
pub fn focused_border_style(theme: &Theme) -> Style {
    theme.focused_border_style()
}

/// Style for unfocused pane border
///
/// # Arguments
///
/// * `theme` - Color theme to use
pub fn unfocused_border_style(theme: &Theme) -> Style {
    theme.unfocused_border_style()
}
