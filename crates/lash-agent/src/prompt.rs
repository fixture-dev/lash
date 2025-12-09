//! Prompt generation and template system for AI agents
//!
//! This module provides the core prompt building functionality that combines
//! schema, examples, and task context into agent-friendly prompts.

use crate::schema::{
    generate_dependency_example, generate_doc_reference_example, generate_minimal_example,
    generate_schema_text,
};
use crate::tokens::{distribute_budget, estimate_tokens, truncate_to_budget};

/// Generate CLI commands reference text
fn generate_cli_commands_text() -> String {
    r"## CLI Commands

```
lash lint [PATH...]       Validate task files for format/semantic errors
lash format [PATH...]     Normalize formatting of task files
lash index                Rebuild SQLite index from Markdown files
lash list [FILTERS]       List tasks (--label, --status, --owner, --path)
lash search <QUERY>       Fuzzy search tasks by keyword
lash show <ID>            Show details for a task or file
lash graph                Output dependency graph (--format dot|mermaid|json)
lash check-links          Find broken @depends-on and @doc references
lash tui                  Launch interactive terminal UI
```

"
    .to_string()
}

/// A documentation reference for inclusion in agent prompts
///
/// Represents a link to documentation with optional validity status.
#[derive(Debug, Clone)]
pub struct DocRefInfo {
    /// Relative path to the document
    pub path: String,
    /// Optional fragment identifier (section anchor)
    pub fragment: Option<String>,
    /// Whether the referenced file exists (None if not checked)
    pub valid: Option<bool>,
}

impl DocRefInfo {
    /// Create a new doc ref info
    #[must_use]
    pub fn new(path: impl Into<String>, fragment: Option<String>) -> Self {
        Self {
            path: path.into(),
            fragment,
            valid: None,
        }
    }

    /// Mark this doc ref as valid or invalid
    #[must_use]
    pub fn with_validity(mut self, valid: bool) -> Self {
        self.valid = Some(valid);
        self
    }

    /// Format the doc ref for display
    ///
    /// Includes a `[missing]` marker if the ref is known to be invalid.
    #[must_use]
    pub fn display(&self) -> String {
        let mut result = self.path.clone();
        if let Some(ref frag) = self.fragment {
            result.push('#');
            result.push_str(frag);
        }
        if self.valid == Some(false) {
            result.push_str(" [missing]");
        }
        result
    }
}

/// A task file summary with associated documentation references
#[derive(Debug, Clone)]
pub struct TaskFileSummary {
    /// Path to the task file
    pub path: String,
    /// Total number of tasks
    pub total: usize,
    /// Number of completed tasks
    pub completed: usize,
    /// Number of open tasks
    pub open: usize,
    /// Number of blocked tasks
    pub blocked: usize,
    /// Documentation references associated with this file
    pub doc_refs: Vec<DocRefInfo>,
}

impl TaskFileSummary {
    /// Create a new task file summary
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            total: 0,
            completed: 0,
            open: 0,
            blocked: 0,
            doc_refs: Vec::new(),
        }
    }

    /// Set task counts
    #[must_use]
    pub fn with_counts(
        mut self,
        total: usize,
        completed: usize,
        open: usize,
        blocked: usize,
    ) -> Self {
        self.total = total;
        self.completed = completed;
        self.open = open;
        self.blocked = blocked;
        self
    }

    /// Add documentation references
    #[must_use]
    pub fn with_doc_refs(mut self, doc_refs: Vec<DocRefInfo>) -> Self {
        self.doc_refs = doc_refs;
        self
    }

    /// Format as a compact summary string
    #[must_use]
    pub fn to_summary_string(&self) -> String {
        let percent = if self.total > 0 {
            (self.completed as f64 / self.total as f64 * 100.0) as usize
        } else {
            0
        };

        let mut summary = format!("{}: {} tasks, {}% complete", self.path, self.total, percent);

        if self.blocked > 0 {
            summary.push_str(&format!(", {} blocked", self.blocked));
        }

        summary
    }

    /// Format with inline doc refs for agent prompt output
    #[must_use]
    pub fn format_with_docs(&self) -> String {
        let mut output = format!("- {}\n", self.to_summary_string());

        for doc_ref in &self.doc_refs {
            output.push_str(&format!("  - Doc: {}\n", doc_ref.display()));
        }

        output
    }
}

