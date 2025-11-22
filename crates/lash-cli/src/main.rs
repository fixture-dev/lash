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
                project_root,
            };
            let exit_code = commands::lint::execute(args)?;
            process::exit(exit_code);
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
                project_root,
            };
            let exit_code = commands::format::execute(args)?;
            process::exit(exit_code);
        }

        Commands::Index { force, show_files } => {
            let args = commands::index::IndexArgs {
                force,
                show_files,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
            };
            let exit_code = commands::index::execute(args)?;
            process::exit(exit_code);
        }

        Commands::CheckIndex { diff } => {
            let args = commands::check_index::CheckIndexArgs {
                diff,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
            };
            let exit_code = commands::check_index::execute(args)?;
            process::exit(exit_code);
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
            // Global --json flag overrides command-specific format
            let output_format = if cli.json {
                commands::list::OutputFormat::JsonPretty
            } else {
                match format {
                    lash_cli::cli::OutputFormat::Text => commands::list::OutputFormat::Text,
                    lash_cli::cli::OutputFormat::Json => commands::list::OutputFormat::Json,
                    lash_cli::cli::OutputFormat::JsonPretty => {
                        commands::list::OutputFormat::JsonPretty
                    }
                }
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
            let exit_code = commands::list::execute(args)?;
            process::exit(exit_code);
        }

        Commands::Search {
            query,
            limit,
            label,
            status,
            owner,
            path,
        } => {
            // Convert status to lash_types::TaskStatus
            let task_status = status.map(|s| match s {
                lash_cli::cli::TaskStatus::Open => lash_types::TaskStatus::Open,
                lash_cli::cli::TaskStatus::Done => lash_types::TaskStatus::Done,
                lash_cli::cli::TaskStatus::Waived => lash_types::TaskStatus::Waived,
                lash_cli::cli::TaskStatus::Blocked => lash_types::TaskStatus::Blocked,
            });

            let args = commands::search::SearchArgs {
                query,
                limit,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
                labels: label,
                status: task_status,
                owner,
                path,
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
            let exit_code = commands::show::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Graph {
            format,
            scope,
            hide_completed,
            output,
        } => {
            // Convert format to GraphFormat
            // Global --json flag overrides command-specific format
            let graph_format = if cli.json {
                commands::graph::GraphFormat::Json
            } else {
                match format {
                    lash_cli::cli::GraphFormat::Dot => commands::graph::GraphFormat::Dot,
                    lash_cli::cli::GraphFormat::Mermaid => commands::graph::GraphFormat::Mermaid,
                    lash_cli::cli::GraphFormat::Json => commands::graph::GraphFormat::Json,
                }
            };

            let args = commands::graph::GraphArgs {
                format: graph_format,
                scope,
                hide_completed,
                output,
                project_root,
            };
            let exit_code = commands::graph::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::CheckLinks { fix, yes, dry_run } => {
            let args = commands::check_links::CheckLinksArgs {
                json: cli.json,
                no_color: cli.no_color,
                project_root,
                fix,
                yes,
                dry_run,
            };
            let exit_code = commands::check_links::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::AgentPrompt {
            format,
            label,
            path,
            max_tokens,
        } => {
            // Global --json flag overrides command-specific format
            let agent_format = if cli.json {
                lash_cli::cli::AgentFormat::Json
            } else {
                format
            };

            let args = commands::agent_prompt::AgentPromptArgs {
                format: agent_format,
                labels: label,
                path,
                max_tokens,
                project_root: None,
                json: cli.json,
                no_color: cli.no_color,
            };
            let exit_code = commands::agent_prompt::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Tui => {
            let args = commands::tui::TuiArgs { project_root };
            commands::tui::execute(&args)?;
            Ok(())
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
