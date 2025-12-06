//! Main TUI application

use ratatui::{backend::CrosstermBackend, Terminal};
use rusqlite::Connection;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use lash_db::repository::dependencies::DependencyRepository;
use lash_db::repository::files::FileRepository;
use lash_db::repository::labels::LabelRepository;
use lash_db::repository::tasks::TaskRepository;

use crate::error::{TuiError, TuiResult};
use crate::event::{poll_event, poll_filter_event, poll_search_event, AppEvent};
use crate::state::AppState;
use crate::terminal;
use crate::ui;

/// Main TUI application
pub struct TuiApp {
    /// Terminal instance
    terminal: Terminal<CrosstermBackend<io::Stdout>>,

    /// Database connection
    conn: Connection,

    /// Application state
    state: AppState,

    /// Project root directory (parent of .lash/)
    project_root: PathBuf,
}

impl TuiApp {
    /// Create a new TUI application
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Database connection fails
    /// - Terminal setup fails
    pub fn new(db_path: &Path) -> TuiResult<Self> {
        Self::new_with_scheme(db_path, None)
    }

    /// Create a new TUI application with a specific color scheme
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Database connection fails
    /// - Terminal setup fails
    /// - Color scheme is invalid
    pub fn new_with_scheme(db_path: &Path, color_scheme: Option<&str>) -> TuiResult<Self> {
        let terminal = terminal::setup()?;
        let conn = lash_db::open_database(db_path)?;

        // Calculate project root from db_path (db is at .lash/lash.db)
        let project_root = db_path
            .parent() // .lash/
            .and_then(|p| p.parent()) // project root
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);

        let mut state = if let Some(scheme_name) = color_scheme {
            AppState::with_color_scheme(scheme_name)
                .map_err(|e| TuiError::App(format!("Invalid color scheme: {e}")))?
        } else {
            AppState::new()
        };

        // Load initial files
        let file_repo = FileRepository::new(&conn);
        state.files = file_repo
            .list_all()
            .map_err(|e| TuiError::App(format!("Failed to load files: {e}")))?;

        // Build file tree for tree view
        state.build_file_tree();

        // Create task repo for loading tasks
        let task_repo = TaskRepository::new(&conn);

        // Load tasks for the initially selected file
        // Use tree-aware selection: try tree view first, then fall back to flat list
        let file_id = if let Some(selected) = state.selected_tree_node() {
            selected.file_record.as_ref().map(|f| f.id)
        } else {
            state.selected_file().map(|f| f.id)
        };

        if let Some(file_id) = file_id {
            state.tasks = task_repo
                .get_by_file(file_id)
                .map_err(|e| TuiError::App(format!("Failed to load tasks: {e}")))?;
            state.build_task_tree();
        }

        // Load labels with stats
        let label_repo = LabelRepository::new(&conn);
        state.labels = label_repo
            .get_label_stats()
            .map_err(|e| TuiError::App(format!("Failed to load labels: {e}")))?;

        // Load project stats (total/completed tasks and root index title)
        let (total_tasks, completed_tasks) = task_repo
            .get_project_counts()
            .map_err(|e| TuiError::App(format!("Failed to load task counts: {e}")))?;

        let root_title = file_repo
            .get_root_index()
            .map_err(|e| TuiError::App(format!("Failed to load root index: {e}")))?
            .map(|f| f.title);

        state.project_stats = crate::state::ProjectStats {
            title: root_title,
            total_tasks,
            completed_tasks,
        };

