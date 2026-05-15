//! Skill installation: write Lash agent skills to a coding-agent's
//! conventional skills directory (`.claude/skills/lash/`, etc.).
//!
//! The installer is pure with respect to content generation — see
//! [`generate_files`] — so callers can `--dry-run` or `--print` without
//! touching the filesystem. Idempotency markers (a `lash-skill-version` field
//! embedded in every generated file) let [`install`] tell user-edited files
//! from generated ones and refuse to overwrite without `--force`.
//!
//! All four targets ([`Target::Claude`], [`Target::Codex`], [`Target::Cursor`],
//! [`Target::AgentsMd`]) are implemented. Claude uses a progressive-disclosure
//! layout (`SKILL.md` plus `references/*.md`). The other three are single-file
//! formats sitting at conventional locations under the install root.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Version stamped into every generated skill file's idempotency marker.
///
/// Bumped when the skill format changes in a way that requires re-installation;
/// distinct from the lash binary's `CARGO_PKG_VERSION` so docs can ship a fix
/// without forcing every user to re-run `lash skill install`.
pub const SKILL_FORMAT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Coding-agent target for skill installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// Claude Code skill (progressive-disclosure tree under `.claude/skills/lash/`).
    Claude,
    /// Codex / `OpenAI` agent (single-file `AGENTS.lash.md` sibling).
    Codex,
    /// Cursor IDE rule (`.cursor/rules/lash.mdc`).
    Cursor,
    /// Generic single-file fragment suitable for inclusion in an `AGENTS.md`.
    AgentsMd,
}

impl Target {
    /// Human-readable name of the target (kebab-case, matches the CLI flag value).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::AgentsMd => "agents-md",
        }
    }
}

/// Where to install — project-local or per-user global.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Inside the current project (`<root>/.claude/skills/lash/`).
    Project,
    /// In the user's home directory (`~/.claude/skills/lash/`).
    User,
}

/// Errors produced by the installer.
#[derive(Debug, thiserror::Error)]
pub enum InstallerError {
    /// Target is not yet implemented.
    #[error("target '{0}' is not yet implemented")]
    TargetNotImplemented(&'static str),

    /// User-scope install was requested but the host environment has no usable
    /// home directory (e.g. running under a service account without `$HOME`).
    #[error("user-scope install requires a home directory, but none was resolvable")]
    NoHomeDir,

    /// A pre-existing file does not look like a Lash-generated artifact and
    /// `--force` was not specified.
    #[error("refusing to overwrite user-edited file: {0} (re-run with --force to replace)")]
    UserEditedFile(PathBuf),

    /// Filesystem error.
    #[error("filesystem error at {path}: {source}")]
    Io {
        /// Path that triggered the I/O failure.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Options for an install run.
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Coding-agent target.
    pub target: Target,
    /// Project-local vs per-user.
    pub scope: Scope,
    /// Project root (used for `Scope::Project`).
    pub project_root: PathBuf,
    /// User home directory (used for `Scope::User`). Resolve via `dirs::home_dir()`
    /// at the call site so this crate stays testable without env mutation.
    pub home_dir: Option<PathBuf>,
    /// Overwrite files even if they appear to be user-edited.
    pub force: bool,
    /// Compute the plan but do not write files. Mutually exclusive with `print_to`.
    pub dry_run: bool,
}

/// A single file the installer would emit, with its absolute path.
#[derive(Debug, Clone)]
pub struct SkillFile {
    /// Where the file will be written.
    pub path: PathBuf,
    /// The file contents (already including the idempotency marker).
    pub content: String,
}

/// Per-file action chosen by the installer for a given run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction {
    /// File did not exist; we will write it.
    Created,
    /// File existed and carried our marker; we overwrote with refreshed content.
    Updated,
    /// File existed but carried our marker and content was identical — no write.
    Unchanged,
    /// File existed without our marker; left alone unless `--force` is set.
    Skipped,
    /// File existed without our marker; `--force` overwrote it.
    Overwritten,
}

/// Outcome for one file in an install run.
#[derive(Debug, Clone)]
pub struct FileOutcome {
    /// File metadata and intended content.
    pub file: SkillFile,
    /// What happened (or would happen, for `dry_run`).
    pub action: FileAction,
}

/// Aggregate report of an install run.
#[derive(Debug, Clone)]
pub struct InstallReport {
    /// Target installed for.
    pub target: Target,
    /// Root directory the skill was written into.
    pub install_root: PathBuf,
    /// Per-file outcomes, in the order returned by [`generate_files`].
    pub files: Vec<FileOutcome>,
    /// Whether this was a dry run (no writes performed).
    pub dry_run: bool,
}

impl InstallReport {
    /// True if at least one file was skipped due to user edits.
    #[must_use]
    pub fn has_skipped_user_edits(&self) -> bool {
        self.files.iter().any(|f| f.action == FileAction::Skipped)
    }

