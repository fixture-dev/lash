//! Lash - Minimalist Markdown-native task tracker
//!
//! Command-line interface for the Lash task management system.

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod commands;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use lash_cli::cli::{Commands, LashCli};
use lash_cli::formatter::{
    JsonFormatter, OutputFormatter, QuietFormatter, TextFormatter, Verbosity,
};
use lash_cli::logging::{init_logging, install_panic_hook, LogConfig};
use lash_cli::project_root::ProjectRootFinder;
use lash_types::error::ExitCode;
use std::process;

fn main() {
    // Install panic hook for better crash reporting
    install_panic_hook();

    // Parse CLI arguments
    let cli = LashCli::parse();

    // Initialize logging based on CLI flags
    let log_config = LogConfig::new(
        if cli.quiet {
            Verbosity::Quiet
        } else {
            Verbosity::from(cli.verbose)
        },
        cli.json,
        cli.no_color,
    );

    if let Err(e) = init_logging(&log_config) {
        eprintln!("Warning: Failed to initialize logging: {e}");
        // Continue execution even if logging fails
    }

    // Log startup information at debug level
    tracing::debug!(
        version = env!("CARGO_PKG_VERSION"),
        args = ?std::env::args().collect::<Vec<_>>(),
        "Lash CLI started"
    );

    // Execute the command and get exit code
    let exit_code = match run(cli) {
        Ok(()) => {
            tracing::debug!("Command completed successfully");
            ExitCode::Success
        }
        Err(e) => {
            tracing::error!(error = %e, "Command failed");
            eprintln!("Error: {e:#}");

            // Convert LashError to appropriate exit code
            // Note: anyhow::Error might wrap a LashError, so we need to downcast
            if let Some(lash_err) = e.downcast_ref::<lash_types::error::LashError>() {
                ExitCode::from(lash_err)
            } else {
                // Non-LashError (e.g., anyhow errors from other sources)
                ExitCode::GeneralError
            }
        }
    };

    process::exit(exit_code.as_i32());
}

