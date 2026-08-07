//! `lash show <task>` — task-detail rendering (text and JSON).
//!
//! Split out of `show/mod.rs` (alongside `file_view.rs`/`format.rs`) to keep
//! that file under the project's ~500-line guideline. This is where GitHub
//! issue #26 lives: the full task view now includes the `@agent-note`,
//! resolved `@depends-on` status, and a direct-children summary (via
//! `detail.rs`), in addition to the fields `show` already printed.

use anyhow::Result;
use lash::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash::formatter::{OutputFormat, Verbosity};
use lash::theme::CliTheme;
use lash_db::repository::files::FileRecord;
use lash_db::repository::tasks::TaskRecord;
use lash_db::{DependencyRepository, DocRefRepository, FileRepository, TaskRepository};
use lash_types::error::LashError;
use std::path::{Path, PathBuf};

use super::detail;
use super::format::{format_task_status, format_task_status_icon};
use super::ShowArgs;
use crate::utils::project_loader::load_project;

/// Show detailed information about a task
#[allow(clippy::too_many_lines)]
pub(super) fn show_task(
    task_repo: &TaskRepository,
    file_repo: &FileRepository,
    dep_repo: &DependencyRepository,
    doc_repo: &DocRefRepository,
    args: &ShowArgs,
    project_root: &Path,
    theme: Option<&CliTheme>,
) -> Result<i32> {
    // Get task record by full id or bare @id (GitHub issue #14).
    let task = match crate::utils::task_target::resolve_task_target(task_repo, &args.target) {
        Ok(task) => task,
        Err(crate::utils::task_target::TargetError::Db(e)) => {
            let error = LashError::internal(
                format!("Database query failed: {e}"),
                Some("get_by_full_id".to_string()),
            );
            if args.json {
                super::output_json_error(&error)?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormat::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: theme.cloned(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_error(&error);
            }
            return Ok(3); // Exit code 3 for DB error
        }
        Err(target_err) => {
            // Not found or ambiguous @id. Report as a not-found diagnostic
            // (exit 5) — never E_INTERNAL, which is miscategorized for a
            // missing task and matters to scripts branching on error class.
            let ambiguous = matches!(
                target_err,
                crate::utils::task_target::TargetError::Ambiguous(_)
            );
            // Try fuzzy matching to suggest similar task IDs
            let all_task_ids = task_repo.get_all_full_ids().unwrap_or_default();
            let suggestions =
                if let crate::utils::task_target::TargetError::Ambiguous(cands) = &target_err {
                    cands.iter().map(|c| (c.clone(), 1.0)).collect()
                } else {
                    super::find_similar_task_ids(&args.target, &all_task_ids)
                };

            let error = LashError::query_no_results(&args.target);
            let mut diag = error.to_diagnostic();
            diag.message = if ambiguous {
                format!("Task @id '{}' is ambiguous", args.target)
            } else {
                format!("Task not found: {}", args.target)
            };

            // Build help message with suggestions if available
            let help_msg = if let Some((best_match, _score)) = suggestions.first() {
                if suggestions.len() == 1 {
                    format!(
                        "Did you mean '{best_match}'?\n\nMake sure the task exists and has been indexed with `lash index`"
                    )
                } else {
                    let matches: Vec<_> = suggestions
                        .iter()
                        .take(3)
                        .map(|(id, _)| format!("  - {id}"))
                        .collect();
                    format!(
                        "Did you mean one of these?\n{}\n\nMake sure the task exists and has been indexed with `lash index`",
                        matches.join("\n")
                    )
                }
            } else {
                "Make sure the task exists and has been indexed with `lash index`".to_string()
            };
            diag.help = Some(help_msg.clone());

            if args.json {
                super::output_json_diagnostic(&diag, &suggestions)?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormat::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: theme.cloned(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_diagnostic(&diag);

                // Always print suggestions in text mode, regardless of verbosity
                if !suggestions.is_empty() && args.verbosity < Verbosity::Verbose {
                    eprintln!();
                    eprintln!("  help: {help_msg}");
                }
            }
            return Ok(5); // Exit code 5 for not found
        }
    };

    // Get file information from the database
    let file = file_repo
        .get_by_db_id(task.file_id)?
        .unwrap_or_else(|| FileRecord {
            id: task.file_id,
            path: PathBuf::from(format!("<file-id-{}>", task.file_id)),
            file_id: format!("file-{}", task.file_id),
            title: String::from("<unknown>"),
            description: String::new(),
            hash: String::new(),
            mtime: 0,
            status: lash_types::FileStatus::InProgress,
            metadata: lash_types::FileMetadata::default(),
            indexed_at: 0,
        });

    // Get dependencies if requested
    let dependencies = if args.deps {
        let dep_records = dep_repo.get_dependencies(task.id)?;
        let mut deps = Vec::new();
        for dep_record in dep_records {
            // Only include dependencies that have been resolved
            if let Some(to_task_id) = dep_record.to_task_id {
                match task_repo.get_by_db_id(to_task_id)? {
                    Some(task_record) => deps.push(task_record),
                    None => {
                        tracing::warn!(
                            "Failed to resolve dependency: task DB ID {} not found",
                            to_task_id
                        );
                    }
                }
            }
        }
        Some(deps)
    } else {
        None
    };

    // Get reverse dependencies if requested
    let dependents = if args.rdeps {
        let dep_records = dep_repo.get_dependents(task.id)?;
        let mut deps = Vec::new();
        for dep_record in dep_records {
            match task_repo.get_by_db_id(dep_record.from_task_id)? {
                Some(task_record) => deps.push(task_record),
                None => {
                    tracing::warn!(
                        "Failed to resolve dependent: task DB ID {} not found",
                        dep_record.from_task_id
                    );
                }
            }
        }
        Some(deps)
    } else {
        None
    };

    // Get task-level doc references
    let doc_refs = doc_repo.find_by_task(task.id)?;

    // Resolve `@depends-on` status and direct children for the full (non
    // `--short`) view (GitHub issue #26). Skipped entirely under `--short`
    // and when the task has no dependencies, since both cases would
    // otherwise reparse the whole project for nothing.
    let (dep_statuses, children) = if args.short {
        (Vec::new(), Vec::new())
    } else {
        let dep_statuses = if task.metadata.depends_on.is_empty() {
            Vec::new()
        } else {
            let (config, project) = load_project(project_root);
            detail::resolve_dependencies(&config, &project, &file.path, &file.file_id, &task)
        };
        let children = detail::children_summary(task_repo, task.id)?;
        (dep_statuses, children)
    };

    // Output results
    if args.json {
        output_json_task(
            &task,
            &file,
            dependencies,
            dependents,
            &doc_refs,
            &dep_statuses,
            &children,
            args.short,
        )?;
    } else {
        output_text_task(
            &task,
            &file,
            dependencies.as_ref(),
            dependents.as_ref(),
            &doc_refs,
            &dep_statuses,
            &children,
            args.short,
            theme,
        );
    }

    Ok(0)
}

/// Output task as JSON
#[allow(clippy::too_many_arguments)]
fn output_json_task(
    task: &TaskRecord,
    file: &FileRecord,
    dependencies: Option<Vec<TaskRecord>>,
    dependents: Option<Vec<TaskRecord>>,
    doc_refs: &[lash_db::repository::DocRefRow],
    dep_statuses: &[detail::DependencyStatus],
    children: &[detail::ChildSummary],
    short: bool,
) -> Result<()> {
    use serde_json::json;

    // `--short` mirrors the terse text view: just enough to identify the
    // task, none of the operational payload (GitHub issue #26).
    if short {
        let output = json!({
            "type": "task",
            "id": task.full_id,
            "title": task.title,
            "status": task.status,
            "file": file.path,
            "labels": task.metadata.labels,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let mut output = json!({
        "type": "task",
        "task": task,
        "file": {
            "path": file.path,
            "title": file.title,
        },
        "doc_refs": doc_refs,
    });

    // Flat convenience fields alongside the nested `task` object, so an
    // agent doesn't need to know the record's internal shape just to find
    // the operational note or check whether it's unblocked.
    if let Some(note) = &task.metadata.agent_note {
        output["agent_note"] = json!(note);
    }

    let (satisfied, dep_total) = detail::dependency_counts(dep_statuses);
    output["depends_on"] = json!({
        "items": dep_statuses,
        "satisfied": satisfied,
        "total": dep_total,
    });

    let (done, child_total) = detail::children_counts(children);
    output["children"] = json!({
        "items": children,
        "done": done,
        "total": child_total,
    });

    if let Some(deps) = dependencies {
        output["dependencies"] = json!(deps);
    }

    if let Some(deps) = dependents {
        output["dependents"] = json!(deps);
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output task as human-readable text
#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
fn output_text_task(
    task: &TaskRecord,
    file: &FileRecord,
    dependencies: Option<&Vec<TaskRecord>>,
    dependents: Option<&Vec<TaskRecord>>,
    doc_refs: &[lash_db::repository::DocRefRow],
    dep_statuses: &[detail::DependencyStatus],
    children: &[detail::ChildSummary],
    short: bool,
    theme: Option<&CliTheme>,
) {
    // Task header
    if let Some(theme) = theme {
        println!("{}", theme.style_info("Task:"));
        println!("  ID:       {}", theme.style_label(&task.full_id));
        println!("  Title:    {}", theme.style_info(&task.title));
        println!(
            "  Status:   {}",
            format_task_status(task.status, Some(theme))
        );
        println!(
            "  File:     {}",
            theme.style_muted(&file.path.display().to_string())
        );
    } else {
        println!("Task:");
        println!("  ID:       {}", task.full_id);
        println!("  Title:    {}", task.title);
        println!("  Status:   {}", format_task_status(task.status, None));
        println!("  File:     {}", file.path.display());
    }

    // `--short` stops here: ID/Title/Status/File plus Labels below, and
    // nothing else — this is the terse view scripts can depend on
    // (GitHub issue #26).
    if short {
        print_labels(task, theme);
        return;
    }

    // Optional fields
    if let Some(ref owner) = task.owner {
        if let Some(theme) = theme {
            println!("  Owner:    {}", theme.style_info(owner));
        } else {
            println!("  Owner:    {owner}");
        }
    }
    if let Some(ref estimate) = task.estimate {
        if let Some(theme) = theme {
            println!("  Estimate: {}", theme.style_info(estimate));
        } else {
            println!("  Estimate: {estimate}");
        }
    }
    detail::render_custom_metadata(&task.metadata.custom, theme);

    print_labels(task, theme);

    // Doc references
    if !doc_refs.is_empty() {
        let doc_str = doc_refs
            .iter()
            .map(|d| {
                if let Some(ref frag) = d.fragment {
                    format!("{}#{}", d.target_path, frag)
                } else {
                    d.target_path.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        if let Some(theme) = theme {
            println!("  Docs:     {}", theme.style_muted(&doc_str));
        } else {
            println!("  Docs:     {doc_str}");
        }
    }

    // Body
    if let Some(ref body) = task.body {
        println!();
        if let Some(theme) = theme {
            println!("{}", theme.style_info("Description:"));
        } else {
            println!("Description:");
        }
        for line in body.lines() {
            println!("  {line}");
        }
    }

    // Contextual Notes
    if !task.contextual_notes.is_empty() {
        println!();
        if let Some(theme) = theme {
            println!("{}", theme.style_info("Notes:"));
        } else {
            println!("Notes:");
        }
        for note in &task.contextual_notes {
            if let Some(theme) = theme {
                println!(
                    "  {} {}",
                    theme.style_muted("·"),
                    theme.style_muted(note.text())
                );
            } else {
                println!("  · {}", note.text());
            }
        }
    }

    detail::render_agent_note(task.metadata.agent_note.as_deref(), theme);
    detail::render_dependencies(dep_statuses, theme);
    detail::render_children(children, theme);

    // Dependencies
    if let Some(deps) = dependencies {
        println!();
        if let Some(theme) = theme {
            println!(
                "{}",
                theme.style_info(&format!("Dependencies ({}):", deps.len()))
            );
        } else {
            println!("Dependencies ({}):", deps.len());
        }

        if deps.is_empty() {
            println!("  (none)");
        } else {
            for dep in deps {
                if let Some(theme) = theme {
                    let checkbox = theme.styled_checkbox(dep.status);
                    let full_id = theme.style_muted(&dep.full_id);
                    println!("  {} {} ({})", checkbox, dep.title, full_id);
                } else {
                    println!(
                        "  {} {} ({})",
                        format_task_status_icon(dep.status),
                        dep.title,
                        dep.full_id
                    );
                }
            }
        }
    }

    // Dependents
    if let Some(deps) = dependents {
        println!();
        if let Some(theme) = theme {
            println!(
                "{}",
                theme.style_info(&format!("Depended on by ({}):", deps.len()))
            );
        } else {
            println!("Depended on by ({}):", deps.len());
        }

        if deps.is_empty() {
            println!("  (none)");
        } else {
            for dep in deps {
                if let Some(theme) = theme {
                    let checkbox = theme.styled_checkbox(dep.status);
                    let full_id = theme.style_muted(&dep.full_id);
                    println!("  {} {} ({})", checkbox, dep.title, full_id);
                } else {
                    println!(
                        "  {} {} ({})",
                        format_task_status_icon(dep.status),
                        dep.title,
                        dep.full_id
                    );
                }
            }
        }
    }
}

/// Print the `Labels:` line, if the task has any. Shared by the `--short`
/// and full text views so they render labels identically.
fn print_labels(task: &TaskRecord, theme: Option<&CliTheme>) {
    if task.metadata.labels.is_empty() {
        return;
    }
    if let Some(theme) = theme {
        let labels = task
            .metadata
            .labels
            .iter()
            .map(|l| theme.style_label(l))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  Labels:   {labels}");
    } else {
        println!("  Labels:   {}", task.metadata.labels.join(", "));
    }
}
