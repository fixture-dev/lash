//! Terminal UI for Lash
//!
//! This crate provides an interactive terminal interface for browsing
//! and managing tasks.

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod app;
pub mod colors;
pub mod components;
mod error;
pub mod event;
pub mod event_source;
pub mod state;
mod terminal;
pub mod testing;
mod ui;
pub mod utils;

pub use app::{TuiApp, TuiAppCore};
pub use error::{TuiError, TuiResult};

/// Run the TUI application
///
/// # Errors
///
/// Returns error if:
/// - Database connection fails
/// - Terminal setup fails
/// - TUI rendering encounters fatal error
pub fn run(db_path: &std::path::Path) -> TuiResult<()> {
    run_with_scheme(db_path, None)
}

/// Run the TUI application with a specific color scheme
///
/// # Errors
///
/// Returns error if:
/// - Database connection fails
/// - Terminal setup fails
/// - TUI rendering encounters fatal error
/// - Color scheme is invalid
pub fn run_with_scheme(db_path: &std::path::Path, color_scheme: Option<&str>) -> TuiResult<()> {
    let mut app = TuiApp::new_with_scheme(db_path, color_scheme)?;
    app.run()
}
