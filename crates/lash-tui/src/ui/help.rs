//! Help overlay rendering

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::state::AppState;

/// Render help overlay
pub fn render(frame: &mut Frame, area: Rect, _state: &AppState) {
    // Create centered rect
    let popup_area = centered_rect(60, 70, area);

    // Clear the area
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let help_text = vec![
        Line::from(vec![Span::styled(
            "Lash TUI - Keyboard Commands",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  j/k or ↑/↓     Move selection up/down"),
        Line::from("  h              Collapse/go to parent"),
        Line::from("  l/Enter        Expand/open selected item"),
        Line::from("  gg             Go to top"),
        Line::from("  G              Go to bottom"),
        Line::from("  {/}            Jump to prev/next top-level task"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Panes:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Tab            Switch between panes"),
        Line::from("  Ctrl-h/l       Switch between panes"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Space          Toggle task status"),
        Line::from("  e              Open file in $EDITOR"),
        Line::from("  /              Search"),
        Line::from("  f              Open label filter selector"),
        Line::from("  F              Toggle Files/Labels view"),
        Line::from("  c              Clear label filter"),
        Line::from("  Ctrl-g         Show dependency graph"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "General:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ?              Toggle this help"),
        Line::from("  t              Open theme selector"),
        Line::from("  q or Ctrl-c    Quit"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press ? or Esc to close this help",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, popup_area);
}

/// Create a centered rect within the given area
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
