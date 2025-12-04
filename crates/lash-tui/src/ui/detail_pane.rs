//! Detail pane rendering

use lash_db::repository::tasks::TaskRecord;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::state::{AppState, FocusedPane};
use crate::ui::themes;
use lash_types::tree::TreeNode;

/// Render the detail pane
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.focused_pane == FocusedPane::Detail;
    let theme = &state.theme;

    let border_style = if is_focused {
        themes::focused_border_style(theme)
    } else {
        themes::unfocused_border_style(theme)
    };

    // Get title - show label filter if active, otherwise selected file
    let title = if let Some(label) = &state.current_label_filter {
        format!(" Tasks [#{label}] ")
    } else {
        state
            .selected_file_title()
            .map_or_else(|| " Tasks ".to_string(), |t| format!(" {t} "))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    if state.tasks.is_empty() {
        // Show empty state
        let text = if state.files.is_empty() {
            "No files indexed. Run 'lash index' to index your project."
        } else {
            "No tasks in this file."
        };
        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    } else {
        // Check if we need to show description and split the area
        let (description_area, tasks_area) = if should_show_description(state) {
            split_area_for_description(area, state)
        } else {
            (None, area)
        };

        // Render description if we have one to show
        if let Some(desc_area) = description_area {
            render_description(frame, desc_area, state);
        }

        // Check if tree view is available
        let items: Vec<ListItem> = if let Some(task_trees) = &state.task_tree {
            render_task_tree(task_trees, state, theme)
        } else {
            render_flat_task_list(state, theme)
        };

        let list = List::new(items)
            .block(block)
            .highlight_style(themes::selected_style(theme));

        // Create list state with current selection
        let mut list_state = ListState::default();
        list_state.select(Some(state.selected_task_index));

        frame.render_stateful_widget(list, tasks_area, &mut list_state);
    }
}

/// Check if we should show the description section
fn should_show_description(state: &AppState) -> bool {
    // Only show description if we're viewing a single file (not a label filter)
    // and the selected file has a non-empty description
    if state.current_label_filter.is_some() {
        return false;
    }

    if let Some(selected_node) = state.selected_tree_node() {
        if let Some(file) = selected_node.file_record.as_ref() {
            return !file.description.is_empty();
        }
    }

    false
}

/// Split the area to make room for description above the tasks
fn split_area_for_description(area: Rect, state: &AppState) -> (Option<Rect>, Rect) {
    // Get the description to calculate how much space we need
    let description = if let Some(selected_node) = state.selected_tree_node() {
        selected_node
            .file_record
            .as_ref()
            .map(|f| f.description.clone())
    } else {
        None
    };

    let Some(desc_text) = description else {
        return (None, area);
    };

    // Calculate lines needed for description (limit to 5 lines for now)
    // Each line is roughly 80 chars at typical terminal width
    let lines_needed = calculate_lines_needed(&desc_text, area.width.saturating_sub(4) as usize);
    let desc_height = lines_needed.clamp(2, 5); // Min 2 lines, max 5 lines

    // Split vertically: description area + tasks area
    #[allow(clippy::cast_possible_truncation)]
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(desc_height as u16 + 2), // +2 for borders
            Constraint::Min(1),                         // Tasks area
        ])
        .split(area);

    (Some(chunks[0]), chunks[1])
}

/// Calculate how many lines are needed to display text with wrapping
fn calculate_lines_needed(text: &str, width: usize) -> usize {
    if width == 0 {
        return text.lines().count();
    }

    text.lines()
        .map(|line| {
            if line.is_empty() {
                1
            } else {
                line.len().div_ceil(width) // Ceiling division
            }
        })
        .sum()
}

