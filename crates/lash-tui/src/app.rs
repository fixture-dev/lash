//! Main TUI application

use ratatui::{backend::CrosstermBackend, Terminal};
use rusqlite::Connection;
use std::io;
use std::path::Path;
use std::time::Duration;

use lash_db::repository::{FileRepository, LabelRepository, TaskRepository};

use crate::error::{TuiError, TuiResult};
use crate::event::{poll_event, AppEvent};
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

        // Load tasks for first file if available
        if let Some(file) = state.selected_file() {
            let task_repo = TaskRepository::new(&conn);
            state.tasks = task_repo
                .get_by_file(file.id)
                .map_err(|e| TuiError::App(format!("Failed to load tasks: {e}")))?;
        }

        // Load labels with stats
        let label_repo = LabelRepository::new(&conn);
        state.labels = label_repo
            .get_label_stats()
            .map_err(|e| TuiError::App(format!("Failed to load labels: {e}")))?;

        Ok(Self {
            terminal,
            conn,
            state,
        })
    }

    /// Run the application event loop
    ///
    /// # Errors
    ///
    /// Returns error if rendering or event handling fails
    pub fn run(&mut self) -> TuiResult<()> {
        loop {
            // Render
            self.terminal.draw(|frame| ui::render(frame, &self.state))?;

            // Handle events
            let event = poll_event(Duration::from_millis(100))?;

            // Route events based on active modal
            #[allow(clippy::match_same_arms)] // Placeholder arms for future features
            if self.state.theme_selector_state.is_some() {
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
                match event {
                    AppEvent::Quit => {
                        self.state.should_quit = true;
                    }
                    AppEvent::Up => self.state.move_up(),
                    AppEvent::Down => self.state.move_down(),
                    AppEvent::Right => self.handle_select()?,
                    AppEvent::Select => self.handle_toggle_status()?,
                    AppEvent::SwitchPane => self.state.switch_pane(),
                    AppEvent::GoTop => self.state.go_top(),
                    AppEvent::GoBottom => self.state.go_bottom(),
                    AppEvent::Help => self.state.toggle_help(),
                    AppEvent::OpenEditor => self.handle_open_editor()?,
                    AppEvent::OpenThemeSelector => self.state.open_theme_selector(),
                    AppEvent::Left => self.handle_left(),
                    AppEvent::ExpandAll => self.handle_expand_all(),
                    AppEvent::CollapseAll => self.handle_collapse_all(),

                    AppEvent::LabelFilter => self.handle_label_toggle()?,
                    AppEvent::ClearFilters => self.state.clear_label_filter(),

                    // TODO: implement these features
                    AppEvent::ExpandNode
                    | AppEvent::CollapseNode
                    | AppEvent::Search
                    | AppEvent::DependencyGraph
                    | AppEvent::PrevTask
                    | AppEvent::NextTask
                    | AppEvent::CloseThemeSelector
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
                                self.state.tasks = task_repo
                                    .get_by_file(file.id)
                                    .map_err(|e| {
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
                                self.state.tasks = task_repo
                                    .get_by_file(file.id)
                                    .map_err(|e| {
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
            FocusedPane::Detail => {
                // Enter on detail pane shows task details
                // TODO: Implement task detail view
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
        // Only works in detail pane on a selected task
        if self.state.focused_pane != crate::state::FocusedPane::Detail {
            return Ok(());
        }

        let task_index = self.state.selected_task_index;
        if task_index >= self.state.tasks.len() {
            return Ok(());
        }

        // Get current task
        let task = &self.state.tasks[task_index];
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
                (new_status.as_str(), task.id),
            )
            .map_err(|e| TuiError::App(format!("Failed to update task status: {e}")))?;

        // Reload tasks to reflect changes
        if let Some(file) = self.state.selected_file() {
            let task_repo = TaskRepository::new(&self.conn);
            self.state.tasks = task_repo
                .get_by_file(file.id)
                .map_err(|e| TuiError::App(format!("Failed to reload tasks: {e}")))?;
        }

        Ok(())
    }

    /// Handle Left event (collapse node or go to parent in tree view)
    #[allow(clippy::unused_self)]
    fn handle_left(&mut self) {
        // TODO: Implement tree navigation for file/task tree
        // For now, no-op (Left is not used in flat list view)
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
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        // Ensure terminal is restored even if app panics
        let _ = terminal::restore();
    }
}