        Ok(Self {
            terminal,
            conn,
            state,
            project_root,
        })
    }

    /// Run the application event loop
    ///
    /// # Errors
    ///
    /// Returns error if rendering or event handling fails
    #[allow(clippy::too_many_lines)]
    pub fn run(&mut self) -> TuiResult<()> {
        loop {
            // Check for expired status messages
            self.state.check_status_expiry();

            // Render
            self.terminal.draw(|frame| ui::render(frame, &self.state))?;

            // Handle events - use different polling depending on modal state
            let event = if self.state.is_search_modal_open() {
                poll_search_event(Duration::from_millis(100))?
            } else if self.state.is_filter_modal_open() {
                poll_filter_event(Duration::from_millis(100))?
            } else {
                poll_event(Duration::from_millis(100))?
            };

            // Route events based on active modal
            #[allow(clippy::match_same_arms)] // Placeholder arms for future features
            if self.state.is_search_modal_open() {
                // Search modal is open - route events to it
                match event {
                    AppEvent::CloseSearch => {
                        self.state.close_search_modal();
                    }
                    AppEvent::ExecuteSearch => {
                        self.handle_execute_search();
                    }
                    AppEvent::Up => self.state.search_modal_up(),
                    AppEvent::Down => self.state.search_modal_down(),
                    AppEvent::Left => self.state.search_modal_cursor_left(),
                    AppEvent::Right => self.state.search_modal_cursor_right(),
                    AppEvent::Home => self.state.search_modal_cursor_home(),
                    AppEvent::End => self.state.search_modal_cursor_end(),
                    AppEvent::Backspace => self.state.search_modal_backspace(),
                    AppEvent::Delete => self.state.search_modal_delete(),
                    AppEvent::ClearFilters => self.state.search_modal_clear(),
                    AppEvent::CharInput(c) => self.state.search_modal_input(c),
                    AppEvent::Select => {
                        // Select current result and navigate to it
                        self.handle_search_result_select()?;
                    }
                    _ => {} // Ignore other events when search is open
                }
            } else if self.state.is_filter_modal_open() {
                // Filter modal is open - route events to it
                match event {
                    AppEvent::CloseFilter => {
                        self.state.close_filter_modal();
                    }
                    AppEvent::ApplyFilter => {
                        self.handle_apply_filter()?;
                    }
                    AppEvent::Up => self.state.filter_modal_up(),
                    AppEvent::Down => self.state.filter_modal_down(),
                    AppEvent::Backspace => self.state.filter_modal_backspace(),
                    AppEvent::Delete => self.state.filter_modal_delete(),
                    AppEvent::ClearFilters => self.state.filter_modal_clear(),
                    AppEvent::CharInput(c) => self.state.filter_modal_input(c),
                    _ => {} // Ignore other events when filter is open
                }
            } else if self.state.is_task_detail_open() {
                // Task detail is open - route events to it
                match event {
                    AppEvent::Quit | AppEvent::CloseThemeSelector => {
                        self.state.close_task_detail();
                    }
                    AppEvent::Up => self.state.task_detail_scroll_up(),
                    AppEvent::Down => {
                        // Calculate visible height (popup height - borders and padding)
                        // Popup is 85% of screen height, minus 2 for borders, minus 2 for padding
                        let screen_height = self.terminal.size()?.height as usize;
                        let popup_height = (screen_height * 85) / 100;
                        let visible_height = popup_height.saturating_sub(4);
                        self.state.task_detail_scroll_down(visible_height);
                    }
                    _ => {} // Ignore other events when detail is open
                }
            } else if self.state.theme_selector_state.is_some() {
                // Theme selector is open - route events to it
                match event {
                    AppEvent::CloseThemeSelector | AppEvent::Quit => {
                        self.state.close_theme_selector();
                    }
                    AppEvent::Up => self.state.theme_selector_up(),
                    AppEvent::Down => self.state.theme_selector_down(),
                    AppEvent::Right | AppEvent::Select => {
                        if let Err(e) = self.state.apply_selected_theme() {
                            eprintln!("Failed to apply theme: {e}");
                        }
                    }
                    _ => {} // Ignore other events when selector is open
                }
            } else if self.state.show_help {
                // Help is open - only handle close events
                match event {
                    AppEvent::Help | AppEvent::CloseThemeSelector => {
                        self.state.toggle_help();
                    }
                    _ => {} // Ignore other events when help is shown
                }
            } else {
                // Normal event handling
                // Calculate description visible height for scrolling
                let screen_height = self.terminal.size().map(|s| s.height).unwrap_or(24) as usize;
                // Description pane is roughly 30% of screen height minus borders
                let description_visible_height = (screen_height * 30 / 100).saturating_sub(2);

                match event {
                    AppEvent::Quit => {
                        self.state.should_quit = true;
                    }
                    AppEvent::Up => {
                        self.state.move_up();
                        self.load_tasks_for_selected_file()?;
                    }
                    AppEvent::Down => {
                        self.state.move_down(description_visible_height);
                        self.load_tasks_for_selected_file()?;
                    }
                    AppEvent::Right => self.handle_select()?,
                    AppEvent::Select => self.handle_toggle_status()?,
                    AppEvent::SwitchPane => self.state.switch_pane(),
                    AppEvent::GoTop => {
                        self.state.go_top();
                        self.load_tasks_for_selected_file()?;
                    }
                    AppEvent::GoBottom => {
                        self.state.go_bottom(description_visible_height);
                        self.load_tasks_for_selected_file()?;
                    }
                    AppEvent::Help => self.state.toggle_help(),
                    AppEvent::OpenEditor => self.handle_open_editor()?,
                    AppEvent::OpenThemeSelector => self.state.open_theme_selector(),
                    AppEvent::Left => self.handle_left(),
                    AppEvent::ExpandAll => self.handle_expand_all(),
                    AppEvent::CollapseAll => self.handle_collapse_all(),

                    AppEvent::LabelFilter => self.handle_label_toggle()?,
                    AppEvent::ClearFilters => self.state.clear_label_filter(),

                    AppEvent::Search => self.state.open_search_modal(),
                    AppEvent::OpenFilter => self.handle_open_filter()?,

                    // TODO: implement these features
                    AppEvent::ExpandNode
                    | AppEvent::CollapseNode
                    | AppEvent::DependencyGraph
                    | AppEvent::PrevTask
                    | AppEvent::NextTask
                    | AppEvent::CloseThemeSelector
                    | AppEvent::CloseSearch
                    | AppEvent::ExecuteSearch
                    | AppEvent::CloseFilter
                    | AppEvent::ApplyFilter
                    | AppEvent::CharInput(_)
                    | AppEvent::Backspace
                    | AppEvent::Delete
                    | AppEvent::Home
                    | AppEvent::End
                    | AppEvent::Resize(_, _)
                    | AppEvent::None => {}
                }
            }

            if self.state.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Open task detail view for the currently selected task
    fn open_task_detail_for_selected(&mut self) -> TuiResult<()> {
        // Get the currently selected task
        let Some(task) = self.state.selected_task() else {
            return Ok(()); // No task selected
        };

        // Get the file path for this task
        let file_record = FileRepository::new(&self.conn)
            .get_by_db_id(task.file_id)
            .map_err(|e| TuiError::App(format!("Failed to get file for task: {e}")))?;

        let Some(file_record) = file_record else {
            return Err(TuiError::App(format!(
                "File not found for task: {}",
                task.id
            )));
        };

        // Get labels for this task
        let labels = LabelRepository::new(&self.conn)
            .get_task_labels(task.id)
            .map_err(|e| TuiError::App(format!("Failed to get labels for task: {e}")))?;

        // Get dependencies for this task
        let dependencies = DependencyRepository::new(&self.conn)
            .get_dependencies(task.id)
            .map_err(|e| TuiError::App(format!("Failed to get dependencies: {e}")))?;

        // Get subtasks (direct children) for this task
        let subtasks = TaskRepository::new(&self.conn)
            .get_children(task.id)
            .map_err(|e| TuiError::App(format!("Failed to get subtasks: {e}")))?;

        // Open the task detail view
        self.state.open_task_detail(
            task.clone(),
            file_record.path,
            labels,
            dependencies,
            subtasks,
        );

        Ok(())
    }

    /// Handle selection/enter in current pane
    fn handle_select(&mut self) -> TuiResult<()> {
        use crate::state::{FocusedPane, NavMode};

        match self.state.focused_pane {
            FocusedPane::Navigation => {
                match self.state.nav_mode {
                    NavMode::Files | NavMode::SearchResults => {
                        // Check if tree view is active and get selected node info
                        if let Some(selected) = self.state.selected_tree_node() {
                            if selected.is_directory {
                                // Toggle expand/collapse for directories
                                self.state.toggle_selected_node();
                            } else if let Some(file) = selected.file_record {
                                // Load tasks for file and switch to detail pane
                                let task_repo = TaskRepository::new(&self.conn);
                                self.state.tasks = task_repo.get_by_file(file.id).map_err(|e| {
                                    TuiError::App(format!("Failed to load tasks: {e}"))
                                })?;
                                self.state.selected_task_index = 0;
                                self.state.build_task_tree();
                                self.state.switch_pane();
                            }
                        } else {
                            // Fallback for flat view: load tasks for selected file
                            if let Some(file) = self.state.selected_file() {
                                let task_repo = TaskRepository::new(&self.conn);
                                self.state.tasks = task_repo.get_by_file(file.id).map_err(|e| {
                                    TuiError::App(format!("Failed to load tasks: {e}"))
                                })?;
                                self.state.selected_task_index = 0;
                                self.state.build_task_tree();
                                self.state.switch_pane();
                            }
                        }
                    }
                    NavMode::Labels => {
                        // Select a label and filter tasks
                        self.handle_label_select()?;
                    }
                }
            }
            FocusedPane::Description => {
                // Enter on description pane moves to tasks pane
                self.state.focused_pane = FocusedPane::Detail;
            }
            FocusedPane::Detail => {
                // Check if task has children that can be expanded
                if let Some(task_node) = self.state.selected_task_tree_node() {
                    if task_node.has_children && !task_node.is_expanded {
                        // Expand task to show subtasks inline
                        self.state.toggle_selected_task_node();
                    } else {
                        // Leaf task or already expanded - show details modal
                        self.open_task_detail_for_selected()?;
                    }
                } else {
                    // No tree view - show details modal
                    self.open_task_detail_for_selected()?;
                }
            }
        }
        Ok(())
    }

    /// Handle opening file in editor
    fn handle_open_editor(&mut self) -> TuiResult<()> {
        // Get the selected file path
        let file_path = match self.state.selected_file() {
            Some(file) => file.path.clone(),
            None => return Ok(()),
        };

        // Get editor from environment
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        // Restore terminal
        terminal::restore()?;

        // Run editor
        let status = std::process::Command::new(&editor)
            .arg(&file_path)
            .status()
            .map_err(|e| TuiError::App(format!("Failed to launch editor: {e}")))?;

        if !status.success() {
            terminal::setup()?;
            return Err(TuiError::App(format!(
                "Editor exited with non-zero status: {status}"
            )));
        }

        // Re-setup terminal
        self.terminal = terminal::setup()?;

        // TODO: Reload file if modified

        Ok(())
    }

    /// Handle toggling task status with Space bar
    fn handle_toggle_status(&mut self) -> TuiResult<()> {
        use crate::state::FocusedPane;

        // Check if we're in the right pane
        match self.state.focused_pane {
            FocusedPane::Navigation => {
                self.state.set_warning_message(
                    "Press Tab twice to focus Tasks pane, then Space to toggle status",
                );
                return Ok(());
            }
            FocusedPane::Description => {
                self.state.set_warning_message(
                    "Press Tab to focus Tasks pane, then Space to toggle status",
                );
                return Ok(());
            }
            FocusedPane::Detail => {
                // Continue with toggle logic below
            }
        }

        // Get the currently selected task (handles tree view correctly)
        let Some(task) = self.state.selected_task() else {
            self.state.set_warning_message("No task selected");
            return Ok(());
        };

        // Get file for this task (needed for markdown update and reloading)
        let file_record = FileRepository::new(&self.conn)
            .get_by_db_id(task.file_id)
            .map_err(|e| TuiError::App(format!("Failed to get file for task: {e}")))?;

        let Some(file_record) = file_record else {
            return Err(TuiError::App(format!(
                "File not found for task: {}",
                task.id
            )));
        };

        let file_path = file_record.path.clone();
        let file_id = file_record.id;

        // Capture task info before updating
        let task_id = task.id;
        let task_title = task.title.clone();
        let old_status = task.status;
        let new_status = match task.status {
            lash_types::TaskStatus::Open => lash_types::TaskStatus::Done,
            lash_types::TaskStatus::Done => lash_types::TaskStatus::Waived,
            lash_types::TaskStatus::Waived | lash_types::TaskStatus::Blocked => {
                lash_types::TaskStatus::Open
            }
        };

        // Update in database
        self.conn
            .execute(
                "UPDATE tasks SET status = ?1 WHERE id = ?2",
                (new_status.as_str(), task_id),
            )
            .map_err(|e| TuiError::App(format!("Failed to update task status: {e}")))?;

        // Update the markdown file
        if let Err(e) =
            self.update_markdown_task_status(&file_path, &task_title, old_status, new_status)
        {
            // Show warning but don't fail - database is already updated
            self.state
                .set_warning_message(format!("DB updated, but markdown update failed: {e}"));
        } else {
            // Show success message
            self.state.set_success_message(format!(
                "Task status: {} -> {}",
                Self::status_display_char(old_status),
                Self::status_display_char(new_status)
            ));
        }

        // Preserve expansion state before rebuilding tree
        let expanded_ids = self.state.collect_expansion_state();

        // Reload tasks for the correct file (using file_id from the task)
        let task_repo = TaskRepository::new(&self.conn);
        self.state.tasks = task_repo
            .get_by_file(file_id)
            .map_err(|e| TuiError::App(format!("Failed to reload tasks: {e}")))?;

        // Rebuild task tree to reflect updated status
        self.state.build_task_tree();

        // Restore expansion state after rebuild
        self.state.restore_expansion_state(&expanded_ids);

        Ok(())
    }

    /// Get display character for a task status
    fn status_display_char(status: lash_types::TaskStatus) -> &'static str {
        match status {
            lash_types::TaskStatus::Open => "[ ]",
            lash_types::TaskStatus::Done => "[x]",
            lash_types::TaskStatus::Waived => "[-]",
            lash_types::TaskStatus::Blocked => "[!]",
        }
    }

    /// Get checkbox character (just the char inside brackets) for a task status
    fn status_checkbox_char(status: lash_types::TaskStatus) -> char {
        match status {
            lash_types::TaskStatus::Open => ' ',
            lash_types::TaskStatus::Done => 'x',
            lash_types::TaskStatus::Waived => '-',
            lash_types::TaskStatus::Blocked => '!',
        }
    }

    /// Update task status in the markdown file
    ///
    /// Finds the task line by matching the title and old status, then updates
    /// the checkbox character to reflect the new status.
    fn update_markdown_task_status(
        &self,
        file_path: &Path,
        task_title: &str,
        old_status: lash_types::TaskStatus,
        new_status: lash_types::TaskStatus,
    ) -> TuiResult<()> {
        use std::fs;

        // Construct full path
        let full_path = self.project_root.join(file_path);

        // Read file content
        let content = fs::read_to_string(&full_path)
            .map_err(|e| TuiError::App(format!("Failed to read file: {e}")))?;

        // Build pattern to find the task line
        // Task lines look like: "- [ ] Task title" with optional leading whitespace
        let old_char = Self::status_checkbox_char(old_status);
        let new_char = Self::status_checkbox_char(new_status);

        // Escape special regex characters in the title
        let escaped_title = regex::escape(task_title);

        // Pattern: whitespace, dash, space, checkbox with old status, space, title
        // Handle both uppercase and lowercase 'x' for Done status
        let pattern = if old_status == lash_types::TaskStatus::Done {
            format!(r"^(\s*- \[)[xX](\] {escaped_title})")
        } else {
            format!(r"^(\s*- \[){old_char}(\] {escaped_title})")
        };

        let re = regex::Regex::new(&pattern)
            .map_err(|e| TuiError::App(format!("Failed to compile regex: {e}")))?;

        // Find and replace the task line
        let mut found = false;
        let updated_content: String = content
            .lines()
            .map(|line| {
                if !found && re.is_match(line) {
                    found = true;
                    re.replace(line, format!("${{1}}{new_char}${{2}}"))
                        .to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Preserve trailing newline if original had one
        let final_content = if content.ends_with('\n') && !updated_content.ends_with('\n') {
            format!("{updated_content}\n")
        } else {
            updated_content
        };

        if !found {
            return Err(TuiError::App(format!(
                "Could not find task '{task_title}' in file"
            )));
        }

        // Write updated content back to file
        fs::write(&full_path, final_content)
            .map_err(|e| TuiError::App(format!("Failed to write file: {e}")))?;

        Ok(())
    }

    /// Handle Left event (collapse node or go to parent in tree view)
    #[allow(clippy::unused_self)]
    fn handle_left(&mut self) {
        // TODO: Implement tree navigation for file/task tree
        // For now, no-op (Left is not used in flat list view)
    }

    /// Load tasks for the currently selected file in Navigation pane
    ///
    /// This is called when navigating up/down in the file list to automatically
    /// show tasks for the highlighted file in the Tasks pane.
    fn load_tasks_for_selected_file(&mut self) -> TuiResult<()> {
        use crate::state::{FocusedPane, NavMode};

        // Only load tasks when in Navigation pane in Files mode
        if self.state.focused_pane != FocusedPane::Navigation {
            return Ok(());
        }

        if self.state.nav_mode != NavMode::Files && self.state.nav_mode != NavMode::SearchResults {
            return Ok(());
        }

        // Try to get file from tree view first, then fall back to flat list
        let (file_id, description) = if let Some(selected) = self.state.selected_tree_node() {
            (
                selected.file_record.as_ref().map(|f| f.id),
                selected
                    .file_record
                    .as_ref()
                    .map(|f| f.description.clone())
                    .unwrap_or_default(),
            )
        } else {
            (
                self.state.selected_file().map(|f| f.id),
                self.state
                    .selected_file()
                    .map(|f| f.description.clone())
                    .unwrap_or_default(),
            )
        };

        // Reset description scroll and calculate content height
        self.state.reset_description_scroll();
        self.state.description_content_height = description.lines().count();

        let Some(file_id) = file_id else {
            // No file selected (might be a directory node) - clear tasks
            self.state.tasks.clear();
            self.state.task_tree = None;
            self.state.selected_task_index = 0;
            return Ok(());
        };

        // Load tasks for the selected file
        let task_repo = TaskRepository::new(&self.conn);
        self.state.tasks = task_repo
            .get_by_file(file_id)
            .map_err(|e| TuiError::App(format!("Failed to load tasks: {e}")))?;

        self.state.selected_task_index = 0;
        self.state.build_task_tree();

        Ok(())
    }

    /// Handle expand all nodes in current tree
    fn handle_expand_all(&mut self) {
        use crate::state::FocusedPane;
        use lash_types::UserConfig;

        let config = UserConfig::load().unwrap_or_default();
        let max_depth = config.tree_view.max_depth;

        match self.state.focused_pane {
            FocusedPane::Navigation => {
                if let Some(trees) = &mut self.state.file_tree {
                    for tree in trees {
                        tree.expand_all(max_depth);
                    }
                }
            }
            FocusedPane::Description => {} // No tree to expand in description pane
            FocusedPane::Detail => {
                if let Some(trees) = &mut self.state.task_tree {
                    for tree in trees {
                        tree.expand_all(max_depth);
                    }
                }
            }
        }
    }

    /// Handle collapse all nodes in current tree
    fn handle_collapse_all(&mut self) {
        use crate::state::FocusedPane;

        match self.state.focused_pane {
            FocusedPane::Navigation => {
                if let Some(trees) = &mut self.state.file_tree {
                    for tree in trees {
                        tree.collapse_all();
                    }
                }
            }
            FocusedPane::Description => {} // No tree to collapse in description pane
            FocusedPane::Detail => {
                if let Some(trees) = &mut self.state.task_tree {
                    for tree in trees {
                        tree.collapse_all();
                    }
                }
            }
        }
    }

    /// Handle toggling between files and labels view
    fn handle_label_toggle(&mut self) -> TuiResult<()> {
        // Reload labels to ensure we have latest stats
        let label_repo = LabelRepository::new(&self.conn);
        self.state.labels = label_repo
            .get_label_stats()
            .map_err(|e| TuiError::App(format!("Failed to load labels: {e}")))?;

        self.state.toggle_nav_mode();
        Ok(())
    }

    /// Handle selecting a label to filter tasks
    fn handle_label_select(&mut self) -> TuiResult<()> {
        let Some(label) = self.state.selected_label() else {
            return Ok(());
        };

        let label_name = label.name.clone();

        // Load tasks with this label
        let task_repo = TaskRepository::new(&self.conn);
        self.state.tasks = task_repo
            .find_by_label(&label_name)
            .map_err(|e| TuiError::App(format!("Failed to load tasks for label: {e}")))?;

        // Set current filter and reset selection
        self.state.current_label_filter = Some(label_name);
        self.state.selected_task_index = 0;
        self.state.build_task_tree();

        // Switch to detail pane to show filtered tasks
        self.state.focused_pane = crate::state::FocusedPane::Detail;

        Ok(())
    }

    /// Execute search with the current query
    fn handle_execute_search(&mut self) {
        use lash_db::search::{search, SearchQuery};

        let Some(query_str) = self.state.search_query() else {
            return;
        };

        // Skip if query is empty
        if query_str.trim().is_empty() {
            self.state.search_modal_set_results(Vec::new(), 0);
            return;
        }

        // Build search query with reasonable defaults
        let query = SearchQuery::new(query_str).with_limit(50);

        // Execute search
        match search(&self.conn, &query) {
            Ok(results) => {
                self.state
                    .search_modal_set_results(results.results, results.total_count);
            }
            Err(e) => {
                self.state
                    .search_modal_set_error(format!("Search failed: {e}"));
            }
        }
    }

    /// Handle selecting a search result
    fn handle_search_result_select(&mut self) -> TuiResult<()> {
        // Get selected result info before closing modal
        let Some(result) = self.state.selected_search_result() else {
            return Ok(());
        };

        let task_id = result.task_id;
        let file_path = result.file_path.clone();

        // Close search modal
        self.state.close_search_modal();

        // Find the file in our files list
        let file_record = self
            .state
            .files
            .iter()
            .find(|f| f.path.to_string_lossy() == file_path);

        let Some(file_record) = file_record else {
            // File not found - might need to refresh file list
            return Err(TuiError::App(format!(
                "File not found: {file_path}. Try refreshing the file list."
            )));
        };

        let file_id = file_record.id;

        // Find file index for selection
        if let Some(file_index) = self
            .state
            .files
            .iter()
            .position(|f| f.path.to_string_lossy() == file_path)
        {
            self.state.selected_file_index = file_index;
        }

        // Load tasks for this file
        let task_repo = TaskRepository::new(&self.conn);
        self.state.tasks = task_repo
            .get_by_file(file_id)
            .map_err(|e| TuiError::App(format!("Failed to load tasks: {e}")))?;

        // Build task tree
        self.state.build_task_tree();

        // Find and select the task
        if let Some(task_index) = self.state.tasks.iter().position(|t| t.id == task_id) {
            self.state.selected_task_index = task_index;
        }

        // Switch to detail pane and file view
        self.state.focused_pane = crate::state::FocusedPane::Detail;
        self.state.nav_mode = crate::state::NavMode::Files;
        self.state.current_label_filter = None;

        Ok(())
    }

    /// Handle opening filter modal
    fn handle_open_filter(&mut self) -> TuiResult<()> {
        // Load labels with stats
        let label_repo = LabelRepository::new(&self.conn);
        let labels = label_repo
            .get_label_stats()
            .map_err(|e| TuiError::App(format!("Failed to load labels: {e}")))?;

        self.state.open_filter_modal(labels);
        Ok(())
    }

    /// Handle applying selected filter
    fn handle_apply_filter(&mut self) -> TuiResult<()> {
        let Some(label) = self.state.selected_filter_label() else {
            // No label selected, close modal
            self.state.close_filter_modal();
            return Ok(());
        };

        let label_name = label.name.clone();

        // Close the filter modal
        self.state.close_filter_modal();

        // Load tasks with this label
        let task_repo = TaskRepository::new(&self.conn);
        self.state.tasks = task_repo
            .find_by_label(&label_name)
            .map_err(|e| TuiError::App(format!("Failed to load tasks for label: {e}")))?;

        // Set current filter and reset selection
        self.state.current_label_filter = Some(label_name);
        self.state.selected_task_index = 0;
        self.state.build_task_tree();

        // Switch to detail pane to show filtered tasks
        self.state.focused_pane = crate::state::FocusedPane::Detail;

        Ok(())
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        // Ensure terminal is restored even if app panics
        let _ = terminal::restore();
    }
}
