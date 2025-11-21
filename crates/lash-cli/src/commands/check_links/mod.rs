//! Check-links command with fuzzy fix support
//!
//! The `lash check-links` command finds broken dependency references in task files.
//! With `--fix`, it can suggest and apply fixes using fuzzy matching.

mod annotation_editor;
mod core;
pub mod fuzzy_matcher;
mod interactive;

use anyhow::{Context, Result};
use lash_db::open_database;
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

// Re-export core types
use annotation_editor::AnnotationEditor;
pub use core::BrokenLinksReport;
use fuzzy_matcher::FuzzyMatcher;
use interactive::{FixDecision, InteractivePrompter};

/// Arguments for the check-links command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // CLI args naturally contain many boolean flags
pub struct CheckLinksArgs {
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Attempt to fix broken links with fuzzy matching
    pub fix: bool,
    /// Auto-accept high-confidence fixes (requires --fix)
    pub yes: bool,
    /// Show what would be fixed without applying changes (requires --fix)
    pub dry_run: bool,
}

/// Execute the check-links command
///
/// # Arguments
///
/// * `args` - Check-links command arguments
///
/// # Returns
///
/// Exit code: 0 (no broken links), 1 (broken links found), 3 (DB error)
pub fn execute(args: &CheckLinksArgs) -> Result<i32> {
    // Determine project root
    let project_root = if let Some(ref root) = args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        fix_mode = args.fix,
        "Starting check-links operation"
    );

    // Determine database path
    let db_path = core::get_database_path(&project_root);

    // Check if database exists
    if !db_path.exists() {
        if args.json {
            core::output_json_no_db()?;
        } else {
            eprintln!("Database not found at {}", db_path.display());
            eprintln!("Run `lash index` to create the database.");
        }
        return Ok(3); // Exit code 3 for DB error
    }

    // Find all broken links
    let report = core::find_broken_links(&db_path).context("Failed to find broken links")?;

    // If no broken links, report success and exit
    if report.total_broken == 0 {
        if args.json {
            core::output_json_report(&report)?;
        } else {
            core::output_text_report(&report, args.no_color);
        }
        tracing::info!("No broken links found");
        return Ok(0);
    }

    // If fix mode is enabled, attempt to fix broken links
    if args.fix {
        return execute_fix_mode(args, &project_root, &db_path, &report);
    }

    // Otherwise, just report the broken links
    if args.json {
        core::output_json_report(&report)?;
    } else {
        core::output_text_report(&report, args.no_color);
    }

    tracing::warn!(broken_count = report.total_broken, "Found broken links");
    Ok(1) // Exit code 1 for broken links found
}

