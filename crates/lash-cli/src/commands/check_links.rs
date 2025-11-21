//! Check-links command implementation
//!
//! The `lash check-links` command finds broken dependency references in task files.

use anyhow::{Context, Result};
use lash_db::open_database;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

/// Arguments for the check-links command
#[derive(Debug, Clone)]
pub struct CheckLinksArgs {
    /// Output JSON diagnostics
    pub json: bool,
    /// Attempt to automatically fix broken links (not yet implemented)
    #[allow(dead_code)] // Reserved for future --fix implementation
    pub fix: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
}

/// A broken link report entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokenLink {
    /// The task ID that has the broken dependency
    pub from_task_full_id: String,
    /// The file containing the task
    pub from_file_path: String,
    /// The raw reference string that couldn't be resolved
    pub raw_ref: String,
    /// The kind of dependency
    pub kind: String,
}

/// Report of all broken links found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokenLinksReport {
    /// Total number of broken links found
    pub total_broken: usize,
    /// Broken links grouped by file
    pub by_file: Vec<FileLinks>,
}

/// Broken links for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLinks {
    /// File path
    pub file_path: String,
    /// Number of broken links in this file
    pub count: usize,
    /// The broken links
    pub links: Vec<BrokenLink>,
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
        "Starting check-links operation"
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

    // Find all broken links (function will open DB internally)
    let report = find_broken_links(&db_path).context("Failed to find broken links")?;

    // Output results
    if args.json {
        output_json_report(&report)?;
    } else {
        output_text_report(&report, args.no_color);
    }

    // Return exit code based on findings
    if report.total_broken == 0 {
        tracing::info!("No broken links found");
        Ok(0)
    } else {
        tracing::warn!(broken_count = report.total_broken, "Found broken links");
        Ok(1) // Exit code 1 for broken links found
    }
}

/// Get the database path for a project
fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/db.sqlite")
}

/// Find all broken dependency links in the database
///
/// A broken link is a dependency where `to_task_id` is NULL, meaning the
/// target task could not be resolved during indexing.
fn find_broken_links(conn: impl AsRef<std::path::Path>) -> Result<BrokenLinksReport> {
    // Re-open connection for querying
    let conn = open_database(conn.as_ref()).context("Failed to open database")?;

    // Query all dependencies with NULL to_task_id
    let mut stmt = conn.prepare(
        "SELECT
            d.raw_ref,
            d.kind,
            t.full_id as from_task_full_id,
            f.path as from_file_path
         FROM dependencies d
         JOIN tasks t ON d.from_task_id = t.id
         JOIN files f ON t.file_id = f.id
         WHERE d.to_task_id IS NULL
         ORDER BY f.path, t.order_index",
    )?;

    let broken_links = stmt
        .query_map([], |row| {
            Ok(BrokenLink {
                from_task_full_id: row.get(2)?,
                from_file_path: row.get(3)?,
                raw_ref: row
                    .get::<_, Option<String>>(0)?
                    .unwrap_or_else(|| "?".to_string()),
                kind: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // Group by file
    let mut by_file: Vec<FileLinks> = Vec::new();
    for link in broken_links {
        if let Some(file_links) = by_file
            .iter_mut()
            .find(|fl| fl.file_path == link.from_file_path)
        {
            file_links.links.push(link);
            file_links.count += 1;
        } else {
            by_file.push(FileLinks {
                file_path: link.from_file_path.clone(),
                count: 1,
                links: vec![link],
            });
        }
    }

    let total_broken = by_file.iter().map(|fl| fl.count).sum();

    Ok(BrokenLinksReport {
        total_broken,
        by_file,
    })
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

/// Output broken links report as JSON
fn output_json_report(report: &BrokenLinksReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Output broken links report as human-readable text
fn output_text_report(report: &BrokenLinksReport, no_color: bool) {
    use owo_colors::OwoColorize;

    let use_color = !no_color;

    // Header
    if report.total_broken == 0 {
        if use_color {
            println!("{}", "No broken links found!".green().bold());
        } else {
            println!("No broken links found!");
        }
        return;
    }

    // Issues found
    if use_color {
        println!(
            "{}",
            format!("Found {} broken link(s)", report.total_broken)
                .red()
                .bold()
        );
    } else {
        println!("Found {} broken link(s)", report.total_broken);
    }
    println!();

    // Group by file
    for file_links in &report.by_file {
        if use_color {
            println!(
                "{} ({})",
                file_links.file_path.cyan().bold(),
                format!("{} broken", file_links.count).red()
            );
        } else {
            println!("{} ({} broken)", file_links.file_path, file_links.count);
        }

        for link in &file_links.links {
            if use_color {
                println!(
                    "  {} {}",
                    "•".dimmed(),
                    format!("Task: {}", link.from_task_full_id).yellow()
                );
                println!(
                    "    {} {}",
                    "Broken reference:".dimmed(),
                    link.raw_ref.red()
                );
                println!("    {} {}", "Dependency kind:".dimmed(), link.kind.dimmed());
            } else {
                println!("  • Task: {}", link.from_task_full_id);
                println!("    Broken reference: {}", link.raw_ref);
                println!("    Dependency kind: {}", link.kind);
            }
            println!();
        }
    }

    // Suggestion
    println!();
    if use_color {
        println!("{}", "What to do:".bold());
        println!("  1. Check that the referenced tasks exist in your Markdown files");
        println!(
            "  2. Fix the {} annotations in the files above",
            "@depends-on".cyan()
        );
        println!("  3. Run {} to rebuild the index", "lash index".cyan());
    } else {
        println!("What to do:");
        println!("  1. Check that the referenced tasks exist in your Markdown files");
        println!("  2. Fix the @depends-on annotations in the files above");
        println!("  3. Run 'lash index' to rebuild the index");
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

    #[test]
    fn test_broken_link_serialization() {
        let link = BrokenLink {
            from_task_full_id: "test#task1".to_string(),
            from_file_path: "test.md".to_string(),
            raw_ref: "other#task999".to_string(),
            kind: "explicit_id".to_string(),
        };

        let json = serde_json::to_string(&link).unwrap();
        let deserialized: BrokenLink = serde_json::from_str(&json).unwrap();

        assert_eq!(link.from_task_full_id, deserialized.from_task_full_id);
        assert_eq!(link.raw_ref, deserialized.raw_ref);
    }

    #[test]
    fn test_report_serialization() {
        let report = BrokenLinksReport {
            total_broken: 2,
            by_file: vec![FileLinks {
                file_path: "test.md".to_string(),
                count: 2,
                links: vec![
                    BrokenLink {
                        from_task_full_id: "test#task1".to_string(),
                        from_file_path: "test.md".to_string(),
                        raw_ref: "other#task999".to_string(),
                        kind: "explicit_id".to_string(),
                    },
                    BrokenLink {
                        from_task_full_id: "test#task2".to_string(),
                        from_file_path: "test.md".to_string(),
                        raw_ref: "missing/file.md".to_string(),
                        kind: "explicit_path".to_string(),
                    },
                ],
            }],
        };

        let json = serde_json::to_string_pretty(&report).unwrap();
        let deserialized: BrokenLinksReport = serde_json::from_str(&json).unwrap();

        assert_eq!(report.total_broken, deserialized.total_broken);
        assert_eq!(report.by_file.len(), deserialized.by_file.len());
    }
}
