# Task 10: Tree View Support - Implementation Plan

**Status:** NOT STARTED
**Priority:** HIGH
**Estimated Effort:** 3-4 days
**Dependencies:** Task 2 (Navigation Pane), Task 3 (Detail Pane), Task 9 (CLI Color Scheme Integration)

---

## Executive Summary

This plan details the implementation of hierarchical tree view support across both TUI and CLI interfaces. The work is organized into 4 phases with clear dependencies, enabling incremental development and testing. The plan prioritizes risk mitigation by implementing core data structures first, followed by TUI integration, then CLI commands, with edge cases handled throughout.

**Key Design Decisions:**
- Generic `TreeNode<T>` structure enables code reuse across file trees and task hierarchies
- Configuration follows existing patterns (`UserConfig` in `~/.lash/config.toml`)
- Unicode/ASCII character sets with automatic terminal detection
- Vim-inspired keyboard shortcuts for familiarity
- Expansion state tracked in TUI session only (not persisted to disk)

---

## Phase 1: Foundation - Configuration & Data Structures

**Effort:** 0.5-1 day
**Dependencies:** None
**Risk:** Low

### 1.1 Add Tree View Configuration

**Files to Modify:**
- `crates/lash-types/src/config.rs`
- `crates/lash-types/src/lib.rs`

**Implementation Details:**

Add tree view configuration to `UserConfig`:

```rust
// In lash-types/src/config.rs

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserConfig {
    /// Selected color scheme name (default: `Base2Tone Desert`)
    #[serde(default = "default_color_scheme")]
    pub color_scheme: String,

    /// Tree view settings
    #[serde(default)]
    pub tree_view: TreeViewConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreeViewConfig {
    /// Enable tree view by default
    #[serde(default = "default_tree_enabled")]
    pub enabled: bool,

    /// Maximum depth to display
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// Start with all nodes expanded
    #[serde(default = "default_expanded")]
    pub default_expanded: bool,

    /// Force ASCII mode (otherwise auto-detect)
    #[serde(default = "default_ascii")]
    pub ascii_mode: bool,
}

fn default_tree_enabled() -> bool { true }
fn default_max_depth() -> usize { 5 }
fn default_expanded() -> bool { false }
fn default_ascii() -> bool { false }

impl Default for TreeViewConfig {
    fn default() -> Self {
        Self {
            enabled: default_tree_enabled(),
            max_depth: default_max_depth(),
            default_expanded: default_expanded(),
            ascii_mode: default_ascii(),
        }
    }
}
```

**CLI Arguments to Add:**

Extend `LashCli` in `crates/lash-cli/src/main.rs`:

```rust
#[derive(Parser)]
#[command(name = "lash")]
pub struct LashCli {
    // ... existing fields ...

    /// Enable tree view output (default: true)
    #[arg(long, global = true)]
    pub tree_view: Option<bool>,

    /// Maximum tree depth to display
    #[arg(long, global = true, value_name = "N")]
    pub max_depth: Option<usize>,

    /// Force ASCII tree characters
    #[arg(long, global = true)]
    pub ascii: bool,
}
```

**Validation:**
- `max_depth` must be 1-10 (validate in `UserConfig::load()`)
- Priority: CLI flag > user config > default

**Tests:**
- Unit test for `TreeViewConfig` defaults
- Unit test for config serialization/deserialization
- Integration test for CLI flag parsing
- Integration test for config file loading with tree view settings

**Success Criteria:**
- [ ] `TreeViewConfig` struct compiles and has correct defaults
- [ ] `UserConfig` includes `tree_view` field
- [ ] CLI flags parse correctly
- [ ] Config loads/saves to `~/.lash/config.toml`
- [ ] All tests pass

---

### 1.2 Implement Generic Tree Data Structure

**Files to Create:**
- `crates/lash-types/src/tree.rs`

**Implementation Details:**

```rust
// In lash-types/src/tree.rs

/// Generic tree node with expansion state tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode<T> {
    /// Node data
    pub data: T,

    /// Child nodes
    pub children: Vec<TreeNode<T>>,

    /// Whether this node is expanded (for interactive views)
    pub expanded: bool,

    /// Depth in the tree (0 = root)
    pub depth: usize,
}

impl<T> TreeNode<T> {
    /// Create a new tree node
    pub fn new(data: T, depth: usize) -> Self {
        Self {
            data,
            children: Vec::new(),
            expanded: false,
            depth,
        }
    }

    /// Create a new tree node with children
    pub fn with_children(data: T, depth: usize, children: Vec<TreeNode<T>>) -> Self {
        Self {
            data,
            children,
            expanded: false,
            depth,
        }
    }

    /// Check if this node has children
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Expand this node
    pub fn expand(&mut self) {
        self.expanded = true;
    }

    /// Collapse this node
    pub fn collapse(&mut self) {
        self.expanded = false;
    }

    /// Toggle expansion state
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Recursively expand all nodes up to max_depth
    pub fn expand_all(&mut self, max_depth: usize) {
        if self.depth < max_depth {
            self.expanded = true;
            for child in &mut self.children {
                child.expand_all(max_depth);
            }
        }
    }

    /// Recursively collapse all nodes
    pub fn collapse_all(&mut self) {
        self.expanded = false;
        for child in &mut self.children {
            child.collapse_all();
        }
    }

    /// Flatten tree into a list for rendering (only expanded nodes)
    /// Returns list of (node, is_last_sibling) tuples
    pub fn flatten(&self) -> Vec<(&T, usize, bool)> {
        let mut result = Vec::new();
        self.flatten_recursive(&mut result, true);
        result
    }

    fn flatten_recursive<'a>(&'a self, result: &mut Vec<(&'a T, usize, bool)>, is_last: bool) {
        result.push((&self.data, self.depth, is_last));

        if self.expanded {
            for (i, child) in self.children.iter().enumerate() {
                let is_last_child = i == self.children.len() - 1;
                child.flatten_recursive(result, is_last_child);
            }
        }
    }

    /// Count total visible nodes (self + expanded children)
    pub fn visible_count(&self) -> usize {
        if !self.expanded {
            return 1;
        }

        1 + self.children.iter().map(|c| c.visible_count()).sum::<usize>()
    }
}
```

