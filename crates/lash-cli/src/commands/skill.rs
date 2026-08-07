//! `lash skill` command — install/list/update/uninstall agent skills.
//!
//! Resolves the CLI's `SkillTarget` / `SkillScope` enums into
//! `lash_agent::installer` values, finds the project root, and calls into the
//! installer. Filesystem I/O lives in the installer crate; this module owns
//! user-facing output and exit codes.

use std::path::PathBuf;

use anyhow::{Context, Result};
use lash::cli::{SkillCommand, SkillScope, SkillTarget};
use lash::theme::CliTheme;
use lash_agent::installer::{
    self, FileAction, FileOutcome, InstallOptions, InstallReport, InstallerError, Scope, Target,
};

use crate::utils::file_discovery::find_project_root;

/// Arguments for the `lash skill` command.
#[derive(Debug, Clone)]
pub struct SkillArgs {
    /// Skill subcommand to execute.
    pub command: SkillCommand,
    /// Whether output should be JSON-formatted.
    pub json: bool,
    /// Whether colored output should be disabled.
    pub no_color: bool,
    /// Optional explicit project root override.
    pub project_root: Option<PathBuf>,
}

/// Shared environment for all `lash skill` subcommands.
struct RunCtx<'a> {
    project_root: PathBuf,
    theme: Option<&'a CliTheme>,
    json: bool,
}

/// Resolved install/update arguments.
#[allow(clippy::struct_excessive_bools)]
struct InstallArgs {
    target: SkillTarget,
    scope: SkillScope,
    force: bool,
    dry_run: bool,
    print: bool,
}

/// Execute a `lash skill` subcommand.
///
/// # Errors
///
/// Returns an error if the installer fails or the project root cannot be
/// resolved.
pub fn execute(args: &SkillArgs) -> Result<i32> {
    let theme = if args.json {
        None
    } else {
        CliTheme::load(None, !args.no_color)?
    };

    let project_root = resolve_project_root(args.project_root.as_deref())?;
    let ctx = RunCtx {
        project_root,
        theme: theme.as_ref(),
        json: args.json,
    };

    match args.command.clone() {
        SkillCommand::Install {
            target,
            scope,
            force,
            dry_run,
            print,
        } => Ok(run_install(
            &InstallArgs {
                target,
                scope,
                force,
                dry_run,
                print,
            },
            &ctx,
        )),
        SkillCommand::Update { target, scope } => Ok(run_install(
            &InstallArgs {
                target,
                scope,
                force: true,
                dry_run: false,
                print: false,
            },
            &ctx,
        )),
        SkillCommand::Uninstall { target, scope } => Ok(run_uninstall(target, scope, &ctx)),
        SkillCommand::List => Ok(run_list(&ctx)),
    }
}

fn resolve_project_root(override_root: Option<&std::path::Path>) -> Result<PathBuf> {
    if let Some(root) = override_root {
        return Ok(root.to_path_buf());
    }
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    Ok(find_project_root(&cwd))
}

fn run_install(args: &InstallArgs, ctx: &RunCtx) -> i32 {
    let installer_target = map_target(args.target);
    let installer_scope = map_scope(args.scope);

    if args.print {
        return run_print(installer_target, ctx);
    }

    let opts = InstallOptions {
        target: installer_target,
        scope: installer_scope,
        project_root: ctx.project_root.clone(),
        home_dir: dirs::home_dir(),
        force: args.force,
        dry_run: args.dry_run,
    };

    match installer::install(&opts) {
        Ok(report) => {
            print_install_report(&report, ctx);
            i32::from(report.has_skipped_user_edits())
        }
        Err(err) => {
            print_installer_error(&err, ctx);
            installer_error_exit_code(&err)
        }
    }
}

