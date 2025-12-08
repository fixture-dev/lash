//! Confirm linked file complete modal rendering
//!
//! Shown when a user attempts to mark a cross-file link task as complete.
//! This cascades to all open tasks in the linked file, so we warn the user
//! and prompt for confirmation.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::state::AppState;

/// Maximum number of tasks to display in the modal
const MAX_DISPLAYED_TASKS: usize = 10;

/// Render confirm linked file complete modal overlay
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(modal_state) = &state.confirm_linked_file_complete_modal_state else {
        return;
    };

    // Create centered rect (65% width, 55% height)
    let popup_area = centered_rect(65, 55, area);

    // Clear the area
    frame.render_widget(Clear, popup_area);

    // Main block with title
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm Linked File Complete ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    frame.render_widget(block.clone(), popup_area);

    // Inner area (inside the block borders)
    let inner = block.inner(popup_area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Warning message
            Constraint::Length(2), // Link task info
            Constraint::Length(2), // Target file info
            Constraint::Min(3),    // Open tasks list
            Constraint::Length(3), // Actions
        ])
        .margin(1)
        .split(inner);

    // Warning message
    render_warning(frame, chunks[0], state);

    // Link task info
    render_link_task(
        frame,
        chunks[1],
        modal_state.link_task.title.as_str(),
        &state.theme,
    );

    // Target file info
    render_target_file(
        frame,
        chunks[2],
        &modal_state.target_file.path,
        &state.theme,
    );

    // Open tasks list
    render_open_tasks(frame, chunks[3], modal_state, &state.theme);

    // Action buttons/instructions
    render_actions(frame, chunks[4], &state.theme);
}

/// Render the warning message
fn render_warning(frame: &mut Frame, area: Rect, state: &AppState) {
    let text = vec![
        Line::from(vec![
            Span::styled(
                "! ",
                Style::default()
                    .fg(state.theme.warning_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("This task links to another file. Marking it complete will also"),
        ]),
        Line::from("  complete all open tasks in the linked file."),
    ];

    let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render the link task info
fn render_link_task(frame: &mut Frame, area: Rect, title: &str, theme: &crate::colors::Theme) {
    // Format the title to strip markdown links and convert @labels to hashtags
    let formatted_title = lash_core::display::format_index_title(title);

    let text = vec![Line::from(vec![
        Span::styled("Link task: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            formatted_title,
            Style::default()
                .fg(theme.foreground())
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, area);
}

/// Render the target file info
fn render_target_file(
    frame: &mut Frame,
    area: Rect,
    file_path: &std::path::Path,
    theme: &crate::colors::Theme,
) {
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let text = vec![Line::from(vec![
        Span::styled("Linked file: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            filename,
            Style::default()
                .fg(theme.foreground())
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, area);
}

/// Render the list of open tasks in the target file
fn render_open_tasks(
    frame: &mut Frame,
    area: Rect,
    modal_state: &crate::state::ConfirmLinkedFileCompleteModalState,
    theme: &crate::colors::Theme,
) {
    let task_count = modal_state.total_open_count;
    let block = Block::default()
        .borders(Borders::TOP)
        .title(format!(
            " {} open task{} will be completed ",
            task_count,
            if task_count == 1 { "" } else { "s" }
        ))
        .style(Style::default().fg(Color::DarkGray));

    // Create list items for open tasks (already truncated to 10 in state)
    let items: Vec<ListItem> = modal_state
        .open_tasks
        .iter()
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
    if modal_state.total_open_count > MAX_DISPLAYED_TASKS {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!(
                "  ... and {} more",
                modal_state.total_open_count - MAX_DISPLAYED_TASKS
            ),
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
            "Enter/Y",
            Style::default()
                .fg(theme.success_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" to confirm  |  "),
        Span::styled(
            "Esc/N",
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
