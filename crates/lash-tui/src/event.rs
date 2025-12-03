//! Event handling for TUI

#![allow(dead_code)] // Some variants reserved for future features

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::error::TuiResult;

/// Application events
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Character input (for search)
    CharInput(char),
    /// Backspace key
    Backspace,
    /// Delete key
    Delete,
    /// Home key
    Home,
    /// End key
    End,
    /// Execute search (Enter in search mode)
    ExecuteSearch,
    /// Close search modal
    CloseSearch,
    /// Open filter modal
    OpenFilter,
    /// Close filter modal
    CloseFilter,
    /// Apply selected filter
    ApplyFilter,
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

/// Poll for search input events
///
/// When in search input mode, key events are handled differently:
/// - Most keys are passed as character input
/// - Enter executes the search
/// - Escape closes the search modal
/// - Navigation keys for result list
pub fn poll_search_event(timeout: Duration) -> TuiResult<AppEvent> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key) => Ok(handle_search_key_event(key)),
            Event::Resize(width, height) => Ok(AppEvent::Resize(width, height)),
            _ => Ok(AppEvent::None),
        }
    } else {
        Ok(AppEvent::None)
    }
}

/// Poll for filter input events
///
/// When in filter modal mode, key events are handled differently:
/// - Most keys are passed as character input for filtering the label list
/// - Enter applies the selected filter
/// - Escape closes the filter modal
/// - Navigation keys for label list
pub fn poll_filter_event(timeout: Duration) -> TuiResult<AppEvent> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key) => Ok(handle_filter_key_event(key)),
            Event::Resize(width, height) => Ok(AppEvent::Resize(width, height)),
            _ => Ok(AppEvent::None),
        }
    } else {
        Ok(AppEvent::None)
    }
}

/// Convert key event to application event for search mode
fn handle_search_key_event(key: KeyEvent) -> AppEvent {
    match (key.code, key.modifiers) {
        // Close search (Esc or Ctrl-C)
        (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => AppEvent::CloseSearch,

        // Execute search
        (KeyCode::Enter, KeyModifiers::NONE) => AppEvent::ExecuteSearch,

        // Navigate results
        (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => AppEvent::Up,
        (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => AppEvent::Down,

        // Cursor movement
        (KeyCode::Left, _) => AppEvent::Left,
        (KeyCode::Right, _) => AppEvent::Right,
        (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => AppEvent::Home,
        (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => AppEvent::End,

        // Text editing
        (KeyCode::Backspace, _) => AppEvent::Backspace,
        (KeyCode::Delete, _) => AppEvent::Delete,

        // Clear input (Ctrl-U)
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => AppEvent::ClearFilters,

        // Character input
        (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => AppEvent::CharInput(c),

        _ => AppEvent::None,
    }
}

/// Convert key event to application event for filter mode
fn handle_filter_key_event(key: KeyEvent) -> AppEvent {
    match (key.code, key.modifiers) {
        // Close filter (Esc or Ctrl-C)
        (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => AppEvent::CloseFilter,

        // Apply filter
        (KeyCode::Enter, KeyModifiers::NONE) => AppEvent::ApplyFilter,

        // Navigate label list
        (KeyCode::Up, _)
        | (KeyCode::Char('p'), KeyModifiers::CONTROL)
        | (KeyCode::Char('k'), KeyModifiers::NONE) => AppEvent::Up,
        (KeyCode::Down, _)
        | (KeyCode::Char('n'), KeyModifiers::CONTROL)
        | (KeyCode::Char('j'), KeyModifiers::NONE) => AppEvent::Down,

        // Text editing (for filtering the label list)
        (KeyCode::Backspace, _) => AppEvent::Backspace,
        (KeyCode::Delete, _) => AppEvent::Delete,

        // Clear input (Ctrl-U)
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => AppEvent::ClearFilters,

        // Character input
        (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => AppEvent::CharInput(c),

        _ => AppEvent::None,
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
        (KeyCode::Char('f'), KeyModifiers::NONE) => AppEvent::OpenFilter,
        (KeyCode::Char('F'), KeyModifiers::SHIFT) => AppEvent::LabelFilter,
        (KeyCode::Char('c'), KeyModifiers::NONE) => AppEvent::ClearFilters,
        (KeyCode::Char('g'), KeyModifiers::CONTROL) => AppEvent::DependencyGraph,
        (KeyCode::Char('?'), _) => AppEvent::Help,
        (KeyCode::Char('t'), KeyModifiers::NONE) => AppEvent::OpenThemeSelector,
        (KeyCode::Esc, _) => AppEvent::CloseThemeSelector,

        _ => AppEvent::None,
    }
}
