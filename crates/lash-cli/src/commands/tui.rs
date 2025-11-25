//! TUI command - Launch interactive terminal UI

use anyhow::{Context, Result};
use lash_db::find_project_root;
use std::path::PathBuf;

/// Arguments for the TUI command
pub struct TuiArgs {
    /// Project root (if None, auto-detect)
    pub project_root: Option<PathBuf>,
    /// Color scheme to use (overrides user config)
    pub color_scheme: Option<String>,
}

/// Execute the TUI command
///
/// # Errors
///
/// Returns error if:
/// - Project root cannot be found
/// - Database cannot be opened
/// - TUI fails to launch
pub fn execute(args: &TuiArgs) -> Result<()> {
    // Find project root
    let project_root = match &args.project_root {
        Some(root) => root.clone(),
        None => find_project_root()
            .context("Could not find project root. Run 'lash index' first or specify --root")?,
    };

    // Get database path
    let db_path = project_root.join(".lash/lash.db");

    // Check if database exists
    if !db_path.exists() {
        anyhow::bail!(
            "Database not found at {}. Run 'lash index' first to index your project.",
            db_path.display()
        );
    }

    // Run the TUI with optional color scheme
    if let Some(scheme) = &args.color_scheme {
        lash_tui::run_with_scheme(&db_path, Some(scheme.as_str()))
            .context("TUI execution failed")?;
    } else {
        lash_tui::run(&db_path).context("TUI execution failed")?;
    }

    Ok(())
}
