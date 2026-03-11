//! Example demonstrating the CLI theme module
//!
//! This example shows how to load and use the `CliTheme` for terminal styling.
//!
//! Run with: cargo run --example `theme_example`

use lash_cli::theme::CliTheme;
use lash_types::TaskStatus;

fn main() -> anyhow::Result<()> {
    println!("=== Lash CLI Theme Example ===\n");

    // Load the default theme with colors enabled
    println!("Loading default theme...");
    if let Some(theme) = CliTheme::load(None, true)? {
        println!("Loaded theme: {}\n", theme.name());

        // Demonstrate semantic styling
        println!("Semantic styles:");
        println!("  {}", theme.style_success("✓ Success message"));
        println!("  {}", theme.style_error("✗ Error message"));
        println!("  {}", theme.style_warning("⚠ Warning message"));
        println!("  {}", theme.style_info("ℹ Info message"));
        println!("  {}", theme.style_muted("(muted/secondary text)"));
        println!("  {}", theme.style_label("#backend #feature"));

        // Demonstrate task status styling
        println!("\nTask status checkboxes:");
        println!("  {} Done task", theme.styled_checkbox(TaskStatus::Done));
        println!(
            "  {} Blocked task",
            theme.styled_checkbox(TaskStatus::Blocked)
        );
        println!("  {} Open task", theme.styled_checkbox(TaskStatus::Open));
        println!(
            "  {} Waived task",
            theme.styled_checkbox(TaskStatus::Waived)
        );

        // Custom status styling
        println!("\nCustom status styling:");
        println!(
            "  {}",
            theme.style_task_status("Completed successfully", TaskStatus::Done)
        );
        println!(
            "  {}",
            theme.style_task_status("Blocked by dependencies", TaskStatus::Blocked)
        );
    } else {
        println!("Colors are disabled");
    }

    // Load a specific theme
    println!("\n\nLoading specific theme (3024 Night)...");
    if let Some(theme) = CliTheme::load(Some("3024 Night"), true)? {
        println!("Loaded theme: {}\n", theme.name());
        println!("  {}", theme.style_success("Success in 3024 Night theme"));
        println!("  {}", theme.style_error("Error in 3024 Night theme"));
    }

    // Demonstrate loading with colors disabled
    println!("\n\nLoading theme with colors disabled...");
    let theme_opt = CliTheme::load(None, false)?;
    if theme_opt.is_none() {
        println!("Theme is None (colors disabled)");
        println!("Plain text: Success message");
        println!("Plain text: Error message");
    }

    Ok(())
}
