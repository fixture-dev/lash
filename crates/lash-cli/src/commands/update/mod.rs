//! Update command implementation
//!
//! The `lash update` command edits fields on a single existing task —
//! title, labels, owner, estimate, agent note, and `@depends-on` — without
//! hand-editing Markdown (GitHub issue #25).
//!
//! The riskiest of these is `--title`: a task with no explicit `@id`
//! derives its id from the first 40 characters of its (kebab-cased) title,
//! so retitling silently changes the id every `@depends-on` reference in
//! the project used to point at. To keep those references resolving, a
//! retitle that would change a derived id first pins the *old* derived slug
//! as an explicit `@id:` annotation, then rewrites the title — see
//! `apply::apply_title`.
//!
//! Mirrors `lash waive`'s shape (resolve target, validate, mutate, reindex,
//! JSON/text output, `--dry-run`) but is split across three files since the
//! mutation surface is much larger:
//! - `mod.rs` (this file) — CLI args, orchestration, output
//! - `apply.rs` — validates flags and applies them to an in-memory edit
//! - `mutations.rs` — the line-editing primitives `apply.rs` calls

mod apply;
mod mutations;

use anyhow::{Context, Result};
use lash::diff_display::DiffDisplay;
use lash::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash::formatter::{OutputFormat, Verbosity};
use lash::theme::CliTheme;
use lash_db::{open_database, TaskRepository};
use lash_types::error::LashError;
use std::path::{Path, PathBuf};

use apply::{Plan, PlanError};

use crate::commands::add_dependency_check::{emit_depends_on_warnings, output_depends_on_errors};
use crate::commands::status_mutation::{self, find_similar_task_ids};
use crate::utils::file_discovery::find_project_root;
use crate::utils::project_loader::{find_task_by_full_id, load_project};
use crate::utils::task_target::TargetError;

/// Arguments for the update command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct UpdateArgs {
    /// Task ID to update (supports fuzzy matching)
    pub task_id: String,
    /// Rewrite the task's title
    pub title: Option<String>,
    /// Labels to add (repeatable)
    pub add_label: Vec<String>,
    /// Labels to remove (repeatable)
    pub remove_label: Vec<String>,
    /// Set (or, given `""`, remove) the task's owner
    pub owner: Option<String>,
    /// Set (or, given `""`, remove) the task's estimate
    pub estimate: Option<String>,
    /// Replace the task's `@agent-note`
    pub agent_note: Option<String>,
    /// Append a line to the task's `@agent-note`
    pub append_agent_note: Option<String>,
    /// `@depends-on` references to add (repeatable), validated against the
    /// project unless `allow_forward_ref` is set
    pub add_depends_on: Vec<String>,
    /// `@depends-on` references to remove (repeatable), matched exactly
    pub remove_depends_on: Vec<String>,
    /// Downgrade an unresolved `--add-depends-on` target to a warning
    pub allow_forward_ref: bool,
    /// Preview what would change without modifying files
    pub dry_run: bool,
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Verbosity level for output
    pub verbosity: Verbosity,
}

impl UpdateArgs {
    /// Whether at least one field-mutating flag was passed. `lash update`
    /// with no mutation flags is a no-op that would be surprising to run
    /// silently, so it's an error instead.
    fn has_mutation(&self) -> bool {
        self.title.is_some()
            || !self.add_label.is_empty()
            || !self.remove_label.is_empty()
            || self.owner.is_some()
            || self.estimate.is_some()
            || self.agent_note.is_some()
            || self.append_agent_note.is_some()
            || !self.add_depends_on.is_empty()
            || !self.remove_depends_on.is_empty()
    }
}