**Tree Character Sets:**

```rust
// In lash-types/src/tree.rs

/// Character sets for rendering tree structures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeChars {
    Unicode,
    Ascii,
}

impl TreeChars {
    /// Get the branch character (intermediate child)
    pub fn branch(&self) -> &'static str {
        match self {
            TreeChars::Unicode => "├── ",
            TreeChars::Ascii => "+-- ",
        }
    }

    /// Get the last branch character (last child)
    pub fn last_branch(&self) -> &'static str {
        match self {
            TreeChars::Unicode => "└── ",
            TreeChars::Ascii => "\\-- ",
        }
    }

    /// Get the vertical line character (continuation)
    pub fn vertical(&self) -> &'static str {
        match self {
            TreeChars::Unicode => "│   ",
            TreeChars::Ascii => "|   ",
        }
    }

    /// Get the empty space (for last siblings)
    pub fn empty(&self) -> &'static str {
        "    "
    }

    /// Get the collapsed indicator
    pub fn collapsed(&self) -> &'static str {
        match self {
            TreeChars::Unicode => "▸ ",
            TreeChars::Ascii => "> ",
        }
    }

    /// Get the expanded indicator
    pub fn expanded(&self) -> &'static str {
        match self {
            TreeChars::Unicode => "▾ ",
            TreeChars::Ascii => "v ",
        }
    }

    /// Auto-detect based on terminal capabilities
    pub fn detect() -> Self {
        // Check if terminal supports Unicode
        // Simple heuristic: check LANG/LC_ALL for UTF-8
        if let Ok(lang) = std::env::var("LANG") {
            if lang.to_lowercase().contains("utf") {
                return TreeChars::Unicode;
            }
        }
        TreeChars::Ascii
    }
}
```

**Tests:**
- Unit test for `TreeNode::new()`
- Unit test for `TreeNode::expand()`, `collapse()`, `toggle()`
- Unit test for `TreeNode::expand_all()`, `collapse_all()`
- Unit test for `TreeNode::flatten()` with various tree structures
- Unit test for `TreeNode::visible_count()`
- Unit test for `TreeChars` methods
- Unit test for `TreeChars::detect()`

**Success Criteria:**
- [ ] `TreeNode<T>` compiles with all methods
- [ ] `TreeChars` enum provides correct character sets
- [ ] All tree operations work correctly (expand, collapse, flatten)
- [ ] All tests pass with 100% coverage

---

## Phase 2: TUI Integration

**Effort:** 1-1.5 days
**Dependencies:** Phase 1
**Risk:** Medium (complex state management)

### 2.1 Update TUI State for Tree Views

**Files to Modify:**
- `crates/lash-tui/src/state.rs`

**Implementation Details:**

Add tree-specific state:

```rust
// In state.rs

use lash_types::{TreeNode, TreeChars};

pub struct AppState {
    // ... existing fields ...

    /// File tree (if tree view enabled)
    pub file_tree: Option<Vec<TreeNode<FileRecord>>>,

    /// Task tree for current file (if tree view enabled)
    pub task_tree: Option<Vec<TreeNode<TaskRecord>>>,

    /// Tree character set to use
    pub tree_chars: TreeChars,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            // ... existing initialization ...
            file_tree: None,
            task_tree: None,
            tree_chars: TreeChars::detect(),
        }
    }

    /// Build file tree from flat file list
    pub fn build_file_tree(&mut self) {
        // Convert flat file list to hierarchical tree
        // Group by directory, create TreeNode<FileRecord> structure
        // Apply default_expanded from config
        // This is called after loading files from DB
    }

    /// Build task tree from flat task list
    pub fn build_task_tree(&mut self) {
        // Convert flat task list to hierarchical tree
        // Use parent_id and depth fields from TaskRecord
        // Apply default_expanded from config
        // This is called after loading tasks for a file
    }
}
```

**Tree Building Algorithm:**

For files (directory tree):
1. Sort files by path
2. For each file, extract directory components
3. Build tree structure from components
4. Attach files as leaf nodes

For tasks (task hierarchy):
1. Tasks already have `parent_id` and `depth` from DB
2. Build tree by grouping by `parent_id`
3. Maintain `order_index` for sibling ordering

**Tests:**
- Unit test for `build_file_tree()` with various directory structures
- Unit test for `build_task_tree()` with various task hierarchies
- Unit test for tree expansion state initialization
- Integration test for loading files and building tree

**Success Criteria:**
- [ ] `AppState` includes tree fields
- [ ] `build_file_tree()` creates correct tree structure
- [ ] `build_task_tree()` creates correct task hierarchy
- [ ] Tree expansion state correctly initialized from config
- [ ] All tests pass

