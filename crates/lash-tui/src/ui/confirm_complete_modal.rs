//! Confirm complete modal rendering
//!
//! Shown when a user attempts to mark a task as complete but it has
//! open subtasks. Prompts the user to confirm cascading completion.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::state::AppState;

/// Render confirm complete modal overlay
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(modal_state) = &state.confirm_complete_modal_state else {
        return;
    };

    // Create centered rect (60% width, 50% height)
    let popup_area = centered_rect(60, 50, area);

    // Clear the area
    frame.render_widget(Clear, popup_area);

    // Main block with title
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm Complete ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    frame.render_widget(block.clone(), popup_area);

    // Inner area (inside the block borders)
    let inner = block.inner(popup_area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Warning message
            Constraint::Length(2), // Parent task
            Constraint::Min(3),    // Subtasks list
            Constraint::Length(3), // Actions
        ])
        .margin(1)
        .split(inner);

    // Warning message
    render_warning(frame, chunks[0], state);

    // Parent task info
    render_parent_task(
        frame,
        chunks[1],
        modal_state.task.title.as_str(),
        &state.theme,
    );

    // Open subtasks list
    render_subtasks(frame, chunks[2], modal_state, &state.theme);

    // Action buttons/instructions
    render_actions(frame, chunks[3], &state.theme);
}

/// Render the warning message
fn render_warning(frame: &mut Frame, area: Rect, state: &AppState) {
    let text = vec![Line::from(vec![
        Span::styled(
            "! ",
            Style::default()
                .fg(state.theme.warning_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("This task has "),
        Span::styled(
            "open subtasks",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(". Marking it complete will also mark all subtasks as complete."),
    ])];

    let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render the parent task info
fn render_parent_task(frame: &mut Frame, area: Rect, title: &str, theme: &crate::colors::Theme) {
    let text = vec![Line::from(vec![
        Span::styled("Task: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            title,
            Style::default()
                .fg(theme.foreground())
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, area);
}

/// Render the list of open subtasks
fn render_subtasks(
    frame: &mut Frame,
    area: Rect,
    modal_state: &crate::state::ConfirmCompleteModalState,
    theme: &crate::colors::Theme,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .title(format!(
            " {} open subtask{} will be completed ",
            modal_state.open_subtasks.len(),
            if modal_state.open_subtasks.len() == 1 {
                ""
            } else {
                "s"
            }
        ))
        .style(Style::default().fg(Color::DarkGray));

    // Create list items for subtasks
    let items: Vec<ListItem> = modal_state
        .open_subtasks
        .iter()
        .take(10) // Limit to 10 to prevent overflow
        .map(|task| {
            let indent = "  ".repeat(task.depth as usize);
            let spans = vec![
                Span::raw(indent),
                Span::styled("[ ] ", Style::default().fg(theme.foreground())),
                Span::styled(&task.title, Style::default().fg(theme.foreground())),
            ];
            ListItem::new(Line::from(spans))
        })
        .collect();

    // Add "and X more..." if truncated
    let mut items = items;
    if modal_state.open_subtasks.len() > 10 {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!("  ... and {} more", modal_state.open_subtasks.len() - 10),
            Style::default().fg(Color::DarkGray),
        )])));
    }

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

/// Render the action instructions
fn render_actions(frame: &mut Frame, area: Rect, theme: &crate::colors::Theme) {
    let text = vec![Line::from(vec![
        Span::styled(
            "Enter",
            Style::default()
                .fg(theme.success_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" to confirm  |  "),
        Span::styled(
            "Esc",
            Style::default()
                .fg(theme.error_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" to cancel"),
    ])];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

/// Create a centered rect within the given area
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
