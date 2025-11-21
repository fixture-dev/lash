//! CLI argument parsing structures
//!
//! This module defines the command-line interface structure using `clap`,
//! including all subcommands, global flags, and argument specifications.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Minimalist Markdown-native task tracker for devs and agents
#[derive(Parser, Debug)]
#[command(
    name = "lash",
    version,
    about = "Minimalist Markdown-native task tracker",
    long_about = "Lash is an ultra-fast, Markdown-native task tracker designed for developers and AI agents.\n\
                  It uses Markdown as the single source of truth and SQLite as an acceleration layer.\n\n\
                  EXIT CODES:\n  \
                  0 - Success\n  \
                  1 - General error\n  \
                  2 - Lint/validation error\n  \
                  3 - Index/database error\n  \
                  4 - Configuration error\n  \
                  5 - Resource not found\n  \
                  6 - Circular dependency detected",
    propagate_version = true,
    arg_required_else_help = true,
    after_long_help = "For more information and documentation, visit: https://github.com/your-org/lash"
)]
pub struct LashCli {
    /// Override project root detection (defaults to searching for lash.index.md or .lash/)
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help_heading = "Global Options"
    )]
    pub root: Option<PathBuf>,

    /// Enable JSON output mode for machine-readable results
    #[arg(long, global = true, help_heading = "Global Options")]
    pub json: bool,

    /// Increase verbosity (can be specified multiple times: -v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count, help_heading = "Global Options")]
    pub verbose: u8,

    /// Suppress all non-essential output
    #[arg(
        short,
        long,
        global = true,
        conflicts_with = "verbose",
        help_heading = "Global Options"
    )]
    pub quiet: bool,

    /// Disable colored output
    #[arg(long, global = true, help_heading = "Global Options")]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Validate Lash task files for errors
    #[command(alias = "check")]
    Lint {
        /// Files or directories to lint (defaults to current project)
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,

        /// Apply auto-fixes where possible
        #[arg(long)]
        fix: bool,

        /// Run only specific rule(s) by code (can be specified multiple times)
        #[arg(long = "rule", value_name = "CODE")]
        rules: Vec<String>,

        /// Only show errors of this severity or higher
        #[arg(long, value_name = "LEVEL", value_enum)]
        min_severity: Option<SeverityLevel>,
    },

    /// Format Lash task files to normalize style
    #[command(alias = "fmt")]
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

    /// Rebuild the `SQLite` index from Markdown files
    #[command()]
    Index {
        /// Force full rebuild even if index is up to date
        #[arg(long)]
        force: bool,

        /// Show which files are being indexed
        #[arg(long)]
        show_files: bool,
    },

    /// Verify that `SQLite` index matches Markdown files
    #[command()]
    CheckIndex {
        /// Show detailed diff of inconsistencies
        #[arg(long)]
        diff: bool,
    },

    /// List tasks matching specified criteria
    List {
        /// Filter by label (can be specified multiple times)
        #[arg(long, value_name = "LABEL")]
        label: Vec<String>,

        /// Filter by status
        #[arg(long, value_name = "STATUS", value_enum)]
        status: Option<TaskStatus>,

        /// Filter by path prefix
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,

        /// Only show blocked tasks
        #[arg(long)]
        blocked: bool,

        /// Filter by owner
        #[arg(long, value_name = "NAME")]
        owner: Option<String>,

        /// Output format
        #[arg(long, value_name = "FORMAT", value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Search tasks by keyword or phrase
    Search {
        /// Search query
        query: String,

        /// Maximum number of results to show
        #[arg(long, short = 'n', default_value = "20")]
        limit: usize,

        /// Fuzzy matching threshold (0.0 = exact, 1.0 = very fuzzy)
        #[arg(long, default_value = "0.3")]
        threshold: f32,
    },

    /// Show detailed information about a specific task or file
    Show {
        /// Task ID or file path
        target: String,

        /// Show dependency tree
        #[arg(long)]
        deps: bool,

        /// Show reverse dependencies (tasks that depend on this)
        #[arg(long)]
        rdeps: bool,
    },

    /// Output dependency graph in various formats
    Graph {
        /// Output format
        #[arg(long, value_enum, default_value = "dot")]
        format: GraphFormat,

        /// Scope to specific path or label
        #[arg(long)]
        scope: Option<String>,

        /// Hide completed tasks from the graph
        #[arg(long)]
        hide_completed: bool,

        /// Output file (defaults to stdout)
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },

    /// Check for broken links and references
    CheckLinks {
        /// Fix broken links automatically if possible
        #[arg(long)]
        fix: bool,
    },

    /// Generate optimized prompts for AI agents
    AgentPrompt {
        /// Prompt format
        #[arg(long, value_enum, default_value = "plain")]
        format: AgentFormat,

        /// Include only tasks matching these labels
        #[arg(long)]
        label: Vec<String>,

        /// Include only tasks from this path
        #[arg(long)]
        path: Option<PathBuf>,

        /// Maximum token budget (approximate)
        #[arg(long)]
        max_tokens: Option<usize>,
    },

    /// Launch the terminal UI
    Tui,

    /// Generate shell completions
    #[command(hide = true)]
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

/// Severity levels for diagnostics
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SeverityLevel {
    Error,
    Warning,
    Info,
    Hint,
}

/// Task status values
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TaskStatus {
    Open,
    Done,
    Waived,
    Blocked,
}

/// Output format options
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    JsonPretty,
}

