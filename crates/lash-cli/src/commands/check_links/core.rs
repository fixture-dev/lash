//! Check-links command implementation
//!
//! The `lash check-links` command finds broken dependency references in task files.

use anyhow::{Context, Result};
use lash_cli::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash_cli::formatter::{OutputFormat, Verbosity};
use lash_cli::theme::CliTheme;
use lash_core::dependency::reference::resolve_reference;
use lash_core::linter::rules::semantic::BrokenDocFragmentRule;
use lash_core::linter::{LintContext, LintDiagnostic, LintRule};
use lash_core::parser::parse_file;
use lash_db::open_database;
use lash_types::dependency::DependencyKind;
use lash_types::error::LashError;
use lash_types::{LashConfig, TaskFile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::{discover_markdown_files, find_project_root};

/// Arguments for the check-links command (legacy, kept for potential future use)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CheckLinksArgs {
    /// Output JSON diagnostics
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Optional CLI theme for styling
    pub theme: Option<CliTheme>,
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

/// A broken `@doc:` fragment reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokenDocFragment {
    /// File containing the bad annotation
    pub from_file_path: String,
    /// The full reference (path#fragment) as written in the source
    pub raw_ref: String,
    /// Line number of the `@doc:` annotation (0 if location couldn't be determined)
    pub line: usize,
    /// Column number of the `@doc:` annotation (0 if location couldn't be determined)
    pub column: usize,
    /// Help/suggestion text from the lint rule
    pub help: Option<String>,
}

/// Report of all broken links found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokenLinksReport {
    /// Total number of broken links found (dependency refs + doc fragments)
    pub total_broken: usize,
    /// Broken links grouped by file
    pub by_file: Vec<FileLinks>,
    /// Broken `@doc:` fragment references, grouped by file
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub broken_doc_fragments: Vec<BrokenDocFragment>,
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
#[allow(dead_code)]
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
        output_text_report(&report, args.theme.as_ref());
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
pub fn get_database_path(project_root: &Path) -> PathBuf {
    project_root.join(".lash/lash.db")
}

/// Find all broken dependency links in the database
///
/// A broken link is a dependency where `to_task_id` is NULL, meaning the
/// target task could not be resolved during indexing.
pub fn find_broken_links(conn: impl AsRef<std::path::Path>) -> Result<BrokenLinksReport> {
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
        broken_doc_fragments: Vec::new(),
    })
}

/// Walk every task file under `project_root` and report `@depends-on`
/// references that don't resolve.
///
/// `lash check-links` historically only reported dependencies the *indexer*
/// had already marked broken (a `NULL` `to_task_id` row) — but explicit
/// `@depends-on` edges aren't stored that way, so `check-links` silently
/// passed while `lash lint` flagged the very same references (GitHub issue
/// #19). This reparses the tree and resolves each reference with the **same**
/// [`resolve_reference`] used by the linter, so the two surfaces always agree.
///
/// Files that fail to parse are skipped (surfaced by `lash lint` instead).
#[must_use]
pub fn find_broken_dependencies(project_root: &Path) -> Vec<FileLinks> {
    let Ok(markdown_files) = discover_markdown_files(&[project_root.to_path_buf()], true) else {
        return Vec::new();
    };

    let config = LashConfig::from_root(project_root).unwrap_or_default();

    let mut files: HashMap<PathBuf, TaskFile> = HashMap::new();
    for path in &markdown_files {
        let Ok(file) = parse_file(path, &config) else {
            continue;
        };
        let relative = path
            .strip_prefix(project_root)
            .unwrap_or(path)
            .to_path_buf();
        files.insert(relative, file);
    }

    let mut by_file: Vec<FileLinks> = Vec::new();

    for (rel_path, file) in &files {
        let ctx = LintContext::new(&config, rel_path.clone(), &files);
        let mut links: Vec<BrokenLink> = Vec::new();

        for task in file.tasks.tasks() {
            for dep in &task.metadata.depends_on {
                if matches!(
                    dep.kind,
                    DependencyKind::Hierarchy | DependencyKind::Directory
                ) {
                    continue;
                }
                let resolve_path = |rel: &str| ctx.resolve_path(Path::new(rel));
                if resolve_reference(&dep.target, rel_path, &file.id, &files, resolve_path).is_err()
                {
                    links.push(BrokenLink {
                        from_task_full_id: lash_types::make_full_id(&file.id, &task.id),
                        from_file_path: rel_path.display().to_string(),
                        raw_ref: dep.target.clone(),
                        kind: dep.kind.as_str().to_string(),
                    });
                }
            }
        }

        if !links.is_empty() {
            by_file.push(FileLinks {
                file_path: rel_path.display().to_string(),
                count: links.len(),
                links,
            });
        }
    }

    by_file.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    by_file
}

