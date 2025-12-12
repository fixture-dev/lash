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
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    // Show different help content based on context
    if state.is_task_creation_modal_open() {
        render_task_creation_help(frame, area, state);
    } else {
        render_main_help(frame, area, state);
    }
}

/// Render help for the main TUI view
fn render_main_help(frame: &mut Frame, area: Rect, state: &AppState) {
    // Create centered rect
    let popup_area = centered_rect(60, 70, area);

    // Clear the area
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let version = env!("CARGO_PKG_VERSION");
    let title = format!("Lash TUI v{version} - Keyboard Commands");
    let help_text = vec![
        Line::from(vec![Span::styled(
            title,
            Style::default()
                .fg(state.theme.info_color())
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
        Line::from("  a or n         Create new task"),
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
            "Dependency Types:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Hierarchy      Parent-child from markdown nesting"),
        Line::from("  Explicit       Declared via @depends-on annotation"),
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

/// Render help for the task creation modal
fn render_task_creation_help(frame: &mut Frame, area: Rect, state: &AppState) {
    // Create centered rect
    let popup_area = centered_rect(65, 75, area);

    // Clear the area
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Task Creation Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let help_text = vec![
        Line::from(vec![Span::styled(
            "Create New Task - Keyboard Shortcuts",
            Style::default()
                .fg(state.theme.info_color())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Form Navigation:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Tab            Next field"),
        Line::from("  Shift+Tab      Previous field"),
        Line::from("  Ctrl+S         Submit form"),
        Line::from("  Ctrl+Enter     Submit form"),
        Line::from("  Esc            Cancel / Close modal"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Text Input:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ← / →          Move cursor left/right"),
        Line::from("  Home / Ctrl+A  Go to beginning of line"),
        Line::from("  End / Ctrl+E   Go to end of line"),
        Line::from("  Ctrl+U         Clear current field"),
        Line::from("  Backspace      Delete character before cursor"),
        Line::from("  Delete         Delete character at cursor"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Parent Selector:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ↑ / ↓          Navigate through tasks"),
        Line::from("  Enter          Select highlighted task"),
        Line::from("  Ctrl+U         Clear selection (top-level)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Labels Field:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Enter          Add typed label as chip"),
        Line::from("  ,              Add typed label as chip"),
        Line::from("  Backspace      Delete last chip (when input empty)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Status Field:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ← / →          Cycle through status options"),
        Line::from("  o / d / w / b  Quick select (Open/Done/Waived/Blocked)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Agent Note (Multi-line):",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ↑ / ↓          Move cursor up/down"),
        Line::from("  Enter          Insert new line"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press F1 or Esc to close this help",
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