---

### 2.2 Add Tree Navigation Event Handlers

**Files to Modify:**
- `crates/lash-tui/src/event.rs`
- `crates/lash-tui/src/app.rs`

**Event Handling:**

```rust
// In event.rs - add new events

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    // ... existing events ...

    /// Expand current node (zo)
    ExpandNode,

    /// Collapse current node (zc)
    CollapseNode,

    /// Expand all nodes (zR)
    ExpandAll,

    /// Collapse all nodes (zM)
    CollapseAll,
}

fn handle_key_event(key: KeyEvent) -> AppEvent {
    match (key.code, key.modifiers) {
        // ... existing key handlers ...

        // Tree navigation (h/l already mapped to Left/Right)
        (KeyCode::Char('H'), KeyModifiers::SHIFT) => AppEvent::CollapseAll,
        (KeyCode::Char('L'), KeyModifiers::SHIFT) => AppEvent::ExpandAll,

        // Vim-style fold commands
        (KeyCode::Char('o'), KeyModifiers::NONE) => {
            // Need to check if previous key was 'z'
            // For simplicity, use two-key shortcuts: zo, zc, zM, zR
            // This requires tracking previous key state
            // Alternative: just use H/L for collapse/expand all
            AppEvent::ExpandNode
        }
        (KeyCode::Char('c'), KeyModifiers::NONE) => AppEvent::CollapseNode,
        (KeyCode::Char('M'), KeyModifiers::SHIFT) => AppEvent::CollapseAll,
        (KeyCode::Char('R'), KeyModifiers::SHIFT) => AppEvent::ExpandAll,

        _ => AppEvent::None,
    }
}
```

**Note on Vim-style commands:** Implementing `zo`, `zc`, `zM`, `zR` requires tracking previous key state. For MVP, we'll use:
- `h`/`←`: Collapse node or go to parent
- `l`/`→`/`Enter`: Expand node or enter
- `H`: Collapse all
- `L`: Expand all
- Optional: Add `o`, `c`, `M`, `R` as simpler alternatives

**App Event Handling:**

```rust
// In app.rs

pub fn run(/* ... */) -> TuiResult<()> {
    loop {
        let event = poll_event(timeout)?;

        match event {
            // ... existing event handlers ...

            AppEvent::Left => {
                if state.focused_pane == FocusedPane::Navigation {
                    if let Some(tree) = &mut state.file_tree {
                        collapse_or_go_to_parent(tree, state.selected_file_index);
                    }
                } else {
                    if let Some(tree) = &mut state.task_tree {
                        collapse_or_go_to_parent(tree, state.selected_task_index);
                    }
                }
            }

            AppEvent::Right => {
                if state.focused_pane == FocusedPane::Navigation {
                    if let Some(tree) = &mut state.file_tree {
                        expand_or_enter(tree, state.selected_file_index);
                    }
                } else {
                    if let Some(tree) = &mut state.task_tree {
                        expand_or_enter(tree, state.selected_task_index);
                    }
                }
            }

            AppEvent::ExpandAll => {
                if state.focused_pane == FocusedPane::Navigation {
                    if let Some(tree) = &mut state.file_tree {
                        for node in tree {
                            node.expand_all(user_config.tree_view.max_depth);
                        }
                    }
                } else {
                    if let Some(tree) = &mut state.task_tree {
                        for node in tree {
                            node.expand_all(user_config.tree_view.max_depth);
                        }
                    }
                }
            }

            AppEvent::CollapseAll => {
                if state.focused_pane == FocusedPane::Navigation {
                    if let Some(tree) = &mut state.file_tree {
                        for node in tree {
                            node.collapse_all();
                        }
                    }
                } else {
                    if let Some(tree) = &mut state.task_tree {
                        for node in tree {
                            node.collapse_all();
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

fn collapse_or_go_to_parent<T>(tree: &mut [TreeNode<T>], selected_index: usize) {
    // If current node is expanded, collapse it
    // Otherwise, move selection to parent node
}

fn expand_or_enter<T>(tree: &mut [TreeNode<T>], selected_index: usize) {
    // If current node is collapsed and has children, expand it
    // Otherwise, do nothing (or enter for files)
}
```

**Tests:**
- Integration test for Left/Right key events
- Integration test for H/L key events (expand/collapse all)
- Integration test for tree navigation with various structures
- Manual test for keyboard responsiveness

**Success Criteria:**
- [ ] Tree navigation keys work in both panes
- [ ] Expand/collapse operations update tree state
- [ ] Selection moves correctly after expand/collapse
- [ ] All tests pass

---

### 2.3 Update TUI Rendering for Tree Views

**Files to Modify:**
- `crates/lash-tui/src/ui/nav_pane.rs`
- `crates/lash-tui/src/ui/detail_pane.rs`

**Navigation Pane Rendering:**

```rust
// In nav_pane.rs

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if let Some(file_tree) = &state.file_tree {
        render_file_tree(frame, area, state, file_tree);
    } else {
        render_flat_file_list(frame, area, state); // Fallback
    }
}

fn render_file_tree(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    tree: &[TreeNode<FileRecord>],
) {
    let items: Vec<ListItem> = tree
        .iter()
        .flat_map(|node| node.flatten())
        .enumerate()
        .map(|(i, (file, depth, is_last))| {
            let prefix = build_tree_prefix(depth, is_last, &state.tree_chars);
            let expansion_indicator = if has_children {
                if expanded {
                    state.tree_chars.expanded()
                } else {
                    state.tree_chars.collapsed()
                }
            } else {
                ""
            };

            let line = format!(
                "{}{}{} [{}]",
                prefix,
                expansion_indicator,
                file.path.display(),
                file.task_counts.total()
            );

            let style = if i == state.selected_file_index {
                state.theme.selected_style()
            } else {
                state.theme.default_style()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    // Render list with ratatui
}

fn build_tree_prefix(depth: usize, is_last: bool, chars: &TreeChars) -> String {
    // Build the tree prefix based on depth and sibling status
    // E.g., "│   ├── " or "    └── "
}
```

