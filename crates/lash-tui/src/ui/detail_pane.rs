//! Detail pane rendering

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::state::{AppState, FocusedPane};
use crate::ui::themes;

/// Render the detail pane
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.focused_pane == FocusedPane::Detail;

    let border_style = if is_focused {
        themes::focused_border_style()
    } else {
        themes::unfocused_border_style()
    };

    // Get title from selected file
    let title = state
        .selected_file()
        .map_or_else(|| " Tasks ".to_string(), |f| format!(" {} ", f.title));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    if state.tasks.is_empty() {
        // Show empty state
        let text = if state.files.is_empty() {
            "No files indexed. Run 'lash index' to index your project."
        } else {
            "No tasks in this file."
        };
        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    } else {
        // Create list items from tasks
        let items: Vec<ListItem> = state
            .tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let indent = "  ".repeat(task.depth as usize);
                let checkbox = themes::checkbox_char(task.status);

                let style = if i == state.selected_task_index {
                    themes::selected_style()
                } else {
                    themes::status_style(task.status)
                };

                let line = Line::from(vec![
                    Span::raw(indent),
                    Span::raw(checkbox),
                    Span::raw(" "),
                    Span::styled(task.title.clone(), style),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(themes::selected_style());

        // Create list state with current selection
        let mut list_state = ListState::default();
        list_state.select(Some(state.selected_task_index));

        frame.render_stateful_widget(list, area, &mut list_state);
    }
}
