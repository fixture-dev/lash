//! Migrate-ids command implementation
//!
//! `lash migrate-ids` finishes the repair that `lash index` starts when the
//! task-ID derivation rules change.
//!
//! The index fixes itself: it notices it was built under older rules,
//! re-derives every file, and records what each affected task's ID used to be
//! against what it is now. What it cannot fix is the Markdown. A
//! `@depends-on` written against an old ID is just text in a file, and it
//! stops resolving the moment the stored IDs move — all of them at once, which
//! makes the rebuild look like the thing that caused the damage.
//!
//! This command reads those recorded renames and rewrites the references.
//! It reports by default and only writes when asked, because it edits files
//! the user owns.

use anyhow::{Context, Result};
use clap::Args;
use lash::theme::CliTheme;
use lash_db::{
    open_database, IdMigrationRepository, Indexer, IndexerConfig, TaskIdRename, TaskRepository,
};
use lash_types::error::Diagnostic;
use lash_types::LashConfig;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::utils::file_discovery::find_project_root;

/// Arguments for the migrate-ids command
#[derive(Args, Debug, Clone)]
pub struct MigrateIdsArgs {
    /// Rewrite the references (without this, only reports what would change)
    #[arg(long)]
    pub write: bool,

    /// Discard the pending renames without rewriting anything
    #[arg(long, conflicts_with = "write")]
    pub forget: bool,

    /// Output format (text, json)
    #[arg(long, default_value = "text")]
    pub format: String,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,

    /// Project root (detected automatically if None)
    #[arg(skip)]
    pub project_root: Option<PathBuf>,
}

/// One reference that a rename applies to
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceRewrite {
    /// File the reference is written in, relative to the project root
    pub source_path: PathBuf,

    /// 1-indexed line the reference sits on
    pub line_number: usize,

    /// The reference as written
    pub old_reference: String,

    /// The reference as it would be written
    pub new_reference: String,
}

/// Execute the migrate-ids command
///
/// # Arguments
///
/// * `args` - Migrate-ids command arguments
///
/// # Returns
///
/// Exit code: 0 (nothing pending, or rewrite succeeded), 1 (renames are
/// pending and nothing was written yet)
///
/// # Errors
///
/// Returns error if the project root cannot be determined, the index cannot be
/// opened, or a file cannot be read or written.
pub fn execute(args: &MigrateIdsArgs) -> Result<i32> {
    let theme = if args.format == "json" {
        None
    } else {
        CliTheme::load(None, !args.no_color)?
    };

    let project_root = if let Some(root) = &args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    let db_path = project_root.join(".lash").join("lash.db");
    if !db_path.exists() {
        return report_no_pending_index(args, theme.as_ref());
    }

    let conn = open_database(&db_path).context("Failed to open the index")?;
    let migrations = IdMigrationRepository::new(&conn);
    let renames = migrations.list_pending()?;

    if renames.is_empty() {
        return report_nothing_pending(args, theme.as_ref());
    }

    if args.forget {
        migrations.clear_all()?;
        return report_forgotten(args, renames.len(), theme.as_ref());
    }

    let rewrites = find_rewrites(&project_root, &renames)?;

    if args.write {
        apply_rewrites(&project_root, &rewrites)?;
        migrations.clear_all()?;
        drop(conn);

        // The rewritten references have to be re-resolved before anything
        // queries them, or `lash list --blocked` and the dependency graph keep
        // answering from edges built against IDs that no longer appear
        // anywhere.
        reindex(&db_path, &project_root)?;
    }

    output(args, &renames, &rewrites, theme.as_ref())?;

    // Pending renames are unfinished work, and an exit code is how a script
    // notices. Once they are written they are done, so 0.
    Ok(i32::from(!args.write))
}

/// Re-index the project after rewriting references
///
/// Opens its own connection so the caller's can be dropped first: the rewrite
/// changed files on disk, and the index has to be rebuilt from what is there
/// now, not from what the calling connection last read.
fn reindex(db_path: &Path, project_root: &Path) -> Result<()> {
    let conn = open_database(db_path).context("Failed to reopen the index")?;
    let config = LashConfig::from_root(project_root).unwrap_or_default();
    let indexer_config = IndexerConfig::new(project_root.to_path_buf())
        .with_incremental(true)
        .with_progress(false);
    let mut indexer = Indexer::new(&conn, indexer_config, &config);
    indexer
        .index_project()
        .context("Failed to re-index after rewriting references")?;
    Ok(())
}

