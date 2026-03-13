//! Lash CLI Library
//!
//! This library provides the command-line interface infrastructure for Lash,
//! including argument parsing, output formatting, progress reporting, command
//! execution framework, and project root detection.

#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)] // Result types in traits are self-explanatory
#![allow(clippy::must_use_candidate)] // Avoid excessive #[must_use] attributes
#![allow(clippy::cast_precision_loss)] // Acceptable for progress percentages
#![allow(clippy::format_push_string)] // More readable than write!() for simple cases

pub mod cli;
pub mod command;
pub mod command_utils;
pub mod config;
pub mod context;
pub mod diff_display;
pub mod error_reporter;
pub mod error_validator;
pub mod formatter;
pub mod logging;
pub mod progress;
pub mod project_root;
pub mod theme;
pub mod tree_formatter;

// Re-export commonly used types
pub use cli::{Commands, LashCli};
pub use command::Command;
pub use config::Config;
pub use context::{Context, ContextBuilder};
pub use diff_display::DiffDisplay;
pub use error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig, ErrorSummary};
pub use error_validator::{ErrorValidator, ValidationResult};
pub use formatter::{
    JsonFormatter, OutputFormat, OutputFormatter, QuietFormatter, TextFormatter, Verbosity,
};
pub use logging::{
    get_diagnostic_info, init_logging, init_logging_with_file, install_panic_hook, parse_log_level,
    verbosity_to_level, DiagnosticInfo, LogConfig,
};
pub use progress::{
    JsonProgressReporter, ProgressReporter, QuietProgressReporter, TerminalProgressReporter,
};
pub use project_root::ProjectRootFinder;
pub use theme::{supports_color, CliTheme, Theme};
