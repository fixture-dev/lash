//! CLI argument parsing structures
//!
//! This module defines the command-line interface structure using `clap`,
//! including all subcommands, global flags, and argument specifications.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// The Lash logo for display in help output
const LOGO_FOR_HELP: &str = "\
┓    ┓
┃ ┏┓┏┣┓
┗┛┗┻┛┛┗
";

/// Minimalist Markdown-native task tracker for devs and agents
#[derive(Parser, Debug)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are inherently boolean
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
    before_help = LOGO_FOR_HELP,
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

    /// Color scheme to use (e.g., "Nord", "Dracula", "Solarized Dark")
    #[arg(
        short = 'c',
        long,
        global = true,
        value_name = "SCHEME",
        help_heading = "Global Options"
    )]
    pub color_scheme: Option<String>,

    /// Enable tree view display
    #[arg(
        long = "tree",
        visible_alias = "tree-view",
        global = true,
        conflicts_with = "no_tree_view",
        help_heading = "Global Options"
    )]
    pub tree_view: bool,

    /// Disable tree view display
    #[arg(
        long = "no-tree",
        visible_alias = "no-tree-view",
        global = true,
        help_heading = "Global Options"
    )]
    pub no_tree_view: bool,

    /// Maximum depth for tree view display (1-10)
    #[arg(
        short = 'd',
        long,
        global = true,
        value_name = "DEPTH",
        help_heading = "Global Options"
    )]
    pub max_depth: Option<usize>,

    /// Force ASCII mode for tree characters instead of Unicode
    #[arg(long, global = true, help_heading = "Global Options")]
    pub ascii: bool,

    /// Suppress the Lash logo in CLI output
    #[arg(long, global = true, help_heading = "Global Options")]
    pub no_logo: bool,

    /// Show errors as they occur (streaming) vs at end (batch)
    #[arg(long, global = true, help_heading = "Global Options")]
    pub errors_streaming: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
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

        /// Confirm each fix before applying (requires --fix)
        #[arg(short = 'i', long)]
        interactive: bool,

        /// Show fix suggestions without applying them
        #[arg(long)]
        suggest: bool,

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
        /// Files or directories to index (defaults to current project)
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,

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
        /// Files or directories to verify (defaults to current project)
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,

        /// Show detailed diff of inconsistencies
        #[arg(long)]
        diff: bool,
    },

    /// List tasks matching specified criteria
    List {
        /// Filter by task ID (supports fuzzy matching)
        #[arg(long, short = 'f', visible_alias = "id", value_name = "ID")]
        filter: Option<String>,

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

        /// Filter by files/tasks that reference a specific document
        #[arg(long, value_name = "DOC_PATH")]
        docs: Option<String>,

        /// Show file descriptions (truncated to 100 chars)
        #[arg(long)]
        show_descriptions: bool,

        /// Show contextual notes for tasks
        #[arg(long)]
        show_notes: bool,

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

        /// Filter by label (can be specified multiple times)
        #[arg(long, value_name = "LABEL")]
        label: Vec<String>,

        /// Filter by status
        #[arg(long, value_name = "STATUS", value_enum)]
        status: Option<TaskStatus>,

        /// Filter by owner
        #[arg(long, value_name = "NAME")]
        owner: Option<String>,

        /// Filter by path prefix
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
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
        #[arg(long, value_enum, default_value = "ascii")]
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
        /// Attempt to fix broken links using fuzzy matching
        #[arg(long)]
        fix: bool,

        /// Auto-accept high-confidence fixes (requires --fix)
        #[arg(long, requires = "fix")]
        yes: bool,

        /// Show what would be fixed without applying changes (requires --fix)
        #[arg(long, requires = "fix")]
        dry_run: bool,
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

        /// Include file descriptions in the prompt (default: true)
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        include_descriptions: bool,

        /// Include contextual notes in the prompt (default: true)
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        include_notes: bool,
    },

    /// Launch the terminal UI
    Tui,

    /// Initialize a new Lash project in the current directory
    #[command()]
    Init {
        /// Target directory path (defaults to current directory)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,

        /// Create index file only (skip running index command)
        #[arg(long)]
        no_index: bool,

        /// Force re-initialization even if project already exists
        #[arg(long)]
        force: bool,
    },

    /// Initialize a demo playground project
    #[command()]
    Playground {
        /// Playground subcommand
        #[command(subcommand)]
        command: PlaygroundCommand,
    },

    /// Generate shell completions
    #[command(hide = true)]
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Explain a specific error code in detail
    #[command()]
    Explain {
        /// The error code to explain (e.g., `E_PARSE_INVALID_CHECKBOX`)
        #[arg(required_unless_present = "list")]
        code: Option<String>,

        /// List all available error codes
        #[arg(long)]
        list: bool,
    },

    /// Manage configuration settings
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Create a new task
    Add {
        /// The task title (required)
        #[arg(required = true)]
        title: String,

        /// Target file path (creates if doesn't exist)
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Title for new file header (only used when creating new file)
        #[arg(long)]
        file_title: Option<String>,

        /// Description for new file's ## Description section
        #[arg(long)]
        file_description: Option<String>,

        /// Parent task ID
        #[arg(short, long)]
        parent: Option<String>,

        /// Insert after this task ID
        #[arg(long)]
        after: Option<String>,

        /// Insert before this task ID
        #[arg(long)]
        before: Option<String>,

        /// Labels (comma-separated, repeatable: -l backend -l urgent)
        #[arg(short, long, value_delimiter = ',')]
        label: Vec<String>,

        /// Task owner
        #[arg(short, long)]
        owner: Option<String>,

        /// Time estimate (e.g., 30m, 2h, 1d, 2w)
        #[arg(short, long)]
        estimate: Option<String>,

        /// Initial status (open, done, waived, blocked)
        #[arg(long, default_value = "open")]
        status: String,

        /// Explicit task ID
        #[arg(long)]
        id: Option<String>,

        /// Dependencies (comma-separated, repeatable)
        #[arg(long, value_delimiter = ',')]
        depends_on: Vec<String>,

        /// Agent note text
        #[arg(long)]
        agent_note: Option<String>,

        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,

        /// Validate without creating
        #[arg(long)]
        dry_run: bool,

        /// Interactive mode (prompt for missing fields)
        #[arg(short, long)]
        interactive: bool,
    },
}