/// Every reference in the project that one of `renames` applies to
///
/// Only `@depends-on:` annotation lines are considered, and only whole
/// comma-separated references on them. Prose that happens to contain an old ID
/// is left alone: it is not a reference, and rewriting it would be editing
/// someone's notes.
///
/// # Errors
///
/// Returns error if the project cannot be walked or a file cannot be read.
pub fn find_rewrites(
    project_root: &Path,
    renames: &[TaskIdRename],
) -> Result<Vec<ReferenceRewrite>> {
    let lookup = RenameLookup::new(renames);
    let mut rewrites = Vec::new();

    for absolute_path in markdown_files(project_root)? {
        let Ok(content) = std::fs::read_to_string(&absolute_path) else {
            continue;
        };
        let relative_path = absolute_path
            .strip_prefix(project_root)
            .unwrap_or(&absolute_path)
            .to_path_buf();

        for (index, line) in content.lines().enumerate() {
            let Some(value) = depends_on_value(line) else {
                continue;
            };

            for reference in value.split(',') {
                let trimmed = reference.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Some(new_reference) = lookup.rewrite(trimmed, &relative_path) {
                    rewrites.push(ReferenceRewrite {
                        source_path: relative_path.clone(),
                        line_number: index + 1,
                        old_reference: trimmed.to_string(),
                        new_reference,
                    });
                }
            }
        }
    }

    Ok(rewrites)
}

/// The value of a `@depends-on:` annotation, if this line is one
fn depends_on_value(line: &str) -> Option<&str> {
    line.trim_start().strip_prefix("@depends-on:")
}

/// Every Markdown file under the project root
///
/// Deliberately not restricted to files the indexer considers task files: a
/// file with no `## Tasks` section of its own can still carry a `@depends-on`
/// pointing at one that does.
fn markdown_files(project_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_markdown(project_root, &mut files)?;
    files.sort();
    Ok(files)
}

/// Recursive half of [`markdown_files`]
fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        // `.lash` holds the index, `.git` holds history, and neither contains
        // task references. Other dot-directories are skipped for the same
        // reason `lash index` ignores them.
        if name.starts_with('.') {
            continue;
        }
        if name == "target" || name == "node_modules" {
            continue;
        }

        if path.is_dir() {
            collect_markdown(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }

    Ok(())
}

/// Matches a written reference against the recorded renames
struct RenameLookup<'a> {
    /// Keyed by `(file spelling, old local id)`, where the file spelling is
    /// whatever the reference used to name the file.
    by_qualifier: HashMap<(String, String), &'a TaskIdRename>,

    /// Keyed by old local id, for same-file (`#task:id`) references
    by_local_id: HashMap<String, Vec<&'a TaskIdRename>>,
}

impl<'a> RenameLookup<'a> {
    fn new(renames: &'a [TaskIdRename]) -> Self {
        let mut by_qualifier = HashMap::new();
        let mut by_local_id: HashMap<String, Vec<&'a TaskIdRename>> = HashMap::new();

        for rename in renames {
            for spelling in file_spellings(rename) {
                by_qualifier.insert((spelling, rename.old_local_id.clone()), rename);
            }
            by_local_id
                .entry(rename.old_local_id.clone())
                .or_default()
                .push(rename);
        }

        Self {
            by_qualifier,
            by_local_id,
        }
    }

    /// What `reference`, written in `source_path`, should become — if anything
    ///
    /// Returns `None` for a reference no rename applies to, and for the
    /// unqualified `old-id` form, which is left alone on purpose: a bare token
    /// can name a file as readily as a task, and rewriting one that turned out
    /// to be a file id would break a reference that currently works.
    fn rewrite(&self, reference: &str, source_path: &Path) -> Option<String> {
        let (qualifier, local_part) = reference.split_once('#')?;
        let old_local_id = local_part.strip_prefix("task:").unwrap_or(local_part);
        let keep_task_prefix = local_part.starts_with("task:");

        let rename = if qualifier.trim().is_empty() {
            // `#task:id` — same file as the reference, by definition.
            self.by_local_id
                .get(old_local_id)?
                .iter()
                .find(|rename| rename.file_path == source_path)?
        } else {
            let key = (normalize_spelling(qualifier), old_local_id.to_string());
            self.by_qualifier.get(&key)?
        };

        let new_local = &rename.new_local_id;
        Some(if keep_task_prefix {
            format!("{qualifier}#task:{new_local}")
        } else {
            format!("{qualifier}#{new_local}")
        })
    }
}

