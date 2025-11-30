//! Event handling for TUI

#![allow(dead_code)] // Some variants reserved for future features

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::error::TuiResult;

/// Application events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    /// Quit the application
    Quit,
    /// Move selection up
    Up,
    /// Move selection down
    Down,
    /// Move selection left (collapse/go to parent)
    Left,
    /// Move selection right (expand/enter)
    Right,
    /// Toggle/select current item
    Select,
    /// Switch panes
    SwitchPane,
    /// Open in editor
    OpenEditor,
    /// Start search
    Search,
    /// Start label filter
    LabelFilter,
    /// Clear filters
    ClearFilters,
    /// Show dependency graph
    DependencyGraph,
    /// Show help
    Help,
    /// Go to top
    GoTop,
    /// Go to bottom
    GoBottom,
    /// Jump to previous top-level task
    PrevTask,
    /// Jump to next top-level task
    NextTask,
    /// Open theme selector
    OpenThemeSelector,
    /// Close theme selector
    CloseThemeSelector,
    /// Expand current node
    ExpandNode,
    /// Collapse current node
    CollapseNode,
    /// Expand all nodes
    ExpandAll,
    /// Collapse all nodes
    CollapseAll,
    /// Resize terminal
    Resize(u16, u16),
    /// No event (timeout)
    None,
}

/// Poll for the next event with timeout
///
/// Returns `AppEvent::None` if no event occurs within the timeout.
pub fn poll_event(timeout: Duration) -> TuiResult<AppEvent> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key) => Ok(handle_key_event(key)),
            Event::Resize(width, height) => Ok(AppEvent::Resize(width, height)),
            _ => Ok(AppEvent::None),
        }
    } else {
        Ok(AppEvent::None)
    }
}

/// Convert key event to application event
fn handle_key_event(key: KeyEvent) -> AppEvent {
    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            AppEvent::Quit
        }

        // Navigation
        (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, _) => AppEvent::Down,
        (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, _) => AppEvent::Up,
        (KeyCode::Char('h'), KeyModifiers::NONE) | (KeyCode::Left, _) => AppEvent::Left,
        (KeyCode::Char('l'), KeyModifiers::NONE) | (KeyCode::Right | KeyCode::Enter, _) => {
            AppEvent::Right
        }

        // Go to top/bottom
        (KeyCode::Char('g'), KeyModifiers::NONE) => AppEvent::GoTop,
        (KeyCode::Char('G'), KeyModifiers::SHIFT) => AppEvent::GoBottom,

        // Task jumps
        (KeyCode::Char('{'), KeyModifiers::SHIFT) => AppEvent::PrevTask,
        (KeyCode::Char('}'), KeyModifiers::SHIFT) => AppEvent::NextTask,

        // Tree expansion (vim-style fold commands)
        (KeyCode::Char('H'), KeyModifiers::SHIFT) => AppEvent::CollapseAll,
        (KeyCode::Char('L'), KeyModifiers::SHIFT) => AppEvent::ExpandAll,

        // Pane switching
        (KeyCode::Tab, _) | (KeyCode::Char('h' | 'l'), KeyModifiers::CONTROL) => {
            AppEvent::SwitchPane
        }

        // Actions
        (KeyCode::Char(' '), KeyModifiers::NONE) => AppEvent::Select,
        (KeyCode::Char('e'), KeyModifiers::NONE) => AppEvent::OpenEditor,
        (KeyCode::Char('/'), KeyModifiers::NONE) => AppEvent::Search,
        (KeyCode::Char('F'), KeyModifiers::SHIFT) => AppEvent::LabelFilter,
        (KeyCode::Char('c'), KeyModifiers::NONE) => AppEvent::ClearFilters,
        (KeyCode::Char('g'), KeyModifiers::CONTROL) => AppEvent::DependencyGraph,
        (KeyCode::Char('?'), KeyModifiers::SHIFT) => AppEvent::Help,
        (KeyCode::Char('t'), KeyModifiers::NONE) => AppEvent::OpenThemeSelector,
        (KeyCode::Esc, _) => AppEvent::CloseThemeSelector,

        _ => AppEvent::None,
    }
}