fn run_print(target: Target, ctx: &RunCtx) -> i32 {
    let files = match installer::generate_files(target) {
        Ok(f) => f,
        Err(err) => {
            print_installer_error(&err, ctx);
            return installer_error_exit_code(&err);
        }
    };

    if ctx.json {
        let payload: Vec<_> = files
            .iter()
            .map(|(path, content)| {
                serde_json::json!({
                    "path": path.display().to_string(),
                    "content": content,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "target": target.name(),
                "files": payload,
            }))
            .unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        for (path, content) in &files {
            println!("===== {} =====", path.display());
            println!("{content}");
        }
    }
    0
}

fn run_uninstall(target: SkillTarget, scope: SkillScope, ctx: &RunCtx) -> i32 {
    let installer_target = map_target(target);
    let installer_scope = map_scope(scope);
    let home = dirs::home_dir();

    let root = match installer::install_root(
        installer_target,
        installer_scope,
        &ctx.project_root,
        home.as_deref(),
    ) {
        Ok(p) => p,
        Err(err) => {
            print_installer_error(&err, ctx);
            return installer_error_exit_code(&err);
        }
    };

    let files = match installer::generate_files(installer_target) {
        Ok(f) => f,
        Err(err) => {
            print_installer_error(&err, ctx);
            return installer_error_exit_code(&err);
        }
    };

    let mut removed: Vec<PathBuf> = Vec::new();
    let mut preserved: Vec<PathBuf> = Vec::new();

    for (rel, _) in files {
        let abs = root.join(&rel);
        match std::fs::read_to_string(&abs) {
            Ok(existing) if installer::is_lash_generated(&existing) => {
                if let Err(e) = std::fs::remove_file(&abs) {
                    print_uninstall_error(&abs, &e, ctx);
                    return 2;
                }
                removed.push(abs);
            }
            Ok(_) => preserved.push(abs),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                print_uninstall_error(&abs, &e, ctx);
                return 2;
            }
        }
    }

    prune_empty_dirs(&root);

    if ctx.json {
        let payload = serde_json::json!({
            "target": installer_target.name(),
            "install_root": root.display().to_string(),
            "removed": removed.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "preserved_user_files": preserved.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Removed {} file(s) from {}", removed.len(), root.display());
        for p in &removed {
            println!("  - {}", p.display());
        }
        if !preserved.is_empty() {
            let msg = format!("\nLeft {} user-edited file(s) in place:", preserved.len());
            if let Some(t) = ctx.theme {
                eprintln!("{}", t.style_warning(&msg));
            } else {
                eprintln!("{msg}");
            }
            for p in &preserved {
                eprintln!("  - {}", p.display());
            }
        }
    }

    0
}

fn run_list(ctx: &RunCtx) -> i32 {
    let home = dirs::home_dir();
    let mut entries: Vec<(Target, Scope, PathBuf, PathBuf)> = Vec::new();
    for target in installer_targets() {
        for scope in [Scope::Project, Scope::User] {
            let Ok(root) =
                installer::install_root(target, scope, &ctx.project_root, home.as_deref())
            else {
                continue;
            };
            let skill_md = root.join("SKILL.md");
            let agents_md = root.join("AGENTS.lash.md");
            let cursor_mdc = root.join(".cursor/rules/lash.mdc");
            let Some(marker_path) = [skill_md, agents_md, cursor_mdc]
                .into_iter()
                .find(|p| p.exists())
            else {
                continue;
            };
            let installed = std::fs::read_to_string(&marker_path)
                .is_ok_and(|s| installer::is_lash_generated(&s));
            if installed {
                entries.push((target, scope, root, marker_path));
            }
        }
    }

    // Codex and AgentsMd share a generator and produce the same file; dedupe so
    // a single install only surfaces once. The first-encountered target wins.
    let mut seen = std::collections::HashSet::new();
    entries.retain(|(_, _, _, marker)| seen.insert(marker.clone()));

    if ctx.json {
        let payload: Vec<_> = entries
            .iter()
            .map(|(target, scope, root, marker)| {
                serde_json::json!({
                    "target": target.name(),
                    "scope": match scope { Scope::Project => "project", Scope::User => "user" },
                    "install_root": root.display().to_string(),
                    "marker_file": marker.display().to_string(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "installed": payload }))
                .unwrap_or_else(|_| "[]".to_string())
        );
    } else if entries.is_empty() {
        println!("No Lash skills installed in this project or for this user.");
        println!("Install one with: lash skill install --target <claude|codex|cursor|agents-md>");
    } else {
        println!("Installed Lash skills:");
        for (target, scope, root, _) in &entries {
            let scope_name = match scope {
                Scope::Project => "project",
                Scope::User => "user",
            };
            println!("  {} ({})  → {}", target.name(), scope_name, root.display());
        }
    }
    0
}

fn installer_targets() -> [Target; 4] {
    [
        Target::Claude,
        Target::Codex,
        Target::Cursor,
        Target::AgentsMd,
    ]
}

fn map_target(t: SkillTarget) -> Target {
    match t {
        SkillTarget::Claude => Target::Claude,
        SkillTarget::Codex => Target::Codex,
        SkillTarget::Cursor => Target::Cursor,
        SkillTarget::AgentsMd => Target::AgentsMd,
    }
}

fn map_scope(s: SkillScope) -> Scope {
    match s {
        SkillScope::Project => Scope::Project,
        SkillScope::User => Scope::User,
    }
}

fn prune_empty_dirs(root: &std::path::Path) {
    if !root.exists() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            prune_empty_dirs(&path);
        }
    }
    if std::fs::read_dir(root).is_ok_and(|mut it| it.next().is_none()) {
        let _ = std::fs::remove_dir(root);
    }
}

fn print_install_report(report: &InstallReport, ctx: &RunCtx) {
    if ctx.json {
        let payload = serde_json::json!({
            "target": report.target.name(),
            "install_root": report.install_root.display().to_string(),
            "dry_run": report.dry_run,
            "files": report.files.iter().map(file_outcome_json).collect::<Vec<_>>(),
            "summary": report.action_counts(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }

    let header = if report.dry_run {
        format!(
            "Plan: install {} to {}",
            report.target.name(),
            report.install_root.display()
        )
    } else {
        format!(
            "Installed {} skill at {}",
            report.target.name(),
            report.install_root.display()
        )
    };
    println!("{header}");

    for outcome in &report.files {
        let tag = action_tag(outcome.action);
        let path = outcome
            .file
            .path
            .strip_prefix(&report.install_root)
            .unwrap_or(&outcome.file.path)
            .display();
        println!("  {tag}  {path}");
    }

    if report.has_skipped_user_edits() {
        let msg = "\nSome files were left untouched because they appear hand-edited. \
                   Re-run with --force to overwrite them.";
        if let Some(t) = ctx.theme {
            eprintln!("{}", t.style_warning(msg));
        } else {
            eprintln!("{msg}");
        }
    }
}

fn file_outcome_json(o: &FileOutcome) -> serde_json::Value {
    serde_json::json!({
        "path": o.file.path.display().to_string(),
        "action": match o.action {
            FileAction::Created => "created",
            FileAction::Updated => "updated",
            FileAction::Unchanged => "unchanged",
            FileAction::Skipped => "skipped",
            FileAction::Overwritten => "overwritten",
        },
    })
}

fn action_tag(action: FileAction) -> &'static str {
    match action {
        FileAction::Created => "[created]    ",
        FileAction::Updated => "[updated]    ",
        FileAction::Unchanged => "[unchanged]  ",
        FileAction::Skipped => "[skipped]    ",
        FileAction::Overwritten => "[overwritten]",
    }
}

fn print_installer_error(err: &InstallerError, ctx: &RunCtx) {
    if ctx.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "error": err.to_string(),
            }))
            .unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }
    let msg = format!("error: {err}");
    if let Some(t) = ctx.theme {
        eprintln!("{}", t.style_error(&msg));
    } else {
        eprintln!("{msg}");
    }
}

