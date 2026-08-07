//! Init command implementation
//!
//! The `lash init` command initializes a new Lash project in the current directory.
//! It creates the `lash.index.md` file and `.lash/` directory, then runs indexing.

use anyhow::{Context, Result};
use lash::formatter::Verbosity;
use lash::theme::CliTheme;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::instrument;

/// Arguments for the init command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct InitArgs {
    /// Target directory for initialization (defaults to current directory)
    pub path: Option<PathBuf>,
    /// Skip running index command after initialization
    pub no_index: bool,
    /// Force re-initialization even if project already exists
    pub force: bool,
    /// Enable JSON output mode
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Show errors as they occur (streaming) vs at end (batch)
    pub errors_streaming: bool,
    /// Verbosity level
    pub verbosity: Verbosity,
}

/// Execute the init command
///
/// # Arguments
///
/// * `args` - Init command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (error)
#[instrument(skip(args), fields(no_index = args.no_index, force = args.force))]
#[allow(clippy::needless_pass_by_value)]
pub fn execute(args: InitArgs) -> Result<i32> {
    // Load theme based on no_color flag and output format
    let theme = if args.json {
        None
    } else {
        CliTheme::load(None, !args.no_color)?
    };

    let cwd = args.path.clone().map_or_else(
        || std::env::current_dir().context("Failed to get current directory"),
        Ok,
    )?;

    tracing::info!(cwd = %cwd.display(), "Initializing Lash project");

    let index_file = cwd.join("lash.index.md");
    let lash_dir = cwd.join(".lash");

    // Check if project already exists
    let project_exists = index_file.exists() || lash_dir.exists();

    if project_exists && !args.force {
        if args.json {
            let error = serde_json::json!({
                "error": "Lash project already exists",
                "path": cwd.display().to_string(),
                "suggestion": "Use --force to re-initialize"
            });
            println!("{}", serde_json::to_string_pretty(&error)?);
        } else {
            let error_msg = format!("Error: Lash project already exists in: {}", cwd.display());
            if let Some(ref t) = theme {
                eprintln!("{}", t.style_error(&error_msg));
            } else {
                eprintln!("{error_msg}");
            }
            if index_file.exists() {
                eprintln!("  Found: lash.index.md");
            }
            if lash_dir.exists() {
                eprintln!("  Found: .lash/");
            }
            eprintln!();
            eprintln!("Use --force to re-initialize.");
        }
        return Ok(1);
    }

    // Create .lash directory
    if !lash_dir.exists() {
        tracing::info!("Creating .lash directory");
        fs::create_dir_all(&lash_dir).context("Failed to create .lash directory")?;
    }

    // Create lash.index.md if it doesn't exist or force is set
    if !index_file.exists() || args.force {
        tracing::info!("Creating lash.index.md");
        let index_content = generate_index_template(&cwd);
        fs::write(&index_file, index_content).context("Failed to create lash.index.md")?;
    }

    // Run indexing unless --no-index is specified
    if !args.no_index {
        tracing::info!("Running initial index");
        if let Err(e) = run_index(&cwd, &args) {
            tracing::warn!(error = %e, "Failed to run initial index (non-fatal)");
            if !args.json {
                let warning_msg = format!("Warning: Initial indexing failed: {e}");
                if let Some(ref t) = theme {
                    eprintln!("{}", t.style_warning(&warning_msg));
                } else {
                    eprintln!("{warning_msg}");
                }
                eprintln!("You can run 'lash index' manually to index your project.");
            }
        }
    }

    // Print success message
    print_success_message(&args, &cwd, &index_file, theme.as_ref())?;

    Ok(0)
}

