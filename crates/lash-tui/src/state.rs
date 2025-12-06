//! Application state management

#![allow(dead_code)] // Some fields/variants reserved for future features

use crate::colors::Theme;
use lash_db::repository::dependencies::DependencyRecord;
use lash_db::repository::files::FileRecord;
use lash_db::repository::labels::LabelStats;
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

/// Ensure all ancestor directories exist in the tree as nodes.
///
/// For example, if the path is `worlds/forest/levels`, this creates
/// nodes for `worlds` and `worlds/forest` if they don't already exist.
fn ensure_ancestors(
    dir_nodes: &mut std::collections::HashMap<PathBuf, TreeNode<DirectoryNode>>,
    path: &std::path::Path,
    default_expanded: bool,
    max_depth: usize,
) {
    // Collect all ancestor paths that need to be created
    let mut ancestors: Vec<PathBuf> = Vec::new();
    let mut current = path.to_path_buf();
    while let Some(parent) = current.parent() {
        if parent.as_os_str().is_empty() {
            break;
        }
        if !dir_nodes.contains_key(parent) {
            ancestors.push(parent.to_path_buf());
        }
        current = parent.to_path_buf();
    }

    // Create ancestor nodes from root to leaf (reverse order)
    for ancestor in ancestors.into_iter().rev() {
        let depth = ancestor.components().count();
        let dir_name = ancestor
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new(""))
            .to_string_lossy()
            .to_string();

        let mut dir_node = TreeNode::new(
            DirectoryNode {
                name: dir_name,
                path: ancestor.clone(),
                is_directory: true,
                file_record: None,
            },
            depth,
        );

        if default_expanded && depth < max_depth {
            dir_node.expand();
        }

        dir_nodes.insert(ancestor, dir_node);
    }
}

/// Which pane is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    /// Navigation pane (left)
    Navigation,
    /// Description pane (top-right)
    Description,
    /// Detail/Tasks pane (bottom-right)
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

    /// Description pane scroll offset
    pub description_scroll: usize,

    /// Description content height (in lines, for scroll bounds)
    pub description_content_height: usize,

    /// Whether help overlay is shown
    pub show_help: bool,

    /// Whether to quit
    pub should_quit: bool,

    /// Color theme
    pub theme: Theme,

    /// Theme selector state (None = closed, Some = open with selection index)
    pub theme_selector_state: Option<ThemeSelectorState>,

    /// Task detail state (None = closed, Some = open with task details)
    pub task_detail_state: Option<TaskDetailState>,

    /// File tree (hierarchical directory structure)
    pub file_tree: Option<Vec<TreeNode<DirectoryNode>>>,

    /// Task tree (hierarchical task structure)
    pub task_tree: Option<Vec<TreeNode<TaskRecord>>>,

    /// Tree rendering characters (Unicode or ASCII)
    pub tree_chars: TreeChars,

    /// Available labels with task counts (for label view)
    pub labels: Vec<LabelStats>,

    /// Currently selected label (index into labels vec)
    pub selected_label_index: usize,

    /// Current label filter (if any)
    pub current_label_filter: Option<String>,

    /// Search modal state (None = closed, Some = open)
    pub search_modal_state: Option<SearchModalState>,

    /// Filter modal state (None = closed, Some = open)
    pub filter_modal_state: Option<FilterModalState>,

    /// Project-level statistics (total tasks, completion, title)
    pub project_stats: ProjectStats,

    /// Status message to display (transient feedback)
    pub status_message: Option<StatusMessage>,
}

/// A transient status message displayed in the UI
#[derive(Debug, Clone)]
pub struct StatusMessage {
    /// The message text
    pub text: String,

    /// Message severity/type for styling
    pub level: StatusLevel,

    /// When the message expires (instant)
    pub expires_at: std::time::Instant,
}

/// Severity level for status messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    /// Informational message
    Info,
    /// Warning message
    Warning,
    /// Error message
    Error,
    /// Success message
    Success,
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

/// State for the task detail modal
#[derive(Debug)]
pub struct TaskDetailState {
    /// The task being displayed
    pub task: TaskRecord,

    /// File path containing this task
    pub file_path: PathBuf,

    /// Scroll offset for content
    pub scroll_offset: usize,

    /// Task labels
    pub labels: Vec<String>,

