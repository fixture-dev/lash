//! Task creation modal rendering
#![allow(clippy::cast_possible_truncation)]

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::colors::Theme;
use crate::state::{AppState, TaskCreationModalState, TaskFormField};

/// Render the task creation modal
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(modal_state) = &state.task_creation_modal_state else {
        return;
    };

    // Create centered popup (70% width, 80% height)
    let popup_area = centered_rect(70, 80, area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Render modal border
    let block = Block::default()
        .title(" Create New Task ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(state.theme.border_focused()));
    frame.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);

    // Split into form fields
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Parent
            Constraint::Length(3), // Labels
            Constraint::Length(3), // Status
            Constraint::Length(3), // Owner + Estimate (side by side)
            Constraint::Min(4),    // Agent Note
            Constraint::Length(1), // Help line
        ])
        .split(inner);

    // Render each field
    render_title_field(frame, chunks[0], modal_state, &state.theme);
    render_parent_field(frame, chunks[1], modal_state, &state.theme);
    render_labels_field(frame, chunks[2], modal_state, &state.theme);
    render_status_field(frame, chunks[3], modal_state, &state.theme);
    render_owner_estimate_fields(frame, chunks[4], modal_state, &state.theme);
    render_agent_note_field(frame, chunks[5], modal_state, &state.theme);
    render_help_line(frame, chunks[6], &state.theme);
}

