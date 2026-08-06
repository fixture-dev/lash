//! Extended rendering for `lash show`'s full (non-`--short`) task output.
//!
//! Factored out of `show/mod.rs` (GitHub issue #26) to keep that file under
//! the project's ~500-line guideline. This module owns everything the old
//! terse view left out that agents actually need in order to act on a task
//! without re-reading the whole file:
//!
//! - `@agent-note` (rendered verbatim, multi-line)
//! - `@depends-on` references resolved to their current status
//! - direct children with checkbox state
//! - any custom metadata the parser captured (e.g. `@created`)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lash_core::dependency::reference::resolve_reference;
use lash_core::linter::LintContext;
use lash_db::repository::tasks::TaskRecord;
use lash_db::{DbResult, TaskRepository};
use lash_types::config::LashConfig;
use lash_types::dependency::DependencyKind;
use lash_types::{TaskFile, TaskStatus};
use serde::Serialize;

use lash_cli::theme::CliTheme;

use crate::utils::project_loader::find_task_by_full_id;

/// Current status of one resolved `@depends-on` reference.
#[derive(Debug, Clone, Serialize)]
pub struct DependencyStatus {
    /// The reference exactly as written in `@depends-on`.
    pub reference: String,
    /// Resolved `file-id#task-id`, when the reference resolved to a task.
    pub full_id: Option<String>,
    /// Target task title, when resolved.
    pub title: Option<String>,
    /// Target task status, when resolved.
    pub status: Option<TaskStatus>,
    /// Whether the target counts as met (done or waived). Always `false`
    /// for an unresolved reference.
    pub satisfied: bool,
}

/// One direct child task.
#[derive(Debug, Clone, Serialize)]
pub struct ChildSummary {
    /// Child's full id (`file-id#task-id`).
    pub full_id: String,
    /// Child's title.
    pub title: String,
    /// Child's current status.
    pub status: TaskStatus,
    /// Descendants of this child, collapsed into a count rather than shown
    /// as their own lines (0 if the child has no further nesting).
    pub nested_count: usize,
}

fn unresolved_dependency(reference: &str) -> DependencyStatus {
    DependencyStatus {
        reference: reference.to_string(),
        full_id: None,
        title: None,
        status: None,
        satisfied: false,
    }
}

/// Resolve every non-hierarchical, non-directory `@depends-on` reference on
/// `task` against the current on-disk project, returning each target's live
/// status.
///
/// Reparses the project (see [`crate::utils::project_loader::load_project`])
/// rather than trusting the `SQLite` index, matching `lash complete`'s
/// unmet-dependency gate: Markdown is the single source of truth, and `show`
/// exists precisely so agents don't have to re-read it themselves to find
/// out. A reference that fails to resolve (dangling target, typo, etc.) is
/// reported as unresolved rather than causing `show` to fail — that
/// diagnosis is `lash check-links`'s job.
///
/// Directory-level dependencies (`path/`) are skipped: they expand to many
/// targets and are better inspected with `lash graph`/`lash check-links`
/// than crammed into a single task's detail view.
#[must_use]
pub fn resolve_dependencies(
    config: &LashConfig,
    project: &HashMap<PathBuf, TaskFile>,
    source_path: &Path,
    source_file_id: &str,
    task: &TaskRecord,
) -> Vec<DependencyStatus> {
    let ctx = LintContext::new(config, source_path.to_path_buf(), project);
    let resolve_path = |rel: &str| ctx.resolve_path(Path::new(rel));

    let mut statuses = Vec::new();
    for dep in &task.metadata.depends_on {
        if matches!(
            dep.kind,
            DependencyKind::Hierarchy | DependencyKind::Directory
        ) {
            continue;
        }

        let resolution = resolve_reference(
            &dep.target,
            source_path,
            source_file_id,
            project,
            resolve_path,
        );
        match resolution {
            Ok(resolution) => {
                for full_id in resolution.full_ids() {
                    if let Some((_, _, target)) = find_task_by_full_id(project, &full_id) {
                        statuses.push(DependencyStatus {
                            reference: dep.target.clone(),
                            full_id: Some(full_id),
                            title: Some(target.title.clone()),
                            status: Some(target.status),
                            satisfied: target.status.is_complete(),
                        });
                    } else {
                        // Resolved to an id that isn't actually in the
                        // project (shouldn't normally happen, but the
                        // resolver and the project map could disagree).
                        statuses.push(unresolved_dependency(&dep.target));
                    }
                }
            }
            Err(_) => statuses.push(unresolved_dependency(&dep.target)),
        }
    }
    statuses
}

