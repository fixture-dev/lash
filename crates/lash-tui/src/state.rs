//! Application state management

#![allow(dead_code)] // Some fields/variants reserved for future features

use crate::colors::Theme;
use lash_db::repository::files::FileRecord;
use lash_db::repository::tasks::TaskRecord;
use lash_types::tree::{TreeChars, TreeNode};
use std::path::PathBuf;

/// Represents a directory or file node in the file tree
#[derive(Debug, Clone)]
pub struct DirectoryNode {
    /// Display name (directory name or file name)
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// `true` for directory, `false` for file
    pub is_directory: bool,
    /// For files, the underlying `FileRecord`
    pub file_record: Option<FileRecord>,
}

/// Which pane is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    /// Navigation pane (left)
    Navigation,
    /// Detail pane (right)
    Detail,
}

/// View mode for navigation pane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavMode {
    /// File tree view
    Files,
    /// Label list view
    Labels,
    /// Search results view
    SearchResults,
}

/// Application state
#[derive(Debug)]
pub struct AppState {
    /// Which pane is focused
    pub focused_pane: FocusedPane,

    /// Navigation pane mode
    pub nav_mode: NavMode,

    /// Currently loaded files
    pub files: Vec<FileRecord>,

    /// Currently selected file (index into files vec)
    pub selected_file_index: usize,

    /// Currently loaded tasks for selected file
    pub tasks: Vec<TaskRecord>,

    /// Currently selected task (index into tasks vec)
    pub selected_task_index: usize,

    /// Navigation pane scroll offset
    pub nav_scroll: usize,

    /// Detail pane scroll offset
    pub detail_scroll: usize,

    /// Whether help overlay is shown
    pub show_help: bool,

    /// Whether to quit
    pub should_quit: bool,

    /// Color theme
    pub theme: Theme,

    /// Theme selector state (None = closed, Some = open with selection index)
    pub theme_selector_state: Option<ThemeSelectorState>,

    /// File tree (hierarchical directory structure)
    pub file_tree: Option<Vec<TreeNode<DirectoryNode>>>,

    /// Task tree (hierarchical task structure)
    pub task_tree: Option<Vec<TreeNode<TaskRecord>>>,

    /// Tree rendering characters (Unicode or ASCII)
    pub tree_chars: TreeChars,
}

/// State for the theme selector modal
#[derive(Debug)]
pub struct ThemeSelectorState {
    /// Index of selected scheme in the list
    pub selected_index: usize,

    /// Scroll offset in the list
    pub scroll_offset: usize,

    /// All available scheme names (cached)
    pub scheme_names: Vec<String>,
}

impl AppState {
    /// Create new application state with default theme
    ///
    /// Loads the theme from user config. If that fails, uses the default `Base2Tone Desert` theme.
    #[must_use]
    pub fn new() -> Self {
        Self::with_theme(Self::load_theme_from_config())
    }

    /// Create new application state with a specific theme
    #[must_use]
    pub fn with_theme(theme: Theme) -> Self {
        Self {
            focused_pane: FocusedPane::Navigation,
            nav_mode: NavMode::Files,
            files: Vec::new(),
            selected_file_index: 0,
            tasks: Vec::new(),
            selected_task_index: 0,
            nav_scroll: 0,
            detail_scroll: 0,
            show_help: false,
            should_quit: false,
            theme,
            theme_selector_state: None,
            file_tree: None,
            task_tree: None,
            tree_chars: TreeChars::detect(),
        }
    }

    /// Create new application state with a specific color scheme by name
    ///
    /// # Errors
    ///
    /// Returns error if the color scheme name is not found. The error message
    /// includes "did you mean?" suggestions if similar schemes exist.
    pub fn with_color_scheme(scheme_name: &str) -> Result<Self, String> {
        use crate::colors::REGISTRY;

        let scheme = REGISTRY.get_scheme(scheme_name);

        if let Some(s) = scheme {
            return Ok(Self::with_theme(Theme::new(s.clone())));
        }

        // Scheme not found - provide helpful suggestions
        let suggestions = REGISTRY.fuzzy_search(scheme_name);

        if suggestions.is_empty() {
            Err(format!("Color scheme not found: '{scheme_name}'"))
        } else if suggestions.len() <= 3 {
            Err(format!(
                "Color scheme not found: '{scheme_name}'\nDid you mean: {}?",
                suggestions.join(", ")
            ))
        } else {
            Err(format!(
                "Color scheme not found: '{scheme_name}'\nDid you mean one of: {}, {}?",
                suggestions[..3].join(", "),
                "..."
            ))
        }
    }

