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
use lash_cli::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash_cli::formatter::{
    JsonFormatter, OutputFormat, OutputFormatter, QuietFormatter, TextFormatter, Verbosity,
};
use lash_cli::logging::{init_logging, install_panic_hook, LogConfig};
use lash_cli::project_root::ProjectRootFinder;
use lash_cli::theme::{supports_color, CliTheme};
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

    // Determine output format for error reporting
    let output_format = if cli.json {
        OutputFormat::Json
    } else if cli.quiet {
        OutputFormat::Quiet
    } else {
        OutputFormat::Text
    };

    // Create verbosity for error reporter
    let verbosity = if cli.quiet {
        Verbosity::Quiet
    } else {
        Verbosity::from(cli.verbose)
    };

    // Load theme for error formatting (only if colors are enabled)
    let colors_enabled = !cli.no_color && !cli.json && supports_color();
    let theme = if colors_enabled {
        CliTheme::load(cli.color_scheme.as_deref(), true)
            .ok()
            .flatten()
    } else {
        None
    };

    // Execute the command and get exit code
    let exit_code = match run(cli) {
        Ok(()) => {
            tracing::debug!("Command completed successfully");
            ExitCode::Success
        }
        Err(e) => {
            tracing::error!(error = %e, "Command failed");

            // Create ErrorReporter for batch mode (collect and display at once)
            let config = ErrorReporterConfig {
                verbosity,
                output_format,
                display_mode: ErrorDisplayMode::Batch,
                theme,
                show_summary: false, // Don't show summary for single errors
            };
            let mut reporter = ErrorReporter::new(config);

            // Check if this is a LashError, otherwise treat as general error
            let exit_code = if let Some(lash_err) = e.downcast_ref::<lash_types::error::LashError>()
            {
                // Report the LashError using the error reporter
                reporter.report_error(lash_err);
                reporter.flush();
                ExitCode::from(lash_err)
            } else {
                // Non-LashError (e.g., anyhow errors from other sources)
                // Format as a general error message
                let formatted = if output_format.is_json() {
                    serde_json::json!({
                        "status": "error",
                        "message": format!("{:#}", e)
                    })
                    .to_string()
                } else {
                    format!("Error: {e:#}")
                };
                eprintln!("{formatted}");
                ExitCode::GeneralError
            };

            exit_code
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
    // Determine if colors should be enabled based on:
    // 1. --no-color flag
    // 2. --json flag (JSON shouldn't have ANSI codes)
    // 3. NO_COLOR environment variable
    // 4. Whether stdout is a TTY
    let colors_enabled = !cli.no_color && !cli.json && supports_color();

    let verbosity = if cli.quiet {
        Verbosity::Quiet
    } else {
        Verbosity::from(cli.verbose)
    };

    // Load the theme based on priority: CLI arg > user config > default
    let theme = if colors_enabled {
        CliTheme::load(cli.color_scheme.as_deref(), true)?
    } else {
        None
    };

    // Create formatter based on output mode
    let formatter: Box<dyn OutputFormatter> = if cli.json {
        Box::new(JsonFormatter::new(false))
    } else if cli.quiet {
        Box::new(QuietFormatter::new())
    } else {
        Box::new(TextFormatter::with_theme(theme.clone(), verbosity))
    };

    // Print logo banner for text output (unless suppressed or using structured formats)
    // Logo is shown when:
    // - Not using JSON output
    // - Not in quiet mode
    // - Not suppressed with --no-logo
    // - Not launching TUI (TUI has its own logo display)
    // - Not using machine-readable graph formats (DOT, Mermaid, JSON)
    let uses_machine_readable_graph_format = matches!(
        &cli.command,
        Commands::Graph { format, .. }
            if matches!(format, lash_cli::cli::GraphFormat::Dot
                               | lash_cli::cli::GraphFormat::Mermaid
                               | lash_cli::cli::GraphFormat::Json)
    );
    let uses_machine_readable_list_format = matches!(
        &cli.command,
        Commands::List { format, .. }
            if matches!(format, lash_cli::cli::OutputFormat::Json
                               | lash_cli::cli::OutputFormat::JsonPretty)
    );
    let show_logo = !cli.json
        && !cli.quiet
        && !cli.no_logo
        && !matches!(cli.command, Commands::Tui)
        && !uses_machine_readable_graph_format
        && !uses_machine_readable_list_format;
    if show_logo {
        print!("{}", TextFormatter::logo_banner());
    }

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
            interactive,
            suggest,
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
                interactive,
                suggest,
                rules,
                min_severity: severity,
                no_color: cli.no_color,
                project_root,
                verbosity,
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
                json: cli.json,
                no_color: cli.no_color,
                project_root,
                verbosity,
            };
            let exit_code = commands::format::execute(args)?;
            process::exit(exit_code);
        }

        Commands::Index {
            paths,
            force,
            show_files,
        } => {
            let args = commands::index::IndexArgs {
                paths,
                force,
                show_files,
                json: cli.json,
                no_color: cli.no_color,
                errors_streaming: cli.errors_streaming,
                project_root,
                verbosity,
            };
            let exit_code = commands::index::execute(args)?;
            process::exit(exit_code);
        }

        Commands::CheckIndex { paths, diff } => {
            let args = commands::check_index::CheckIndexArgs {
                paths,
                diff,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
                verbosity,
            };
            let exit_code = commands::check_index::execute(args)?;
            process::exit(exit_code);
        }

        Commands::List {
            filter,
            label,
            status,
            path,
            blocked,
            owner,
            docs,
            show_descriptions,
            show_notes,
            limit,
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

            // Determine tree view settings
            let tree_view = if cli.tree_view {
                Some(true)
            } else if cli.no_tree_view {
                Some(false)
            } else {
                None
            };

            let args = commands::list::ListArgs {
                filter,
                labels: label,
                status: task_status,
                path,
                blocked,
                owner,
                docs,
                show_descriptions,
                show_notes,
                limit,
                format: output_format,
                project_root,
                theme: theme.clone(),
                tree_view,
                max_depth: cli.max_depth,
                ascii: cli.ascii,
                verbosity,
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

            // Determine tree view settings
            let tree_view = if cli.tree_view {
                Some(true)
            } else if cli.no_tree_view {
                Some(false)
            } else {
                None
            };

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
                color_scheme: cli.color_scheme.clone(),
                tree_view,
                max_depth: cli.max_depth,
                ascii: cli.ascii,
                verbosity,
            };
            let exit_code = commands::search::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Show {
            target,
            deps,
            rdeps,
            short,
        } => {
            // Determine tree view settings
            let tree_view = if cli.tree_view {
                Some(true)
            } else if cli.no_tree_view {
                Some(false)
            } else {
                None
            };

            let args = commands::show::ShowArgs {
                target,
                deps,
                rdeps,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
                tree_view,
                max_depth: cli.max_depth,
                ascii: cli.ascii,
                verbosity,
                short,
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
                    lash_cli::cli::GraphFormat::Ascii => commands::graph::GraphFormat::Ascii,
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
                theme: theme.clone(),
                verbosity,
            };
            let exit_code = commands::graph::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::CheckLinks { fix, yes, dry_run } => {
            let args = commands::check_links::CheckLinksArgs {
                json: cli.json,
                project_root,
                fix,
                yes,
                dry_run,
                theme: theme.clone(),
                verbosity,
            };
            let exit_code = commands::check_links::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::AgentPrompt {
            format,
            label,
            path,
            max_tokens,
            include_descriptions,
            include_notes,
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
                include_descriptions,
                include_notes,
            };
            let exit_code = commands::agent_prompt::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Tui => {
            let args = commands::tui::TuiArgs {
                project_root,
                color_scheme: cli.color_scheme,
            };
            commands::tui::execute(&args)?;
            Ok(())
        }

        Commands::Init {
            path,
            no_index,
            force,
        } => {
            let args = commands::init::InitArgs {
                path,
                no_index,
                force,
                json: cli.json,
                no_color: cli.no_color,
                errors_streaming: cli.errors_streaming,
                verbosity,
            };
            let exit_code = commands::init::execute(args)?;
            process::exit(exit_code);
        }

        Commands::Playground { command } => {
            use lash_cli::cli::PlaygroundCommand;
            match command {
                PlaygroundCommand::Init { path, reset } => {
                    let args = commands::playground::PlaygroundArgs {
                        path,
                        reset,
                        json: cli.json,
                        no_color: cli.no_color,
                    };
                    let exit_code = commands::playground::execute(args)?;
                    process::exit(exit_code);
                }
            }
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

        Commands::Explain { code, list } => {
            let args = commands::explain::ExplainArgs {
                code: code.unwrap_or_default(),
                list,
                json: cli.json,
                no_color: cli.no_color,
            };
            let exit_code = commands::explain::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Config { command } => {
            let args = commands::config::ConfigArgs {
                command,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
            };
            let exit_code = commands::config::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Complete {
            task_ids,
            dry_run,
            cascade,
            force,
        } => {
            let args = commands::complete::CompleteArgs {
                task_ids,
                dry_run,
                cascade,
                force,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
                verbosity,
            };
            let exit_code = commands::complete::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Waive {
            task_ids,
            dry_run,
            cascade,
            reason,
        } => {
            let args = commands::waive::WaiveArgs {
                task_ids,
                dry_run,
                cascade,
                reason,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
                verbosity,
            };
            let exit_code = commands::waive::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Update {
            task_id,
            title,
            add_label,
            remove_label,
            owner,
            estimate,
            agent_note,
            append_agent_note,
            add_depends_on,
            remove_depends_on,
            allow_forward_ref,
            dry_run,
        } => {
            let args = commands::update::UpdateArgs {
                task_id,
                title,
                add_label,
                remove_label,
                owner,
                estimate,
                agent_note,
                append_agent_note,
                add_depends_on,
                remove_depends_on,
                allow_forward_ref,
                dry_run,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
                verbosity,
            };
            let exit_code = commands::update::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Start { task_ids, dry_run } => {
            let args = commands::start::StartArgs {
                task_ids,
                dry_run,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
                verbosity,
            };
            let exit_code = commands::start::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Status {
            limit,
            label,
            path,
            owner,
            since,
            compact,
        } => {
            let args = commands::status::StatusArgs {
                limit,
                labels: label,
                path,
                owner,
                since,
                compact,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
                verbosity,
            };
            let exit_code = commands::status::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Add {
            title,
            file,
            file_title,
            file_description,
            parent,
            after,
            before,
            label,
            owner,
            estimate,
            status,
            id,
            depends_on,
            allow_forward_ref,
            agent_note,
            format,
            dry_run,
            interactive,
        } => {
            let args = commands::add::AddArgs {
                title,
                file,
                file_title,
                file_description,
                parent,
                after,
                before,
                label,
                owner,
                estimate,
                status,
                id,
                depends_on,
                allow_forward_ref,
                agent_note,
                format,
                dry_run,
                interactive,
                no_color: cli.no_color,
                project_root,
            };
            let exit_code = commands::add::execute(&args)?;
            process::exit(exit_code);
        }

        Commands::Skill { command } => {
            let args = commands::skill::SkillArgs {
                command,
                json: cli.json,
                no_color: cli.no_color,
                project_root,
            };
            let exit_code = commands::skill::execute(&args)?;
            process::exit(exit_code);
        }
    }
}