/// Configuration subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum ConfigCommand {
    /// Get a configuration value
    Get {
        /// The configuration key (e.g., `output.default_format`, `linter.max_depth`)
        key: String,
    },
    /// Set a configuration value
    Set {
        /// The configuration key
        key: String,
        /// The value to set
        value: String,
        /// Write to user config (~/.config/lash/config.toml) instead of project config
        #[arg(long)]
        user: bool,
    },
    /// List all configuration settings
    List {
        /// Show only values that differ from defaults
        #[arg(long)]
        changed: bool,
    },
}

/// Playground subcommands
#[derive(Subcommand, Debug)]
pub enum PlaygroundCommand {
    /// Initialize a demo playground project with realistic sample data
    Init {
        /// Target directory path (defaults to ./playground/)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,

        /// Delete and regenerate if playground already exists
        #[arg(long)]
        reset: bool,
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
    /// ASCII/Unicode box-drawing format for terminal display (default)
    Ascii,
    /// Graphviz DOT format
    Dot,
    /// Mermaid diagram format
    Mermaid,
    /// JSON format for programmatic access
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
    fn test_cli_parse_lint_with_interactive() {
        let cli =
            LashCli::try_parse_from(["lash", "lint", "--fix", "--interactive", "test.md"]).unwrap();
        if let Commands::Lint {
            fix, interactive, ..
        } = cli.command
        {
            assert!(fix);
            assert!(interactive);
        } else {
            panic!("Expected Lint command");
        }
    }

    #[test]
    fn test_cli_parse_lint_with_interactive_short() {
        let cli = LashCli::try_parse_from(["lash", "lint", "--fix", "-i", "test.md"]).unwrap();
        if let Commands::Lint {
            fix, interactive, ..
        } = cli.command
        {
            assert!(fix);
            assert!(interactive);
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
        if let Commands::Index { paths, force, .. } = cli.command {
            assert!(paths.is_empty());
            assert!(!force);
        }
    }

    #[test]
    fn test_cli_parse_index_with_paths() {
        let cli = LashCli::try_parse_from(["lash", "index", "tasks/"]).unwrap();
        if let Commands::Index { paths, .. } = cli.command {
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0], PathBuf::from("tasks/"));
        } else {
            panic!("Expected Index command");
        }
    }

