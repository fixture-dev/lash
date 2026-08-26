//! Sparse context generation for AI agents
//!
//! This module provides functionality for generating minimal yet complete context
//! for AI agents by selecting only relevant tasks and dependencies while respecting
//! token budgets.
//!
//! The sparse context approach includes:
//! - Target task/file (full detail)
//! - Direct dependencies (summaries)
//! - Blockers (full detail)
//! - Excludes completed dependencies
//! - Excludes unrelated files
//!
//! # Example
//!
//! ```
//! use lash_agent::context::{ContextBuilder, InclusionRules, ContextFormat};
//! use lash_core::dependency::{DependencyGraph, NodeData};
//! use lash_types::TaskStatus;
//! use std::collections::HashMap;
//!
//! // Create a simple graph
//! let mut graph = DependencyGraph::new();
//! graph.add_node(
//!     "core.api#setup".to_string(),
//!     NodeData::new("Setup API".to_string(), TaskStatus::Open, "core.api".to_string(), 0)
//! );
//!
//! // Build sparse context
//! let mut builder = ContextBuilder::new("core.api#setup");
//! builder.with_graph(&graph);
//! builder.with_token_budget(1000);
//!
//! let context = builder.build();
//! assert!(context.token_count <= 1000);
//! ```

use crate::tokens::estimate_tokens;
use lash_core::dependency::DependencyGraph;
use lash_types::TaskStatus;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Output format for sparse context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextFormat {
    /// Plain text Markdown
    Markdown,
    /// Structured JSON
    Json,
}

/// Rules for what to include in context
#[derive(Debug, Clone)]
pub struct InclusionRules {
    /// Include direct dependencies
    pub include_dependencies: bool,
    /// Include blockers (always recommended)
    pub include_blockers: bool,
    /// Include completed tasks (usually false)
    pub include_completed: bool,
    /// Maximum depth to traverse for dependencies
    pub max_dependency_depth: u8,
}

impl Default for InclusionRules {
    fn default() -> Self {
        Self {
            include_dependencies: false,
            include_blockers: true,
            include_completed: false,
            max_dependency_depth: 2,
        }
    }
}

/// A task node in the sparse context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTask {
    /// Full task ID (`file_id#task_id`)
    pub id: String,
    /// Task title
    pub title: String,
    /// Task status
    pub status: TaskStatus,
    /// File containing this task
    pub file_id: String,
    /// Task depth in hierarchy
    pub depth: u8,
    /// Detail level (full or summary)
    pub detail_level: DetailLevel,
    /// Optional body text (only for full detail)
    pub body: Option<String>,
}

/// Level of detail for a task in context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    /// Full task details including body and subtasks
    Full,
    /// Summary only (ID, title, status, file)
    Summary,
}

/// Generated sparse context
#[derive(Debug, Clone)]
pub struct SparseContext {
    /// The context content
    pub content: String,
    /// Estimated token count
    pub token_count: usize,
    /// Whether content was truncated to fit budget
    pub truncated: bool,
    /// Tasks included in context
    pub included_tasks: Vec<String>,
    /// Tasks excluded from context
    pub excluded_tasks: Vec<String>,
}

/// Builder for constructing sparse contexts
///
/// The builder allows configuring what information to include, token budgets,
/// and detail levels for different types of tasks.
///
/// # Example
///
/// ```
/// use lash_agent::context::{ContextBuilder, InclusionRules};
/// use lash_core::dependency::{DependencyGraph, NodeData};
/// use lash_types::TaskStatus;
/// use std::collections::HashMap;
///
/// let mut graph = DependencyGraph::new();
/// graph.add_node(
///     "test#task1".to_string(),
///     NodeData::new("Task 1".to_string(), TaskStatus::Open, "test".to_string(), 0)
/// );
///
/// let mut builder = ContextBuilder::new("test#task1");
/// builder.with_graph(&graph);
/// let context = builder.build();
///
/// assert!(!context.content.is_empty());
/// assert!(context.included_tasks.contains(&"test#task1".to_string()));
/// ```
pub struct ContextBuilder<'a> {
    /// Target task or file ID
    target_id: String,
    /// Dependency graph
    graph: Option<&'a DependencyGraph>,
    /// Task file contents (`file_id` -> (title, content))
    file_contents: HashMap<String, (String, String)>,
    /// Token budget (None = unlimited)
    token_budget: Option<usize>,
    /// Inclusion rules
    rules: InclusionRules,
    /// Output format
    format: ContextFormat,
}