/// Render the description section
fn render_description(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;

    // Get description from selected file
    let description = if let Some(selected_node) = state.selected_tree_node() {
        selected_node.file_record.as_ref().and_then(|f| {
            if f.description.is_empty() {
                None
            } else {
                Some(f.description.clone())
            }
        })
    } else {
        None
    };

    let Some(desc_text) = description else {
        return;
    };

    // Parse description and highlight @agent-note annotations
    let lines = parse_description_with_highlights(&desc_text, theme);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Description ")
        .border_style(themes::unfocused_border_style(theme));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Parse description text and highlight @agent-note annotations
fn parse_description_with_highlights(
    text: &str,
    theme: &crate::colors::Theme,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for line in text.lines() {
        if line.contains("@agent-note") {
            // Parse line with @agent-note highlighting
            let mut spans = Vec::new();
            let mut remaining = line;

            while let Some(idx) = remaining.find("@agent-note") {
                // Add text before @agent-note
                if idx > 0 {
                    spans.push(Span::raw(remaining[..idx].to_string()));
                }

                // Find the end of the annotation (end of line or next space after colon)
                let annotation_start = idx;
                let after_annotation = &remaining[annotation_start..];

                // Find where the annotation value ends (at next sentence or line end)
                let annotation_end = if let Some(colon_idx) = after_annotation.find(':') {
                    // Find the end of the sentence or line
                    let after_colon = &after_annotation[colon_idx + 1..];
                    after_colon.find('.').map_or(remaining.len(), |i| {
                        annotation_start + colon_idx + 1 + i + 1
                    })
                } else {
                    // No colon, just highlight the keyword
                    annotation_start + "@agent-note".len()
                };

                // Add highlighted annotation
                spans.push(Span::styled(
                    remaining[annotation_start..annotation_end].to_string(),
                    Style::default()
                        .fg(theme.warning_color())
                        .add_modifier(Modifier::BOLD),
                ));

                remaining = &remaining[annotation_end..];
            }

            // Add remaining text
            if !remaining.is_empty() {
                spans.push(Span::raw(remaining.to_string()));
            }

            lines.push(Line::from(spans));
        } else {
            // Regular line without annotation
            lines.push(Line::from(line.to_string()));
        }
    }

    lines
}

/// Render tasks in tree view
fn render_task_tree(
    trees: &[TreeNode<TaskRecord>],
    state: &AppState,
    theme: &crate::colors::Theme,
) -> Vec<ListItem<'static>> {
    let mut items = Vec::new();
    let chars = state.tree_chars;

    for tree in trees {
        render_task_node(tree, &mut items, state, theme, chars, &[], true);
    }

    items
}

/// Recursively render a task tree node
fn render_task_node(
    node: &TreeNode<TaskRecord>,
    items: &mut Vec<ListItem<'static>>,
    state: &AppState,
    theme: &crate::colors::Theme,
    chars: lash_types::tree::TreeChars,
    ancestors_is_last: &[bool],
    is_last: bool,
) {
    let current_index = items.len();
    let is_selected = current_index == state.selected_task_index;

    // Build tree prefix
    let prefix = build_tree_prefix(node.depth, is_last, ancestors_is_last, chars);

    // Add expand/collapse indicator
    let expand_indicator = if node.has_children() {
        if node.expanded {
            chars.expanded()
        } else {
            chars.collapsed()
        }
    } else {
        ""
    };

    // Get checkbox character
    let checkbox = themes::checkbox_char(node.data.status, theme);

    let style = if is_selected {
        themes::selected_style(theme)
    } else {
        themes::status_style(node.data.status, theme)
    };

    let line = Line::from(vec![
        Span::raw(prefix),
        Span::raw(expand_indicator),
        Span::raw(checkbox),
        Span::raw(" "),
        Span::styled(node.data.title.clone(), style),
    ]);

    items.push(ListItem::new(line));

    // Recursively render children if expanded
    if node.expanded {
        let mut new_ancestors = ancestors_is_last.to_vec();
        new_ancestors.push(is_last);

        let child_count = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            let is_last_child = i == child_count - 1;
            render_task_node(
                child,
                items,
                state,
                theme,
                chars,
                &new_ancestors,
                is_last_child,
            );
        }
    }
}

/// Render tasks in flat list view (fallback)
fn render_flat_task_list(state: &AppState, theme: &crate::colors::Theme) -> Vec<ListItem<'static>> {
    state
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let indent = "  ".repeat(task.depth as usize);
            let checkbox = themes::checkbox_char(task.status, theme);

            let style = if i == state.selected_task_index {
                themes::selected_style(theme)
            } else {
                themes::status_style(task.status, theme)
            };

            let line = Line::from(vec![
                Span::raw(indent),
                Span::raw(checkbox),
                Span::raw(" "),
                Span::styled(task.title.clone(), style),
            ]);

            ListItem::new(line)
        })
        .collect()
}

/// Build tree prefix string for rendering
fn build_tree_prefix(
    depth: usize,
    is_last: bool,
    ancestors_is_last: &[bool],
    chars: lash_types::tree::TreeChars,
) -> String {
    let mut prefix = String::new();

    // Add indentation for ancestor levels
    for &ancestor_is_last in ancestors_is_last {
        if ancestor_is_last {
            prefix.push_str(chars.empty());
        } else {
            prefix.push_str(chars.vertical());
        }
    }

    // Add branch character for current level
    if depth > 0 {
        if is_last {
            prefix.push_str(chars.last_branch());
        } else {
            prefix.push_str(chars.branch());
        }
    }

    prefix
}