/// Output format for agent prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptFormat {
    /// Plain text Markdown format
    Plain,
    /// JSON structured format
    Json,
    /// Claude Code skill specification
    ClaudeSkill,
    /// Ready-to-paste Markdown fragment for agents.md files
    AgentsMd,
}

/// Configuration for prompt generation
#[derive(Debug, Clone)]
pub struct PromptConfig {
    /// Output format
    pub format: PromptFormat,
    /// Include examples in the prompt
    pub include_examples: bool,
    /// Include current project tasks
    pub include_tasks: bool,
    /// Token budget (None = unlimited)
    pub token_budget: Option<usize>,
    /// Labels to filter tasks by
    pub label_filter: Vec<String>,
    /// Path to filter tasks by
    pub path_filter: Option<String>,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            format: PromptFormat::Plain,
            include_examples: true,
            include_tasks: true,
            token_budget: None,
            label_filter: Vec::new(),
            path_filter: None,
        }
    }
}

/// A generated prompt for an AI agent
#[derive(Debug, Clone)]
pub struct AgentPrompt {
    /// The prompt content
    pub content: String,
    /// Estimated token count
    pub token_count: usize,
    /// Whether content was truncated to fit budget
    pub truncated: bool,
}

/// Builder for constructing agent prompts
pub struct PromptBuilder {
    config: PromptConfig,
    task_summaries: Vec<String>,
    task_file_summaries: Vec<TaskFileSummary>,
    sparse_context: Option<String>,
}

impl PromptBuilder {
    /// Create a new prompt builder with the given configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::prompt::{PromptBuilder, PromptConfig};
    ///
    /// let config = PromptConfig::default();
    /// let builder = PromptBuilder::new(config);
    /// ```
    pub fn new(config: PromptConfig) -> Self {
        Self {
            config,
            task_summaries: Vec::new(),
            task_file_summaries: Vec::new(),
            sparse_context: None,
        }
    }

    /// Add a task summary to include in the prompt
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::prompt::{PromptBuilder, PromptConfig};
    ///
    /// let config = PromptConfig::default();
    /// let mut builder = PromptBuilder::new(config);
    /// builder.add_task_summary("features/auth.md: 10 tasks, 70% complete".to_string());
    /// ```
    pub fn add_task_summary(&mut self, summary: String) {
        self.task_summaries.push(summary);
    }

    /// Add a task file summary with documentation references
    ///
    /// Use this method instead of `add_task_summary` when you have
    /// documentation references to include inline with the task summary.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::prompt::{PromptBuilder, PromptConfig, TaskFileSummary, DocRefInfo};
    ///
    /// let config = PromptConfig::default();
    /// let mut builder = PromptBuilder::new(config);
    ///
    /// let summary = TaskFileSummary::new("features/auth.md")
    ///     .with_counts(10, 7, 2, 1)
    ///     .with_doc_refs(vec![
    ///         DocRefInfo::new("../docs/auth-spec.md", Some("oauth".to_string())),
    ///     ]);
    ///
    /// builder.add_task_file_summary(summary);
    /// ```
    pub fn add_task_file_summary(&mut self, summary: TaskFileSummary) {
        self.task_file_summaries.push(summary);
    }

    /// Add sparse context to include in the prompt
    ///
    /// When sparse context is provided, it will be included as a dedicated section
    /// in the prompt, replacing or supplementing the task summaries.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::prompt::{PromptBuilder, PromptConfig};
    /// use lash_agent::context::ContextBuilder;
    /// use lash_core::dependency::{DependencyGraph, NodeData};
    /// use lash_types::TaskStatus;
    ///
    /// let mut graph = DependencyGraph::new();
    /// graph.add_node(
    ///     "test#task1".to_string(),
    ///     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0)
    /// );
    ///
    /// let mut context_builder = ContextBuilder::new("test#task1");
    /// context_builder.with_graph(&graph);
    /// let sparse_context = context_builder.build();
    ///
    /// let config = PromptConfig::default();
    /// let mut builder = PromptBuilder::new(config);
    /// builder.set_sparse_context(sparse_context.content);
    /// let prompt = builder.build();
    ///
    /// assert!(prompt.content.contains("Sparse Context"));
    /// ```
    pub fn set_sparse_context(&mut self, context: String) {
        self.sparse_context = Some(context);
    }

