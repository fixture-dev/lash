//! Terminal UI for Lash
//!
//! This crate provides an interactive terminal interface for browsing
//! and managing tasks.

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod app;
mod error;
mod event;
mod state;
mod terminal;
mod ui;

pub use app::TuiApp;
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
    let mut app = TuiApp::new(db_path)?;
    app.run()
}
