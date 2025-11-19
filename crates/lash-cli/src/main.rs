//! Lash - Minimalist Markdown-native task tracker
//!
//! Command-line interface for the Lash task management system.

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod commands;
mod utils;

use clap::{Parser, Subcommand};
use lash_types::Severity;
use std::path::PathBuf;
use std::process;

/// Parse a severity level from a string
fn parse_severity(s: &str) -> Result<Severity, String> {
    match s.to_lowercase().as_str() {
        "error" => Ok(Severity::Error),
        "warning" => Ok(Severity::Warning),
        "info" => Ok(Severity::Info),
        "hint" => Ok(Severity::Hint),
        _ => Err(format!(
            "Invalid severity level: '{s}'. Must be one of: error, warning, info, hint"
        )),
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "lash",
    version,
    about = "Minimalist Markdown-native task tracker",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Display version information
    Version,

    /// Lint Lash task files for errors
    #[command(name = "lint")]
    Lint {
        /// Files or directories to lint (defaults to current project)
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,

        /// Output diagnostics in JSON format
        #[arg(long)]
        json: bool,

        /// Apply auto-fixes where possible
        #[arg(long)]
        fix: bool,

        /// Run only specific rule(s) by code (can be specified multiple times)
        #[arg(long = "rule", value_name = "CODE")]
        rules: Vec<String>,

        /// Only show errors of this severity or higher (error, warning, info, hint)
        #[arg(long = "severity", value_name = "LEVEL")]
        min_severity: Option<String>,

        /// Disable colored output
        #[arg(long = "no-color")]
        no_color: bool,
    },

    /// Format Lash task files
    #[command(name = "format")]
    Format {
        /// Files or directories to format (defaults to current project)
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,

        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,

        /// Show diff of formatting changes
        #[arg(long)]
        diff: bool,

        /// Only normalize formatting, don't apply lint fixes
        #[arg(long = "no-fix")]
        no_fix: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Some(Commands::Version) | None => {
            println!("lash {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some(Commands::Lint {
            paths,
            json,
            fix,
            rules,
            min_severity,
            no_color,
        }) => {
            // Parse severity string if provided
            let severity = if let Some(s) = min_severity {
                match parse_severity(&s) {
                    Ok(sev) => Some(sev),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        process::exit(1);
                    }
                }
            } else {
                None
            };

            let args = commands::lint::LintArgs {
                paths,
                json,
                fix,
                rules,
                min_severity: severity,
                no_color,
            };

            match commands::lint::execute(args) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Error: {e:#}");
                    1
                }
            }
        }
        Some(Commands::Format {
            paths,
            check,
            diff,
            no_fix,
        }) => {
            let args = commands::format::FormatArgs {
                paths,
                check,
                diff,
                no_fix,
            };

            match commands::format::execute(args) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Error: {e:#}");
                    1
                }
            }
        }
    };

    process::exit(exit_code);
}
