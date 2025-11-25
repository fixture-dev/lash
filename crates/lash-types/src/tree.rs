//! Generic tree data structure for hierarchical rendering
//!
//! Provides a flexible tree structure for representing hierarchical data
//! (file trees, task hierarchies, etc.) with support for expansion state
//! and rendering with Unicode or ASCII tree characters.

use std::env;

/// A generic tree node that can hold any data type
///
/// The tree supports expansion/collapse state tracking and provides
/// methods for recursive operations and flattening for display.
///
/// # Examples
///
/// ```
/// use lash_types::tree::TreeNode;
///
/// let mut root = TreeNode::new("root".to_string(), 0);
/// root.children.push(TreeNode::new("child1".to_string(), 1));
/// root.children.push(TreeNode::new("child2".to_string(), 1));
///
/// assert_eq!(root.data, "root");
/// assert_eq!(root.children.len(), 2);
/// assert!(root.has_children());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode<T> {
    /// The data stored in this node
    pub data: T,
    /// Child nodes
    pub children: Vec<TreeNode<T>>,
    /// Whether this node is expanded (shows children)
    pub expanded: bool,
    /// Depth level in the tree (0 = root)
    pub depth: usize,
}

impl<T> TreeNode<T> {
    /// Create a new tree node with the given data and depth
    ///
    /// The node starts with no children and in collapsed state.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeNode;
    ///
    /// let node = TreeNode::new("data".to_string(), 0);
    /// assert_eq!(node.depth, 0);
    /// assert!(!node.expanded);
    /// assert!(!node.has_children());
    /// ```
    #[must_use]
    pub fn new(data: T, depth: usize) -> Self {
        Self {
            data,
            children: Vec::new(),
            expanded: false,
            depth,
        }
    }

    /// Create a new tree node with children
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeNode;
    ///
    /// let children = vec![
    ///     TreeNode::new("child1".to_string(), 1),
    ///     TreeNode::new("child2".to_string(), 1),
    /// ];
    /// let node = TreeNode::with_children("parent".to_string(), 0, children);
    /// assert_eq!(node.children.len(), 2);
    /// assert!(node.has_children());
    /// ```
    #[must_use]
    pub fn with_children(data: T, depth: usize, children: Vec<TreeNode<T>>) -> Self {
        Self {
            data,
            children,
            expanded: false,
            depth,
        }
    }

    /// Check if this node has children
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeNode;
    ///
    /// let mut node = TreeNode::new("data".to_string(), 0);
    /// assert!(!node.has_children());
    ///
    /// node.children.push(TreeNode::new("child".to_string(), 1));
    /// assert!(node.has_children());
    /// ```
    #[must_use]
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Expand this node to show its children
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeNode;
    ///
    /// let mut node = TreeNode::new("data".to_string(), 0);
    /// assert!(!node.expanded);
    ///
    /// node.expand();
    /// assert!(node.expanded);
    /// ```
    pub fn expand(&mut self) {
        self.expanded = true;
    }

    /// Collapse this node to hide its children
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeNode;
    ///
    /// let mut node = TreeNode::new("data".to_string(), 0);
    /// node.expand();
    /// assert!(node.expanded);
    ///
    /// node.collapse();
    /// assert!(!node.expanded);
    /// ```
    pub fn collapse(&mut self) {
        self.expanded = false;
    }

    /// Toggle the expansion state of this node
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeNode;
    ///
    /// let mut node = TreeNode::new("data".to_string(), 0);
    /// assert!(!node.expanded);
    ///
    /// node.toggle();
    /// assert!(node.expanded);
    ///
    /// node.toggle();
    /// assert!(!node.expanded);
    /// ```
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Recursively expand all nodes up to the given maximum depth
    ///
    /// # Arguments
    ///
    /// * `max_depth` - Maximum depth to expand to (nodes deeper than this remain collapsed)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeNode;
    ///
    /// let mut root = TreeNode::new("root".to_string(), 0);
    /// let mut child = TreeNode::new("child".to_string(), 1);
    /// child.children.push(TreeNode::new("grandchild".to_string(), 2));
    /// root.children.push(child);
    ///
    /// root.expand_all(2);
    /// assert!(root.expanded);
    /// assert!(root.children[0].expanded);
    /// assert!(!root.children[0].children[0].expanded);
    /// ```
    pub fn expand_all(&mut self, max_depth: usize) {
        if self.depth < max_depth {
            self.expanded = true;
            for child in &mut self.children {
                child.expand_all(max_depth);
            }
        }
    }

