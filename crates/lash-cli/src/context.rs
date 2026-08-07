//! Shared context for command execution
//!
//! The `Context` struct provides shared state and resources for all CLI commands.
//! It uses lazy initialization for expensive resources like database connections
//! to avoid overhead when they're not needed.
//!
//! # Design
//!
//! The context follows these principles:
//! - Immutable after construction (commands receive `&Context`)
//! - Lazy initialization using `OnceCell` for expensive resources
//! - Explicit resource access through getter methods
//! - No global state - passed explicitly to commands
//!
//! # Example
//!
//! ```
//! use lash::context::Context;
//! use std::path::PathBuf;
//!
//! // Create context with project root
//! let ctx = Context::builder()
//!     .project_root(PathBuf::from("/path/to/project"))
//!     .build()
//!     .expect("Failed to build context");
//!
//! // Access configuration
//! let config = ctx.config();
//! println!("Max depth: {}", config.linter.max_depth);
//! ```

use anyhow::{Context as AnyhowContext, Result};
use lash_types::LashConfig;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::config::Config as CliConfig;
use crate::formatter::OutputFormatter;

/// Shared context for command execution
///
/// Provides access to configuration, project root, output formatter,
/// and lazily-initialized resources like database connections.
///
/// # Lazy Initialization
///
/// Expensive resources (database, parser) are only initialized when first accessed.
/// This keeps command startup fast for operations that don't need these resources.
pub struct Context {
    /// CLI configuration (from config files)
    cli_config: CliConfig,

    /// Project-level configuration (from .lash/config.toml)
    project_config: LashConfig,

    /// Project root directory (if detected)
    project_root: Option<PathBuf>,

    /// Output formatter for command results
    formatter: Box<dyn OutputFormatter>,

    /// Lazily-initialized database connection
    /// Note: Using `OnceLock` for thread-safe lazy initialization
    /// In the future, this will hold an actual DB connection
    #[allow(dead_code)]
    db: OnceLock<()>,

    /// Lazily-initialized markdown parser
    #[allow(dead_code)]
    parser: OnceLock<()>,
}

impl Context {
    /// Create a new context builder
    ///
    /// # Example
    ///
    /// ```
    /// use lash::context::Context;
    ///
    /// let ctx = Context::builder()
    ///     .build()
    ///     .expect("Failed to build context");
    /// ```
    #[must_use]
    pub fn builder() -> ContextBuilder {
        ContextBuilder::new()
    }

    /// Get the CLI configuration
    ///
    /// # Example
    ///
    /// ```
    /// # use lash::context::Context;
    /// # let ctx = Context::new_for_testing();
    /// let config = ctx.config();
    /// println!("Verbosity: {}", config.output.verbosity);
    /// ```
    #[must_use]
    pub fn config(&self) -> &CliConfig {
        &self.cli_config
    }

    /// Get the project-level configuration
    ///
    /// This is the configuration loaded from `.lash/config.toml` in the project root.
    ///
    /// # Example
    ///
    /// ```
    /// # use lash::context::Context;
    /// # let ctx = Context::new_for_testing();
    /// let project_config = ctx.project_config();
    /// println!("Max depth: {}", project_config.max_depth);
    /// ```
    #[must_use]
    pub fn project_config(&self) -> &LashConfig {
        &self.project_config
    }

    /// Get the project root directory
    ///
    /// Returns `None` if no project root was detected.
    ///
    /// # Example
    ///
    /// ```
    /// # use lash::context::Context;
    /// # let ctx = Context::new_for_testing();
    /// if let Some(root) = ctx.project_root() {
    ///     println!("Project root: {}", root.display());
    /// }
    /// ```
    #[must_use]
    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    /// Get the output formatter
    ///
    /// Returns a reference to the formatter for displaying command results.
    ///
    /// # Example
    ///
    /// ```
    /// # use lash::context::Context;
    /// # let ctx = Context::new_for_testing();
    /// let formatter = ctx.formatter();
    /// // Use formatter to display results
    /// ```
    #[must_use]
    pub fn formatter(&self) -> &dyn OutputFormatter {
        self.formatter.as_ref()
    }

    /// Create a context for testing
    ///
    /// Uses default configuration and no project root.
    /// Should only be used in tests.
    ///
    /// # Example
    ///
    /// ```
    /// use lash::context::Context;
    ///
    /// let ctx = Context::new_for_testing();
    /// assert!(ctx.project_root().is_none());
    /// ```
    #[must_use]
    pub fn new_for_testing() -> Self {
        use crate::formatter::Verbosity;
        use crate::TextFormatter;

        Self {
            cli_config: CliConfig::default(),
            project_config: LashConfig::default(),
            project_root: None,
            formatter: Box::new(TextFormatter::new(false, Verbosity::Normal)),
            db: OnceLock::new(),
            parser: OnceLock::new(),
        }
    }
}

/// Builder for constructing a `Context`
///
/// Allows step-by-step construction of a context with optional components.
///
/// # Example
///
/// ```
/// use lash::context::Context;
/// use std::path::PathBuf;
///
/// let ctx = Context::builder()
///     .project_root(PathBuf::from("/tmp"))
///     .build()
///     .expect("Failed to build context");
/// ```
pub struct ContextBuilder {
    cli_config: Option<CliConfig>,
    project_root: Option<PathBuf>,
    formatter: Option<Box<dyn OutputFormatter>>,
}

