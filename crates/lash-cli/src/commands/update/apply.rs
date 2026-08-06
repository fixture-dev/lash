//! Translate `lash update` arguments into a validated, in-memory edit of a
//! task's markdown, without touching disk.
//!
//! [`build_plan`] does all validation up front (dangling `--add-depends-on`
//! targets, missing `--remove-label`/`--remove-depends-on` targets) so a
//! failure never leaves a partially-edited file: every mutation happens on
//! an in-memory [`TaskLines`], and the caller only writes it to disk (or
//! prints a diff, for `--dry-run`) once the whole plan has succeeded.

use std::collections::HashSet;
use std::path::Path;

use lash_types::label::normalize as normalize_label;
use lash_types::Task;

use crate::commands::add_dependency_check::{
    validate_depends_on, DependsOnValidation, UnresolvedDependency,
};

use super::mutations::TaskLines;
use super::UpdateArgs;

/// A successfully validated, applied-in-memory update.
pub struct Plan {
    /// The task's file content before any mutation, for `--dry-run` diffs.
    pub original_content: String,
    /// The task's file lines after every mutation has been applied.
    pub lines: TaskLines,
    /// Human-readable descriptions of each change made, in the order applied.
    pub changes: Vec<String>,
    /// The old title-derived slug, if `--title` triggered an auto-pin.
    pub pinned_id: Option<String>,
    /// `--add-depends-on` targets that didn't resolve but were allowed
    /// through by `--allow-forward-ref`.
    pub warnings: Vec<UnresolvedDependency>,
}

/// Why a plan could not be built.
pub enum PlanError {
    /// One or more `--add-depends-on` targets failed to resolve (and
    /// `--allow-forward-ref` wasn't passed to downgrade them to warnings).
    DependsOn(Vec<UnresolvedDependency>),
    /// Any other validation failure: (error code, message).
    Message(String, String),
}

/// Build the full edit plan for one `lash update` invocation.
///
/// Validates every mutation before applying any of them, so a single
/// invalid flag (a dangling `--add-depends-on` target, a `--remove-label`
/// the task doesn't have) leaves the in-memory `TaskLines` never written and
/// the caller free to just discard it.
///
/// # Errors
///
/// Returns [`PlanError`] if `--add-depends-on` contains an unresolvable
/// reference (and `--allow-forward-ref` wasn't passed), a `--remove-label`
/// or `--remove-depends-on` target isn't present on the task, or the file
/// can't be read.
pub fn build_plan(
    project_root: &Path,
    rel_path: &Path,
    task: &Task,
    args: &UpdateArgs,
) -> Result<Plan, PlanError> {
    let validation = if args.add_depends_on.is_empty() {
        DependsOnValidation::default()
    } else {
        validate_depends_on(
            project_root,
            Some(rel_path),
            &args.add_depends_on,
            args.allow_forward_ref,
        )
    };
    if validation.has_errors() {
        return Err(PlanError::DependsOn(validation.errors));
    }

    let full_path = project_root.join(rel_path);
    let mut lines = TaskLines::load(&full_path, task.line_number)
        .map_err(|e| PlanError::Message("E_FILE_READ".to_string(), e.to_string()))?;
    let original_content = lines.render();

    let mut changes = Vec::new();
    let mut pinned_id = None;

    apply_title(&mut lines, task, args, &mut changes, &mut pinned_id);
    apply_labels(&mut lines, task, args, &mut changes)?;
    apply_single_annotation(
        &mut lines,
        "owner",
        task.metadata.owner.as_deref(),
        args.owner.as_ref(),
        &mut changes,
    );
    apply_single_annotation(
        &mut lines,
        "estimate",
        task.metadata.estimate.as_deref(),
        args.estimate.as_ref(),
        &mut changes,
    );
    apply_agent_note(&mut lines, args, &mut changes);
    apply_depends_on(&mut lines, args, &mut changes)?;

    Ok(Plan {
        original_content,
        lines,
        changes,
        pinned_id,
        warnings: validation.warnings,
    })
}

