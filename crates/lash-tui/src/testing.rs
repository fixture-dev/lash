//! Testing utilities for headless TUI tests
//!
//! This module provides helpers for creating test TUI applications that use
//! synthetic events instead of real terminal I/O.

use crossterm::event::Event;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::path::Path;

use crate::app::TuiAppCore;
use crate::error::{TuiError, TuiResult};
use crate::event_source::TestEventSource;
use crate::state::AppState;

/// Test TUI application type that uses a test backend and synthetic events
pub type TestTuiApp = TuiAppCore<TestBackend, TestEventSource>;

/// Builder for creating test TUI applications
pub struct TestAppBuilder {
    db_path: Option<std::path::PathBuf>,
    events: Vec<Event>,
    width: u16,
    height: u16,
    color_scheme: Option<String>,
}

impl TestAppBuilder {
    /// Create a new test app builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            db_path: None,
            events: Vec::new(),
            width: 80,
            height: 24,
            color_scheme: None,
        }
    }

    /// Set the database path
    #[must_use]
    pub fn with_db(mut self, db_path: impl AsRef<Path>) -> Self {
        self.db_path = Some(db_path.as_ref().to_path_buf());
        self
    }

    /// Set the terminal size
    #[must_use]
    pub fn with_size(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Add events to the event queue
    #[must_use]
    pub fn with_events(mut self, events: Vec<Event>) -> Self {
        self.events = events;
        self
    }

    /// Add a single event
    #[must_use]
    pub fn with_event(mut self, event: Event) -> Self {
        self.events.push(event);
        self
    }

    /// Set the color scheme
    #[must_use]
    pub fn with_color_scheme(mut self, scheme: impl Into<String>) -> Self {
        self.color_scheme = Some(scheme.into());
        self
    }

    /// Build the test TUI app
    ///
    /// # Errors
    ///
    /// Returns error if database connection fails or if initialization fails.
    pub fn build(self) -> TuiResult<TestTuiApp> {
        use lash_db::repository::{
            files::FileRepository, labels::LabelRepository, tasks::TaskRepository,
        };

        let db_path = self
            .db_path
            .ok_or_else(|| TuiError::App("Database path is required for test app".to_string()))?;

        let backend = TestBackend::new(self.width, self.height);
        let terminal = Terminal::new(backend)?;
        let event_source = TestEventSource::new(self.events);
        let conn = lash_db::open_database(&db_path)?;

        // Calculate project root from db_path
        let project_root = db_path
            .parent()
            .and_then(|p| p.parent())
            .map_or_else(|| std::path::PathBuf::from("."), Path::to_path_buf);

        let mut state = if let Some(scheme_name) = self.color_scheme {
            AppState::with_color_scheme(&scheme_name)
                .map_err(|e| TuiError::App(format!("Invalid color scheme: {e}")))?
        } else {
            AppState::new()
        };

        // Load initial data (same as TuiApp::new_with_scheme)

        let file_repo = FileRepository::new(&conn);
        state.files = file_repo
            .list_all()
            .map_err(|e| TuiError::App(format!("Failed to load files: {e}")))?;

        state.build_file_tree();

        let task_repo = TaskRepository::new(&conn);
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

        let label_repo = LabelRepository::new(&conn);
        state.labels = label_repo
            .get_label_stats()
            .map_err(|e| TuiError::App(format!("Failed to load labels: {e}")))?;

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

        // Seed activity.in_progress from the DB exactly as production startup
        // does. Skipping this would mean tests see a stale empty slot when a
        // task is already in-progress on disk at app start.
        if let Ok(mut in_progress_tasks) =
            task_repo.find_by_status(lash_types::TaskStatus::InProgress)
        {
            if let Some(first) = in_progress_tasks.drain(..).next() {
                state
                    .activity
                    .set_in_progress(crate::activity::ActivityEntry {
                        full_id: first.full_id,
                        title: first.title,
                        at: std::time::Instant::now(),
                    });
            }
        }

        Ok(TestTuiApp::new_core(
            terminal,
            event_source,
            conn,
            state,
            project_root,
        ))
    }
}

impl Default for TestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Capture the terminal buffer as a string for snapshot testing
///
/// This converts the terminal's buffer into a string representation
/// that can be used with snapshot testing tools like insta.
#[must_use]
pub fn capture_buffer(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let width = buffer.area.width;
    let height = buffer.area.height;

    let mut result =
        String::with_capacity(usize::from(width) * usize::from(height) + usize::from(height));
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            result.push_str(cell.symbol());
        }
        result.push('\n');
    }
    result
}

/// Helper module for creating common keyboard events
pub mod keys {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    /// Create a key event for a character
    #[must_use]
    pub fn char(c: char) -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
    }

    /// Create a key event for a character with modifiers
    #[must_use]
    pub fn char_with_mods(c: char, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char(c), modifiers))
    }

    /// Create a key event for Enter
    #[must_use]
    pub fn enter() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    }

    /// Create a key event for Escape
    #[must_use]
    pub fn esc() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
    }

    /// Create a key event for Tab
    #[must_use]
    pub fn tab() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
    }

    /// Create a key event for Backspace
    #[must_use]
    pub fn backspace() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
    }

    /// Create a key event for up arrow
    #[must_use]
    pub fn up() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
    }

    /// Create a key event for down arrow
    #[must_use]
    pub fn down() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
    }

    /// Create a key event for left arrow
    #[must_use]
    pub fn left() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))
    }

    /// Create a key event for right arrow
    #[must_use]
    pub fn right() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
    }

    /// Create a key event for Ctrl+C (quit)
    #[must_use]
    pub fn ctrl_c() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
    }
}
