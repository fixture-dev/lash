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

use crate::components::TreeSelectItem;
use crate::error::{TuiError, TuiResult};
use crate::event::{
    poll_confirm_complete_event, poll_confirm_incomplete_event,
    poll_confirm_linked_file_complete_event, poll_event, poll_filter_event, poll_search_event,
    poll_task_creation_event, AppEvent,
};
use crate::state::AppState;
use crate::terminal;
use crate::ui;
use crate::utils;

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
            self.terminal
                .draw(|frame| ui::render(frame, &self.state, &self.conn))?;

            // Handle events - use different polling depending on modal state
            let event = if self.state.is_task_creation_modal_open() {
                poll_task_creation_event(Duration::from_millis(100))?
            } else if self.state.is_search_modal_open() {
                poll_search_event(Duration::from_millis(100))?
            } else if self.state.is_filter_modal_open() {
                poll_filter_event(Duration::from_millis(100))?
            } else if self.state.is_confirm_complete_modal_open() {
                poll_confirm_complete_event(Duration::from_millis(100))?
            } else if self.state.is_confirm_incomplete_modal_open() {
                poll_confirm_incomplete_event(Duration::from_millis(100))?
            } else if self.state.is_confirm_linked_file_complete_modal_open() {
                poll_confirm_linked_file_complete_event(Duration::from_millis(100))?
            } else {
                poll_event(Duration::from_millis(100))?
            };

            // Route events based on active modal
            #[allow(clippy::match_same_arms)] // Placeholder arms for future features
            if self.state.is_task_creation_modal_open() {
                // Task creation modal is open - route events to it
                match event {
                    AppEvent::CloseTaskCreation => {
                        self.state.close_task_creation_modal();
                    }
                    AppEvent::SubmitTaskCreation => {
                        self.handle_submit_task_creation()?;
                    }
                    AppEvent::TaskFormNextField => {
                        if let Some(modal) = &mut self.state.task_creation_modal_state {
                            modal.next_field();
                        }
                    }
                    AppEvent::TaskFormPrevField => {
                        if let Some(modal) = &mut self.state.task_creation_modal_state {
                            modal.prev_field();
                        }
                    }
                    AppEvent::TaskFormTogglePreview => {
                        if let Some(modal) = &mut self.state.task_creation_modal_state {
                            modal.toggle_preview();
                        }
                    }
                    AppEvent::CharInput(c) => {
                        self.handle_char_input_in_modal(c);
                        self.validate_modal();
                    }
                    AppEvent::Backspace => {
                        self.handle_backspace_in_modal();
                        self.validate_modal();
                    }
                    AppEvent::Delete => {
                        self.handle_delete_in_modal();
                        self.validate_modal();
                    }
                    AppEvent::Left => {
                        self.handle_left_in_modal();
                    }
                    AppEvent::Right => {
                        self.handle_right_in_modal();
                    }
                    AppEvent::Up => {
                        self.handle_up_in_modal();
                    }
                    AppEvent::Down => {
                        self.handle_down_in_modal();
                    }
                    AppEvent::Home => {
                        self.handle_home_in_modal();
                    }
                    AppEvent::End => {
                        self.handle_end_in_modal();
                    }
                    AppEvent::Select => {
                        self.handle_select_in_modal();
                        self.validate_modal();
                    }
                    AppEvent::ClearFilters => {
                        self.handle_clear_field_in_modal();
                        self.validate_modal();
                    }
                    AppEvent::Help => {
                        // Toggle help overlay within modal
                        self.state.show_help = !self.state.show_help;
                    }
                    _ => {} // Ignore other events when task creation modal is open
                }
            } else if self.state.is_search_modal_open() {
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
            } else if self.state.is_confirm_complete_modal_open() {
                // Confirm complete modal is open - route events to it
                match event {
                    AppEvent::CloseConfirmComplete => {
                        self.state.close_confirm_complete_modal();
                    }
                    AppEvent::ConfirmComplete => {
                        self.handle_confirm_cascading_complete()?;
                    }
                    _ => {} // Ignore other events when confirm modal is open
                }
            } else if self.state.is_confirm_incomplete_modal_open() {
                // Confirm incomplete modal is open - route events to it
                match event {
                    AppEvent::CloseConfirmIncomplete => {
                        self.state.close_confirm_incomplete_modal();
                    }
                    AppEvent::ConfirmIncomplete => {
                        self.handle_confirm_cascading_incomplete()?;
                    }
                    _ => {} // Ignore other events when confirm modal is open
                }
            } else if self.state.is_confirm_linked_file_complete_modal_open() {
                // Confirm linked file complete modal is open - route events to it
                match event {
                    AppEvent::CloseConfirmLinkedFileComplete => {
                        self.state.close_confirm_linked_file_complete_modal();
                    }
                    AppEvent::ConfirmLinkedFileComplete => {
                        self.handle_confirm_linked_file_complete()?;
                    }
                    _ => {} // Ignore other events when confirm modal is open
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

                    AppEvent::OpenTaskCreation => self.handle_open_task_creation()?,
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
                    | AppEvent::CloseConfirmComplete
                    | AppEvent::ConfirmComplete
                    | AppEvent::CloseConfirmIncomplete
                    | AppEvent::ConfirmIncomplete
                    | AppEvent::CloseConfirmLinkedFileComplete
                    | AppEvent::ConfirmLinkedFileComplete
                    | AppEvent::CloseTaskCreation
                    | AppEvent::SubmitTaskCreation
                    | AppEvent::TaskFormNextField
                    | AppEvent::TaskFormPrevField
                    | AppEvent::TaskFormTogglePreview
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
                    tracing::debug!(
                        "Task node: has_children={}, is_expanded={}",
                        task_node.has_children,
                        task_node.is_expanded
                    );

                    // Check for cross-file link navigation FIRST (takes priority over expansion)
                    // This ensures pressing Enter on a cross-file link navigates to the target
                    // even if the task has subtasks
                    let is_index = self.is_viewing_index_file();
                    tracing::debug!("is_viewing_index_file={}", is_index);
                    if is_index {
                        if let Some(task) = self.state.selected_task() {
                            tracing::debug!(
                                "Selected task: id={}, title={:?}",
                                task.id,
                                task.title
                            );
                            if let Some(target) = utils::get_link_target(&self.conn, task.id) {
                                tracing::debug!(
                                    "Navigating to target: file_id={}, task_id={:?}",
                                    target.file_id,
                                    target.task_id
                                );
                                return self.navigate_to_file(target.file_id, target.task_id);
                            }
                            tracing::debug!("get_link_target returned None");
                        }
                    }

                    // Then check for children expansion
                    if task_node.has_children && !task_node.is_expanded {
                        tracing::debug!("Expanding task node");
                        self.state.toggle_selected_task_node();
                    } else {
                        // Fall back to showing modal
                        tracing::debug!("Falling back to modal");
                        self.open_task_detail_for_selected()?;
                    }
                } else {
                    // No tree view - check for cross-file link before showing modal
                    tracing::debug!("No task tree node");
                    if self.is_viewing_index_file() {
                        if let Some(task) = self.state.selected_task() {
                            tracing::debug!(
                                "Selected task (no tree): id={}, title={:?}",
                                task.id,
                                task.title
                            );
                            if let Some(target) = utils::get_link_target(&self.conn, task.id) {
                                return self.navigate_to_file(target.file_id, target.task_id);
                            }
                        }
                    }
                    // Fall back to showing modal
                    self.open_task_detail_for_selected()?;
                }
            }
        }
        Ok(())
    }

    /// Refresh project statistics (total/completed task counts)
    ///
    /// Queries the database for current project-wide task counts and updates
    /// the state. Call this after any operation that changes task status.
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    fn refresh_project_stats(&mut self) -> TuiResult<()> {
        let task_repo = TaskRepository::new(&self.conn);
        let (total_tasks, completed_tasks) = task_repo
            .get_project_counts()
            .map_err(|e| TuiError::App(format!("Failed to refresh project stats: {e}")))?;

        self.state.project_stats.total_tasks = total_tasks;
        self.state.project_stats.completed_tasks = completed_tasks;

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
    #[allow(clippy::too_many_lines)]
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

        // If transitioning to Done, check for special cases
        if old_status == lash_types::TaskStatus::Open && new_status == lash_types::TaskStatus::Done
        {
            // First check if this is a cross-file link task in an index file
            // This takes priority because it has broader implications
            if self.is_viewing_index_file() && utils::is_cross_file_link(&self.conn, task_id) {
                // Get the target file and its open tasks
                if let Some(target) = utils::get_link_target(&self.conn, task_id) {
                    // Get the target file record
                    let file_repo = FileRepository::new(&self.conn);
                    if let Ok(Some(target_file_record)) = file_repo.get_by_db_id(target.file_id) {
                        // Get all tasks in the target file
                        let task_repo = TaskRepository::new(&self.conn);
                        if let Ok(target_tasks) = task_repo.get_by_file(target.file_id) {
                            // Filter to only open (incomplete) tasks
                            let open_tasks: Vec<_> = target_tasks
                                .into_iter()
                                .filter(|t| !t.status.is_complete())
                                .collect();

                            // If there are open tasks in the target file, show confirmation modal
                            if !open_tasks.is_empty() {
                                self.state.open_confirm_linked_file_complete_modal(
                                    task.clone(),
                                    file_path,
                                    target_file_record,
                                    open_tasks,
                                );
                                return Ok(());
                            }
                        }
                    }
                }
            }

            // Then check for open subtasks (within-file cascading)
            let task_repo = TaskRepository::new(&self.conn);
            let descendants = task_repo
                .get_descendants(task_id)
                .map_err(|e| TuiError::App(format!("Failed to get subtasks: {e}")))?;

            // Filter to only open (incomplete) subtasks
            let open_subtasks: Vec<_> = descendants
                .into_iter()
                .filter(|t| !t.status.is_complete())
                .collect();

            // If there are open subtasks, show confirmation modal
            if !open_subtasks.is_empty() {
                self.state
                    .open_confirm_complete_modal(task.clone(), file_path, open_subtasks);
                return Ok(());
            }
        }

        // If transitioning to incomplete (Open), check for completed ancestors
        // This applies when: (Done -> Waived -> Open) or (Blocked -> Open)
        // We only care about the case where a completed task becomes incomplete
        if old_status.is_complete() && new_status == lash_types::TaskStatus::Open {
            // Get all ancestors of this task
            let task_repo = TaskRepository::new(&self.conn);
            let ancestors = task_repo
                .get_ancestors(task_id)
                .map_err(|e| TuiError::App(format!("Failed to get ancestors: {e}")))?;

            // Filter to only completed (Done) ancestors
            // Waived ancestors don't need to be changed since they're intentionally skipped
            let completed_ancestors: Vec<_> = ancestors
                .into_iter()
                .filter(|t| t.status == lash_types::TaskStatus::Done)
                .collect();

            // If there are completed ancestors, show confirmation modal
            if !completed_ancestors.is_empty() {
                self.state.open_confirm_incomplete_modal(
                    task.clone(),
                    file_path,
                    completed_ancestors,
                );
                return Ok(());
            }
        }

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

        // Refresh project stats to update progress bar
        self.refresh_project_stats()?;

        Ok(())
    }

    /// Handle confirmed cascading completion
    ///
    /// Called when user confirms marking a task with open subtasks as complete.
    /// Marks the parent task and all open subtasks as Done.
    fn handle_confirm_cascading_complete(&mut self) -> TuiResult<()> {
        // Take the modal state (this also closes the modal)
        let Some(modal_state) = self.state.confirm_complete_modal_state.take() else {
            return Ok(());
        };

        let parent_task = modal_state.task;
        let file_path = modal_state.file_path;
        let open_subtasks = modal_state.open_subtasks;

        // Get file_id for reloading tasks later
        let file_id = parent_task.file_id;

        // Count for feedback message
        let subtask_count = open_subtasks.len();

        // Update parent task to Done in database
        self.conn
            .execute(
                "UPDATE tasks SET status = ?1 WHERE id = ?2",
                (lash_types::TaskStatus::Done.as_str(), parent_task.id),
            )
            .map_err(|e| TuiError::App(format!("Failed to update parent task: {e}")))?;

        // Update parent task in markdown
        if let Err(e) = self.update_markdown_task_status(
            &file_path,
            &parent_task.title,
            lash_types::TaskStatus::Open,
            lash_types::TaskStatus::Done,
        ) {
            self.state.set_warning_message(format!(
                "Parent DB updated, but markdown update failed: {e}"
            ));
        }

        // Update all open subtasks to Done
        for subtask in &open_subtasks {
            // Update in database
            self.conn
                .execute(
                    "UPDATE tasks SET status = ?1 WHERE id = ?2",
                    (lash_types::TaskStatus::Done.as_str(), subtask.id),
                )
                .map_err(|e| TuiError::App(format!("Failed to update subtask: {e}")))?;

            // Update in markdown
            if let Err(e) = self.update_markdown_task_status(
                &file_path,
                &subtask.title,
                subtask.status, // Use actual status (could be Open or Blocked)
                lash_types::TaskStatus::Done,
            ) {
                // Log warning but continue with other subtasks
                self.state.set_warning_message(format!(
                    "Subtask '{}' markdown update failed: {e}",
                    subtask.title
                ));
            }
        }

        // Show success message
        self.state.set_success_message(format!(
            "Marked task and {} subtask{} as complete",
            subtask_count,
            if subtask_count == 1 { "" } else { "s" }
        ));

        // Preserve expansion state before rebuilding tree
        let expanded_ids = self.state.collect_expansion_state();

        // Reload tasks for the file
        let task_repo = TaskRepository::new(&self.conn);
        self.state.tasks = task_repo
            .get_by_file(file_id)
            .map_err(|e| TuiError::App(format!("Failed to reload tasks: {e}")))?;

        // Rebuild task tree to reflect updated status
        self.state.build_task_tree();

        // Restore expansion state after rebuild
        self.state.restore_expansion_state(&expanded_ids);

        // Refresh project stats to update progress bar
        self.refresh_project_stats()?;

        Ok(())
    }

    /// Handle confirmed linked file complete
    ///
    /// Called when user confirms marking a cross-file link task as complete.
    /// Marks the link task in the index file as Done (both DB and markdown),
    /// and marks all open tasks in the target file as Done (both DB and markdown).
    fn handle_confirm_linked_file_complete(&mut self) -> TuiResult<()> {
        // Take the modal state (this also closes the modal)
        let Some(modal_state) = self.state.confirm_linked_file_complete_modal_state.take() else {
            return Ok(());
        };

        let link_task = modal_state.link_task;
        let index_file_path = modal_state.index_file_path;
        let target_file = modal_state.target_file;
        let total_open_count = modal_state.total_open_count;

        // Get file_id for reloading tasks later
        let index_file_id = link_task.file_id;

        // Update link task to Done in database
        self.conn
            .execute(
                "UPDATE tasks SET status = ?1 WHERE id = ?2",
                (lash_types::TaskStatus::Done.as_str(), link_task.id),
            )
            .map_err(|e| TuiError::App(format!("Failed to update link task: {e}")))?;

        // Update link task in markdown (index file)
        if let Err(e) = self.update_markdown_task_status(
            &index_file_path,
            &link_task.title,
            lash_types::TaskStatus::Open,
            lash_types::TaskStatus::Done,
        ) {
            self.state.set_warning_message(format!(
                "Link task DB updated, but markdown update failed: {e}"
            ));
        }

        // Get all open tasks from target file (not just the truncated list)
        let task_repo = TaskRepository::new(&self.conn);
        let all_target_tasks = task_repo
            .get_by_file(target_file.id)
            .map_err(|e| TuiError::App(format!("Failed to get target file tasks: {e}")))?;

        let all_open_tasks: Vec<_> = all_target_tasks
            .into_iter()
            .filter(|t| !t.status.is_complete())
            .collect();

        // Update all open tasks in target file to Done
        for target_task in &all_open_tasks {
            // Update in database
            self.conn
                .execute(
                    "UPDATE tasks SET status = ?1 WHERE id = ?2",
                    (lash_types::TaskStatus::Done.as_str(), target_task.id),
                )
                .map_err(|e| TuiError::App(format!("Failed to update target task: {e}")))?;

            // Update in markdown (target file)
            if let Err(e) = self.update_markdown_task_status(
                &target_file.path,
                &target_task.title,
                target_task.status, // Use actual status (could be Open or Blocked)
                lash_types::TaskStatus::Done,
            ) {
                // Log warning but continue with other tasks
                self.state.set_warning_message(format!(
                    "Task '{}' markdown update failed: {e}",
                    target_task.title
                ));
            }
        }

        // Show success message
        self.state.set_success_message(format!(
            "Marked link task and {} task{} in {} as complete",
            total_open_count,
            if total_open_count == 1 { "" } else { "s" },
            target_file
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("linked file")
        ));

        // Preserve expansion state before rebuilding tree
        let expanded_ids = self.state.collect_expansion_state();

        // Reload tasks for the index file (where we are viewing)
        let task_repo = TaskRepository::new(&self.conn);
        self.state.tasks = task_repo
            .get_by_file(index_file_id)
            .map_err(|e| TuiError::App(format!("Failed to reload tasks: {e}")))?;

        // Rebuild task tree to reflect updated status
        self.state.build_task_tree();

        // Restore expansion state after rebuild
        self.state.restore_expansion_state(&expanded_ids);

        // Refresh project stats to update progress bar
        self.refresh_project_stats()?;

        Ok(())
    }

    /// Handle confirmed cascading incomplete
    ///
    /// Called when user confirms marking a completed subtask as incomplete
    /// when its parent is also complete. Marks the subtask and all completed
    /// ancestors as Open.
    fn handle_confirm_cascading_incomplete(&mut self) -> TuiResult<()> {
        // Take the modal state (this also closes the modal)
        let Some(modal_state) = self.state.confirm_incomplete_modal_state.take() else {
            return Ok(());
        };

        let subtask = modal_state.task;
        let file_path = modal_state.file_path;
        let completed_ancestors = modal_state.completed_ancestors;

        // Get file_id for reloading tasks later
        let file_id = subtask.file_id;

        // Count for feedback message
        let ancestor_count = completed_ancestors.len();

        // Determine the old status of the subtask
        // The transition to Open happens from Waived or Blocked
        let old_status = subtask.status;

        // Update subtask to Open in database
        self.conn
            .execute(
                "UPDATE tasks SET status = ?1 WHERE id = ?2",
                (lash_types::TaskStatus::Open.as_str(), subtask.id),
            )
            .map_err(|e| TuiError::App(format!("Failed to update subtask: {e}")))?;

        // Update subtask in markdown
        if let Err(e) = self.update_markdown_task_status(
            &file_path,
            &subtask.title,
            old_status,
            lash_types::TaskStatus::Open,
        ) {
            self.state
                .set_warning_message(format!("Subtask DB updated, but markdown failed: {e}"));
        }

        // Update all completed ancestors to Open
        for ancestor in &completed_ancestors {
            // Update in database
            self.conn
                .execute(
                    "UPDATE tasks SET status = ?1 WHERE id = ?2",
                    (lash_types::TaskStatus::Open.as_str(), ancestor.id),
                )
                .map_err(|e| TuiError::App(format!("Failed to update ancestor: {e}")))?;

            // Update in markdown (ancestors are Done, so old_status is Done)
            if let Err(e) = self.update_markdown_task_status(
                &file_path,
                &ancestor.title,
                lash_types::TaskStatus::Done,
                lash_types::TaskStatus::Open,
            ) {
                // Log warning but continue with other ancestors
                self.state.set_warning_message(format!(
                    "Ancestor '{}' markdown update failed: {e}",
                    ancestor.title
                ));
            }
        }

        // Show success message
        self.state.set_success_message(format!(
            "Marked task and {} parent{} as incomplete",
            ancestor_count,
            if ancestor_count == 1 { "" } else { "s" }
        ));

        // Preserve expansion state before rebuilding tree
        let expanded_ids = self.state.collect_expansion_state();

        // Reload tasks for the file
        let task_repo = TaskRepository::new(&self.conn);
        self.state.tasks = task_repo
            .get_by_file(file_id)
            .map_err(|e| TuiError::App(format!("Failed to reload tasks: {e}")))?;

        // Rebuild task tree to reflect updated status
        self.state.build_task_tree();

        // Restore expansion state after rebuild
        self.state.restore_expansion_state(&expanded_ids);

        // Refresh project stats to update progress bar
        self.refresh_project_stats()?;

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

    /// Check if the currently viewed file (whose tasks are displayed) is an index file
    ///
    /// Index files are named `lash.index.md` or `index.lash.md`.
    ///
    /// This uses the same logic as task loading: in tree view mode, we check the
    /// tree node's file; otherwise we fall back to the flat file list selection.
    fn is_viewing_index_file(&self) -> bool {
        // In tree view mode, check the tree node's file (matches task loading logic)
        if let Some(node) = self.state.selected_tree_node() {
            if let Some(file) = node.file_record.as_ref() {
                let is_index = lash_core::display::is_index_file(&file.path);
                tracing::debug!(
                    "is_viewing_index_file (tree view): path={:?}, is_index={}",
                    file.path,
                    is_index
                );
                return is_index;
            }
            tracing::debug!("is_viewing_index_file: tree node has no file_record");
        }
        // Fall back to flat list selection
        let result = self
            .state
            .selected_file()
            .is_some_and(|f| lash_core::display::is_index_file(&f.path));
        tracing::debug!("is_viewing_index_file (flat list): result={}", result);
        result
    }

    /// Navigate to a target file from a cross-file link.
    ///
    /// This expands the file tree to reveal the target file, selects it,
    /// loads its tasks, and optionally selects a specific task.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Target file is not found in the file list
    /// - Failed to load tasks for the target file
    fn navigate_to_file(
        &mut self,
        target_file_id: i64,
        target_task_id: Option<i64>,
    ) -> TuiResult<()> {
        // Expand path and get flat file index
        let flat_file_index = match self.state.expand_path_to_file(target_file_id) {
            Ok(index) => index,
            Err(e) => {
                self.state
                    .set_error_message(format!("Navigation failed: {e}"));
                return Ok(());
            }
        };

        // Get the filename for the status message
        let filename = self.state.files[flat_file_index]
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // In tree view mode, we need the visual index (position in flattened visible tree)
        // In flat list mode, we use the flat file index directly
        let selected_index = if self.state.file_tree.is_some() {
            // Get visual index in tree view (after path expansion)
            self.state
                .visual_index_of_file(target_file_id)
                .unwrap_or(flat_file_index)
        } else {
            flat_file_index
        };

        // Update selected file index
        self.state.selected_file_index = selected_index;

        // Load tasks for target file
        let task_repo = TaskRepository::new(&self.conn);
        self.state.tasks = task_repo
            .get_by_file(target_file_id)
            .map_err(|e| TuiError::App(format!("Failed to load tasks for target file: {e}")))?;

        // Build task tree if tree view enabled
        self.state.build_task_tree();

        // If target_task_id provided, find and select that task index
        if let Some(task_id) = target_task_id {
            if let Some(task_index) = self.state.tasks.iter().position(|t| t.id == task_id) {
                self.state.selected_task_index = task_index;
            }
        } else {
            // No specific task - select first task
            self.state.selected_task_index = 0;
        }

        // Set success status message
        self.state
            .set_success_message(format!("Navigated to {filename}"));

        // Switch focus to detail pane
        self.state.focused_pane = crate::state::FocusedPane::Detail;

        Ok(())
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

    /// Handle opening task creation modal
    #[allow(clippy::unnecessary_wraps)]
    fn handle_open_task_creation(&mut self) -> TuiResult<()> {
        use lash_db::repository::LabelRepository;

        // Get currently selected file
        let target_file = if let Some(selected) = self.state.selected_tree_node() {
            selected.file_record.map(|f| f.path)
        } else {
            self.state.selected_file().map(|f| f.path.clone())
        };

        let Some(target_file) = target_file else {
            self.state.set_warning_message("No file selected");
            return Ok(());
        };

        // Get tasks from current file for parent selection
        let tasks: Vec<TreeSelectItem> = self
            .state
            .tasks
            .iter()
            .map(|t| TreeSelectItem {
                id: t.local_id.clone(),
                title: t.title.clone(),
                depth: t.depth,
                status_indicator: match t.status {
                    lash_types::TaskStatus::Done => 'x',
                    lash_types::TaskStatus::Waived => '-',
                    lash_types::TaskStatus::Blocked => '!',
                    lash_types::TaskStatus::Open => ' ',
                },
            })
            .collect();

        self.state.open_task_creation_modal(target_file, tasks);

        // Fetch label stats for autocomplete
        if let Some(modal_state) = &mut self.state.task_creation_modal_state {
            let label_repo = LabelRepository::new(&self.conn);
            match label_repo.get_label_stats() {
                Ok(label_stats) => {
                    modal_state.set_label_suggestions(label_stats);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load label stats: {e}");
                }
            }
        }

        Ok(())
    }

    /// Handle submitting task creation
    fn handle_submit_task_creation(&mut self) -> TuiResult<()> {
        use lash_core::creation::TaskCreationService;
        use lash_db::{Indexer, IndexerConfig};
        use lash_types::LashConfig;

        let Some(modal_state) = &self.state.task_creation_modal_state else {
            return Ok(());
        };

        // Check if form can be submitted
        if !modal_state.can_submit() {
            self.state
                .set_error_message("Please fill in all required fields");
            return Ok(());
        }

        let request = modal_state.to_request();
        let config = LashConfig::default();
        let service = TaskCreationService::new(config.clone());

        match service.create_task(&request) {
            Ok(result) => {
                self.state.close_task_creation_modal();

                // Re-index to update the database with the new task
                // Use incremental indexing for efficiency
                let indexer_config =
                    IndexerConfig::new(self.project_root.clone()).with_incremental(true);
                let mut indexer = Indexer::new(&self.conn, indexer_config, &config);

                if let Err(e) = indexer.index_project() {
                    self.state
                        .set_warning_message(format!("Task created but indexing failed: {e}"));
                } else {
                    self.state
                        .set_success_message(format!("Created task: {}", result.task_id));
                }

                // Reload tasks for the file (now that it's indexed)
                let file_repo = FileRepository::new(&self.conn);
                if let Ok(Some(file_record)) = file_repo.get_by_path(&result.file_path) {
                    let task_repo = TaskRepository::new(&self.conn);
                    if let Ok(tasks) = task_repo.get_by_file(file_record.id) {
                        self.state.tasks = tasks;
                        self.state.build_task_tree();
                    }
                }

                // Refresh project stats
                self.refresh_project_stats()?;

                // Also refresh the file list in case a new file was created
                let file_repo = FileRepository::new(&self.conn);
                if let Ok(files) = file_repo.list_all() {
                    self.state.files = files;
                    self.state.build_file_tree();
                }
            }
            Err(errors) => {
                // Display first error
                let error_msg = errors.first().map_or_else(
                    || "Unknown error".to_string(),
                    lash_types::TaskCreationError::message,
                );
                self.state.set_error_message(error_msg);
            }
        }

        Ok(())
    }

    /// Handle character input in modal
    fn handle_char_input_in_modal(&mut self, c: char) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Title => modal.title.input_char(c),
            TaskFormField::Labels => modal.labels.input_char(c),
            TaskFormField::Owner => modal.owner.input_char(c),
            TaskFormField::Estimate => modal.estimate.input_char(c),
            TaskFormField::AgentNote => modal.agent_note.input_char(c),
            _ => {}
        }
    }

    /// Handle backspace in modal
    fn handle_backspace_in_modal(&mut self) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Title => modal.title.backspace(),
            TaskFormField::Labels => modal.labels.backspace(),
            TaskFormField::Owner => modal.owner.backspace(),
            TaskFormField::Estimate => modal.estimate.backspace(),
            TaskFormField::AgentNote => modal.agent_note.backspace(),
            _ => {}
        }
    }

    /// Handle delete in modal
    fn handle_delete_in_modal(&mut self) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Title => modal.title.delete(),
            TaskFormField::Labels => {
                // ChipInputState doesn't have delete, only backspace
                modal.labels.backspace();
            }
            TaskFormField::Owner => modal.owner.delete(),
            TaskFormField::Estimate => modal.estimate.delete(),
            TaskFormField::AgentNote => modal.agent_note.delete(),
            _ => {}
        }
    }

    /// Handle left arrow in modal
    fn handle_left_in_modal(&mut self) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Title => modal.title.cursor_left(),
            TaskFormField::Owner => modal.owner.cursor_left(),
            TaskFormField::Estimate => modal.estimate.cursor_left(),
            TaskFormField::AgentNote => modal.agent_note.cursor_left(),
            TaskFormField::Status => modal.status.select_prev(),
            _ => {}
        }
    }

    /// Handle right arrow in modal
    fn handle_right_in_modal(&mut self) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Title => modal.title.cursor_right(),
            TaskFormField::Owner => modal.owner.cursor_right(),
            TaskFormField::Estimate => modal.estimate.cursor_right(),
            TaskFormField::AgentNote => modal.agent_note.cursor_right(),
            TaskFormField::Status => modal.status.select_next(),
            _ => {}
        }
    }

    /// Handle up arrow in modal
    fn handle_up_in_modal(&mut self) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Parent => modal.parent_selector.select_prev(),
            TaskFormField::AgentNote => modal.agent_note.cursor_up(),
            _ => {}
        }
    }

    /// Handle down arrow in modal
    fn handle_down_in_modal(&mut self) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Parent => modal.parent_selector.select_next(),
            TaskFormField::AgentNote => modal.agent_note.cursor_down(),
            _ => {}
        }
    }

    /// Handle home key in modal
    fn handle_home_in_modal(&mut self) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Title => modal.title.home(),
            TaskFormField::Owner => modal.owner.home(),
            TaskFormField::Estimate => modal.estimate.home(),
            TaskFormField::AgentNote => modal.agent_note.home(),
            _ => {}
        }
    }

    /// Handle end key in modal
    fn handle_end_in_modal(&mut self) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Title => modal.title.end(),
            TaskFormField::Owner => modal.owner.end(),
            TaskFormField::Estimate => modal.estimate.end(),
            TaskFormField::AgentNote => modal.agent_note.end(),
            _ => {}
        }
    }

    /// Handle select/enter in modal
    fn handle_select_in_modal(&mut self) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Parent => modal.parent_selector.confirm_selection(),
            TaskFormField::Labels => modal.labels.add_chip(),
            TaskFormField::AgentNote => modal.agent_note.newline(),
            _ => {}
        }
    }

    /// Handle clear field (Ctrl+U) in modal
    fn handle_clear_field_in_modal(&mut self) {
        use crate::state::TaskFormField;

        let Some(modal) = &mut self.state.task_creation_modal_state else {
            return;
        };

        match modal.focused_field {
            TaskFormField::Title => modal.title.clear(),
            TaskFormField::Labels => {
                modal.labels.input.clear();
                modal.labels.cursor_position = 0;
            }
            TaskFormField::Owner => modal.owner.clear(),
            TaskFormField::Estimate => modal.estimate.clear(),
            TaskFormField::AgentNote => modal.agent_note.clear(),
            TaskFormField::Parent => {
                modal.parent_selector.clear_selection();
            }
            TaskFormField::Status => {
                // Reset to first option (Open)
                modal.status.selected_index = 0;
            }
        }
    }

    /// Validate the modal form fields
    fn validate_modal(&mut self) {
        if let Some(modal) = &mut self.state.task_creation_modal_state {
            modal.validate();
        }
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        // Ensure terminal is restored even if app panics
        let _ = terminal::restore();
    }
}