    /// Dependencies (tasks this task depends on)
    pub dependencies: Vec<DependencyRecord>,

    /// Direct child tasks (subtasks)
    pub subtasks: Vec<TaskRecord>,

    /// Total content height for scroll bounds
    pub content_height: usize,
}

/// State for the search modal
#[derive(Debug)]
pub struct SearchModalState {
    /// Current search input text
    pub input: String,

    /// Cursor position in the input
    pub cursor_position: usize,

    /// Search results from the database
    pub results: Vec<lash_db::search::SearchResult>,

    /// Index of the currently selected result
    pub selected_result_index: usize,

    /// Total number of results (before pagination)
    pub total_count: usize,

    /// Whether a search has been executed
    pub has_searched: bool,

    /// Error message if search failed
    pub error: Option<String>,
}

/// Project-level statistics
#[derive(Debug, Clone, Default)]
pub struct ProjectStats {
    /// Project title from root index file
    pub title: Option<String>,

    /// Total number of tasks across all files
    pub total_tasks: usize,

    /// Number of completed tasks (done or waived)
    pub completed_tasks: usize,
}

impl ProjectStats {
    /// Calculate completion percentage (0-100)
    #[must_use]
    pub fn completion_percent(&self) -> u8 {
        if self.total_tasks == 0 {
            0
        } else {
            #[allow(clippy::cast_possible_truncation)]
            let percent = (self.completed_tasks * 100 / self.total_tasks) as u8;
            percent.min(100)
        }
    }
}

/// State for the filter modal
#[derive(Debug)]
pub struct FilterModalState {
    /// Available labels to filter by (with task counts)
    pub available_labels: Vec<LabelStats>,

    /// Currently selected label index in the list
    pub selected_index: usize,

    /// Scroll offset for the label list
    pub scroll_offset: usize,

    /// Current text input for filtering the label list
    pub input: String,

    /// Cursor position in input
    pub cursor_position: usize,

    /// Filtered labels (indices into `available_labels`)
    pub filtered_indices: Vec<usize>,
}

/// Information about a selected tree node
#[derive(Debug, Clone)]
pub struct SelectedTreeNode {
    /// Whether this is a directory node
    pub is_directory: bool,

    /// Whether the node is currently expanded
    pub is_expanded: bool,

    /// Whether the node has children
    pub has_children: bool,

    /// The file record if this is a file node
    pub file_record: Option<FileRecord>,

    /// Path to this node in the tree (indices at each level)
    pub path: Vec<usize>,
}

/// Information about a selected task tree node
#[derive(Debug, Clone)]
pub struct SelectedTaskNode {
    /// Whether the node is currently expanded
    pub is_expanded: bool,

    /// Whether the node has children (subtasks)
    pub has_children: bool,

    /// Path to this node in the tree (indices at each level)
    pub path: Vec<usize>,
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
            description_scroll: 0,
            description_content_height: 0,
            show_help: false,
            should_quit: false,
            theme,
            theme_selector_state: None,
            task_detail_state: None,
            file_tree: None,
            task_tree: None,
            tree_chars: TreeChars::detect(),
            labels: Vec::new(),
            selected_label_index: 0,
            current_label_filter: None,
            search_modal_state: None,
            filter_modal_state: None,
            project_stats: ProjectStats::default(),
            status_message: None,
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

