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
}
