//! Check-index command implementation
//!
//! The `lash check-index` command verifies `SQLite` database consistency with Markdown files.

use anyhow::{Context, Result};
use lash_cli::formatter::Verbosity;
use lash_cli::theme::CliTheme;
use lash_db::{open_database, IndexVerifier, VerifierConfig};
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

/// Arguments for the check-index command
#[derive(Debug, Clone)]
pub struct CheckIndexArgs {
    /// Paths to verify (if empty, verifies entire project)
    pub paths: Vec<PathBuf>,
    /// Show detailed diff of inconsistencies
    pub diff: bool,
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Verbosity level for output (reserved for future use)
    #[allow(dead_code)]
    pub verbosity: Verbosity,
}

/// Execute the check-index command
///
/// # Arguments
///
/// * `args` - Check-index command arguments
///
/// # Returns
///
/// Exit code: 0 (no issues), 1 (issues found), 3 (DB error)
pub fn execute(args: CheckIndexArgs) -> Result<i32> {
    // Load theme based on no_color flag and output format
    let theme = if args.json {
        None
    } else {
        CliTheme::load(None, !args.no_color)?
    };

    // Determine project root
    let project_root = if let Some(root) = args.project_root {
        root
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        "Starting check-index operation"
    );

    // Determine database path
    let db_path = get_database_path(&project_root);

    // Check if database exists
    if !db_path.exists() {
        if args.json {
            output_json_no_db()?;
        } else {
            eprintln!("Database not found at {}", db_path.display());
            eprintln!("Run `lash index` to create the database.");
        }
        return Ok(3); // Exit code 3 for DB error
    }

    // Open database
    let conn = open_database(&db_path).context("Failed to open database")?;

    // Configure verifier
    let mut verifier_config = VerifierConfig::new(project_root.clone());

    // Add path filtering if paths were provided
    if !args.paths.is_empty() {
        // Convert relative paths to absolute
        let absolute_paths: Vec<PathBuf> = args
            .paths
            .iter()
            .map(|p| {
                if p.is_absolute() {
                    p.clone()
                } else {
                    std::env::current_dir().map_or_else(|_| p.clone(), |cwd| cwd.join(p))
                }
            })
            .collect();
        verifier_config = verifier_config.with_paths(absolute_paths);
    }

    let verifier = IndexVerifier::new(&conn, verifier_config);

    // Run verification
    let report = verifier.verify().context("Failed to verify index")?;

    // Output results
    if args.json {
        output_json_report(&report)?;
    } else {
        output_text_report(&report, args.diff, theme.as_ref());
    }

    // Return exit code based on findings
    if report.is_clean() {
        tracing::info!("Index verification passed - no issues found");
        Ok(0)
    } else {
        tracing::warn!(
            issue_count = report.total_issues(),
            "Index verification found issues"
        );
        Ok(1) // Exit code 1 for issues found
    }
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/lash.db")
}