impl ContextBuilder {
    /// Create a new context builder with defaults
    fn new() -> Self {
        Self {
            cli_config: None,
            project_root: None,
            formatter: None,
        }
    }

    /// Set the CLI configuration
    ///
    /// If not set, defaults will be used.
    ///
    /// # Example
    ///
    /// ```
    /// use lash::context::Context;
    /// use lash::config::Config;
    ///
    /// let config = Config::default();
    /// let ctx = Context::builder()
    ///     .cli_config(config)
    ///     .build()
    ///     .expect("Failed to build");
    /// ```
    #[must_use]
    pub fn cli_config(mut self, config: CliConfig) -> Self {
        self.cli_config = Some(config);
        self
    }

    /// Set the project root directory
    ///
    /// If not set, commands will run without a project root.
    ///
    /// # Example
    ///
    /// ```
    /// use lash::context::Context;
    /// use std::path::PathBuf;
    ///
    /// let ctx = Context::builder()
    ///     .project_root(PathBuf::from("/tmp"))
    ///     .build()
    ///     .expect("Failed to build");
    /// ```
    #[must_use]
    pub fn project_root(mut self, root: PathBuf) -> Self {
        self.project_root = Some(root);
        self
    }

    /// Set the output formatter
    ///
    /// If not set, a default text formatter will be used.
    ///
    /// # Example
    ///
    /// ```
    /// use lash::context::Context;
    /// use lash::formatter::{JsonFormatter, Verbosity};
    ///
    /// let formatter = Box::new(JsonFormatter::new(false));
    /// let ctx = Context::builder()
    ///     .formatter(formatter)
    ///     .build()
    ///     .expect("Failed to build");
    /// ```
    #[must_use]
    pub fn formatter(mut self, formatter: Box<dyn OutputFormatter>) -> Self {
        self.formatter = Some(formatter);
        self
    }

    /// Build the context
    ///
    /// Loads configuration files if a project root is set.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration files exist but cannot be parsed
    /// - Configuration validation fails
    ///
    /// # Example
    ///
    /// ```
    /// use lash::context::Context;
    ///
    /// let ctx = Context::builder()
    ///     .build()
    ///     .expect("Failed to build context");
    /// ```
    pub fn build(self) -> Result<Context> {
        use crate::formatter::{TextFormatter, Verbosity};

        // Load CLI config (merged from user and project config)
        let cli_config = if let Some(config) = self.cli_config {
            config
        } else {
            CliConfig::load_merged(self.project_root.as_deref())
                .context("Failed to load configuration")?
        };

        // Load project config (for lash-core operations)
        // Note: LashConfig and CliConfig are different types
        // LashConfig is used by lash-core for parsing and linting
        // It's constructed from the project root, not loaded from a file
        let project_config = if let Some(ref root) = self.project_root {
            LashConfig::from_root(root).unwrap_or_else(|_| LashConfig::default())
        } else {
            LashConfig::default()
        };

        // Use provided formatter or create default
        let formatter = self
            .formatter
            .unwrap_or_else(|| Box::new(TextFormatter::new(true, Verbosity::Normal)));

        Ok(Context {
            cli_config,
            project_config,
            project_root: self.project_root,
            formatter,
            db: OnceLock::new(),
            parser: OnceLock::new(),
        })
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatter::JsonFormatter;
    use tempfile::TempDir;

    #[test]
    fn test_context_builder_defaults() {
        let ctx = Context::builder().build().unwrap();
        assert!(ctx.project_root().is_none());
        assert_eq!(ctx.config().output.default_format, "text");
    }

    #[test]
    fn test_context_builder_with_root() {
        let temp = TempDir::new().unwrap();
        let ctx = Context::builder()
            .project_root(temp.path().to_path_buf())
            .build()
            .unwrap();

        assert_eq!(ctx.project_root(), Some(temp.path()));
    }

    #[test]
    fn test_context_builder_with_config() {
        let mut config = CliConfig::default();
        config.output.default_format = "json".to_string();

        let ctx = Context::builder().cli_config(config).build().unwrap();

        assert_eq!(ctx.config().output.default_format, "json");
    }

    #[test]
    fn test_context_builder_with_formatter() {
        let formatter = Box::new(JsonFormatter::new(false));
        let ctx = Context::builder().formatter(formatter).build().unwrap();

        // Formatter is set - just verify we can access it without panicking
        let _formatter = ctx.formatter();
    }

    #[test]
    fn test_context_loads_project_config() {
        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        std::fs::create_dir(&lash_dir).unwrap();

        let config_path = lash_dir.join("config.toml");
        let toml = r#"
[output]
default_format = "json"
"#;
        std::fs::write(&config_path, toml).unwrap();

        let ctx = Context::builder()
            .project_root(temp.path().to_path_buf())
            .build()
            .unwrap();

        assert_eq!(ctx.config().output.default_format, "json");
    }

    #[test]
    fn test_context_new_for_testing() {
        let ctx = Context::new_for_testing();
        assert!(ctx.project_root().is_none());
        assert_eq!(ctx.config().output.default_format, "text");
    }

    #[test]
    fn test_context_getters() {
        let ctx = Context::new_for_testing();

        // Test all getters
        let _config = ctx.config();
        let _project_config = ctx.project_config();
        let _root = ctx.project_root();
        let _formatter = ctx.formatter();

        // All getters should work without panicking
    }
}
