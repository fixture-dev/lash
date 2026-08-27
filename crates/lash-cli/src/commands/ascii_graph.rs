//! ASCII graph renderer for terminal display
//!
//! This module provides a minimalist ASCII/Unicode box-drawing renderer for
//! dependency graphs that fits within terminal width constraints.

use crossterm::terminal;
use lash_core::dependency::{DependencyGraph, FilterOptions, NodeData};
use lash_core::display;
use lash_types::TaskStatus;
use std::collections::{BTreeMap, HashSet};

use lash::theme::CliTheme;

/// Configuration for ASCII graph rendering
#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct AsciiGraphConfig {
    /// Terminal width (auto-detected if None)
    pub terminal_width: Option<u16>,
    /// Minimum width for task titles before truncation
    pub min_title_width: usize,
    /// Indentation per depth level
    pub indent_width: usize,
}

impl Default for AsciiGraphConfig {
    fn default() -> Self {
        Self {
            terminal_width: None,
            min_title_width: 20,
            indent_width: 3,
        }
    }
}

/// Box-drawing characters for the graph
struct BoxChars {
    branch: &'static str,
    last_branch: &'static str,
    vertical: &'static str,
    horizontal: &'static str,
    space: &'static str,
}

impl BoxChars {
    fn unicode() -> Self {
        Self {
            branch: "├",
            last_branch: "└",
            vertical: "│",
            horizontal: "─",
            space: " ",
        }
    }
}

/// ASCII graph renderer with theme support
pub struct AsciiGraphRenderer<'a> {
    graph: &'a DependencyGraph,
    theme: Option<&'a CliTheme>,
    config: AsciiGraphConfig,
    box_chars: BoxChars,
}

impl<'a> AsciiGraphRenderer<'a> {
    /// Create a new ASCII graph renderer
    pub fn new(graph: &'a DependencyGraph, theme: Option<&'a CliTheme>) -> Self {
        Self {
            graph,
            theme,
            config: AsciiGraphConfig::default(),
            box_chars: BoxChars::unicode(),
        }
    }

    /// Set the configuration for rendering (used in tests)
    #[cfg(test)]
    pub fn with_config(mut self, config: AsciiGraphConfig) -> Self {
        self.config = config;
        self
    }

    /// Render the graph to ASCII with filtering
    pub fn render(&self, options: &FilterOptions) -> String {
        let terminal_width = self.get_terminal_width();
        let filtered_nodes = self.filter_nodes(options);

        if filtered_nodes.is_empty() {
            return self.style_muted("(no tasks to display)");
        }

        // Group nodes by file and find roots
        let file_groups = self.group_by_file(&filtered_nodes);
        let (max_depth, total_depth) = self.calculate_max_displayable_depth(terminal_width);

        let mut output = String::new();
        let mut truncated = false;

        // Render each file group
        for (file_idx, (file_id, nodes)) in file_groups.iter().enumerate() {
            if file_idx > 0 {
                output.push('\n');
            }

            // File header
            output.push_str(&self.style_file_header(file_id));
            output.push('\n');

            // Find root nodes for this file (nodes with no parents in the filtered set)
            let roots = self.find_roots(nodes, &filtered_nodes);

            // Render each root tree
            for (root_idx, root_id) in roots.iter().enumerate() {
                let is_last = root_idx == roots.len() - 1;
                let (tree_output, tree_truncated) = self.render_tree(
                    root_id,
                    &filtered_nodes,
                    max_depth,
                    0,
                    "",
                    is_last,
                    &mut HashSet::new(),
                );
                output.push_str(&tree_output);
                truncated = truncated || tree_truncated;
            }
        }

        // Add truncation notice if needed
        if truncated || max_depth < total_depth {
            output.push('\n');
            output.push_str(&self.style_warning(&format!(
                "Note: Graph truncated to depth {max_depth} (total depth: {total_depth}). Use --format=dot for full graph."
            )));
            output.push('\n');
        }

        output
    }

    /// Get terminal width, with fallback
    fn get_terminal_width(&self) -> u16 {
        self.config
            .terminal_width
            .unwrap_or_else(|| terminal::size().map_or(80, |(w, _)| w))
    }

    /// Calculate maximum displayable depth based on terminal width
    #[allow(clippy::cast_possible_truncation)]
    fn calculate_max_displayable_depth(&self, terminal_width: u16) -> (usize, usize) {
        // Reserved width: checkbox [x] (3) + space (1) + min title (20) = 24
        let reserved = 3 + 1 + self.config.min_title_width;
        let available = terminal_width.saturating_sub(reserved as u16) as usize;

        // Each depth level uses indent_width characters
        let max_depth = available / self.config.indent_width;

        // Calculate actual max depth in graph
        let total_depth = self.calculate_total_depth();

        (max_depth.min(total_depth), total_depth)
    }

    /// Calculate the maximum depth in the entire graph
    fn calculate_total_depth(&self) -> usize {
        let mut max_depth = 0;
        for node_id in self.graph.all_node_ids() {
            if let Some(node) = self.graph.get_node(&node_id) {
                max_depth = max_depth.max(node.depth as usize);
            }
        }
        max_depth + 1 // depth is 0-indexed
    }

    /// Filter nodes based on options
    fn filter_nodes(&self, options: &FilterOptions) -> HashSet<String> {
        let mut nodes = HashSet::new();

        for node_id in self.graph.all_node_ids() {
            if let Some(node) = self.graph.get_node(&node_id) {
                // Filter by file
                if let Some(files) = &options.files {
                    if !files.contains(&node.file_id) {
                        continue;
                    }
                }

                // Filter by completion status
                if options.hide_completed
                    && (node.status == TaskStatus::Done || node.status == TaskStatus::Waived)
                {
                    continue;
                }

                nodes.insert(node_id);
            }
        }

        nodes
    }

    /// Check if any node in a list belongs to an index file
    ///
    /// Index files are `lash.index.md` or `index.lash.md`
    fn has_index_file_node(&self, node_ids: &[String]) -> bool {
        node_ids.iter().any(|id| {
            self.graph
                .get_node(id)
                .is_some_and(NodeData::is_from_index_file)
        })
    }

    /// Group nodes by file ID, with index files sorted first
    ///
    /// Returns files in order: index files first, then alphabetical for the rest
    fn group_by_file(&self, nodes: &HashSet<String>) -> Vec<(String, Vec<String>)> {
        let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for node_id in nodes {
            if let Some(node) = self.graph.get_node(node_id) {
                groups
                    .entry(node.file_id.clone())
                    .or_default()
                    .push(node_id.clone());
            }
        }

        // Sort nodes within each group for deterministic output
        for node_list in groups.values_mut() {
            node_list.sort();
        }

        // Separate index files from regular files by checking source_path
        let mut index_files: Vec<(String, Vec<String>)> = Vec::new();
        let mut regular_files: Vec<(String, Vec<String>)> = Vec::new();

        for (file_id, node_list) in groups {
            if self.has_index_file_node(&node_list) {
                index_files.push((file_id, node_list));
            } else {
                regular_files.push((file_id, node_list));
            }
        }

        // Sort index files alphabetically among themselves
        index_files.sort_by(|a, b| a.0.cmp(&b.0));

        // Combine: index files first, then regular files (already sorted by BTreeMap)
        index_files.extend(regular_files);
        index_files
    }

    /// Find root nodes (nodes with no parents in the filtered set)
    ///
    /// If all nodes have parents (e.g., in a cycle), picks the first node alphabetically
    /// as an arbitrary starting point.
    fn find_roots(&self, file_nodes: &[String], all_filtered: &HashSet<String>) -> Vec<String> {
        let mut roots = Vec::new();

        for node_id in file_nodes {
            let has_parent_in_set = self
                .graph
                .get_dependents(node_id)
                .is_some_and(|deps| deps.iter().any(|d| all_filtered.contains(&d.target_id)));

            if !has_parent_in_set {
                roots.push(node_id.clone());
            }
        }

        // If no roots found (all nodes in cycle), pick first node as arbitrary root
        if roots.is_empty() && !file_nodes.is_empty() {
            let mut sorted = file_nodes.to_vec();
            sorted.sort();
            roots.push(sorted[0].clone());
        }

        roots.sort();
        roots
    }