/// Walk every task file under `project_root` and report `@doc:` annotations
/// whose `#fragment` does not resolve to a heading in the target document.
///
/// This is the canonical place to scan for broken `@doc:` references — `lash
/// lint` raises the same warnings under `W_SEM_DOC_FRAGMENT`, but
/// `check-links` is the user-facing "are my cross-file references intact?"
/// command, so it should not silently pass while `lint` is failing.
///
/// Files that fail to parse are silently skipped: parse errors are surfaced
/// by `lash lint`, and this command should not duplicate that signal.
#[must_use]
pub fn find_broken_doc_fragments(project_root: &Path) -> Vec<BrokenDocFragment> {
    let Ok(markdown_files) = discover_markdown_files(&[project_root.to_path_buf()], true) else {
        return Vec::new();
    };

    let config = LashConfig::from_root(project_root).unwrap_or_default();

    // Parse files into the LintContext-shaped map. Skip unparseable files
    // (lash lint will surface those errors separately).
    let mut files: HashMap<PathBuf, TaskFile> = HashMap::new();
    for path in &markdown_files {
        let Ok(file) = parse_file(path, &config) else {
            continue;
        };
        let relative = path
            .strip_prefix(project_root)
            .unwrap_or(path)
            .to_path_buf();
        files.insert(relative, file);
    }

    let rule = BrokenDocFragmentRule::new();
    let mut findings: Vec<BrokenDocFragment> = Vec::new();

    for (rel_path, file) in &files {
        let ctx = LintContext::new(&config, rel_path.clone(), &files);
        let diagnostics: Vec<LintDiagnostic> = rule
            .check_file(file, &ctx)
            .into_iter()
            .chain(
                file.tasks
                    .tasks()
                    .iter()
                    .flat_map(|task| rule.check_task(task, &ctx)),
            )
            .collect();

        for diag in diagnostics {
            findings.push(BrokenDocFragment {
                from_file_path: rel_path.display().to_string(),
                raw_ref: extract_raw_ref_from_message(&diag.message),
                line: diag.location.line.unwrap_or(0),
                column: diag.location.column.unwrap_or(0),
                help: diag.help.clone(),
            });
        }
    }

    findings.sort_by(|a, b| {
        a.from_file_path
            .cmp(&b.from_file_path)
            .then(a.line.cmp(&b.line))
            .then(a.raw_ref.cmp(&b.raw_ref))
    });

    findings
}

/// Pull the `path#fragment` substring out of a `W_SEM_DOC_FRAGMENT` message.
/// The rule emits messages like:
///   `Fragment 'foo' not found in 'docs/bar.md'`
///   `Empty fragment identifier in doc reference: 'docs/bar.md'`
/// We re-assemble a single `raw_ref` string for the report.
fn extract_raw_ref_from_message(message: &str) -> String {
    let in_split = message.split_once("' not found in '");
    if let Some((before, after)) = in_split {
        let frag = before.rsplit_once('\'').map_or(before, |(_, f)| f);
        let path = after.trim_end_matches('\'');
        return format!("{path}#{frag}");
    }

    if let Some(start) = message.find("doc reference: '") {
        let rest = &message[start + "doc reference: '".len()..];
        if let Some(end) = rest.find('\'') {
            return format!("{}#", &rest[..end]);
        }
    }

    message.to_string()
}

