//! Playground command implementation
//!
//! The `lash playground init` command generates a realistic demo project
//! for exploring Lash features and capabilities.

#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::instrument;

/// Arguments for the playground command
#[derive(Debug, Clone)]
pub struct PlaygroundArgs {
    /// Target directory path (defaults to ./playground/)
    pub path: Option<PathBuf>,
    /// Delete and regenerate if playground already exists
    pub reset: bool,
    /// Enable JSON output mode
    pub json: bool,
    /// Disable colored output (currently unused but reserved for future use)
    #[allow(dead_code)]
    pub no_color: bool,
}

/// Execute the playground init command
///
/// # Arguments
///
/// * `args` - Playground command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (error)
#[instrument(skip(args), fields(path = ?args.path, reset = args.reset))]
pub fn execute(args: PlaygroundArgs) -> Result<i32> {
    // Determine target path (default to ./playground/)
    let target_path = args.path.as_ref().map_or_else(
        || std::env::current_dir().map(|p| p.join("playground")),
        |p| Ok(p.clone()),
    )?;

    tracing::info!(target_path = %target_path.display(), "Initializing playground");

    // Check if directory exists
    if target_path.exists() {
        if !args.reset {
            if args.json {
                let error = serde_json::json!({
                    "error": "Playground already exists",
                    "path": target_path.display().to_string(),
                    "suggestion": "Use --reset to regenerate"
                });
                println!("{}", serde_json::to_string_pretty(&error)?);
            } else {
                eprintln!(
                    "Error: Playground already exists at: {}",
                    target_path.display()
                );
                eprintln!("Use --reset to delete and regenerate.");
            }
            return Ok(1);
        }

        // Remove existing directory
        tracing::info!("Removing existing playground directory");
        fs::remove_dir_all(&target_path)
            .context("Failed to remove existing playground directory")?;
    }

    // Create playground directory
    tracing::info!("Creating playground directory");
    fs::create_dir_all(&target_path).context("Failed to create playground directory")?;

    // Generate PixelQuest demo project
    tracing::info!("Generating PixelQuest demo project");
    generate_pixelquest_project(&target_path).context("Failed to generate PixelQuest project")?;

    // Auto-run indexing on generated project
    tracing::info!("Indexing playground project");
    if let Err(e) = index_playground(&target_path) {
        tracing::warn!(error = %e, "Failed to index playground (non-fatal)");
    }

    // Generate PLAYGROUND_GUIDE.md
    tracing::info!("Generating playground guide");
    generate_playground_guide(&target_path).context("Failed to generate playground guide")?;

    // Print success message
    if args.json {
        let success = serde_json::json!({
            "success": true,
            "path": target_path.display().to_string(),
            "message": "PixelQuest playground initialized successfully"
        });
        println!("{}", serde_json::to_string_pretty(&success)?);
    } else {
        println!();
        println!("🎮 PixelQuest playground initialized successfully!");
        println!();
        println!("Location: {}", target_path.display());
        println!();
        println!("Next steps:");
        println!("  cd {}", target_path.display());
        println!("  lash list              # List all tasks");
        println!("  lash search \"boss\"     # Search for tasks");
        println!("  lash show features/player-movement.md  # View specific file");
        println!("  cat PLAYGROUND_GUIDE.md  # Read the full guide");
        println!();
    }

    Ok(0)
}

/// Index the playground project
fn index_playground(project_root: &Path) -> Result<()> {
    use crate::commands::index::{execute, IndexArgs};

    let args = IndexArgs {
        force: true,
        show_files: false,
        json: false,
        no_color: false,
        project_root: Some(project_root.to_path_buf()),
    };

    execute(args)?;
    Ok(())
}

/// Generate the `PixelQuest` project by copying from the existing fixture
///
/// This uses a simple recursive copy of the pre-generated fixture directory.
/// The fixture was created by the generator in `tests/fixtures/generators/pixelquest.rs`
fn generate_pixelquest_project(output_dir: &Path) -> std::io::Result<()> {
    // Source: the pre-generated pixelquest project in test fixtures
    // We assume the binary is run from the repo root, so we can find the fixture
    let fixture_source = find_pixelquest_fixture_source()?;

    // Copy all files recursively
    copy_dir_recursive(&fixture_source, output_dir)?;

    Ok(())
}

