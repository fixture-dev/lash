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
    use crate::state::TaskDetailSection;

    let mut content_lines = Vec::new();

    // Header section
    add_header_section(&mut content_lines, state, detail_state);

    // Metadata section
    let metadata_selected = matches!(
        detail_state.selected_section,
        Some(TaskDetailSection::Metadata)
    );
    add_metadata_section(&mut content_lines, state, detail_state, metadata_selected);

    // Labels section
    let selected_label_index =
        if let Some(TaskDetailSection::Labels(idx)) = detail_state.selected_section {
            Some(idx)
        } else {
            None
        };
    add_labels_section(
        &mut content_lines,
        state,
        detail_state,
        selected_label_index,
    );

    // Description section
    add_description_section(&mut content_lines, state, detail_state);

    // Subtasks section
    add_subtasks_section(&mut content_lines, state, detail_state);

    // Dependencies section
    let selected_dep_index =
        if let Some(TaskDetailSection::Parent(idx)) = detail_state.selected_section {
            Some(idx)
        } else {
            None
        };
    add_dependencies_section(&mut content_lines, state, detail_state, selected_dep_index);

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
    is_selected: bool,
) {
    let section_style = if is_selected {
        state.theme.selected_style()
    } else {
        Style::default()
            .fg(state.theme.info_color())
            .add_modifier(Modifier::BOLD)
    };

    lines.push(Line::from(vec![Span::styled("Metadata", section_style)]));

    // File path - use selected style if this section is selected
    let file_style = if is_selected {
        state.theme.selected_style()
    } else {
        Style::default()
    };

    lines.push(Line::from(vec![
        Span::styled(
            "  File: ",
            if is_selected {
                state.theme.selected_style()
            } else {
                Style::default().fg(state.theme.emphasis_color())
            },
        ),
        Span::styled(detail_state.file_path.display().to_string(), file_style),
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
    selected_label_index: Option<usize>,
) {
    if detail_state.labels.is_empty() {
        return;
    }

    let section_selected = selected_label_index.is_some();
    let section_style = if section_selected {
        state.theme.selected_style()
    } else {
        Style::default()
            .fg(state.theme.info_color())
            .add_modifier(Modifier::BOLD)
    };

    lines.push(Line::from(vec![Span::styled("Labels", section_style)]));

    let label_spans: Vec<Span> = detail_state
        .labels
        .iter()
        .enumerate()
        .flat_map(|(idx, label)| {
            let is_this_label_selected = selected_label_index == Some(idx);
            let label_style = if is_this_label_selected {
                state.theme.selected_style()
            } else {
                Style::default().fg(state.theme.label_color())
            };

            vec![
                Span::raw("  "),
                Span::styled(format!("#{label}"), label_style),
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
    selected_dep_index: Option<usize>,
) {
    use lash_types::DependencyKind;

    if detail_state.dependencies.is_empty() {
        return;
    }

    let section_selected = selected_dep_index.is_some();
    let section_style = if section_selected {
        state.theme.selected_style()
    } else {
        Style::default()
            .fg(state.theme.info_color())
            .add_modifier(Modifier::BOLD)
    };

    // Use "Parent" as label since hierarchy deps show parent task
    lines.push(Line::from(vec![Span::styled("Parent", section_style)]));

    for (idx, dep) in detail_state.dependencies.iter().enumerate() {
        let is_this_dep_selected = selected_dep_index == Some(idx);
        // Display the dependency with its kind
        // Prefer to_full_id if available, otherwise use raw_ref
        let dep_id = dep
            .to_full_id
            .as_deref()
            .or(dep.raw_ref.as_deref())
            .unwrap_or("unresolved");

        // Format kind label for display
        let kind_label = match dep.kind {
            DependencyKind::Hierarchy => "Hierarchy",
            DependencyKind::ExplicitId | DependencyKind::ExplicitPath => "Explicit",
            DependencyKind::Directory => "Directory",
        };

        // Build the line with ID and optional title
        // Use selected style if this dependency is selected
        let mut spans = vec![
            Span::raw("  "),
            Span::styled(
                "← ",
                if is_this_dep_selected {
                    state.theme.selected_style()
                } else {
                    Style::default().fg(state.theme.muted_color())
                },
            ),
            Span::styled(
                dep_id.to_string(),
                if is_this_dep_selected {
                    state.theme.selected_style()
                } else {
                    Style::default().fg(state.theme.emphasis_color())
                },
            ),
        ];

        // Add title in distinct color if available
        if let Some(title) = &dep.to_title {
            spans.push(Span::styled(
                " | ",
                if is_this_dep_selected {
                    state.theme.selected_style()
                } else {
                    Style::default().fg(state.theme.muted_color())
                },
            ));
            spans.push(Span::styled(
                title.clone(),
                if is_this_dep_selected {
                    state.theme.selected_style()
                } else {
                    Style::default().fg(state.theme.label_color())
                },
            ));
        }

        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("({kind_label})"),
            if is_this_dep_selected {
                state.theme.selected_style()
            } else {
                Style::default().fg(state.theme.muted_color())
            },
        ));

        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
}

/// Add subtasks section
fn add_subtasks_section(
    lines: &mut Vec<Line<'static>>,
    state: &AppState,
    detail_state: &crate::state::TaskDetailState,
) {
    if detail_state.subtasks.is_empty() {
        return;
    }

    lines.push(Line::from(vec![Span::styled(
        "Subtasks",
        Style::default()
            .fg(state.theme.info_color())
            .add_modifier(Modifier::BOLD),
    )]));

    for subtask in &detail_state.subtasks {
        let checkbox = state.theme.checkbox_char(subtask.status);
        let status_style = state.theme.task_status_style(subtask.status);

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(checkbox.to_string(), status_style),
            Span::raw(" "),
            Span::styled(subtask.title.clone(), status_style),
        ]));
    }

    lines.push(Line::from(""));
}

/// Add footer section
fn add_footer_section(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Tab/Shift+Tab to navigate sections • h/l to select items • Enter to activate • Esc to close",
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