**Detail Pane Rendering:**

```rust
// In detail_pane.rs

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if let Some(task_tree) = &state.task_tree {
        render_task_tree(frame, area, state, task_tree);
    } else {
        render_flat_task_list(frame, area, state); // Fallback
    }
}

fn render_task_tree(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    tree: &[TreeNode<TaskRecord>],
) {
    // Similar to file tree rendering
    // Show checkboxes, task titles, labels
    // Use tree characters for hierarchy
}
```

**Tests:**
- Unit test for `build_tree_prefix()` with various depths
- Visual test for tree rendering with Unicode characters
- Visual test for tree rendering with ASCII characters
- Integration test for rendering with selection
- Manual test for visual inspection

**Success Criteria:**
- [ ] File tree renders with correct indentation and tree characters
- [ ] Task tree renders with correct hierarchy
- [ ] Expansion indicators show correctly
- [ ] Unicode and ASCII modes both work
- [ ] Selection highlighting works
- [ ] All tests pass

---

## Phase 3: CLI Command Integration

**Effort:** 1-1.5 days
**Dependencies:** Phase 1, Phase 2
**Risk:** Medium (formatting complexity)

### 3.1 CLI Tree Rendering Utilities

**Files to Create:**
- `crates/lash-cli/src/tree_formatter.rs`

**Implementation Details:**

```rust
// In tree_formatter.rs

use lash_types::{TreeNode, TreeChars};
use owo_colors::{OwoColorize, Style};

pub struct TreeFormatter {
    chars: TreeChars,
    max_depth: usize,
}

impl TreeFormatter {
    pub fn new(ascii: bool, max_depth: usize) -> Self {
        let chars = if ascii {
            TreeChars::Ascii
        } else {
            TreeChars::detect()
        };

        Self { chars, max_depth }
    }

    /// Format a tree node with proper indentation and tree characters
    pub fn format_node<T, F>(
        &self,
        node: &TreeNode<T>,
        is_last: bool,
        ancestors_last: &[bool],
        format_data: F,
    ) -> String
    where
        F: Fn(&T) -> String,
    {
        let mut result = String::new();

        // Build prefix from ancestors
        for &ancestor_is_last in ancestors_last {
            if ancestor_is_last {
                result.push_str(self.chars.empty());
            } else {
                result.push_str(self.chars.vertical());
            }
        }

        // Add branch character
        if is_last {
            result.push_str(self.chars.last_branch());
        } else {
            result.push_str(self.chars.branch());
        }

        // Add expansion indicator
        if node.has_children() {
            if node.expanded {
                result.push_str(self.chars.expanded());
            } else {
                result.push_str(self.chars.collapsed());
            }
        }

        // Add formatted data
        result.push_str(&format_data(&node.data));

        result
    }

    /// Format an entire tree
    pub fn format_tree<T, F>(
        &self,
        tree: &[TreeNode<T>],
        format_data: F,
    ) -> Vec<String>
    where
        F: Fn(&T) -> String,
    {
        let mut lines = Vec::new();

        for node in tree {
            self.format_tree_recursive(node, true, &[], &format_data, &mut lines);
        }

        lines
    }

    fn format_tree_recursive<T, F>(
        &self,
        node: &TreeNode<T>,
        is_last: bool,
        ancestors_last: &[bool],
        format_data: &F,
        lines: &mut Vec<String>,
    ) where
        F: Fn(&T) -> String,
    {
        // Check max depth
        if node.depth >= self.max_depth {
            return;
        }

        // Format this node
        lines.push(self.format_node(node, is_last, ancestors_last, format_data));

        // Format children (if expanded or CLI mode)
        if node.expanded || !node.children.is_empty() {
            let mut new_ancestors = ancestors_last.to_vec();
            new_ancestors.push(is_last);

            for (i, child) in node.children.iter().enumerate() {
                let child_is_last = i == node.children.len() - 1;
                self.format_tree_recursive(
                    child,
                    child_is_last,
                    &new_ancestors,
                    format_data,
                    lines,
                );
            }
        }
    }
}
```

**Tests:**
- Unit test for `format_node()` with various depths
- Unit test for `format_tree()` with various tree structures
- Unit test for max_depth limiting
- Unit test for Unicode vs ASCII rendering
- Snapshot test for tree output formatting

**Success Criteria:**
- [ ] `TreeFormatter` correctly formats tree structures
- [ ] Tree characters render correctly
- [ ] Max depth limiting works
- [ ] All tests pass

---

### 3.2 Update `list` Command for Tree View

**Files to Modify:**
- `crates/lash-cli/src/commands/list.rs`

**Implementation Details:**

Add tree view support to list command:

