//! Status bar rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::state::{AppState, StatusLevel};

/// Render the status bar
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    // Check if we have an active status message to display
    if let Some(msg) = &state.status_message {
        render_status_message(frame, area, msg, state);
        return;
    }

    // Default status bar rendering
    render_default(frame, area, state);
}

/// Render a status message (replaces default status bar temporarily)
fn render_status_message(
    frame: &mut Frame,
    area: Rect,
    msg: &crate::state::StatusMessage,
    state: &AppState,
) {
    let (icon, fg_color, bg_color) = match msg.level {
        StatusLevel::Info => ("i", state.theme.background(), state.theme.info_color()),
        StatusLevel::Warning => ("!", state.theme.background(), state.theme.warning_color()),
        StatusLevel::Error => ("x", state.theme.background(), state.theme.error_color()),
        StatusLevel::Success => ("✓", state.theme.background(), state.theme.success_color()),
    };

    let spans = vec![
        Span::styled(
            format!(" {icon} "),
            Style::default()
                .fg(fg_color)
                .bg(bg_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", msg.text),
            Style::default().fg(fg_color).bg(bg_color),
        ),
        // Fill remaining space with background color
        Span::styled(
            {
                #[allow(clippy::cast_possible_truncation)]
                let text_len = msg.text.len() as u16;
                " ".repeat(area.width.saturating_sub(text_len + 5) as usize)
            },
            Style::default().bg(bg_color),
        ),
    ];

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(state.theme.background()));

    frame.render_widget(paragraph, area);
}

/// Render the default status bar (no active message)
fn render_default(frame: &mut Frame, area: Rect, state: &AppState) {
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