impl<'a> ContextBuilder<'a> {
    /// Create a new context builder for the given target
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::context::ContextBuilder;
    ///
    /// let builder = ContextBuilder::new("core.api#setup");
    /// ```
    #[must_use]
    pub fn new(target_id: impl Into<String>) -> Self {
        Self {
            target_id: target_id.into(),
            graph: None,
            file_contents: HashMap::new(),
            token_budget: None,
            rules: InclusionRules::default(),
            format: ContextFormat::Markdown,
        }
    }

    /// Set the dependency graph
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::context::ContextBuilder;
    /// use lash_core::dependency::DependencyGraph;
    ///
    /// let graph = DependencyGraph::new();
    /// let mut builder = ContextBuilder::new("test#task1");
    /// builder.with_graph(&graph);
    /// ```
    pub fn with_graph(&mut self, graph: &'a DependencyGraph) -> &mut Self {
        self.graph = Some(graph);
        self
    }

    /// Add file content for a specific file
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::context::ContextBuilder;
    ///
    /// let mut builder = ContextBuilder::new("test#task1");
    /// builder.add_file_content("test", "Test File", "# Test\n\n- [ ] Task 1");
    /// ```
    pub fn add_file_content(
        &mut self,
        file_id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> &mut Self {
        self.file_contents
            .insert(file_id.into(), (title.into(), content.into()));
        self
    }

    /// Set the token budget
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::context::ContextBuilder;
    ///
    /// let mut builder = ContextBuilder::new("test#task1");
    /// builder.with_token_budget(1000);
    /// ```
    pub fn with_token_budget(&mut self, budget: usize) -> &mut Self {
        self.token_budget = Some(budget);
        self
    }

    /// Set the inclusion rules
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::context::{ContextBuilder, InclusionRules};
    ///
    /// let mut builder = ContextBuilder::new("test#task1");
    /// let mut rules = InclusionRules::default();
    /// rules.include_completed = true;
    /// builder.with_rules(rules);
    /// ```
    pub fn with_rules(&mut self, rules: InclusionRules) -> &mut Self {
        self.rules = rules;
        self
    }

    /// Set the output format
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_agent::context::{ContextBuilder, ContextFormat};
    ///
    /// let mut builder = ContextBuilder::new("test#task1");
    /// builder.with_format(ContextFormat::Json);
    /// ```
    pub fn with_format(&mut self, format: ContextFormat) -> &mut Self {
        self.format = format;
        self
    }

    /// Build the sparse context
    ///
    /// Generates the context according to the configuration, applying selection
    /// and budget constraints.
    ///
    /// # Examples
    ///
    /// ```
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
    /// let mut builder = ContextBuilder::new("test#task1");
    /// builder.with_graph(&graph);
    /// let context = builder.build();
    ///
    /// assert!(context.token_count > 0);
    /// ```
    pub fn build(self) -> SparseContext {
        // Select tasks to include
        let selected_tasks = self.select_tasks();

        // Generate content based on format
        let content = match self.format {
            ContextFormat::Markdown => self.generate_markdown(&selected_tasks),
            ContextFormat::Json => self.generate_json(&selected_tasks),
        };

        let token_count = estimate_tokens(&content);
        let truncated = if let Some(budget) = self.token_budget {
            token_count > budget
        } else {
            false
        };

        // Determine which tasks were excluded
        let included_ids: HashSet<String> = selected_tasks.iter().map(|t| t.id.clone()).collect();
        let all_task_ids = if let Some(graph) = self.graph {
            graph.all_node_ids()
        } else {
            Vec::new()
        };
        let excluded_tasks: Vec<String> = all_task_ids
            .iter()
            .filter(|id| !included_ids.contains(*id))
            .cloned()
            .collect();

        SparseContext {
            content,
            token_count,
            truncated,
            included_tasks: selected_tasks.iter().map(|t| t.id.clone()).collect(),
            excluded_tasks,
        }
    }

    /// Select which tasks to include in the context
    fn select_tasks(&self) -> Vec<ContextTask> {
        let mut tasks = Vec::new();

        let Some(graph) = self.graph else {
            return tasks;
        };

        // Always include the target task with full detail
        if let Some(node) = graph.get_node(&self.target_id) {
            tasks.push(ContextTask {
                id: self.target_id.clone(),
                title: node.title.clone(),
                status: node.status,
                file_id: node.file_id.clone(),
                depth: node.depth,
                detail_level: DetailLevel::Full,
                body: self.get_task_body(&self.target_id),
            });
        }

        // Include dependencies if requested
        if self.rules.include_dependencies {
            self.add_dependencies(&mut tasks, &self.target_id, 0);
        }

        // Include blockers if requested (with full detail)
        if self.rules.include_blockers {
            self.add_blockers(&mut tasks);
        }

        tasks
    }

    /// Add dependencies of a task to the context
    fn add_dependencies(&self, tasks: &mut Vec<ContextTask>, task_id: &str, current_depth: u8) {
        if current_depth >= self.rules.max_dependency_depth {
            return;
        }

        let Some(graph) = self.graph else {
            return;
        };

        if let Some(deps) = graph.get_dependencies(task_id) {
            for edge_ref in deps {
                let dep_id = &edge_ref.target_id;

                // Skip if already included
                if tasks.iter().any(|t| &t.id == dep_id) {
                    continue;
                }

                if let Some(node) = graph.get_node(dep_id) {
                    // Skip completed tasks unless explicitly requested
                    if !self.rules.include_completed && node.status.is_complete() {
                        continue;
                    }

                    // Add as summary (not full detail)
                    tasks.push(ContextTask {
                        id: dep_id.clone(),
                        title: node.title.clone(),
                        status: node.status,
                        file_id: node.file_id.clone(),
                        depth: node.depth,
                        detail_level: DetailLevel::Summary,
                        body: None,
                    });

                    // Recursively add dependencies
                    self.add_dependencies(tasks, dep_id, current_depth + 1);
                }
            }
        }
    }

    /// Add blocked tasks to the context with full detail
    fn add_blockers(&self, tasks: &mut Vec<ContextTask>) {
        let Some(graph) = self.graph else {
            return;
        };

        // Find all blocked tasks that are related to our target
        for task_id in graph.all_node_ids() {
            if let Some(node) = graph.get_node(&task_id) {
                if node.status == TaskStatus::Blocked {
                    // Check if this blocker is related to our target
                    if self.is_related_to_target(&task_id) {
                        // Skip if already included
                        if tasks.iter().any(|t| t.id == task_id) {
                            continue;
                        }

                        // Add with full detail
                        tasks.push(ContextTask {
                            id: task_id.clone(),
                            title: node.title.clone(),
                            status: node.status,
                            file_id: node.file_id.clone(),
                            depth: node.depth,
                            detail_level: DetailLevel::Full,
                            body: self.get_task_body(&task_id),
                        });
                    }
                }
            }
        }
    }

    /// Check if a task is related to the target (same file or dependent/dependency)
    fn is_related_to_target(&self, task_id: &str) -> bool {
        let Some(graph) = self.graph else {
            return false;
        };

        // Same file as target
        if let (Some(target_node), Some(task_node)) =
            (graph.get_node(&self.target_id), graph.get_node(task_id))
        {
            if target_node.file_id == task_node.file_id {
                return true;
            }
        }

        // Check if it's a dependency or dependent of target
        if let Ok(descendants) = graph.get_descendants(&self.target_id) {
            if descendants.contains(&task_id.to_string()) {
                return true;
            }
        }

        if let Ok(ancestors) = graph.get_ancestors(&self.target_id) {
            if ancestors.contains(&task_id.to_string()) {
                return true;
            }
        }

        false
    }

    /// Get the body content for a task from file contents
    #[allow(clippy::unused_self)]
    fn get_task_body(&self, _task_id: &str) -> Option<String> {
        // For now, we don't have task body extraction implemented
        // This would require parsing the markdown content
        // TODO: Implement when markdown parser is available
        None
    }

    /// Generate markdown output
    fn generate_markdown(&self, tasks: &[ContextTask]) -> String {
        let mut output = String::new();

        output.push_str("# Sparse Context\n\n");

        // Add context note
        output.push_str("## Context Note\n\n");
        output.push_str(&format!("Target: `{}`\n\n", self.target_id));
        output.push_str("This context includes:\n");
        output.push_str("- Target task (full detail)\n");
        if self.rules.include_dependencies {
            output.push_str(&format!(
                "- Dependencies (up to {} levels, summaries)\n",
                self.rules.max_dependency_depth
            ));
        }
        if self.rules.include_blockers {
            output.push_str("- Blocked tasks (full detail)\n");
        }
        if !self.rules.include_completed {
            output.push_str("\nCompleted dependencies are excluded.\n");
        }
        output.push('\n');

        // Group tasks by file
        let mut by_file: HashMap<String, Vec<&ContextTask>> = HashMap::new();
        for task in tasks {
            by_file.entry(task.file_id.clone()).or_default().push(task);
        }

        // Output tasks grouped by file
        for (file_id, file_tasks) in by_file {
            output.push_str(&format!("## File: {file_id}\n\n"));

            for task in file_tasks {
                let status_symbol = match task.status {
                    TaskStatus::Open => "[ ]",
                    TaskStatus::InProgress => "[>]",
                    TaskStatus::Done => "[x]",
                    TaskStatus::Waived => "[-]",
                    TaskStatus::Blocked => "[!]",
                };

                let detail_marker = match task.detail_level {
                    DetailLevel::Full => " (full detail)",
                    DetailLevel::Summary => " (summary)",
                };

                output.push_str(&format!(
                    "- {} **{}**{}\n",
                    status_symbol, task.title, detail_marker
                ));
                output.push_str(&format!("  - ID: `{}`\n", task.id));
                output.push_str(&format!("  - Status: {:?}\n", task.status));

                if let Some(ref body) = task.body {
                    output.push_str(&format!("  - Details: {body}\n"));
                }
                output.push('\n');
            }
        }

        output
    }

    /// Generate JSON output
    fn generate_json(&self, tasks: &[ContextTask]) -> String {
        let output = serde_json::json!({
            "format": "lash-sparse-context",
            "version": "1.0",
            "target": self.target_id,
            "rules": {
                "include_dependencies": self.rules.include_dependencies,
                "include_blockers": self.rules.include_blockers,
                "include_completed": self.rules.include_completed,
                "max_dependency_depth": self.rules.max_dependency_depth,
            },
            "tasks": tasks,
            "stats": {
                "included_count": tasks.len(),
                "full_detail_count": tasks.iter().filter(|t| t.detail_level == DetailLevel::Full).count(),
                "summary_count": tasks.iter().filter(|t| t.detail_level == DetailLevel::Summary).count(),
            }
        });

        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    }
}

impl Default for ContextBuilder<'_> {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_core::dependency::{EdgeData, NodeData};
    use lash_types::DependencyKind;