    /// Count of files in each `FileAction` category.
    #[must_use]
    pub fn action_counts(&self) -> BTreeMap<&'static str, usize> {
        let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        for outcome in &self.files {
            let key = match outcome.action {
                FileAction::Created => "created",
                FileAction::Updated => "updated",
                FileAction::Unchanged => "unchanged",
                FileAction::Skipped => "skipped",
                FileAction::Overwritten => "overwritten",
            };
            *counts.entry(key).or_insert(0) += 1;
        }
        counts
    }
}

/// Resolve the directory the skill will be installed into.
///
/// # Errors
///
/// Returns [`InstallerError::NoHomeDir`] if `Scope::User` is requested and no
/// home directory is available.
pub fn install_root(
    target: Target,
    scope: Scope,
    project_root: &Path,
    home_dir: Option<&Path>,
) -> Result<PathBuf, InstallerError> {
    let base = match scope {
        Scope::Project => project_root.to_path_buf(),
        Scope::User => home_dir.ok_or(InstallerError::NoHomeDir)?.to_path_buf(),
    };
    Ok(match target {
        Target::Claude => base.join(".claude").join("skills").join("lash"),
        Target::Codex | Target::Cursor | Target::AgentsMd => base,
    })
}

/// Generate the files the given target would install, with their content but
/// without writing anything to disk.
///
/// Paths are relative to the install root returned by [`install_root`]. Pure
/// function — safe to call repeatedly and from tests.
///
/// # Errors
///
/// Returns [`InstallerError::TargetNotImplemented`] only when a target is
/// added to the enum without a corresponding generator. All current targets
/// produce files.
pub fn generate_files(target: Target) -> Result<Vec<(PathBuf, String)>, InstallerError> {
    match target {
        Target::Claude => Ok(claude::generate()),
        Target::Codex | Target::AgentsMd => Ok(single_file_agents_md::generate()),
        Target::Cursor => Ok(cursor::generate()),
    }
}

/// Substring required to be present in any generated file for the installer to
/// treat it as "lash-generated" and safe to overwrite without `--force`.
///
/// Per-target generators embed this key either directly inside YAML
/// frontmatter (for SKILL.md and Cursor `.mdc`) or via [`marker_comment`] for
/// files that don't naturally take frontmatter.
pub const IDEMPOTENCY_MARKER_KEY: &str = "lash-skill-version";

/// HTML comment marker for files that don't naturally take YAML frontmatter.
///
/// Tests assert this string contains [`IDEMPOTENCY_MARKER_KEY`].
#[must_use]
pub fn marker_comment() -> String {
    format!(
        "<!-- {IDEMPOTENCY_MARKER_KEY}: {SKILL_FORMAT_VERSION} — generated by `lash skill install`; do not edit by hand -->"
    )
}

/// Detect whether `content` was generated by a prior `lash skill install`.
#[must_use]
pub fn is_lash_generated(content: &str) -> bool {
    content.contains(IDEMPOTENCY_MARKER_KEY)
}