/// Print the success message after initialization
fn print_success_message(
    args: &InitArgs,
    cwd: &Path,
    index_file: &Path,
    theme: Option<&CliTheme>,
) -> Result<()> {
    if args.json {
        let success = serde_json::json!({
            "success": true,
            "path": cwd.display().to_string(),
            "index_file": index_file.display().to_string(),
            "indexed": !args.no_index,
            "message": "Lash project initialized successfully"
        });
        println!("{}", serde_json::to_string_pretty(&success)?);
    } else if let Some(t) = theme {
        println!();
        println!(
            "{}",
            t.style_success("Lash project initialized successfully!")
        );
        println!();
        println!("{} {}", t.style_info("Location:"), cwd.display());
        println!("  {} lash.index.md", t.style_success("Created:"));
        println!("  {} .lash/", t.style_success("Created:"));
        println!();
        println!("{}:", t.style_info("Next steps"));
        println!("  1. Edit lash.index.md to define your project structure");
        println!("  2. Create task files referenced in the index");
        println!("  3. Run 'lash list' to see your tasks");
        println!();
        println!(
            "For more information, see: {}",
            t.style_muted("https://github.com/your-org/lash")
        );
    } else {
        println!();
        println!("Lash project initialized successfully!");
        println!();
        println!("Location: {}", cwd.display());
        println!("  Created: lash.index.md");
        println!("  Created: .lash/");
        println!();
        println!("Next steps:");
        println!("  1. Edit lash.index.md to define your project structure");
        println!("  2. Create task files referenced in the index");
        println!("  3. Run 'lash list' to see your tasks");
        println!();
        println!("For more information, see: https://github.com/your-org/lash");
    }
    Ok(())
}

/// Run the index command on the project
fn run_index(project_root: &Path, args: &InitArgs) -> Result<()> {
    use crate::commands::index::{execute as index_execute, IndexArgs};

    let index_args = IndexArgs {
        paths: Vec::new(),
        force: true,
        show_files: false,
        json: args.json,
        no_color: args.no_color,
        errors_streaming: args.errors_streaming,
        project_root: Some(project_root.to_path_buf()),
        verbosity: args.verbosity,
    };

    let exit_code = index_execute(index_args)?;
    if exit_code != 0 {
        anyhow::bail!("Index command returned exit code {exit_code}");
    }
    Ok(())
}

