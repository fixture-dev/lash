//! UI rendering and layout

mod detail_pane;
mod help;
mod nav_pane;
mod status_bar;
mod task_detail;
mod theme_selector;
mod themes;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::state::AppState;

/// Main render function
pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(frame.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Navigation pane
            Constraint::Percentage(70), // Detail pane
        ])
        .split(chunks[0]);

    // Render panes
    nav_pane::render(frame, main_chunks[0], state);
    detail_pane::render(frame, main_chunks[1], state);

    // Render status bar
    status_bar::render(frame, chunks[1], state);

    // Render help overlay if active
    if state.show_help {
        help::render(frame, frame.area(), state);
    }

    // Render theme selector overlay if active
    if state.theme_selector_state.is_some() {
        theme_selector::render(frame, frame.area(), state);
    }

    // Render task detail overlay if active
    if state.task_detail_state.is_some() {
        task_detail::render(frame, frame.area(), state);
    }
}