/// Plan an install run without touching the filesystem.
///
/// Returns the same [`InstallReport`] [`install`] would produce, except that
/// `dry_run` is forced `true` and no I/O occurs beyond reading existing files
/// to decide each file's [`FileAction`].
///
/// # Errors
///
/// See [`install`].
pub fn plan(opts: &InstallOptions) -> Result<InstallReport, InstallerError> {
    let mut probe = opts.clone();
    probe.dry_run = true;
    install(&probe)
}

/// Install the skill onto disk (or compute the plan if `opts.dry_run`).
///
/// # Errors
///
/// - [`InstallerError::TargetNotImplemented`] if the target's generator isn't built yet
/// - [`InstallerError::NoHomeDir`] for user-scope without a resolvable home
/// - [`InstallerError::UserEditedFile`] if a file exists without our marker and `force` is false
/// - [`InstallerError::Io`] for any filesystem failure
pub fn install(opts: &InstallOptions) -> Result<InstallReport, InstallerError> {
    let root = install_root(
        opts.target,
        opts.scope,
        &opts.project_root,
        opts.home_dir.as_deref(),
    )?;
    let files = generate_files(opts.target)?;

    let mut outcomes = Vec::with_capacity(files.len());

    for (rel_path, content) in files {
        let abs_path = root.join(&rel_path);
        let action = decide_action(&abs_path, &content, opts.force)?;

        if !opts.dry_run {
            apply_action(&abs_path, &content, action)?;
        }

        outcomes.push(FileOutcome {
            file: SkillFile {
                path: abs_path,
                content,
            },
            action,
        });
    }

    Ok(InstallReport {
        target: opts.target,
        install_root: root,
        files: outcomes,
        dry_run: opts.dry_run,
    })
}

