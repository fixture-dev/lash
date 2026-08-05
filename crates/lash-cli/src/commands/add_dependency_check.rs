//! Validation of `--depends-on` references for `lash add` (GitHub issue #27)
//!
//! `lash add --depends-on <ref>` used to write the reference straight into
//! the Markdown file with no validation at all: a typo, or a target that
//! simply doesn't exist yet, only surfaced later via `lash lint`
//! (`E_LINK_NOT_FOUND`). This module resolves every `--depends-on` reference
//! against the *current* on-disk project, using the same
//! [`resolve_reference`] resolver that `lint`, `check-links`, and `complete`
//! use, so all four surfaces agree on what resolves.
//!
//! By default an unresolvable reference is a hard error: the task is not
//! created and no file is written. Passing `--allow-forward-ref` downgrades
//! that to a warning on stderr and writes the task anyway, for the
//! legitimate case of creating tasks in dependency order before their
//! targets.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use lash_cli::theme::CliTheme;
use lash_core::dependency::reference::{resolve_reference, RefError};
use lash_core::fuzzy::FuzzyMatcher;
use lash_core::linter::LintContext;
use lash_types::creation::FileTarget;
use lash_types::TaskFile;

use crate::utils::project_loader::load_project;

/// A single `--depends-on` reference that failed to resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedDependency {
    /// The reference exactly as passed on the command line.
    pub target: String,
    /// Human-readable reason it didn't resolve.
    pub reason: String,
    /// Fuzzy "did you mean" suggestions, closest first (may be empty).
    pub suggestions: Vec<String>,
}

/// Outcome of validating a set of `--depends-on` references.
#[derive(Debug, Clone, Default)]
pub struct DependsOnValidation {
    /// References that failed to resolve but were downgraded to warnings
    /// because `--allow-forward-ref` was passed. The caller should print
    /// these and proceed.
    pub warnings: Vec<UnresolvedDependency>,
    /// References that failed to resolve and were NOT allowed as forward
    /// refs. Non-empty means the caller must refuse to create the task.
    pub errors: Vec<UnresolvedDependency>,
}

