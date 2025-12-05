//! Status bar rendering

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::state::AppState;

/// Render the status bar
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused_pane_name = match state.focused_pane {
        crate::state::FocusedPane::Navigation => "Files",
        crate::state::FocusedPane::Description => "Description",
        crate::state::FocusedPane::Detail => "Tasks",
    };

    let file_count = state.files.len();
    let task_count = state.tasks.len();

    // Build spans for status bar
    let mut spans = vec![
        Span::styled(
            format!(" {focused_pane_name} "),
            Style::default()
                .fg(state.theme.background())
                .bg(state.theme.border_focused()),
        ),
        Span::raw(format!("  Files: {file_count}  Tasks: {task_count}  ")),
    ];

    // Add active filter indicator if present
    if let Some(filter) = &state.current_label_filter {
        spans.push(Span::styled(
            format!("#{filter}  "),
            Style::default().fg(state.theme.label_color()),
        ));
    }

    spans.push(Span::styled(
        " Press ? for help ",
        Style::default().fg(Color::DarkGray),
    ));

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, area);
}