/// Execute the update command
///
/// # Arguments
///
/// * `args` - Update command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (validation error), 3 (DB/IO error), 5 (not found)
#[allow(clippy::too_many_lines)]
pub fn execute(args: &UpdateArgs) -> Result<i32> {
    let project_root = if let Some(ref root) = args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        task_id = %args.task_id,
        dry_run = args.dry_run,
        "Starting update operation"
    );

    let theme = CliTheme::load(None, !args.no_color)?;

    if !args.has_mutation() {
        let message = "At least one mutation flag is required: --title, --add-label, \
             --remove-label, --owner, --estimate, --agent-note, --append-agent-note, \
             --add-depends-on, or --remove-depends-on"
            .to_string();
        report_error(args.json, theme.as_ref(), "E_NO_MUTATION", &message, &[])?;
        return Ok(1);
    }

    let db_path = project_root.join(".lash/lash.db");
    if !db_path.exists() {
        let error = LashError::io_file_not_found(db_path.clone());
        let mut diag = error.to_diagnostic();
        diag.help = Some("Run `lash index` to create the database".to_string());
        report_diagnostic(args, theme.as_ref(), &diag);
        return Ok(3);
    }

    let conn = match open_database(&db_path) {
        Ok(conn) => conn,
        Err(e) => {
            let error = LashError::index_corrupted(format!("Failed to open database: {e}"));
            let mut diag = error.to_diagnostic();
            diag.help = Some("Try running `lash index` to rebuild the database".to_string());
            report_diagnostic(args, theme.as_ref(), &diag);
            return Ok(3);
        }
    };

    let task_repo = TaskRepository::new(&conn);

    let record = match crate::utils::task_target::resolve_task_target(&task_repo, &args.task_id) {
        Ok(record) => record,
        Err(TargetError::NotFound) => {
            let all_ids = task_repo.get_all_full_ids().unwrap_or_default();
            let suggestions: Vec<String> = find_similar_task_ids(&args.task_id, &all_ids)
                .into_iter()
                .map(|(id, _)| id)
                .collect();
            let message = format!("Task not found: {}", args.task_id);
            report_error(
                args.json,
                theme.as_ref(),
                "E_NOT_FOUND",
                &message,
                &suggestions,
            )?;
            return Ok(5);
        }
        Err(TargetError::Ambiguous(candidates)) => {
            let message = format!(
                "Task @id '{}' is ambiguous; matches {} tasks",
                args.task_id,
                candidates.len()
            );
            report_error(
                args.json,
                theme.as_ref(),
                "E_AMBIGUOUS",
                &message,
                &candidates,
            )?;
            return Ok(1);
        }
        Err(TargetError::Db(e)) => {
            let message = format!("Database error: {e}");
            report_error(args.json, theme.as_ref(), "E_DB_ERROR", &message, &[])?;
            return Ok(3);
        }
    };

    let (_config, files) = load_project(&project_root);
    let Some((rel_path, _file, task)) = find_task_by_full_id(&files, &record.full_id) else {
        let message = format!(
            "Task '{}' is in the index but not in the current Markdown; run `lash index`",
            record.full_id
        );
        report_error(args.json, theme.as_ref(), "E_INDEX_STALE", &message, &[])?;
        return Ok(3);
    };

    match apply::build_plan(&project_root, rel_path, task, args) {
        Ok(plan) => finish(
            args,
            &project_root,
            &record.full_id,
            rel_path,
            &plan,
            theme.as_ref(),
        ),
        Err(PlanError::DependsOn(errors)) => {
            let format = if args.json { "json" } else { "text" };
            output_depends_on_errors(&errors, format, theme.as_ref())?;
            Ok(1)
        }
        Err(PlanError::Message(code, message)) => {
            let exit_code = if code == "E_FILE_READ" { 3 } else { 1 };
            report_error(args.json, theme.as_ref(), &code, &message, &[])?;
            Ok(exit_code)
        }
    }
}

/// Write the plan (or, for `--dry-run`, just print what it would do) and
/// re-index the project.
fn finish(
    args: &UpdateArgs,
    project_root: &Path,
    task_id: &str,
    rel_path: &Path,
    plan: &Plan,
    theme: Option<&CliTheme>,
) -> Result<i32> {
    if args.dry_run {
        output_result(task_id, rel_path, plan, theme, args.json, true)?;
        return Ok(0);
    }

    let full_path = project_root.join(rel_path);
    plan.lines
        .write(&full_path)
        .with_context(|| format!("Failed to write file: {}", full_path.display()))?;

    if let Err(e) = status_mutation::reindex_project(project_root, "updating task") {
        tracing::warn!("Failed to re-index after update: {e}");
    }

    output_result(task_id, rel_path, plan, theme, args.json, false)?;
    Ok(0)
}

/// Render a diagnostic (DB-not-found / DB-corrupted) consistently with the
/// rest of the mutation commands (`waive`, `start`, ...).
fn report_diagnostic(
    args: &UpdateArgs,
    theme: Option<&CliTheme>,
    diag: &lash_types::error::Diagnostic,
) {
    if args.json {
        let mut json = serde_json::json!({
            "success": false,
            "error": { "code": diag.code, "message": diag.message },
        });
        if let Some(help) = &diag.help {
            json["error"]["help"] = serde_json::Value::String(help.clone());
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&json).unwrap_or_default()
        );
    } else {
        let reporter_config = ErrorReporterConfig {
            verbosity: args.verbosity,
            output_format: OutputFormat::Text,
            display_mode: ErrorDisplayMode::Streaming,
            theme: theme.cloned(),
            show_summary: false,
        };
        let mut reporter = ErrorReporter::new(reporter_config);
        reporter.report_diagnostic(diag);
    }
}

