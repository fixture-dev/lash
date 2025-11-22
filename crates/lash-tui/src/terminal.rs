//! Terminal setup and teardown utilities

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::panic;

use crate::error::TuiResult;

/// Setup terminal for TUI
///
/// - Enables raw mode
/// - Enters alternate screen
/// - Sets up panic hook for proper cleanup
pub fn setup() -> TuiResult<Terminal<CrosstermBackend<io::Stdout>>> {
    // Set up panic hook to restore terminal
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal before showing panic
        let _ = restore();
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    Ok(terminal)
}

/// Restore terminal to normal state
///
/// - Leaves alternate screen
/// - Disables raw mode
pub fn restore() -> TuiResult<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