/// Run the CLI application
///
/// # Errors
///
/// Returns an error if command execution fails. The error type may be:
/// - `LashError` for Lash-specific errors (will map to specific exit codes)
/// - `anyhow::Error` for other errors (will use `GeneralError` exit code)
#[allow(clippy::too_many_lines)] // Will be refactored when commands are implemented
#[allow(unused_variables)] // Variables will be used when commands are implemented
fn run(cli: LashCli) -> Result<()> {
    // Determine output format based on flags
    let use_color = !cli.no_color && !cli.json;
    let verbosity = if cli.quiet {
        Verbosity::Quiet
    } else {
        Verbosity::from(cli.verbose)
    };

    // Create formatter based on output mode
    let formatter: Box<dyn OutputFormatter> = if cli.json {
        Box::new(JsonFormatter::new(false))
    } else if cli.quiet {
        Box::new(QuietFormatter::new())
    } else {
        Box::new(TextFormatter::new(use_color, verbosity))
    };

    // Find project root if needed
    let project_root = if let Some(root) = cli.root {
        // Explicit root provided
        let finder = ProjectRootFinder::new();
        finder
            .validate_root(&root)
            .context("Invalid project root")?;
        Some(root)
    } else {
        // Try to auto-detect, but don't fail if not found
        // (some commands might not need it)
        let finder = ProjectRootFinder::new();
        finder.find_from_cwd().ok()
    };

    // Dispatch to appropriate command handler
    match cli.command {
        Commands::Lint {
            paths,
            fix,
            rules,
            min_severity,
        } => {
            // Convert min_severity to lash_types::Severity
            let severity = min_severity.map(|s| match s {
                lash_cli::cli::SeverityLevel::Error => lash_types::Severity::Error,
                lash_cli::cli::SeverityLevel::Warning => lash_types::Severity::Warning,
                lash_cli::cli::SeverityLevel::Info => lash_types::Severity::Info,
                lash_cli::cli::SeverityLevel::Hint => lash_types::Severity::Hint,
            });

            let args = commands::lint::LintArgs {
                paths,
                json: cli.json,
                fix,
                rules,
                min_severity: severity,
                no_color: cli.no_color,
            };
            commands::lint::execute(args)?;
            Ok(())
        }

        Commands::Format {
            paths,
            check,
            diff,
            no_fix,
        } => {
            let args = commands::format::FormatArgs {
                paths,
                check,
                diff,
                no_fix,
            };
            commands::format::execute(args)?;
            Ok(())
        }

        Commands::Index { force, show_files } => {
            let args = commands::index::IndexArgs {
                force,
                show_files,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
            };
            commands::index::execute(args)?;
            Ok(())
        }

        Commands::CheckIndex { diff } => {
            let args = commands::check_index::CheckIndexArgs {
                diff,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
            };
            commands::check_index::execute(args)?;
            Ok(())
        }

        Commands::List {
            label,
            status,
            path,
            blocked,
            owner,
            format,
        } => {
            // Convert status to lash_types::TaskStatus
            let task_status = status.map(|s| match s {
                lash_cli::cli::TaskStatus::Open => lash_types::TaskStatus::Open,
                lash_cli::cli::TaskStatus::Done => lash_types::TaskStatus::Done,
                lash_cli::cli::TaskStatus::Waived => lash_types::TaskStatus::Waived,
                lash_cli::cli::TaskStatus::Blocked => lash_types::TaskStatus::Blocked,
            });

            // Convert format to OutputFormat
            let output_format = match format {
                lash_cli::cli::OutputFormat::Text => commands::list::OutputFormat::Text,
                lash_cli::cli::OutputFormat::Json => commands::list::OutputFormat::Json,
                lash_cli::cli::OutputFormat::JsonPretty => commands::list::OutputFormat::JsonPretty,
            };

            let args = commands::list::ListArgs {
                labels: label,
                status: task_status,
                path,
                blocked,
                owner,
                format: output_format,
                no_color: cli.no_color,
                project_root,
            };
            commands::list::execute(args)?;
            Ok(())
        }

        Commands::Search {
            query,
            limit,
            threshold,
        } => {
            let args = commands::search::SearchArgs {
                query,
                limit,
                threshold,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
            };
            let exit_code = commands::search::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Show {
            target,
            deps,
            rdeps,
        } => {
            let args = commands::show::ShowArgs {
                target,
                deps,
                rdeps,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
            };
            commands::show::execute(&args)?;
            Ok(())
        }

        Commands::Graph {
            format,
            scope,
            output,
        } => {
            // TODO: Implement graph command
            anyhow::bail!("The 'graph' command is not yet implemented")
        }

        Commands::CheckLinks { fix } => {
            // TODO: Implement check-links command
            anyhow::bail!("The 'check-links' command is not yet implemented")
        }

        Commands::AgentPrompt {
            format: _,
            label: _,
            path: _,
            max_tokens: _,
        } => {
            // TODO: Implement agent-prompt command
            anyhow::bail!("The 'agent-prompt' command is not yet implemented")
        }

        Commands::Tui => {
            // TODO: Implement TUI command
            anyhow::bail!("The 'tui' command is not yet implemented")
        }

        Commands::Completion { shell } => {
            use clap::CommandFactory;
            use lash_cli::cli::Shell;

            let shell_type = match shell {
                Shell::Bash => clap_complete::Shell::Bash,
                Shell::Zsh => clap_complete::Shell::Zsh,
                Shell::Fish => clap_complete::Shell::Fish,
                Shell::Powershell => clap_complete::Shell::PowerShell,
                Shell::Elvish => clap_complete::Shell::Elvish,
            };

            let mut cmd = LashCli::command();
            let bin_name = cmd.get_name().to_string();
            clap_complete::generate(shell_type, &mut cmd, bin_name, &mut std::io::stdout());
            Ok(())
        }
    }
}