/// Report a single error, as JSON or as themed text, with optional
/// "did you mean" suggestions.
fn report_error(
    json: bool,
    theme: Option<&CliTheme>,
    code: &str,
    message: &str,
    suggestions: &[String],
) -> Result<()> {
    if json {
        let mut json_value = serde_json::json!({
            "success": false,
            "error": { "code": code, "message": message },
        });
        if !suggestions.is_empty() {
            json_value["error"]["suggestions"] = serde_json::json!(suggestions);
        }
        println!("{}", serde_json::to_string_pretty(&json_value)?);
        return Ok(());
    }
    if let Some(t) = theme {
        eprintln!("{} [{code}]: {message}", t.style_error("Error"));
        if !suggestions.is_empty() {
            eprintln!(
                "  {} Did you mean: {}",
                t.style_info("hint:"),
                suggestions.join(", ")
            );
        }
    } else {
        eprintln!("Error [{code}]: {message}");
        if !suggestions.is_empty() {
            eprintln!("  hint: Did you mean: {}", suggestions.join(", "));
        }
    }
    Ok(())
}

/// Render the outcome of a successful (possibly dry-run) update.
fn output_result(
    task_id: &str,
    file_path: &Path,
    plan: &Plan,
    theme: Option<&CliTheme>,
    json: bool,
    dry_run: bool,
) -> Result<()> {
    if json {
        let json_value = serde_json::json!({
            "success": true,
            "dry_run": dry_run,
            "task_id": task_id,
            "file_path": file_path,
            "pinned_id": plan.pinned_id,
            "changes": plan.changes,
            "warnings": plan.warnings.iter().map(|w| serde_json::json!({
                "code": "E_CREATE_DEPENDENCY_NOT_FOUND",
                "target": w.target,
                "message": w.reason,
                "suggestions": w.suggestions,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&json_value)?);
        return Ok(());
    }

    let verb = if dry_run { "Would update" } else { "Updated" };
    if let Some(t) = theme {
        println!(
            "{} {} -> {}",
            t.style_success(verb),
            t.style_label(task_id),
            t.style_muted(&file_path.display().to_string())
        );
    } else {
        println!("{verb} {task_id} -> {}", file_path.display());
    }

    if let Some(slug) = &plan.pinned_id {
        let line = format!("pinned @id: {slug} to preserve references");
        if let Some(t) = theme {
            println!("  {}", t.style_info(&line));
        } else {
            println!("  {line}");
        }
    }
    for change in &plan.changes {
        println!("  {change}");
    }

    emit_depends_on_warnings(&plan.warnings, "text", theme);

    if dry_run {
        let display = theme
            .cloned()
            .map_or_else(DiffDisplay::new, DiffDisplay::with_theme);
        if let Some(diff) = display.unified_diff(&plan.original_content, &plan.lines.render()) {
            println!();
            println!("{diff}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal `UpdateArgs` with every mutation flag unset, for tests that
    /// only care about one field at a time.
    fn base_args() -> UpdateArgs {
        UpdateArgs {
            task_id: "t#a".to_string(),
            title: None,
            add_label: vec![],
            remove_label: vec![],
            owner: None,
            estimate: None,
            agent_note: None,
            append_agent_note: None,
            add_depends_on: vec![],
            remove_depends_on: vec![],
            allow_forward_ref: false,
            dry_run: false,
            json: false,
            no_color: true,
            project_root: None,
            verbosity: Verbosity::Normal,
        }
    }

    #[test]
    fn has_mutation_false_when_nothing_set() {
        assert!(!base_args().has_mutation());
    }

    #[test]
    fn has_mutation_true_for_title() {
        let mut args = base_args();
        args.title = Some("New".to_string());
        assert!(args.has_mutation());
    }

    #[test]
    fn has_mutation_true_for_add_label() {
        let mut args = base_args();
        args.add_label = vec!["urgent".to_string()];
        assert!(args.has_mutation());
    }

    #[test]
    fn has_mutation_true_for_remove_depends_on() {
        let mut args = base_args();
        args.remove_depends_on = vec!["other".to_string()];
        assert!(args.has_mutation());
    }
}
