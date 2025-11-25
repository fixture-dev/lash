//! Tree formatting utilities for CLI output
//!
//! Provides formatting functions for rendering hierarchical data structures
//! as ASCII or Unicode tree diagrams in the terminal.

use crate::theme::CliTheme;
use lash_types::tree::{TreeChars, TreeNode};

/// Formatter for rendering tree structures in CLI output
///
/// The `TreeFormatter` provides methods to format hierarchical data as
/// tree diagrams with proper indentation and tree characters.
///
/// # Examples
///
/// ```no_run
/// use lash_cli::tree_formatter::TreeFormatter;
/// use lash_types::tree::{TreeNode, TreeChars};
///
/// let formatter = TreeFormatter::new(false, 5, None);
/// let roots = vec![TreeNode::new("root".to_string(), 0)];
///
/// let lines = formatter.format_tree(&roots, |data, _| data.clone());
/// for line in lines {
///     println!("{}", line);
/// }
/// ```
pub struct TreeFormatter {
    /// Tree character set (Unicode or ASCII)
    chars: TreeChars,
    /// Maximum depth to render
    max_depth: usize,
    /// Optional theme for colored output
    theme: Option<CliTheme>,
}

impl TreeFormatter {
    /// Create a new tree formatter
    ///
    /// # Arguments
    ///
    /// * `ascii` - Use ASCII characters instead of Unicode
    /// * `max_depth` - Maximum depth to render (nodes deeper than this are not displayed)
    /// * `theme` - Optional theme for colored output
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_cli::tree_formatter::TreeFormatter;
    ///
    /// // Unicode formatter with max depth of 5
    /// let formatter = TreeFormatter::new(false, 5, None);
    ///
    /// // ASCII formatter with max depth of 3
    /// let formatter = TreeFormatter::new(true, 3, None);
    /// ```
    #[must_use]
    pub fn new(ascii: bool, max_depth: usize, theme: Option<CliTheme>) -> Self {
        let chars = if ascii {
            TreeChars::Ascii
        } else {
            TreeChars::detect()
        };

        Self {
            chars,
            max_depth,
            theme,
        }
    }

    /// Format a tree into printable lines
    ///
    /// Takes a list of tree roots and formats each node into a string line
    /// using the provided formatter callback.
    ///
    /// # Arguments
    ///
    /// * `roots` - Root nodes of the tree(s) to format
    /// * `format_data` - Callback to format the data of each node
    ///
    /// # Returns
    ///
    /// A vector of formatted strings, one per visible node
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lash_cli::tree_formatter::TreeFormatter;
    /// use lash_types::tree::TreeNode;
    ///
    /// let formatter = TreeFormatter::new(false, 5, None);
    /// let mut root = TreeNode::new("root".to_string(), 0);
    /// root.children.push(TreeNode::new("child".to_string(), 1));
    /// root.expand();
    ///
    /// let lines = formatter.format_tree(&[root], |data, _| data.clone());
    /// assert_eq!(lines.len(), 2); // root + child
    /// ```
    pub fn format_tree<T, F>(&self, roots: &[TreeNode<T>], format_data: F) -> Vec<String>
    where
        F: Fn(&T, &Self) -> String,
    {
        let mut lines = Vec::new();

        for root in roots {
            // All roots are treated as "last" to avoid unnecessary branch characters
            // since there's no actual "parent" context for roots
            self.format_node_recursive(root, true, &[], &format_data, &mut lines);
        }

        lines
    }

    /// Format a single node with its prefix
    ///
    /// Formats a node with the appropriate tree prefix based on its position
    /// in the hierarchy.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to format
    /// * `is_last` - Whether this is the last sibling at its level
    /// * `ancestors_is_last` - Boolean flags for each ancestor level indicating if that ancestor was the last sibling
    /// * `format_data` - Callback to format the node's data
    ///
    /// # Returns
    ///
    /// A formatted string representing the node
    pub fn format_node<T, F>(
        &self,
        node: &TreeNode<T>,
        is_last: bool,
        ancestors_is_last: &[bool],
        format_data: &F,
    ) -> String
    where
        F: Fn(&T, &Self) -> String,
    {
        let prefix = self.build_prefix(node.depth, is_last, ancestors_is_last);
        let data_str = format_data(&node.data, self);
        format!("{prefix}{data_str}")
    }

    /// Build tree prefix string from ancestor information
    ///
    /// Creates the indentation and tree characters for a node based on
    /// its depth and the last-sibling status of its ancestors.
    ///
    /// # Arguments
    ///
    /// * `depth` - Depth of the current node in the tree
    /// * `is_last` - Whether this node is the last sibling
    /// * `ancestors_is_last` - Boolean flags for each ancestor level
    ///
    /// # Returns
    ///
    /// A string containing the tree prefix characters
    fn build_prefix(&self, depth: usize, is_last: bool, ancestors_is_last: &[bool]) -> String {
        let mut prefix = String::new();

        // Add indentation for ancestor levels
        for &ancestor_is_last in ancestors_is_last {
            if ancestor_is_last {
                prefix.push_str(self.chars.empty());
            } else {
                prefix.push_str(self.chars.vertical());
            }
        }

        // Add branch character for current level (skip for root nodes at depth 0)
        if depth > 0 {
            if is_last {
                prefix.push_str(self.chars.last_branch());
            } else {
                prefix.push_str(self.chars.branch());
            }
        }

        prefix
    }