```rust
// In list.rs

use crate::tree_formatter::TreeFormatter;

pub fn execute(/* ... */, tree_view: bool, max_depth: Option<usize>) -> Result<()> {
    // ... load files from DB ...

    if tree_view {
        print_file_tree(&files, max_depth, ascii, theme);
    } else {
        print_flat_file_list(&files, theme); // Existing implementation
    }

    Ok(())
}

fn print_file_tree(
    files: &[FileRecord],
    max_depth: Option<usize>,
    ascii: bool,
    theme: &CliTheme,
) {
    // Build tree from flat file list
    let tree = build_file_tree_from_flat_list(files);

    let formatter = TreeFormatter::new(ascii, max_depth.unwrap_or(5));

    let lines = formatter.format_tree(&tree, |file| {
        format!(
            "{} [{}]",
            theme.style_info().style(file.path.display()),
            format_task_counts(&file.task_counts, theme)
        )
    });

    for line in lines {
        println!("{}", line);
    }
}

fn build_file_tree_from_flat_list(files: &[FileRecord]) -> Vec<TreeNode<FileRecord>> {
    // Group files by directory
    // Create tree structure
}
```

**Example Output:**

```
tasks/
├── tasks.md [3/5]
├── backend/
│   ├── api.md [2/4]
│   └── db.md [5/5] ✓
└── frontend/
    ├── components.md [1/3]
    └── styles.md [0/2]
```

**Tests:**
- Integration test for `lash list --tree`
- Integration test for `lash list --no-tree`
- Integration test for `lash list --depth 3`
- Integration test for `lash list --ascii`
- Snapshot test for tree output format
- Manual test for visual inspection

**Success Criteria:**
- [ ] `lash list --tree` displays hierarchical tree
- [ ] `lash list --no-tree` displays flat list
- [ ] `--depth` flag limits tree depth
- [ ] `--ascii` flag uses ASCII characters
- [ ] All tests pass

---

### 3.3 Update `search` Command for Tree View

**Files to Modify:**
- `crates/lash-cli/src/commands/search.rs`

**Implementation Details:**

Group search results by file path in tree format:

```rust
// In search.rs

pub fn execute(query: &str, /* ... */, tree_view: bool) -> Result<()> {
    // ... execute search ...

    if tree_view {
        print_search_results_tree(&results, theme);
    } else {
        print_search_results_flat(&results, theme); // Existing
    }

    Ok(())
}

fn print_search_results_tree(results: &[SearchResult], theme: &CliTheme) {
    // Group results by file path
    // Build tree structure from file paths
    // Under each file node, list matching tasks

    // Example output:
    // tasks/
    // ├── backend/
    // │   └── api.md [3 matches]
    // │       ├── Line 12: "Implement authentication"
    // │       ├── Line 45: "Add API rate limiting"
    // │       └── Line 78: "Write API documentation"
    // └── frontend/
    //     └── components.md [1 match]
    //         └── Line 23: "Create Button component"
}
```

**Tests:**
- Integration test for `lash search --tree "TODO"`
- Integration test for search results grouping
- Snapshot test for tree output format
- Manual test for visual inspection

**Success Criteria:**
- [ ] Search results grouped by file in tree format
- [ ] Matches shown under file nodes
- [ ] Tree structure correct
- [ ] All tests pass

---

### 3.4 Update `show` Command for Tree View

**Files to Modify:**
- `crates/lash-cli/src/commands/show.rs`

**Implementation Details:**

Show task with subtask hierarchy and parent context:

```rust
// In show.rs

pub fn execute(task_id: &str, /* ... */) -> Result<()> {
    // ... load task ...

    // Show parent context (breadcrumb trail)
    print_parent_context(&task, theme);

    println!(); // Separator

    // Show task details
    print_task_details(&task, theme);

    println!(); // Separator

    // Show subtask hierarchy
    if has_subtasks {
        print_subtask_tree(&task, theme);
    }

    // Show dependencies
    if !dependencies.is_empty() {
        print_dependency_tree(&task, &dependencies, theme);
    }

    Ok(())
}

fn print_parent_context(task: &TaskRecord, theme: &CliTheme) {
    // Show breadcrumb: File > Parent Task > Parent Task > Current Task
    // Use tree characters for visual hierarchy
}

fn print_subtask_tree(task: &TaskRecord, theme: &CliTheme) {
    // Load subtasks from DB
    // Build tree structure
    // Render with TreeFormatter

    println!("Subtasks:");
    // └── Subtask 1 [x]
    //     ├── Sub-subtask 1.1 [ ]
    //     └── Sub-subtask 1.2 [x]
}

fn print_dependency_tree(
    task: &TaskRecord,
    dependencies: &[Dependency],
    theme: &CliTheme,
) {
    // Render dependency graph as tree
    println!("Dependencies:");
    // ├── Dependency 1 [x]
    // └── Dependency 2 [ ] (blocks this task)
}
```

**Tests:**
- Integration test for `lash show task.md#1`
- Integration test for parent context rendering
- Integration test for subtask tree rendering
- Integration test for dependency tree rendering
- Snapshot test for output format
- Manual test for visual inspection

**Success Criteria:**
- [ ] Parent context shows correctly
- [ ] Subtask hierarchy renders as tree
- [ ] Dependency tree shows correctly
- [ ] All tests pass

---

## Phase 4: Edge Cases & Polish

**Effort:** 0.5-1 day
**Dependencies:** Phases 1-3
**Risk:** Low

### 4.1 Handle Edge Cases

**Edge Cases to Address:**

1. **Very deep hierarchies (beyond max_depth)**
   - Show `...` indicator when depth limit reached
   - Test with 10+ level hierarchies
   - Verify graceful degradation