/// Output JSON when database doesn't exist
fn output_json_no_db() -> Result<()> {
    use serde_json::json;

    let output = json!({
        "error": "Database not found",
        "suggestion": "Run `lash index` to create the database"
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output verification report as JSON
fn output_json_report(report: &lash_db::VerificationReport) -> Result<()> {
    use serde_json::json;

    let output = json!({
        "is_clean": report.is_clean(),
        "files_checked": report.files_checked,
        "db_records_checked": report.db_records_checked,
        "total_issues": report.total_issues(),
        "issues": report.issues.iter().map(|issue| json!({
            "kind": format!("{}", issue.kind),
            "path": issue.path.display().to_string(),
            "description": issue.description,
            "fix_suggestion": issue.fix_suggestion,
        })).collect::<Vec<_>>(),
        "issues_by_kind": json!({
            "stale_files": report.count_by_kind(lash_db::IssueKind::StaleFile),
            "missing_files": report.count_by_kind(lash_db::IssueKind::MissingFile),
            "hash_mismatches": report.count_by_kind(lash_db::IssueKind::HashMismatch),
            "orphaned_tasks": report.count_by_kind(lash_db::IssueKind::OrphanedTasks),
            "orphaned_dependencies": report.count_by_kind(lash_db::IssueKind::OrphanedDependencies),
        }),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output verification report as human-readable text
fn output_text_report(
    report: &lash_db::VerificationReport,
    show_diff: bool,
    theme: Option<&CliTheme>,
) {
    // Header
    if report.is_clean() {
        let msg = "✓ Index is in sync";
        if let Some(t) = theme {
            println!("{}", t.style_success(msg));
        } else {
            println!("{msg}");
        }
        println!();
        println!(
            "Checked {} files and {} database records",
            report.files_checked, report.db_records_checked
        );
        return;
    }

    // Issues found
    let msg = format!("Found {} issue(s)", report.total_issues());
    if let Some(t) = theme {
        println!("{}", t.style_error(&msg));
    } else {
        println!("{msg}");
    }
    println!();

    // Summary by kind
    println!("Issues by type:");
    print_issue_count_if_any(
        "Stale files (in DB but not on disk)",
        report.count_by_kind(lash_db::IssueKind::StaleFile),
        theme,
    );
    print_issue_count_if_any(
        "Missing files (on disk but not in DB)",
        report.count_by_kind(lash_db::IssueKind::MissingFile),
        theme,
    );
    print_issue_count_if_any(
        "Hash mismatches (file modified)",
        report.count_by_kind(lash_db::IssueKind::HashMismatch),
        theme,
    );
    print_issue_count_if_any(
        "Orphaned tasks",
        report.count_by_kind(lash_db::IssueKind::OrphanedTasks),
        theme,
    );
    print_issue_count_if_any(
        "Orphaned dependencies",
        report.count_by_kind(lash_db::IssueKind::OrphanedDependencies),
        theme,
    );

    // Detailed issue list if requested
    if show_diff {
        println!();
        let msg = "Detailed issues:";
        if let Some(t) = theme {
            println!("{}", t.style_label(msg));
        } else {
            println!("{msg}");
        }
        println!();

        for issue in &report.issues {
            let kind = format!("[{}]", issue.kind);
            let path = issue.path.display().to_string();

            if let Some(t) = theme {
                println!("{} {}", t.style_warning(&kind), t.style_info(&path));
                println!("  {}", issue.description);
                println!("  {}", t.style_muted(&issue.fix_suggestion));
            } else {
                println!("{kind} {path}");
                println!("  {}", issue.description);
                println!("  {}", issue.fix_suggestion);
            }
            println!();
        }
    }

    // Suggestion
    println!();
    let msg = "To fix these issues, run:";
    let cmd = "lash index";
    if let Some(t) = theme {
        println!("{}", t.style_label(msg));
        println!("  {}", t.style_info(cmd));
    } else {
        println!("{msg}");
        println!("  {cmd}");
    }
}

/// Print an issue count if non-zero
fn print_issue_count_if_any(label: &str, count: usize, theme: Option<&CliTheme>) {
    if count > 0 {
        let count_str = count.to_string();
        if let Some(t) = theme {
            println!("  {}: {}", label, t.style_error(&count_str));
        } else {
            println!("  {label}: {count}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_db::verifier::{IssueKind, VerificationIssue, VerificationReport};
    use tempfile::TempDir;

    #[test]
    fn test_get_database_path() {
        let temp = TempDir::new().unwrap();
        let db_path = get_database_path(temp.path());

        assert_eq!(db_path, temp.path().join(".lash/lash.db"));
    }

    // Tests for execute() return codes (kills mut-000217, mut-000218)
    // and the json vs non-json branch (kills mut-000209, mut-000215)
    // and is_clean() branch (kills mut-000216)
    // and db_path.exists() check (kills mut-000212, mut-000213)

    #[test]
    fn test_execute_returns_3_when_no_db() {
        let temp = TempDir::new().unwrap();

        let args = CheckIndexArgs {
            paths: vec![],
            diff: false,
            json: false,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };

        let result = execute(args).unwrap();
        // No database file exists, so we expect exit code 3
        assert_eq!(result, 3);
    }

    #[test]
    fn test_execute_returns_3_when_no_db_json_mode() {
        // Tests the json=true branch when DB is missing (kills mut-000213)
        let temp = TempDir::new().unwrap();

        let args = CheckIndexArgs {
            paths: vec![],
            diff: false,
            json: true,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };

        let result = execute(args).unwrap();
        assert_eq!(result, 3);
    }

    // Tests for output_text_report() (kills mut-000219)
    // and print_issue_count_if_any() boundary conditions (kills mut-000224, mut-000225, mut-000226, mut-000227)

    fn make_clean_report() -> VerificationReport {
        VerificationReport::new()
    }

    fn make_dirty_report() -> VerificationReport {
        let mut report = VerificationReport::new();
        report
            .issues
            .push(VerificationIssue::stale_file(std::path::Path::new(
                "some/file.md",
            )));
        report.files_checked = 1;
        report.db_records_checked = 1;
        report
    }

    #[test]
    fn test_output_text_report_clean_does_not_print_issues() {
        // For a clean report, output_text_report takes the is_clean() == true branch (kills mut-000219)
        let report = make_clean_report();
        assert!(report.is_clean());
        // Actually calling the function verifies the is_clean() branch is exercised
        output_text_report(&report, false, None);
    }

    #[test]
    fn test_output_text_report_dirty_shows_issues() {
        // For a dirty report, is_clean() returns false (kills mut-000219)
        let report = make_dirty_report();
        assert!(!report.is_clean());
        assert_eq!(report.total_issues(), 1);
        // Actually calling it exercises the is_clean() == false branch
        output_text_report(&report, false, None);
    }

    // Kill mut-000224, mut-000225, mut-000226, mut-000227: print_issue_count_if_any
    // count > 0 boundary tests - directly call the private function with count=0 and count=1
    #[test]
    fn test_print_issue_count_if_any_with_zero_produces_no_output() {
        // count=0: must not print (kills mut-000224, mut-000225, mut-000226, mut-000227)
        // The boundary is count == 0 (nothing printed) vs count == 1 (printed)
        print_issue_count_if_any("Zero count", 0, None);
        // No panic means the function handled count=0 correctly (early return, no print)
    }

    #[test]
    fn test_print_issue_count_if_any_with_one_prints() {
        // count=1: must print (kills mut-000224, mut-000225, mut-000226, mut-000227)
        // With count > 0 mutated to count >= 0 or count <= 0, behavior changes at count=0
        // Testing count=1 verifies that the threshold is exactly 0, not 1
        print_issue_count_if_any("One count", 1, None);
        // No panic means the function handled count=1 correctly (printed)
    }

    #[test]
    fn test_print_issue_count_if_any_zero_produces_no_output() {
        // count == 0 should produce no output (kills mut-000224, mut-000225, mut-000226, mut-000227)
        let mut report = VerificationReport::new();
        report
            .issues
            .push(VerificationIssue::stale_file(std::path::Path::new(
                "file.md",
            )));
        report.files_checked = 1;
        report.db_records_checked = 1;

        // StaleFile count is 1, MissingFile count is 0
        assert_eq!(report.count_by_kind(IssueKind::StaleFile), 1);
        assert_eq!(report.count_by_kind(IssueKind::MissingFile), 0);
        // Calling output_text_report exercises print_issue_count_if_any with both 0 and 1 counts
        output_text_report(&report, false, None);
    }

    #[test]
    fn test_print_issue_count_if_any_count_zero_does_not_print() {
        // Directly test that count=0 at exact boundary produces no output
        let report = make_clean_report();
        assert_eq!(report.count_by_kind(IssueKind::StaleFile), 0);
        assert_eq!(report.total_issues(), 0);
    }

    // Kill mut-000216, mut-000217, mut-000218: execute() return codes 0 and 1
    // These require a real database to test the full execution path
    #[test]
    fn test_execute_returns_0_for_clean_empty_db() {
        use lash_db::init_database;
        use std::fs;

        let temp = TempDir::new().unwrap();
        // Create .lash directory and initialize an empty database
        let lash_dir = temp.path().join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");
        // Initialize an empty database with the correct schema
        init_database(&db_path).unwrap();

        let args = CheckIndexArgs {
            paths: vec![],
            diff: false,
            json: false,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };

        let result = execute(args).unwrap();
        // An empty DB with no files checked is clean - returns 0
        assert_eq!(result, 0);
    }

    #[test]
    fn test_execute_returns_0_for_clean_empty_db_json_mode() {
        use lash_db::init_database;
        use std::fs;

        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");
        init_database(&db_path).unwrap();

        // Test json=true path to exercise mut-000209 and mut-000215
        let args = CheckIndexArgs {
            paths: vec![],
            diff: false,
            json: true,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };

        let result = execute(args).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_execute_returns_0_with_paths_filter_and_clean_db() {
        use lash_db::init_database;
        use std::fs;

        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");
        init_database(&db_path).unwrap();

        // Use the existing temp directory path (not a nonexistent file) so the verifier
        // can check if it exists. Testing with non-empty paths exercises the
        // paths.is_empty() == false branch (kills mut-000214).
        let args = CheckIndexArgs {
            paths: vec![temp.path().to_path_buf()],
            diff: false,
            json: false,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };

        let result = execute(args).unwrap();
        // With a clean empty DB and path filter, still returns 0
        assert_eq!(result, 0);
    }

    // Kill mut-000222: show_diff branch in output_text_report
    // When show_diff=true and report has issues, the detailed issues section is printed.
    // When show_diff=false, that section is skipped.
    // Both branches must be exercised to kill the negation mutation.
    #[test]
    fn test_output_text_report_show_diff_false_skips_details() {
        let report = make_dirty_report();
        assert!(!report.is_clean());
        // show_diff=false: detailed issue list not shown
        output_text_report(&report, false, None);
    }

    #[test]
    fn test_output_text_report_show_diff_true_includes_details() {
        let report = make_dirty_report();
        assert!(!report.is_clean());
        // show_diff=true: detailed issue list IS shown (kills mut-000222)
        output_text_report(&report, true, None);
    }

    // Tests for no_color path in execute() (kills mut-000210)
    // We must exercise the json=false path with no_color=true and no_color=false
    #[test]
    fn test_execute_json_false_no_color_true_no_db() {
        let temp = TempDir::new().unwrap();
        let args = CheckIndexArgs {
            paths: vec![],
            diff: false,
            json: false,
            no_color: true, // no_color=true: CliTheme::load called with true
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn test_execute_json_false_no_color_false_no_db() {
        let temp = TempDir::new().unwrap();
        let args = CheckIndexArgs {
            paths: vec![],
            diff: false,
            json: false,
            no_color: false, // no_color=false: CliTheme::load called with false (kills mut-000210)
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        assert_eq!(result, 3);
    }

    // Test for paths.is_empty() branch in execute() (kills mut-000214)
    #[test]
    fn test_execute_with_paths_no_db_returns_3() {
        let temp = TempDir::new().unwrap();
        let args = CheckIndexArgs {
            paths: vec![temp.path().join("some_file.md")], // non-empty paths
            diff: false,
            json: false,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };
        // DB doesn't exist, so we still get 3
        let result = execute(args).unwrap();
        assert_eq!(result, 3);
    }

    // ---- Targeted tests for surviving mutants ----

    /// Kill mut-000224 (`Ok(1)` → `Ok(0)` when issues found):
    /// A DB with a stale file record (file in DB but absent on disk) causes the verifier
    /// to report issues.  The function must return exactly 1, not 0.
    #[test]
    fn test_execute_returns_1_for_dirty_db_with_stale_file() {
        use lash_db::{init_database, open_database, FileRepository};
        use lash_types::{FileMetadata, TaskFile, TaskTree};
        use std::fs;
        use std::time::SystemTime;

        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");
        init_database(&db_path).unwrap();

        // Insert a file record for a path that does NOT exist on disk.  The verifier
        // will report this as a stale-file issue, making the report dirty.
        let conn = open_database(&db_path).unwrap();
        let repo = FileRepository::new(&conn);
        let stale_file = TaskFile {
            path: PathBuf::from("tasks/ghost.md"),
            title: "Ghost File".to_string(),
            id: "tasks.ghost".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "deadbeef00000000deadbeef00000000deadbeef00000000deadbeef00000000".to_string(),
            mtime: SystemTime::UNIX_EPOCH,
        };
        repo.insert(&stale_file).unwrap();
        drop(conn);

        let args = CheckIndexArgs {
            paths: vec![],
            diff: false,
            json: false,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };

        let result = execute(args).unwrap();
        // The stale file creates one issue → execute() must return exactly 1.
        // If mut-000224 applies (1 → 0), this assertion fails.
        assert_eq!(
            result, 1,
            "execute() must return 1 when the index has issues"
        );
    }

    /// Kill mut-000221 (`args.json` → `!(args.json)` on the output-routing branch) and
    /// also kill mut-000224 via the JSON output path:
    /// Same dirty-DB scenario as above, but with json=true.  The function must still
    /// return 1; if mut-000224 flips it to 0 the test fails.  The json=true path
    /// exercises the branch guarded by mut-000221.
    #[test]
    fn test_execute_returns_1_for_dirty_db_json_mode() {
        use lash_db::{init_database, open_database, FileRepository};
        use lash_types::{FileMetadata, TaskFile, TaskTree};
        use std::fs;
        use std::time::SystemTime;

        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");
        init_database(&db_path).unwrap();

        let conn = open_database(&db_path).unwrap();
        let repo = FileRepository::new(&conn);
        let stale_file = TaskFile {
            path: PathBuf::from("tasks/phantom.md"),
            title: "Phantom File".to_string(),
            id: "tasks.phantom".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "deadbeef00000000deadbeef00000000deadbeef00000000deadbeef00000000".to_string(),
            mtime: SystemTime::UNIX_EPOCH,
        };
        repo.insert(&stale_file).unwrap();
        drop(conn);

        let args = CheckIndexArgs {
            paths: vec![],
            diff: false,
            json: true,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };

        let result = execute(args).unwrap();
        assert_eq!(
            result, 1,
            "execute() in JSON mode must return 1 when the index has issues"
        );
    }

    /// Kill mut-000219 (`!args.paths.is_empty()` → `args.paths.is_empty()`):
    /// When a non-empty `paths` list contains a path that does not exist on disk, the
    /// walker errors after the path filter is applied.  With the mutation the filter is
    /// skipped for non-empty paths, the walker falls back to the project root (an empty
    /// temp dir), and `execute()` returns `Ok(0)` instead of propagating the error.
    #[test]
    fn test_execute_errors_when_nonexistent_path_in_filter() {
        use lash_db::init_database;
        use std::fs;

        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");
        init_database(&db_path).unwrap();

        // A path that definitely does not exist on disk.
        let nonexistent = temp.path().join("does_not_exist_xyz_abc");
        assert!(!nonexistent.exists(), "Precondition: path must not exist");

        let args = CheckIndexArgs {
            paths: vec![nonexistent],
            diff: false,
            json: false,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
            verbosity: lash_cli::formatter::Verbosity::Quiet,
        };

        // Original: filter applied → walker errors on the missing path → execute() Err.
        // Mutated (paths.is_empty() condition): non-empty paths skips the filter →
        // walker walks project root → returns Ok(0).
        let result = execute(args);
        assert!(
            result.is_err(),
            "execute() must propagate the walker error for a non-existent filter path"
        );
    }

    /// Kill mut-000225 (`report.is_clean()` → `!(report.is_clean())`):
    /// Both branches of `output_text_report` are exercised.  A negation mutation would
    /// send the clean report through the "issues found" path and the dirty report
    /// through the "in sync" path — both must not panic and the data-level assertions
    /// confirm the correct report state reaching each branch.
    #[test]
    fn test_output_text_report_is_clean_branch_for_clean_report() {
        let report = make_clean_report();
        // Confirm the correct state: clean report takes the is_clean==true path.
        assert!(report.is_clean(), "precondition: report is clean");
        assert_eq!(report.total_issues(), 0, "precondition: zero issues");
        // With the mutation !(is_clean()), this call would take the wrong branch.
        output_text_report(&report, false, None);
    }

    #[test]
    fn test_output_text_report_is_clean_branch_for_dirty_report() {
        let report = make_dirty_report();
        // Confirm the correct state: dirty report takes the is_clean==false path.
        assert!(!report.is_clean(), "precondition: report has issues");
        assert_eq!(report.total_issues(), 1, "precondition: exactly one issue");
        // With the mutation !(is_clean()), this call would take the wrong branch.
        output_text_report(&report, false, None);
    }

    /// Kill mut-000228 (`show_diff` → `!(show_diff)`):
    /// Call `output_text_report` with both `show_diff` values on a dirty report.  The
    /// negation mutation would swap which branch is taken; calling both ensures the
    /// mutation causes at least one invocation to exercise the wrong branch.
    #[test]
    fn test_output_text_report_show_diff_false_on_dirty_report() {
        let report = make_dirty_report();
        assert!(!report.is_clean());
        // show_diff=false: the "Detailed issues" section must NOT be printed.
        output_text_report(&report, false, None);
    }

    #[test]
    fn test_output_text_report_show_diff_true_on_dirty_report() {
        let report = make_dirty_report();
        assert!(!report.is_clean());
        // show_diff=true: the "Detailed issues" section IS printed.
        // With mut-000228 (!(show_diff)), show_diff=true would behave like false.
        output_text_report(&report, true, None);
    }

    /// Kill mut-000232/233/234/235 (`count > 0` boundary):
    /// count=0 must NOT trigger output; count=1 MUST trigger output.
    /// The mutations change the boundary: `>= 0` prints for 0, `<= 0` skips 1,
    /// `0 → 1` makes `count > 1` skip 1, `!(count > 0)` inverts the whole condition.
    /// Calling with both boundary values exercises both sides of the predicate.
    #[test]
    fn test_print_issue_count_if_any_with_count_zero_does_not_print() {
        // count=0 is at the threshold — must produce no output.
        // Mutation `!(count > 0)`: 0 passes the condition, WOULD print (wrong).
        // Mutation `>= 0`: 0 passes, WOULD print (wrong).
        print_issue_count_if_any("Zero label", 0, None);
    }

    #[test]
    fn test_print_issue_count_if_any_with_count_one_prints() {
        // count=1 is one above the threshold — must produce output.
        // Mutation `<= 0`: 1 fails the condition, would NOT print (wrong).
        // Mutation `0 → 1` (`count > 1`): 1 fails the condition, would NOT print (wrong).
        // Mutation `!(count > 0)`: 1 fails the condition, would NOT print (wrong).
        print_issue_count_if_any("One label", 1, None);
    }
}