/// Execute fix mode: find candidates and apply fixes
#[allow(clippy::too_many_lines)] // Complex workflow requires many lines
fn execute_fix_mode(
    args: &CheckLinksArgs,
    project_root: &Path,
    db_path: &Path,
    report: &BrokenLinksReport,
) -> Result<i32> {
    // Get all valid task IDs from the database
    let valid_task_ids = get_all_valid_task_ids(db_path)?;

    // Initialize fuzzy matcher
    let matcher = FuzzyMatcher::default();

    // Initialize interactive prompter (for interactive mode)
    let prompter = InteractivePrompter::new(args.no_color);

    // Initialize annotation editor
    let editor = AnnotationEditor::new(project_root.to_path_buf(), !args.dry_run);

    // Track statistics
    let mut accepted = 0;
    let mut manual = 0;
    let mut skipped = 0;
    let mut user_quit = false;

    // Process each broken link
    for file_links in &report.by_file {
        for broken_link in &file_links.links {
            // Find fuzzy match candidates
            let candidates = matcher.find_matches(&broken_link.raw_ref, &valid_task_ids);

            // Determine what to do
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let decision = if args.yes {
                // Auto-accept mode: only apply high-confidence fixes
                if let Some(best) = candidates.first() {
                    if best.score >= 0.85 {
                        if args.no_color {
                            println!(
                                "Auto-fixing {} -> {} ({}%)",
                                broken_link.raw_ref,
                                best.task_id,
                                (best.score * 100.0) as u8
                            );
                        } else {
                            println!(
                                "{} Auto-fixing {} -> {} ({}%)",
                                "✓".green(),
                                broken_link.raw_ref.red(),
                                best.task_id.green(),
                                (best.score * 100.0) as u8
                            );
                        }
                        Some(FixDecision::Accept(best.task_id.clone()))
                    } else {
                        if args.no_color {
                            println!(
                                "Skipping {} (confidence too low: {}%)",
                                broken_link.raw_ref,
                                (best.score * 100.0) as u8
                            );
                        } else {
                            println!(
                                "{} Skipping {} (confidence too low: {}%)",
                                "⊘".yellow(),
                                broken_link.raw_ref.red(),
                                (best.score * 100.0) as u8
                            );
                        }
                        Some(FixDecision::Skip)
                    }
                } else {
                    if args.no_color {
                        println!("Skipping {} (no candidates)", broken_link.raw_ref);
                    } else {
                        println!(
                            "{} Skipping {} (no candidates)",
                            "⊘".yellow(),
                            broken_link.raw_ref.red()
                        );
                    }
                    Some(FixDecision::Skip)
                }
            } else {
                // Interactive mode: prompt user
                if let Some(decision) = prompter.prompt_fix(
                    &file_links.file_path,
                    &broken_link.from_task_full_id,
                    &broken_link.raw_ref,
                    &candidates,
                )? {
                    Some(decision)
                } else {
                    // User quit
                    user_quit = true;
                    None
                }
            };

            // Apply the decision
            if let Some(decision) = decision {
                match &decision {
                    FixDecision::Accept(new_ref) | FixDecision::Manual(new_ref) => {
                        if args.dry_run {
                            if args.no_color {
                                println!(
                                    "[DRY RUN] Would fix {} -> {}",
                                    broken_link.raw_ref, new_ref
                                );
                            } else {
                                println!(
                                    "{} Would fix {} -> {}",
                                    "[DRY RUN]".cyan().bold(),
                                    broken_link.raw_ref.red(),
                                    new_ref.green()
                                );
                            }
                        } else {
                            // Extract the local task ID (part after the # in full_id)
                            let task_local_id = broken_link
                                .from_task_full_id
                                .split('#')
                                .nth(1)
                                .unwrap_or(&broken_link.from_task_full_id);

                            // Apply the fix
                            let file_path = project_root.join(&file_links.file_path);
                            if let Err(e) = editor.update_annotation(
                                &file_path,
                                task_local_id,
                                &broken_link.raw_ref,
                                new_ref,
                            ) {
                                if args.no_color {
                                    eprintln!("Failed to apply fix: {e}");
                                } else {
                                    eprintln!("{} Failed to apply fix: {}", "✗".red().bold(), e);
                                }
                                skipped += 1;
                                continue;
                            }

                            if args.no_color {
                                println!("Fixed {} -> {}", broken_link.raw_ref, new_ref);
                            } else {
                                println!(
                                    "{} Fixed {} -> {}",
                                    "✓".green().bold(),
                                    broken_link.raw_ref.red(),
                                    new_ref.green()
                                );
                            }
                        }

                        // Update statistics
                        match decision {
                            FixDecision::Accept(_) => accepted += 1,
                            FixDecision::Manual(_) => manual += 1,
                            FixDecision::Skip => {}
                        }
                    }
                    FixDecision::Skip => {
                        skipped += 1;
                    }
                }
            }

            if user_quit {
                break;
            }
        }

        if user_quit {
            break;
        }
    }

    // Show summary
    if !args.yes && !user_quit {
        prompter.show_summary(accepted, skipped, manual);
    }

    // Re-index if we made changes (and not in dry-run mode)
    if !args.dry_run && (accepted > 0 || manual > 0) {
        if args.no_color {
            println!();
            println!("Re-indexing project...");
        } else {
            println!();
            println!("{}", "Re-indexing project...".bold());
        }

        // Run indexing
        if let Err(e) = reindex_project(project_root) {
            if args.no_color {
                eprintln!("Failed to re-index: {e}");
                eprintln!("Please run 'lash index' manually.");
            } else {
                eprintln!("{} Failed to re-index: {}", "✗".red().bold(), e);
                eprintln!("Please run {} manually.", "lash index".cyan());
            }
        } else if args.no_color {
            println!("Re-indexing complete");
        } else {
            println!("{} Re-indexing complete", "✓".green().bold());
        }
    }

    // Determine exit code
    if user_quit {
        if args.no_color {
            println!();
            println!("Exited by user.");
        } else {
            println!();
            println!("{}", "Exited by user.".yellow());
        }
        Ok(1)
    } else if skipped == report.total_broken {
        // All were skipped
        Ok(1)
    } else {
        // Some were fixed
        Ok(0)
    }
}

/// Get all valid task IDs from the database
fn get_all_valid_task_ids(db_path: &Path) -> Result<Vec<String>> {
    let conn = open_database(db_path).context("Failed to open database")?;

    let mut stmt = conn.prepare("SELECT full_id FROM tasks ORDER BY full_id")?;

    let task_ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(task_ids)
}

/// Re-index the project after applying fixes
fn reindex_project(project_root: &Path) -> Result<()> {
    use crate::commands::index::{execute as index_execute, IndexArgs};

    let index_args = IndexArgs {
        force: false,
        show_files: false,
        json: false,
        no_color: false,
        project_root: Some(project_root.to_path_buf()),
    };

    index_execute(index_args)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_structure() {
        let args = CheckLinksArgs {
            json: false,
            no_color: false,
            project_root: None,
            fix: false,
            yes: false,
            dry_run: false,
        };

        assert!(!args.fix);
        assert!(!args.yes);
        assert!(!args.dry_run);
    }

    #[test]
    fn test_fix_decision_equality() {
        use interactive::FixDecision;

        assert_eq!(FixDecision::Skip, FixDecision::Skip);
        assert_ne!(FixDecision::Skip, FixDecision::Accept("test".to_string()));
    }
}
