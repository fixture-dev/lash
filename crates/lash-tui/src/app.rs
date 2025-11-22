//! Main TUI application

use ratatui::{backend::CrosstermBackend, Terminal};
use rusqlite::Connection;
use std::io;
use std::path::Path;
use std::time::Duration;

use lash_db::repository::{FileRepository, TaskRepository};

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
        let terminal = terminal::setup()?;
        let conn = lash_db::open_database(db_path)?;
        let mut state = AppState::new();

        // Load initial files
        let file_repo = FileRepository::new(&conn);
        state.files = file_repo
            .list_all()
            .map_err(|e| TuiError::App(format!("Failed to load files: {e}")))?;

        // Load tasks for first file if available
        if let Some(file) = state.selected_file() {
            let task_repo = TaskRepository::new(&conn);
            state.tasks = task_repo
                .get_by_file(file.id)
                .map_err(|e| TuiError::App(format!("Failed to load tasks: {e}")))?;
        }

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

            #[allow(clippy::match_same_arms)] // Placeholder arms for future features
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

                // TODO: implement these features
                AppEvent::Left
                | AppEvent::ClearFilters
                | AppEvent::Search
                | AppEvent::LabelFilter
                | AppEvent::DependencyGraph
                | AppEvent::PrevTask
                | AppEvent::NextTask
                | AppEvent::Resize(_, _)
                | AppEvent::None => {}
            }

            if self.state.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle selection/enter in current pane
    fn handle_select(&mut self) -> TuiResult<()> {
        match self.state.focused_pane {
            crate::state::FocusedPane::Navigation => {
                // Load tasks for selected file
                if let Some(file) = self.state.selected_file() {
                    let task_repo = TaskRepository::new(&self.conn);
                    self.state.tasks = task_repo
                        .get_by_file(file.id)
                        .map_err(|e| TuiError::App(format!("Failed to load tasks: {e}")))?;
                    self.state.selected_task_index = 0;
                    self.state.switch_pane();
                }
            }
            crate::state::FocusedPane::Detail => {
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
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        // Ensure terminal is restored even if app panics
        let _ = terminal::restore();
    }
}
