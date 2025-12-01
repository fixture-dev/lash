//! UI rendering and layout

mod detail_pane;
mod filter_modal;
mod help;
mod logo;
mod nav_pane;
mod search_modal;
mod status_bar;
mod task_detail;
mod theme_selector;
mod themes;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::state::{AppState, FocusedPane};

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
            Constraint::Percentage(30), // Left pane (logo + navigation)
            Constraint::Percentage(70), // Detail pane
        ])
        .split(chunks[0]);

    // Split the left pane into logo and navigation
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(logo::LOGO_HEIGHT), // Logo area
            Constraint::Min(1),                    // Navigation pane
        ])
        .split(main_chunks[0]);

    // Render logo in upper-left
    let is_nav_focused = state.focused_pane == FocusedPane::Navigation;
    logo::render(frame, left_chunks[0], &state.theme, is_nav_focused);

    // Render navigation pane below logo
    nav_pane::render(frame, left_chunks[1], state);

    // Render detail pane
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

    // Render search modal overlay if active
    if state.search_modal_state.is_some() {
        search_modal::render(frame, frame.area(), state);
    }

    // Render filter modal overlay if active
    if state.filter_modal_state.is_some() {
        filter_modal::render(frame, frame.area(), state);
    }
}
