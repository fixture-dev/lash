//! Detail pane rendering

use lash_db::repository::tasks::TaskRecord;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
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

        frame.render_stateful_widget(list, area, &mut list_state);
    }
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
