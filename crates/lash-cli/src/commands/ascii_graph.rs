//! ASCII graph renderer for terminal display
//!
//! This module provides a minimalist ASCII/Unicode box-drawing renderer for
//! dependency graphs that fits within terminal width constraints.

use crossterm::terminal;
use lash_core::dependency::{DependencyGraph, FilterOptions, NodeData};
use lash_core::display;
use lash_types::TaskStatus;
use std::collections::{BTreeMap, HashSet};

use lash_cli::theme::CliTheme;

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
            .unwrap_or_else(|| terminal::size().map(|(w, _)| w).unwrap_or(80))
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
            format!("{}...", &title[..max_title_len - 3])
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
}