/// `(satisfied, total)` counts for a dependency status list.
#[must_use]
pub fn dependency_counts(deps: &[DependencyStatus]) -> (usize, usize) {
    (deps.iter().filter(|d| d.satisfied).count(), deps.len())
}

/// Direct children of `task_db_id`, each annotated with how many further
/// descendants it has — so a deep hierarchy collapses to one line per direct
/// child instead of a full recursive tree (that's what `--deps`/`lash list
/// --tree` are for).
///
/// # Errors
///
/// Returns the underlying database error if the query fails.
pub fn children_summary(
    task_repo: &TaskRepository,
    task_db_id: i64,
) -> DbResult<Vec<ChildSummary>> {
    let children = task_repo.get_children(task_db_id)?;
    let mut summaries = Vec::with_capacity(children.len());
    for child in children {
        let nested_count = task_repo.get_descendants(child.id)?.len();
        summaries.push(ChildSummary {
            full_id: child.full_id,
            title: child.title,
            status: child.status,
            nested_count,
        });
    }
    Ok(summaries)
}

/// `(done, total)` counts for a children summary list. A child counts as
/// done if its status is done or waived, matching how dependency
/// satisfaction is computed elsewhere.
#[must_use]
pub fn children_counts(children: &[ChildSummary]) -> (usize, usize) {
    (
        children.iter().filter(|c| c.status.is_complete()).count(),
        children.len(),
    )
}

/// Render the `@agent-note` block, if present, preserving line breaks.
pub fn render_agent_note(note: Option<&str>, theme: Option<&CliTheme>) {
    let Some(note) = note.filter(|n| !n.is_empty()) else {
        return;
    };

    println!();
    if let Some(theme) = theme {
        println!("{}", theme.style_info("Agent note:"));
    } else {
        println!("Agent note:");
    }
    for line in note.lines() {
        println!("  {line}");
    }
}

/// Render any custom annotation fields (e.g. `@created`) not otherwise
/// surfaced by a dedicated field.
pub fn render_custom_metadata(custom: &HashMap<String, String>, theme: Option<&CliTheme>) {
    if custom.is_empty() {
        return;
    }

    let mut keys: Vec<&String> = custom.keys().collect();
    keys.sort();

    for key in keys {
        let value = &custom[key];
        let label = format!("  {}:", capitalize(key));
        if let Some(theme) = theme {
            println!("{} {}", theme.style_muted(&label), value);
        } else {
            println!("{label} {value}");
        }
    }
}

/// Render the resolved `@depends-on` list with a satisfied/total summary.
/// No-op when `task` has no (non-directory, non-hierarchy) dependencies.
pub fn render_dependencies(deps: &[DependencyStatus], theme: Option<&CliTheme>) {
    if deps.is_empty() {
        return;
    }
    let (satisfied, total) = dependency_counts(deps);

    println!();
    let header = format!("Depends on ({satisfied}/{total} satisfied):");
    if let Some(theme) = theme {
        println!("{}", theme.style_info(&header));
    } else {
        println!("{header}");
    }

    for dep in deps {
        let mark = if dep.satisfied {
            "\u{2713}"
        } else {
            "\u{2717}"
        };
        match (&dep.full_id, &dep.title, dep.status) {
            (Some(full_id), Some(title), Some(status)) => {
                if let Some(theme) = theme {
                    let checkbox = theme.styled_checkbox(status);
                    let full_id = theme.style_muted(full_id);
                    println!("  {mark} {checkbox} {title} ({full_id})");
                } else {
                    println!("  {mark} [{}] {title} ({full_id})", status.as_str());
                }
            }
            _ => {
                if let Some(theme) = theme {
                    println!(
                        "  {mark} {} {}",
                        theme.style_error("[unresolved]"),
                        theme.style_muted(&dep.reference)
                    );
                } else {
                    println!("  {mark} [unresolved] {}", dep.reference);
                }
            }
        }
    }
}

