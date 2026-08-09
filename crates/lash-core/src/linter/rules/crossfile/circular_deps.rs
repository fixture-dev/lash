//! Rule: No circular dependencies
//!
//! Detects cycles in the dependency graph across all files.
//! Uses DFS with visited set to detect cycles.
//!
//! Error code: `E_LINK_CYCLE`

use lash_types::{dependency::DependencyKind, Severity, TaskFile};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that detects circular dependencies
///
/// This rule builds a dependency graph from all `@depends-on` annotations
/// across all files in the project and detects cycles using depth-first search.
///
/// # Examples
///
/// Valid (no cycles):
/// ```markdown
/// // File A
/// @depends-on: b.md#task1
///
/// // File B
/// @depends-on: c.md#task1
/// ```
///
/// Invalid (`E_LINK_CYCLE`):
/// ```markdown
/// // File A
/// @depends-on: b.md#task1
///
/// // File B
/// @depends-on: a.md#task1  // Cycle: A → B → A
/// ```
pub struct CircularDepsRule;

impl CircularDepsRule {
    /// Create a new circular dependencies rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Build a full task ID from file path and task ID
    fn make_full_id(file_path: &Path, task_id: &str) -> String {
        format!("{}#{}", file_path.display(), task_id)
    }

    /// Resolve a dependency reference to a full task ID
    fn resolve_dep_to_full_id(
        ctx: &LintContext,
        _from_file: &Path,
        dep_ref: &lash_types::dependency::DependencyRef,
    ) -> Option<String> {
        match dep_ref.kind {
            DependencyKind::ExplicitPath => {
                if let Some((path_part, task_part)) = dep_ref.target.split_once("#task:") {
                    let target_path = ctx.resolve_path(Path::new(path_part));
                    Some(format!("{}#{task_part}", target_path.display()))
                } else {
                    // Just a file reference, no specific task
                    None
                }
            }
            DependencyKind::ExplicitId => {
                if let Some((file_id, task_id)) = dep_ref.target.split_once('#') {
                    // Strip "task:" prefix if present
                    let task_id = task_id.strip_prefix("task:").unwrap_or(task_id);

                    // Try to find file by ID
                    if let Some(target_file) = ctx.all_files.values().find(|f| f.id == file_id) {
                        Some(format!("{}#{task_id}", target_file.path.display()))
                    } else {
                        // Try as path
                        Some(format!("{file_id}#{task_id}"))
                    }
                } else {
                    // Bare file ID, no task
                    None
                }
            }
            DependencyKind::Hierarchy => {
                // Hierarchy dependencies are within-file only
                None
            }
            DependencyKind::Directory => {
                // Directory dependencies don't create task-level edges
                None
            }
        }
    }

    /// Build dependency graph from all files
    fn build_dependency_graph(ctx: &LintContext) -> HashMap<String, Vec<String>> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Build graph from all tasks in all files
        for (file_path, file) in ctx.all_files {
            for task in file.tasks.tasks() {
                let from_id = Self::make_full_id(file_path, &task.id);
                let mut edges = Vec::new();

                // Add edges for all dependencies
                for dep_ref in &task.metadata.depends_on {
                    if let Some(to_id) = Self::resolve_dep_to_full_id(ctx, file_path, dep_ref) {
                        edges.push(to_id);
                    }
                }

                // Add parent-child hierarchy edges (child depends on parent)
                if let Some(parent_id) = &task.parent_id {
                    let parent_full_id = Self::make_full_id(file_path, parent_id);
                    edges.push(parent_full_id);
                }

                if !edges.is_empty() {
                    graph.insert(from_id, edges);
                }
            }
        }

        graph
    }

    /// Detect cycles using DFS
    fn detect_cycle(
        graph: &HashMap<String, Vec<String>>,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) =
                        Self::detect_cycle(graph, neighbor, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap();
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(neighbor.clone());
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        None
    }

    /// Find all cycles in the graph
    fn find_cycles(graph: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
        let mut visited = HashSet::new();
        let mut cycles = Vec::new();

        for node in graph.keys() {
            if !visited.contains(node) {
                let mut rec_stack = HashSet::new();
                let mut path = Vec::new();
                if let Some(cycle) =
                    Self::detect_cycle(graph, node, &mut visited, &mut rec_stack, &mut path)
                {
                    cycles.push(cycle);
                }
            }
        }

        cycles
    }
}

