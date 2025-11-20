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
        Ok(code) => {
            tracing::debug!(exit_code = code, "Command completed successfully");
            code
        }
        Err(e) => {
            tracing::error!(error = %e, "Command failed");
            eprintln!("Error: {e:#}");
            1
        }
    };

    process::exit(exit_code);
}

/// Run the CLI application
#[allow(clippy::too_many_lines)] // Will be refactored when commands are implemented
#[allow(unused_variables)] // Variables will be used when commands are implemented
fn run(cli: LashCli) -> Result<i32> {
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
            commands::lint::execute(args)
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
            commands::format::execute(args)
        }

        Commands::Index {
            force: _,
            show_files: _,
        } => {
            // TODO: Implement index command
            eprintln!("The 'index' command is not yet implemented");
            Ok(1)
        }

        Commands::CheckIndex { diff } => {
            // TODO: Implement check-index command
            eprintln!("The 'check-index' command is not yet implemented");
            Ok(1)
        }

        Commands::List {
            label,
            status,
            path,
            blocked,
            owner,
            format,
        } => {
            // TODO: Implement list command
            eprintln!("The 'list' command is not yet implemented");
            Ok(1)
        }

        Commands::Search {
            query,
            limit,
            threshold,
        } => {
            // TODO: Implement search command
            eprintln!("The 'search' command is not yet implemented");
            Ok(1)
        }

        Commands::Show {
            target,
            deps,
            rdeps,
        } => {
            // TODO: Implement show command
            eprintln!("The 'show' command is not yet implemented");
            Ok(1)
        }

        Commands::Graph {
            format,
            scope,
            output,
        } => {
            // TODO: Implement graph command
            eprintln!("The 'graph' command is not yet implemented");
            Ok(1)
        }

        Commands::CheckLinks { fix } => {
            // TODO: Implement check-links command
            eprintln!("The 'check-links' command is not yet implemented");
            Ok(1)
        }

        Commands::AgentPrompt {
            format: _,
            label: _,
            path: _,
            max_tokens: _,
        } => {
            // TODO: Implement agent-prompt command
            eprintln!("The 'agent-prompt' command is not yet implemented");
            Ok(1)
        }

        Commands::Tui => {
            // TODO: Implement TUI command
            eprintln!("The 'tui' command is not yet implemented");
            Ok(1)
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
            Ok(0)
        }
    }
}
