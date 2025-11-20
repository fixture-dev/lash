//! Lash CLI Library
//!
//! This library provides the command-line interface infrastructure for Lash,
//! including argument parsing, output formatting, progress reporting, and
//! project root detection.

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)] // Result types in traits are self-explanatory
#![allow(clippy::must_use_candidate)] // Avoid excessive #[must_use] attributes
#![allow(clippy::cast_precision_loss)] // Acceptable for progress percentages
#![allow(clippy::format_push_string)] // More readable than write!() for simple cases

pub mod cli;
pub mod config;
pub mod formatter;
pub mod progress;
pub mod project_root;

// Re-export commonly used types
pub use cli::{Commands, LashCli};
pub use config::Config;
pub use formatter::{
    JsonFormatter, OutputFormat, OutputFormatter, QuietFormatter, TextFormatter, Verbosity,
};
pub use progress::{
    JsonProgressReporter, ProgressReporter, QuietProgressReporter, TerminalProgressReporter,
};
pub use project_root::ProjectRootFinder;
