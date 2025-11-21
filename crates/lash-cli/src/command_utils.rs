//! Utility functions for command implementations
//!
//! This module provides common utilities that commands frequently need,
//! reducing boilerplate and ensuring consistent behavior across commands.

use anyhow::{Context as AnyhowContext, Result};
use lash_types::error::LashError;
use std::io::{self, Write};

/// Prompt the user for yes/no confirmation
///
/// Displays a prompt and waits for user input. Returns `true` if the user
/// responds with 'y' or 'yes' (case-insensitive), `false` otherwise.
///
/// # Arguments
///
/// * `prompt` - The question to ask the user
/// * `default` - Default answer if user just presses Enter
///
/// # Returns
///
/// `true` if user confirms, `false` if user declines
///
/// # Errors
///
/// Returns an error if stdin/stdout operations fail
///
/// # Example
///
/// ```no_run
/// use lash_cli::command_utils::prompt_confirmation;
///
/// if prompt_confirmation("Delete all tasks?", false)? {
///     println!("Deleting...");
/// } else {
///     println!("Cancelled");
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn prompt_confirmation(prompt: &str, default: bool) -> Result<bool> {
    let default_hint = if default { "[Y/n]" } else { "[y/N]" };
    print!("{prompt} {default_hint}: ");
    io::stdout().flush().context("Failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read from stdin")?;

    let input = input.trim().to_lowercase();

    Ok(match input.as_str() {
        "" => default,
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => {
            eprintln!("Invalid input. Please enter 'y' or 'n'.");
            prompt_confirmation(prompt, default)?
        }
    })
}

/// Ensure the database index is up to date
///
/// Checks if the `SQLite` index is in sync with the markdown files.
/// Returns an error if the index is out of sync and prompts the user
/// to run `lash index`.
///
/// # Arguments
///
/// * `force` - If true, always reindex regardless of sync status
///
/// # Returns
///
/// `Ok(())` if index is up to date or was successfully rebuilt
///
/// # Errors
///
/// Returns a `LashError::Index` error if:
/// - Index is out of sync and user declines to rebuild
/// - Index rebuild fails
///
/// # Example
///
/// ```no_run
/// use lash_cli::command_utils::ensure_indexed;
///
/// // Ensure index is ready before querying
/// ensure_indexed(false)?;
/// # Ok::<(), lash_types::error::LashError>(())
/// ```
#[allow(clippy::result_large_err)]
pub fn ensure_indexed(_force: bool) -> lash_types::error::Result<()> {
    // TODO: Implement once database/indexing is available
    // For now, this is a placeholder that always succeeds
    //
    // Future implementation will:
    // 1. Check if .lash/index.db exists
    // 2. Compare file mtimes with index metadata
    // 3. If out of sync:
    //    - If force=true, rebuild immediately
    //    - If force=false, prompt user and rebuild if confirmed
    // 4. Return appropriate errors if rebuild fails

    Ok(())
}

/// Get a database connection from the context
///
/// Returns a connection to the `SQLite` index database.
/// The connection is lazily initialized on first access.
///
/// # Returns
///
/// A reference to the database connection
///
/// # Errors
///
/// Returns a `LashError::Index` error if:
/// - Database file doesn't exist
/// - Connection cannot be established
/// - Database is corrupted
///
/// # Example
///
/// ```no_run
/// use lash_cli::context::Context;
/// use lash_cli::command_utils::get_db;
///
/// # let ctx = Context::new_for_testing();
/// // Will be implemented once database module is available
/// // let db = get_db(&ctx)?;
/// # Ok::<(), lash_types::error::LashError>(())
/// ```
#[allow(clippy::result_large_err)]
pub fn get_db(_ctx: &crate::context::Context) -> lash_types::error::Result<()> {
    // TODO: Implement once lash-db crate is available
    // This will return a reference to the database connection
    // stored in ctx.db (using OnceLock for lazy init)
    Err(LashError::internal(
        "Database support not yet implemented",
        None,
    ))
}

/// Get a markdown parser instance
///
/// Returns a configured parser for reading Lash markdown files.
/// The parser is lazily initialized on first access.
///
/// # Returns
///
/// A reference to the markdown parser
///
/// # Example
///
/// ```no_run
/// use lash_cli::context::Context;
/// use lash_cli::command_utils::get_parser;
///
/// # let ctx = Context::new_for_testing();
/// // Will be implemented once parser is needed
/// // let parser = get_parser(&ctx)?;
/// # Ok::<(), lash_types::error::LashError>(())
/// ```
#[allow(clippy::result_large_err)]
pub fn get_parser(_ctx: &crate::context::Context) -> lash_types::error::Result<()> {
    // TODO: Implement once we need a shared parser instance
    // Currently, commands create parsers directly via lash_core::parser::parse_file
    // This function is a placeholder for future optimization where we might
    // want to share a parser instance across operations
    Err(LashError::internal(
        "Shared parser not yet implemented",
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_indexed_placeholder() {
        // Should succeed (placeholder implementation)
        let result = ensure_indexed(false);
        assert!(result.is_ok());

        let result = ensure_indexed(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_db_not_implemented() {
        let ctx = crate::context::Context::new_for_testing();
        let result = get_db(&ctx);
        assert!(result.is_err());
        // Should return internal error for not implemented
        let err = result.unwrap_err();
        assert_eq!(err.code(), lash_types::error::codes::E_INTERNAL);
    }

    #[test]
    fn test_get_parser_not_implemented() {
        let ctx = crate::context::Context::new_for_testing();
        let result = get_parser(&ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), lash_types::error::codes::E_INTERNAL);
    }

    // Note: prompt_confirmation cannot be easily unit tested as it requires stdin
    // Integration tests should be used for testing interactive behavior
}
