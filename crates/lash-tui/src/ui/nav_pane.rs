//! Navigation pane rendering

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::state::{AppState, DirectoryNode, FocusedPane};
use crate::ui::themes;
use lash_types::tree::TreeNode;

/// Render the navigation pane
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.focused_pane == FocusedPane::Navigation;
    let theme = &state.theme;

    let border_style = if is_focused {
        themes::focused_border_style(theme)
    } else {
        themes::unfocused_border_style(theme)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Files ")
        .border_style(border_style);

    // Check if tree view is available
    let items: Vec<ListItem> = if let Some(file_trees) = &state.file_tree {
        render_file_tree(file_trees, state, theme)
    } else {
        render_flat_file_list(state, theme)
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(themes::selected_style(theme));

    // Create list state with current selection
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_file_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Render files in tree view
fn render_file_tree(
    trees: &[TreeNode<DirectoryNode>],
    state: &AppState,
    theme: &crate::colors::Theme,
) -> Vec<ListItem<'static>> {
    let mut items = Vec::new();
    let chars = state.tree_chars;

    for tree in trees {
        render_file_node(tree, &mut items, state, theme, chars, &[], true);
    }

    items
}

/// Recursively render a file tree node
fn render_file_node(
    node: &TreeNode<DirectoryNode>,
    items: &mut Vec<ListItem<'static>>,
    state: &AppState,
    theme: &crate::colors::Theme,
    chars: lash_types::tree::TreeChars,
    ancestors_is_last: &[bool],
    is_last: bool,
) {
    let current_index = items.len();
    let is_selected = current_index == state.selected_file_index;

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

    // Build the line content
    let mut spans = vec![Span::raw(prefix), Span::raw(expand_indicator)];

    if node.data.is_directory {
        // Directory node
        let style = if is_selected {
            themes::selected_style(theme)
        } else {
            ratatui::style::Style::default()
        };
        spans.push(Span::styled(node.data.name.clone(), style));
    } else {
        // File node
        if let Some(file) = &node.data.file_record {
            let status_indicator = match file.status {
                lash_types::FileStatus::Complete => "✓",
                lash_types::FileStatus::Blocked => "!",
                lash_types::FileStatus::InProgress => "○",
                lash_types::FileStatus::Empty => "·",
            };

            let style = if is_selected {
                themes::selected_style(theme)
            } else {
                themes::file_status_style(file.status, theme)
            };

            spans.push(Span::raw(status_indicator));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(file.title.clone(), style));
        }
    }

    items.push(ListItem::new(Line::from(spans)));

    // Recursively render children if expanded
    if node.expanded {
        let mut new_ancestors = ancestors_is_last.to_vec();
        new_ancestors.push(is_last);

        let child_count = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            let is_last_child = i == child_count - 1;
            render_file_node(
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

/// Render files in flat list view (fallback)
fn render_flat_file_list(state: &AppState, theme: &crate::colors::Theme) -> Vec<ListItem<'static>> {
    state
        .files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let status_indicator = match file.status {
                lash_types::FileStatus::Complete => "✓",
                lash_types::FileStatus::Blocked => "!",
                lash_types::FileStatus::InProgress => "○",
                lash_types::FileStatus::Empty => "·",
            };

            let style = if i == state.selected_file_index {
                themes::selected_style(theme)
            } else {
                themes::file_status_style(file.status, theme)
            };

            let line = Line::from(vec![
                Span::raw(status_indicator),
                Span::raw(" "),
                Span::styled(file.title.clone(), style),
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
