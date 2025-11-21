//! Command trait and execution framework
//!
//! This module defines the core `Command` trait that all Lash CLI commands implement.
//! The trait provides a consistent execution pattern with shared context and error handling.
//!
//! # Architecture
//!
//! Commands follow a simple pattern:
//! 1. Parse CLI arguments into a command-specific struct
//! 2. Create a shared `Context` with configuration, project root, etc.
//! 3. Execute the command with access to the context
//! 4. Return a result with appropriate error handling
//!
//! # Example
//!
//! ```
//! use lash_cli::command::Command;
//! use lash_cli::context::Context;
//! use lash_types::error::Result;
//!
//! struct MyCommand {
//!     some_arg: String,
//! }
//!
//! impl Command for MyCommand {
//!     fn execute(&self, ctx: &Context) -> Result<()> {
//!         // Use context resources
//!         let config = ctx.config();
//!         println!("Executing with arg: {}", self.some_arg);
//!         Ok(())
//!     }
//! }
//! ```

use lash_types::error::Result;

use crate::context::Context;

/// Command trait implemented by all Lash CLI commands
///
/// Commands receive a shared `Context` containing configuration, project root,
/// and lazily-initialized resources like database connections.
///
/// # Error Handling
///
/// Commands should return `lash_types::error::Result<()>` to enable consistent
/// error reporting and exit code mapping.
///
/// # Example Implementation
///
/// ```
/// use lash_cli::command::Command;
/// use lash_cli::context::Context;
/// use lash_types::error::Result;
///
/// struct ListCommand {
///     label: Option<String>,
/// }
///
/// impl Command for ListCommand {
///     fn execute(&self, ctx: &Context) -> Result<()> {
///         let config = ctx.config();
///         println!("Listing tasks with config: {:?}", config);
///         Ok(())
///     }
/// }
/// ```
pub trait Command {
    /// Execute the command with the given context
    ///
    /// # Arguments
    ///
    /// * `ctx` - Shared context containing configuration and resources
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a `LashError` on failure
    ///
    /// # Errors
    ///
    /// Commands should return appropriate `LashError` variants for different
    /// failure scenarios. The error will be formatted and displayed to the user
    /// with an appropriate exit code.
    #[allow(clippy::result_large_err)]
    fn execute(&self, ctx: &Context) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test command for verifying trait implementation
    struct TestCommand {
        should_fail: bool,
    }

    impl Command for TestCommand {
        fn execute(&self, _ctx: &Context) -> Result<()> {
            if self.should_fail {
                Err(lash_types::error::LashError::internal("test error", None))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_command_trait_success() {
        let ctx = Context::new_for_testing();
        let cmd = TestCommand { should_fail: false };
        let result = cmd.execute(&ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_trait_error() {
        let ctx = Context::new_for_testing();
        let cmd = TestCommand { should_fail: true };
        let result = cmd.execute(&ctx);
        assert!(result.is_err());
    }
}