fn decide_action(
    abs_path: &Path,
    new_content: &str,
    force: bool,
) -> Result<FileAction, InstallerError> {
    match fs::read_to_string(abs_path) {
        Ok(existing) => {
            if existing == new_content {
                Ok(FileAction::Unchanged)
            } else if is_lash_generated(&existing) {
                Ok(FileAction::Updated)
            } else if force {
                Ok(FileAction::Overwritten)
            } else {
                Ok(FileAction::Skipped)
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(FileAction::Created),
        Err(source) => Err(InstallerError::Io {
            path: abs_path.to_path_buf(),
            source,
        }),
    }
}

fn apply_action(abs_path: &Path, content: &str, action: FileAction) -> Result<(), InstallerError> {
    match action {
        FileAction::Created | FileAction::Updated | FileAction::Overwritten => {
            write_file(abs_path, content)
        }
        FileAction::Unchanged | FileAction::Skipped => Ok(()),
    }
}

fn write_file(abs_path: &Path, content: &str) -> Result<(), InstallerError> {
    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent).map_err(|source| InstallerError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(abs_path, content).map_err(|source| InstallerError::Io {
        path: abs_path.to_path_buf(),
        source,
    })
}

/// Claude Code target: progressive disclosure under `<root>/.claude/skills/lash/`.
mod claude {
    use super::{IDEMPOTENCY_MARKER_KEY, SKILL_FORMAT_VERSION};
    use crate::content;
    use std::path::PathBuf;

    pub fn generate() -> Vec<(PathBuf, String)> {
        vec![
            (PathBuf::from("SKILL.md"), skill_md()),
            (
                PathBuf::from("references").join("commands.md"),
                wrap_reference("Lash CLI Reference", content::cli_reference()),
            ),
            (
                PathBuf::from("references").join("workflow.md"),
                wrap_reference("Lash Workflow", content::workflow()),
            ),
            (
                PathBuf::from("references").join("dependencies.md"),
                wrap_reference_raw(content::dependencies_reference()),
            ),
            (
                PathBuf::from("references").join("errors.md"),
                wrap_reference("Lash Error Recovery", content::error_recovery()),
            ),
            (
                PathBuf::from("references").join("safety.md"),
                wrap_reference("Lash Safety Guidelines", content::safety_guidelines()),
            ),
        ]
    }

    fn skill_md() -> String {
        let description = content::when_to_use();
        let hot = content::hot_commands();
        format!(
            "---\n\
             name: lash\n\
             description: {description}\n\
             {IDEMPOTENCY_MARKER_KEY}: {SKILL_FORMAT_VERSION}\n\
             ---\n\
             \n\
             # Lash skill\n\
             \n\
             Lash tracks tasks as Markdown checkboxes with annotations. Markdown is\n\
             the source of truth; SQLite is an index.\n\
             \n\
             ## When to use\n\
             \n\
             - The user asks to add, list, find, complete, or update tasks\n\
             - A `tasks/` directory, `lash.index.md`, or `.lash/` directory exists\n\
             - You are working with task hierarchies, labels, or dependencies\n\
             \n\
             ## Hot commands\n\
             \n\
             {hot}\n\
             ## Always\n\
             \n\
             1. Run `lash lint <path>` after editing any task file\n\
             2. Run `lash index` after structural changes (new files, renames)\n\
             3. For live project task state in your context, run `lash agent-prompt`\n\
                rather than reading raw task files\n\
             \n\
             ## When you need more\n\
             \n\
             Each topic has a dedicated reference loaded on demand:\n\
             \n\
             - Full command surface: `references/commands.md`\n\
             - Discover → read → modify → validate workflow: `references/workflow.md`\n\
             - `@depends-on`, `@doc`, fragment slugs: `references/dependencies.md`\n\
             - Lint error codes & recovery: `references/errors.md`\n\
             - Depth limits, ID uniqueness, status rules: `references/safety.md`\n"
        )
    }

    fn wrap_reference(heading: &str, body: &str) -> String {
        // Strip the leading `## ` heading if present so we can render a top-level
        // `# Heading` here. The content::* helpers use `##` because they're
        // embedded inside a larger prompt.
        let stripped = strip_leading_h2(body);
        format!(
            "{marker}\n\n# {heading}\n\n{stripped}",
            marker = super::marker_comment()
        )
    }

    /// Wrap a body that already starts with its own `#` heading.
    fn wrap_reference_raw(body: &str) -> String {
        format!("{marker}\n\n{body}", marker = super::marker_comment())
    }

    fn strip_leading_h2(body: &str) -> String {
        // If body begins with `## Some Heading\n\n`, drop that line and the
        // following blank line.
        let mut lines = body.lines();
        if let Some(first) = lines.next() {
            if let Some(rest) = first.strip_prefix("## ") {
                // Skip a single blank line after the heading.
                let _ = rest; // heading text intentionally ignored
                let mut iter = lines.peekable();
                if iter.peek().is_some_and(|s| s.is_empty()) {
                    iter.next();
                }
                return iter.collect::<Vec<_>>().join("\n") + "\n";
            }
        }
        body.to_string()
    }
}

/// Codex / generic AGENTS.md target: single self-contained Markdown file at
/// the project root, written as `AGENTS.lash.md` to avoid clobbering a
/// user-authored `AGENTS.md`.
mod single_file_agents_md {
    use crate::content;
    use std::path::PathBuf;

    pub fn generate() -> Vec<(PathBuf, String)> {
        vec![(PathBuf::from("AGENTS.lash.md"), body())]
    }

    fn body() -> String {
        format!(
            "{marker}\n\
             \n\
             # Lash — Markdown-Native Task Tracker (for agents)\n\
             \n\
             {when_to_use}\n\
             \n\
             To wire this into your agent setup, reference `AGENTS.lash.md` from\n\
             your project's `AGENTS.md` (e.g. add `See @AGENTS.lash.md.` or copy\n\
             a link to it) so the agent loads this guide.\n\
             \n\
             {overview}\
             {project_structure}\
             ## Hot Commands\n\
             \n\
             {hot}\n\
             {cli_ref}\
             {workflow}\
             {safety}\
             {errors}\
             {deps}",
            marker = super::marker_comment(),
            when_to_use = content::when_to_use(),
            overview = content::overview(),
            project_structure = content::project_structure(),
            hot = content::hot_commands(),
            cli_ref = content::cli_reference(),
            workflow = content::workflow(),
            safety = content::safety_guidelines(),
            errors = content::error_recovery(),
            deps = strip_leading_h1(content::dependencies_reference()),
        )
    }

    /// `content::dependencies_reference()` starts with `# Dependencies...`.
    /// When inlining it under an existing `# Lash` document we want a `##`
    /// heading instead, so promote-by-demote here.
    fn strip_leading_h1(body: &str) -> String {
        if let Some(rest) = body.strip_prefix("# ") {
            format!("## {rest}")
        } else {
            body.to_string()
        }
    }
}

/// Cursor IDE target: `.cursor/rules/lash.mdc` — a single MDC file with
/// Cursor-specific frontmatter declaring when the rule applies.
mod cursor {
    use super::{IDEMPOTENCY_MARKER_KEY, SKILL_FORMAT_VERSION};
    use crate::content;
    use std::path::PathBuf;

    pub fn generate() -> Vec<(PathBuf, String)> {
        let mut path = PathBuf::from(".cursor");
        path.push("rules");
        path.push("lash.mdc");
        vec![(path, body())]
    }

    fn body() -> String {
        let description = content::when_to_use();
        format!(
            "---\n\
             description: {description}\n\
             globs:\n\
             \x20\x20- \"**/*.md\"\n\
             \x20\x20- \"tasks/**\"\n\
             \x20\x20- \"lash.index.md\"\n\
             alwaysApply: false\n\
             {IDEMPOTENCY_MARKER_KEY}: {SKILL_FORMAT_VERSION}\n\
             ---\n\
             \n\
             # Lash Task Tracker\n\
             \n\
             {when_to_use}\n\
             \n\
             ## Hot Commands\n\
             \n\
             {hot}\n\
             {cli_ref}\
             {safety}\
             {errors}",
            when_to_use = content::when_to_use(),
            hot = content::hot_commands(),
            cli_ref = content::cli_reference(),
            safety = content::safety_guidelines(),
            errors = content::error_recovery(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn opts(target: Target, root: &Path) -> InstallOptions {
        InstallOptions {
            target,
            scope: Scope::Project,
            project_root: root.to_path_buf(),
            home_dir: None,
            force: false,
            dry_run: false,
        }
    }

    #[test]
    fn target_names_round_trip() {
        for t in [
            Target::Claude,
            Target::Codex,
            Target::Cursor,
            Target::AgentsMd,
        ] {
            assert!(!t.name().is_empty());
        }
    }

    #[test]
    fn install_root_claude_project() {
        let root =
            install_root(Target::Claude, Scope::Project, Path::new("/tmp/proj"), None).unwrap();
        assert_eq!(root, PathBuf::from("/tmp/proj/.claude/skills/lash"));
    }

    #[test]
    fn install_root_claude_user_requires_home() {
        let err = install_root(Target::Claude, Scope::User, Path::new("/tmp/proj"), None);
        assert!(matches!(err, Err(InstallerError::NoHomeDir)));

        let ok = install_root(
            Target::Claude,
            Scope::User,
            Path::new("/tmp/proj"),
            Some(Path::new("/home/me")),
        )
        .unwrap();
        assert_eq!(ok, PathBuf::from("/home/me/.claude/skills/lash"));
    }

    #[test]
    fn generate_files_claude_produces_expected_layout() {
        let files = generate_files(Target::Claude).unwrap();
        let paths: Vec<&Path> = files.iter().map(|(p, _)| p.as_path()).collect();

        assert!(paths.contains(&Path::new("SKILL.md")));
        assert!(paths.contains(&Path::new("references/commands.md")));
        assert!(paths.contains(&Path::new("references/workflow.md")));
        assert!(paths.contains(&Path::new("references/dependencies.md")));
        assert!(paths.contains(&Path::new("references/errors.md")));
        assert!(paths.contains(&Path::new("references/safety.md")));

        // Every generated file must contain the idempotency marker.
        for (path, content) in &files {
            assert!(
                is_lash_generated(content),
                "{path:?} missing idempotency marker"
            );
        }

        // SKILL.md must contain the trigger description and a hot command sample.
        let skill_md = files
            .iter()
            .find(|(p, _)| p == Path::new("SKILL.md"))
            .map(|(_, c)| c.as_str())
            .unwrap();
        assert!(skill_md.starts_with("---\n"));
        assert!(skill_md.contains("name: lash"));
        assert!(skill_md.contains("lash status"));
        assert!(skill_md.contains("lash agent-prompt"));
    }

    #[test]
    fn generate_files_codex_writes_single_agents_lash_md() {
        let files = generate_files(Target::Codex).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, Path::new("AGENTS.lash.md"));
        assert!(is_lash_generated(&files[0].1));
        assert!(files[0].1.contains("Lash"));
        assert!(files[0].1.contains("lash status"));
        assert!(files[0].1.contains("AGENTS.lash.md"));
    }

    #[test]
    fn generate_files_agents_md_matches_codex() {
        let codex = generate_files(Target::Codex).unwrap();
        let agents_md = generate_files(Target::AgentsMd).unwrap();
        assert_eq!(codex, agents_md, "Codex and AgentsMd share a generator");
    }

    #[test]
    fn generate_files_cursor_writes_mdc_with_frontmatter() {
        let files = generate_files(Target::Cursor).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, Path::new(".cursor/rules/lash.mdc"));
        let content = &files[0].1;
        assert!(content.starts_with("---\n"));
        assert!(content.contains("description: "));
        assert!(content.contains("globs:"));
        assert!(content.contains("alwaysApply:"));
        assert!(is_lash_generated(content));
    }

    #[test]
    fn install_root_for_single_file_targets_is_project_base() {
        for t in [Target::Codex, Target::AgentsMd, Target::Cursor] {
            let root = install_root(t, Scope::Project, Path::new("/tmp/proj"), None).unwrap();
            assert_eq!(root, PathBuf::from("/tmp/proj"));
        }
    }

    #[test]
    fn install_codex_round_trip_creates_and_uninstalls_file() {
        let temp = TempDir::new().unwrap();
        let report = install(&opts(Target::Codex, temp.path())).unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].action, FileAction::Created);
        let path = report.files[0].file.path.clone();
        assert!(path.exists());
        // Second run is a no-op.
        let again = install(&opts(Target::Codex, temp.path())).unwrap();
        assert_eq!(again.files[0].action, FileAction::Unchanged);
    }

    #[test]
    fn install_cursor_creates_nested_directories() {
        let temp = TempDir::new().unwrap();
        let report = install(&opts(Target::Cursor, temp.path())).unwrap();
        let mdc = temp.path().join(".cursor/rules/lash.mdc");
        assert!(mdc.exists());
        assert_eq!(report.files[0].file.path, mdc);
    }

    #[test]
    fn install_creates_files_on_clean_dir() {
        let temp = TempDir::new().unwrap();
        let report = install(&opts(Target::Claude, temp.path())).unwrap();

        assert!(!report.dry_run);
        assert_eq!(report.files.len(), 6);
        for outcome in &report.files {
            assert_eq!(outcome.action, FileAction::Created);
            assert!(
                outcome.file.path.exists(),
                "missing: {:?}",
                outcome.file.path
            );
        }
    }

    #[test]
    fn install_twice_is_unchanged() {
        let temp = TempDir::new().unwrap();
        install(&opts(Target::Claude, temp.path())).unwrap();
        let report = install(&opts(Target::Claude, temp.path())).unwrap();
        for outcome in &report.files {
            assert_eq!(
                outcome.action,
                FileAction::Unchanged,
                "expected Unchanged for {:?}",
                outcome.file.path
            );
        }
    }

    #[test]
    fn install_skips_user_edited_files_without_force() {
        let temp = TempDir::new().unwrap();
        install(&opts(Target::Claude, temp.path())).unwrap();

        // Tamper with SKILL.md — strip the marker so it looks user-authored.
        let skill_path = temp.path().join(".claude/skills/lash/SKILL.md");
        fs::write(&skill_path, "# my hand-written skill\n").unwrap();

        let report = install(&opts(Target::Claude, temp.path())).unwrap();
        let skill_outcome = report
            .files
            .iter()
            .find(|o| o.file.path == skill_path)
            .unwrap();
        assert_eq!(skill_outcome.action, FileAction::Skipped);
        assert!(report.has_skipped_user_edits());

        // Confirm the file is still the user's content.
        assert_eq!(
            fs::read_to_string(&skill_path).unwrap(),
            "# my hand-written skill\n"
        );
    }

    #[test]
    fn install_force_overwrites_user_edited_files() {
        let temp = TempDir::new().unwrap();
        install(&opts(Target::Claude, temp.path())).unwrap();

        let skill_path = temp.path().join(".claude/skills/lash/SKILL.md");
        fs::write(&skill_path, "# my hand-written skill\n").unwrap();

        let mut o = opts(Target::Claude, temp.path());
        o.force = true;
        let report = install(&o).unwrap();
        let skill_outcome = report
            .files
            .iter()
            .find(|out| out.file.path == skill_path)
            .unwrap();
        assert_eq!(skill_outcome.action, FileAction::Overwritten);

        // File has been replaced with generated content.
        let after = fs::read_to_string(&skill_path).unwrap();
        assert!(is_lash_generated(&after));
        assert!(after.contains("name: lash"));
    }

    #[test]
    fn install_updates_lash_generated_files_with_changed_content() {
        let temp = TempDir::new().unwrap();
        install(&opts(Target::Claude, temp.path())).unwrap();

        // Simulate a previous version: keep the marker but change other content.
        let commands_path = temp
            .path()
            .join(".claude/skills/lash/references/commands.md");
        let stale = format!(
            "{}\n\nold content from a previous lash version\n",
            marker_comment()
        );
        fs::write(&commands_path, &stale).unwrap();

        let report = install(&opts(Target::Claude, temp.path())).unwrap();
        let outcome = report
            .files
            .iter()
            .find(|o| o.file.path == commands_path)
            .unwrap();
        assert_eq!(outcome.action, FileAction::Updated);

        let after = fs::read_to_string(&commands_path).unwrap();
        assert!(after.contains("lash lint"));
        assert!(!after.contains("old content from a previous lash version"));
    }

    #[test]
    fn plan_does_not_touch_filesystem() {
        let temp = TempDir::new().unwrap();
        let report = plan(&opts(Target::Claude, temp.path())).unwrap();
        assert!(report.dry_run);
        // Nothing should have been written.
        assert!(!temp.path().join(".claude").exists());
        // But the plan still describes Created actions.
        for outcome in &report.files {
            assert_eq!(outcome.action, FileAction::Created);
        }
    }

    #[test]
    fn marker_comment_contains_marker_key_and_version() {
        let c = marker_comment();
        assert!(c.contains(IDEMPOTENCY_MARKER_KEY));
        assert!(c.contains(SKILL_FORMAT_VERSION));
    }

    #[test]
    fn action_counts_aggregates() {
        let temp = TempDir::new().unwrap();
        let report = install(&opts(Target::Claude, temp.path())).unwrap();
        let counts = report.action_counts();
        assert_eq!(counts.get("created").copied(), Some(6));
    }
}