fn print_uninstall_error(path: &std::path::Path, err: &std::io::Error, ctx: &RunCtx) {
    let msg = format!("error: failed to remove {}: {err}", path.display());
    if ctx.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "error": msg }))
                .unwrap_or_else(|_| "{}".to_string())
        );
    } else if let Some(t) = ctx.theme {
        eprintln!("{}", t.style_error(&msg));
    } else {
        eprintln!("{msg}");
    }
}

fn installer_error_exit_code(err: &InstallerError) -> i32 {
    match err {
        InstallerError::NoHomeDir => 4,
        InstallerError::TargetNotImplemented(_) | InstallerError::UserEditedFile(_) => 1,
        InstallerError::Io { .. } => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_target_covers_all_variants() {
        for (cli, expected_name) in [
            (SkillTarget::Claude, "claude"),
            (SkillTarget::Codex, "codex"),
            (SkillTarget::Cursor, "cursor"),
            (SkillTarget::AgentsMd, "agents-md"),
        ] {
            assert_eq!(map_target(cli).name(), expected_name);
        }
    }

    #[test]
    fn map_scope_round_trips() {
        assert!(matches!(map_scope(SkillScope::Project), Scope::Project));
        assert!(matches!(map_scope(SkillScope::User), Scope::User));
    }

    #[test]
    fn action_tag_distinct_per_variant() {
        let tags = [
            action_tag(FileAction::Created),
            action_tag(FileAction::Updated),
            action_tag(FileAction::Unchanged),
            action_tag(FileAction::Skipped),
            action_tag(FileAction::Overwritten),
        ];
        let mut set: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for t in &tags {
            assert!(set.insert(t), "duplicate action tag: {t}");
        }
    }
}
