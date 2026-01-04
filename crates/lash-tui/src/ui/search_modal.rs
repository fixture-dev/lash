//! Search modal rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::state::{AppState, SearchModalState};

/// Render search modal overlay
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(search_state) = &state.search_modal_state else {
        return;
    };

    // Create centered rect (80% width, 80% height)
    let popup_area = centered_rect(80, 80, area);

    // Clear the area
    frame.render_widget(Clear, popup_area);

    // Split into title, input, and results areas
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Input field
            Constraint::Min(0),    // Results
        ])
        .split(popup_area);

    // Render title
    render_title(frame, chunks[0], &state.theme);

    // Render input field
    render_input(frame, chunks[1], search_state, &state.theme);

    // Render results
    render_results(frame, chunks[2], search_state, &state.theme);
}

/// Render the title section
fn render_title(frame: &mut Frame, area: Rect, theme: &crate::colors::Theme) {
    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let title_text = vec![Line::from(vec![Span::styled(
        "Search Tasks - Type to search, Enter to select, Esc to close",
        Style::default()
            .fg(theme.info_color())
            .add_modifier(Modifier::BOLD),
    )])];

    let paragraph = Paragraph::new(title_text)
        .block(title_block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render the input field
fn render_input(
    frame: &mut Frame,
    area: Rect,
    search_state: &SearchModalState,
    theme: &crate::colors::Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Search ")
        .style(Style::default().bg(Color::Black).fg(Color::White));

    // Create input text with cursor
    let input = &search_state.input;
    let cursor_pos = search_state.cursor_position;

    // Build the input line with cursor indicator
    let mut spans = Vec::new();

    // Add the search prompt
    spans.push(Span::styled("/ ", Style::default().fg(theme.info_color())));

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

    let input_line = Line::from(spans);
    let paragraph = Paragraph::new(vec![input_line])
        .block(block)
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render search results
fn render_results(
    frame: &mut Frame,
    area: Rect,
    search_state: &SearchModalState,
    theme: &crate::colors::Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " Results ({}/{}) ",
            if search_state.results.is_empty() {
                0
            } else {
                search_state.selected_result_index + 1
            },
            search_state.total_count
        ))
        .style(Style::default().bg(Color::Black).fg(Color::White));

    // Handle different states
    if let Some(error) = &search_state.error {
        // Show error message
        let error_text = Paragraph::new(vec![Line::from(vec![Span::styled(
            error.as_str(),
            Style::default().fg(theme.error_color()),
        )])])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(error_text, area);
        return;
    }

    if !search_state.has_searched {
        // Show placeholder
        let placeholder = Paragraph::new(vec![Line::from(vec![Span::styled(
            "Type to search for tasks...",
            Style::default().fg(Color::DarkGray),
        )])])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(placeholder, area);
        return;
    }

    if search_state.results.is_empty() {
        // No results found
        let no_results = Paragraph::new(vec![Line::from(vec![Span::styled(
            "No tasks found matching your query",
            Style::default().fg(Color::Yellow),
        )])])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(no_results, area);
        return;
    }

    // Create list items from results
    let items: Vec<ListItem> = search_state
        .results
        .iter()
        .map(|result| {
            let status_color = match result.status {
                lash_types::TaskStatus::Done => theme.success_color(),
                lash_types::TaskStatus::Blocked => theme.error_color(),
                lash_types::TaskStatus::Waived => theme.muted_color(),
                lash_types::TaskStatus::InProgress => theme.task_in_progress(),
                lash_types::TaskStatus::Open => theme.foreground(),
            };

            let checkbox = match result.status {
                lash_types::TaskStatus::Done => "[x]",
                lash_types::TaskStatus::Blocked => "[!]",
                lash_types::TaskStatus::Waived => "[-]",
                lash_types::TaskStatus::InProgress => "[>]",
                lash_types::TaskStatus::Open => "[ ]",
            };

            // Build the result line
            let mut spans = vec![
                Span::styled(format!("{checkbox} "), Style::default().fg(status_color)),
                Span::styled(
                    truncate_string(&result.title, 50),
                    Style::default().fg(Color::White),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("({})", result.file_path),
                    Style::default().fg(Color::DarkGray),
                ),
            ];

            // Add labels if any
            if !result.labels.is_empty() {
                spans.push(Span::styled(" ", Style::default()));
                for label in &result.labels {
                    spans.push(Span::styled(
                        format!("#{label} "),
                        Style::default().fg(theme.label_color()),
                    ));
                }
            }

            // Add score indicator (subtle)
            spans.push(Span::styled(
                format!(" [{:.0}%]", result.score * 100.0),
                Style::default().fg(Color::DarkGray),
            ));

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
    list_state.select(Some(search_state.selected_result_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Truncate a string to a maximum length, adding ellipsis if needed
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
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
