//! Detail pane rendering

use lash_core::display;
use lash_db::repository::tasks::TaskRecord;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use rusqlite::Connection;
use std::collections::HashSet;

use crate::state::{AppState, FocusedPane};
use crate::ui::themes;
use crate::utils;
use lash_types::tree::TreeNode;

/// Render the detail pane
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, conn: &Connection) {
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

    // First, check if we need to show description and split the area
    // This is done regardless of task state - the selected file may have a description
    // even if its tasks haven't been loaded yet (e.g., when navigating with Up/Down)
    let (description_area, content_area) = if should_show_description(state) {
        split_area_for_description(area, state)
    } else {
        (None, area)
    };

    // Render description if we have one to show
    if let Some(desc_area) = description_area {
        render_description(frame, desc_area, state);
    }

    // Now render the task content (or empty state message)
    if state.tasks.is_empty() {
        // Show empty state
        let text = if state.files.is_empty() {
            "No files indexed. Run 'lash index' to index your project."
        } else {
            "No tasks in this file."
        };
        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, content_area);
    } else {
        // Check if tree view is available
        let items: Vec<ListItem> = if let Some(task_trees) = &state.task_tree {
            render_task_tree(task_trees, state, theme, conn)
        } else {
            render_flat_task_list(state, theme, conn)
        };

        let list = List::new(items)
            .block(block)
            .highlight_style(themes::selected_style(theme));

        // Create list state with current selection
        let mut list_state = ListState::default();
        list_state.select(Some(state.selected_task_index));

        frame.render_stateful_widget(list, content_area, &mut list_state);
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
fn split_area_for_description(area: Rect, _state: &AppState) -> (Option<Rect>, Rect) {
    // Use a fixed percentage for description area to allow scrolling
    // 30% for description, 70% for tasks
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30), // Description area (scrollable)
            Constraint::Percentage(70), // Tasks area
        ])
        .split(area);

    (Some(chunks[0]), chunks[1])
}

/// Render the description section
fn render_description(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let is_focused = state.focused_pane == FocusedPane::Description;

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

    // Build title with scroll hint when focused
    let title = if is_focused {
        " Description (j/k to scroll) "
    } else {
        " Description "
    };

    let border_style = if is_focused {
        themes::focused_border_style(theme)
    } else {
        themes::unfocused_border_style(theme)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    // Apply scroll offset when focused
    #[allow(clippy::cast_possible_truncation)]
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((state.description_scroll as u16, 0));

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
    conn: &Connection,
) -> Vec<ListItem<'static>> {
    let mut items = Vec::new();
    let chars = state.tree_chars;

    // Check if current file is an index file
    // Try tree view first (like task loading does), then fall back to flat list
    let is_index = if let Some(selected) = state.selected_tree_node() {
        selected
            .file_record
            .is_some_and(|f| display::is_index_file(&f.path))
    } else {
        state
            .selected_file()
            .is_some_and(|f| display::is_index_file(&f.path))
    };

    // Pre-compute cross-file links if viewing an index file
    let cross_file_links: HashSet<i64> = if is_index {
        state
            .tasks
            .iter()
            .filter(|t| utils::is_cross_file_link(conn, t.id))
            .map(|t| t.id)
            .collect()
    } else {
        HashSet::new()
    };

    for tree in trees {
        render_task_node(
            tree,
            &mut items,
            state,
            theme,
            chars,
            &[],
            true,
            is_index,
            &cross_file_links,
        );
    }

    items
}

/// Recursively render a task tree node
#[allow(clippy::too_many_arguments)]
fn render_task_node(
    node: &TreeNode<TaskRecord>,
    items: &mut Vec<ListItem<'static>>,
    state: &AppState,
    theme: &crate::colors::Theme,
    chars: lash_types::tree::TreeChars,
    ancestors_is_last: &[bool],
    is_last: bool,
    is_index_file: bool,
    cross_file_links: &HashSet<i64>,
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

    // Format title - for index files, extract link text and convert annotations to hashtags
    let title = if is_index_file {
        display::format_index_title(&node.data.title)
    } else {
        node.data.title.clone()
    };

    // Check if this task is a cross-file link
    let is_cross_file_link = cross_file_links.contains(&node.data.id);

    // Build the line with styled labels (hashtags) and cross-file link indicator
    let line = build_styled_task_line(
        prefix,
        expand_indicator,
        checkbox,
        &title,
        style,
        theme,
        is_cross_file_link,
    );

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
                is_index_file,
                cross_file_links,
            );
        }
    }
}

/// Build a styled task line with colored labels
#[allow(clippy::too_many_arguments)]
fn build_styled_task_line(
    prefix: String,
    expand_indicator: &str,
    checkbox: &str,
    title: &str,
    base_style: Style,
    theme: &crate::colors::Theme,
    is_cross_file_link: bool,
) -> Line<'static> {
    let mut spans = vec![
        Span::raw(prefix),
        Span::raw(expand_indicator.to_string()),
        Span::raw(checkbox.to_string()),
        Span::raw(" ".to_string()),
    ];

    // Add cross-file link indicator if applicable
    if is_cross_file_link {
        spans.push(Span::styled(
            "→ ".to_string(),
            Style::default().fg(theme.info_color()),
        ));
    }

    // Parse the title for labels and style them differently
    let mut chars = title.char_indices().peekable();
    let mut last_pos = 0;

    while let Some((i, c)) = chars.next() {
        if c == '#' {
            // Found potential label start
            // First, add the text before this point with base styling
            if i > last_pos {
                let text_segment = title[last_pos..i].to_string();
                spans.push(Span::styled(text_segment, base_style));
            }

            // Collect the label
            let label_start = i;
            let mut label_end = i + 1;

            while let Some(&(j, next_c)) = chars.peek() {
                if next_c.is_alphanumeric() || next_c == '-' || next_c == '_' {
                    label_end = j + next_c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }

            if label_end > label_start + 1 {
                let label = title[label_start..label_end].to_string();
                spans.push(Span::styled(label, themes::label_style(theme)));
            } else {
                spans.push(Span::styled(
                    title[label_start..label_end].to_string(),
                    base_style,
                ));
            }
            last_pos = label_end;
        }
    }

    // Add any remaining text with base styling
    if last_pos < title.len() {
        spans.push(Span::styled(title[last_pos..].to_string(), base_style));
    }

    Line::from(spans)
}

/// Render tasks in flat list view (fallback)
fn render_flat_task_list(
    state: &AppState,
    theme: &crate::colors::Theme,
    conn: &Connection,
) -> Vec<ListItem<'static>> {
    // Check if current file is an index file
    // Try tree view first (like task loading does), then fall back to flat list
    let is_index = if let Some(selected) = state.selected_tree_node() {
        selected
            .file_record
            .is_some_and(|f| display::is_index_file(&f.path))
    } else {
        state
            .selected_file()
            .is_some_and(|f| display::is_index_file(&f.path))
    };

    // Pre-compute cross-file links if viewing an index file
    let cross_file_links: HashSet<i64> = if is_index {
        state
            .tasks
            .iter()
            .filter(|t| utils::is_cross_file_link(conn, t.id))
            .map(|t| t.id)
            .collect()
    } else {
        HashSet::new()
    };

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

            // Format title - for index files, extract link text and convert annotations
            let title = if is_index {
                display::format_index_title(&task.title)
            } else {
                task.title.clone()
            };

            // Check if this task is a cross-file link
            let is_cross_file_link = cross_file_links.contains(&task.id);

            let line = build_styled_task_line(
                indent,
                "",
                checkbox,
                &title,
                style,
                theme,
                is_cross_file_link,
            );

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