    /// Build the final prompt
    ///
    /// Generates the complete prompt according to the configuration,
    /// applying token budgets if specified.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::prompt::{PromptBuilder, PromptConfig, PromptFormat};
    ///
    /// let mut config = PromptConfig::default();
    /// config.format = PromptFormat::Plain;
    /// let builder = PromptBuilder::new(config);
    /// let prompt = builder.build();
    /// assert!(prompt.token_count > 0);
    /// ```
    pub fn build(self) -> AgentPrompt {
        match self.config.format {
            PromptFormat::Plain => self.build_plain(),
            PromptFormat::Json => self.build_json(),
            PromptFormat::ClaudeSkill => self.build_claude_skill(),
            PromptFormat::AgentsMd => self.build_agents_md(),
        }
    }

    fn build_plain(self) -> AgentPrompt {
        let mut sections = Vec::new();

        // Header
        sections.push((
            "header",
            "# Lash Agent Usage Guide\n\n".to_string(),
            10, // priority
        ));

        // Overview
        let overview = r"## Overview

Lash is a minimalist, Markdown-native task tracker where:
- Markdown files are the single source of truth
- Tasks are hierarchical checkbox lists with annotations
- Directory structure provides implicit hierarchy (parent directories depend on children)
- SQLite provides fast indexing and search (fully reconstructible from Markdown)
- Format is strictly enforced by linting for predictability

"
        .to_string();
        sections.push(("overview", overview, 10));

        // Schema
        let schema_text = generate_schema_text();
        sections.push(("schema", format!("## File Format\n\n{schema_text}\n"), 10));

        // Examples
        if self.config.include_examples {
            let mut examples_text = String::from("## Examples\n\n");
            examples_text.push_str("### Minimal Valid File\n\n");
            examples_text.push_str("```markdown\n");
            examples_text.push_str(&generate_minimal_example());
            examples_text.push_str("```\n\n");
            examples_text.push_str("### File with Dependencies\n\n");
            examples_text.push_str("```markdown\n");
            examples_text.push_str(&generate_dependency_example());
            examples_text.push_str("```\n\n");
            examples_text.push_str("### File with Documentation References\n\n");
            examples_text.push_str("```markdown\n");
            examples_text.push_str(&generate_doc_reference_example());
            examples_text.push_str("```\n\n");
            sections.push(("examples", examples_text, 8));
        }

        // Sparse context (if provided, takes precedence over task summaries)
        if let Some(ref context) = self.sparse_context {
            let mut context_text = String::from("## Task Context\n\n");
            context_text.push_str(context);
            context_text.push('\n');
            sections.push(("sparse_context", context_text, 7));
        } else if self.config.include_tasks
            && (!self.task_summaries.is_empty() || !self.task_file_summaries.is_empty())
        {
            // Task summaries (only if no sparse context)
            let mut tasks_text = String::from("## Current Project Tasks\n\n");
            if !self.config.label_filter.is_empty() {
                tasks_text.push_str(&format!(
                    "Filtered by labels: {}\n\n",
                    self.config.label_filter.join(", ")
                ));
            }
            if let Some(ref path) = self.config.path_filter {
                tasks_text.push_str(&format!("Filtered by path: {path}\n\n"));
            }
            // Prefer task file summaries with doc refs if available
            if self.task_file_summaries.is_empty() {
                for summary in &self.task_summaries {
                    tasks_text.push_str(&format!("- {summary}\n"));
                }
            } else {
                for summary in &self.task_file_summaries {
                    tasks_text.push_str(&summary.format_with_docs());
                }
            }
            tasks_text.push('\n');
            sections.push(("tasks", tasks_text, 5));
        }

        // CLI commands
        sections.push(("cli_commands", generate_cli_commands_text(), 9));

        // Safety guidelines
        let safety = r"## Safety Guidelines

When working with Lash files:

1. **Always run `lash lint` after modifications** to validate your changes
2. **Respect depth limits** (3-4 levels maximum for task hierarchies)
3. **Don't break dependency references** - ensure `@depends-on` targets exist
4. **Maintain status consistency** - parent tasks complete only when children are done/waived
5. **Use unique IDs** within each file
6. **Run `lash index`** after making changes to update the search index
7. **Keep `@doc` references valid** - ensure referenced documentation files exist

"
        .to_string();
        sections.push(("safety", safety, 9));

        // Apply token budget if specified
        let (final_content, truncated) = if let Some(budget) = self.config.token_budget {
            Self::apply_budget(&sections, budget)
        } else {
            let content: String = sections.iter().map(|(_, text, _)| text.as_str()).collect();
            (content, false)
        };

        let token_count = estimate_tokens(&final_content);

        AgentPrompt {
            content: final_content,
            token_count,
            truncated,
        }
    }

