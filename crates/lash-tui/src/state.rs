//! Application state management

#![allow(dead_code)] // Some fields/variants reserved for future features

use lash_db::repository::files::FileRecord;
use lash_db::repository::tasks::TaskRecord;

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
}

impl AppState {
    /// Create new application state
    #[must_use]
    pub fn new() -> Self {
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
        }
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
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