/// Find the source directory for the `PixelQuest` fixture
///
/// Tries multiple possible locations relative to the binary
fn find_pixelquest_fixture_source() -> std::io::Result<PathBuf> {
    // Try relative to current directory (if run from repo root)
    let candidates = vec![
        PathBuf::from("crates/lash-cli/tests/fixtures/repos/pixelquest-project"),
        PathBuf::from("tests/fixtures/repos/pixelquest-project"),
        // Try relative to executable (in target/debug or target/release)
        std::env::current_exe()?
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|p| p.join("crates/lash-cli/tests/fixtures/repos/pixelquest-project"))
            .unwrap_or_default(),
    ];

    for candidate in candidates {
        if candidate.exists() && candidate.is_dir() {
            tracing::debug!(source = %candidate.display(), "Found PixelQuest fixture source");
            return Ok(candidate);
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Could not find PixelQuest fixture source. Make sure you're running from the repository root.",
    ))
}

/// Copy a directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Generate the `PLAYGROUND_GUIDE.md` file
fn generate_playground_guide(output_dir: &Path) -> std::io::Result<()> {
    let guide_content = r#"# PixelQuest: A Lash Playground Demo

Welcome to PixelQuest - a realistic 2D platformer game development project that demonstrates all of Lash's task tracking features!

## Quick Start

```bash
# List all gameplay-related tasks
lash list --label gameplay

# Show player movement tasks
lash show features/player-movement.md

# Search for boss-related work
lash search "boss"

# View full task list
lash list
```

## Project Overview

This demo project represents a game in active development:
- **24 task files** across features, systems, content, infrastructure, design, and milestones
- **393 total tasks** showing realistic project complexity
- **Mixed statuses**: 70% open, 28% done, 2% waived

### Project Structure

- `features/` - Game mechanics and features
- `systems/` - Core engine systems (rendering, physics, audio, input)
- `content/` - Art, animations, music, sound effects, levels
- `infrastructure/` - Build pipeline, asset processing, testing
- `design/` - Game design documents
- `milestones/` - Release planning (alpha, beta, release)

## Exploring the Project

### View by Category

```bash
# Backend/engine tasks
lash list --label backend

# Art and graphics
lash list --label art

# Audio work
lash list --label audio

# High priority items
lash list --label p0
```

### Dependency Visualization

```bash
# Export full dependency graph
lash graph --output pixelquest.dot

# View in Graphviz
dot -Tpng pixelquest.dot -o graph.png && open graph.png
```

### Search Examples

```bash
# Find all AI-related tasks
lash search "AI behavior"

# Find collision detection work
lash search "collision"

# Find music composition tasks
lash search "music theme"
```

### Advanced Queries

```bash
# Show blocked tasks
lash list --blocked

# Find tasks by owner
lash list | grep "owner: Alice"

# Generate agent prompt for high-priority gameplay work
lash agent-prompt --label gameplay --label p0
```

## What Makes This Realistic?

- **Cross-file dependencies**: Boss fights depend on enemy AI system
- **Milestone tracking**: Alpha (done) → Beta (in-progress) → Release (planned)
- **Rich annotations**: @owner, @estimate, @created, @depends-on, @agent-note
- **Authentic content**: Real game dev terminology and workflows
- **Mixed statuses**: Shows natural project progression

## Resetting the Playground

Want to start fresh?

```bash
lash playground init --reset
```

This will delete and regenerate the entire project.

## Next Steps

- Try the TUI: `lash tui`
- Create your own project following this structure
- See the Lash documentation for more features

Happy task tracking!
"#;

    fs::write(output_dir.join("PLAYGROUND_GUIDE.md"), guide_content)?;
    Ok(())
}