/// Every way a reference could have spelled the file a rename belongs to
///
/// `lash show` prints the file's `@id`; `@depends-on` is documented as a path.
/// Both forms are in the wild, often in the same project.
fn file_spellings(rename: &TaskIdRename) -> Vec<String> {
    let path = rename.file_path.to_string_lossy().replace('\\', "/");
    let mut spellings = vec![
        normalize_spelling(&rename.file_id),
        normalize_spelling(&path),
    ];

    if let Some(stem) = path.strip_suffix(".md") {
        spellings.push(normalize_spelling(stem));
    }
    if let Some(name) = rename.file_path.file_name() {
        spellings.push(normalize_spelling(&name.to_string_lossy()));
    }

    spellings.sort();
    spellings.dedup();
    spellings
}

/// Fold a file spelling to a comparable form
fn normalize_spelling(spelling: &str) -> String {
    spelling
        .trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .to_lowercase()
}

/// Rewrite the references in place
///
/// Each file is read, edited and written once. Only the exact reference tokens
/// found by [`find_rewrites`] are replaced, on the lines they were found on.
fn apply_rewrites(project_root: &Path, rewrites: &[ReferenceRewrite]) -> Result<()> {
    let mut by_file: HashMap<&PathBuf, Vec<&ReferenceRewrite>> = HashMap::new();
    for rewrite in rewrites {
        by_file
            .entry(&rewrite.source_path)
            .or_default()
            .push(rewrite);
    }

    for (relative_path, file_rewrites) in by_file {
        let absolute_path = project_root.join(relative_path);
        let content = std::fs::read_to_string(&absolute_path)
            .with_context(|| format!("Failed to read {}", absolute_path.display()))?;

        let ends_with_newline = content.ends_with('\n');
        let mut lines: Vec<String> = content.lines().map(String::from).collect();

        for rewrite in file_rewrites {
            let Some(line) = lines.get_mut(rewrite.line_number - 1) else {
                continue;
            };
            *line = replace_reference(line, &rewrite.old_reference, &rewrite.new_reference);
        }

        let mut updated = lines.join("\n");
        if ends_with_newline {
            updated.push('\n');
        }

        std::fs::write(&absolute_path, updated)
            .with_context(|| format!("Failed to write {}", absolute_path.display()))?;
    }

    Ok(())
}

/// Replace one whole reference on a `@depends-on:` line
///
/// Splits on commas and swaps the matching token rather than doing a substring
/// replace, so a reference that is a prefix of another (`a#task-1` inside
/// `a#task-10`) cannot be corrupted, and the line's spacing survives.
fn replace_reference(line: &str, old_reference: &str, new_reference: &str) -> String {
    let Some((prefix, value)) = line.split_once("@depends-on:") else {
        return line.to_string();
    };

    let rewritten: Vec<String> = value
        .split(',')
        .map(|part| {
            if part.trim() == old_reference {
                part.replace(old_reference, new_reference)
            } else {
                part.to_string()
            }
        })
        .collect();

    format!("{prefix}@depends-on:{}", rewritten.join(","))
}

/// Report when there is no index to read renames from
#[allow(clippy::unnecessary_wraps)]
fn report_no_pending_index(args: &MigrateIdsArgs, theme: Option<&CliTheme>) -> Result<i32> {
    if args.format == "json" {
        println!(
            "{}",
            serde_json::json!({ "pending_renames": 0, "rewrites": [], "written": false })
        );
    } else if let Some(t) = theme {
        println!(
            "{} run `lash index` first.",
            t.style_info("No index found;")
        );
    } else {
        println!("No index found; run `lash index` first.");
    }
    Ok(0)
}

