//! Check-index command implementation
//!
//! The `lash check-index` command verifies `SQLite` database consistency with Markdown files.

use anyhow::{Context, Result};
use lash_db::{open_database, IndexVerifier, VerifierConfig};
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

/// Arguments for the check-index command
#[derive(Debug, Clone)]
pub struct CheckIndexArgs {
    /// Show detailed diff of inconsistencies
    pub diff: bool,
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
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
    let verifier_config = VerifierConfig::new(project_root.clone());
    let verifier = IndexVerifier::new(&conn, verifier_config);

    // Run verification
    let report = verifier.verify().context("Failed to verify index")?;

    // Output results
    if args.json {
        output_json_report(&report)?;
    } else {
        output_text_report(&report, args.diff, args.no_color);
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
    project_root.join(".lash/db.sqlite")
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
fn output_text_report(report: &lash_db::VerificationReport, show_diff: bool, no_color: bool) {
    use owo_colors::OwoColorize;

    let use_color = !no_color;

    // Header
    if report.is_clean() {
        if use_color {
            println!("{}", "Index is clean!".green().bold());
        } else {
            println!("Index is clean!");
        }
        println!();
        println!(
            "Checked {} files and {} database records",
            report.files_checked, report.db_records_checked
        );
        return;
    }

    // Issues found
    if use_color {
        println!(
            "{}",
            format!("Found {} issue(s)", report.total_issues())
                .red()
                .bold()
        );
    } else {
        println!("Found {} issue(s)", report.total_issues());
    }
    println!();

    // Summary by kind
    println!("Issues by type:");
    print_issue_count_if_any(
        "Stale files (in DB but not on disk)",
        report.count_by_kind(lash_db::IssueKind::StaleFile),
        use_color,
    );
    print_issue_count_if_any(
        "Missing files (on disk but not in DB)",
        report.count_by_kind(lash_db::IssueKind::MissingFile),
        use_color,
    );
    print_issue_count_if_any(
        "Hash mismatches (file modified)",
        report.count_by_kind(lash_db::IssueKind::HashMismatch),
        use_color,
    );
    print_issue_count_if_any(
        "Orphaned tasks",
        report.count_by_kind(lash_db::IssueKind::OrphanedTasks),
        use_color,
    );
    print_issue_count_if_any(
        "Orphaned dependencies",
        report.count_by_kind(lash_db::IssueKind::OrphanedDependencies),
        use_color,
    );

    // Detailed issue list if requested
    if show_diff {
        println!();
        if use_color {
            println!("{}", "Detailed issues:".bold());
        } else {
            println!("Detailed issues:");
        }
        println!();

        for issue in &report.issues {
            if use_color {
                println!(
                    "{} {}",
                    format!("[{}]", issue.kind).yellow().bold(),
                    issue.path.display().to_string().cyan()
                );
                println!("  {}", issue.description);
                println!("  {}", issue.fix_suggestion.dimmed());
            } else {
                println!("[{}] {}", issue.kind, issue.path.display());
                println!("  {}", issue.description);
                println!("  {}", issue.fix_suggestion);
            }
            println!();
        }
    }

    // Suggestion
    println!();
    if use_color {
        println!("{}", "To fix these issues, run:".bold());
        println!("  {}", "lash index".cyan());
    } else {
        println!("To fix these issues, run:");
        println!("  lash index");
    }
}

/// Print an issue count if non-zero
fn print_issue_count_if_any(label: &str, count: usize, use_color: bool) {
    use owo_colors::OwoColorize;

    if count > 0 {
        if use_color {
            println!("  {} {}", label, count.to_string().red());
        } else {
            println!("  {label}: {count}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_database_path() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let db_path = get_database_path(temp.path());

        assert_eq!(db_path, temp.path().join(".lash/db.sqlite"));
    }
}