/// Render the direct-children summary with a done/total header. No-op when
/// `children` is empty.
pub fn render_children(children: &[ChildSummary], theme: Option<&CliTheme>) {
    if children.is_empty() {
        return;
    }
    let (done, total) = children_counts(children);

    println!();
    let header = format!("Children ({done}/{total} done):");
    if let Some(theme) = theme {
        println!("{}", theme.style_info(&header));
    } else {
        println!("{header}");
    }

    for child in children {
        let nested = if child.nested_count > 0 {
            format!(" — {} nested", child.nested_count)
        } else {
            String::new()
        };
        if let Some(theme) = theme {
            let checkbox = theme.styled_checkbox(child.status);
            let full_id = theme.style_muted(&child.full_id);
            println!("  {checkbox} {}{nested} ({full_id})", child.title);
        } else {
            println!(
                "  {} {}{nested} ({})",
                checkbox_icon(child.status),
                child.title,
                child.full_id
            );
        }
    }
}

/// Plain-text checkbox icon (mirrors `show/mod.rs`'s themed equivalent for
/// the no-color / no-theme path).
fn checkbox_icon(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Open => "[ ]",
        TaskStatus::InProgress => "[>]",
        TaskStatus::Done => "[x]",
        TaskStatus::Waived => "[-]",
        TaskStatus::Blocked => "[!]",
    }
}

/// Capitalize the first character of `s` (ASCII-aware; good enough for the
/// short lowercase annotation keys Lash uses, e.g. `created` -> `Created`).
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::dependency::DependencyRef;
    use lash_types::task::TaskBuilder;
    use lash_types::{FileMetadata, TaskTree};
    use std::time::SystemTime;

    fn file_with_tasks(path: &str, id: &str, tasks: Vec<lash_types::Task>) -> (PathBuf, TaskFile) {
        let mut tree = TaskTree::new();
        for task in tasks {
            tree.add_task(task).unwrap();
        }
        let pb = PathBuf::from(path);
        let file = TaskFile {
            path: pb.clone(),
            title: "T".to_string(),
            id: id.to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: tree,
            hash: "h".to_string(),
            mtime: SystemTime::now(),
        };
        (pb, file)
    }

    fn task_record(depends_on: Vec<DependencyRef>) -> TaskRecord {
        TaskRecord {
            id: 1,
            file_id: 1,
            local_id: "source".to_string(),
            full_id: "src#source".to_string(),
            title: "Source task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            owner: None,
            estimate: None,
            body: None,
            metadata: lash_types::TaskMetadata {
                depends_on,
                ..Default::default()
            },
            contextual_notes: Vec::new(),
        }
    }

    #[test]
    fn resolves_dependency_to_satisfied_status() {
        let done_task = TaskBuilder::new("Pay flow")
            .id("pay-flow")
            .status(TaskStatus::Done)
            .build()
            .unwrap();
        let (p, f) = file_with_tasks("launch.md", "launch", vec![done_task]);
        let mut project = HashMap::new();
        project.insert(p.clone(), f);

        let task = task_record(vec![DependencyRef::new(
            "launch#pay-flow".to_string(),
            DependencyKind::ExplicitId,
        )]);
        let config = LashConfig::default();
        let src_path = PathBuf::from("src.md");
        let deps = resolve_dependencies(&config, &project, &src_path, "src", &task);

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].full_id.as_deref(), Some("launch#pay-flow"));
        assert_eq!(deps[0].status, Some(TaskStatus::Done));
        assert!(deps[0].satisfied);
        assert_eq!(dependency_counts(&deps), (1, 1));
    }

    #[test]
    fn unresolvable_reference_reports_unresolved_not_a_crash() {
        let project = HashMap::new();
        let task = task_record(vec![DependencyRef::new(
            "ghost-file#ghost-task".to_string(),
            DependencyKind::ExplicitId,
        )]);
        let config = LashConfig::default();
        let src_path = PathBuf::from("src.md");
        let deps = resolve_dependencies(&config, &project, &src_path, "src", &task);

        assert_eq!(deps.len(), 1);
        assert!(deps[0].full_id.is_none());
        assert!(!deps[0].satisfied);
        assert_eq!(dependency_counts(&deps), (0, 1));
    }

    #[test]
    fn directory_dependencies_are_skipped() {
        let project = HashMap::new();
        let task = task_record(vec![DependencyRef::new(
            "some-dir/".to_string(),
            DependencyKind::Directory,
        )]);
        let config = LashConfig::default();
        let src_path = PathBuf::from("src.md");
        let deps = resolve_dependencies(&config, &project, &src_path, "src", &task);

        assert!(deps.is_empty());
    }

    #[test]
    fn capitalize_handles_empty_and_normal_strings() {
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("created"), "Created");
    }
}