2. **Empty directories**
   - Option 1: Show "(empty)" suffix
   - Option 2: Hide empty directories entirely
   - Make configurable via `TreeViewConfig::show_empty_dirs`

3. **Single-file projects**
   - Detect when tree view provides no value
   - Automatically fall back to flat view
   - Or show minimal tree with single file

4. **Circular dependencies in task trees**
   - Database schema should prevent this
   - But add defensive check in `build_task_tree()`
   - Detect cycles and warn user
   - Break cycle at detection point

5. **Unicode rendering issues**
   - Test on various terminals (iTerm2, Terminal.app, Windows Terminal)
   - Verify fallback to ASCII when needed
   - Test `LANG` environment variable detection

6. **Very long file paths**
   - Truncate paths that exceed terminal width
   - Add ellipsis (...) for truncated paths
   - Ensure tree structure remains readable

7. **No files/tasks**
   - Show helpful empty state message
   - Don't attempt to build tree from empty list

**Implementation:**

```rust
// In tree.rs

impl<T> TreeNode<T> {
    /// Flatten with depth limit and overflow indicator
    pub fn flatten_with_limit(
        &self,
        max_depth: usize,
    ) -> (Vec<(&T, usize, bool)>, bool) {
        let mut result = Vec::new();
        let mut overflowed = false;

        self.flatten_limited(&mut result, max_depth, true, &mut overflowed);

        (result, overflowed)
    }

    fn flatten_limited<'a>(
        &'a self,
        result: &mut Vec<(&'a T, usize, bool)>,
        max_depth: usize,
        is_last: bool,
        overflowed: &mut bool,
    ) {
        result.push((&self.data, self.depth, is_last));

        if self.depth >= max_depth {
            if !self.children.is_empty() {
                *overflowed = true;
            }
            return;
        }

        if self.expanded {
            for (i, child) in self.children.iter().enumerate() {
                let is_last_child = i == self.children.len() - 1;
                child.flatten_limited(result, max_depth, is_last_child, overflowed);
            }
        }
    }
}

// In tree_formatter.rs

impl TreeFormatter {
    pub fn format_tree_with_overflow<T, F>(
        &self,
        tree: &[TreeNode<T>],
        format_data: F,
    ) -> Vec<String>
    where
        F: Fn(&T) -> String,
    {
        let mut lines = Vec::new();

        for node in tree {
            let (flattened, overflowed) = node.flatten_with_limit(self.max_depth);

            for (data, depth, is_last) in flattened {
                // Format node...
            }

            if overflowed {
                lines.push(format!("{}...", "    ".repeat(self.max_depth)));
            }
        }

        lines
    }
}
```

**Circular Dependency Detection:**

```rust
// In state.rs

use std::collections::HashSet;

fn detect_circular_dependency(
    task_id: i64,
    parent_id: Option<i64>,
    visited: &mut HashSet<i64>,
) -> bool {
    if let Some(pid) = parent_id {
        if visited.contains(&pid) {
            return true; // Circular dependency detected
        }
        visited.insert(pid);
    }
    false
}

pub fn build_task_tree(&mut self) {
    let mut visited = HashSet::new();

    for task in &self.tasks {
        if detect_circular_dependency(task.id, task.parent_id, &mut visited) {
            eprintln!("Warning: Circular dependency detected in task {}", task.full_id);
            // Break the cycle by setting parent_id to None
        }
    }

    // Continue building tree...
}
```

**Tests:**
- Integration test for depth limit with overflow indicator
- Integration test for empty directory handling
- Integration test for single-file project
- Integration test for circular dependency detection
- Integration test for Unicode fallback
- Integration test for path truncation
- Integration test for empty file/task lists
- Manual test on multiple terminal emulators

**Success Criteria:**
- [ ] Deep hierarchies show `...` indicator
- [ ] Empty directories handled gracefully
- [ ] Single-file projects work correctly
- [ ] Circular dependencies detected and broken
- [ ] Unicode rendering works on common terminals
- [ ] ASCII fallback works correctly
- [ ] Long paths truncated properly
- [ ] Empty states show helpful messages
- [ ] All tests pass

---

### 4.2 Documentation & Help Text

**Files to Modify:**
- `crates/lash-cli/src/main.rs` (help text)
- `crates/lash-tui/src/ui/help.rs` (TUI help overlay)
- `docs/user-guide.md` (user documentation)

**Updates Needed:**

1. **CLI Help Text:**
   - Document `--tree-view`, `--no-tree-view`, `--max-depth`, `--ascii` flags
   - Add examples of tree view output
   - Explain configuration options

2. **TUI Help Overlay:**
   - Add tree navigation shortcuts to help screen
   - Document `h`/`l`, `H`/`L` keys
   - Explain expand/collapse behavior

3. **User Documentation:**
   - Add section on tree view configuration
   - Show example config file
   - Provide screenshots of tree view
   - Document keyboard shortcuts
   - Explain Unicode vs ASCII modes

**Example Config Documentation:**

```toml
# ~/.lash/config.toml

# Tree view settings
[tree_view]
enabled = true          # Enable tree view by default
max_depth = 5           # Maximum depth to display
default_expanded = false # Start with nodes collapsed
ascii_mode = false      # Use Unicode characters (auto-detect if false)
```

**Tests:**
- Manual test for `lash --help` output
- Manual test for TUI help overlay
- Manual test for config file examples
- Documentation review