    /// Recursively collapse all nodes
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeNode;
    ///
    /// let mut root = TreeNode::new("root".to_string(), 0);
    /// let mut child = TreeNode::new("child".to_string(), 1);
    /// child.children.push(TreeNode::new("grandchild".to_string(), 2));
    /// root.children.push(child);
    ///
    /// root.expand_all(5);
    /// assert!(root.expanded);
    ///
    /// root.collapse_all();
    /// assert!(!root.expanded);
    /// assert!(!root.children[0].expanded);
    /// ```
    pub fn collapse_all(&mut self) {
        self.expanded = false;
        for child in &mut self.children {
            child.collapse_all();
        }
    }

    /// Count the number of visible nodes (this node + expanded children)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeNode;
    ///
    /// let mut root = TreeNode::new("root".to_string(), 0);
    /// root.children.push(TreeNode::new("child1".to_string(), 1));
    /// root.children.push(TreeNode::new("child2".to_string(), 1));
    ///
    /// // Collapsed: only root is visible
    /// assert_eq!(root.visible_count(), 1);
    ///
    /// // Expanded: root + 2 children
    /// root.expand();
    /// assert_eq!(root.visible_count(), 3);
    /// ```
    #[must_use]
    pub fn visible_count(&self) -> usize {
        let mut count = 1; // Count this node

        if self.expanded {
            for child in &self.children {
                count += child.visible_count();
            }
        }

        count
    }

    /// Flatten the tree into a list of (data reference, depth, `is_last_sibling`) tuples
    ///
    /// This is useful for rendering the tree as a flat list while preserving
    /// the hierarchical structure information needed for drawing tree characters.
    ///
    /// Only includes visible nodes (respects expansion state).
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeNode;
    ///
    /// let mut root = TreeNode::new("root".to_string(), 0);
    /// root.children.push(TreeNode::new("child1".to_string(), 1));
    /// root.children.push(TreeNode::new("child2".to_string(), 1));
    /// root.expand();
    ///
    /// let flat = root.flatten();
    /// assert_eq!(flat.len(), 3);
    /// assert_eq!(flat[0].0, &"root".to_string());
    /// assert_eq!(flat[0].1, 0);
    /// assert_eq!(flat[1].0, &"child1".to_string());
    /// assert_eq!(flat[1].2, false); // not last sibling
    /// assert_eq!(flat[2].0, &"child2".to_string());
    /// assert_eq!(flat[2].2, true); // is last sibling
    /// ```
    #[must_use]
    pub fn flatten(&self) -> Vec<(&T, usize, bool)> {
        let mut result = Vec::new();
        self.flatten_recursive(&mut result, true);
        result
    }

    /// Recursive helper for flattening
    fn flatten_recursive<'a>(&'a self, result: &mut Vec<(&'a T, usize, bool)>, is_last: bool) {
        result.push((&self.data, self.depth, is_last));

        if self.expanded {
            let child_count = self.children.len();
            for (i, child) in self.children.iter().enumerate() {
                let is_last_child = i == child_count - 1;
                child.flatten_recursive(result, is_last_child);
            }
        }
    }
}

/// Tree character sets for rendering (Unicode or ASCII)
///
/// Provides the box-drawing characters needed to render tree structures.
/// Unicode characters look better but may not be supported on all terminals.
/// ASCII provides a fallback for limited terminals.
///
/// # Examples
///
/// ```
/// use lash_types::tree::TreeChars;
///
/// let unicode = TreeChars::Unicode;
/// assert_eq!(unicode.branch(), "├── ");
/// assert_eq!(unicode.last_branch(), "└── ");
///
/// let ascii = TreeChars::Ascii;
/// assert_eq!(ascii.branch(), "+-- ");
/// assert_eq!(ascii.last_branch(), "\\-- ");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeChars {
    /// Unicode box-drawing characters (├──, └──, │, ▸, ▾)
    Unicode,
    /// ASCII fallback characters (+, \, |, >, v)
    Ascii,
}

