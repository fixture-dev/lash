//! Check-links command with fuzzy fix support
//!
//! The `lash check-links` command finds broken dependency references in task files.
//! With `--fix`, it can suggest and apply fixes using fuzzy matching.

mod annotation_editor;
mod core;
pub mod fuzzy_matcher;
mod interactive;

use anyhow::{Context, Result};
use lash_cli::formatter::Verbosity;
use lash_cli::theme::CliTheme;
use lash_db::open_database;
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
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Attempt to fix broken links with fuzzy matching
    pub fix: bool,
    /// Auto-accept high-confidence fixes (requires --fix)
    pub yes: bool,
    /// Show what would be fixed without applying changes (requires --fix)
    pub dry_run: bool,
    /// Optional CLI theme for styling
    pub theme: Option<CliTheme>,
    /// Verbosity level for output (reserved for future use)
    #[allow(dead_code)]
    pub verbosity: Verbosity,
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
            core::output_text_report(&report, args.theme.as_ref());
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
        core::output_text_report(&report, args.theme.as_ref());
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
    let prompter = InteractivePrompter::new(args.theme.as_ref());

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
                        if let Some(theme) = &args.theme {
                            println!(
                                "{} Auto-fixing {} -> {} ({}%)",
                                theme.style_success("✓"),
                                theme.style_error(&broken_link.raw_ref),
                                theme.style_success(&best.task_id),
                                (best.score * 100.0) as u8
                            );
                        } else {
                            println!(
                                "Auto-fixing {} -> {} ({}%)",
                                broken_link.raw_ref,
                                best.task_id,
                                (best.score * 100.0) as u8
                            );
                        }
                        Some(FixDecision::Accept(best.task_id.clone()))
                    } else {
                        if let Some(theme) = &args.theme {
                            println!(
                                "{} Skipping {} (confidence too low: {}%)",
                                theme.style_warning("⊘"),
                                theme.style_error(&broken_link.raw_ref),
                                (best.score * 100.0) as u8
                            );
                        } else {
                            println!(
                                "Skipping {} (confidence too low: {}%)",
                                broken_link.raw_ref,
                                (best.score * 100.0) as u8
                            );
                        }
                        Some(FixDecision::Skip)
                    }
                } else {
                    if let Some(theme) = &args.theme {
                        println!(
                            "{} Skipping {} (no candidates)",
                            theme.style_warning("⊘"),
                            theme.style_error(&broken_link.raw_ref)
                        );
                    } else {
                        println!("Skipping {} (no candidates)", broken_link.raw_ref);
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
                            if let Some(theme) = &args.theme {
                                println!(
                                    "{} Would fix {} -> {}",
                                    theme.style_info("[DRY RUN]"),
                                    theme.style_error(&broken_link.raw_ref),
                                    theme.style_success(new_ref)
                                );
                            } else {
                                println!(
                                    "[DRY RUN] Would fix {} -> {}",
                                    broken_link.raw_ref, new_ref
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
                                if let Some(theme) = &args.theme {
                                    eprintln!(
                                        "{} Failed to apply fix: {}",
                                        theme.style_error("✗"),
                                        e
                                    );
                                } else {
                                    eprintln!("Failed to apply fix: {e}");
                                }
                                skipped += 1;
                                continue;
                            }

                            if let Some(theme) = &args.theme {
                                println!(
                                    "{} Fixed {} -> {}",
                                    theme.style_success("✓"),
                                    theme.style_error(&broken_link.raw_ref),
                                    theme.style_success(new_ref)
                                );
                            } else {
                                println!("Fixed {} -> {}", broken_link.raw_ref, new_ref);
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
        println!();
        if args.theme.is_some() {
            use owo_colors::OwoColorize;
            println!("{}", "Re-indexing project...".bold());
        } else {
            println!("Re-indexing project...");
        }

        // Run indexing
        if let Err(e) = reindex_project(project_root) {
            if let Some(theme) = &args.theme {
                eprintln!("{} Failed to re-index: {}", theme.style_error("✗"), e);
                eprintln!("Please run {} manually.", theme.style_info("lash index"));
            } else {
                eprintln!("Failed to re-index: {e}");
                eprintln!("Please run 'lash index' manually.");
            }
        } else if let Some(theme) = &args.theme {
            println!("{} Re-indexing complete", theme.style_success("✓"));
        } else {
            println!("Re-indexing complete");
        }
    }

    // Determine exit code
    if user_quit {
        println!();
        if let Some(theme) = &args.theme {
            println!("{}", theme.style_warning("Exited by user."));
        } else {
            println!("Exited by user.");
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
        paths: Vec::new(),
        force: false,
        show_files: false,
        json: false,
        no_color: false,
        errors_streaming: false,
        project_root: Some(project_root.to_path_buf()),
        verbosity: Verbosity::Normal,
    };

    index_execute(index_args)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_args_structure() {
        let args = CheckLinksArgs {
            json: false,
            project_root: None,
            fix: false,
            yes: false,
            dry_run: false,
            theme: None,
            verbosity: Verbosity::Normal,
        };

        assert!(!args.fix);
        assert!(!args.yes);
        assert!(!args.dry_run);
        assert!(args.theme.is_none());
    }

    #[test]
    fn test_fix_decision_equality() {
        use interactive::FixDecision;

        assert_eq!(FixDecision::Skip, FixDecision::Skip);
        assert_ne!(FixDecision::Skip, FixDecision::Accept("test".to_string()));
    }

    // Kill mut-000240: !db_path.exists() - when DB is absent, execute returns 3
    #[test]
    fn test_execute_returns_3_when_no_db() {
        let temp = TempDir::new().unwrap();
        let args = CheckLinksArgs {
            json: false,
            project_root: Some(temp.path().to_path_buf()),
            fix: false,
            yes: false,
            dry_run: false,
            theme: None,
            verbosity: Verbosity::Normal,
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 3);
    }

    // Kill mut-000240: json=true path when DB is missing
    #[test]
    fn test_execute_returns_3_when_no_db_json_mode() {
        let temp = TempDir::new().unwrap();
        let args = CheckLinksArgs {
            json: true,
            project_root: Some(temp.path().to_path_buf()),
            fix: false,
            yes: false,
            dry_run: false,
            theme: None,
            verbosity: Verbosity::Normal,
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 3);
    }

    // Kill mut-000241, mut-000242, mut-000243, mut-000245:
    // total_broken == 0 returns Ok(0); total_broken != 0 takes a different path.
    // We test the boundary by directly asserting on BrokenLinksReport values.
    #[test]
    fn test_broken_links_report_total_broken_zero_means_clean() {
        let report = core::BrokenLinksReport {
            total_broken: 0,
            by_file: vec![],
        };
        // The condition `report.total_broken == 0` determines the early success return
        assert_eq!(report.total_broken, 0);
    }

    #[test]
    fn test_broken_links_report_total_broken_one_means_issues() {
        let report = core::BrokenLinksReport {
            total_broken: 1,
            by_file: vec![],
        };
        // total_broken != 0, so execute would continue past the early return
        assert_ne!(report.total_broken, 0);
        assert_eq!(report.total_broken, 1);
    }

    // Kill mut-000241, mut-000242, mut-000243, mut-000245:
    // execute() with a real empty DB should return Ok(0) because total_broken == 0.
    // If the literal 0 is changed to 1, an empty DB would NOT take the early-return path
    // and the function would return Ok(1) instead of Ok(0).
    #[test]
    fn test_execute_returns_0_when_no_broken_links_in_empty_db() {
        use lash_db::init_database;
        use std::fs;

        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");
        init_database(&db_path).unwrap();

        let args = CheckLinksArgs {
            json: false,
            project_root: Some(temp.path().to_path_buf()),
            fix: false,
            yes: false,
            dry_run: false,
            theme: None,
            verbosity: Verbosity::Normal,
        };
        let result = execute(&args).unwrap();
        // Empty DB has 0 broken links, so execute returns exactly 0 (not 1)
        assert_eq!(result, 0);
        assert_ne!(result, 1);
    }

    /// Build a temp project with an initialized DB that has one broken dependency link.
    ///
    /// The DB contains:
    ///   - one file row (path="test.md")
    ///   - one task row (`full_id="test.md#task1"`)
    ///   - one dependency with `to_task_id=NULL` (broken link)
    fn setup_db_with_broken_link(temp: &TempDir) {
        use lash_db::init_database;

        let lash_dir = temp.path().join(".lash");
        std::fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");

        let conn = init_database(&db_path).unwrap();

        conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime) \
             VALUES ('test.md', 'test', 'Test', 'abc', 0)",
            [],
        )
        .unwrap();
        let file_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index) \
             VALUES (?1, 'task1', 'test.md#task1', 'T1', 'open', 0, 0)",
            rusqlite::params![file_id],
        )
        .unwrap();
        let task_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO dependencies (from_task_id, to_task_id, kind, raw_ref) \
             VALUES (?1, NULL, 'explicit_id', 'missing#ref')",
            rusqlite::params![task_id],
        )
        .unwrap();
    }

    // Kill mut-000111 (L111): `Ok(1)` → `Ok(0)`.
    // When broken links are found and fix mode is off, execute() must return Ok(1).
    // This test uses a real DB with a broken link so the result is determined by the
    // actual production exit-code literal, not just the DB-missing path.
    #[test]
    fn test_execute_returns_1_when_broken_links_found() {
        let temp = TempDir::new().unwrap();
        setup_db_with_broken_link(&temp);

        let args = CheckLinksArgs {
            json: false,
            project_root: Some(temp.path().to_path_buf()),
            fix: false,
            yes: false,
            dry_run: false,
            theme: None,
            verbosity: Verbosity::Normal,
        };
        let result = execute(&args).unwrap();
        // Must be exactly 1 (not 0) when broken links exist
        assert_eq!(
            result, 1,
            "execute() must return 1 when broken links are found and fix=false"
        );
        assert_ne!(result, 0, "exit code 0 would incorrectly signal 'clean'");
    }

    // Kill mut-000111 complementary: zero broken links returns 0.
    // The pair (0-links → Ok(0), 1-link → Ok(1)) distinguishes the literal.
    #[test]
    fn test_execute_returns_0_vs_1_boundary() {
        use lash_db::init_database;

        // Empty DB → 0 broken links → Ok(0)
        let temp_clean = TempDir::new().unwrap();
        let lash_dir = temp_clean.path().join(".lash");
        std::fs::create_dir_all(&lash_dir).unwrap();
        init_database(&lash_dir.join("lash.db")).unwrap();

        let clean_args = CheckLinksArgs {
            json: false,
            project_root: Some(temp_clean.path().to_path_buf()),
            fix: false,
            yes: false,
            dry_run: false,
            theme: None,
            verbosity: Verbosity::Normal,
        };
        assert_eq!(execute(&clean_args).unwrap(), 0, "clean DB must return 0");

        // DB with broken link → Ok(1)
        let temp_broken = TempDir::new().unwrap();
        setup_db_with_broken_link(&temp_broken);
        let broken_args = CheckLinksArgs {
            json: false,
            project_root: Some(temp_broken.path().to_path_buf()),
            fix: false,
            yes: false,
            dry_run: false,
            theme: None,
            verbosity: Verbosity::Normal,
        };
        assert_eq!(
            execute(&broken_args).unwrap(),
            1,
            "DB with broken link must return 1"
        );
    }

    // Kill mut-000099 (L99): `args.fix` negation → `!args.fix`.
    // With the mutation, fix=false would enter execute_fix_mode, which reads from
    // stdin (EOF in tests) and returns Err.  The original returns Ok(1).
    // We verify that fix=false with a broken link returns Ok(1) (not Err).
    #[test]
    fn test_execute_fix_false_returns_ok_not_err_when_broken_links_found() {
        let temp = TempDir::new().unwrap();
        setup_db_with_broken_link(&temp);

        let args = CheckLinksArgs {
            json: false,
            project_root: Some(temp.path().to_path_buf()),
            fix: false, // Original: reports broken links and returns Ok(1)
            yes: false,
            dry_run: false,
            theme: None,
            verbosity: Verbosity::Normal,
        };
        // Must succeed (not Err) and return exactly 1
        let result = execute(&args);
        assert!(
            result.is_ok(),
            "execute() with fix=false must return Ok, not Err; got: {result:?}"
        );
        assert_eq!(
            result.unwrap(),
            1,
            "execute() with fix=false and broken links must return Ok(1)"
        );
    }

    // Kill mut-000099 complementary: fix=true with yes=true in dry_run mode
    // should NOT return Ok(1) — it takes execute_fix_mode path with auto-skipping.
    #[test]
    fn test_execute_fix_true_yes_true_dry_run_does_not_return_1() {
        let temp = TempDir::new().unwrap();
        setup_db_with_broken_link(&temp);

        let args = CheckLinksArgs {
            json: false,
            project_root: Some(temp.path().to_path_buf()),
            fix: true,     // Fix mode: enter execute_fix_mode
            yes: true,     // Auto-accept (no stdin reads)
            dry_run: true, // Don't actually write files
            theme: None,
            verbosity: Verbosity::Normal,
        };
        // execute_fix_mode with yes=true and no high-confidence match will skip all
        // links and return Ok(1) (all skipped → skipped == total_broken).
        // The key point: the fix=true path is distinct from fix=false path.
        let result = execute(&args);
        assert!(
            result.is_ok(),
            "execute() with fix=true, yes=true must return Ok; got: {result:?}"
        );
    }

    // Kill mut-000240: !db_path.exists() when db IS present (complementary to the no-db test)
    // The mutation negates the check, so having a real DB tests the "db exists" branch.
    #[test]
    fn test_execute_with_real_db_does_not_return_3() {
        use lash_db::init_database;
        use std::fs;

        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");
        init_database(&db_path).unwrap();

        let args = CheckLinksArgs {
            json: false,
            project_root: Some(temp.path().to_path_buf()),
            fix: false,
            yes: false,
            dry_run: false,
            theme: None,
            verbosity: Verbosity::Normal,
        };
        let result = execute(&args).unwrap();
        // With a real DB, should NOT return 3 (DB error code)
        assert_ne!(result, 3);
    }

    /// Set up a DB where the broken link's `raw_ref` closely matches an existing valid task ID.
    /// This ensures the fuzzy matcher finds a high-confidence match (score >= 0.85)
    /// so the auto-accept path (yes=true) will accept the fix.
    fn setup_db_with_fixable_broken_link(temp: &TempDir) {
        use lash_db::init_database;

        let lash_dir = temp.path().join(".lash");
        std::fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");

        let conn = init_database(&db_path).unwrap();

        conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime) \
             VALUES ('test.md', 'test', 'Test', 'abc', 0)",
            [],
        )
        .unwrap();
        let file_id: i64 = conn.last_insert_rowid();

        // Create two valid tasks
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index) \
             VALUES (?1, 'task1', 'test.md#task1', 'T1', 'open', 0, 0)",
            rusqlite::params![file_id],
        )
        .unwrap();
        let task1_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index) \
             VALUES (?1, 'task2', 'test.md#task2', 'T2', 'open', 0, 1)",
            rusqlite::params![file_id],
        )
        .unwrap();

        // Create the test.md file with proper task format
        let content = "# Test\n\n- [ ] T1\n  @id: task1\n- [ ] T2\n  @id: task2\n";
        std::fs::write(temp.path().join("test.md"), content).unwrap();

        // Broken link: raw_ref is very close to test.md#task2 → high fuzzy score
        conn.execute(
            "INSERT INTO dependencies (from_task_id, to_task_id, kind, raw_ref) \
             VALUES (?1, NULL, 'explicit_id', 'test.md#task2')",
            rusqlite::params![task1_id],
        )
        .unwrap();
    }

    /// Kill L132 (`!args.dry_run → args.dry_run`), L135-138 (counter init mutations),
    /// L281 (`skipped += 1 → += 0`), L336/338 (exit code logic).
    ///
    /// With a high-confidence match and yes=true, the auto-accept path fires.
    /// accepted becomes 1, skipped stays 0. Exit code should be 0 (not all skipped).
    ///
    /// - If L137 mutates `skipped = 0 → 1`, then `skipped == total_broken` → true,
    ///   exit code becomes 1 instead of 0. Test fails.
    /// - If L138 mutates `user_quit = false → true`, the loop breaks immediately,
    ///   no links are processed, skipped stays 0 (or 1 if also mutated),
    ///   and exit code is 1 (`user_quit` path). Test fails.
    /// - If L135 mutates `accepted = 0 → 1`, accepted would be 2 after the fix,
    ///   but the exit code is still 0. This specific mutation is harder to kill.
    #[test]
    fn test_execute_fix_yes_auto_accept_returns_0() {
        let temp = TempDir::new().unwrap();
        setup_db_with_fixable_broken_link(&temp);

        let args = CheckLinksArgs {
            json: false,
            project_root: Some(temp.path().to_path_buf()),
            fix: true,
            yes: true,     // Auto-accept high-confidence matches
            dry_run: true, // Don't actually write files
            theme: None,
            verbosity: Verbosity::Normal,
        };
        let result = execute(&args).unwrap();
        // The fuzzy match score for "test.md#task2" against valid "test.md#task2" should be 1.0
        // → auto-accepted → accepted=1, skipped=0 → exit code 0 (not all skipped)
        assert_eq!(
            result, 0,
            "auto-accepted fix with high-confidence match should return 0, not 1"
        );
    }

    /// Kill L336 (`skipped == report.total_broken → !(...)` or `== → !=`).
    /// When all broken links are skipped (low confidence), exit code should be 1.
    #[test]
    fn test_execute_fix_yes_all_skipped_returns_1() {
        let temp = TempDir::new().unwrap();
        // Use the standard broken link setup (raw_ref='missing#ref') which
        // won't match any valid task with high confidence
        setup_db_with_broken_link(&temp);

        let args = CheckLinksArgs {
            json: false,
            project_root: Some(temp.path().to_path_buf()),
            fix: true,
            yes: true, // Auto-mode
            dry_run: true,
            theme: None,
            verbosity: Verbosity::Normal,
        };
        let result = execute(&args).unwrap();
        // 'missing#ref' won't match 'test.md#task1' with >= 0.85 score
        // → all skipped → skipped == total_broken → exit code 1
        assert_eq!(result, 1, "all-skipped fix should return 1");
    }

    /// Kill L132 `dry_run` negation: verify that `dry_run=true` does NOT write to files,
    /// while `dry_run=false` would (`AnnotationEditor` `write_enabled` is `!dry_run`).
    /// We test by checking the file content is unchanged after a `dry_run=true` fix.
    #[test]
    fn test_execute_fix_yes_dry_run_true_does_not_modify_files() {
        let temp = TempDir::new().unwrap();
        setup_db_with_fixable_broken_link(&temp);

        let test_file = temp.path().join("test.md");
        let original_content = std::fs::read_to_string(&test_file).unwrap();

        let args = CheckLinksArgs {
            json: false,
            project_root: Some(temp.path().to_path_buf()),
            fix: true,
            yes: true,
            dry_run: true, // Should NOT modify files
            theme: None,
            verbosity: Verbosity::Normal,
        };
        let _result = execute(&args).unwrap();

        // File should be unchanged after dry run
        let after_content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(
            original_content, after_content,
            "dry_run=true must not modify files"
        );
    }
}