impl Default for CircularDepsRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for CircularDepsRule {
    fn code(&self) -> &'static str {
        "E_LINK_CYCLE"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn description(&self) -> &'static str {
        "Detects circular dependencies in the task dependency graph"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Only check cycles if we have the full context (all files)
        if ctx.all_files.is_empty() {
            return diagnostics;
        }

        // Build dependency graph
        let graph = Self::build_dependency_graph(ctx);

        // Find cycles
        let cycles = Self::find_cycles(&graph);

        // Report cycles that involve tasks in this file
        for cycle in cycles {
            // Check if any task in the cycle is from this file
            let involves_this_file = cycle
                .iter()
                .any(|task_id| task_id.starts_with(&format!("{}#", file.path.display())));

            if involves_this_file {
                let cycle_path = cycle.join(" → ");
                diagnostics.push(
                    LintDiagnostic::error(
                        self.code(),
                        format!("Circular dependency detected: {cycle_path}"),
                        file.path.clone(),
                        0,
                        0,
                    )
                    .with_help("Remove one of the dependencies in the cycle to break it"),
                );
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{
        dependency::{parse_dependency_ref, DependencyKind, DependencyRef},
        task::{Task, TaskMetadata, TaskTree},
        FileMetadata, LashConfig, TaskStatus,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn make_task(id: &str, deps: &[&str]) -> Task {
        let metadata = TaskMetadata {
            depends_on: deps
                .iter()
                .map(|d| parse_dependency_ref(d).unwrap())
                .collect(),
            ..Default::default()
        };

        Task {
            id: id.to_string(),
            has_explicit_id: false,
            title: format!("Task {id}"),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            annotation_line_count: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
        }
    }

    fn make_file_with_tasks(path: &str, id: &str, tasks: Vec<Task>) -> TaskFile {
        let mut task_tree = TaskTree::new();
        for task in tasks {
            let _ = task_tree.add_task(task);
        }

        TaskFile {
            path: PathBuf::from(path),
            title: "Test File".to_string(),
            id: id.to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: task_tree,
            hash: "test-hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_no_cycle_valid() {
        let rule = CircularDepsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("a.md"),
            make_file_with_tasks("a.md", "a", vec![make_task("task1", &["b.md#task:task1"])]),
        );
        files.insert(
            PathBuf::from("b.md"),
            make_file_with_tasks("b.md", "b", vec![make_task("task1", &[])]),
        );

        let ctx = LintContext::new(&config, PathBuf::from("a.md"), &files);
        let file_a = files.get(&PathBuf::from("a.md")).unwrap();

        let diagnostics = rule.check_file(file_a, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_simple_cycle_two_files() {
        let rule = CircularDepsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("a.md"),
            make_file_with_tasks("a.md", "a", vec![make_task("task1", &["b.md#task:task1"])]),
        );
        files.insert(
            PathBuf::from("b.md"),
            make_file_with_tasks("b.md", "b", vec![make_task("task1", &["a.md#task:task1"])]),
        );

        let ctx = LintContext::new(&config, PathBuf::from("a.md"), &files);
        let file_a = files.get(&PathBuf::from("a.md")).unwrap();

        let diagnostics = rule.check_file(file_a, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_CYCLE");
        assert!(diagnostics[0].message.contains("Circular dependency"));
    }

    #[test]
    fn test_complex_cycle_three_files() {
        let rule = CircularDepsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("a.md"),
            make_file_with_tasks("a.md", "a", vec![make_task("task1", &["b.md#task:task1"])]),
        );
        files.insert(
            PathBuf::from("b.md"),
            make_file_with_tasks("b.md", "b", vec![make_task("task1", &["c.md#task:task1"])]),
        );
        files.insert(
            PathBuf::from("c.md"),
            make_file_with_tasks("c.md", "c", vec![make_task("task1", &["a.md#task:task1"])]),
        );

        let ctx = LintContext::new(&config, PathBuf::from("a.md"), &files);
        let file_a = files.get(&PathBuf::from("a.md")).unwrap();

        let diagnostics = rule.check_file(file_a, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_CYCLE");
    }

    #[test]
    fn test_self_reference_cycle() {
        let rule = CircularDepsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("a.md"),
            make_file_with_tasks("a.md", "a", vec![make_task("task1", &["a.md#task:task1"])]),
        );

        let ctx = LintContext::new(&config, PathBuf::from("a.md"), &files);
        let file_a = files.get(&PathBuf::from("a.md")).unwrap();

        let diagnostics = rule.check_file(file_a, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_CYCLE");
    }

    #[test]
    fn test_no_cycle_with_shared_dependency() {
        let rule = CircularDepsRule::new();
        let config = LashConfig::default();

        // Diamond pattern: A depends on B and C, both depend on D (no cycle)
        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("a.md"),
            make_file_with_tasks(
                "a.md",
                "a",
                vec![make_task("task1", &["b.md#task:task1", "c.md#task:task1"])],
            ),
        );
        files.insert(
            PathBuf::from("b.md"),
            make_file_with_tasks("b.md", "b", vec![make_task("task1", &["d.md#task:task1"])]),
        );
        files.insert(
            PathBuf::from("c.md"),
            make_file_with_tasks("c.md", "c", vec![make_task("task1", &["d.md#task:task1"])]),
        );
        files.insert(
            PathBuf::from("d.md"),
            make_file_with_tasks("d.md", "d", vec![make_task("task1", &[])]),
        );

        let ctx = LintContext::new(&config, PathBuf::from("a.md"), &files);
        let file_a = files.get(&PathBuf::from("a.md")).unwrap();

        let diagnostics = rule.check_file(file_a, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_cycle_with_explicit_id() {
        let rule = CircularDepsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        // Create tasks with explicit ID references
        let mut task_a = make_task("task1", &[]);
        task_a.metadata.depends_on = vec![DependencyRef::new(
            "b#task1".to_string(),
            DependencyKind::ExplicitId,
        )];

        let mut task_b = make_task("task1", &[]);
        task_b.metadata.depends_on = vec![DependencyRef::new(
            "a#task1".to_string(),
            DependencyKind::ExplicitId,
        )];

        files.insert(
            PathBuf::from("a.md"),
            make_file_with_tasks("a.md", "a", vec![task_a]),
        );
        files.insert(
            PathBuf::from("b.md"),
            make_file_with_tasks("b.md", "b", vec![task_b]),
        );

        let ctx = LintContext::new(&config, PathBuf::from("a.md"), &files);
        let file_a = files.get(&PathBuf::from("a.md")).unwrap();

        let diagnostics = rule.check_file(file_a, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_CYCLE");
    }

    #[test]
    fn test_parent_child_hierarchy_no_cycle() {
        let rule = CircularDepsRule::new();
        let config = LashConfig::default();

        let task_parent = make_task("parent", &[]);
        let mut task_child = make_task("child", &[]);
        task_child.parent_id = Some("parent".to_string());
        task_child.depth = 1;

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("a.md"),
            make_file_with_tasks("a.md", "a", vec![task_parent, task_child]),
        );

        let ctx = LintContext::new(&config, PathBuf::from("a.md"), &files);
        let file_a = files.get(&PathBuf::from("a.md")).unwrap();

        let diagnostics = rule.check_file(file_a, &ctx);
        // Parent-child is not a cycle (it's a valid hierarchy)
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_empty_context_returns_no_diagnostics() {
        let rule = CircularDepsRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();

        let file = make_file_with_tasks("a.md", "a", vec![make_task("task1", &[])]);
        let ctx = LintContext::new(&config, PathBuf::from("a.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_cycle_path_formatting() {
        let rule = CircularDepsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("a.md"),
            make_file_with_tasks("a.md", "a", vec![make_task("task1", &["b.md#task:task1"])]),
        );
        files.insert(
            PathBuf::from("b.md"),
            make_file_with_tasks("b.md", "b", vec![make_task("task1", &["a.md#task:task1"])]),
        );

        let ctx = LintContext::new(&config, PathBuf::from("a.md"), &files);
        let file_a = files.get(&PathBuf::from("a.md")).unwrap();

        let diagnostics = rule.check_file(file_a, &ctx);
        assert_eq!(diagnostics.len(), 1);
        // Check that the cycle path is formatted with arrows
        assert!(diagnostics[0].message.contains("→"));
    }
}