    /// Recursively format a node and its children
    ///
    /// Internal helper that handles the recursive tree traversal and formatting.
    fn format_node_recursive<T, F>(
        &self,
        node: &TreeNode<T>,
        is_last: bool,
        ancestors_is_last: &[bool],
        format_data: &F,
        lines: &mut Vec<String>,
    ) where
        F: Fn(&T, &Self) -> String,
    {
        // Check if we've exceeded max depth
        if node.depth >= self.max_depth {
            return;
        }

        // Format this node
        let line = self.format_node(node, is_last, ancestors_is_last, format_data);
        lines.push(line);

        // Format children if expanded
        if node.expanded && !node.children.is_empty() {
            // Only add to ancestors if this node has depth > 0
            // (depth 0 nodes don't contribute to indentation)
            let new_ancestors = if node.depth > 0 {
                let mut ancestors = ancestors_is_last.to_vec();
                ancestors.push(is_last);
                ancestors
            } else {
                ancestors_is_last.to_vec()
            };

            let child_count = node.children.len();
            for (i, child) in node.children.iter().enumerate() {
                let is_last_child = i == child_count - 1;
                self.format_node_recursive(
                    child,
                    is_last_child,
                    &new_ancestors,
                    format_data,
                    lines,
                );
            }
        }
    }

    /// Get the theme (for use in format callbacks)
    ///
    /// Returns a reference to the theme if one is configured.
    #[must_use]
    pub fn theme(&self) -> Option<&CliTheme> {
        self.theme.as_ref()
    }

    /// Get the tree characters being used
    #[must_use]
    pub fn chars(&self) -> TreeChars {
        self.chars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_empty_tree() {
        let formatter = TreeFormatter::new(true, 5, None);
        let roots: Vec<TreeNode<String>> = vec![];

        let lines = formatter.format_tree(&roots, |data, _| data.clone());
        assert!(lines.is_empty());
    }

    #[test]
    fn test_format_single_root() {
        let formatter = TreeFormatter::new(true, 5, None);
        let roots = vec![TreeNode::new("root".to_string(), 0)];

        let lines = formatter.format_tree(&roots, |data, _| data.clone());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "root");
    }

    #[test]
    fn test_format_root_with_children() {
        let formatter = TreeFormatter::new(true, 5, None);
        let mut root = TreeNode::new("root".to_string(), 0);
        root.children.push(TreeNode::new("child1".to_string(), 1));
        root.children.push(TreeNode::new("child2".to_string(), 1));
        root.expand();

        let lines = formatter.format_tree(&[root], |data, _| data.clone());
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "root");
        assert_eq!(lines[1], "+-- child1");
        assert_eq!(lines[2], "\\-- child2");
    }

    #[test]
    fn test_format_collapsed_node() {
        let formatter = TreeFormatter::new(true, 5, None);
        let mut root = TreeNode::new("root".to_string(), 0);
        root.children.push(TreeNode::new("child1".to_string(), 1));
        root.children.push(TreeNode::new("child2".to_string(), 1));
        // Don't expand - children should not be rendered

        let lines = formatter.format_tree(&[root], |data, _| data.clone());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "root");
    }

    #[test]
    fn test_format_nested_tree() {
        let formatter = TreeFormatter::new(true, 5, None);
        let mut root = TreeNode::new("root".to_string(), 0);
        let mut child1 = TreeNode::new("child1".to_string(), 1);
        child1
            .children
            .push(TreeNode::new("grandchild1".to_string(), 2));
        child1
            .children
            .push(TreeNode::new("grandchild2".to_string(), 2));
        child1.expand();
        root.children.push(child1);
        root.children.push(TreeNode::new("child2".to_string(), 1));
        root.expand();

        let lines = formatter.format_tree(&[root], |data, _| data.clone());
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], "root");
        assert_eq!(lines[1], "+-- child1");
        assert_eq!(lines[2], "|   +-- grandchild1");
        assert_eq!(lines[3], "|   \\-- grandchild2");
        assert_eq!(lines[4], "\\-- child2");
    }

    #[test]
    fn test_format_multiple_roots() {
        let formatter = TreeFormatter::new(true, 5, None);
        let roots = vec![
            TreeNode::new("root1".to_string(), 0),
            TreeNode::new("root2".to_string(), 0),
        ];

        let lines = formatter.format_tree(&roots, |data, _| data.clone());
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "root1");
        assert_eq!(lines[1], "root2");
    }

    #[test]
    fn test_max_depth_limiting() {
        let formatter = TreeFormatter::new(true, 2, None);
        let mut root = TreeNode::new("root".to_string(), 0);
        let mut child = TreeNode::new("child".to_string(), 1);
        child
            .children
            .push(TreeNode::new("grandchild".to_string(), 2));
        child.expand();
        root.children.push(child);
        root.expand();

        let lines = formatter.format_tree(&[root], |data, _| data.clone());
        // Should only include root and child (depth 0 and 1), not grandchild (depth 2 >= max_depth 2)
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "root");
        assert_eq!(lines[1], "\\-- child");
    }

    #[test]
    fn test_unicode_characters() {
        let formatter = TreeFormatter::new(false, 5, None);
        let mut root = TreeNode::new("root".to_string(), 0);
        root.children.push(TreeNode::new("child1".to_string(), 1));
        root.children.push(TreeNode::new("child2".to_string(), 1));
        root.expand();

        let lines = formatter.format_tree(&[root], |data, _| data.clone());
        assert_eq!(lines.len(), 3);
        // Unicode characters depend on environment detection
        // Just verify we got the right number of lines
    }

    #[test]
    fn test_custom_formatter() {
        let formatter = TreeFormatter::new(true, 5, None);
        let mut root = TreeNode::new(1, 0);
        root.children.push(TreeNode::new(2, 1));
        root.expand();

        let lines = formatter.format_tree(&[root], |data, _| format!("Item {data}"));
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Item 1");
        assert_eq!(lines[1], "\\-- Item 2");
    }
}
