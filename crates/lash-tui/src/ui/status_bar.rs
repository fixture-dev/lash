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
        crate::state::FocusedPane::Detail => "Tasks",
    };

    let file_count = state.files.len();
    let task_count = state.tasks.len();

    let line = Line::from(vec![
        Span::styled(
            format!(" {focused_pane_name} "),
            Style::default().fg(Color::Black).bg(Color::Cyan),
        ),
        Span::raw(format!("  Files: {file_count}  Tasks: {task_count}  ")),
        Span::styled(" Press ? for help ", Style::default().fg(Color::DarkGray)),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, area);
}