/// Output JSON when database doesn't exist
pub fn output_json_no_db() -> Result<()> {
    use serde_json::json;

    let output = json!({
        "error": "Database not found",
        "suggestion": "Run `lash index` to create the database"
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output broken links report as JSON
pub fn output_json_report(report: &BrokenLinksReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Output broken links report as human-readable text using `ErrorReporter`
#[allow(clippy::too_many_lines)]
pub fn output_text_report(report: &BrokenLinksReport, theme: Option<&CliTheme>) {
    // Header
    if report.total_broken == 0 {
        if let Some(theme) = theme {
            println!("{}", theme.style_success("No broken links found!"));
        } else {
            println!("No broken links found!");
        }
        return;
    }

    // Create error reporter for consistent formatting
    let reporter_config = ErrorReporterConfig {
        verbosity: Verbosity::Normal,
        output_format: OutputFormat::Text,
        display_mode: ErrorDisplayMode::Batch,
        theme: theme.cloned(),
        show_summary: false,
    };
    let mut reporter = ErrorReporter::new(reporter_config);

    // Convert broken links to LashError::Dependency variants and collect
    for file_links in &report.by_file {
        for link in &file_links.links {
            // Extract location from the from_task_full_id
            // Format is typically: "file_path#task_id"
            let (file_path, _task_id) = link
                .from_task_full_id
                .split_once('#')
                .unwrap_or((&file_links.file_path, ""));

            let error = LashError::dep_not_found(
                PathBuf::from(file_path),
                0, // We don't have line info from the DB
                0, // We don't have column info from the DB
                &link.raw_ref,
            )
            .to_diagnostic()
            .with_help(format!(
                "Task '{}' references '{}' but the target could not be found",
                link.from_task_full_id, link.raw_ref
            ))
            .with_labels(vec![
                ("from_task".to_string(), link.from_task_full_id.clone()),
                ("dependency_kind".to_string(), link.kind.clone()),
            ]);

            reporter.report_diagnostic(&error);
        }
    }

    // Print all collected errors
    reporter.flush();

    // Render broken @doc: fragments using the same human-readable style.
    let dep_count = report.by_file.iter().map(|fl| fl.count).sum::<usize>();
    let doc_count = report.broken_doc_fragments.len();
    if doc_count > 0 {
        println!();
        if let Some(theme) = theme {
            println!(
                "{}",
                theme.style_warning(&format!("Broken @doc: fragment(s) ({doc_count}):"))
            );
        } else {
            println!("Broken @doc: fragment(s) ({doc_count}):");
        }
        for frag in &report.broken_doc_fragments {
            let location = if frag.line > 0 {
                format!("{}:{}:{}", frag.from_file_path, frag.line, frag.column)
            } else {
                frag.from_file_path.clone()
            };
            if let Some(theme) = theme {
                println!(
                    "  {} {}  {}",
                    theme.style_error("✗"),
                    theme.style_label(&location),
                    theme.style_muted(&frag.raw_ref)
                );
            } else {
                println!("  ✗ {location}  {}", frag.raw_ref);
            }
            if let Some(help) = &frag.help {
                if let Some(theme) = theme {
                    println!("    {}", theme.style_muted(help));
                } else {
                    println!("    {help}");
                }
            }
        }
    }

    // Print summary
    println!();
    if let Some(theme) = theme {
        use owo_colors::OwoColorize;
        println!(
            "{}",
            theme.style_error(&format!("Found {} broken link(s)", report.total_broken))
        );
        if dep_count > 0 && doc_count > 0 {
            println!("  ({dep_count} dependency reference(s), {doc_count} @doc: fragment(s))");
        }
        println!();
        println!("{}", "What to do:".bold());
        println!("  1. Check that the referenced tasks exist in your Markdown files");
        println!(
            "  2. Fix the {} annotations in the files above",
            theme.style_info("@depends-on")
        );
        if doc_count > 0 {
            println!(
                "  3. Fix the {} fragments — run `lash explain W_SEM_DOC_FRAGMENT` for slug rules",
                theme.style_info("@doc:")
            );
            println!(
                "  4. Run {} to rebuild the index",
                theme.style_info("lash index")
            );
        } else {
            println!(
                "  3. Run {} to rebuild the index",
                theme.style_info("lash index")
            );
        }
    } else {
        println!("Found {} broken link(s)", report.total_broken);
        if dep_count > 0 && doc_count > 0 {
            println!("  ({dep_count} dependency reference(s), {doc_count} @doc: fragment(s))");
        }
        println!();
        println!("What to do:");
        println!("  1. Check that the referenced tasks exist in your Markdown files");
        println!("  2. Fix the @depends-on annotations in the files above");
        if doc_count > 0 {
            println!("  3. Fix the @doc: fragments — run `lash explain W_SEM_DOC_FRAGMENT` for slug rules");
            println!("  4. Run 'lash index' to rebuild the index");
        } else {
            println!("  3. Run 'lash index' to rebuild the index");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a test database with a single broken dependency link and return
    /// the path to the db file.  The broken link has:
    ///   - `raw_ref`  = "missing#ref" (column 0 in the SELECT)
    ///   - kind     = "`explicit_id`" (column 1)
    ///   - `full_id`  = "test.md#task1" (column 2)
    ///   - file path = "test.md" (column 3)
    fn create_db_with_broken_link(dir: &std::path::Path) -> PathBuf {
        use lash_db::init_database;
        use std::fs;

        let lash_dir = dir.join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");

        let conn = init_database(&db_path).unwrap();

        // Insert a file row
        conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime) \
             VALUES ('test.md', 'test', 'Test File', 'abc123', 0)",
            [],
        )
        .unwrap();
        let file_id: i64 = conn.last_insert_rowid();

        // Insert a task row whose full_id matches "test.md#task1"
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index) \
             VALUES (?1, 'task1', 'test.md#task1', 'Task One', 'open', 0, 0)",
            rusqlite::params![file_id],
        )
        .unwrap();
        let task_id: i64 = conn.last_insert_rowid();

        // Insert a dependency with NULL to_task_id (broken link)
        conn.execute(
            "INSERT INTO dependencies (from_task_id, to_task_id, kind, raw_ref) \
             VALUES (?1, NULL, 'explicit_id', 'missing#ref')",
            rusqlite::params![task_id],
        )
        .unwrap();

        db_path
    }

    /// Create a test database with two broken links in the same file.
    fn create_db_with_two_broken_links(dir: &std::path::Path) -> PathBuf {
        use lash_db::init_database;
        use std::fs;

        let lash_dir = dir.join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        let db_path = lash_dir.join("lash.db");

        let conn = init_database(&db_path).unwrap();

        // One file, two tasks, each with a broken link
        conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime) \
             VALUES ('multi.md', 'multi', 'Multi File', 'def456', 0)",
            [],
        )
        .unwrap();
        let file_id: i64 = conn.last_insert_rowid();

        for i in 0..2_i64 {
            conn.execute(
                "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index) \
                 VALUES (?1, ?2, ?3, 'Task', 'open', 0, ?4)",
                rusqlite::params![
                    file_id,
                    format!("task{i}"),
                    format!("multi.md#task{i}"),
                    i,
                ],
            )
            .unwrap();
            let task_id: i64 = conn.last_insert_rowid();

            conn.execute(
                "INSERT INTO dependencies (from_task_id, to_task_id, kind, raw_ref) \
                 VALUES (?1, NULL, 'explicit_id', ?2)",
                rusqlite::params![task_id, format!("broken#ref{i}")],
            )
            .unwrap();
        }

        db_path
    }

    // ---------------------------------------------------------------------------
    // Tests targeting surviving mutants in find_broken_links
    // ---------------------------------------------------------------------------

    // Kill L154 (0 → 1 in row.get::<_, Option<String>>(0)?):
    // Column 0 is raw_ref.  With the mutation (get(1)), the "kind" value would be
    // placed in raw_ref.  We assert that raw_ref is exactly "missing#ref".
    //
    // Kill L156 (1 → 0 in row.get(1)?):
    // Column 1 is kind.  With the mutation (get(0)), raw_ref would be placed in
    // kind.  We assert that kind is exactly "explicit_id".
    #[test]
    fn test_find_broken_links_columns_raw_ref_and_kind_are_correct() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let db_path = create_db_with_broken_link(temp.path());

        let report = find_broken_links(&db_path).unwrap();

        assert_eq!(
            report.total_broken, 1,
            "expected exactly 1 broken link, got {}",
            report.total_broken
        );

        let link = &report.by_file[0].links[0];

        // Column 0 must be raw_ref; if mutated to 1, this would contain "explicit_id"
        assert_eq!(
            link.raw_ref, "missing#ref",
            "raw_ref must come from column 0 (raw_ref), not column 1 (kind)"
        );

        // Column 1 must be kind; if mutated to 0, this would contain "missing#ref"
        assert_eq!(
            link.kind, "explicit_id",
            "kind must come from column 1 (kind), not column 0 (raw_ref)"
        );
    }

    // Kill L166 (== replaced with != in the grouping condition):
    // With `!=`, every link would be placed into a new FileLinks group rather than
    // merging with the existing one.  Two broken links in the same file must produce
    // exactly one FileLinks group with count == 2.
    #[test]
    fn test_find_broken_links_groups_links_by_file() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let db_path = create_db_with_two_broken_links(temp.path());

        let report = find_broken_links(&db_path).unwrap();

        // Total count must be 2 (one file, two broken links)
        assert_eq!(
            report.total_broken, 2,
            "expected 2 total broken links, got {}",
            report.total_broken
        );

        // All links are from the same file, so they must be grouped into exactly one
        // FileLinks entry.  If the grouping condition uses != instead of ==, each link
        // would land in its own group and we would get 2 FileLinks here.
        assert_eq!(
            report.by_file.len(),
            1,
            "two broken links in the same file must be grouped into 1 FileLinks, \
             but got {}",
            report.by_file.len()
        );

        // The single FileLinks group must have count == 2
        assert_eq!(
            report.by_file[0].count, 2,
            "FileLinks count must be 2 for two broken links in the same file"
        );

        // And the links vec must also have 2 entries
        assert_eq!(
            report.by_file[0].links.len(),
            2,
            "FileLinks.links must contain 2 BrokenLink entries"
        );
    }

    // Kill L173 (1 → 0 in `count: 1` for new FileLinks):
    // When a new FileLinks group is created for a file that hasn't been seen yet,
    // count must start at 1 (not 0).  We verify by checking a single broken link
    // produces FileLinks with count == 1.
    #[test]
    fn test_find_broken_links_new_file_group_starts_with_count_one() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let db_path = create_db_with_broken_link(temp.path());

        let report = find_broken_links(&db_path).unwrap();

        assert_eq!(
            report.by_file.len(),
            1,
            "expected exactly 1 FileLinks group"
        );
        assert_eq!(
            report.by_file[0].count, 1,
            "new FileLinks group must start with count=1, not count=0"
        );
    }

    // Kill L173 and L166 together: verify that the link's from_file_path matches
    // the FileLinks file_path (i.e., grouping was done correctly by equality).
    #[test]
    fn test_find_broken_links_file_path_in_group_matches_link() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let db_path = create_db_with_broken_link(temp.path());

        let report = find_broken_links(&db_path).unwrap();

        let file_links = &report.by_file[0];
        let link = &file_links.links[0];

        // The group's file_path and the link's from_file_path must match
        assert_eq!(
            file_links.file_path, "test.md",
            "FileLinks.file_path must be 'test.md'"
        );
        assert_eq!(
            link.from_file_path, "test.md",
            "BrokenLink.from_file_path must be 'test.md'"
        );
        assert_eq!(
            file_links.file_path, link.from_file_path,
            "FileLinks.file_path must equal the link's from_file_path"
        );
    }

    // Kill full_id column (column 2) and from_file_path column (column 3):
    // Verify that from_task_full_id and from_file_path are populated correctly.
    #[test]
    fn test_find_broken_links_full_id_and_file_path_columns() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let db_path = create_db_with_broken_link(temp.path());

        let report = find_broken_links(&db_path).unwrap();
        let link = &report.by_file[0].links[0];

        assert_eq!(
            link.from_task_full_id, "test.md#task1",
            "from_task_full_id must be the full_id from column 2"
        );
        assert_eq!(
            link.from_file_path, "test.md",
            "from_file_path must be the file path from column 3"
        );
    }

    // Verify that total_broken is computed as the sum of per-file counts.
    // With two links in one file, count=2 and total_broken must also be 2.
    #[test]
    fn test_find_broken_links_total_is_sum_of_file_counts() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let db_path = create_db_with_two_broken_links(temp.path());

        let report = find_broken_links(&db_path).unwrap();

        let sum_of_counts: usize = report.by_file.iter().map(|fl| fl.count).sum();
        assert_eq!(
            report.total_broken, sum_of_counts,
            "total_broken must equal the sum of all FileLinks.count values"
        );
    }

    #[test]
    fn test_get_database_path() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let db_path = get_database_path(temp.path());

        assert_eq!(db_path, temp.path().join(".lash/lash.db"));
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
            broken_doc_fragments: vec![],
        };

        let json = serde_json::to_string_pretty(&report).unwrap();
        let deserialized: BrokenLinksReport = serde_json::from_str(&json).unwrap();

        assert_eq!(report.total_broken, deserialized.total_broken);
        assert_eq!(report.by_file.len(), deserialized.by_file.len());
    }

    // Kill mut-000235, mut-000236, mut-000237:
    // output_text_report when total_broken == 0 should print "No broken links found!"
    // output_text_report when total_broken != 0 should NOT print "No broken links found!"

    #[test]
    fn test_output_text_report_with_zero_broken_links() {
        // total_broken == 0 takes the early return path
        let report = BrokenLinksReport {
            total_broken: 0,
            by_file: vec![],
            broken_doc_fragments: vec![],
        };
        // Calling should not panic; the function prints "No broken links found!" and returns
        output_text_report(&report, None);
        // Verify total_broken is exactly 0 (not 1 or any other value)
        assert_eq!(report.total_broken, 0);
    }

    #[test]
    fn test_output_text_report_with_one_broken_link_does_not_early_return() {
        // total_broken == 1: the is_empty check fails, so we fall through to the reporter
        let report = BrokenLinksReport {
            total_broken: 1,
            by_file: vec![FileLinks {
                file_path: "test.md".to_string(),
                count: 1,
                links: vec![BrokenLink {
                    from_task_full_id: "test#task1".to_string(),
                    from_file_path: "test.md".to_string(),
                    raw_ref: "other#task999".to_string(),
                    kind: "explicit_id".to_string(),
                }],
            }],
            broken_doc_fragments: vec![],
        };
        // With total_broken=1, the function should NOT take the early return path
        assert_eq!(report.total_broken, 1);
        // Calling should not panic
        output_text_report(&report, None);
    }

    #[test]
    fn test_report_total_broken_is_exact_zero_not_one() {
        // Verifies that the literal 0 in the comparison matters (kills mut-000237)
        let empty_report = BrokenLinksReport {
            total_broken: 0,
            by_file: vec![],
            broken_doc_fragments: vec![],
        };
        let non_empty_report = BrokenLinksReport {
            total_broken: 1,
            by_file: vec![],
            broken_doc_fragments: vec![],
        };
        // These two must produce different behavior in output_text_report
        assert_eq!(empty_report.total_broken, 0);
        assert_ne!(non_empty_report.total_broken, 0);
        assert_eq!(empty_report.total_broken, 0);
        assert_ne!(non_empty_report.total_broken, 0);
    }

    // Kill mut-000243/244/245: total_broken == 0 is the exact early-return boundary.
    // Verify the boundary: total_broken=0 satisfies the condition; total_broken=1 does not.
    #[test]
    fn test_output_text_report_zero_boundary_exact() {
        let zero_report = BrokenLinksReport {
            total_broken: 0,
            by_file: vec![],
            broken_doc_fragments: vec![],
        };
        let one_report = BrokenLinksReport {
            total_broken: 1,
            by_file: vec![FileLinks {
                file_path: "test.md".to_string(),
                count: 1,
                links: vec![BrokenLink {
                    from_task_full_id: "test#task1".to_string(),
                    from_file_path: "test.md".to_string(),
                    raw_ref: "other#missing".to_string(),
                    kind: "explicit_id".to_string(),
                }],
            }],
            broken_doc_fragments: vec![],
        };
        assert_eq!(zero_report.total_broken, 0);
        assert_ne!(zero_report.total_broken, 1);
        assert_eq!(one_report.total_broken, 1);
        assert_ne!(one_report.total_broken, 0);
        output_text_report(&zero_report, None);
        output_text_report(&one_report, None);
    }

    // Kill mut-000247: show_summary must be false (not true).
    #[test]
    fn test_error_reporter_config_show_summary_is_false() {
        let config = ErrorReporterConfig {
            verbosity: Verbosity::Normal,
            output_format: OutputFormat::Text,
            display_mode: ErrorDisplayMode::Batch,
            theme: None,
            show_summary: false,
        };
        assert!(
            !config.show_summary,
            "show_summary must be false (assert_eq)"
        );
        assert!(!config.show_summary, "show_summary must not be true");
        let _reporter = ErrorReporter::new(config);
    }

    // Kill mut-000248/249: dep_not_found is called with line=0 and col=0.
    // The formatted location must contain ":0:0", not ":1:0" or ":0:1".
    #[test]
    fn test_dep_not_found_zero_line_col_formats_as_colon_zero_zero() {
        let err = LashError::dep_not_found(PathBuf::from("tasks/test.md"), 0, 0, "target#ref");
        let diag = err.to_diagnostic();
        let loc = diag
            .location
            .as_ref()
            .expect("dep_not_found must set a location");
        assert_eq!(loc.line, Some(0));
        assert_eq!(loc.column, Some(0));
        let reporter = ErrorReporter::new(ErrorReporterConfig {
            verbosity: Verbosity::Normal,
            output_format: OutputFormat::Text,
            display_mode: ErrorDisplayMode::Batch,
            theme: None,
            show_summary: false,
        });
        let formatted = reporter.format_diagnostic(&diag);
        assert!(
            formatted.contains(":0:0"),
            "dep_not_found(path,0,0) must format as ':0:0', got: {formatted}"
        );
        assert!(
            !formatted.contains(":1:"),
            "dep_not_found with line=0 must not format as ':1:...' (0->1 mutation), got: {formatted}"
        );
    }

    // Issue #11: find_broken_doc_fragments must surface @doc: fragment issues
    // that `lash lint` would also flag — otherwise check-links silently passes
    // while lint fails on the same tree.
    #[test]
    fn test_find_broken_doc_fragments_flags_unresolved_fragment() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Target doc has a heading "Overview", and a task references a
        // fragment that does NOT exist there.
        std::fs::write(root.join("design.md"), "# Design\n\n## Overview\n\nText.\n").unwrap();
        std::fs::write(
            root.join("tasks.md"),
            "# Tasks\n\n@id: tasks-file\n\n## Tasks\n\n\
             - [ ] Do the thing\n  \
             @id: thing\n  \
             @doc: design.md#does-not-exist\n",
        )
        .unwrap();

        let findings = find_broken_doc_fragments(root);
        assert_eq!(
            findings.len(),
            1,
            "expected one broken @doc: fragment, got {findings:?}"
        );
        let frag = &findings[0];
        assert_eq!(frag.from_file_path, "tasks.md");
        assert_eq!(frag.raw_ref, "design.md#does-not-exist");
        assert!(
            frag.line > 0,
            "should locate the @doc: line, got {}",
            frag.line
        );
    }

    // Issue #19: check-links must flag @depends-on references that lint flags.
    #[test]
    fn test_find_broken_dependencies_flags_unresolvable_ref() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        std::fs::write(root.join("lash.index.md"), "# Index\n").unwrap();
        std::fs::write(
            root.join("tasks.md"),
            "# Tasks\n\n@id: repro\n\n## Tasks\n\n\
             - [ ] Base\n  \
             @id: base-task\n\
             - [ ] Dependent\n  \
             @id: dep\n  \
             @depends-on: does-not-exist\n",
        )
        .unwrap();

        let by_file = find_broken_dependencies(root);
        assert_eq!(by_file.len(), 1, "expected one file with a broken dep");
        assert_eq!(by_file[0].count, 1);
        assert_eq!(by_file[0].links[0].raw_ref, "does-not-exist");
    }

    // Issue #19: forms that lint accepts must NOT be reported by check-links.
    #[test]
    fn test_find_broken_dependencies_accepts_valid_forms() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        std::fs::write(root.join("lash.index.md"), "# Index\n").unwrap();
        std::fs::write(
            root.join("tasks.md"),
            "# Tasks\n\n@id: repro\n\n## Tasks\n\n\
             - [ ] Base\n  \
             @id: base-task\n\
             - [ ] Other\n  \
             @id: other-task\n\
             - [ ] Bare id\n  \
             @id: d1\n  \
             @depends-on: base-task\n\
             - [ ] Samefile form\n  \
             @id: d2\n  \
             @depends-on: #task:base-task\n\
             - [ ] File id form\n  \
             @id: d3\n  \
             @depends-on: repro#task:base-task\n\
             - [ ] Comma list\n  \
             @id: d4\n  \
             @depends-on: base-task, other-task\n",
        )
        .unwrap();

        let by_file = find_broken_dependencies(root);
        assert!(
            by_file.is_empty(),
            "all references resolve, expected no broken deps, got {by_file:?}"
        );
    }

    #[test]
    fn test_find_broken_doc_fragments_clean_project_returns_empty() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        std::fs::write(root.join("design.md"), "# Design\n\n## Overview\n").unwrap();
        std::fs::write(
            root.join("tasks.md"),
            "# Tasks\n\n@id: tasks-file\n\n## Tasks\n\n\
             - [ ] Do the thing\n  \
             @id: thing\n  \
             @doc: design.md#overview\n",
        )
        .unwrap();

        let findings = find_broken_doc_fragments(root);
        assert!(
            findings.is_empty(),
            "valid fragments must produce no findings, got {findings:?}"
        );
    }

    // Verify line=0 and line=1 produce different output (mutation distinguishability).
    #[test]
    fn test_dep_not_found_one_vs_zero_formats_differently() {
        let err_zero = LashError::dep_not_found(PathBuf::from("t.md"), 0, 0, "r");
        let err_one = LashError::dep_not_found(PathBuf::from("t.md"), 1, 0, "r");
        let reporter = ErrorReporter::new(ErrorReporterConfig {
            verbosity: Verbosity::Normal,
            output_format: OutputFormat::Text,
            display_mode: ErrorDisplayMode::Batch,
            theme: None,
            show_summary: false,
        });
        let fmt_zero = reporter.format_diagnostic(&err_zero.to_diagnostic());
        let fmt_one = reporter.format_diagnostic(&err_one.to_diagnostic());
        assert_ne!(
            fmt_zero, fmt_one,
            "line=0 and line=1 must produce different output"
        );
    }
}