    fn create_test_graph() -> DependencyGraph {
        let mut graph = DependencyGraph::new();

        // Add nodes
        graph.add_node(
            "test#task1".to_string(),
            NodeData::new(
                "Task 1".to_string(),
                TaskStatus::Open,
                "test".to_string(),
                0,
            ),
        );
        graph.add_node(
            "test#task2".to_string(),
            NodeData::new(
                "Task 2".to_string(),
                TaskStatus::Done,
                "test".to_string(),
                0,
            ),
        );
        graph.add_node(
            "test#task3".to_string(),
            NodeData::new(
                "Task 3".to_string(),
                TaskStatus::Blocked,
                "test".to_string(),
                0,
            ),
        );

        // Add edges (task1 depends on task2)
        graph.add_edge(
            "test#task1".to_string(),
            "test#task2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        graph
    }

    #[test]
    fn test_context_builder_basic() {
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);

        let context = builder.build();

        assert!(!context.content.is_empty());
        assert!(context.token_count > 0);
        assert!(context.included_tasks.contains(&"test#task1".to_string()));
    }

    #[test]
    fn test_context_includes_target() {
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);

        let context = builder.build();

        assert!(context.included_tasks.contains(&"test#task1".to_string()));
        assert!(context.content.contains("Task 1"));
    }

    #[test]
    fn test_context_excludes_completed_by_default() {
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);

        let context = builder.build();

        // task2 is completed and should be excluded by default
        assert!(!context.included_tasks.contains(&"test#task2".to_string()));
    }

    #[test]
    fn test_context_includes_blockers() {
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);

        let context = builder.build();

        // task3 is blocked and in same file, should be included
        assert!(context.included_tasks.contains(&"test#task3".to_string()));
    }

    #[test]
    fn test_context_with_token_budget() {
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);
        builder.with_token_budget(100);

        let context = builder.build();

        // Content should exist but may be truncated
        assert!(!context.content.is_empty());
    }

    #[test]
    fn test_context_json_format() {
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);
        builder.with_format(ContextFormat::Json);

        let context = builder.build();

        assert!(context
            .content
            .contains("\"format\": \"lash-sparse-context\""));
        assert!(context.content.contains("\"target\": \"test#task1\""));

        // Verify it's valid JSON
        let _parsed: serde_json::Value = serde_json::from_str(&context.content).unwrap();
    }

    #[test]
    fn test_context_markdown_format() {
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);
        builder.with_format(ContextFormat::Markdown);

        let context = builder.build();

        assert!(context.content.contains("# Sparse Context"));
        assert!(context.content.contains("## Context Note"));
        assert!(context.content.contains("Target: `test#task1`"));
    }

    #[test]
    fn test_inclusion_rules() {
        let rules = InclusionRules {
            include_completed: true,
            include_dependencies: false,
            ..Default::default()
        };

        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);
        builder.with_rules(rules);

        let context = builder.build();

        // With include_dependencies = false, should only have target
        assert_eq!(context.included_tasks.len(), 2); // target + blocker in same file
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);

        let context = builder.build();

        // Should handle empty graph gracefully
        assert!(context.included_tasks.is_empty());
    }

    // --- Mutant-killing tests ---

    #[test]
    fn test_inclusion_rules_default_values() {
        // Kills mut-000000 (true→false for include_dependencies in default()):
        // Verify every default field has the exact expected value.
        let rules = InclusionRules::default();
        assert!(
            rules.include_dependencies,
            "include_dependencies default must be true"
        );
        assert!(
            rules.include_blockers,
            "include_blockers default must be true"
        );
        assert!(
            !rules.include_completed,
            "include_completed default must be false"
        );
        assert_eq!(rules.max_dependency_depth, 2);
    }

    #[test]
    fn test_build_truncated_false_when_no_budget() {
        // Kills mut-000006 (false→true in the else branch when token_budget is None):
        // With no token budget, truncated must be exactly false.
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);
        // Do NOT call with_token_budget

        let context = builder.build();
        assert!(
            !context.truncated,
            "truncated must be false when no budget is set"
        );
    }

    #[test]
    fn test_build_truncated_true_when_over_budget() {
        // Kills mut-000004 (> vs >=) and mut-000005 (> vs <=):
        // Set a budget of 1 token so token_count > budget is guaranteed.
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);
        builder.with_token_budget(1);

        let context = builder.build();
        assert!(
            context.truncated,
            "truncated must be true when token_count > budget"
        );
    }

    #[test]
    fn test_build_truncated_false_when_exactly_at_budget() {
        // Kills mut-000004 (> vs >=): when token_count == budget, truncated must be false.
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");
        builder.with_graph(&graph);
        let ctx_no_budget = {
            let mut b = ContextBuilder::new("test#task1");
            b.with_graph(&graph);
            b.build()
        };
        // Use exact token count as budget — not over budget, so not truncated.
        builder.with_token_budget(ctx_no_budget.token_count);

        let context = builder.build();
        assert!(
            !context.truncated,
            "truncated must be false when token_count == budget"
        );
    }

    #[test]
    fn test_excluded_tasks_are_not_included_tasks() {
        // Kills mut-000008 (negation of !included_ids.contains(*id)):
        // Excluded tasks should be those NOT in included_tasks.
        let graph = create_test_graph();
        let mut builder = ContextBuilder::new("test#task1");

        // Use rules that only include the target (no deps, no blockers)
        let rules = InclusionRules {
            include_dependencies: false,
            include_blockers: false,
            ..Default::default()
        };
        builder.with_graph(&graph);
        builder.with_rules(rules);

        let context = builder.build();

        // task1 is included, task2 and task3 must be excluded
        assert!(context.included_tasks.contains(&"test#task1".to_string()));
        assert!(context.excluded_tasks.contains(&"test#task2".to_string()));
        assert!(context.excluded_tasks.contains(&"test#task3".to_string()));

        // No overlap between included and excluded
        for included in &context.included_tasks {
            assert!(
                !context.excluded_tasks.contains(included),
                "task {included} should not be in both included and excluded"
            );
        }
    }

    #[test]
    fn test_add_dependencies_starts_at_depth_zero() {
        // Kills mut-000011 (0→1 in the initial add_dependencies call):
        // With max_dependency_depth=1, starting at depth=0 should traverse one level.
        // Starting at depth=1 would immediately return without adding any deps.
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "root#main".to_string(),
            NodeData::new("Main".to_string(), TaskStatus::Open, "root".to_string(), 0),
        );
        graph.add_node(
            "root#dep1".to_string(),
            NodeData::new("Dep1".to_string(), TaskStatus::Open, "root".to_string(), 0),
        );
        graph.add_edge(
            "root#main".to_string(),
            "root#dep1".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let mut builder = ContextBuilder::new("root#main");
        builder.with_graph(&graph);
        let rules = InclusionRules {
            max_dependency_depth: 1,
            include_blockers: false,
            ..Default::default()
        };
        builder.with_rules(rules);

        let context = builder.build();
        // dep1 should be included because we start at depth 0, which is < max (1)
        assert!(
            context.included_tasks.contains(&"root#dep1".to_string()),
            "dep1 should be included when starting depth is 0 and max_depth is 1"
        );
    }

    #[test]
    fn test_add_dependencies_depth_limit_exact_boundary() {
        // Kills mut-000013 (negation), mut-000014 (>= vs >), mut-000015 (>= vs <):
        // At max_dependency_depth, recursion stops — so a dep-of-dep is NOT included
        // when max_depth=1 but the chain is 2 levels deep.
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "a#root".to_string(),
            NodeData::new("Root".to_string(), TaskStatus::Open, "a".to_string(), 0),
        );
        graph.add_node(
            "a#level1".to_string(),
            NodeData::new("Level1".to_string(), TaskStatus::Open, "a".to_string(), 0),
        );
        graph.add_node(
            "a#level2".to_string(),
            NodeData::new("Level2".to_string(), TaskStatus::Open, "a".to_string(), 0),
        );
        graph.add_edge(
            "a#root".to_string(),
            "a#level1".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "a#level1".to_string(),
            "a#level2".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let mut builder = ContextBuilder::new("a#root");
        builder.with_graph(&graph);
        let rules = InclusionRules {
            include_dependencies: true,
            include_blockers: false,
            include_completed: true,
            max_dependency_depth: 1,
        };
        builder.with_rules(rules);

        let context = builder.build();
        // level1 is at depth 0 < 1, so it should be included
        assert!(
            context.included_tasks.contains(&"a#level1".to_string()),
            "level1 (depth 0) should be included with max_depth=1"
        );
        // level2 would be added at depth 1, which >= max_depth (1), so it should NOT be included
        assert!(
            !context.included_tasks.contains(&"a#level2".to_string()),
            "level2 (depth 1) should be excluded when max_depth=1"
        );
    }

    #[test]
    fn test_add_dependencies_deduplication() {
        // Kills mut-000017 (negation of tasks.iter().any) and mut-000018 (== vs !=):
        // A task that is already in the list must not be added again.
        // Create a diamond dependency: root->a, root->b, a->c, b->c
        // c should appear only once.
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "d#root".to_string(),
            NodeData::new("Root".to_string(), TaskStatus::Open, "d".to_string(), 0),
        );
        graph.add_node(
            "d#a".to_string(),
            NodeData::new("A".to_string(), TaskStatus::Open, "d".to_string(), 0),
        );
        graph.add_node(
            "d#b".to_string(),
            NodeData::new("B".to_string(), TaskStatus::Open, "d".to_string(), 0),
        );
        graph.add_node(
            "d#c".to_string(),
            NodeData::new("C".to_string(), TaskStatus::Open, "d".to_string(), 0),
        );
        graph.add_edge(
            "d#root".to_string(),
            "d#a".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "d#root".to_string(),
            "d#b".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "d#a".to_string(),
            "d#c".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "d#b".to_string(),
            "d#c".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let mut builder = ContextBuilder::new("d#root");
        builder.with_graph(&graph);
        let rules = InclusionRules {
            include_dependencies: true,
            include_blockers: false,
            include_completed: true,
            max_dependency_depth: 3,
        };
        builder.with_rules(rules);

        let context = builder.build();
        let c_count = context
            .included_tasks
            .iter()
            .filter(|id| *id == "d#c")
            .count();
        assert_eq!(
            c_count, 1,
            "d#c should appear exactly once despite diamond dependency"
        );
    }

    #[test]
    fn test_add_dependencies_skips_completed_when_not_requested() {
        // Kills mut-000021 (&& vs || for !include_completed && is_complete()):
        // When include_completed=false (default), a completed dependency must be skipped.
        // The && means BOTH conditions must be true to skip: not-include AND is-complete.
        // With || it would skip if EITHER is true — so an open dep would also be skipped.
        let mut graph = DependencyGraph::new();
        graph.add_node(
            "e#root".to_string(),
            NodeData::new("Root".to_string(), TaskStatus::Open, "e".to_string(), 0),
        );
        graph.add_node(
            "e#done_dep".to_string(),
            NodeData::new("DoneDep".to_string(), TaskStatus::Done, "e".to_string(), 0),
        );
        graph.add_node(
            "e#open_dep".to_string(),
            NodeData::new("OpenDep".to_string(), TaskStatus::Open, "e".to_string(), 0),
        );
        graph.add_edge(
            "e#root".to_string(),
            "e#done_dep".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );
        graph.add_edge(
            "e#root".to_string(),
            "e#open_dep".to_string(),
            EdgeData::new(DependencyKind::ExplicitId, None),
        );

        let mut builder = ContextBuilder::new("e#root");
        builder.with_graph(&graph);
        let rules = InclusionRules {
            include_dependencies: true,
            include_blockers: false,
            include_completed: false, // <-- do not include completed
            max_dependency_depth: 2,
        };
        builder.with_rules(rules);

        let context = builder.build();
        // done_dep is complete and include_completed=false → must be excluded
        assert!(
            !context.included_tasks.contains(&"e#done_dep".to_string()),
            "completed dep must be excluded when include_completed=false"
        );
        // open_dep is not complete → must be included
        assert!(
            context.included_tasks.contains(&"e#open_dep".to_string()),
            "open dep must be included when include_completed=false"
        );
    }

    #[test]
    fn test_generate_markdown_include_dependencies_flag() {
        // Kills mut-000033 (negation of self.rules.include_dependencies in generate_markdown):
        // When include_dependencies=true, the "Dependencies" line appears.
        // When false, it does not.
        let graph = create_test_graph();

        let mut builder_with_deps = ContextBuilder::new("test#task1");
        builder_with_deps.with_graph(&graph);
        let rules_with = InclusionRules {
            include_dependencies: true,
            ..Default::default()
        };
        builder_with_deps.with_rules(rules_with);
        let ctx_with = builder_with_deps.build();
        assert!(
            ctx_with.content.contains("Dependencies"),
            "markdown must mention Dependencies when include_dependencies=true"
        );

        let mut builder_without = ContextBuilder::new("test#task1");
        builder_without.with_graph(&graph);
        let rules_without = InclusionRules {
            include_dependencies: false,
            ..Default::default()
        };
        builder_without.with_rules(rules_without);
        let ctx_without = builder_without.build();
        assert!(
            !ctx_without.content.contains("Dependencies (up to"),
            "markdown must not mention Dependencies when include_dependencies=false"
        );
    }

    #[test]
    fn test_generate_markdown_include_blockers_flag() {
        // Kills mut-000034 (negation of self.rules.include_blockers):
        // When include_blockers=true, the "Blocked tasks" line appears.
        // When false, it does not.
        let graph = create_test_graph();

        let mut builder_with = ContextBuilder::new("test#task1");
        builder_with.with_graph(&graph);
        let rules_with = InclusionRules {
            include_blockers: true,
            ..Default::default()
        };
        builder_with.with_rules(rules_with);
        let ctx_with = builder_with.build();
        assert!(
            ctx_with.content.contains("Blocked tasks"),
            "markdown must mention Blocked tasks when include_blockers=true"
        );

        let mut builder_without = ContextBuilder::new("test#task1");
        builder_without.with_graph(&graph);
        let rules_without = InclusionRules {
            include_blockers: false,
            ..Default::default()
        };
        builder_without.with_rules(rules_without);
        let ctx_without = builder_without.build();
        assert!(
            !ctx_without.content.contains("Blocked tasks"),
            "markdown must not mention Blocked tasks when include_blockers=false"
        );
    }

    #[test]
    fn test_generate_markdown_include_completed_flag() {
        // Kills mut-000035 (negation of !self.rules.include_completed):
        // When include_completed=false (default), the "Completed dependencies are excluded" note appears.
        // When include_completed=true, it does not.
        let graph = create_test_graph();

        let mut builder_excl = ContextBuilder::new("test#task1");
        builder_excl.with_graph(&graph);
        let rules_excl = InclusionRules {
            include_completed: false,
            ..Default::default()
        };
        builder_excl.with_rules(rules_excl);
        let ctx_excl = builder_excl.build();
        assert!(
            ctx_excl
                .content
                .contains("Completed dependencies are excluded"),
            "must show exclusion note when include_completed=false"
        );

        let mut builder_incl = ContextBuilder::new("test#task1");
        builder_incl.with_graph(&graph);
        let rules_incl = InclusionRules {
            include_completed: true,
            ..Default::default()
        };
        builder_incl.with_rules(rules_incl);
        let ctx_incl = builder_incl.build();
        assert!(
            !ctx_incl
                .content
                .contains("Completed dependencies are excluded"),
            "must not show exclusion note when include_completed=true"
        );
    }
}
