//! Color schemes and theming

use ratatui::style::{Color, Modifier, Style};

use lash_types::{FileStatus, TaskStatus};

/// Get style for task status
pub fn status_style(status: TaskStatus) -> Style {
    match status {
        TaskStatus::Done => Style::default().fg(Color::Green),
        TaskStatus::Blocked => Style::default().fg(Color::Red),
        TaskStatus::Open => Style::default().fg(Color::White),
        TaskStatus::Waived => Style::default().fg(Color::DarkGray),
    }
}

/// Get style for file status
pub fn file_status_style(status: FileStatus) -> Style {
    match status {
        FileStatus::Complete => Style::default().fg(Color::Green),
        FileStatus::Blocked => Style::default().fg(Color::Red),
        FileStatus::InProgress => Style::default().fg(Color::Yellow),
        FileStatus::Empty => Style::default().fg(Color::DarkGray),
    }
}

/// Get checkbox character for task status
pub fn checkbox_char(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Open => "[ ]",
        TaskStatus::Done => "[x]",
        TaskStatus::Waived => "[-]",
        TaskStatus::Blocked => "[!]",
    }
}

/// Style for selected items
pub fn selected_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD)
}

/// Style for focused pane border
pub fn focused_border_style() -> Style {
    Style::default().fg(Color::Cyan)
}

/// Style for unfocused pane border
pub fn unfocused_border_style() -> Style {
    Style::default().fg(Color::DarkGray)
}