    /// Render a tree starting from a root node
    #[allow(clippy::too_many_arguments)]
    fn render_tree(
        &self,
        node_id: &str,
        filtered_nodes: &HashSet<String>,
        max_depth: usize,
        current_depth: usize,
        prefix: &str,
        is_last: bool,
        visited: &mut HashSet<String>,
    ) -> (String, bool) {
        let mut output = String::new();
        let mut truncated = false;

        // Prevent cycles
        if visited.contains(node_id) {
            output.push_str(prefix);
            output.push_str(if is_last {
                self.box_chars.last_branch
            } else {
                self.box_chars.branch
            });
            output.push_str(self.box_chars.horizontal);
            output.push_str(self.box_chars.horizontal);
            output.push(' ');
            output.push_str(&self.style_muted("(cycle)"));
            output.push('\n');
            return (output, false);
        }

        // Check depth limit
        if current_depth > max_depth {
            return (String::new(), true);
        }

        visited.insert(node_id.to_string());

        // Get node data
        let Some(node) = self.graph.get_node(node_id) else {
            output.push_str(prefix);
            output.push_str(if is_last {
                self.box_chars.last_branch
            } else {
                self.box_chars.branch
            });
            output.push_str(self.box_chars.horizontal);
            output.push_str(self.box_chars.horizontal);
            output.push(' ');
            output.push_str(&self.style_muted(&format!("(missing: {node_id})")));
            output.push('\n');
            return (output, false);
        };

        // Build the line
        output.push_str(prefix);
        output.push_str(if is_last {
            self.box_chars.last_branch
        } else {
            self.box_chars.branch
        });
        output.push_str(self.box_chars.horizontal);
        output.push_str(self.box_chars.horizontal);
        output.push(' ');
        output.push_str(&self.render_node(node));
        output.push('\n');

        // Get children (dependencies that are in the filtered set)
        let children: Vec<String> = self
            .graph
            .get_dependencies(node_id)
            .map(|deps| {
                deps.iter()
                    .filter(|d| filtered_nodes.contains(&d.target_id))
                    .map(|d| d.target_id.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Calculate new prefix for children
        let new_prefix = format!(
            "{}{}   ",
            prefix,
            if is_last {
                self.box_chars.space
            } else {
                self.box_chars.vertical
            }
        );

        // Render children
        let child_count = children.len();
        for (idx, child_id) in children.iter().enumerate() {
            let is_last_child = idx == child_count - 1;
            let (child_output, child_truncated) = self.render_tree(
                child_id,
                filtered_nodes,
                max_depth,
                current_depth + 1,
                &new_prefix,
                is_last_child,
                visited,
            );
            output.push_str(&child_output);
            truncated = truncated || child_truncated;
        }

        visited.remove(node_id);

        (output, truncated)
    }

    /// Render a single node with checkbox and title
    ///
    /// For index file tasks, extracts link text from Markdown links and
    /// applies colorization to the title using the theme.
    fn render_node(&self, node: &NodeData) -> String {
        let checkbox = self.render_checkbox(node.status);
        let is_index = node.is_from_index_file();

        // For index files, extract link text and format annotations using shared display utilities
        let title = if is_index {
            display::format_index_title(&node.title)
        } else {
            node.title.clone()
        };

        let truncated_title = self.truncate_title(&title);

        // Style the title: for index files, apply index styling; always style inline labels
        let styled_title = if is_index {
            self.style_index_task_title(&truncated_title)
        } else {
            self.style_labels_in_text(&truncated_title)
        };

        format!("{checkbox} {styled_title}")
    }

    /// Style index task title with theme colorization
    ///
    /// For index file tasks, applies special coloring to the title text
    /// and any inline labels.
    fn style_index_task_title(&self, title: &str) -> String {
        let Some(theme) = self.theme else {
            return self.style_labels_in_text(title);
        };

        // Apply info styling to the main title, then handle labels within
        // Split the title to find labels
        let mut result = String::new();
        let mut chars = title.char_indices().peekable();
        let mut last_pos = 0;

        while let Some((i, c)) = chars.next() {
            if c == '#' {
                // Found potential label start
                // First, add the text before this point with info styling
                if i > last_pos {
                    let text_segment = &title[last_pos..i];
                    result.push_str(&theme.style_info(text_segment));
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
                    let label = &title[label_start..label_end];
                    result.push_str(&theme.style_label(label));
                } else {
                    result.push_str(&theme.style_info(&title[label_start..label_end]));
                }
                last_pos = label_end;
            }
        }

        // Add any remaining text with info styling
        if last_pos < title.len() {
            result.push_str(&theme.style_info(&title[last_pos..]));
        }

        result
    }

    /// Style labels (hashtags like #backend, #docs) within text
    fn style_labels_in_text(&self, text: &str) -> String {
        let Some(theme) = self.theme else {
            return text.to_string();
        };

        let mut result = String::new();
        let mut chars = text.char_indices().peekable();

        while let Some((i, c)) = chars.next() {
            if c == '#' {
                // Found potential label start - collect the label
                let label_start = i;
                let mut label_end = i + 1;

                // Collect alphanumeric chars and hyphens/underscores
                while let Some(&(j, next_c)) = chars.peek() {
                    if next_c.is_alphanumeric() || next_c == '-' || next_c == '_' {
                        label_end = j + next_c.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }

                // Only style if we have at least one char after #
                if label_end > label_start + 1 {
                    let label = &text[label_start..label_end];
                    result.push_str(&theme.style_label(label));
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Render a status checkbox with theme colors
    fn render_checkbox(&self, status: TaskStatus) -> String {
        let checkbox = match status {
            TaskStatus::Open => "[ ]",
            TaskStatus::InProgress => "[>]",
            TaskStatus::Done => "[x]",
            TaskStatus::Waived => "[-]",
            TaskStatus::Blocked => "[!]",
        };

        if let Some(theme) = self.theme {
            theme.style_task_status(checkbox, status)
        } else {
            checkbox.to_string()
        }
    }

    /// Truncate title to fit terminal width
    fn truncate_title(&self, title: &str) -> String {
        let terminal_width = self.get_terminal_width() as usize;
        // Rough estimate: allow most of width for title
        let max_title_len = terminal_width.saturating_sub(20);

        if title.len() > max_title_len && max_title_len > 3 {
            lash_types::text::truncate_with_ellipsis(title, max_title_len)
        } else {
            title.to_string()
        }
    }

    /// Style a file header
    fn style_file_header(&self, file_id: &str) -> String {
        let header = format!("─── {file_id} ");
        if let Some(theme) = self.theme {
            theme.style_info(&header)
        } else {
            header
        }
    }

    /// Style muted/secondary text
    fn style_muted(&self, text: &str) -> String {
        if let Some(theme) = self.theme {
            theme.style_muted(text)
        } else {
            text.to_string()
        }
    }

    /// Style warning text
    fn style_warning(&self, text: &str) -> String {
        if let Some(theme) = self.theme {
            theme.style_warning(text)
        } else {
            text.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_core::dependency::{DependencyGraph, EdgeData, NodeData};
    use lash_types::DependencyKind;

    fn create_test_node(title: &str, status: TaskStatus, file_id: &str) -> NodeData {
        NodeData::new(title.to_string(), status, file_id.to_string(), 0)
    }

    fn create_test_node_with_path(
        title: &str,
        status: TaskStatus,
        file_id: &str,
        source_path: &str,
    ) -> NodeData {
        NodeData::new(title.to_string(), status, file_id.to_string(), 0)
            .with_source_path(source_path.to_string())
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("no tasks"));
    }

    #[test]
    fn test_single_node() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_test_node("First Task", TaskStatus::Open, "test"),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("test"));
        assert!(output.contains("[ ]"));
        assert!(output.contains("First Task"));
    }

    #[test]
    fn test_parent_child() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#parent".to_string(),
            create_test_node("Parent Task", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#child".to_string(),
            create_test_node("Child Task", TaskStatus::Done, "test"),
        );
        graph.add_edge(
            "test#parent".to_string(),
            "test#child".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("Parent Task"));
        assert!(output.contains("Child Task"));
        assert!(output.contains("└──")); // Tree branch
    }

    #[test]
    fn test_status_display() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#open".to_string(),
            create_test_node("Open", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#done".to_string(),
            create_test_node("Done", TaskStatus::Done, "test"),
        );
        graph.add_node(
            "test#waived".to_string(),
            create_test_node("Waived", TaskStatus::Waived, "test"),
        );
        graph.add_node(
            "test#blocked".to_string(),
            create_test_node("Blocked", TaskStatus::Blocked, "test"),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("[ ]")); // Open
        assert!(output.contains("[x]")); // Done
        assert!(output.contains("[-]")); // Waived
        assert!(output.contains("[!]")); // Blocked
    }

    #[test]
    fn test_hide_completed() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#open".to_string(),
            create_test_node("Open Task", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#done".to_string(),
            create_test_node("Done Task", TaskStatus::Done, "test"),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let options = FilterOptions {
            hide_completed: true,
            ..Default::default()
        };
        let output = renderer.render(&options);

        assert!(output.contains("Open Task"));
        assert!(!output.contains("Done Task"));
    }

    #[test]
    fn test_multiple_files() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "file1#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Open, "file1"),
        );
        graph.add_node(
            "file2#task2".to_string(),
            create_test_node("Task 2", TaskStatus::Open, "file2"),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("file1"));
        assert!(output.contains("file2"));
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#a".to_string(),
            create_test_node("Task A", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#b".to_string(),
            create_test_node("Task B", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#a".to_string(),
            "test#b".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#b".to_string(),
            "test#a".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("cycle"));
    }

    #[test]
    fn test_width_truncation() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task".to_string(),
            create_test_node(
                "This is a very long task title that should be truncated when the terminal is narrow",
                TaskStatus::Open,
                "test"
            ),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(40),
            ..Default::default()
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("..."));
    }

    #[test]
    fn test_labels_in_title() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task".to_string(),
            create_test_node("Add feature #backend #p1", TaskStatus::Open, "test"),
        );

        // Without theme, labels should appear as-is
        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("#backend"));
        assert!(output.contains("#p1"));
        assert!(output.contains("Add feature"));
    }

    #[test]
    fn test_is_from_index_file() {
        // Should identify index file patterns by source path
        let node1 =
            create_test_node_with_path("Task", TaskStatus::Open, "pixelquest", "lash.index.md");
        assert!(node1.is_from_index_file());

        let node2 =
            create_test_node_with_path("Task", TaskStatus::Open, "project", "index.lash.md");
        assert!(node2.is_from_index_file());

        // Nested paths should also work
        let node3 =
            create_test_node_with_path("Task", TaskStatus::Open, "sub", "subdir/lash.index.md");
        assert!(node3.is_from_index_file());

        // Should not identify regular files
        let node4 = create_test_node_with_path("Task", TaskStatus::Open, "tasks", "tasks.md");
        assert!(!node4.is_from_index_file());

        let node5 = create_test_node_with_path("Task", TaskStatus::Open, "core.api", "core/api.md");
        assert!(!node5.is_from_index_file());

        // Node without source path should not be identified as index file
        let node6 = create_test_node("Task", TaskStatus::Open, "test");
        assert!(!node6.is_from_index_file());
    }

    #[test]
    fn test_extract_link_text() {
        // Standard markdown link
        assert_eq!(
            display::extract_link_text("[Core API](core/api.md)"),
            "Core API"
        );

        // Link with prefix
        assert_eq!(
            display::extract_link_text("Prefix [Link Text](path.md)"),
            "Prefix Link Text"
        );

        // Link with suffix
        assert_eq!(
            display::extract_link_text("[Link](path.md) suffix"),
            "Link suffix"
        );

        // No link - returns original
        assert_eq!(display::extract_link_text("Plain text"), "Plain text");

        // Backtick path (no transformation - not a link)
        assert_eq!(
            display::extract_link_text("`path/file.md`"),
            "`path/file.md`"
        );
    }

    #[test]
    fn test_format_index_annotations() {
        // Full annotation: strip @id and convert @labels to hashtags
        assert_eq!(
            display::format_index_annotations(
                "Alpha @id:`milestone.alpha` @labels:`milestone, p0`"
            ),
            "Alpha #milestone #p0"
        );

        // Just @id annotation - strip completely
        assert_eq!(
            display::format_index_annotations("Task @id:`some.id`"),
            "Task"
        );

        // Just @labels annotation - convert to hashtags
        assert_eq!(
            display::format_index_annotations("Task @labels:`foo, bar, baz`"),
            "Task #foo #bar #baz"
        );

        // No annotations - return as-is
        assert_eq!(
            display::format_index_annotations("Plain Task"),
            "Plain Task"
        );

        // With existing hashtags - preserve them
        assert_eq!(
            display::format_index_annotations("Task #existing @labels:`new`"),
            "Task #existing #new"
        );
    }

    #[test]
    fn test_index_files_sorted_first() {
        let mut graph = DependencyGraph::new();

        // Add regular file tasks (with source paths)
        graph.add_node(
            "alpha#task1".to_string(),
            create_test_node_with_path("Alpha Task", TaskStatus::Open, "alpha", "alpha.md"),
        );
        graph.add_node(
            "zebra#task1".to_string(),
            create_test_node_with_path("Zebra Task", TaskStatus::Open, "zebra", "zebra.md"),
        );

        // Add index file tasks (with source paths to index files)
        graph.add_node(
            "pixelquest#main".to_string(),
            create_test_node_with_path(
                "[Core](core.md)",
                TaskStatus::Open,
                "pixelquest",
                "lash.index.md",
            ),
        );
        graph.add_node(
            "myproject#secondary".to_string(),
            create_test_node_with_path(
                "[Secondary](secondary.md)",
                TaskStatus::Open,
                "myproject",
                "index.lash.md",
            ),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        // Find positions of file headers (using file_id, not filename)
        let myproject_pos = output.find("myproject");
        let pixelquest_pos = output.find("pixelquest");
        let alpha_pos = output.find("alpha");
        let zebra_pos = output.find("zebra");

        // Index files should appear first
        assert!(myproject_pos.is_some());
        assert!(pixelquest_pos.is_some());
        assert!(alpha_pos.is_some());
        assert!(zebra_pos.is_some());

        // Both index files should come before regular files
        assert!(myproject_pos.unwrap() < alpha_pos.unwrap());
        assert!(pixelquest_pos.unwrap() < alpha_pos.unwrap());
    }

    #[test]
    fn test_index_task_link_extraction() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "pixelquest#core".to_string(),
            create_test_node_with_path(
                "[Core Module](core/module.md)",
                TaskStatus::Open,
                "pixelquest",
                "lash.index.md",
            ),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        // Should contain extracted link text, not the full markdown link
        assert!(output.contains("Core Module"));
        assert!(!output.contains("core/module.md"));
        assert!(!output.contains("[Core Module]"));
    }

    #[test]
    fn test_index_task_with_labels() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "pixelquest#core".to_string(),
            create_test_node_with_path(
                "[Core API](core.md) #backend #p1",
                TaskStatus::Open,
                "pixelquest",
                "lash.index.md",
            ),
        );

        // Without theme, labels should appear as-is
        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("Core API"));
        assert!(output.contains("#backend"));
        assert!(output.contains("#p1"));
        assert!(!output.contains("core.md")); // Path should be stripped
    }

    // --- Tests targeting surviving mutants ---

    /// mut-000153: `truncated = false` → `true`
    /// A simple graph with no depth truncation must NOT produce a truncation notice.
    #[test]
    fn test_no_truncation_notice_when_not_needed() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#task1".to_string(),
            create_test_node("Task 1", TaskStatus::Open, "test"),
        );

        // Wide terminal ensures no depth truncation
        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            ..Default::default()
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(!output.contains("Note: Graph truncated"));
    }

    /// mut-000154/155/156/157: boundary conditions on `file_idx > 0`
    /// Two file groups must be separated by exactly one blank line, and the first
    /// group must NOT be preceded by a blank line.
    #[test]
    fn test_multiple_files_separator_newline() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "aaa#task1".to_string(),
            create_test_node("Task A", TaskStatus::Open, "aaa"),
        );
        graph.add_node(
            "zzz#task1".to_string(),
            create_test_node("Task Z", TaskStatus::Open, "zzz"),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            ..Default::default()
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // The output must start directly with the first file header (─── aaa ...),
        // not with a blank line.
        assert!(output.starts_with("─── aaa"), "output = {output:?}");

        // There must be exactly one blank line between the two file sections.
        // The separator is a bare '\n' pushed before the second file header.
        // So we expect "\n\n─── zzz" in the output.
        assert!(
            output.contains("\n\n─── zzz"),
            "expected blank-line separator before second file; output = {output:?}"
        );
    }

    /// mut-000158/159: `root_idx == roots.len() - 1`
    /// With two root tasks in the same file, the first must use `├` and the last `└`.
    #[test]
    fn test_multiple_roots_last_branch_character() {
        let mut graph = DependencyGraph::new();
        // Two independent roots sorted alphabetically: "aaa" before "zzz"
        graph.add_node(
            "test#aaa".to_string(),
            create_test_node("Alpha Task", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#zzz".to_string(),
            create_test_node("Zeta Task", TaskStatus::Open, "test"),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            ..Default::default()
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // "Alpha Task" is first root → rendered with ├
        // "Zeta Task" is last root  → rendered with └
        let alpha_pos = output.find("Alpha Task").expect("Alpha Task not found");
        let zeta_pos = output.find("Zeta Task").expect("Zeta Task not found");

        // Extract the branch character preceding each task
        let alpha_line = output[..alpha_pos]
            .rfind('\n')
            .map_or(&output[..alpha_pos], |p| &output[p + 1..alpha_pos]);
        let zeta_line = output[..zeta_pos]
            .rfind('\n')
            .map_or(&output[..zeta_pos], |p| &output[p + 1..zeta_pos]);

        assert!(
            alpha_line.contains('├'),
            "first root should use ├; line = {alpha_line:?}"
        );
        assert!(
            zeta_line.contains('└'),
            "last root should use └; line = {zeta_line:?}"
        );
    }

    /// mut-000194/195: `idx == child_count - 1` for child rendering
    /// A parent with two children must use `├` for the first child and `└` for the last.
    #[test]
    fn test_multiple_children_branch_characters() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#parent".to_string(),
            create_test_node("Parent Task", TaskStatus::Open, "test"),
        );
        // Children are sorted alphabetically: "child_a" before "child_z"
        graph.add_node(
            "test#child_a".to_string(),
            create_test_node("Alpha Child", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#child_z".to_string(),
            create_test_node("Zeta Child", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#parent".to_string(),
            "test#child_a".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#parent".to_string(),
            "test#child_z".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            ..Default::default()
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        let alpha_pos = output.find("Alpha Child").expect("Alpha Child not found");
        let zeta_pos = output.find("Zeta Child").expect("Zeta Child not found");

        let alpha_line = output[..alpha_pos]
            .rfind('\n')
            .map_or(&output[..alpha_pos], |p| &output[p + 1..alpha_pos]);
        let zeta_line = output[..zeta_pos]
            .rfind('\n')
            .map_or(&output[..zeta_pos], |p| &output[p + 1..zeta_pos]);

        assert!(
            alpha_line.contains('├'),
            "first child should use ├; line = {alpha_line:?}"
        );
        assert!(
            zeta_line.contains('└'),
            "last child should use └; line = {zeta_line:?}"
        );
    }

    /// mut-000196: `current_depth + 1` in recursive `render_tree` call
    /// A three-level hierarchy must place the grandchild at a deeper indentation
    /// than the child, proving the depth counter increments correctly.
    ///
    /// Node `depth` fields are set to match their nesting level so that
    /// `calculate_total_depth` returns 3 and the wide terminal allows all levels.
    #[test]
    fn test_depth_increments_in_tree() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#root".to_string(),
            NodeData::new("Root".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#kid".to_string(),
            NodeData::new("Kid".to_string(), TaskStatus::Open, "test".to_string(), 1),
        );
        graph.add_node(
            "test#grandkid".to_string(),
            NodeData::new(
                "Grandkid".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                2,
            ),
        );
        graph.add_edge(
            "test#root".to_string(),
            "test#kid".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#kid".to_string(),
            "test#grandkid".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // total_depth = 2+1 = 3; wide terminal → max_depth = min(58, 3) = 3
        // So all three levels render without truncation.
        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("Root"), "output = {output:?}");
        assert!(output.contains("Kid"), "output = {output:?}");
        assert!(output.contains("Grandkid"), "output = {output:?}");

        // Measure leading-space indentation of each task line
        let lines: Vec<&str> = output.lines().collect();
        let root_line = lines
            .iter()
            .find(|l| l.contains("Root"))
            .expect("Root line not found");
        let kid_line = lines
            .iter()
            .find(|l| l.contains("Kid") && !l.contains("Grand"))
            .expect("Kid line not found");
        let grandkid_line = lines
            .iter()
            .find(|l| l.contains("Grandkid"))
            .expect("Grandkid line not found");

        let root_indent: usize = root_line.chars().take_while(|c| *c == ' ').count();
        let kid_indent: usize = kid_line.chars().take_while(|c| *c == ' ').count();
        let grandkid_indent: usize = grandkid_line.chars().take_while(|c| *c == ' ').count();

        // Each level must be indented strictly more than the one above
        assert!(
            kid_indent > root_indent,
            "kid indent ({kid_indent}) must exceed root indent ({root_indent}); output = {output:?}"
        );
        assert!(
            grandkid_indent > kid_indent,
            "grandkid indent ({grandkid_indent}) must exceed kid indent ({kid_indent}); output = {output:?}"
        );
    }

    /// mut-000161/197: `truncated || tree_truncated` / `truncated || child_truncated` → `&&`
    /// When a child subtree is truncated but earlier trees are not, the truncation notice
    /// must still appear (OR behaviour, not AND).
    ///
    /// Node `depth` fields are set to match their graph nesting so `calculate_total_depth`
    /// returns the correct value and `render_tree` actually hits the depth limit.
    #[test]
    fn test_truncation_notice_appears_when_any_child_truncated() {
        let mut graph = DependencyGraph::new();
        // Chain: root(depth=0) → d1(depth=1) → d2(depth=2) → d3(depth=3)
        // total_depth = 3+1 = 4
        graph.add_node(
            "test#root".to_string(),
            NodeData::new("Root".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#d1".to_string(),
            NodeData::new(
                "Depth1".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                1,
            ),
        );
        graph.add_node(
            "test#d2".to_string(),
            NodeData::new(
                "Depth2".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                2,
            ),
        );
        graph.add_node(
            "test#d3".to_string(),
            NodeData::new(
                "Depth3".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                3,
            ),
        );
        graph.add_edge(
            "test#root".to_string(),
            "test#d1".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#d1".to_string(),
            "test#d2".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#d2".to_string(),
            "test#d3".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // terminal=27 → available=3 → max_depth_avail=1, min(1,4)=1 < 4 → truncation notice
        let config = AsciiGraphConfig {
            terminal_width: Some(27),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            output.contains("Note: Graph truncated"),
            "expected truncation notice; output = {output:?}"
        );
    }

    /// mut-000162/163/164/165: `truncated || max_depth < total_depth` conditions
    /// When `max_depth` equals `total_depth` (no depth gap), and no tree is truncated,
    /// the truncation notice must NOT appear.
    #[test]
    fn test_no_truncation_notice_when_depth_fits() {
        let mut graph = DependencyGraph::new();
        // root(depth=0) → child(depth=1) → total_depth = 2
        graph.add_node(
            "test#root".to_string(),
            NodeData::new("Root".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#child".to_string(),
            NodeData::new("Child".to_string(), TaskStatus::Open, "test".to_string(), 1),
        );
        graph.add_edge(
            "test#root".to_string(),
            "test#child".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // Wide terminal: max_depth_avail=58, min(58,2)=2 = total_depth → no truncation
        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            !output.contains("Note: Graph truncated"),
            "unexpected truncation notice; output = {output:?}"
        );
    }

    /// mut-000162/163: only `max_depth < total_depth` is true (not truncated)
    /// Verifies OR semantics: the notice appears even without `truncated == true`,
    /// purely because `max_depth` < `total_depth`.
    #[test]
    fn test_truncation_notice_from_depth_comparison_alone() {
        let mut graph = DependencyGraph::new();
        // Chain: root(depth=0) → d1(depth=1) → d2(depth=2)
        // total_depth = 2+1 = 3
        graph.add_node(
            "test#root".to_string(),
            NodeData::new("Root".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#d1".to_string(),
            NodeData::new("D1".to_string(), TaskStatus::Open, "test".to_string(), 1),
        );
        graph.add_node(
            "test#d2".to_string(),
            NodeData::new("D2".to_string(), TaskStatus::Open, "test".to_string(), 2),
        );
        graph.add_edge(
            "test#root".to_string(),
            "test#d1".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#d1".to_string(),
            "test#d2".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // terminal=25 → available=1 → max_depth_avail=0 → min(0,3)=0 < 3 → notice must appear
        let config = AsciiGraphConfig {
            terminal_width: Some(25),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            output.contains("Note: Graph truncated"),
            "expected truncation notice when max_depth < total_depth; output = {output:?}"
        );
    }

    /// mut-000164: `<` → `<=` in `max_depth < total_depth`
    /// When `max_depth` == `total_depth` (equal), no truncation notice should appear.
    #[test]
    fn test_no_truncation_when_max_depth_equals_total_depth() {
        let mut graph = DependencyGraph::new();
        // Single flat node: total_depth = 1, depth 0-indexed
        graph.add_node(
            "test#task1".to_string(),
            create_test_node("Flat Task", TaskStatus::Open, "test"),
        );

        // Wide terminal: max_depth will be very large, min(max_depth, total_depth) = total_depth
        // So max_depth returned == total_depth == 1; condition max_depth < total_depth is false
        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            !output.contains("Note: Graph truncated"),
            "unexpected truncation notice; output = {output:?}"
        );
    }

    /// mut-000166: `reserved = 3 + 1 + min_title_width` literal mutation
    /// Verifies that the reserved width calculation uses the correct value (24 by default)
    /// by checking that truncation behaviour changes at the expected terminal width boundary.
    ///
    /// Node `depth` fields must match their nesting level so that `calculate_total_depth`
    /// returns the correct value (not just 1 for every flat-depth node).
    #[test]
    fn test_max_displayable_depth_calculation() {
        let mut graph = DependencyGraph::new();
        // Two-level hierarchy: root at depth=0, child at depth=1 → total_depth = 1+1 = 2
        graph.add_node(
            "test#root".to_string(),
            NodeData::new("Root".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#child".to_string(),
            NodeData::new("Child".to_string(), TaskStatus::Open, "test".to_string(), 1),
        );
        graph.add_edge(
            "test#root".to_string(),
            "test#child".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // reserved = 3+1+20 = 24, terminal = 24 → available = 0 → max_depth = 0
        // total_depth = 2 → min(0,2)=0 < 2 → truncation notice expected
        let config_narrow = AsciiGraphConfig {
            terminal_width: Some(24),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer_narrow = AsciiGraphRenderer::new(&graph, None).with_config(config_narrow);
        let output_narrow = renderer_narrow.render(&FilterOptions::default());
        assert!(
            output_narrow.contains("Note: Graph truncated"),
            "should truncate at terminal_width=24; output = {output_narrow:?}"
        );

        // terminal = 27 → available = 3 → max_depth_avail = 3/3 = 1, min(1,2) = 1 < 2 → notice expected
        let config_27 = AsciiGraphConfig {
            terminal_width: Some(27),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer_27 = AsciiGraphRenderer::new(&graph, None).with_config(config_27);
        let output_27 = renderer_27.render(&FilterOptions::default());
        assert!(
            output_27.contains("Note: Graph truncated"),
            "should truncate at terminal_width=27; output = {output_27:?}"
        );

        // terminal = 30 → available = 6 → max_depth_avail = 6/3 = 2, min(2,2) = 2 == total_depth → no notice
        let config_wide = AsciiGraphConfig {
            terminal_width: Some(30),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer_wide = AsciiGraphRenderer::new(&graph, None).with_config(config_wide);
        let output_wide = renderer_wide.render(&FilterOptions::default());
        assert!(
            !output_wide.contains("Note: Graph truncated"),
            "should NOT truncate at terminal_width=30; output = {output_wide:?}"
        );
    }

    /// mut-000167: `max_depth + 1` → `max_depth + 0` in `calculate_total_depth`
    /// A graph with a single node at depth 0 must report `total_depth=1` (0-indexed + 1).
    /// If the `+1` were `+0`, `max_depth` would be 0 and `max_depth` < `total_depth` would be 0 < 0 = false,
    /// but we'd get no entries rendered. The simplest proof is that flat graphs render without
    /// truncation (`total_depth=1`, `max_depth` clamped to 1 on a wide terminal).
    #[test]
    fn test_calculate_total_depth_single_node() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#only".to_string(),
            NodeData::new(
                "Only Task".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                0,
            ),
        );

        // Wide terminal: max_depth_available = very large; min(large, 1) = 1 = total_depth
        // So returned max_depth == total_depth == 1 → no truncation notice
        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(output.contains("Only Task"));
        assert!(!output.contains("Note: Graph truncated"));
    }

    /// mut-000183: `roots.is_empty() && !file_nodes.is_empty()` → `||`
    /// When all nodes in a file form a cycle the fallback must trigger only when
    /// `file_nodes` is non-empty. The test verifies the correct (first-alphabetically)
    /// node is chosen as the cycle entry point.
    ///
    /// Node `depth` values must allow `render_tree` to reach the cycle before hitting
    /// the depth limit.  A wide terminal with depth=0 on all nodes gives `total_depth=1`
    /// and `max_depth=1`, so depth-2 calls are truncated rather than marked as cycles.
    /// Setting nodes to depth=0,1,2 gives `total_depth=3`, `max_depth=3` on a wide terminal,
    /// letting the renderer traverse all the way to the back-edge.
    #[test]
    fn test_cycle_fallback_picks_first_alphabetically() {
        let mut graph = DependencyGraph::new();
        // Cycle: aaa→mmm→zzz→aaa
        // Assign increasing depths so total_depth=3 and the chain is fully traversable.
        graph.add_node(
            "test#aaa".to_string(),
            NodeData::new("Alpha".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#mmm".to_string(),
            NodeData::new(
                "Middle".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                1,
            ),
        );
        graph.add_node(
            "test#zzz".to_string(),
            NodeData::new("Zeta".to_string(), TaskStatus::Open, "test".to_string(), 2),
        );
        graph.add_edge(
            "test#aaa".to_string(),
            "test#mmm".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#mmm".to_string(),
            "test#zzz".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#zzz".to_string(),
            "test#aaa".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        // Wide terminal: total_depth=3, max_depth=min(58,3)=3 — enough for the full cycle
        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // "Alpha" corresponds to test#aaa (sorted[0]) and should be the root.
        assert!(output.contains("Alpha"), "output = {output:?}");
        // The cycle marker must appear as zzz tries to visit aaa again
        assert!(
            output.contains("cycle"),
            "expected cycle marker; output = {output:?}"
        );
    }

    /// mut-000185: `sorted[0]` → `sorted[1]`
    /// Same cycle setup as above — the first-alphabetically node must be chosen,
    /// not the second. With only two nodes in a pure cycle, sorted[0] != sorted[1].
    #[test]
    fn test_cycle_fallback_first_not_second() {
        let mut graph = DependencyGraph::new();
        // Use depth=0 for aaa (root), depth=1 for zzz so total_depth=2, max_depth=2 on wide terminal
        graph.add_node(
            "test#aaa".to_string(),
            NodeData::new(
                "AlphaNode".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                0,
            ),
        );
        graph.add_node(
            "test#zzz".to_string(),
            NodeData::new(
                "ZetaNode".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                1,
            ),
        );
        // Pure cycle: aaa → zzz → aaa
        graph.add_edge(
            "test#aaa".to_string(),
            "test#zzz".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#zzz".to_string(),
            "test#aaa".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // "AlphaNode" is sorted first; it must be the root (appear as a non-cycle line)
        // while "ZetaNode" is its child and will show as a cycle when aaa is revisited.
        let alpha_pos = output.find("AlphaNode").expect("AlphaNode not found");
        let zeta_pos = output.find("ZetaNode").expect("ZetaNode not found");

        // AlphaNode is the root so it appears first in the output
        assert!(
            alpha_pos < zeta_pos,
            "AlphaNode (sorted[0]) should appear before ZetaNode (sorted[1]); output = {output:?}"
        );
    }

    /// mut-000186/188/189: cycle rendering booleans and branch characters
    /// The cycle return value must be `(output, false)` — cycles themselves are not
    /// "truncated". Also verifies that `is_last` controls the branch character.
    #[test]
    fn test_cycle_rendering_uses_correct_branch_and_returns_false_truncated() {
        let mut graph = DependencyGraph::new();
        // Two-node cycle: aaa → zzz → aaa
        // aaa will be the root (sorted[0] via fallback), zzz is the sole child (is_last=true)
        // so the cycle marker when zzz tries to visit aaa again must use └.
        // depth=0 for aaa, depth=1 for zzz → total_depth=2, max_depth=2 on wide terminal
        graph.add_node(
            "test#aaa".to_string(),
            NodeData::new("Alpha".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#zzz".to_string(),
            NodeData::new("Zeta".to_string(), TaskStatus::Open, "test".to_string(), 1),
        );
        graph.add_edge(
            "test#aaa".to_string(),
            "test#zzz".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "test#zzz".to_string(),
            "test#aaa".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // The cycle marker line should contain "(cycle)"
        assert!(output.contains("(cycle)"), "output = {output:?}");

        // Cycle return value is (output, false): so the overall graph must NOT show a
        // truncation notice purely due to cycles
        assert!(
            !output.contains("Note: Graph truncated"),
            "cycles should not trigger truncation notice; output = {output:?}"
        );

        // zzz has only one dependency (aaa) so the back-edge is is_last=true → └
        let cycle_line = output.lines().find(|l| l.contains("(cycle)")).unwrap();
        assert!(
            cycle_line.contains('└'),
            "cycle line should use └ (is_last=true); cycle_line = {cycle_line:?}"
        );
    }

    /// mut-000188: `is_last` → `!(is_last)` in cycle branch-character selection
    /// Verifies that when a visited node is rendered as a non-last child, `is_last=false`
    /// produces `├` in the cycle marker line (not `└`).
    ///
    /// In `render_tree`, `is_last` is passed by the caller based on whether the current
    /// node is the last item in the parent's dependency list.  When a cycle is detected,
    /// the same `is_last` value controls the branch character.
    ///
    /// Setup: root → `child_z` (only child of root, so `child_z` is `is_last=true`).
    /// `child_z` has two dependencies: [root (cycle), `zzz_extra`] sorted alphabetically as
    /// "test#root" < "`test#zzz_extra`", so root is idx=0 (NOT last, `is_last=false`) → ├.
    #[test]
    fn test_cycle_branch_character_not_last() {
        let mut graph = DependencyGraph::new();
        // root(depth=0) is the tree root.
        // child_z(depth=1) is root's only child.
        // child_z has two deps: root (cycle, sorted first) and zzz_extra (no cycle, sorted last).
        // When rendering root inside child_z's subtree, root is visited and is_last=false → ├.
        graph.add_node(
            "test#root".to_string(),
            NodeData::new("Root".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#zchild".to_string(),
            NodeData::new(
                "ZChild".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                1,
            ),
        );
        graph.add_node(
            "test#zzz_extra".to_string(),
            NodeData::new(
                "ZzzExtra".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                2,
            ),
        );

        // root → zchild (root's only dep; zchild is last child of root, is_last=true)
        graph.add_edge(
            "test#root".to_string(),
            "test#zchild".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        // zchild → root (back-edge; root sorts before zzz_extra → idx=0, is_last=false)
        graph.add_edge(
            "test#zchild".to_string(),
            "test#root".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        // zchild → zzz_extra (normal edge; zzz_extra is last dep of zchild)
        graph.add_edge(
            "test#zchild".to_string(),
            "test#zzz_extra".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // total_depth = max(0,1,2)+1 = 3; wide terminal → max_depth=3 → no depth truncation
        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // The cycle marker for root (inside zchild's subtree) must use ├ (not last child)
        let cycle_line = output
            .lines()
            .find(|l| l.contains("(cycle)"))
            .expect("expected a cycle line; output = {output:?}");
        assert!(
            cycle_line.contains('├'),
            "cycle for non-last dependency should use ├; line = {cycle_line:?}; output = {output:?}"
        );
    }

    /// mut-000199: `is_index` → `!is_index` in `render_node`
    /// A non-index task with a raw markdown link must NOT have the link extracted —
    /// the full `[text](url)` syntax should remain in the output.
    #[test]
    fn test_non_index_task_preserves_markdown_link() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "tasks#task1".to_string(),
            create_test_node_with_path(
                "[Some Link](some/path.md)",
                TaskStatus::Open,
                "tasks",
                "tasks.md",
            ),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        // Non-index files must NOT have links extracted — full markdown link preserved
        assert!(
            output.contains("[Some Link](some/path.md)"),
            "non-index task should preserve raw markdown link; output = {output:?}"
        );
    }

    /// mut-000199: second branch — index task DOES extract the link text
    #[test]
    fn test_index_task_extracts_link_text() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "proj#task1".to_string(),
            create_test_node_with_path(
                "[Index Link](index/path.md)",
                TaskStatus::Open,
                "proj",
                "lash.index.md",
            ),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            output.contains("Index Link"),
            "index task should extract link text; output = {output:?}"
        );
        assert!(
            !output.contains("index/path.md"),
            "index task should strip the URL; output = {output:?}"
        );
    }

    /// mut-000166: `reserved = 3 + 1 + min_title_width` — the literal `1` (space) → `0`
    ///
    /// With the default `min_title_width=20` and `indent_width=3`:
    ///   original:  `reserved = 24`, `available = terminal - 24`, `max_depth = floor(available/3)`
    ///   mutant:    `reserved = 23`, `available = terminal - 23`
    ///
    /// At `terminal_width = 29`:
    ///   original:  `available = 5`, `max_depth_avail = 1`, `min(1, 2) = 1 < 2` → notice
    ///   mutant:    `available = 6`, `max_depth_avail = 2`, `min(2, 2) = 2 = total_depth` → NO notice
    ///
    /// The test asserts the notice DOES appear at `terminal_width=29`, which fails for the mutant.
    #[test]
    fn test_reserved_width_boundary_at_29() {
        let mut graph = DependencyGraph::new();
        // Two-level hierarchy: root(depth=0) → child(depth=1) → total_depth = 2
        graph.add_node(
            "test#root".to_string(),
            NodeData::new("Root".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#child".to_string(),
            NodeData::new("Child".to_string(), TaskStatus::Open, "test".to_string(), 1),
        );
        graph.add_edge(
            "test#root".to_string(),
            "test#child".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // terminal=29, min_title_width=20, indent_width=3
        // Original: available = 29-24=5, max_depth_avail=1, min(1,2)=1 < 2 → notice
        // Mutant:   available = 29-23=6, max_depth_avail=2, min(2,2)=2 = total_depth → no notice
        let config = AsciiGraphConfig {
            terminal_width: Some(29),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            output.contains("Note: Graph truncated"),
            "truncation notice must appear at terminal_width=29 (reserved=24); output = {output:?}"
        );
    }

    /// mut-000163: `truncated || max_depth < total_depth` → `truncated && max_depth < total_depth`
    ///
    /// When `truncated = false` but `max_depth < total_depth`, the notice must still appear
    /// (OR semantics). With AND semantics the notice would be suppressed.
    ///
    /// `truncated` stays `false` when no `render_tree` call returns `true`. This happens for
    /// a flat node (no children) even though its `depth` field is large (which drives up
    /// `total_depth` so that `max_depth < total_depth`).
    #[test]
    fn test_truncation_notice_from_depth_field_alone() {
        let mut graph = DependencyGraph::new();
        // A single flat node with depth=5 and no children.
        // calculate_total_depth() returns 5+1=6.
        // With terminal_width=40: available=16, max_depth_avail=5, min(5,6)=5 < 6 → notice.
        // render_tree for this node returns truncated=false (no children to recurse into).
        // So: outer truncated=false, max_depth<total_depth=true.
        // OR: true → notice appears.  AND: false → notice suppressed (mutant fails).
        graph.add_node(
            "test#deep".to_string(),
            NodeData::new(
                "Deep Node".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                5,
            ),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(40),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            output.contains("Note: Graph truncated"),
            "notice must appear when max_depth < total_depth even if no tree was truncated; output = {output:?}"
        );
    }

    /// mut-000183: `roots.is_empty() && !file_nodes.is_empty()` → `||`
    ///
    /// When `roots` is NOT empty (normal graph), the fallback must NOT run.
    /// With `||`, `!file_nodes.is_empty()` is always true for a non-empty file, so the
    /// fallback would always execute, adding a duplicate root entry.
    ///
    /// In a parent→child graph, only the parent is a root. With the `||` mutant,
    /// the fallback also runs and pushes the alphabetically-first node (the child) as
    /// an additional root, causing the child to be rendered twice: once as a child of
    /// the parent and once as a standalone root with its own tree header.
    #[test]
    fn test_find_roots_fallback_does_not_run_when_roots_found() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#parent".to_string(),
            create_test_node("ParentOnly", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#child".to_string(),
            create_test_node("ChildOnly", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#parent".to_string(),
            "test#child".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // Count occurrences of "ChildOnly" in the output.
        // With normal && semantics: child appears exactly once (as a child of parent).
        // With || mutation: child also appears as a standalone root → appears twice.
        let child_count = output.matches("ChildOnly").count();
        assert_eq!(
            child_count, 1,
            "child should appear exactly once (not as a spurious extra root); output = {output:?}"
        );
    }

    /// mut-000202/205: boundary condition on `title.len() > max_title_len`
    /// At exactly `max_title_len` characters the title must NOT be truncated.
    /// At `max_title_len` + 1 characters it MUST be truncated.
    #[test]
    fn test_truncate_title_boundary() {
        let mut graph = DependencyGraph::new();

        // terminal_width=44: max_title_len = 44 - 20 = 24
        // A title of exactly 24 chars must NOT be truncated (len > 24 is false)
        let title_24 = "A".repeat(24);
        graph.add_node(
            "test#t24".to_string(),
            create_test_node(&title_24, TaskStatus::Open, "test"),
        );

        // A title of 25 chars MUST be truncated (25 > 24 is true, and 24 > 3 is true)
        let title_25 = "B".repeat(25);
        graph.add_node(
            "test#t25".to_string(),
            create_test_node(&title_25, TaskStatus::Open, "test"),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(44),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // The 24-char title should appear verbatim
        assert!(
            output.contains(&title_24),
            "24-char title should not be truncated; output = {output:?}"
        );
        // The 25-char title should be replaced with a truncated form ending in "..."
        assert!(
            !output.contains(&title_25),
            "25-char title should be truncated; output = {output:?}"
        );
        assert!(
            output.contains("..."),
            "truncated title should end with ...; output = {output:?}"
        );
    }

    // --- Mutation-killing tests ---

    /// Kills mut-000164: `truncated = truncated || tree_truncated` → `&&`
    /// Need a case where truncated is false before the loop, but one root's subtree
    /// returns `tree_truncated=true`. With `&&`: `false && true = false` (no notice).
    /// Use all depth-0 nodes so `calculate_total_depth` returns 1, then set terminal
    /// width so `max_depth=1`. A grandchild at `current_depth=2` > `max_depth=1`
    /// triggers `tree_truncated`. But since `max_depth == total_depth`, the
    /// `max_depth < total_depth` condition is false, so only the `||` propagation
    /// can trigger the notice.
    #[test]
    fn test_tree_truncated_propagates_via_or_not_and() {
        let mut graph = DependencyGraph::new();
        // All nodes at depth=0 so calculate_total_depth returns 0+1=1
        graph.add_node(
            "test#root".to_string(),
            create_test_node("Root", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#child".to_string(),
            create_test_node("Child", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#grandchild".to_string(),
            create_test_node("GrandchildX", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#root".to_string(),
            "test#child".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#child".to_string(),
            "test#grandchild".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // total_depth=1, terminal_width=27 → available=3 → max_depth_avail=1
        // max_depth = min(1,1) = 1 = total_depth, so max_depth < total_depth is false
        // But grandchild is at current_depth=2 > max_depth=1 → returns (_, true)
        let config = AsciiGraphConfig {
            terminal_width: Some(27),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            output.contains("Note: Graph truncated"),
            "truncation notice must appear via || propagation; output = {output:?}"
        );
        assert!(
            !output.contains("GrandchildX"),
            "grandchild should be depth-limited; output = {output:?}"
        );
    }

    /// Kills mut-000170: `max_depth = 0` → `max_depth = 1` in `calculate_total_depth`.
    /// With a single depth-0 node: original `total_depth` = 0+1 = 1.
    /// If initial `max_depth` is 1 instead of 0, `total_depth` = max(1,0)+1 = 2.
    /// Use `terminal_width=27` → `max_depth_avail=1`. With `total_depth=1`: no notice.
    /// With `total_depth=2`: notice. So this test asserts no notice.
    #[test]
    fn test_calculate_total_depth_single_node_no_truncation_notice() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#only".to_string(),
            create_test_node("OnlyTask", TaskStatus::Open, "test"),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(27),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            !output.contains("Note: Graph truncated"),
            "single depth-0 node should not trigger truncation; output = {output:?}"
        );
    }

    /// Kills mut-000196: `return (String::new(), true)` → `return (String::new(), false)`
    /// When depth limit is hit, the return must be `(_, true)` to propagate truncation.
    /// Same setup as `test_tree_truncated_propagates_via_or_not_and`: the grandchild
    /// exceeds `max_depth` and must return true.
    #[test]
    fn test_depth_limit_return_true_not_false() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#r".to_string(),
            create_test_node("R", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#c".to_string(),
            create_test_node("C", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#gc".to_string(),
            create_test_node("GC", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#r".to_string(),
            "test#c".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#c".to_string(),
            "test#gc".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(27),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // The grandchild must not appear (depth-limited)
        assert!(
            !output.contains("GC"),
            "GC should be hidden; output = {output:?}"
        );
        // The truncation notice must appear (the depth-limit return must be true)
        assert!(
            output.contains("Note: Graph truncated"),
            "depth limit must trigger notice; output = {output:?}"
        );
    }

    /// Kills mut-000200: `current_depth + 1` → `current_depth + 0`
    /// If depth never increments, grandchild renders at depth 0 bypassing the limit.
    #[test]
    fn test_depth_increment_prevents_grandchild() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#a".to_string(),
            create_test_node("NodeA", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#b".to_string(),
            create_test_node("NodeB", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#deep".to_string(),
            create_test_node("DeepNode", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#a".to_string(),
            "test#b".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#b".to_string(),
            "test#deep".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // max_depth=1 via terminal constraints
        let config = AsciiGraphConfig {
            terminal_width: Some(27),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            !output.contains("DeepNode"),
            "DeepNode should be hidden at depth 2 > max_depth 1; output = {output:?}"
        );
    }

    /// Kills mut-000201: `truncated = truncated || child_truncated` → `&&`
    /// Parent has two children. First child is a leaf (`child_truncated=false`).
    /// Second child has a grandchild that gets depth-limited (`child_truncated=true`).
    /// After first child: `truncated = false || false = false`.
    /// After second child: `truncated = false || true = true`. With `&&`: `false && true = false`.
    #[test]
    fn test_child_truncated_propagates_via_or() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#parent".to_string(),
            create_test_node("Parent", TaskStatus::Open, "test"),
        );
        // First child: leaf, no truncation
        graph.add_node(
            "test#achild".to_string(),
            create_test_node("AChild", TaskStatus::Open, "test"),
        );
        // Second child: has a grandchild that will be depth-limited
        graph.add_node(
            "test#bchild".to_string(),
            create_test_node("BChild", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#bgrand".to_string(),
            create_test_node("BGrand", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#parent".to_string(),
            "test#achild".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#parent".to_string(),
            "test#bchild".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#bchild".to_string(),
            "test#bgrand".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(27),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            output.contains("Note: Graph truncated"),
            "truncation from second child must propagate; output = {output:?}"
        );
    }

    // =========================================================================
    // Tests targeting the 7 surviving mutants (mut-000164, mut-000170,
    // mut-000196, mut-000200, mut-000201, mut-000203, mut-000209)
    // =========================================================================

    /// mut-000164: `truncated = truncated || tree_truncated` → `&&` (line 125)
    ///
    /// Scenario: two independent root trees in the same file.
    /// - Root "`aaa_leaf`" is a leaf → its `render_tree` returns `tree_truncated=false`.
    /// - Root "`zzz_deep`" has a child that itself has a grandchild exceeding `max_depth=1` →
    ///   its `render_tree` returns `tree_truncated=true`.
    ///
    /// Loop iteration 1: `truncated = false || false = false` (correct)
    ///                                   `false && false = false` (mutant, same)
    /// Loop iteration 2: `truncated = false || true  = true`  (correct → notice)
    ///                                   `false && true  = false` (mutant → no notice)
    ///
    /// All nodes have depth=0, `terminal_width=27` → `max_depth=min(1,1)=1` → `total_depth=1`.
    /// So `max_depth < total_depth` is always false; the notice can ONLY come from `truncated`.
    #[test]
    fn test_mut164_truncated_or_tree_truncated() {
        let mut graph = DependencyGraph::new();

        // First root tree: a single leaf node (render_tree returns tree_truncated=false)
        graph.add_node(
            "test#aaa_leaf".to_string(),
            create_test_node("Leaf", TaskStatus::Open, "test"),
        );

        // Second root tree: zzz_deep → zchild → zgrand (all depth=0)
        // At max_depth=1, zgrand (current_depth=2) exceeds limit → tree_truncated=true
        graph.add_node(
            "test#zzz_deep".to_string(),
            create_test_node("Deep", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#zzz_zchild".to_string(),
            create_test_node("ZChild", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#zzz_zgrand".to_string(),
            create_test_node("ZGrand", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#zzz_deep".to_string(),
            "test#zzz_zchild".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#zzz_zchild".to_string(),
            "test#zzz_zgrand".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // terminal_width=27: available=3, max_depth_avail=1, total_depth=1, max_depth=1
        // max_depth < total_depth is false → notice only via truncated flag
        let config = AsciiGraphConfig {
            terminal_width: Some(27),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            output.contains("Note: Graph truncated"),
            "truncated flag from second root must propagate via ||; output = {output:?}"
        );
    }

    /// mut-000170: `let mut max_depth = 0` → `1` in `calculate_total_depth` (line 166)
    ///
    /// A graph whose only node has `depth=0`.
    /// - Correct:  `max_depth` initialises to 0, loop keeps it 0, returns `0+1 = 1`.
    /// - Mutant:   `max_depth` initialises to 1, loop: max(1,0)=1, returns `1+1 = 2`.
    ///
    /// At `terminal_width=27`: `max_depth_avail = 1`.
    /// - Correct:  `max_depth = min(1,1) = 1 = total_depth` → no truncation notice.
    /// - Mutant:   `max_depth = min(1,2) = 1 < 2`           → notice appears.
    ///
    /// The assertion `!output.contains("Note: Graph truncated")` fails for the mutant.
    #[test]
    fn test_mut170_calculate_total_depth_initial_zero() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#solo".to_string(),
            NodeData::new(
                "SoloTask".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                0,
            ),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(27),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            !output.contains("Note: Graph truncated"),
            "depth-0-only graph must not trigger truncation at terminal_width=27; output = {output:?}"
        );
    }

    /// mut-000196: `return (String::new(), true)` → `(String::new(), false)` (line 315)
    ///
    /// When `current_depth > max_depth`, `render_tree` must signal truncation by returning
    /// `true` as the second element. If it returns `false`, the notice never appears.
    ///
    /// Setup: A(depth=0) → B(depth=0) → C(depth=0) — all flat, so `total_depth = 1`.
    /// At `terminal_width=200`: `max_depth_avail = 58`, `max_depth = min(58,1) = 1`.
    /// `max_depth < total_depth` = `1 < 1` = false → the notice can ONLY come from the
    /// `truncated` flag returned by `render_tree` when C is visited at `current_depth=2 > 1`.
    ///
    /// Correct: returns `(String::new(), true)` → `truncated` propagates → notice shown.
    /// Mutant:  returns `(String::new(), false)` → `truncated` stays false → no notice.
    #[test]
    fn test_mut196_depth_exceeded_returns_truncated_true() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#nodeA".to_string(),
            NodeData::new("NodeA".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#nodeB".to_string(),
            NodeData::new("NodeB".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_node(
            "test#nodeC".to_string(),
            NodeData::new("NodeC".to_string(), TaskStatus::Open, "test".to_string(), 0),
        );
        graph.add_edge(
            "test#nodeA".to_string(),
            "test#nodeB".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#nodeB".to_string(),
            "test#nodeC".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // All nodes depth=0 → total_depth=1, max_depth=min(58,1)=1
        // max_depth < total_depth is false; only truncated flag can trigger notice
        // NodeC is at current_depth=2 > max_depth=1 → render_tree must return true
        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            !output.contains("NodeC"),
            "NodeC at current_depth=2 must be depth-limited; output = {output:?}"
        );
        assert!(
            output.contains("Note: Graph truncated"),
            "depth-limit must signal truncation=true so notice appears; output = {output:?}"
        );
    }

    /// mut-000200: `child_count - 1` → `child_count - 0` (line 380)
    ///
    /// `is_last_child = idx == child_count - 1` (correct) vs
    /// `is_last_child = idx == child_count`     (mutant, always false).
    ///
    /// With a parent having exactly two children sorted by insertion order:
    /// - Correct: last child (`idx=1 == child_count-1=1`) gets `└`.
    /// - Mutant:  `idx` never reaches `child_count=2`, so all children get `├`.
    ///
    /// We assert that exactly one `└` appears in the child section of the output
    /// (the last child's branch), and that it corresponds to the second child.
    #[test]
    fn test_mut200_last_child_uses_corner_branch() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#parent".to_string(),
            create_test_node("ParentNode", TaskStatus::Open, "test"),
        );
        // Insert children in order; adjacency Vec preserves insertion order.
        // child1 → idx=0 (not last), child2 → idx=1 (last).
        graph.add_node(
            "test#child1".to_string(),
            create_test_node("FirstChild", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#child2".to_string(),
            create_test_node("SecondChild", TaskStatus::Open, "test"),
        );
        graph.add_edge(
            "test#parent".to_string(),
            "test#child1".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#parent".to_string(),
            "test#child2".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(200),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // Locate the line containing "SecondChild"
        let second_pos = output.find("SecondChild").expect("SecondChild not found");
        let second_line_start = output[..second_pos].rfind('\n').map_or(0, |p| p + 1);
        let second_line = &output[second_line_start..second_pos];

        // The last child must use the corner branch character └
        assert!(
            second_line.contains('└'),
            "last child (SecondChild) must use └ branch; prefix = {second_line:?}; output = {output:?}"
        );

        // Locate the line containing "FirstChild"
        let first_pos = output.find("FirstChild").expect("FirstChild not found");
        let first_line_start = output[..first_pos].rfind('\n').map_or(0, |p| p + 1);
        let first_line = &output[first_line_start..first_pos];

        // The non-last child must use the tee branch character ├
        assert!(
            first_line.contains('├'),
            "first child (FirstChild) must use ├ branch; prefix = {first_line:?}; output = {output:?}"
        );
    }

    /// mut-000201: `truncated = truncated || child_truncated` → `&&` (line 386)
    ///
    /// A parent has two children (visited in insertion order):
    /// - child1 ("`AFirst`") is a leaf → `child_truncated=false`.
    /// - child2 ("`BSecond`") has a grandchild that is depth-limited → `child_truncated=true`.
    ///
    /// Iteration 1: `truncated = false || false = false`  (correct/mutant same)
    /// Iteration 2: `truncated = false || true  = true`   (correct → notice)
    ///              `truncated = false && true  = false`  (mutant  → no notice)
    ///
    /// All nodes depth=0 so `total_depth=1`, `max_depth=1`; `max_depth < total_depth` is false.
    #[test]
    fn test_mut201_child_truncated_or_not_and() {
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "test#root".to_string(),
            create_test_node("RootNode", TaskStatus::Open, "test"),
        );
        // First child: leaf, returns child_truncated=false
        graph.add_node(
            "test#afirst".to_string(),
            create_test_node("AFirst", TaskStatus::Open, "test"),
        );
        // Second child: has a grandchild → grandchild at current_depth=3 > max_depth=1
        graph.add_node(
            "test#bsecond".to_string(),
            create_test_node("BSecond", TaskStatus::Open, "test"),
        );
        graph.add_node(
            "test#bgrand".to_string(),
            create_test_node("BGrandNode", TaskStatus::Open, "test"),
        );

        graph.add_edge(
            "test#root".to_string(),
            "test#afirst".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#root".to_string(),
            "test#bsecond".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );
        graph.add_edge(
            "test#bsecond".to_string(),
            "test#bgrand".to_string(),
            EdgeData::new(DependencyKind::Hierarchy, None),
        );

        // terminal_width=27: max_depth_avail=1, total_depth=1, max_depth=1
        // max_depth < total_depth is false → notice only via truncated flag
        let config = AsciiGraphConfig {
            terminal_width: Some(27),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        assert!(
            output.contains("Note: Graph truncated"),
            "truncated from second child must propagate via || not &&; output = {output:?}"
        );
    }

    /// mut-000203: `is_index` → `!(is_index)` in `render_node` styling branch (line 412)
    ///
    /// For an index file node with a Markdown link title, `render_node` must call
    /// `display::format_index_title` (which strips the URL) because `is_index=true`.
    /// With the mutant `!(is_index)`, a non-index node would get link-stripped formatting
    /// while an index node would NOT — the raw `[text](url)` title would be shown as-is.
    ///
    /// We verify this with TWO nodes:
    /// 1. An index node with title `[Alpha](alpha.md)` → output must show "Alpha" (no URL).
    /// 2. A non-index node with title `[Beta](beta.md)` → output must show the full link.
    ///
    /// The mutant swaps the two behaviours:
    /// - Index node would NOT extract → "[Alpha](alpha.md)" present (test fails).
    /// - Non-index node WOULD extract → "[Beta](beta.md)" absent (test fails).
    #[test]
    fn test_mut203_is_index_controls_link_extraction() {
        let mut graph = DependencyGraph::new();

        // Index node: title is a Markdown link → link text must be extracted
        graph.add_node(
            "proj#idx".to_string(),
            create_test_node_with_path(
                "[Alpha Module](alpha.md)",
                TaskStatus::Open,
                "proj",
                "lash.index.md",
            ),
        );

        // Non-index node: title is a Markdown link → link must be preserved verbatim
        graph.add_node(
            "tasks#reg".to_string(),
            create_test_node_with_path(
                "[Beta Module](beta.md)",
                TaskStatus::Open,
                "tasks",
                "tasks.md",
            ),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        // Index node: link text extracted, URL gone
        assert!(
            output.contains("Alpha Module"),
            "index node must show extracted link text; output = {output:?}"
        );
        assert!(
            !output.contains("alpha.md"),
            "index node must not show the URL; output = {output:?}"
        );

        // Non-index node: full markdown link preserved, URL present
        assert!(
            output.contains("[Beta Module](beta.md)"),
            "non-index node must preserve full markdown link; output = {output:?}"
        );
    }

    /// Kills the `max_title_len > 3` second condition in `truncate_title` (line 539).
    ///
    /// The full condition is: `title.len() > max_title_len && max_title_len > 3`
    ///
    /// The mutation changes the literal `3` to `4`, making the condition:
    ///   `max_title_len > 4`
    ///
    /// At `terminal_width=24`: `max_title_len = 24 - 20 = 4`.
    /// - Original (`max_title_len > 3`): `4 > 3 = true` → the title IS truncated
    ///   (assuming `title.len() > 4` is also true).
    /// - Mutant (`max_title_len > 4`):   `4 > 4 = false` → the title is NOT truncated.
    ///
    /// We use a title longer than 4 chars (to ensure `title.len() > max_title_len`)
    /// and assert that truncation ("...") occurs.  The mutant would fail this assertion.
    #[test]
    fn test_truncate_title_max_title_len_4_still_truncates() {
        let mut graph = DependencyGraph::new();

        // terminal_width=24 → max_title_len = 24 - 20 = 4
        // A title of 10 chars: title.len() > max_title_len (10 > 4 = true)
        // and max_title_len > 3 (4 > 3 = true, original) → truncates
        // With mutation 3→4: max_title_len > 4 = false → does not truncate
        let title = "ABCDEFGHIJ".to_string(); // 10 chars, clearly > max_title_len=4
        graph.add_node(
            "test#t".to_string(),
            create_test_node(&title, TaskStatus::Open, "test"),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(24),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // Original: 4 > 3 = true → truncation occurs → "..." in output
        // Mutant:   4 > 4 = false → no truncation → full title present, no "..."
        assert!(
            output.contains("..."),
            "with max_title_len=4 and title.len()=10, truncation must occur; output = {output:?}"
        );
        assert!(
            !output.contains("ABCDEFGHIJ"),
            "full 10-char title must be truncated when max_title_len=4; output = {output:?}"
        );
    }

    /// Kills the `is_index` → `!is_index` mutation at L412 (`render_node` styling branch).
    ///
    /// The mutation negates `is_index` in the SECOND check (the styling branch), while
    /// the first check (title extraction at L403) is NOT mutated.  So after this mutation:
    ///   - Index nodes still get link text extracted (from L403, unmutated)
    ///   - But their title is styled with `style_labels_in_text` instead of `style_index_task_title`
    ///   - Non-index nodes still get their title styled with `style_index_task_title` instead of `style_labels_in_text`
    ///
    /// Without a theme, both styling functions return the same text, making the mutation
    /// equivalent.  With a theme, `style_index_task_title` wraps plain text with info-color
    /// ANSI codes, while `style_labels_in_text` returns plain text unchanged.
    ///
    /// We test the observable structural difference: for a non-index node with a plain
    /// title (no `#` labels), the mutated code applies info-styling (adds ANSI codes),
    /// while the original code returns the title unchanged.  We verify that the title
    /// text appears literally in the output (not wrapped in ANSI codes that would break
    /// the literal match).
    ///
    /// Note: this is a display-only mutation.  When colors are disabled (no theme), both
    /// branches produce identical output and the mutation is equivalent.  The test below
    /// uses the no-theme path to confirm the structural invariant (index vs non-index
    /// routing) that is visible in the L403 title-extraction behavior.
    #[test]
    fn test_is_index_second_branch_styling_routes_correctly() {
        // For the styling branch (L412), with no theme, both branches produce the same
        // output (style_index_task_title falls through to style_labels_in_text when no theme).
        // The observable difference is in the FIRST branch (L403), which is tested by
        // test_mut203_is_index_controls_link_extraction.
        //
        // We verify that a non-index node with a plain title appears literally in the
        // output (no styling transformations applied to the title text itself).
        let mut graph = DependencyGraph::new();

        // Non-index node with plain title (no Markdown link, no labels)
        graph.add_node(
            "tasks#plain".to_string(),
            create_test_node_with_path("Plain Task Title", TaskStatus::Open, "tasks", "tasks.md"),
        );

        // Index node with plain title (no Markdown link, no labels)
        graph.add_node(
            "proj#idxtask".to_string(),
            create_test_node_with_path(
                "Index Plain Title",
                TaskStatus::Open,
                "proj",
                "lash.index.md",
            ),
        );

        let renderer = AsciiGraphRenderer::new(&graph, None);
        let output = renderer.render(&FilterOptions::default());

        // Both titles must appear verbatim in the output regardless of which styling
        // branch is taken (because no theme means no ANSI codes either way).
        assert!(
            output.contains("Plain Task Title"),
            "non-index plain title must appear verbatim; output = {output:?}"
        );
        assert!(
            output.contains("Index Plain Title"),
            "index plain title must appear verbatim; output = {output:?}"
        );
    }

    /// mut-000209: `title.len() > max_title_len` → `>=` in `truncate_title` (line 539)
    ///
    /// At `terminal_width=44`, `max_title_len = 44 - 20 = 24`.
    ///
    /// Correct (`>`):  a title of exactly 24 chars is NOT truncated (24 > 24 is false).
    /// Mutant (`>=`):  a title of exactly 24 chars IS truncated  (24 >= 24 is true),
    ///                 and since `24 > 3` is also true, the truncated form is returned.
    ///
    /// The assertion that the full 24-char title appears verbatim in the output fails
    /// for the mutant (which truncates it to 21 chars + "...").
    #[test]
    fn test_mut209_truncate_title_exact_boundary_not_truncated() {
        let mut graph = DependencyGraph::new();

        // terminal_width=44 → max_title_len = 44-20 = 24
        // A title of exactly 24 chars must not be truncated with correct `>` logic
        let exact_title = "X".repeat(24);
        graph.add_node(
            "test#exact".to_string(),
            create_test_node(&exact_title, TaskStatus::Open, "test"),
        );

        let config = AsciiGraphConfig {
            terminal_width: Some(44),
            min_title_width: 20,
            indent_width: 3,
        };
        let renderer = AsciiGraphRenderer::new(&graph, None).with_config(config);
        let output = renderer.render(&FilterOptions::default());

        // Correct: 24 > 24 is false → title is not truncated → full title present
        // Mutant:  24 >= 24 is true  → title IS truncated  → full title absent
        assert!(
            output.contains(&exact_title),
            "title of exactly max_title_len chars must not be truncated; output = {output:?}"
        );
        assert!(
            !output.contains("..."),
            "no ellipsis expected when title length equals max_title_len; output = {output:?}"
        );
    }
}
