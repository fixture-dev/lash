//! Navigation pane rendering

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::state::{AppState, FocusedPane};
use crate::ui::themes;

/// Render the navigation pane
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.focused_pane == FocusedPane::Navigation;
    let theme = &state.theme;

    let border_style = if is_focused {
        themes::focused_border_style(theme)
    } else {
        themes::unfocused_border_style(theme)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Files ")
        .border_style(border_style);

    // Create list items from files
    let items: Vec<ListItem> = state
        .files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let status_indicator = match file.status {
                lash_types::FileStatus::Complete => "✓",
                lash_types::FileStatus::Blocked => "!",
                lash_types::FileStatus::InProgress => "○",
                lash_types::FileStatus::Empty => "·",
            };

            let style = if i == state.selected_file_index {
                themes::selected_style(theme)
            } else {
                themes::file_status_style(file.status, theme)
            };

            let line = Line::from(vec![
                Span::raw(status_indicator),
                Span::raw(" "),
                Span::styled(file.title.clone(), style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(themes::selected_style(theme));

    // Create list state with current selection
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_file_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}