**Success Criteria:**
- [ ] CLI help text includes tree view options
- [ ] TUI help overlay documents tree navigation
- [ ] User guide includes tree view section
- [ ] Config examples are correct
- [ ] Documentation is clear and complete

---

## Testing Strategy

### Unit Tests (throughout implementation)

**Coverage Goals:**
- `TreeNode<T>`: 100% coverage of all methods
- `TreeChars`: 100% coverage
- `TreeFormatter`: 100% coverage
- Tree building algorithms: 100% coverage

**Key Test Cases:**
- Empty trees
- Single-node trees
- Deeply nested trees (10+ levels)
- Wide trees (many siblings)
- Mixed depth trees
- Expansion state transitions

### Integration Tests

**Test Fixtures:**
Create test fixtures in `fixtures/tree-view-tests/`:

```
tree-view-tests/
├── lash.index.md
├── deep-hierarchy/
│   ├── level1/
│   │   ├── level2/
│   │   │   ├── level3/
│   │   │   │   ├── level4/
│   │   │   │   │   └── deep-task.md
├── wide-hierarchy/
│   ├── file1.md
│   ├── file2.md
│   ├── file3.md
│   └── file4.md
├── mixed-hierarchy/
│   ├── shallow.md
│   └── deep/
│       └── nested/
│           └── task.md
└── empty-dirs/
    ├── empty-dir-1/
    └── empty-dir-2/
```

**Integration Test Cases:**
1. `test_list_tree_view()` - Verify tree structure in list output
2. `test_list_flat_view()` - Verify fallback to flat view
3. `test_list_max_depth()` - Verify depth limiting
4. `test_list_ascii_mode()` - Verify ASCII character rendering
5. `test_search_tree_view()` - Verify search results grouped by file
6. `test_show_subtask_tree()` - Verify subtask hierarchy
7. `test_tui_tree_navigation()` - Verify keyboard navigation in TUI
8. `test_tui_expand_collapse()` - Verify expansion state changes
9. `test_config_loading()` - Verify tree view config loads correctly
10. `test_cli_flag_priority()` - Verify CLI flags override config

### Manual Testing

**Test Matrix:**

| Terminal | OS | Unicode | ASCII |
|----------|----|---------:|------:|
| iTerm2 | macOS | ✓ | ✓ |
| Terminal.app | macOS | ✓ | ✓ |
| Windows Terminal | Windows | ✓ | ✓ |
| GNOME Terminal | Linux | ✓ | ✓ |
| VS Code Terminal | All | ✓ | ✓ |

**Manual Test Cases:**
1. Visual inspection of tree rendering
2. Keyboard navigation responsiveness
3. Expand/collapse animation smoothness
4. Long file path handling
5. Terminal resize behavior
6. Color theme integration
7. Help text clarity

**Acceptance Criteria:**
- Tree renders correctly in all tested terminals
- Keyboard navigation is intuitive
- Unicode and ASCII modes both work
- Performance is acceptable (no lag with 100+ files/tasks)

---

## Risk Assessment & Mitigation

### High-Risk Areas

**1. Tree Building Performance**
- **Risk:** Building trees from flat lists could be slow for large projects
- **Mitigation:**
  - Profile tree building with large fixtures (1000+ files)
  - Optimize algorithm if needed (use HashMap for O(1) lookups)
  - Cache built trees in `AppState`
  - Lazy-load tree nodes on expansion (TUI only)

**2. Rendering Complexity**
- **Risk:** Tree rendering logic is complex and error-prone
- **Mitigation:**
  - Write comprehensive unit tests for rendering logic
  - Use snapshot tests for output verification
  - Manual visual testing on multiple terminals
  - Start with simple cases, iterate to complex ones

**3. State Management in TUI**
- **Risk:** Expansion state management could become inconsistent
- **Mitigation:**
  - Centralize state updates in `AppState` methods
  - Test state transitions thoroughly
  - Add assertions to catch invalid states
  - Document state invariants

**4. Unicode Compatibility**
- **Risk:** Unicode characters may not render correctly on all terminals
- **Mitigation:**
  - Implement robust terminal detection
  - Provide `--ascii` flag for manual override
  - Test on multiple terminals and OS combinations
  - Fall back gracefully to ASCII when needed

### Medium-Risk Areas

**1. Configuration Priority**
- **Risk:** Priority logic (CLI flag > user config > default) could have bugs
- **Mitigation:**
  - Write explicit tests for each priority level
  - Document priority order clearly
  - Use existing patterns from color scheme implementation

**2. CLI Output Formatting**
- **Risk:** Tree output may not align properly or be readable
- **Mitigation:**
  - Test with various terminal widths
  - Implement path truncation for long paths
  - Use snapshot tests for output verification
  - Get user feedback on readability

**3. Dependency Handling**
- **Risk:** Dependency trees could be confusing or incorrect
- **Mitigation:**
  - Start with simple linear dependencies
  - Add tests for complex dependency graphs
  - Clearly indicate blocked/blocking relationships
  - Consider deferring dependency tree visualization if too complex

---

## Implementation Sequence

### Recommended Order

1. **Phase 1.1: Configuration** (0.5 day)
   - Low risk, enables all subsequent work
   - Can be developed and tested in isolation

2. **Phase 1.2: Tree Data Structure** (0.5 day)
   - Core foundation for everything else
   - Can be fully tested with unit tests

3. **Phase 2.1: TUI State** (0.5 day)
   - Builds on data structure
   - Testable with integration tests

