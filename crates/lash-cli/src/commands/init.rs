//! Init command implementation
//!
//! The `lash init` command initializes a new Lash project in the current directory.
//! It creates the `lash.index.md` file and `.lash/` directory, then runs indexing.

use anyhow::{Context, Result};
use lash_cli::formatter::Verbosity;
use lash_cli::theme::CliTheme;
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
@status: in-progress
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
    use tempfile::TempDir;

    #[test]
    fn test_generate_index_template() {
        let project_root = PathBuf::from("/tmp/test-project");
        let template = generate_index_template(&project_root);

        assert!(template.contains("# test-project"));
        assert!(template.contains("@id: index"));
        assert!(template.contains("@status: in-progress"));
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
}
