//! Theme selector modal rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::colors::{ColorScheme, REGISTRY};
use crate::state::{AppState, ThemeSelectorState};

/// Render theme selector overlay
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(selector_state) = &state.theme_selector_state else {
        return;
    };

    // Create centered rect (wider than help to accommodate swatches)
    let popup_area = centered_rect(80, 80, area);

    // Clear the area
    frame.render_widget(Clear, popup_area);

    // Split into title and content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(popup_area);

    // Render title
    render_title(frame, chunks[0]);

    // Render scheme list with swatches
    render_scheme_list(frame, chunks[1], selector_state, &state.theme);
}

/// Render the title section
fn render_title(frame: &mut Frame, area: Rect) {
    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let title_text = vec![Line::from(vec![Span::styled(
        "Theme Selector - Navigate with j/k, select with Enter, close with Esc",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )])];

    let paragraph = Paragraph::new(title_text)
        .block(title_block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render the scheme list with color swatches
fn render_scheme_list(
    frame: &mut Frame,
    area: Rect,
    selector_state: &ThemeSelectorState,
    current_theme: &crate::colors::Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    // Handle empty scheme list (should never happen in production)
    if selector_state.scheme_names.is_empty() {
        let empty_msg = Paragraph::new("No color schemes available")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(empty_msg, area);
        return;
    }

    // Create list items with swatches
    let items: Vec<ListItem> = selector_state
        .scheme_names
        .iter()
        .map(|name| {
            let is_current = name == current_theme.name();
            let scheme = REGISTRY.get_scheme(name).unwrap();

            // Create swatch preview (2 rows of 8 colors)
            let swatch_line = create_swatch_line(scheme);

            // Create the item line
            let mut spans = vec![];

            // Current indicator
            if is_current {
                spans.push(Span::styled(
                    "● ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::raw("  "));
            }

            // Scheme name (truncate if too long)
            let display_name = if name.len() > 28 {
                format!("{}… ", &name[..27])
            } else {
                format!("{name:<30}")
            };
            spans.push(Span::raw(display_name));

            // Swatch
            spans.extend(swatch_line);

            let line = Line::from(spans);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    // Create list state with current selection
    let mut list_state = ListState::default();
    list_state.select(Some(selector_state.selected_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Create a swatch line showing 16 colors in a 2x8 grid representation
///
/// Shows colors 0-7 in first row, 8-15 in second row, represented as colored blocks
fn create_swatch_line(scheme: &ColorScheme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();

    // First row: colors 0-7
    for i in 0..8 {
        let color = ColorScheme::hex_to_rgb(match i {
            0 => &scheme.color_01,
            1 => &scheme.color_02,
            2 => &scheme.color_03,
            3 => &scheme.color_04,
            4 => &scheme.color_05,
            5 => &scheme.color_06,
            6 => &scheme.color_07,
            7 => &scheme.color_08,
            _ => unreachable!(),
        });

        spans.push(Span::styled("█", Style::default().fg(color)));
    }

    spans.push(Span::raw(" "));

    // Second row: colors 8-15
    for i in 8..16 {
        let color = ColorScheme::hex_to_rgb(match i {
            8 => &scheme.color_09,
            9 => &scheme.color_10,
            10 => &scheme.color_11,
            11 => &scheme.color_12,
            12 => &scheme.color_13,
            13 => &scheme.color_14,
            14 => &scheme.color_15,
            15 => &scheme.color_16,
            _ => unreachable!(),
        });

        spans.push(Span::styled("█", Style::default().fg(color)));
    }

    spans
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