impl DependsOnValidation {
    /// True when at least one reference is a hard error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Validate every `--depends-on` reference against the on-disk project.
///
/// `target_path` is the root-relative path of the file the new task will be
/// written to. It is used to resolve same-file (`#task:id`) references and
/// relative paths, exactly like `lash lint` resolves them for an existing
/// task. Pass `None` when it can't be determined (e.g. no `--file` was
/// given); same-file and relative references simply won't resolve in that
/// case, same as they wouldn't for `lint`.
///
/// Returns immediately with an empty result if `depends_on` is empty, so
/// callers can call this unconditionally without a project scan on the
/// common case of a task with no dependencies.
#[must_use]
pub fn validate_depends_on(
    project_root: &Path,
    target_path: Option<&Path>,
    depends_on: &[String],
    allow_forward_ref: bool,
) -> DependsOnValidation {
    let mut result = DependsOnValidation::default();
    if depends_on.is_empty() {
        return result;
    }

    let (config, files) = load_project(project_root);
    let empty_path = PathBuf::new();
    let source_path = target_path.unwrap_or(&empty_path);
    let ctx = LintContext::new(&config, source_path.to_path_buf(), &files);
    let candidates = reference_candidates(&files);

    for target in depends_on {
        let resolve_path = |rel: &str| ctx.resolve_path(Path::new(rel));
        if let Err(err) = resolve_reference(target, source_path, "", &files, resolve_path) {
            let unresolved = UnresolvedDependency {
                target: target.clone(),
                reason: describe_error(&err),
                suggestions: suggest(target, &candidates),
            };
            if allow_forward_ref {
                result.warnings.push(unresolved);
            } else {
                result.errors.push(unresolved);
            }
        }
    }

    result
}

/// Every `file-id#task-id` (plus bare file ids and bare task ids) in the
/// project, as candidates for fuzzy "did you mean" suggestions.
///
/// Bare task ids are included alongside the fully-qualified form because
/// `--depends-on` references are usually written as a bare `@id` (e.g.
/// `base-task`, not `tasks#base-task`); comparing a typo'd bare id only
/// against `file#id`-shaped candidates would fail on length alone.
fn reference_candidates(files: &HashMap<PathBuf, TaskFile>) -> Vec<String> {
    let mut candidates = Vec::new();
    for file in files.values() {
        candidates.push(file.id.clone());
        for task in file.tasks.tasks() {
            candidates.push(task.id.clone());
            candidates.push(format!("{}#{}", file.id, task.id));
        }
    }
    candidates
}

/// Find close fuzzy matches for an unresolved reference among known ids.
fn suggest(target: &str, candidates: &[String]) -> Vec<String> {
    let matcher = FuzzyMatcher::new(0.5, 3);
    matcher
        .find_matches(target, candidates)
        .into_iter()
        .map(|c| c.task_id)
        .collect()
}

/// Turn a [`RefError`] into a one-line, user-facing explanation, matching
/// the wording `lash lint`'s `E_LINK_NOT_FOUND` rule uses.
fn describe_error(err: &RefError) -> String {
    match err {
        RefError::FileNotFound { reference } => format!(
            "'{reference}' does not match any file id, file path, or task @id in the project"
        ),
        RefError::TaskNotFound {
            file_label,
            task,
            available,
        } => {
            if available.is_empty() {
                format!("task '{task}' not found in file '{file_label}'")
            } else {
                format!(
                    "task '{task}' not found in file '{file_label}' (available: {})",
                    available.join(", ")
                )
            }
        }
    }
}

/// Root-relative path of the file a task will be written to, for resolving
/// `--depends-on` references (GitHub issue #27).
///
/// `resolve_reference` keys its project map by root-relative path, matching
/// what `lash lint`/`check-links` use. Returns `None` for `FileTarget::Current`
/// or `FileTarget::ContainingTask`, which `lash add` doesn't currently
/// produce (no `--file` resolves to `Current`; `ContainingTask` is TUI-only) —
/// same-file and relative `--depends-on` references simply won't resolve in
/// that case, matching how `lint` treats an unknown file.
#[must_use]
pub fn file_target_relative_path(target: &FileTarget, project_root: &Path) -> Option<PathBuf> {
    let abs_path = match target {
        FileTarget::Path(path) | FileTarget::NewFile { path, .. } => path,
        FileTarget::Current | FileTarget::ContainingTask(_) => return None,
    };
    Some(
        abs_path
            .strip_prefix(project_root)
            .map_or_else(|_| abs_path.clone(), Path::to_path_buf),
    )
}

/// Print warnings for `--depends-on` refs that didn't resolve but were
/// allowed through by `--allow-forward-ref`.
pub fn emit_depends_on_warnings(
    warnings: &[UnresolvedDependency],
    format: &str,
    theme: Option<&CliTheme>,
) {
    if format == "json" || warnings.is_empty() {
        // JSON output is machine-consumed; forward-ref warnings are folded
        // into the success payload's `warnings` field instead (see
        // `commands::add::output_success`) rather than interleaved on stderr.
        return;
    }
    for w in warnings {
        let message = format!(
            "depends-on target '{}' not resolved ({}); writing anyway due to --allow-forward-ref",
            w.target, w.reason
        );
        if let Some(t) = theme {
            eprintln!("{} {}", t.style_warning("Warning:"), message);
        } else {
            eprintln!("Warning: {message}");
        }
        if !w.suggestions.is_empty() {
            eprintln!("  hint: did you mean: {}", w.suggestions.join(", "));
        }
    }
}

/// Report unresolved `--depends-on` refs as a hard error and refuse to
/// create the task (GitHub issue #27).
///
/// Reuses the error code `TaskCreationError::DependencyNotFound` already
/// defines (`E_CREATE_DEPENDENCY_NOT_FOUND`) and `docs/error-codes.md`
/// already documents, even though this check runs ahead of — and
/// independently from — the `TaskCreationError` validation pipeline.
///
/// # Errors
///
/// Returns an error only if writing to stdout fails.
pub fn output_depends_on_errors(
    errors: &[UnresolvedDependency],
    format: &str,
    theme: Option<&CliTheme>,
) -> Result<()> {
    if format == "json" {
        let json = serde_json::json!({
            "success": false,
            "errors": errors.iter().map(|e| serde_json::json!({
                "code": "E_CREATE_DEPENDENCY_NOT_FOUND",
                "target": e.target,
                "message": e.reason,
                "suggestions": e.suggestions,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    for e in errors {
        let header = format!("dependency not found '{}': {}", e.target, e.reason);
        if let Some(t) = theme {
            eprintln!(
                "{} [E_CREATE_DEPENDENCY_NOT_FOUND]: {}",
                t.style_error("Error"),
                header
            );
            if !e.suggestions.is_empty() {
                eprintln!(
                    "  {} did you mean: {}",
                    t.style_info("hint:"),
                    e.suggestions.join(", ")
                );
            }
            eprintln!(
                "  {} ensure the referenced task exists, or pass --allow-forward-ref to create the task anyway and add the dependency later",
                t.style_info("help:")
            );
        } else {
            eprintln!("Error [E_CREATE_DEPENDENCY_NOT_FOUND]: {header}");
            if !e.suggestions.is_empty() {
                eprintln!("  hint: did you mean: {}", e.suggestions.join(", "));
            }
            eprintln!(
                "  help: ensure the referenced task exists, or pass --allow-forward-ref to create the task anyway and add the dependency later"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn project_with_task(temp: &Path) {
        fs::write(
            temp.join("tasks.md"),
            "# Tasks\n\n@id: tasks\n\n## Tasks\n\n- [ ] Base task\n  @id: base-task\n",
        )
        .unwrap();
    }

    #[test]
    fn empty_depends_on_is_always_valid() {
        let temp = tempfile::TempDir::new().unwrap();
        let result = validate_depends_on(temp.path(), None, &[], false);
        assert!(!result.has_errors());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn resolvable_reference_produces_no_issues() {
        let temp = tempfile::TempDir::new().unwrap();
        project_with_task(temp.path());

        let result = validate_depends_on(
            temp.path(),
            Some(Path::new("tasks.md")),
            &["base-task".to_string()],
            false,
        );
        assert!(!result.has_errors());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn dangling_reference_is_a_hard_error_by_default() {
        let temp = tempfile::TempDir::new().unwrap();
        project_with_task(temp.path());

        let result = validate_depends_on(
            temp.path(),
            Some(Path::new("tasks.md")),
            &["does-not-exist".to_string()],
            false,
        );
        assert!(result.has_errors());
        assert!(result.warnings.is_empty());
        assert_eq!(result.errors[0].target, "does-not-exist");
    }

    #[test]
    fn dangling_reference_downgrades_to_warning_with_allow_forward_ref() {
        let temp = tempfile::TempDir::new().unwrap();
        project_with_task(temp.path());

        let result = validate_depends_on(
            temp.path(),
            Some(Path::new("tasks.md")),
            &["not-yet-created".to_string()],
            true,
        );
        assert!(!result.has_errors());
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].target, "not-yet-created");
    }

    #[test]
    fn close_typo_surfaces_a_fuzzy_suggestion() {
        let temp = tempfile::TempDir::new().unwrap();
        project_with_task(temp.path());

        let result = validate_depends_on(
            temp.path(),
            Some(Path::new("tasks.md")),
            &["base-taks".to_string()], // typo: transposed letters
            false,
        );
        assert!(result.has_errors());
        assert!(
            result.errors[0]
                .suggestions
                .iter()
                .any(|s| s.contains("base-task")),
            "expected a fuzzy suggestion for 'base-taks', got: {:?}",
            result.errors[0].suggestions
        );
    }

    #[test]
    fn mixed_refs_partition_into_errors_and_warnings() {
        let temp = tempfile::TempDir::new().unwrap();
        project_with_task(temp.path());

        let result = validate_depends_on(
            temp.path(),
            Some(Path::new("tasks.md")),
            &["base-task".to_string(), "missing".to_string()],
            true,
        );
        assert!(!result.has_errors());
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].target, "missing");
    }

    #[test]
    fn file_target_relative_path_strips_project_root() {
        let root = Path::new("/project");
        let target = FileTarget::Path(PathBuf::from("/project/tasks/backend.md"));
        assert_eq!(
            file_target_relative_path(&target, root),
            Some(PathBuf::from("tasks/backend.md"))
        );
    }

    #[test]
    fn file_target_relative_path_handles_new_file_target() {
        let root = Path::new("/project");
        let target = FileTarget::NewFile {
            path: PathBuf::from("/project/new.md"),
            title: None,
            description: None,
        };
        assert_eq!(
            file_target_relative_path(&target, root),
            Some(PathBuf::from("new.md"))
        );
    }

    #[test]
    fn file_target_relative_path_is_none_for_current_and_containing_task() {
        let root = Path::new("/project");
        assert_eq!(file_target_relative_path(&FileTarget::Current, root), None);
        assert_eq!(
            file_target_relative_path(
                &FileTarget::ContainingTask("tasks.md#task:x".to_string()),
                root
            ),
            None
        );
    }
}
