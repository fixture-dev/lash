//! Agent prompt command implementation
//!
//! The `lash agent-prompt` command generates optimized prompts for AI agents
//! to use Lash effectively. It provides schema documentation, examples, and
//! filtered task lists in various output formats.

use anyhow::{Context, Result};
use lash_agent::{PromptBuilder, PromptConfig, PromptFormat as AgentPromptFormat};
use lash_cli::cli::AgentFormat;
use lash_db::{open_database, FileRepository, TaskRepository};
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

/// Arguments for the agent-prompt command
#[derive(Debug, Clone)]
pub struct AgentPromptArgs {
    /// Output format
    pub format: AgentFormat,
    /// Filter tasks by labels
    pub labels: Vec<String>,
    /// Filter tasks by path
    pub path: Option<PathBuf>,
    /// Maximum token budget (approximate)
    pub max_tokens: Option<usize>,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Output in JSON format
    #[allow(dead_code)] // Reserved for future JSON output formatting
    pub json: bool,
    /// Disable colored output
    #[allow(dead_code)] // Reserved for future colored output
    pub no_color: bool,
}

/// Execute the agent-prompt command
///
/// # Arguments
///
/// * `args` - Agent prompt command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (general error), 3 (DB error)
///
/// # Errors
///
/// Returns an error if:
/// - Project root cannot be found
/// - Database does not exist or cannot be opened (if task filtering is requested)
/// - Query execution fails
pub fn execute(args: &AgentPromptArgs) -> Result<i32> {
    // Determine project root
    let project_root = if let Some(ref root) = args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        format = ?args.format,
        "Generating agent prompt"
    );

    // Convert CLI format to agent library format
    let prompt_format = match args.format {
        AgentFormat::Plain => AgentPromptFormat::Plain,
        AgentFormat::Json => AgentPromptFormat::Json,
        AgentFormat::ClaudeSkill => AgentPromptFormat::ClaudeSkill,
        AgentFormat::AgentsMd => AgentPromptFormat::AgentsMd,
    };

    // Create prompt configuration
    let config = PromptConfig {
        format: prompt_format,
        include_examples: true,
        include_tasks: true,
        token_budget: args.max_tokens,
        label_filter: args.labels.clone(),
        path_filter: args.path.as_ref().map(|p| p.display().to_string()),
    };

    // Load task summaries if we have filters or want to include tasks
    let task_summaries = if config.include_tasks && (!args.labels.is_empty() || args.path.is_some())
    {
        load_task_summaries(&project_root, &args.labels, args.path.as_deref())?
    } else if config.include_tasks {
        // Load all task summaries
        load_task_summaries(&project_root, &[], None)?
    } else {
        Vec::new()
    };

    // Build prompt
    let mut builder = PromptBuilder::new(config);
    for summary in task_summaries {
        builder.add_task_summary(summary);
    }

    let prompt = builder.build();

    // Output the prompt
    println!("{}", prompt.content);

    // Log token usage at debug level
    tracing::debug!(
        tokens = prompt.token_count,
        truncated = prompt.truncated,
        "Prompt generated"
    );

    if prompt.truncated && !args.json {
        eprintln!(
            "\nNote: Content was truncated to fit within token budget (estimated {} tokens)",
            prompt.token_count
        );
    }

    Ok(0)
}

/// Load task summaries from the database
fn load_task_summaries(
    project_root: &Path,
    label_filter: &[String],
    path_filter: Option<&Path>,
) -> Result<Vec<String>> {
    let db_path = get_database_path(project_root);

    // If database doesn't exist, return empty summaries
    if !db_path.exists() {
        tracing::warn!("Database not found, skipping task summaries");
        return Ok(Vec::new());
    }

    // Open database and create repositories
    let conn = open_database(&db_path).context("Failed to open database")?;
    let file_repo = FileRepository::new(&conn);
    let task_repo = TaskRepository::new(&conn);

    // Get all files
    let files = file_repo.list_all().context("Failed to list files")?;

    let mut summaries = Vec::new();

    for file in files {
        // Apply path filter if specified
        if let Some(filter_path) = path_filter {
            if !file
                .path
                .starts_with(filter_path.display().to_string().as_str())
            {
                continue;
            }
        }

        // Build filter for this file
        let filter = lash_db::repository::tasks::TaskFilter {
            status: None,
            labels: label_filter.to_vec(),
            owner: None,
            file_path: Some(file.path.display().to_string()),
            blocked: None,
        };

        // Load tasks for this file
        let tasks = task_repo.find(&filter).context("Failed to load tasks")?;

        if tasks.is_empty() {
            continue;
        }

        // Calculate statistics
        let total = tasks.len();
        let completed = tasks.iter().filter(|t| t.status.as_str() == "done").count();
        let open = tasks.iter().filter(|t| t.status.as_str() == "open").count();
        let blocked = tasks
            .iter()
            .filter(|t| t.status.as_str() == "blocked")
            .count();

        // Generate summary using token utilities
        let summary = lash_agent::tokens::summarize_task_file(
            &file.path.display().to_string(),
            total,
            completed,
            open,
            blocked,
        );
        summaries.push(summary);
    }

    Ok(summaries)
}

/// Get the database path for a project root
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/db.sqlite")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_prompt_args_structure() {
        let args = AgentPromptArgs {
            format: AgentFormat::Plain,
            labels: vec!["test".to_string()],
            path: None,
            max_tokens: Some(1000),
            project_root: None,
            json: false,
            no_color: false,
        };

        assert_eq!(args.max_tokens, Some(1000));
        assert_eq!(args.labels.len(), 1);
    }

    #[test]
    fn test_format_conversion() {
        // Test that we can convert CLI formats to agent library formats
        let cli_formats = vec![
            AgentFormat::Plain,
            AgentFormat::Json,
            AgentFormat::ClaudeSkill,
            AgentFormat::AgentsMd,
        ];

        for format in cli_formats {
            let _agent_format = match format {
                AgentFormat::Plain => AgentPromptFormat::Plain,
                AgentFormat::Json => AgentPromptFormat::Json,
                AgentFormat::ClaudeSkill => AgentPromptFormat::ClaudeSkill,
                AgentFormat::AgentsMd => AgentPromptFormat::AgentsMd,
            };
            // No panic means conversion works
        }
    }

    #[test]
    fn test_get_database_path() {
        let root = PathBuf::from("/tmp/test-project");
        let db_path = get_database_path(&root);
        assert_eq!(db_path, PathBuf::from("/tmp/test-project/.lash/db.sqlite"));
    }
}