/// Render the title input field with validation indicator
fn render_title_field(
    frame: &mut Frame,
    area: Rect,
    state: &TaskCreationModalState,
    theme: &Theme,
) {
    let is_focused = state.focused_field == TaskFormField::Title;
    let has_error = state.has_error(TaskFormField::Title);
    let is_valid = state.is_field_valid(TaskFormField::Title);

    // Build label with character count and validation indicator
    let char_count = state.title.value().chars().count();
    let indicator = if is_valid && char_count > 0 {
        " ✓"
    } else if has_error && state.has_blocking_error(TaskFormField::Title) {
        " ✗"
    } else {
        ""
    };
    let label = format!("Title * ({char_count}/200){indicator}");

    // Style based on state
    let border_color = if has_error && state.has_blocking_error(TaskFormField::Title) {
        theme.error_color()
    } else if is_focused {
        theme.border_focused()
    } else if is_valid && char_count > 0 {
        theme.success_color()
    } else {
        theme.foreground()
    };

    let value = if state.title.value().is_empty() && !is_focused {
        state.title.placeholder.clone()
    } else {
        state.title.value().to_string()
    };

    let paragraph = Paragraph::new(value)
        .block(
            Block::default()
                .title(label)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(if state.title.value().is_empty() {
            Color::DarkGray
        } else {
            theme.foreground()
        }));

    frame.render_widget(paragraph, area);

    // Show cursor if focused
    if is_focused && !state.title.value().is_empty() {
        let cursor_x = area.x + 1 + state.title.cursor_position as u16;
        let cursor_y = area.y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Render the parent task selector field
fn render_parent_field(
    frame: &mut Frame,
    area: Rect,
    state: &TaskCreationModalState,
    theme: &Theme,
) {
    let is_focused = state.focused_field == TaskFormField::Parent;
    let label = "Parent Task";

    let style = if is_focused {
        Style::default().fg(theme.border_focused())
    } else {
        Style::default().fg(theme.foreground())
    };

    let value = if let Some(selected_label) = &state.parent_selector.selected_item {
        format!("{} [▼]", selected_label.title)
    } else {
        "None (top-level task) [▼]".to_string()
    };

    let paragraph = Paragraph::new(value)
        .block(
            Block::default()
                .title(label)
                .borders(Borders::ALL)
                .border_style(style),
        )
        .style(Style::default().fg(theme.foreground()));

    frame.render_widget(paragraph, area);
}

/// Render the labels chip input field with validation indicator
fn render_labels_field(
    frame: &mut Frame,
    area: Rect,
    state: &TaskCreationModalState,
    theme: &Theme,
) {
    let is_focused = state.focused_field == TaskFormField::Labels;
    let has_error = state.has_error(TaskFormField::Labels);

    // Build label with hint
    let label = if has_error {
        "Labels ✗"
    } else {
        "Labels (type and press Enter)"
    };

    // Style based on state
    let border_color = if has_error && state.has_blocking_error(TaskFormField::Labels) {
        theme.error_color()
    } else if is_focused {
        theme.border_focused()
    } else {
        theme.foreground()
    };

    // Build chip display
    let mut spans = Vec::new();
    for chip in &state.labels.chips {
        spans.push(Span::styled(
            format!("[{chip}] "),
            Style::default()
                .fg(theme.label_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Add current input if any
    if !state.labels.input.is_empty() {
        spans.push(Span::raw(&state.labels.input));
    }

    let paragraph = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .title(label)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(theme.foreground()));

    frame.render_widget(paragraph, area);

    // Show cursor if focused
    if is_focused {
        let cursor_x = area.x
            + 1
            + (state.labels.chips.len() * 10) as u16
            + state.labels.cursor_position as u16;
        let cursor_y = area.y + 1;
        frame.set_cursor_position((cursor_x.min(area.x + area.width - 2), cursor_y));
    }
}

/// Render the status radio selector
fn render_status_field(
    frame: &mut Frame,
    area: Rect,
    state: &TaskCreationModalState,
    theme: &Theme,
) {
    let is_focused = state.focused_field == TaskFormField::Status;
    let label = "Status";

    let style = if is_focused {
        Style::default().fg(theme.border_focused())
    } else {
        Style::default().fg(theme.foreground())
    };

    // Build radio options display
    let mut spans = Vec::new();
    for (i, option) in state.status.options.iter().enumerate() {
        let is_selected = i == state.status.selected_index;
        let marker = if is_selected { "(*) " } else { "( ) " };

        spans.push(Span::styled(
            marker,
            Style::default().fg(if is_selected {
                theme.border_focused()
            } else {
                theme.foreground()
            }),
        ));
        spans.push(Span::styled(
            format!("{} ", option.label),
            Style::default().fg(if is_selected {
                theme.border_focused()
            } else {
                theme.foreground()
            }),
        ));
    }

    let paragraph = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .title(label)
            .borders(Borders::ALL)
            .border_style(style),
    );

    frame.render_widget(paragraph, area);
}

/// Render the owner and estimate fields side by side
fn render_owner_estimate_fields(
    frame: &mut Frame,
    area: Rect,
    state: &TaskCreationModalState,
    theme: &Theme,
) {
    // Split into two columns
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Owner field
    {
        let is_focused = state.focused_field == TaskFormField::Owner;
        let label = "Owner";

        let style = if is_focused {
            Style::default().fg(theme.border_focused())
        } else {
            Style::default().fg(theme.foreground())
        };

        let value = if state.owner.value().is_empty() && !is_focused {
            state.owner.placeholder.clone()
        } else {
            state.owner.value().to_string()
        };

        let paragraph = Paragraph::new(value)
            .block(
                Block::default()
                    .title(label)
                    .borders(Borders::ALL)
                    .border_style(style),
            )
            .style(Style::default().fg(if state.owner.value().is_empty() {
                Color::DarkGray
            } else {
                theme.foreground()
            }));

        frame.render_widget(paragraph, chunks[0]);

        // Show cursor if focused
        if is_focused && !state.owner.value().is_empty() {
            let cursor_x = chunks[0].x + 1 + state.owner.cursor_position as u16;
            let cursor_y = chunks[0].y + 1;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    // Estimate field with validation
    {
        let is_focused = state.focused_field == TaskFormField::Estimate;
        let has_error = state.has_error(TaskFormField::Estimate);

        // Build label with hint
        let label = if has_error {
            "Estimate ⚠"
        } else {
            "Estimate"
        };

        // Style based on state
        let border_color = if has_error {
            theme.warning_color()
        } else if is_focused {
            theme.border_focused()
        } else {
            theme.foreground()
        };

        let value = if state.estimate.value().is_empty() && !is_focused {
            state.estimate.placeholder.clone()
        } else {
            state.estimate.value().to_string()
        };

        let paragraph = Paragraph::new(value)
            .block(
                Block::default()
                    .title(label)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .style(Style::default().fg(if state.estimate.value().is_empty() {
                Color::DarkGray
            } else {
                theme.foreground()
            }));

        frame.render_widget(paragraph, chunks[1]);

        // Show cursor if focused
        if is_focused && !state.estimate.value().is_empty() {
            let cursor_x = chunks[1].x + 1 + state.estimate.cursor_position as u16;
            let cursor_y = chunks[1].y + 1;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

/// Render the agent note text area field
fn render_agent_note_field(
    frame: &mut Frame,
    area: Rect,
    state: &TaskCreationModalState,
    theme: &Theme,
) {
    let is_focused = state.focused_field == TaskFormField::AgentNote;
    let label = "Agent Note (optional)";

    let style = if is_focused {
        Style::default().fg(theme.border_focused())
    } else {
        Style::default().fg(theme.foreground())
    };

    let content = if state.agent_note.lines.is_empty() && !is_focused {
        vec![Line::from(Span::styled(
            "Notes for AI agents...",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        state
            .agent_note
            .lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect()
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(label)
                .borders(Borders::ALL)
                .border_style(style),
        )
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(theme.foreground()));

    frame.render_widget(paragraph, area);

    // Show cursor if focused
    if is_focused {
        let cursor_x = area.x + 1 + state.agent_note.cursor_col as u16;
        let cursor_y = area.y + 1 + state.agent_note.cursor_row as u16;
        frame.set_cursor_position((
            cursor_x.min(area.x + area.width - 2),
            cursor_y.min(area.y + area.height - 2),
        ));
    }
}

/// Render help line at the bottom
fn render_help_line(frame: &mut Frame, area: Rect, theme: &Theme) {
    let help = Line::from(vec![
        Span::styled("Tab", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Next | "),
        Span::styled("Shift+Tab", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Prev | "),
        Span::styled("Ctrl+U", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Clear | "),
        Span::styled("Ctrl+S", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Save | "),
        Span::styled("F1", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Help | "),
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Cancel"),
    ]);

    let paragraph = Paragraph::new(help)
        .style(Style::default().fg(theme.muted_color()))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