    fn build_json(self) -> AgentPrompt {
        use crate::schema::generate_schema;

        let schema = generate_schema();

        let mut examples = Vec::new();
        if self.config.include_examples {
            examples.push(serde_json::json!({
                "name": "minimal",
                "content": generate_minimal_example(),
            }));
            examples.push(serde_json::json!({
                "name": "with_dependencies",
                "content": generate_dependency_example(),
            }));
            examples.push(serde_json::json!({
                "name": "with_doc_references",
                "content": generate_doc_reference_example(),
            }));
        }

        // Build task file summaries with doc refs for JSON output
        let task_files_json: Vec<serde_json::Value> =
            if self.config.include_tasks && !self.task_file_summaries.is_empty() {
                self.task_file_summaries
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "path": s.path,
                            "total": s.total,
                            "completed": s.completed,
                            "open": s.open,
                            "blocked": s.blocked,
                            "doc_refs": s.doc_refs.iter().map(|d| {
                                let mut obj = serde_json::json!({
                                    "path": d.path,
                                });
                                if let Some(ref frag) = d.fragment {
                                    obj["fragment"] = serde_json::json!(frag);
                                }
                                if let Some(valid) = d.valid {
                                    obj["valid"] = serde_json::json!(valid);
                                }
                                obj
                            }).collect::<Vec<_>>(),
                        })
                    })
                    .collect()
            } else {
                Vec::new()
            };

        let json_output = serde_json::json!({
            "format": "lash-agent-prompt",
            "version": "1.0",
            "schema": schema,
            "examples": examples,
            "tasks": if self.config.include_tasks && task_files_json.is_empty() {
                serde_json::json!(self.task_summaries)
            } else {
                serde_json::json!(null)
            },
            "task_files": if task_files_json.is_empty() {
                serde_json::json!(null)
            } else {
                serde_json::json!(task_files_json)
            },
            "filters": {
                "labels": self.config.label_filter,
                "path": self.config.path_filter,
            },
            "safety_guidelines": [
                "Always run `lash lint` after modifications",
                "Respect depth limits (3-4 levels)",
                "Don't break dependency references",
                "Maintain status consistency",
                "Use unique IDs within each file",
                "Run `lash index` after changes",
                "Keep @doc references valid",
            ],
        });

        let content =
            serde_json::to_string_pretty(&json_output).unwrap_or_else(|_| "{}".to_string());
        let token_count = estimate_tokens(&content);

        AgentPrompt {
            content,
            token_count,
            truncated: false,
        }
    }

    fn build_claude_skill(mut self) -> AgentPrompt {
        // Suppress clippy warning - we consume self for consistency with other methods
        self.task_summaries.clear();
        // Placeholder for Claude Code skill format
        // This would be a JSON/YAML spec defining skill commands
        let skill_spec = serde_json::json!({
            "name": "lash",
            "version": "1.0",
            "description": "Lash task tracker integration",
            "commands": [
                {
                    "name": "lint",
                    "description": "Validate task files",
                    "usage": "lash lint [paths...]"
                },
                {
                    "name": "index",
                    "description": "Update search index",
                    "usage": "lash index"
                },
                {
                    "name": "list",
                    "description": "List tasks with filters",
                    "usage": "lash list [--label <label>] [--status <status>]"
                },
                {
                    "name": "search",
                    "description": "Search tasks and files",
                    "usage": "lash search <query>"
                }
            ],
            "file_format": {
                "description": "Markdown with hierarchical checkboxes and annotations",
                "example": generate_minimal_example(),
            }
        });

        let content =
            serde_json::to_string_pretty(&skill_spec).unwrap_or_else(|_| "{}".to_string());
        let token_count = estimate_tokens(&content);

        AgentPrompt {
            content,
            token_count,
            truncated: false,
        }
    }

    fn build_agents_md(mut self) -> AgentPrompt {
        // Suppress clippy warning - we consume self for consistency with other methods
        self.task_summaries.clear();
        // Ready-to-paste Markdown fragment for agents.md files
        let mut content = String::from("## Using Lash\n\n");
        content.push_str(
            "Lash is a Markdown-native task tracker. All tasks live in `.md` files as checkbox lists.\n\n",
        );

        content.push_str("### Quick Start\n\n");
        content.push_str("```bash\n");
        content.push_str("# Validate files\n");
        content.push_str("lash lint\n\n");
        content.push_str("# List all tasks\n");
        content.push_str("lash list\n\n");
        content.push_str("# Search for tasks\n");
        content.push_str("lash search <query>\n\n");
        content.push_str("# Update search index\n");
        content.push_str("lash index\n");
        content.push_str("```\n\n");

        content.push_str("### File Format\n\n");
        content.push_str("```markdown\n");
        content.push_str(&generate_minimal_example());
        content.push_str("```\n\n");

        content.push_str("### Key Rules\n\n");
        content.push_str("- Task IDs must be unique within each file\n");
        content.push_str("- Maximum nesting depth: 3-4 levels\n");
        content.push_str("- Always run `lash lint` after editing\n");
        content.push_str("- Use `@depends-on` for cross-file dependencies\n");
        content.push_str("- Use `@doc` to link to documentation resources\n");

        let token_count = estimate_tokens(&content);

        AgentPrompt {
            content,
            token_count,
            truncated: false,
        }
    }

    fn apply_budget(sections: &[(&str, String, u8)], budget: usize) -> (String, bool) {
        // Estimate tokens for each section
        let section_estimates: Vec<(&str, usize, u8)> = sections
            .iter()
            .map(|(name, text, priority)| (*name, estimate_tokens(text), *priority))
            .collect();

        // Distribute budget across sections
        let allocations = distribute_budget(budget, &section_estimates);

        let mut final_content = String::new();
        let mut truncated = false;

        for ((_name, text, _priority), (_alloc_name, allocation)) in
            sections.iter().zip(allocations.iter())
        {
            if *allocation == 0 {
                truncated = true;
                continue;
            }

            let estimated = estimate_tokens(text);
            if estimated > *allocation {
                let truncated_text = truncate_to_budget(text, *allocation);
                final_content.push_str(&truncated_text);
                truncated = true;
            } else {
                final_content.push_str(text);
            }
        }

        (final_content, truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_config_default() {
        let config = PromptConfig::default();
        assert_eq!(config.format, PromptFormat::Plain);
        assert!(config.include_examples);
        assert!(config.include_tasks);
        assert!(config.token_budget.is_none());
    }

    #[test]
    fn test_prompt_builder_plain() {
        let config = PromptConfig::default();
        let builder = PromptBuilder::new(config);
        let prompt = builder.build();

        assert!(prompt.token_count > 0);
        assert!(!prompt.truncated);
        assert!(prompt.content.contains("# Lash Agent Usage Guide"));
        assert!(prompt.content.contains("## File Format"));
        assert!(prompt.content.contains("## Examples"));
        assert!(prompt.content.contains("## Safety Guidelines"));
    }

    #[test]
    fn test_prompt_builder_with_tasks() {
        let config = PromptConfig::default();
        let mut builder = PromptBuilder::new(config);
        builder.add_task_summary("features/auth.md: 10 tasks, 70% complete".to_string());
        builder.add_task_summary("core/api.md: 5 tasks, 100% complete".to_string());

        let prompt = builder.build();
        assert!(prompt.content.contains("features/auth.md"));
        assert!(prompt.content.contains("core/api.md"));
    }

    #[test]
    fn test_prompt_builder_json() {
        let config = PromptConfig {
            format: PromptFormat::Json,
            ..Default::default()
        };
        let builder = PromptBuilder::new(config);
        let prompt = builder.build();

        assert!(prompt.token_count > 0);
        assert!(prompt.content.contains("\"format\": \"lash-agent-prompt\""));
        assert!(prompt.content.contains("\"schema\""));

        // Verify it's valid JSON
        let _parsed: serde_json::Value = serde_json::from_str(&prompt.content).unwrap();
    }

    #[test]
    fn test_prompt_builder_claude_skill() {
        let config = PromptConfig {
            format: PromptFormat::ClaudeSkill,
            ..Default::default()
        };
        let builder = PromptBuilder::new(config);
        let prompt = builder.build();

        assert!(prompt.content.contains("\"name\": \"lash\""));
        assert!(prompt.content.contains("\"commands\""));

        // Verify it's valid JSON
        let _parsed: serde_json::Value = serde_json::from_str(&prompt.content).unwrap();
    }

    #[test]
    fn test_prompt_builder_agents_md() {
        let config = PromptConfig {
            format: PromptFormat::AgentsMd,
            ..Default::default()
        };
        let builder = PromptBuilder::new(config);
        let prompt = builder.build();

        assert!(prompt.content.contains("## Using Lash"));
        assert!(prompt.content.contains("### Quick Start"));
        assert!(prompt.content.contains("### Key Rules"));
    }

    #[test]
    fn test_prompt_builder_no_examples() {
        let config = PromptConfig {
            include_examples: false,
            ..Default::default()
        };
        let builder = PromptBuilder::new(config);
        let prompt = builder.build();

        // Should not contain example code blocks
        assert!(!prompt.content.contains("### Minimal Valid File"));
    }

    #[test]
    fn test_prompt_builder_no_tasks() {
        let config = PromptConfig {
            include_tasks: false,
            ..Default::default()
        };
        let mut builder = PromptBuilder::new(config);
        builder.add_task_summary("test.md: 5 tasks".to_string());

        let prompt = builder.build();
        // Task summaries should not be included
        assert!(!prompt.content.contains("test.md"));
    }

    #[test]
    fn test_prompt_builder_with_budget() {
        let config = PromptConfig {
            token_budget: Some(500), // Small budget to force truncation
            ..Default::default()
        };
        let builder = PromptBuilder::new(config);
        let prompt = builder.build();

        assert!(prompt.token_count <= 500 + 50); // Allow small margin
                                                 // With such a small budget, content should be truncated
        assert!(prompt.truncated);
    }

    #[test]
    fn test_prompt_builder_with_filters() {
        let config = PromptConfig {
            label_filter: vec!["backend".to_string(), "api".to_string()],
            path_filter: Some("features/".to_string()),
            include_tasks: true,
            ..Default::default()
        };

        let mut builder = PromptBuilder::new(config);
        // Add a task summary so the filter messages are displayed
        builder.add_task_summary("test.md: 5 tasks".to_string());

        let prompt = builder.build();

        assert!(prompt.content.contains("Filtered by labels: backend, api"));
        assert!(prompt.content.contains("Filtered by path: features/"));
    }

    #[test]
    fn test_add_task_summary() {
        let config = PromptConfig::default();
        let mut builder = PromptBuilder::new(config);

        assert_eq!(builder.task_summaries.len(), 0);

        builder.add_task_summary("test1.md: 5 tasks".to_string());
        assert_eq!(builder.task_summaries.len(), 1);

        builder.add_task_summary("test2.md: 3 tasks".to_string());
        assert_eq!(builder.task_summaries.len(), 2);
    }

    #[test]
    fn test_doc_ref_info_display() {
        let doc = DocRefInfo::new("../docs/design.md", None);
        assert_eq!(doc.display(), "../docs/design.md");

        let doc_with_frag = DocRefInfo::new("../docs/design.md", Some("section-7".to_string()));
        assert_eq!(doc_with_frag.display(), "../docs/design.md#section-7");

        let invalid_doc = DocRefInfo::new("../docs/missing.md", None).with_validity(false);
        assert_eq!(invalid_doc.display(), "../docs/missing.md [missing]");

        let valid_doc = DocRefInfo::new("../docs/exists.md", None).with_validity(true);
        assert_eq!(valid_doc.display(), "../docs/exists.md");
    }

    #[test]
    fn test_task_file_summary_format() {
        let summary = TaskFileSummary::new("features/auth.md").with_counts(10, 7, 2, 1);

        assert_eq!(
            summary.to_summary_string(),
            "features/auth.md: 10 tasks, 70% complete, 1 blocked"
        );

        let summary_no_blocked = TaskFileSummary::new("features/api.md").with_counts(5, 5, 0, 0);

        assert_eq!(
            summary_no_blocked.to_summary_string(),
            "features/api.md: 5 tasks, 100% complete"
        );
    }

    #[test]
    fn test_task_file_summary_with_docs() {
        let summary = TaskFileSummary::new("features/auth.md")
            .with_counts(10, 7, 2, 1)
            .with_doc_refs(vec![
                DocRefInfo::new("../docs/auth-spec.md", Some("oauth".to_string()))
                    .with_validity(true),
                DocRefInfo::new("../docs/missing.md", None).with_validity(false),
            ]);

        let formatted = summary.format_with_docs();

        assert!(formatted.contains("features/auth.md: 10 tasks"));
        assert!(formatted.contains("../docs/auth-spec.md#oauth"));
        assert!(formatted.contains("../docs/missing.md [missing]"));
    }

    #[test]
    fn test_prompt_builder_with_task_file_summaries() {
        let config = PromptConfig::default();
        let mut builder = PromptBuilder::new(config);

        let summary = TaskFileSummary::new("features/auth.md")
            .with_counts(10, 7, 2, 1)
            .with_doc_refs(vec![DocRefInfo::new(
                "../docs/auth-spec.md",
                Some("oauth".to_string()),
            )]);

        builder.add_task_file_summary(summary);

        let prompt = builder.build();

        assert!(prompt.content.contains("features/auth.md"));
        assert!(prompt.content.contains("../docs/auth-spec.md#oauth"));
    }

    #[test]
    fn test_prompt_builder_json_with_doc_refs() {
        let config = PromptConfig {
            format: PromptFormat::Json,
            ..Default::default()
        };
        let mut builder = PromptBuilder::new(config);

        let summary = TaskFileSummary::new("features/auth.md")
            .with_counts(10, 7, 2, 1)
            .with_doc_refs(vec![DocRefInfo::new(
                "../docs/auth-spec.md",
                Some("oauth".to_string()),
            )
            .with_validity(true)]);

        builder.add_task_file_summary(summary);

        let prompt = builder.build();

        // Verify it's valid JSON and contains doc refs
        let parsed: serde_json::Value = serde_json::from_str(&prompt.content).unwrap();
        assert!(parsed["task_files"].is_array());
        let task_files = parsed["task_files"].as_array().unwrap();
        assert_eq!(task_files.len(), 1);
        assert_eq!(task_files[0]["path"], "features/auth.md");
        assert!(task_files[0]["doc_refs"].is_array());
    }

    #[test]
    fn test_plain_prompt_includes_doc_example() {
        let config = PromptConfig::default();
        let builder = PromptBuilder::new(config);
        let prompt = builder.build();

        // Should include the doc reference example
        assert!(prompt
            .content
            .contains("### File with Documentation References"));
        assert!(prompt.content.contains("@doc:"));
    }

    #[test]
    fn test_safety_guidelines_include_doc_refs() {
        let config = PromptConfig::default();
        let builder = PromptBuilder::new(config);
        let prompt = builder.build();

        assert!(prompt.content.contains("Keep `@doc` references valid"));
    }
}