/// `--title`: pin the old title-derived `@id` first if the task doesn't
/// already have an explicit one (GitHub issue #25's core requirement — every
/// `@depends-on` reference pointing at the old derived slug must keep
/// resolving), then rewrite the title.
fn apply_title(
    lines: &mut TaskLines,
    task: &Task,
    args: &UpdateArgs,
    changes: &mut Vec<String>,
    pinned_id: &mut Option<String>,
) {
    let Some(new_title) = &args.title else {
        return;
    };
    if !task.has_explicit_id {
        lines.pin_id(&task.id);
        *pinned_id = Some(task.id.clone());
    }
    let old_title = lines.title_text().to_string();
    lines.retitle(new_title);
    changes.push(format!("title: '{old_title}' -> '{}'", lines.title_text()));
}

/// `--add-label` / `--remove-label`: edit whichever form (inline `#tag` or
/// `@labels:`) the task already uses; new tasks with no labels yet default
/// to the inline form, matching how `lash add --label` writes new tasks.
fn apply_labels(
    lines: &mut TaskLines,
    task: &Task,
    args: &UpdateArgs,
    changes: &mut Vec<String>,
) -> Result<(), PlanError> {
    let mut current_labels: HashSet<String> = task.metadata.labels.iter().cloned().collect();

    for raw in &args.add_label {
        let norm = normalize_label(raw);
        if current_labels.contains(&norm) {
            changes.push(format!("label '{norm}' already present (no-op)"));
            continue;
        }
        if lines.has_labels_annotation() {
            lines.add_labels_annotation_value(raw);
        } else {
            lines.add_inline_label(raw);
        }
        current_labels.insert(norm.clone());
        changes.push(format!("added label #{norm}"));
    }

    for raw in &args.remove_label {
        let norm = normalize_label(raw);
        if !current_labels.contains(&norm) {
            return Err(PlanError::Message(
                "E_LABEL_NOT_FOUND".to_string(),
                format!("Task does not have label '{norm}'"),
            ));
        }
        let removed = lines.remove_inline_label(raw) || lines.remove_labels_annotation_value(raw);
        if !removed {
            return Err(PlanError::Message(
                "E_LABEL_NOT_FOUND".to_string(),
                format!("Task does not have label '{norm}'"),
            ));
        }
        current_labels.remove(&norm);
        changes.push(format!("removed label #{norm}"));
    }

    Ok(())
}

/// `--owner` / `--estimate`: set, replace, or (given `""`) remove a
/// single-value annotation.
fn apply_single_annotation(
    lines: &mut TaskLines,
    key: &str,
    old_value: Option<&str>,
    new_value: Option<&String>,
    changes: &mut Vec<String>,
) {
    let Some(new_value) = new_value else {
        return;
    };
    let value = if new_value.is_empty() {
        None
    } else {
        Some(new_value.as_str())
    };
    lines.set_single_annotation(key, value);
    changes.push(describe_single_set(key, old_value, value));
}

fn describe_single_set(key: &str, old: Option<&str>, new: Option<&str>) -> String {
    match (old, new) {
        (Some(o), Some(n)) => format!("{key}: '{o}' -> '{n}'"),
        (None, Some(n)) => format!("set {key}: '{n}'"),
        (Some(o), None) => format!("removed {key} (was '{o}')"),
        (None, None) => format!("{key} unchanged (already unset)"),
    }
}

/// `--agent-note` (replace) / `--append-agent-note` (add a continuation
/// line).
fn apply_agent_note(lines: &mut TaskLines, args: &UpdateArgs, changes: &mut Vec<String>) {
    if let Some(text) = &args.agent_note {
        lines.set_agent_note(text);
        changes.push("replaced @agent-note".to_string());
    }
    if let Some(text) = &args.append_agent_note {
        lines.append_agent_note(text);
        changes.push("appended to @agent-note".to_string());
    }
}

/// `--add-depends-on` (already validated by the caller) / `--remove-depends-on`.
fn apply_depends_on(
    lines: &mut TaskLines,
    args: &UpdateArgs,
    changes: &mut Vec<String>,
) -> Result<(), PlanError> {
    for dep in &args.add_depends_on {
        lines.add_depends_on(dep);
        changes.push(format!("added dependency: {dep}"));
    }
    for dep in &args.remove_depends_on {
        lines
            .remove_depends_on(dep)
            .map_err(|e| PlanError::Message("E_DEPENDS_ON_NOT_FOUND".to_string(), e.to_string()))?;
        changes.push(format!("removed dependency: {dep}"));
    }
    Ok(())
}
