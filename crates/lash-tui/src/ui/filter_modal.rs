//! Filter modal rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::state::{AppState, FilterModalState};

/// Render filter modal overlay
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(filter_state) = &state.filter_modal_state else {
        return;
    };

    // Create centered rect (60% width, 70% height)
    let popup_area = centered_rect(60, 70, area);

    // Clear the area
    frame.render_widget(Clear, popup_area);

    // Split into title, input, and label list areas
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Input field (optional filter)
            Constraint::Min(0),    // Label list
        ])
        .split(popup_area);

    // Render title
    render_title(frame, chunks[0], &state.theme);

    // Render input field
    render_input(frame, chunks[1], filter_state, &state.theme);

    // Render label list
    render_label_list(
        frame,
        chunks[2],
        filter_state,
        &state.theme,
        state.current_label_filter.as_ref(),
    );
}

/// Render the title section
fn render_title(frame: &mut Frame, area: Rect, theme: &crate::colors::Theme) {
    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let title_text = vec![Line::from(vec![Span::styled(
        "Filter by Label - ↑↓ to navigate, Enter to apply, Esc to close",
        Style::default()
            .fg(theme.info_color())
            .add_modifier(Modifier::BOLD),
    )])];

    let paragraph = Paragraph::new(title_text)
        .block(title_block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render the input field for filtering labels
fn render_input(
    frame: &mut Frame,
    area: Rect,
    filter_state: &FilterModalState,
    theme: &crate::colors::Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Filter ")
        .style(Style::default().bg(Color::Black).fg(Color::White));

    // Create input text with cursor
    let input = &filter_state.input;
    let cursor_pos = filter_state.cursor_position;

    // Build the input line with cursor indicator
    let mut spans = Vec::new();

    // Add prompt
    if input.is_empty() {
        spans.push(Span::styled(
            "Type to filter labels...",
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        // Text before cursor
        if cursor_pos > 0 {
            spans.push(Span::raw(&input[..cursor_pos]));
        }

        // Cursor character (block cursor style)
        if cursor_pos < input.len() {
            spans.push(Span::styled(
                &input[cursor_pos..=cursor_pos],
                Style::default()
                    .bg(Color::White)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
            // Text after cursor
            if cursor_pos + 1 < input.len() {
                spans.push(Span::raw(&input[cursor_pos + 1..]));
            }
        } else {
            // Cursor at end of input - show block cursor
            spans.push(Span::styled(
                " ",
                Style::default()
                    .bg(Color::White)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }

    let input_line = Line::from(spans);
    let paragraph = Paragraph::new(vec![input_line])
        .block(block)
        .style(Style::default().fg(theme.foreground()));

    frame.render_widget(paragraph, area);
}

/// Render label list
fn render_label_list(
    frame: &mut Frame,
    area: Rect,
    filter_state: &FilterModalState,
    theme: &crate::colors::Theme,
    current_filter: Option<&String>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " Labels ({}/{}) ",
            filter_state.filtered_indices.len(),
            filter_state.available_labels.len()
        ))
        .style(Style::default().bg(Color::Black).fg(Color::White));

    // Handle empty state
    if filter_state.filtered_indices.is_empty() {
        let no_labels = Paragraph::new(vec![Line::from(vec![Span::styled(
            "No labels match your filter",
            Style::default().fg(Color::Yellow),
        )])])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(no_labels, area);
        return;
    }

    // Create list items from filtered labels
    let items: Vec<ListItem> = filter_state
        .filtered_indices
        .iter()
        .map(|&idx| {
            let label = &filter_state.available_labels[idx];
            let is_current_filter = current_filter == Some(&label.name);

            // Build the label line
            let spans = vec![
                // Show indicator for currently active filter
                if is_current_filter {
                    Span::styled("● ", Style::default().fg(theme.success_color()))
                } else {
                    Span::raw("  ")
                },
                // Label name
                Span::styled(
                    format!("#{}", label.name),
                    Style::default().fg(theme.label_color()),
                ),
                // Spacing
                Span::raw(" "),
                // Task count
                Span::styled(
                    format!("({} tasks)", label.task_count),
                    Style::default().fg(Color::DarkGray),
                ),
            ];

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    // Create list state with current selection
    let mut list_state = ListState::default();
    list_state.select(Some(filter_state.selected_index));

    frame.render_stateful_widget(list, area, &mut list_state);
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