    /// Switch focus to the next pane (Navigation → Description → Detail → Navigation)
    pub fn switch_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::Navigation => FocusedPane::Description,
            FocusedPane::Description => FocusedPane::Detail,
            FocusedPane::Detail => FocusedPane::Navigation,
        };
    }

    /// Move selection up (or scroll up for Description pane)
    pub fn move_up(&mut self) {
        match self.focused_pane {
            FocusedPane::Navigation => match self.nav_mode {
                NavMode::Files | NavMode::SearchResults => {
                    if self.selected_file_index > 0 {
                        self.selected_file_index -= 1;
                    }
                }
                NavMode::Labels => {
                    if self.selected_label_index > 0 {
                        self.selected_label_index -= 1;
                    }
                }
            },
            FocusedPane::Description => {
                self.description_scroll_up();
            }
            FocusedPane::Detail => {
                if self.selected_task_index > 0 {
                    self.selected_task_index -= 1;
                }
            }
        }
    }

    /// Move selection down (or scroll down for Description pane)
    pub fn move_down(&mut self, description_visible_height: usize) {
        match self.focused_pane {
            FocusedPane::Navigation => match self.nav_mode {
                NavMode::Files | NavMode::SearchResults => {
                    let max_index = self.visible_tree_node_count();
                    if self.selected_file_index + 1 < max_index {
                        self.selected_file_index += 1;
                    }
                }
                NavMode::Labels => {
                    if self.selected_label_index + 1 < self.labels.len() {
                        self.selected_label_index += 1;
                    }
                }
            },
            FocusedPane::Description => {
                self.description_scroll_down(description_visible_height);
            }
            FocusedPane::Detail => {
                if self.selected_task_index + 1 < self.tasks.len() {
                    self.selected_task_index += 1;
                }
            }
        }
    }

    /// Scroll description pane up
    pub fn description_scroll_up(&mut self) {
        if self.description_scroll > 0 {
            self.description_scroll -= 1;
        }
    }

    /// Scroll description pane down
    pub fn description_scroll_down(&mut self, visible_height: usize) {
        let max_scroll = self
            .description_content_height
            .saturating_sub(visible_height);
        if self.description_scroll < max_scroll {
            self.description_scroll += 1;
        }
    }

    /// Reset description scroll when changing files
    pub fn reset_description_scroll(&mut self) {
        self.description_scroll = 0;
    }

    /// Go to top of current list (or scroll to top for Description pane)
    pub fn go_top(&mut self) {
        match self.focused_pane {
            FocusedPane::Navigation => match self.nav_mode {
                NavMode::Files | NavMode::SearchResults => self.selected_file_index = 0,
                NavMode::Labels => self.selected_label_index = 0,
            },
            FocusedPane::Description => self.description_scroll = 0,
            FocusedPane::Detail => self.selected_task_index = 0,
        }
    }

    /// Go to bottom of current list (or scroll to bottom for Description pane)
    pub fn go_bottom(&mut self, description_visible_height: usize) {
        match self.focused_pane {
            FocusedPane::Navigation => match self.nav_mode {
                NavMode::Files | NavMode::SearchResults => {
                    let max_index = self.visible_tree_node_count();
                    if max_index > 0 {
                        self.selected_file_index = max_index - 1;
                    }
                }
                NavMode::Labels => {
                    if !self.labels.is_empty() {
                        self.selected_label_index = self.labels.len() - 1;
                    }
                }
            },
            FocusedPane::Description => {
                let max_scroll = self
                    .description_content_height
                    .saturating_sub(description_visible_height);
                self.description_scroll = max_scroll;
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

    /// Set a status message with the given level and duration
    ///
    /// The message will automatically expire after the specified duration.
    pub fn set_status_message(
        &mut self,
        text: impl Into<String>,
        level: StatusLevel,
        duration_ms: u64,
    ) {
        self.status_message = Some(StatusMessage {
            text: text.into(),
            level,
            expires_at: std::time::Instant::now() + std::time::Duration::from_millis(duration_ms),
        });
    }

    /// Set an info status message (default 3 second duration)
    pub fn set_info_message(&mut self, text: impl Into<String>) {
        self.set_status_message(text, StatusLevel::Info, 3000);
    }

    /// Set a warning status message (default 4 second duration)
    pub fn set_warning_message(&mut self, text: impl Into<String>) {
        self.set_status_message(text, StatusLevel::Warning, 4000);
    }

    /// Set an error status message (default 5 second duration)
    pub fn set_error_message(&mut self, text: impl Into<String>) {
        self.set_status_message(text, StatusLevel::Error, 5000);
    }

    /// Set a success status message (default 3 second duration)
    pub fn set_success_message(&mut self, text: impl Into<String>) {
        self.set_status_message(text, StatusLevel::Success, 3000);
    }

    /// Clear the status message
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    /// Check if status message has expired and clear it if so
    pub fn check_status_expiry(&mut self) {
        if let Some(msg) = &self.status_message {
            if std::time::Instant::now() >= msg.expires_at {
                self.status_message = None;
            }
        }
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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The selected scheme is not found in the registry
    /// - Failed to save the theme to user config
    /// - Theme selector is not open
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

    /// Open task detail view
    ///
    /// Opens a modal overlay displaying comprehensive details for a task.
    pub fn open_task_detail(
        &mut self,
        task: TaskRecord,
        file_path: PathBuf,
        labels: Vec<String>,
        dependencies: Vec<DependencyRecord>,
        subtasks: Vec<TaskRecord>,
    ) {
        // Calculate content height based on sections that will be rendered
        // Header: 2 lines (ID/Status + blank)
        let mut content_height = 2;

        // Metadata: header + file + optional owner + optional estimate + blank
        content_height += 3; // header + file + blank
        if task.owner.is_some() {
            content_height += 1;
        }
        if task.estimate.is_some() {
            content_height += 1;
        }

        // Labels: header + labels line + blank (if not empty)
        if !labels.is_empty() {
            content_height += 3;
        }

        // Description: header + body lines + blank (if present)
        if let Some(body) = &task.body {
            content_height += 2 + body.lines().count();
        }

        // Subtasks: header + count + blank (if not empty)
        if !subtasks.is_empty() {
            content_height += 2 + subtasks.len();
        }

        // Dependencies: header + count + blank (if not empty)
        if !dependencies.is_empty() {
            content_height += 2 + dependencies.len();
        }

        // Footer: blank + instructions
        content_height += 2;

        self.task_detail_state = Some(TaskDetailState {
            task,
            file_path,
            scroll_offset: 0,
            labels,
            dependencies,
            subtasks,
            content_height,
        });
    }

    /// Close task detail view
    pub fn close_task_detail(&mut self) {
        self.task_detail_state = None;
    }

    /// Check if task detail view is open
    #[must_use]
    pub fn is_task_detail_open(&self) -> bool {
        self.task_detail_state.is_some()
    }

    /// Scroll up in task detail view
    pub fn task_detail_scroll_up(&mut self) {
        if let Some(detail) = &mut self.task_detail_state {
            if detail.scroll_offset > 0 {
                detail.scroll_offset -= 1;
            }
        }
    }

    /// Scroll down in task detail view
    ///
    /// # Arguments
    ///
    /// * `visible_height` - The height of the visible area in lines
    pub fn task_detail_scroll_down(&mut self, visible_height: usize) {
        if let Some(detail) = &mut self.task_detail_state {
            let max_scroll = detail.content_height.saturating_sub(visible_height);
            if detail.scroll_offset < max_scroll {
                detail.scroll_offset += 1;
            }
        }
    }

    /// Get currently selected file
    #[must_use]
    pub fn selected_file(&self) -> Option<&FileRecord> {
        self.files.get(self.selected_file_index)
    }

    /// Get the title of the currently selected file in the navigation pane.
    ///
    /// When in tree view mode, this returns the title from the selected tree node
    /// (which correctly maps to the visual selection). Falls back to the flat
    /// file list if tree view is not available.
    #[must_use]
    pub fn selected_file_title(&self) -> Option<String> {
        // In tree view mode, get title from the selected tree node
        if let Some(node) = self.selected_tree_node() {
            if let Some(file) = node.file_record {
                return Some(file.title);
            }
            // Directory nodes don't have a meaningful title for detail pane
            return None;
        }

        // Fall back to flat file list
        self.files
            .get(self.selected_file_index)
            .map(|f| f.title.clone())
    }

    /// Get the selected node from the file tree view
    ///
    /// Returns the node at the current `selected_file_index` position in the
    /// flattened visible tree. This accounts for expand/collapse state.
    #[must_use]
    pub fn selected_tree_node(&self) -> Option<SelectedTreeNode> {
        let trees = self.file_tree.as_ref()?;

        let mut current_index = 0;
        for (root_idx, tree) in trees.iter().enumerate() {
            if let Some(result) = Self::find_node_at_index(
                tree,
                self.selected_file_index,
                &mut current_index,
                &[root_idx],
            ) {
                return Some(result);
            }
        }
        None
    }

    /// Recursively find the node at a given visual index
    fn find_node_at_index(
        node: &TreeNode<DirectoryNode>,
        target_index: usize,
        current_index: &mut usize,
        path: &[usize],
    ) -> Option<SelectedTreeNode> {
        if *current_index == target_index {
            return Some(SelectedTreeNode {
                is_directory: node.data.is_directory,
                is_expanded: node.expanded,
                has_children: node.has_children(),
                file_record: node.data.file_record.clone(),
                path: path.to_vec(),
            });
        }

        *current_index += 1;

        if node.expanded {
            for (child_idx, child) in node.children.iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(child_idx);
                if let Some(result) =
                    Self::find_node_at_index(child, target_index, current_index, &child_path)
                {
                    return Some(result);
                }
            }
        }

        None
    }

    /// Toggle expand/collapse on the currently selected tree node
    ///
    /// Returns `true` if a node was toggled, `false` if nothing was done.
    pub fn toggle_selected_node(&mut self) -> bool {
        let Some(selected) = self.selected_tree_node() else {
            return false;
        };

        if !selected.is_directory || !selected.has_children {
            return false;
        }

        let Some(trees) = &mut self.file_tree else {
            return false;
        };

        // Navigate to the node using the path
        if selected.path.is_empty() {
            return false;
        }

        let root_idx = selected.path[0];
        if root_idx >= trees.len() {
            return false;
        }

        let mut node = &mut trees[root_idx];

        for &child_idx in &selected.path[1..] {
            if child_idx >= node.children.len() {
                return false;
            }
            node = &mut node.children[child_idx];
        }

        // Toggle
        if node.expanded {
            node.collapse();
        } else {
            node.expand();
        }

        true
    }

    /// Count total visible nodes in file tree
    ///
    /// Used for bounds checking in `move_up`/`move_down` when tree view is active.
    #[must_use]
    pub fn visible_tree_node_count(&self) -> usize {
        let Some(trees) = &self.file_tree else {
            return self.files.len();
        };

        trees.iter().map(Self::count_visible_nodes).sum()
    }

    /// Recursively count visible nodes in a tree
    fn count_visible_nodes(node: &TreeNode<DirectoryNode>) -> usize {
        let mut count = 1; // This node
        if node.expanded {
            for child in &node.children {
                count += Self::count_visible_nodes(child);
            }
        }
        count
    }

    /// Get currently selected task
    ///
    /// When tree view is active, this returns the task at the visual position
    /// in the flattened tree. Otherwise, it returns from the flat task list.
    #[must_use]
    pub fn selected_task(&self) -> Option<&TaskRecord> {
        // When tree view is active, we need to get the task from the flattened tree
        // because the visual index corresponds to the flattened tree position,
        // not the flat tasks vector
        if let Some(task_trees) = &self.task_tree {
            // Flatten all trees to get tasks in visual order
            let mut flat_tasks: Vec<&TaskRecord> = Vec::new();
            for tree in task_trees {
                Self::flatten_task_tree(tree, &mut flat_tasks);
            }
            flat_tasks.get(self.selected_task_index).copied()
        } else {
            self.tasks.get(self.selected_task_index)
        }
    }

    /// Recursively flatten a task tree into a vector of task references
    ///
    /// Only includes visible nodes (respects expansion state).
    fn flatten_task_tree<'a>(node: &'a TreeNode<TaskRecord>, result: &mut Vec<&'a TaskRecord>) {
        result.push(&node.data);

        if node.expanded {
            for child in &node.children {
                Self::flatten_task_tree(child, result);
            }
        }
    }

    /// Get information about the selected task tree node
    ///
    /// Returns details about the task at the current `selected_task_index` position
    /// in the task tree, including whether it has children and is expanded.
    #[must_use]
    pub fn selected_task_tree_node(&self) -> Option<SelectedTaskNode> {
        let trees = self.task_tree.as_ref()?;

        let mut current_index = 0;
        for (root_idx, tree) in trees.iter().enumerate() {
            if let Some(result) = Self::find_task_node_at_index(
                tree,
                self.selected_task_index,
                &mut current_index,
                &[root_idx],
            ) {
                return Some(result);
            }
        }
        None
    }

    /// Recursively find the task node at a given visual index
    fn find_task_node_at_index(
        node: &TreeNode<TaskRecord>,
        target_index: usize,
        current_index: &mut usize,
        path: &[usize],
    ) -> Option<SelectedTaskNode> {
        if *current_index == target_index {
            return Some(SelectedTaskNode {
                is_expanded: node.expanded,
                has_children: node.has_children(),
                path: path.to_vec(),
            });
        }

        *current_index += 1;

        if node.expanded {
            for (child_idx, child) in node.children.iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(child_idx);
                if let Some(result) =
                    Self::find_task_node_at_index(child, target_index, current_index, &child_path)
                {
                    return Some(result);
                }
            }
        }

        None
    }

    /// Toggle expand/collapse on the currently selected task tree node
    ///
    /// Returns `true` if a node was toggled, `false` if nothing was done.
    pub fn toggle_selected_task_node(&mut self) -> bool {
        let Some(selected) = self.selected_task_tree_node() else {
            return false;
        };

        if !selected.has_children {
            return false;
        }

        let Some(trees) = &mut self.task_tree else {
            return false;
        };

        // Navigate to the node using the path
        if selected.path.is_empty() {
            return false;
        }

        let root_idx = selected.path[0];
        if root_idx >= trees.len() {
            return false;
        }

        let mut node = &mut trees[root_idx];

        for &child_idx in &selected.path[1..] {
            if child_idx >= node.children.len() {
                return false;
            }
            node = &mut node.children[child_idx];
        }

        // Toggle
        node.toggle();

        true
    }

    /// Get currently selected label
    #[must_use]
    pub fn selected_label(&self) -> Option<&LabelStats> {
        self.labels.get(self.selected_label_index)
    }

    /// Switch to labels view mode
    pub fn switch_to_labels(&mut self) {
        self.nav_mode = NavMode::Labels;
        self.selected_label_index = 0;
    }

    /// Switch to files view mode
    pub fn switch_to_files(&mut self) {
        self.nav_mode = NavMode::Files;
        self.current_label_filter = None;
    }

    /// Toggle between files and labels view
    pub fn toggle_nav_mode(&mut self) {
        match self.nav_mode {
            NavMode::Files => self.switch_to_labels(),
            NavMode::Labels | NavMode::SearchResults => self.switch_to_files(),
        }
    }

    /// Clear label filter and return to file view
    pub fn clear_label_filter(&mut self) {
        self.current_label_filter = None;
        self.nav_mode = NavMode::Files;
    }

    /// Open search modal
    pub fn open_search_modal(&mut self) {
        self.search_modal_state = Some(SearchModalState {
            input: String::new(),
            cursor_position: 0,
            results: Vec::new(),
            selected_result_index: 0,
            total_count: 0,
            has_searched: false,
            error: None,
        });
    }

    /// Close search modal without navigating
    pub fn close_search_modal(&mut self) {
        self.search_modal_state = None;
    }

    /// Check if search modal is open
    #[must_use]
    pub fn is_search_modal_open(&self) -> bool {
        self.search_modal_state.is_some()
    }

    /// Handle text input for search modal
    ///
    /// Returns `true` if the input was handled.
    pub fn search_modal_input(&mut self, ch: char) {
        if let Some(modal) = &mut self.search_modal_state {
            modal.input.insert(modal.cursor_position, ch);
            modal.cursor_position += 1;
        }
    }

    /// Handle backspace in search modal
    pub fn search_modal_backspace(&mut self) {
        if let Some(modal) = &mut self.search_modal_state {
            if modal.cursor_position > 0 {
                modal.cursor_position -= 1;
                modal.input.remove(modal.cursor_position);
            }
        }
    }

    /// Handle delete key in search modal
    pub fn search_modal_delete(&mut self) {
        if let Some(modal) = &mut self.search_modal_state {
            if modal.cursor_position < modal.input.len() {
                modal.input.remove(modal.cursor_position);
            }
        }
    }

    /// Move cursor left in search modal
    pub fn search_modal_cursor_left(&mut self) {
        if let Some(modal) = &mut self.search_modal_state {
            if modal.cursor_position > 0 {
                modal.cursor_position -= 1;
            }
        }
    }

    /// Move cursor right in search modal
    pub fn search_modal_cursor_right(&mut self) {
        if let Some(modal) = &mut self.search_modal_state {
            if modal.cursor_position < modal.input.len() {
                modal.cursor_position += 1;
            }
        }
    }

    /// Move cursor to start of input
    pub fn search_modal_cursor_home(&mut self) {
        if let Some(modal) = &mut self.search_modal_state {
            modal.cursor_position = 0;
        }
    }

    /// Move cursor to end of input
    pub fn search_modal_cursor_end(&mut self) {
        if let Some(modal) = &mut self.search_modal_state {
            modal.cursor_position = modal.input.len();
        }
    }

    /// Clear search input
    pub fn search_modal_clear(&mut self) {
        if let Some(modal) = &mut self.search_modal_state {
            modal.input.clear();
            modal.cursor_position = 0;
            modal.results.clear();
            modal.total_count = 0;
            modal.has_searched = false;
            modal.error = None;
        }
    }

    /// Move selection up in search results
    pub fn search_modal_up(&mut self) {
        if let Some(modal) = &mut self.search_modal_state {
            if modal.selected_result_index > 0 {
                modal.selected_result_index -= 1;
            }
        }
    }

    /// Move selection down in search results
    pub fn search_modal_down(&mut self) {
        if let Some(modal) = &mut self.search_modal_state {
            if modal.selected_result_index + 1 < modal.results.len() {
                modal.selected_result_index += 1;
            }
        }
    }

    /// Update search results
    pub fn search_modal_set_results(
        &mut self,
        results: Vec<lash_db::search::SearchResult>,
        total_count: usize,
    ) {
        if let Some(modal) = &mut self.search_modal_state {
            modal.results = results;
            modal.total_count = total_count;
            modal.selected_result_index = 0;
            modal.has_searched = true;
            modal.error = None;
        }
    }

    /// Set search error
    pub fn search_modal_set_error(&mut self, error: String) {
        if let Some(modal) = &mut self.search_modal_state {
            modal.error = Some(error);
            modal.has_searched = true;
        }
    }

    /// Get currently selected search result
    #[must_use]
    pub fn selected_search_result(&self) -> Option<&lash_db::search::SearchResult> {
        self.search_modal_state
            .as_ref()
            .and_then(|modal| modal.results.get(modal.selected_result_index))
    }

    /// Get the search input query
    #[must_use]
    pub fn search_query(&self) -> Option<&str> {
        self.search_modal_state
            .as_ref()
            .map(|modal| modal.input.as_str())
    }

    /// Build file tree from flat file list
    ///
    /// Converts the flat `self.files` list into a hierarchical directory tree.
    /// Groups files by directory path components and creates intermediate directory nodes.
    ///
    /// # Panics
    ///
    /// This function does not panic under normal operation. The internal `unwrap()`
    /// is safe because we iterate over keys that are guaranteed to exist in the map.
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

            // Ensure all ancestor directories exist (e.g., "worlds" for "worlds/forest")
            ensure_ancestors(&mut dir_nodes, dir_path, default_expanded, max_depth);

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
        // Sort from deepest to shallowest so children are processed before parents
        let mut dir_paths: Vec<PathBuf> = dir_nodes.keys().cloned().collect();
        dir_paths.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
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

    /// Open filter modal
    pub fn open_filter_modal(&mut self, labels: Vec<LabelStats>) {
        // Create filtered indices (initially all labels)
        let filtered_indices: Vec<usize> = (0..labels.len()).collect();

        // Find current filter in the list
        let selected_index = if let Some(current_filter) = &self.current_label_filter {
            labels
                .iter()
                .position(|l| &l.name == current_filter)
                .unwrap_or(0)
        } else {
            0
        };

        self.filter_modal_state = Some(FilterModalState {
            available_labels: labels,
            selected_index,
            scroll_offset: selected_index.saturating_sub(5), // Center selection
            input: String::new(),
            cursor_position: 0,
            filtered_indices,
        });
    }

    /// Close filter modal without applying
    pub fn close_filter_modal(&mut self) {
        self.filter_modal_state = None;
    }

    /// Check if filter modal is open
    #[must_use]
    pub fn is_filter_modal_open(&self) -> bool {
        self.filter_modal_state.is_some()
    }

    /// Move selection up in filter modal
    pub fn filter_modal_up(&mut self) {
        if let Some(modal) = &mut self.filter_modal_state {
            if modal.selected_index > 0 {
                modal.selected_index -= 1;
            }
        }
    }

    /// Move selection down in filter modal
    pub fn filter_modal_down(&mut self) {
        if let Some(modal) = &mut self.filter_modal_state {
            let max_index = modal.filtered_indices.len();
            if modal.selected_index + 1 < max_index {
                modal.selected_index += 1;
            }
        }
    }

    /// Update filter modal input and recompute filtered labels
    pub fn filter_modal_update_input(&mut self, input: &str) {
        if let Some(modal) = &mut self.filter_modal_state {
            modal.input.clone_from(&input.to_string());

            // Filter labels based on input (case-insensitive substring match)
            let input_lower = input.to_lowercase();
            modal.filtered_indices = modal
                .available_labels
                .iter()
                .enumerate()
                .filter(|(_, label)| label.name.to_lowercase().contains(&input_lower))
                .map(|(idx, _)| idx)
                .collect();

            // Reset selection if current selection is not in filtered list
            if modal.selected_index >= modal.filtered_indices.len() {
                modal.selected_index = 0;
            }
        }
    }

    /// Handle text input for filter modal
    pub fn filter_modal_input(&mut self, ch: char) {
        if let Some(modal) = &mut self.filter_modal_state {
            modal.input.insert(modal.cursor_position, ch);
            modal.cursor_position += 1;

            // Update filtered list
            let input = modal.input.clone();
            self.filter_modal_update_input(&input);
        }
    }

    /// Handle backspace in filter modal
    pub fn filter_modal_backspace(&mut self) {
        if let Some(modal) = &mut self.filter_modal_state {
            if modal.cursor_position > 0 {
                modal.cursor_position -= 1;
                modal.input.remove(modal.cursor_position);

                // Update filtered list
                let input = modal.input.clone();
                self.filter_modal_update_input(&input);
            }
        }
    }

    /// Handle delete key in filter modal
    pub fn filter_modal_delete(&mut self) {
        if let Some(modal) = &mut self.filter_modal_state {
            if modal.cursor_position < modal.input.len() {
                modal.input.remove(modal.cursor_position);

                // Update filtered list
                let input = modal.input.clone();
                self.filter_modal_update_input(&input);
            }
        }
    }

    /// Clear filter input
    pub fn filter_modal_clear(&mut self) {
        if let Some(modal) = &mut self.filter_modal_state {
            modal.input.clear();
            modal.cursor_position = 0;

            // Reset filtered list to show all
            modal.filtered_indices = (0..modal.available_labels.len()).collect();
            modal.selected_index = 0;
        }
    }

    /// Get currently selected label from filter modal
    #[must_use]
    pub fn selected_filter_label(&self) -> Option<&LabelStats> {
        self.filter_modal_state.as_ref().and_then(|modal| {
            modal
                .filtered_indices
                .get(modal.selected_index)
                .and_then(|&idx| modal.available_labels.get(idx))
        })
    }

    /// Collect the expansion state of all task tree nodes
    ///
    /// Returns a set of task IDs that are currently expanded.
    /// Use with `restore_expansion_state()` to preserve expansion state
    /// across tree rebuilds.
    #[must_use]
    pub fn collect_expansion_state(&self) -> std::collections::HashSet<i64> {
        fn collect_from_nodes(
            nodes: &[TreeNode<TaskRecord>],
            expanded_ids: &mut std::collections::HashSet<i64>,
        ) {
            for node in nodes {
                if node.expanded {
                    expanded_ids.insert(node.data.id);
                }
                collect_from_nodes(&node.children, expanded_ids);
            }
        }

        let mut expanded_ids = std::collections::HashSet::new();
        if let Some(tree) = &self.task_tree {
            collect_from_nodes(tree, &mut expanded_ids);
        }
        expanded_ids
    }

    /// Restore expansion state to task tree nodes
    ///
    /// Takes a set of task IDs that should be expanded and sets the
    /// expansion state accordingly. Used after rebuilding the tree
    /// to preserve user's manual expansion choices.
    pub fn restore_expansion_state(&mut self, expanded_ids: &std::collections::HashSet<i64>) {
        fn restore_to_nodes(
            nodes: &mut [TreeNode<TaskRecord>],
            expanded_ids: &std::collections::HashSet<i64>,
        ) {
            for node in nodes {
                if expanded_ids.contains(&node.data.id) && !node.children.is_empty() {
                    node.expanded = true;
                }
                restore_to_nodes(&mut node.children, expanded_ids);
            }
        }

        if let Some(tree) = &mut self.task_tree {
            restore_to_nodes(tree, expanded_ids);
        }
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
