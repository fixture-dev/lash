//! Task detail modal rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::state::AppState;

/// Render task detail modal overlay
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(detail_state) = &state.task_detail_state else {
        return;
    };

    // Create centered popup (80% width, 85% height)
    let popup_area = centered_rect(80, 85, area);

    // Clear the area
    frame.render_widget(Clear, popup_area);

    // Create bordered block with task title
    let block = Block::default()
        .title(format!(" {} ", detail_state.task.title))
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    // Build content sections
    let content_lines = build_content_lines(state, detail_state);

    // Create paragraph with wrapping
    #[allow(clippy::cast_possible_truncation)]
    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((detail_state.scroll_offset as u16, 0));

    frame.render_widget(paragraph, popup_area);
}

/// Build all content lines for the task detail view
fn build_content_lines(
    state: &AppState,
    detail_state: &crate::state::TaskDetailState,
) -> Vec<Line<'static>> {
    let mut content_lines = Vec::new();

    // Header section
    add_header_section(&mut content_lines, state, detail_state);

    // Metadata section
    add_metadata_section(&mut content_lines, state, detail_state);

    // Labels section
    add_labels_section(&mut content_lines, state, detail_state);

    // Description section
    add_description_section(&mut content_lines, state, detail_state);

    // Dependencies section
    add_dependencies_section(&mut content_lines, state, detail_state);

    // Footer
    add_footer_section(&mut content_lines, state);

    content_lines
}

/// Add header section with task ID and status
fn add_header_section(
    lines: &mut Vec<Line<'static>>,
    state: &AppState,
    detail_state: &crate::state::TaskDetailState,
) {
    let checkbox = state.theme.checkbox_char(detail_state.task.status);
    let status_style = state.theme.task_status_style(detail_state.task.status);

    lines.push(Line::from(vec![
        Span::styled(
            "ID: ",
            Style::default()
                .fg(state.theme.emphasis_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(detail_state.task.full_id.clone()),
        Span::raw("  "),
        Span::styled(
            "Status: ",
            Style::default()
                .fg(state.theme.emphasis_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(checkbox.to_string(), status_style),
    ]));
    lines.push(Line::from(""));
}

/// Add metadata section with file, owner, and estimate
fn add_metadata_section(
    lines: &mut Vec<Line<'static>>,
    state: &AppState,
    detail_state: &crate::state::TaskDetailState,
) {
    lines.push(Line::from(vec![Span::styled(
        "Metadata",
        Style::default()
            .fg(state.theme.info_color())
            .add_modifier(Modifier::BOLD),
    )]));

    // File path
    lines.push(Line::from(vec![
        Span::styled(
            "  File: ",
            Style::default().fg(state.theme.emphasis_color()),
        ),
        Span::raw(detail_state.file_path.display().to_string()),
    ]));

    // Owner (if present)
    if let Some(owner) = &detail_state.task.owner {
        lines.push(Line::from(vec![
            Span::styled(
                "  Owner: ",
                Style::default().fg(state.theme.emphasis_color()),
            ),
            Span::raw(owner.clone()),
        ]));
    }

    // Estimate (if present)
    if let Some(estimate) = &detail_state.task.estimate {
        lines.push(Line::from(vec![
            Span::styled(
                "  Estimate: ",
                Style::default().fg(state.theme.emphasis_color()),
            ),
            Span::raw(estimate.clone()),
        ]));
    }

    lines.push(Line::from(""));
}

/// Add labels section
fn add_labels_section(
    lines: &mut Vec<Line<'static>>,
    state: &AppState,
    detail_state: &crate::state::TaskDetailState,
) {
    if detail_state.labels.is_empty() {
        return;
    }

    lines.push(Line::from(vec![Span::styled(
        "Labels",
        Style::default()
            .fg(state.theme.info_color())
            .add_modifier(Modifier::BOLD),
    )]));

    let label_spans: Vec<Span> = detail_state
        .labels
        .iter()
        .flat_map(|label| {
            vec![
                Span::raw("  "),
                Span::styled(
                    format!("#{label}"),
                    Style::default().fg(state.theme.label_color()),
                ),
            ]
        })
        .collect();

    lines.push(Line::from(label_spans));
    lines.push(Line::from(""));
}

/// Add description section
fn add_description_section(
    lines: &mut Vec<Line<'static>>,
    state: &AppState,
    detail_state: &crate::state::TaskDetailState,
) {
    let Some(body) = &detail_state.task.body else {
        return;
    };

    lines.push(Line::from(vec![Span::styled(
        "Description",
        Style::default()
            .fg(state.theme.info_color())
            .add_modifier(Modifier::BOLD),
    )]));

    // Split body into lines and add each with indentation
    for line in body.lines() {
        lines.push(Line::from(format!("  {line}")));
    }
    lines.push(Line::from(""));
}

/// Add dependencies section
fn add_dependencies_section(
    lines: &mut Vec<Line<'static>>,
    state: &AppState,
    detail_state: &crate::state::TaskDetailState,
) {
    if detail_state.dependencies.is_empty() {
        return;
    }

    lines.push(Line::from(vec![Span::styled(
        "Dependencies",
        Style::default()
            .fg(state.theme.info_color())
            .add_modifier(Modifier::BOLD),
    )]));

    for dep in &detail_state.dependencies {
        // Display the dependency with its kind
        // Prefer to_full_id if available, otherwise use raw_ref
        let dep_id = dep
            .to_full_id
            .as_deref()
            .or(dep.raw_ref.as_deref())
            .unwrap_or("unresolved");
        let kind_str = format!("{:?}", dep.kind);

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("→ ", Style::default().fg(state.theme.muted_color())),
            Span::styled(
                dep_id.to_string(),
                Style::default().fg(state.theme.emphasis_color()),
            ),
            Span::raw(" "),
            Span::styled(
                format!("({kind_str})"),
                Style::default().fg(state.theme.muted_color()),
            ),
        ]));
    }

    lines.push(Line::from(""));
}

/// Add footer section
fn add_footer_section(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Press Esc to close • j/k to scroll",
        Style::default().fg(state.theme.muted_color()),
    )]));
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