impl TreeChars {
    /// Get the branch character for intermediate children
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeChars;
    ///
    /// assert_eq!(TreeChars::Unicode.branch(), "├── ");
    /// assert_eq!(TreeChars::Ascii.branch(), "+-- ");
    /// ```
    #[must_use]
    pub fn branch(&self) -> &'static str {
        match self {
            Self::Unicode => "├── ",
            Self::Ascii => "+-- ",
        }
    }

    /// Get the branch character for the last child
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeChars;
    ///
    /// assert_eq!(TreeChars::Unicode.last_branch(), "└── ");
    /// assert_eq!(TreeChars::Ascii.last_branch(), "\\-- ");
    /// ```
    #[must_use]
    pub fn last_branch(&self) -> &'static str {
        match self {
            Self::Unicode => "└── ",
            Self::Ascii => "\\-- ",
        }
    }

    /// Get the vertical continuation character
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeChars;
    ///
    /// assert_eq!(TreeChars::Unicode.vertical(), "│   ");
    /// assert_eq!(TreeChars::Ascii.vertical(), "|   ");
    /// ```
    #[must_use]
    pub fn vertical(&self) -> &'static str {
        match self {
            Self::Unicode => "│   ",
            Self::Ascii => "|   ",
        }
    }

    /// Get the empty space character
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeChars;
    ///
    /// assert_eq!(TreeChars::Unicode.empty(), "    ");
    /// assert_eq!(TreeChars::Ascii.empty(), "    ");
    /// ```
    #[must_use]
    pub fn empty(&self) -> &'static str {
        "    "
    }

    /// Get the collapsed indicator (node has hidden children)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeChars;
    ///
    /// assert_eq!(TreeChars::Unicode.collapsed(), "▸ ");
    /// assert_eq!(TreeChars::Ascii.collapsed(), "> ");
    /// ```
    #[must_use]
    pub fn collapsed(&self) -> &'static str {
        match self {
            Self::Unicode => "▸ ",
            Self::Ascii => "> ",
        }
    }

    /// Get the expanded indicator (node has visible children)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeChars;
    ///
    /// assert_eq!(TreeChars::Unicode.expanded(), "▾ ");
    /// assert_eq!(TreeChars::Ascii.expanded(), "v ");
    /// ```
    #[must_use]
    pub fn expanded(&self) -> &'static str {
        match self {
            Self::Unicode => "▾ ",
            Self::Ascii => "v ",
        }
    }

    /// Auto-detect the appropriate character set based on environment
    ///
    /// Checks the LANG environment variable for UTF-8 support.
    /// Falls back to ASCII if UTF-8 is not detected.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::tree::TreeChars;
    ///
    /// let chars = TreeChars::detect();
    /// // Result depends on environment
    /// ```
    #[must_use]
    pub fn detect() -> Self {
        // Check LANG environment variable for UTF-8 support
        if let Ok(lang) = env::var("LANG") {
            if lang.to_lowercase().contains("utf-8") || lang.to_lowercase().contains("utf8") {
                return Self::Unicode;
            }
        }

        // Default to ASCII for safety
        Self::Ascii
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_node_new() {
        let node = TreeNode::new("test", 0);
        assert_eq!(node.data, "test");
        assert_eq!(node.depth, 0);
        assert!(!node.expanded);
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_tree_node_with_children() {
        let children = vec![TreeNode::new("child1", 1), TreeNode::new("child2", 1)];
        let node = TreeNode::with_children("parent", 0, children);
        assert_eq!(node.data, "parent");
        assert_eq!(node.children.len(), 2);
        assert!(!node.expanded);
    }

    #[test]
    fn test_tree_node_has_children() {
        let mut node = TreeNode::new("test", 0);
        assert!(!node.has_children());

        node.children.push(TreeNode::new("child", 1));
        assert!(node.has_children());
    }

    #[test]
    fn test_tree_node_expand_collapse() {
        let mut node = TreeNode::new("test", 0);
        assert!(!node.expanded);

        node.expand();
        assert!(node.expanded);

        node.collapse();
        assert!(!node.expanded);
    }

    #[test]
    fn test_tree_node_toggle() {
        let mut node = TreeNode::new("test", 0);
        assert!(!node.expanded);

        node.toggle();
        assert!(node.expanded);

        node.toggle();
        assert!(!node.expanded);
    }

    #[test]
    fn test_tree_node_expand_all() {
        let mut root = TreeNode::new("root", 0);
        let mut child1 = TreeNode::new("child1", 1);
        let mut child2 = TreeNode::new("child2", 1);
        child1.children.push(TreeNode::new("grandchild1", 2));
        child2.children.push(TreeNode::new("grandchild2", 2));
        root.children.push(child1);
        root.children.push(child2);

        // Expand to depth 2 (root and first level)
        root.expand_all(2);
        assert!(root.expanded);
        assert!(root.children[0].expanded);
        assert!(root.children[1].expanded);
        assert!(!root.children[0].children[0].expanded);
        assert!(!root.children[1].children[0].expanded);

        // Expand to depth 3 (all levels)
        root.collapse_all();
        root.expand_all(3);
        assert!(root.expanded);
        assert!(root.children[0].expanded);
        assert!(root.children[0].children[0].expanded);
    }

    #[test]
    fn test_tree_node_collapse_all() {
        let mut root = TreeNode::new("root", 0);
        let mut child = TreeNode::new("child", 1);
        child.children.push(TreeNode::new("grandchild", 2));
        root.children.push(child);

        root.expand_all(5);
        assert!(root.expanded);
        assert!(root.children[0].expanded);

        root.collapse_all();
        assert!(!root.expanded);
        assert!(!root.children[0].expanded);
    }

    #[test]
    fn test_tree_node_visible_count() {
        let mut root = TreeNode::new("root", 0);
        root.children.push(TreeNode::new("child1", 1));
        root.children.push(TreeNode::new("child2", 1));

        // Collapsed: only root is visible
        assert_eq!(root.visible_count(), 1);

        // Expanded: root + 2 children
        root.expand();
        assert_eq!(root.visible_count(), 3);
    }

    #[test]
    fn test_tree_node_visible_count_nested() {
        let mut root = TreeNode::new("root", 0);
        let mut child1 = TreeNode::new("child1", 1);
        child1.children.push(TreeNode::new("grandchild1", 2));
        child1.children.push(TreeNode::new("grandchild2", 2));
        root.children.push(child1);
        root.children.push(TreeNode::new("child2", 1));

        // Only root visible
        assert_eq!(root.visible_count(), 1);

        // Root + children
        root.expand();
        assert_eq!(root.visible_count(), 3);

        // Root + children + grandchildren
        root.children[0].expand();
        assert_eq!(root.visible_count(), 5);
    }

    #[test]
    fn test_tree_node_flatten() {
        let mut root = TreeNode::new("root", 0);
        root.children.push(TreeNode::new("child1", 1));
        root.children.push(TreeNode::new("child2", 1));

        // Collapsed: only root
        let flat = root.flatten();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].0, &"root");

        // Expanded: root + children
        root.expand();
        let flat = root.flatten();
        assert_eq!(flat.len(), 3);
        assert_eq!(flat[0].0, &"root");
        assert_eq!(flat[0].1, 0); // depth
        assert_eq!(flat[1].0, &"child1");
        assert_eq!(flat[1].1, 1); // depth
        assert!(!flat[1].2); // not last sibling
        assert_eq!(flat[2].0, &"child2");
        assert_eq!(flat[2].1, 1); // depth
        assert!(flat[2].2); // is last sibling
    }

    #[test]
    fn test_tree_node_flatten_nested() {
        let mut root = TreeNode::new("root", 0);
        let mut child1 = TreeNode::new("child1", 1);
        child1.children.push(TreeNode::new("grandchild1", 2));
        child1.children.push(TreeNode::new("grandchild2", 2));
        root.children.push(child1);
        root.children.push(TreeNode::new("child2", 1));

        root.expand();
        root.children[0].expand();

        let flat = root.flatten();
        assert_eq!(flat.len(), 5);
        assert_eq!(flat[0].0, &"root");
        assert_eq!(flat[1].0, &"child1");
        assert!(!flat[1].2); // not last sibling (has child2)
        assert_eq!(flat[2].0, &"grandchild1");
        assert!(!flat[2].2); // not last sibling
        assert_eq!(flat[3].0, &"grandchild2");
        assert!(flat[3].2); // is last sibling
        assert_eq!(flat[4].0, &"child2");
        assert!(flat[4].2); // is last sibling
    }

    #[test]
    fn test_tree_chars_unicode() {
        let chars = TreeChars::Unicode;
        assert_eq!(chars.branch(), "├── ");
        assert_eq!(chars.last_branch(), "└── ");
        assert_eq!(chars.vertical(), "│   ");
        assert_eq!(chars.empty(), "    ");
        assert_eq!(chars.collapsed(), "▸ ");
        assert_eq!(chars.expanded(), "▾ ");
    }

    #[test]
    fn test_tree_chars_ascii() {
        let chars = TreeChars::Ascii;
        assert_eq!(chars.branch(), "+-- ");
        assert_eq!(chars.last_branch(), "\\-- ");
        assert_eq!(chars.vertical(), "|   ");
        assert_eq!(chars.empty(), "    ");
        assert_eq!(chars.collapsed(), "> ");
        assert_eq!(chars.expanded(), "v ");
    }

    #[test]
    fn test_tree_chars_detect() {
        // We can't reliably test auto-detection since it depends on environment
        // but we can verify it returns a valid value
        let chars = TreeChars::detect();
        assert!(matches!(chars, TreeChars::Unicode | TreeChars::Ascii));
    }

    #[test]
    fn test_tree_chars_equality() {
        assert_eq!(TreeChars::Unicode, TreeChars::Unicode);
        assert_eq!(TreeChars::Ascii, TreeChars::Ascii);
        assert_ne!(TreeChars::Unicode, TreeChars::Ascii);
    }
}