    #[test]
    fn test_cli_parse_index_with_multiple_paths() {
        let cli =
            LashCli::try_parse_from(["lash", "index", "tasks/", "projects/", "--force"]).unwrap();
        if let Commands::Index { paths, force, .. } = cli.command {
            assert_eq!(paths.len(), 2);
            assert_eq!(paths[0], PathBuf::from("tasks/"));
            assert_eq!(paths[1], PathBuf::from("projects/"));
            assert!(force);
        } else {
            panic!("Expected Index command");
        }
    }

    #[test]
    fn test_cli_parse_check_index() {
        let cli = LashCli::try_parse_from(["lash", "check-index"]).unwrap();
        assert!(matches!(cli.command, Commands::CheckIndex { .. }));
        if let Commands::CheckIndex { paths, diff } = cli.command {
            assert!(paths.is_empty());
            assert!(!diff);
        }
    }

    #[test]
    fn test_cli_parse_check_index_with_paths() {
        let cli = LashCli::try_parse_from(["lash", "check-index", "tasks/", "--diff"]).unwrap();
        if let Commands::CheckIndex { paths, diff } = cli.command {
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0], PathBuf::from("tasks/"));
            assert!(diff);
        } else {
            panic!("Expected CheckIndex command");
        }
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

    #[test]
    fn test_cli_parse_playground_init() {
        let cli = LashCli::try_parse_from(["lash", "playground", "init"]).unwrap();
        assert!(matches!(cli.command, Commands::Playground { .. }));
        if let Commands::Playground { command } = cli.command {
            assert!(matches!(command, PlaygroundCommand::Init { .. }));
        }
    }

    #[test]
    fn test_cli_parse_playground_init_with_path() {
        let cli =
            LashCli::try_parse_from(["lash", "playground", "init", "--path", "/tmp/demo"]).unwrap();
        if let Commands::Playground {
            command: PlaygroundCommand::Init { path, .. },
        } = cli.command
        {
            assert_eq!(path, Some(PathBuf::from("/tmp/demo")));
        } else {
            panic!("Expected Playground Init command");
        }
    }

    #[test]
    fn test_cli_parse_playground_init_with_reset() {
        let cli = LashCli::try_parse_from(["lash", "playground", "init", "--reset"]).unwrap();
        if let Commands::Playground {
            command: PlaygroundCommand::Init { reset, .. },
        } = cli.command
        {
            assert!(reset);
        } else {
            panic!("Expected Playground Init command");
        }
    }

    #[test]
    fn test_cli_parse_tree_flags() {
        // Test new --tree flag
        let cli = LashCli::try_parse_from(["lash", "--tree", "list"]).unwrap();
        assert!(cli.tree_view);
        assert!(!cli.no_tree_view);

        // Test --no-tree flag
        let cli = LashCli::try_parse_from(["lash", "--no-tree", "list"]).unwrap();
        assert!(!cli.tree_view);
        assert!(cli.no_tree_view);

        // Test backward compatibility with --tree-view alias
        let cli = LashCli::try_parse_from(["lash", "--tree-view", "list"]).unwrap();
        assert!(cli.tree_view);

        // Test backward compatibility with --no-tree-view alias
        let cli = LashCli::try_parse_from(["lash", "--no-tree-view", "list"]).unwrap();
        assert!(cli.no_tree_view);

        // Test conflict between --tree and --no-tree
        let result = LashCli::try_parse_from(["lash", "--tree", "--no-tree", "list"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parse_max_depth() {
        let cli = LashCli::try_parse_from(["lash", "--max-depth", "3", "list"]).unwrap();
        assert_eq!(cli.max_depth, Some(3));

        let cli = LashCli::try_parse_from(["lash", "-d", "10", "list"]).unwrap();
        assert_eq!(cli.max_depth, Some(10));
    }

    #[test]
    fn test_cli_parse_ascii_flag() {
        let cli = LashCli::try_parse_from(["lash", "--ascii", "list"]).unwrap();
        assert!(cli.ascii);

        let cli = LashCli::try_parse_from(["lash", "list"]).unwrap();
        assert!(!cli.ascii);
    }

    #[test]
    fn test_cli_parse_combined_tree_flags() {
        // Test with new --tree flag
        let cli =
            LashCli::try_parse_from(["lash", "--tree", "--max-depth", "7", "--ascii", "list"])
                .unwrap();
        assert!(cli.tree_view);
        assert_eq!(cli.max_depth, Some(7));
        assert!(cli.ascii);

        // Test with backward-compatible --tree-view alias
        let cli = LashCli::try_parse_from(["lash", "--tree-view", "-d", "5", "list"]).unwrap();
        assert!(cli.tree_view);
        assert_eq!(cli.max_depth, Some(5));
    }

    #[test]
    fn test_cli_parse_add_basic() {
        let cli = LashCli::try_parse_from(["lash", "add", "Test task"]).unwrap();
        if let Commands::Add { title, .. } = cli.command {
            assert_eq!(title, "Test task");
        } else {
            panic!("Expected Add command");
        }
    }

    #[test]
    fn test_cli_parse_add_with_options() {
        let cli = LashCli::try_parse_from([
            "lash",
            "add",
            "Test task",
            "--file",
            "tasks.md",
            "--label",
            "backend",
            "--owner",
            "alice",
        ])
        .unwrap();
        if let Commands::Add {
            title,
            file,
            label,
            owner,
            ..
        } = cli.command
        {
            assert_eq!(title, "Test task");
            assert_eq!(file, Some(PathBuf::from("tasks.md")));
            assert_eq!(label, vec!["backend"]);
            assert_eq!(owner, Some("alice".to_string()));
        } else {
            panic!("Expected Add command");
        }
    }

    #[test]
    fn test_cli_parse_add_with_parent() {
        let cli = LashCli::try_parse_from(["lash", "add", "Child task", "--parent", "parent-id"])
            .unwrap();
        if let Commands::Add { title, parent, .. } = cli.command {
            assert_eq!(title, "Child task");
            assert_eq!(parent, Some("parent-id".to_string()));
        } else {
            panic!("Expected Add command");
        }
    }

    #[test]
    fn test_cli_parse_add_with_position() {
        let cli = LashCli::try_parse_from(["lash", "add", "Task", "--after", "task-1"]).unwrap();
        if let Commands::Add { title, after, .. } = cli.command {
            assert_eq!(title, "Task");
            assert_eq!(after, Some("task-1".to_string()));
        } else {
            panic!("Expected Add command");
        }
    }

    #[test]
    fn test_cli_parse_add_dry_run() {
        let cli = LashCli::try_parse_from(["lash", "add", "Task", "--dry-run"]).unwrap();
        if let Commands::Add { title, dry_run, .. } = cli.command {
            assert_eq!(title, "Task");
            assert!(dry_run);
        } else {
            panic!("Expected Add command");
        }
    }
}