/// Report when nothing is pending
#[allow(clippy::unnecessary_wraps)]
fn report_nothing_pending(args: &MigrateIdsArgs, theme: Option<&CliTheme>) -> Result<i32> {
    if args.format == "json" {
        println!(
            "{}",
            serde_json::json!({ "pending_renames": 0, "rewrites": [], "written": false })
        );
    } else if let Some(t) = theme {
        println!("{}", t.style_success("No task IDs are pending migration."));
    } else {
        println!("No task IDs are pending migration.");
    }
    Ok(0)
}

/// Report a `--forget`
#[allow(clippy::unnecessary_wraps)]
fn report_forgotten(args: &MigrateIdsArgs, count: usize, theme: Option<&CliTheme>) -> Result<i32> {
    if args.format == "json" {
        println!(
            "{}",
            serde_json::json!({ "pending_renames": count, "rewrites": [], "forgotten": true })
        );
    } else {
        let message = format!("Discarded {count} pending rename(s) without rewriting anything.");
        if let Some(t) = theme {
            println!("{}", t.style_warning(&message));
        } else {
            println!("{message}");
        }
    }
    Ok(0)
}

/// Report the renames and the references they apply to
fn output(
    args: &MigrateIdsArgs,
    renames: &[TaskIdRename],
    rewrites: &[ReferenceRewrite],
    theme: Option<&CliTheme>,
) -> Result<()> {
    if args.format == "json" {
        let json = serde_json::json!({
            "pending_renames": renames.len(),
            "renames": renames.iter().map(|r| serde_json::json!({
                "file": r.file_path.display().to_string(),
                "old_id": r.old_full_id(),
                "new_id": r.new_full_id(),
                "title": r.title,
            })).collect::<Vec<_>>(),
            "rewrites": rewrites.iter().map(|r| serde_json::json!({
                "file": r.source_path.display().to_string(),
                "line": r.line_number,
                "old_reference": r.old_reference,
                "new_reference": r.new_reference,
            })).collect::<Vec<_>>(),
            "written": args.write,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    let heading = if args.write {
        format!("Migrated {} task ID(s):", renames.len())
    } else {
        format!(
            "{} task ID(s) changed when the index was re-derived:",
            renames.len()
        )
    };
    if let Some(t) = theme {
        println!("{}", t.style_warning(&heading));
    } else {
        println!("{heading}");
    }

    for rename in renames {
        println!(
            "  {} → {}   ({})",
            rename.old_full_id(),
            rename.new_full_id(),
            rename.title
        );
    }

    println!();
    if rewrites.is_empty() {
        println!("No `@depends-on` reference uses an old ID, so nothing needs rewriting.");
        if !args.write {
            println!("Run `lash migrate-ids --write` to clear the pending list.");
        }
    } else {
        let verb = if args.write {
            "Rewrote"
        } else {
            "Would rewrite"
        };
        println!("{verb} {} reference(s):", rewrites.len());
        for rewrite in rewrites {
            println!(
                "  {}:{}  {} → {}",
                rewrite.source_path.display(),
                rewrite.line_number,
                rewrite.old_reference,
                rewrite.new_reference
            );
        }
        if !args.write {
            println!();
            println!("Nothing has been written. Run `lash migrate-ids --write` to apply.");
        }
    }

    // A reference lash cannot see is one it cannot fix, and staying quiet
    // about that would leave the author believing the migration was complete.
    if !args.write {
        println!();
        println!("Unqualified references (a bare `old-id` with no `file#`) are not rewritten:");
        println!("a bare token can name a file as readily as a task. Check those by hand");
        println!("with `lash lint` after migrating.");
    }

    Ok(())
}

/// What an ID-drift annotation pass found
///
/// Returned so the caller can say something once, at normal verbosity, rather
/// than relying on per-diagnostic help that only appears under `-v`. This
/// particular note is the difference between "lint is wrong" and "here is what
/// happened", so it has to be visible by default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IdDriftNotice {
    /// References explained by a rename already recorded and awaiting a rewrite
    pub pending_migration: usize,

    /// References explained by an index that has not been re-derived yet
    pub stale_index: usize,
}

impl IdDriftNotice {
    /// The one-line advice that follows from what was found
    #[must_use]
    pub fn advice(self) -> Option<&'static str> {
        if self.stale_index > 0 {
            // Re-deriving comes first: it is what produces the rename records
            // that `migrate-ids` then works from.
            Some("Run `lash index` to re-derive them, then `lash migrate-ids --write`.")
        } else if self.pending_migration > 0 {
            Some("Run `lash migrate-ids --write` to update these references.")
        } else {
            None
        }
    }
}

/// Explain unresolved references that are the signature of an ID drift
///
/// An `E_LINK_NOT_FOUND` whose target the index still recognises is not an
/// ordinary broken link — it is a reference that was correct when it was
/// written and stopped being correct because the derivation rules moved
/// underneath it. Without saying so, the error reads as a false positive: the
/// ID it names is exactly the one `lash show` prints back.
///
/// Appends to the `help` of any matching diagnostic and reports what it found.
/// Silently does nothing when there is no index to consult, since lint works
/// fine without one.
pub fn annotate_id_drift(diagnostics: &mut [Diagnostic], project_root: &Path) -> IdDriftNotice {
    let unresolved: Vec<String> = diagnostics
        .iter()
        .filter(|d| d.code == "E_LINK_NOT_FOUND")
        .filter_map(|d| quoted_task_id(&d.message))
        .collect();
    if unresolved.is_empty() {
        return IdDriftNotice::default();
    }

    let db_path = project_root.join(".lash").join("lash.db");
    let Ok(conn) = open_database(&db_path) else {
        return IdDriftNotice::default();
    };

    let pending: HashMap<String, String> = IdMigrationRepository::new(&conn)
        .list_pending()
        .unwrap_or_default()
        .into_iter()
        .map(|r| (r.old_local_id.clone(), r.new_full_id()))
        .collect();

    let stored: std::collections::HashSet<String> = TaskRepository::new(&conn)
        .get_all_local_ids()
        .unwrap_or_default()
        .into_iter()
        .collect();

    let mut notice = IdDriftNotice::default();

    for diagnostic in diagnostics
        .iter_mut()
        .filter(|d| d.code == "E_LINK_NOT_FOUND")
    {
        let Some(task_id) = quoted_task_id(&diagnostic.message) else {
            continue;
        };

        let note = if let Some(new_full_id) = pending.get(&task_id) {
            notice.pending_migration += 1;
            format!(
                "'{task_id}' was renamed to '{new_full_id}' when the task-ID derivation \
                 rules changed. Run `lash migrate-ids --write` to update references."
            )
        } else if stored.contains(&task_id) {
            notice.stale_index += 1;
            format!(
                "The index still stores '{task_id}', so `lash show` prints it while this \
                 check rejects it — the index was built under older ID rules. Run \
                 `lash index` to re-derive, then `lash migrate-ids`."
            )
        } else {
            continue;
        };

        diagnostic.help = Some(match diagnostic.help.take() {
            Some(existing) => format!("{existing}\n{note}"),
            None => note,
        });
    }

    notice
}

/// The task ID an `E_LINK_NOT_FOUND` message names
///
/// The message reads `Task 'some-id' not found in file 'some-file'`, so the
/// first quoted run is the ID.
fn quoted_task_id(message: &str) -> Option<String> {
    let (_, rest) = message.split_once('\'')?;
    let (id, _) = rest.split_once('\'')?;
    Some(id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rename(file: &str, file_id: &str, old: &str, new: &str) -> TaskIdRename {
        TaskIdRename {
            file_path: PathBuf::from(file),
            file_id: file_id.to_string(),
            old_local_id: old.to_string(),
            new_local_id: new.to_string(),
            title: "A task".to_string(),
        }
    }

    #[test]
    fn test_rewrites_a_reference_qualified_with_the_file_id() {
        let renames = vec![rename("tasks.md", "tasks", "old-id", "new-id")];
        let lookup = RenameLookup::new(&renames);

        assert_eq!(
            lookup.rewrite("tasks#old-id", Path::new("other.md")),
            Some("tasks#new-id".to_string())
        );
    }

    #[test]
    fn test_rewrites_a_reference_qualified_with_the_path() {
        // The documented `@depends-on` spelling.
        let renames = vec![rename("tasks.md", "tasks", "old-id", "new-id")];
        let lookup = RenameLookup::new(&renames);

        assert_eq!(
            lookup.rewrite("tasks.md#task:old-id", Path::new("other.md")),
            Some("tasks.md#task:new-id".to_string())
        );
    }

    #[test]
    fn test_keeps_the_task_prefix_it_found() {
        // Rewriting `#task:id` as `#id` would be a second, gratuitous change
        // to a line the author wrote.
        let renames = vec![rename("tasks.md", "tasks", "old-id", "new-id")];
        let lookup = RenameLookup::new(&renames);

        assert_eq!(
            lookup.rewrite("tasks#task:old-id", Path::new("other.md")),
            Some("tasks#task:new-id".to_string())
        );
        assert_eq!(
            lookup.rewrite("tasks#old-id", Path::new("other.md")),
            Some("tasks#new-id".to_string())
        );
    }

    #[test]
    fn test_same_file_reference_matches_only_within_that_file() {
        // `#task:id` means "in this file", so the same text in another file
        // refers to a different task and must not be touched.
        let renames = vec![rename("tasks.md", "tasks", "old-id", "new-id")];
        let lookup = RenameLookup::new(&renames);

        assert_eq!(
            lookup.rewrite("#task:old-id", Path::new("tasks.md")),
            Some("#task:new-id".to_string())
        );
        assert_eq!(lookup.rewrite("#task:old-id", Path::new("other.md")), None);
    }

    #[test]
    fn test_leaves_a_bare_id_alone() {
        // A bare token can name a file, and rewriting one that did would break
        // a reference that currently resolves.
        let renames = vec![rename("tasks.md", "tasks", "old-id", "new-id")];
        let lookup = RenameLookup::new(&renames);

        assert_eq!(lookup.rewrite("old-id", Path::new("tasks.md")), None);
    }

    #[test]
    fn test_leaves_an_unrelated_reference_alone() {
        let renames = vec![rename("tasks.md", "tasks", "old-id", "new-id")];
        let lookup = RenameLookup::new(&renames);

        assert_eq!(lookup.rewrite("tasks#other-id", Path::new("a.md")), None);
        assert_eq!(lookup.rewrite("elsewhere#old-id", Path::new("a.md")), None);
    }

    #[test]
    fn test_nested_paths_match_by_id_and_by_path() {
        let renames = vec![rename(
            "area/backend.md",
            "area.backend",
            "old-id",
            "new-id",
        )];
        let lookup = RenameLookup::new(&renames);

        for spelling in [
            "area.backend#old-id",
            "area/backend.md#task:old-id",
            "area/backend#old-id",
        ] {
            assert!(
                lookup.rewrite(spelling, Path::new("a.md")).is_some(),
                "expected '{spelling}' to match"
            );
        }
    }

    #[test]
    fn test_replace_reference_leaves_the_rest_of_the_line_intact() {
        let line = "  @depends-on: a#one, tasks#old-id, b#two";
        assert_eq!(
            replace_reference(line, "tasks#old-id", "tasks#new-id"),
            "  @depends-on: a#one, tasks#new-id, b#two"
        );
    }

    #[test]
    fn test_replace_reference_does_not_corrupt_a_prefix_match() {
        // A substring replace would turn `tasks#old-id-2` into
        // `tasks#new-id-2`, silently repointing a task nobody renamed.
        let line = "@depends-on: tasks#old-id-2";
        assert_eq!(
            replace_reference(line, "tasks#old-id", "tasks#new-id"),
            "@depends-on: tasks#old-id-2"
        );
    }

    #[test]
    fn test_replace_reference_ignores_a_line_without_the_annotation() {
        let line = "- [ ] A task mentioning tasks#old-id in prose";
        assert_eq!(
            replace_reference(line, "tasks#old-id", "tasks#new-id"),
            line
        );
    }

    #[test]
    fn test_depends_on_value_requires_the_annotation() {
        assert_eq!(depends_on_value("  @depends-on: a, b"), Some(" a, b"));
        assert_eq!(depends_on_value("- [ ] not an annotation"), None);
        assert_eq!(depends_on_value("@doc: something.md"), None);
    }
}