    /// Load theme from user config
    ///
    /// If loading fails, returns the default theme (`Base2Tone Desert`).
    #[must_use]
    fn load_theme_from_config() -> Theme {
        use crate::colors::REGISTRY;
        use lash_types::UserConfig;

        let scheme_name = UserConfig::load().ok().map_or_else(
            || "Base2Tone Desert".to_string(),
            |config| config.color_scheme,
        );

        let scheme = REGISTRY.get_scheme_or_default(&scheme_name);
        Theme::new(scheme.clone())
    }

    /// Switch focus to the other pane
    pub fn switch_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::Navigation => FocusedPane::Detail,
            FocusedPane::Detail => FocusedPane::Navigation,
        };
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        match self.focused_pane {
            FocusedPane::Navigation => {
                if self.selected_file_index > 0 {
                    self.selected_file_index -= 1;
                }
            }
            FocusedPane::Detail => {
                if self.selected_task_index > 0 {
                    self.selected_task_index -= 1;
                }
            }
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        match self.focused_pane {
            FocusedPane::Navigation => {
                if self.selected_file_index + 1 < self.files.len() {
                    self.selected_file_index += 1;
                }
            }
            FocusedPane::Detail => {
                if self.selected_task_index + 1 < self.tasks.len() {
                    self.selected_task_index += 1;
                }
            }
        }
    }

    /// Go to top of current list
    pub fn go_top(&mut self) {
        match self.focused_pane {
            FocusedPane::Navigation => self.selected_file_index = 0,
            FocusedPane::Detail => self.selected_task_index = 0,
        }
    }

    /// Go to bottom of current list
    pub fn go_bottom(&mut self) {
        match self.focused_pane {
            FocusedPane::Navigation => {
                if !self.files.is_empty() {
                    self.selected_file_index = self.files.len() - 1;
                }
            }
            FocusedPane::Detail => {
                if !self.tasks.is_empty() {
                    self.selected_task_index = self.tasks.len() - 1;
                }
            }
        }
    }

    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Open theme selector
    pub fn open_theme_selector(&mut self) {
        use crate::colors::REGISTRY;

        let scheme_names = REGISTRY.scheme_names();
        let current_name = self.theme.name();

        // Find current theme in the list
        let selected_index = scheme_names
            .iter()
            .position(|name| name == current_name)
            .unwrap_or(0);

        self.theme_selector_state = Some(ThemeSelectorState {
            selected_index,
            scroll_offset: selected_index.saturating_sub(5), // Center selection
            scheme_names,
        });
    }

    /// Close theme selector without applying
    pub fn close_theme_selector(&mut self) {
        self.theme_selector_state = None;
    }

    /// Move selection up in theme selector
    pub fn theme_selector_up(&mut self) {
        if let Some(selector) = &mut self.theme_selector_state {
            if selector.selected_index > 0 {
                selector.selected_index -= 1;
            }
        }
    }

    /// Move selection down in theme selector
    pub fn theme_selector_down(&mut self) {
        if let Some(selector) = &mut self.theme_selector_state {
            if selector.selected_index + 1 < selector.scheme_names.len() {
                selector.selected_index += 1;
            }
        }
    }

    /// Apply selected theme and close selector
    ///
    /// Saves the theme to user config and updates the current theme.
    pub fn apply_selected_theme(&mut self) -> Result<(), String> {
        use crate::colors::REGISTRY;
        use lash_types::UserConfig;

        if let Some(selector) = &self.theme_selector_state {
            let scheme_name = &selector.scheme_names[selector.selected_index];
            let scheme = REGISTRY
                .get_scheme(scheme_name)
                .ok_or_else(|| format!("Scheme not found: {scheme_name}"))?;

            // Update theme
            self.theme = Theme::new(scheme.clone());

            // Save to user config
            let mut user_config = UserConfig::load().unwrap_or_default();
            user_config.color_scheme.clone_from(scheme_name);
            user_config
                .save()
                .map_err(|e| format!("Failed to save theme: {e}"))?;

            // Close selector
            self.theme_selector_state = None;

            Ok(())
        } else {
            Err("Theme selector not open".to_string())
        }
    }

    /// Get currently selected file
    #[must_use]
    pub fn selected_file(&self) -> Option<&FileRecord> {
        self.files.get(self.selected_file_index)
    }

    /// Get currently selected task
    #[must_use]
    pub fn selected_task(&self) -> Option<&TaskRecord> {
        self.tasks.get(self.selected_task_index)
    }

    /// Build file tree from flat file list
    ///
    /// Converts the flat `self.files` list into a hierarchical directory tree.
    /// Groups files by directory path components and creates intermediate directory nodes.
    pub fn build_file_tree(&mut self) {
        use lash_types::UserConfig;
        use std::collections::HashMap;

        if self.files.is_empty() {
            self.file_tree = None;
            return;
        }

        // Load config for default_expanded setting
        let config = UserConfig::load().unwrap_or_default();
        let default_expanded = config.tree_view.default_expanded;
        let max_depth = config.tree_view.max_depth;

        // Group files by directory
        let mut dir_map: HashMap<PathBuf, Vec<FileRecord>> = HashMap::new();
        for file in &self.files {
            let dir = file
                .path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(""));
            dir_map
                .entry(dir.to_path_buf())
                .or_default()
                .push(file.clone());
        }

        // Build tree structure
        let mut roots: Vec<TreeNode<DirectoryNode>> = Vec::new();
        let mut dir_nodes: HashMap<PathBuf, TreeNode<DirectoryNode>> = HashMap::new();

        // Sort directories by path for consistent ordering
        let mut sorted_dirs: Vec<_> = dir_map.keys().collect();
        sorted_dirs.sort();

        for dir_path in sorted_dirs {
            let files = dir_map.get(dir_path).unwrap();
            let depth = dir_path.components().count();

            // Create directory node if it doesn't exist
            if !dir_nodes.contains_key(dir_path) && depth > 0 {
                let dir_name = dir_path
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new(""))
                    .to_string_lossy()
                    .to_string();

                let mut dir_node = TreeNode::new(
                    DirectoryNode {
                        name: dir_name,
                        path: dir_path.clone(),
                        is_directory: true,
                        file_record: None,
                    },
                    depth,
                );

                if default_expanded && depth < max_depth {
                    dir_node.expand();
                }

                dir_nodes.insert(dir_path.clone(), dir_node);
            }

            // Add file nodes to directory
            for file in files {
                let file_name = file
                    .path
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new(""))
                    .to_string_lossy()
                    .to_string();

                let file_node = TreeNode::new(
                    DirectoryNode {
                        name: file_name,
                        path: file.path.clone(),
                        is_directory: false,
                        file_record: Some(file.clone()),
                    },
                    depth + 1,
                );

                // Add to parent directory or root
                if depth == 0 {
                    roots.push(file_node);
                } else if let Some(parent) = dir_nodes.get_mut(dir_path) {
                    parent.children.push(file_node);
                }
            }
        }

        // Build parent-child relationships for directories
        let dir_paths: Vec<PathBuf> = dir_nodes.keys().cloned().collect();
        for dir_path in &dir_paths {
            if let Some(parent_path) = dir_path.parent() {
                if parent_path.as_os_str().is_empty() {
                    // This is a root directory
                    if let Some(node) = dir_nodes.remove(dir_path) {
                        roots.push(node);
                    }
                } else if dir_nodes.contains_key(parent_path) {
                    // This directory has a parent in the tree
                    if let Some(node) = dir_nodes.remove(dir_path) {
                        if let Some(parent) = dir_nodes.get_mut(parent_path) {
                            parent.children.push(node);
                        }
                    }
                } else {
                    // Parent doesn't exist, add as root
                    if let Some(node) = dir_nodes.remove(dir_path) {
                        roots.push(node);
                    }
                }
            }
        }

        self.file_tree = Some(roots);
    }

    /// Build task tree from flat task list
    ///
    /// Converts the flat `self.tasks` list into a hierarchical task tree.
    /// Uses `parent_id` and `depth` fields from `TaskRecord` to build the hierarchy.
    pub fn build_task_tree(&mut self) {
        use lash_types::UserConfig;

        /// Recursively build tree from root nodes (`parent_id = None`)
        fn build_subtree(
            parent_id: Option<i64>,
            children_map: &std::collections::HashMap<Option<i64>, Vec<TaskRecord>>,
            default_expanded: bool,
            max_depth: usize,
        ) -> Vec<TreeNode<TaskRecord>> {
            let Some(children) = children_map.get(&parent_id) else {
                return Vec::new();
            };

            children
                .iter()
                .map(|task| {
                    let depth = task.depth as usize;
                    let mut node = TreeNode::new(task.clone(), depth);

                    // Recursively add children
                    node.children =
                        build_subtree(Some(task.id), children_map, default_expanded, max_depth);

                    // Expand if configured and within depth limit
                    if default_expanded && depth < max_depth && !node.children.is_empty() {
                        node.expand();
                    }

                    node
                })
                .collect()
        }

        if self.tasks.is_empty() {
            self.task_tree = None;
            return;
        }

        // Load config for default_expanded setting
        let config = UserConfig::load().unwrap_or_default();
        let default_expanded = config.tree_view.default_expanded;
        let max_depth = config.tree_view.max_depth;

        // Group tasks by parent_id
        let mut children_map: std::collections::HashMap<Option<i64>, Vec<TaskRecord>> =
            std::collections::HashMap::new();

        for task in &self.tasks {
            children_map
                .entry(task.parent_id)
                .or_default()
                .push(task.clone());
        }

        // Sort children by order_index within each parent group
        for children in children_map.values_mut() {
            children.sort_by_key(|t| t.order_index);
        }

        let roots = build_subtree(None, &children_map, default_expanded, max_depth);
        self.task_tree = Some(roots);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