/// Graph output formats
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GraphFormat {
    Dot,
    Mermaid,
    Json,
}

/// Agent prompt formats
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AgentFormat {
    Plain,
    Json,
    ClaudeSkill,
    AgentsMd,
}

/// Shell types for completion generation
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_version() {
        let cli = LashCli::try_parse_from(["lash", "--version"]);
        // Version flag should cause early exit, so parse will fail
        assert!(cli.is_err());
    }

    #[test]
    fn test_cli_parse_lint() {
        let cli = LashCli::try_parse_from(["lash", "lint", "test.md"]).unwrap();
        assert!(matches!(cli.command, Commands::Lint { .. }));
        if let Commands::Lint { paths, .. } = cli.command {
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0], PathBuf::from("test.md"));
        }
    }

    #[test]
    fn test_cli_parse_lint_with_fix() {
        let cli = LashCli::try_parse_from(["lash", "lint", "--fix", "test.md"]).unwrap();
        if let Commands::Lint { fix, .. } = cli.command {
            assert!(fix);
        } else {
            panic!("Expected Lint command");
        }
    }

    #[test]
    fn test_cli_parse_format() {
        let cli = LashCli::try_parse_from(["lash", "format", "--check"]).unwrap();
        assert!(matches!(cli.command, Commands::Format { .. }));
        if let Commands::Format { check, .. } = cli.command {
            assert!(check);
        }
    }

    #[test]
    fn test_cli_parse_global_flags() {
        let cli = LashCli::try_parse_from([
            "lash",
            "--root",
            "/tmp",
            "--json",
            "--verbose",
            "lint",
            "test.md",
        ])
        .unwrap();
        assert_eq!(cli.root, Some(PathBuf::from("/tmp")));
        assert!(cli.json);
        assert_eq!(cli.verbose, 1);
    }

    #[test]
    fn test_cli_parse_multiple_verbose() {
        let cli = LashCli::try_parse_from(["lash", "-vvv", "index"]).unwrap();
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn test_cli_parse_quiet_conflicts_with_verbose() {
        let result = LashCli::try_parse_from(["lash", "-v", "--quiet", "lint"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parse_index() {
        let cli = LashCli::try_parse_from(["lash", "index"]).unwrap();
        assert!(matches!(cli.command, Commands::Index { .. }));
    }

    #[test]
    fn test_cli_parse_list_with_filters() {
        let cli = LashCli::try_parse_from([
            "lash",
            "list",
            "--label",
            "backend",
            "--status",
            "open",
            "--blocked",
        ])
        .unwrap();
        if let Commands::List {
            label,
            status,
            blocked,
            ..
        } = cli.command
        {
            assert_eq!(label, vec!["backend"]);
            assert!(matches!(status, Some(TaskStatus::Open)));
            assert!(blocked);
        } else {
            panic!("Expected List command");
        }
    }

    #[test]
    fn test_cli_parse_search() {
        let cli = LashCli::try_parse_from(["lash", "search", "implement parser"]).unwrap();
        if let Commands::Search { query, .. } = cli.command {
            assert_eq!(query, "implement parser");
        } else {
            panic!("Expected Search command");
        }
    }

    #[test]
    fn test_cli_parse_show() {
        let cli = LashCli::try_parse_from(["lash", "show", "task:123", "--deps"]).unwrap();
        if let Commands::Show { target, deps, .. } = cli.command {
            assert_eq!(target, "task:123");
            assert!(deps);
        } else {
            panic!("Expected Show command");
        }
    }

    #[test]
    fn test_cli_parse_graph() {
        let cli =
            LashCli::try_parse_from(["lash", "graph", "--format", "mermaid", "-o", "graph.mmd"])
                .unwrap();
        if let Commands::Graph { format, output, .. } = cli.command {
            assert!(matches!(format, GraphFormat::Mermaid));
            assert_eq!(output, Some(PathBuf::from("graph.mmd")));
        } else {
            panic!("Expected Graph command");
        }
    }

    #[test]
    fn test_cli_parse_agent_prompt() {
        let cli = LashCli::try_parse_from([
            "lash",
            "agent-prompt",
            "--format",
            "json",
            "--max-tokens",
            "1000",
        ])
        .unwrap();
        if let Commands::AgentPrompt {
            format, max_tokens, ..
        } = cli.command
        {
            assert!(matches!(format, AgentFormat::Json));
            assert_eq!(max_tokens, Some(1000));
        } else {
            panic!("Expected AgentPrompt command");
        }
    }

    #[test]
    fn test_cli_alias_check_for_lint() {
        let cli = LashCli::try_parse_from(["lash", "check", "test.md"]).unwrap();
        assert!(matches!(cli.command, Commands::Lint { .. }));
    }

    #[test]
    fn test_cli_alias_fmt_for_format() {
        let cli = LashCli::try_parse_from(["lash", "fmt"]).unwrap();
        assert!(matches!(cli.command, Commands::Format { .. }));
    }
}