4. **Phase 2.2: TUI Events** (0.5 day)
   - Can be developed in parallel with 3.1
   - Testable with event simulation

5. **Phase 3.1: CLI Formatter** (0.5 day)
   - Can be developed in parallel with 2.2
   - Testable with unit tests

6. **Phase 2.3: TUI Rendering** (0.5 day)
   - Depends on 2.1 and 2.2
   - Visual testing required

7. **Phase 3.2-3.4: CLI Commands** (1 day)
   - Can be done in sequence or parallel
   - Each command is independent

8. **Phase 4.1: Edge Cases** (0.5 day)
   - Address issues discovered during testing
   - Fix bugs and handle corner cases

9. **Phase 4.2: Documentation** (0.5 day)
   - Final polish
   - Can be done alongside Phase 4.1

**Total Estimated Time:** 4-5 days

### Parallel Work Opportunities

- Phase 2.2 (TUI Events) can be developed in parallel with Phase 3.1 (CLI Formatter)
- CLI commands (3.2, 3.3, 3.4) can be developed in parallel once 3.1 is complete
- Documentation (4.2) can be written alongside implementation

---

## Validation & Acceptance

### Definition of Done

For each phase:
- [ ] All code compiles without warnings
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Clippy has no warnings
- [ ] Code is documented with rustdoc comments
- [ ] Changes are committed with clear commit messages

For the overall task:
- [ ] All subtasks in Task 10 are checked off
- [ ] TUI tree view works for files and tasks
- [ ] CLI tree view works for list, search, and show commands
- [ ] Configuration persists correctly
- [ ] Keyboard navigation is intuitive
- [ ] Unicode and ASCII modes both work
- [ ] All edge cases handled gracefully
- [ ] Documentation is complete and accurate
- [ ] Manual testing completed on multiple terminals
- [ ] Performance is acceptable (no user-perceptible lag)

### Performance Benchmarks

**Target Metrics:**
- Tree building: < 100ms for 1000 files
- Tree rendering (TUI): < 16ms (60 FPS)
- Tree formatting (CLI): < 50ms for 1000 files
- Keyboard navigation: < 10ms response time

**Benchmarking Approach:**
- Use `cargo bench` for micro-benchmarks
- Create large fixtures for realistic testing
- Profile with `cargo flamegraph` if performance issues detected

### User Acceptance Criteria

From a user perspective:
- [ ] Tree view makes hierarchical structure immediately obvious
- [ ] Keyboard shortcuts feel natural and responsive
- [ ] Configuration options are discoverable
- [ ] Output is visually appealing and readable
- [ ] ASCII fallback works when needed
- [ ] Tree view enhances workflow (doesn't hinder it)

---

## Appendix: Code Snippets

### Example Configuration File

```toml
# ~/.lash/config.toml

color_scheme = "Base2Tone Desert"

[tree_view]
enabled = true
max_depth = 5
default_expanded = false
ascii_mode = false
```

### Example CLI Usage

```bash
# Enable tree view (default)
lash list

# Disable tree view
lash list --no-tree-view

# Limit depth
lash list --max-depth 3

# Force ASCII mode
lash list --ascii

# Search with tree grouping
lash search --tree "TODO"

# Show task with subtree
lash show tasks.md#implementation
```

### Example TUI Keyboard Shortcuts

```
Navigation Pane (File Tree):
  j/k, ↑/↓    Move selection up/down
  h, ←        Collapse node or go to parent
  l, →, Enter Expand node or open file
  H           Collapse all nodes
  L           Expand all nodes (to max_depth)
  gg          Go to top
  G           Go to bottom

Detail Pane (Task Tree):
  j/k, ↑/↓    Move selection up/down
  h, ←        Collapse task or go to parent
  l, →, Enter Expand task or show details
  H           Collapse all tasks
  L           Expand all tasks
  gg          Go to top
  G           Go to bottom
```

### Example Output (Unicode)

```
tasks/
├── tasks.md [3/5]
├── backend/
│   ├── api.md [2/4]
│   └── db.md [5/5] ✓
└── frontend/
    ├── components/
    │   ├── Button.md [1/2]
    │   └── Input.md [2/2] ✓
    └── styles.md [0/2]
```

### Example Output (ASCII)

```
tasks/
+-- tasks.md [3/5]
+-- backend/
|   +-- api.md [2/4]
|   \-- db.md [5/5] OK
\-- frontend/
    +-- components/
    |   +-- Button.md [1/2]
    |   \-- Input.md [2/2] OK
    \-- styles.md [0/2]
```

---

## References

**Existing Code Patterns:**
- Configuration: `crates/lash-types/src/config.rs`
- TUI State: `crates/lash-tui/src/state.rs`
- TUI Rendering: `crates/lash-tui/src/ui/*.rs`
- CLI Formatting: `crates/lash-cli/src/formatter.rs`
- Theme Integration: `crates/lash-cli/src/theme.rs`

**External References:**
- ratatui tree examples: https://github.com/ratatui-org/ratatui/tree/main/examples
- Vim fold commands: `:help folding`
- Unicode box-drawing: https://en.wikipedia.org/wiki/Box-drawing_character
- Terminal capability detection: https://github.com/crossterm-rs/crossterm

**Task File:**
- `tasks/tasks.tui.md` (Task 10)

---

**Document Version:** 1.0
**Last Updated:** 2025-11-25
**Author:** Claude (Project Manager)