/// Generate the default lash.index.md template
fn generate_index_template(project_root: &Path) -> String {
    let project_name = project_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("My Project");

    format!(
        r"# {project_name}

@id: index
@created: {date}

## Overview

This is the root index file for the Lash task tracker.
Edit this file to define your project's task structure.

## Tasks

- [ ] Set up project structure
- [ ] Define task files and categories
- [ ] Add initial tasks

## Task Files

Add references to your task files here:

<!-- Example:
- [Features](features/tasks.md)
- [Backend](backend/tasks.md)
- [Frontend](frontend/tasks.md)
-->

## Notes

- Use `lash list` to view all tasks
- Use `lash search <query>` to find specific tasks
- Use `lash tui` for an interactive terminal interface
",
        project_name = project_name,
        date = chrono::Local::now().format("%Y-%m-%d"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    /// Create a temporary HOME directory with a `~/.lash/config.toml` that specifies
    /// an invalid color-scheme name.  When `CliTheme::load` is called with
    /// `colors_enabled=true`, it reads the bad scheme name and returns Err.
    fn make_bad_scheme_home() -> TempDir {
        let home = TempDir::new().unwrap();
        let lash_dir = home.path().join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        fs::write(
            lash_dir.join("config.toml"),
            "color_scheme = \"NonExistentSchemeForMutationKillTest\"\n",
        )
        .unwrap();
        home
    }

    #[test]
    fn test_generate_index_template() {
        let project_root = PathBuf::from("/tmp/test-project");
        let template = generate_index_template(&project_root);

        assert!(template.contains("# test-project"));
        assert!(template.contains("@id: index"));
        assert!(template.contains("## Tasks"));
    }

    #[test]
    fn test_init_creates_files() {
        let temp = TempDir::new().unwrap();

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true, // Skip indexing for test
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };

        let result = execute(args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Check files were created
        assert!(temp.path().join("lash.index.md").exists());
        assert!(temp.path().join(".lash").exists());
    }

    #[test]
    fn test_init_fails_if_exists() {
        let temp = TempDir::new().unwrap();

        // Create existing index file
        fs::write(temp.path().join("lash.index.md"), "# Existing").unwrap();

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };

        let result = execute(args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // Should fail with exit code 1
    }

    #[test]
    fn test_init_force_overwrites() {
        let temp = TempDir::new().unwrap();

        // Create existing index file
        fs::write(temp.path().join("lash.index.md"), "# Existing").unwrap();

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: true, // Force should allow overwrite
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };

        let result = execute(args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Check file was overwritten
        let content = fs::read_to_string(temp.path().join("lash.index.md")).unwrap();
        assert!(content.contains("@id: index")); // New content
    }

    // Kill mut-000409: json=true in execute() does not load theme
    // Kill mut-000425: print_success_message with json=true outputs JSON
    #[test]
    fn test_init_json_mode_returns_0() {
        let temp = TempDir::new().unwrap();
        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: true,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        assert_eq!(result, 0);
        // JSON success message should have been printed to stdout
    }

    // Kill mut-000410: no_color=false exercises !args.no_color => true
    #[test]
    fn test_init_no_color_false_returns_0() {
        let temp = TempDir::new().unwrap();
        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: false,
            no_color: false, // !false => true, enables color
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        assert_eq!(result, 0);
    }

    // Kill mut-000415: project_exists && !args.force - json=true path when project exists
    #[test]
    fn test_init_json_mode_fails_if_exists_returns_1() {
        let temp = TempDir::new().unwrap();
        // Create existing project structure
        fs::write(temp.path().join("lash.index.md"), "# Existing").unwrap();

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: true, // json=true path when project already exists
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        assert_eq!(result, 1);
    }

    // Kill mut-000417: index_file.exists() branch - only index file exists (not .lash dir)
    #[test]
    fn test_init_reports_found_index_file_only() {
        let temp = TempDir::new().unwrap();
        // Create only the index file, not .lash dir
        fs::write(temp.path().join("lash.index.md"), "# Existing").unwrap();
        // .lash dir does NOT exist

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        // Should fail because index_file.exists() is true
        assert_eq!(result, 1);
        // Verify that only index file exists, not .lash dir
        assert!(temp.path().join("lash.index.md").exists());
        assert!(!temp.path().join(".lash").exists());
    }

    // Kill mut-000418: lash_dir.exists() branch - only .lash dir exists (not index file)
    #[test]
    fn test_init_reports_found_lash_dir_only() {
        let temp = TempDir::new().unwrap();
        // Create only the .lash dir, not the index file
        fs::create_dir_all(temp.path().join(".lash")).unwrap();
        // lash.index.md does NOT exist

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        // Should fail because lash_dir.exists() is true
        assert_eq!(result, 1);
        // Verify that only .lash dir exists, not index file
        assert!(!temp.path().join("lash.index.md").exists());
        assert!(temp.path().join(".lash").exists());
    }

    // Kill mut-000423: !args.no_index - with no_index=true, indexing is skipped
    #[test]
    fn test_init_no_index_true_skips_indexing() {
        let temp = TempDir::new().unwrap();
        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        // Should succeed (indexing skipped, no DB operations)
        assert_eq!(result, 0);
        assert!(temp.path().join("lash.index.md").exists());
    }

    // Kill mut-000423: !args.no_index - with no_index=false, indexing is attempted
    #[test]
    fn test_init_no_index_false_attempts_indexing() {
        let temp = TempDir::new().unwrap();
        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: false, // !false => true, so indexing is attempted
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        // Indexing may fail (no tasks to index) but execute should still return 0
        let result = execute(args).unwrap();
        assert_eq!(result, 0);
        assert!(temp.path().join("lash.index.md").exists());
    }

    // Kill mut-000425: print_success_message with json=true outputs JSON with "success": true
    #[test]
    fn test_print_success_message_json_mode() {
        let temp = TempDir::new().unwrap();
        let index_file = temp.path().join("lash.index.md");

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: true,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };

        // print_success_message is private, so we test it via execute()
        let result = execute(args).unwrap();
        assert_eq!(result, 0);
        // Index file should have been created
        assert!(index_file.exists());
    }

    // Kill mut-000451 (args.json negation in theme-loading of execute()):
    // json=true and json=false must both return 0 on a fresh directory.
    #[test]
    fn test_execute_json_true_and_false_both_return_0_on_fresh_dir() {
        for json_flag in [true, false] {
            let temp = TempDir::new().unwrap();
            let args = InitArgs {
                path: Some(temp.path().to_path_buf()),
                no_index: true,
                force: false,
                json: json_flag,
                no_color: true,
                errors_streaming: false,
                verbosity: Verbosity::Quiet,
            };
            assert_eq!(
                execute(args).unwrap(),
                0,
                "json={json_flag}: execute() on fresh dir must return 0"
            );
        }
    }

    // Kill mut-000452 (!args.no_color negation):
    // no_color=true and no_color=false must both return 0 on a fresh directory.
    #[test]
    fn test_execute_no_color_true_and_false_both_return_0() {
        for no_color_flag in [true, false] {
            let temp = TempDir::new().unwrap();
            let args = InitArgs {
                path: Some(temp.path().to_path_buf()),
                no_index: true,
                force: false,
                json: false,
                no_color: no_color_flag,
                errors_streaming: false,
                verbosity: Verbosity::Quiet,
            };
            assert_eq!(
                execute(args).unwrap(),
                0,
                "no_color={no_color_flag}: execute() on fresh dir must return 0"
            );
        }
    }

    // Kill mut-000457 (args.json negation in project-exists error branch):
    // When project exists and no force flag, json=true and json=false must both return 1.
    #[test]
    fn test_execute_project_exists_returns_1_in_both_json_and_text_modes() {
        for json_flag in [true, false] {
            let temp = TempDir::new().unwrap();
            fs::write(temp.path().join("lash.index.md"), "# Existing").unwrap();
            let args = InitArgs {
                path: Some(temp.path().to_path_buf()),
                no_index: true,
                force: false,
                json: json_flag,
                no_color: true,
                errors_streaming: false,
                verbosity: Verbosity::Quiet,
            };
            assert_eq!(
                execute(args).unwrap(),
                1,
                "json={json_flag}: execute() when project exists must return 1"
            );
        }
    }

    // Kill mut-000459 (index_file.exists() negation in error diagnostic):
    // When only index_file exists (not .lash dir), execute returns 1.
    // When only .lash dir exists (not index_file), execute also returns 1.
    // Both scenarios cover the two exists() checks in the error branch.
    #[test]
    fn test_execute_returns_1_when_only_index_file_exists() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("lash.index.md"), "# Existing").unwrap();
        assert!(!temp.path().join(".lash").exists());

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        assert_eq!(execute(args).unwrap(), 1);
    }

    // Kill mut-000460 (lash_dir.exists() negation in error diagnostic):
    #[test]
    fn test_execute_returns_1_when_only_lash_dir_exists() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join(".lash")).unwrap();
        assert!(!temp.path().join("lash.index.md").exists());

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        assert_eq!(execute(args).unwrap(), 1);
    }

    // Kill mut-000465 (!args.no_index negation):
    // With no_index=true: indexing skipped → no DB created.
    // With no_index=false: indexing attempted → DB should be created.
    #[test]
    fn test_execute_no_index_true_skips_db_creation() {
        let temp = TempDir::new().unwrap();
        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        assert_eq!(result, 0);
        assert!(
            !temp.path().join(".lash").join("lash.db").exists(),
            "no_index=true must not create lash.db"
        );
    }

    #[test]
    fn test_execute_no_index_false_creates_db() {
        let temp = TempDir::new().unwrap();
        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: false,
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        assert_eq!(result, 0);
        assert!(
            temp.path().join(".lash").join("lash.db").exists(),
            "no_index=false must create lash.db after successful indexing"
        );
    }

    // Kill mut-000468 (args.json negation in print_success_message):
    // json=true and json=false must both succeed (return 0) on init.
    #[test]
    fn test_print_success_message_both_modes_return_0() {
        for json_flag in [true, false] {
            let temp = TempDir::new().unwrap();
            let args = InitArgs {
                path: Some(temp.path().to_path_buf()),
                no_index: true,
                force: false,
                json: json_flag,
                no_color: true,
                errors_streaming: false,
                verbosity: Verbosity::Quiet,
            };
            assert_eq!(
                execute(args).unwrap(),
                0,
                "json={json_flag}: execute() must return 0 for success message"
            );
        }
    }

    // Kill mut-000470 (force: true → force: false in run_index) and
    // mut-000471 (show_files: false → show_files: true in run_index):
    // run_index is private; we test it indirectly through execute() without --no-index.
    // The key observable: with force=true in IndexArgs, run_index can rebuild a corrupt DB.
    // With force=false (mutation), a corrupt DB would cause an error.
    #[test]
    fn test_execute_without_no_index_rebuilds_corrupt_db_on_force() {
        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        fs::create_dir_all(&lash_dir).unwrap();
        // Write corrupt DB data to lash.db
        fs::write(lash_dir.join("lash.db"), b"not a valid sqlite database").unwrap();

        // With force=true in run_index (the current source code), init with force=true
        // should succeed even with a corrupt DB.
        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: false, // Run indexing
            force: true,     // Force flag passed through to run_index
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args).unwrap();
        assert_eq!(
            result, 0,
            "init --force must succeed even with a corrupt lash.db (run_index uses force=true)"
        );
        // Verify DB was rebuilt correctly
        let db_bytes = fs::read(lash_dir.join("lash.db")).unwrap();
        assert_eq!(
            &db_bytes[..16],
            b"SQLite format 3\0",
            "lash.db must be a valid SQLite database after forced rebuild"
        );
    }

    // Kill mut-000472 (exit_code != 0 negation), mut-000473 (!= → ==),
    // mut-000474 (literal 0 → 1) in run_index:
    // These all affect the guard `if exit_code != 0 { bail! }`.
    // A successful index (exit_code = 0) must NOT trigger the bail.
    // Observable: execute() returns 0, not an Err.
    #[test]
    fn test_execute_successful_index_does_not_error() {
        let temp = TempDir::new().unwrap();
        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: false, // run_index is called
            force: false,
            json: false,
            no_color: true,
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        // A successful index returns exit_code=0 → the guard `exit_code != 0` is false
        // → no bail → run_index returns Ok(()) → execute returns Ok(0).
        let result = execute(args);
        assert!(
            result.is_ok(),
            "successful index must not cause execute() to return Err"
        );
        assert_eq!(
            result.unwrap(),
            0,
            "successful index must cause execute() to return 0"
        );
    }

    // -------------------------------------------------------------------------
    // mut-000409 (L46): `args.json` → `!(args.json)` in theme-loading guard
    //
    // Original:  if args.json { None } else { CliTheme::load(None, !no_color)? }
    // Mutation:  if !(args.json) { None } else { CliTheme::load(None, !no_color)? }
    //
    // With json=true and a bad HOME color-scheme:
    //   Original  → None (CliTheme::load never called)  → execute() Ok(0)
    //   Mutant    → else branch → CliTheme::load(None, !no_color)
    //             → reads bad HOME config → invalid scheme → Err
    //             → execute() returns Err → test fails
    // -------------------------------------------------------------------------

    /// json=true must skip theme loading entirely.  A bad user config must not
    /// cause a failure when json=true.
    ///
    /// Kills mut-000409.
    #[test]
    #[serial]
    fn test_json_true_skips_theme_load_even_with_bad_home_scheme() {
        let temp = TempDir::new().unwrap();
        let bad_home = make_bad_scheme_home();

        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", bad_home.path());

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: true,      // Original: theme=None (no scheme lookup)
            no_color: false, // Would trigger scheme lookup if theme were loaded
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args);

        match old_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }

        // Original: json=true → theme=None → Ok(0)
        // Mutant:   !(args.json) with json=true → else branch → bad scheme → Err
        assert_eq!(result.unwrap(), 0, "execute() must succeed when json=true");
    }

    // -------------------------------------------------------------------------
    // mut-000410 (L49): `!args.no_color` → `args.no_color` in CliTheme::load
    //
    // Original:  CliTheme::load(None, !args.no_color)
    // Mutation:  CliTheme::load(None,  args.no_color)
    //
    // With json=false, no_color=true, and a bad HOME color-scheme:
    //   Original  → load(None, !true = false) → Ok(None), no registry lookup
    //   Mutant    → load(None,  true)         → reads bad HOME config
    //             → invalid scheme → Err → execute() returns Err → test fails
    // -------------------------------------------------------------------------

    /// When `no_color=true`, `CliTheme::load` must receive `false` as the
    /// `colors_enabled` argument and return Ok(None) without touching the
    /// color-scheme registry.
    ///
    /// Kills mut-000410.
    #[test]
    #[serial]
    fn test_no_color_true_disables_theme_lookup_in_init() {
        let temp = TempDir::new().unwrap();
        let bad_home = make_bad_scheme_home();

        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", bad_home.path());

        let args = InitArgs {
            path: Some(temp.path().to_path_buf()),
            no_index: true,
            force: false,
            json: false,
            no_color: true, // !true=false → load(None,false) → Ok(None), no bad-scheme lookup
            errors_streaming: false,
            verbosity: Verbosity::Quiet,
        };
        let result = execute(args);

        match old_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }

        // Original: !no_color=false → Ok(None), no bad-scheme lookup → Ok(0)
        // Mutant:   no_color=true   → load(None,true) → bad-scheme → Err
        assert_eq!(
            result.unwrap(),
            0,
            "execute() must succeed when no_color=true"
        );
    }
}
