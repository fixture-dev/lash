//! Interactive confirmation UI for applying fixes
//!
//! Prompts the user to accept, reject, or manually specify fixes for broken links.

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::io::{self, Write};

use lash::theme::CliTheme;

use super::fuzzy_matcher::FuzzyCandidate;

/// User's decision for a proposed fix
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixDecision {
    /// Accept the suggested fix
    Accept(String),
    /// Reject and skip this broken link
    Skip,
    /// Manually specify a different fix
    Manual(String),
}

/// Interactive prompter for confirming fixes
pub struct InteractivePrompter<'a> {
    /// Optional theme for colored output
    theme: Option<&'a CliTheme>,
}

impl<'a> InteractivePrompter<'a> {
    /// Create a new interactive prompter
    ///
    /// # Arguments
    ///
    /// * `theme` - Optional theme for colored output
    pub fn new(theme: Option<&'a CliTheme>) -> Self {
        Self { theme }
    }

    /// Prompt the user to confirm or reject a fix
    ///
    /// Displays the broken reference, the suggested fix with its confidence score,
    /// and allows the user to:
    /// - Accept (y)
    /// - Skip (n)
    /// - Manually enter a different fix (m)
    /// - Quit the fix process (q)
    ///
    /// # Returns
    ///
    /// - `Ok(Some(decision))` - User made a decision
    /// - `Ok(None)` - User chose to quit
    /// - `Err(_)` - I/O error
    #[allow(clippy::too_many_lines)] // Interactive prompts require many lines
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Confidence percentages are 0-100
    pub fn prompt_fix(
        &self,
        file_path: &str,
        from_task_id: &str,
        broken_ref: &str,
        candidates: &[FuzzyCandidate],
    ) -> Result<Option<FixDecision>> {
        println!();
        self.print_separator();

        // Show context
        if let Some(theme) = self.theme {
            println!("{} {}", "File:".bold(), theme.style_info(file_path));
            println!("{} {}", "Task:".bold(), theme.style_warning(from_task_id));
            println!(
                "{} {}",
                "Broken reference:".bold(),
                theme.style_error(broken_ref)
            );
        } else {
            println!("File: {file_path}");
            println!("Task: {from_task_id}");
            println!("Broken reference: {broken_ref}");
        }
        println!();

        // Show candidates if available
        if candidates.is_empty() {
            if let Some(theme) = self.theme {
                println!("{}", theme.style_muted("No similar tasks found."));
            } else {
                println!("No similar tasks found.");
            }
        } else {
            if let Some(_theme) = self.theme {
                println!("{}", "Suggested fixes:".bold());
            } else {
                println!("Suggested fixes:");
            }

            for (idx, candidate) in candidates.iter().enumerate().take(5) {
                let confidence = (candidate.score * 100.0) as u8;
                if let Some(theme) = self.theme {
                    let confidence_display = if confidence >= 85 {
                        theme.style_success(&format!("{confidence}% match"))
                    } else if confidence >= 70 {
                        theme.style_warning(&format!("{confidence}% match"))
                    } else {
                        theme.style_muted(&format!("{confidence}% match"))
                    };
                    println!(
                        "  {}. {} ({})",
                        format!("{}", idx + 1).bold(),
                        theme.style_info(&candidate.task_id),
                        confidence_display
                    );
                } else {
                    println!(
                        "  {}. {} ({}% match)",
                        idx + 1,
                        candidate.task_id,
                        confidence
                    );
                }
            }
        }

        println!();

        // Prompt for decision
        loop {
            if candidates.is_empty() {
                if let Some(theme) = self.theme {
                    print!(
                        "{} {}kip, {}anual fix, {}uit: ",
                        "Action:".bold(),
                        theme.style_success("[s]"),
                        theme.style_info("[m]"),
                        theme.style_error("[q]")
                    );
                } else {
                    print!("Action: [s]kip, [m]anual fix, [q]uit: ");
                }
            } else if let Some(theme) = self.theme {
                print!(
                    "{} {}es (accept #1), {}o (skip), {}1-{}{} (choose), {}anual, {}uit: ",
                    "Action:".bold(),
                    theme.style_success("[y]"),
                    theme.style_warning("[n]"),
                    theme.style_muted("["),
                    candidates.len().min(5),
                    theme.style_muted("]"),
                    theme.style_info("[m]"),
                    theme.style_error("[q]")
                );
            } else {
                print!(
                    "Action: [y]es (accept #1), [n]o (skip), [1-{}] (choose), [m]anual, [q]uit: ",
                    candidates.len().min(5)
                );
            }

            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .context("Failed to read user input")?;

            let input = input.trim().to_lowercase();

            match input.as_str() {
                "y" | "yes" if !candidates.is_empty() => {
                    return Ok(Some(FixDecision::Accept(candidates[0].task_id.clone())));
                }
                "n" | "no" | "s" | "skip" => {
                    return Ok(Some(FixDecision::Skip));
                }
                "m" | "manual" => {
                    return self.prompt_manual_fix();
                }
                "q" | "quit" => {
                    return Ok(None);
                }
                num if !candidates.is_empty() => {
                    // Try to parse as number
                    if let Ok(idx) = num.parse::<usize>() {
                        if idx > 0 && idx <= candidates.len().min(5) {
                            return Ok(Some(FixDecision::Accept(
                                candidates[idx - 1].task_id.clone(),
                            )));
                        }
                    }
                    // Invalid input, loop continues
                    if let Some(theme) = self.theme {
                        println!("{}", theme.style_error("Invalid choice. Please try again."));
                    } else {
                        println!("Invalid choice. Please try again.");
                    }
                }
                _ => {
                    if let Some(theme) = self.theme {
                        println!("{}", theme.style_error("Invalid input. Please try again."));
                    } else {
                        println!("Invalid input. Please try again.");
                    }
                }
            }
        }
    }

    /// Prompt for manual fix input
    fn prompt_manual_fix(&self) -> Result<Option<FixDecision>> {
        if let Some(_theme) = self.theme {
            print!(
                "{} ",
                "Enter the correct task reference (or 'cancel'):".bold()
            );
        } else {
            print!("Enter the correct task reference (or 'cancel'): ");
        }
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read user input")?;

        let input = input.trim();

        if input.is_empty() || input.eq_ignore_ascii_case("cancel") {
            return Ok(Some(FixDecision::Skip));
        }

        Ok(Some(FixDecision::Manual(input.to_string())))
    }

    /// Print a visual separator
    fn print_separator(&self) {
        if let Some(theme) = self.theme {
            println!("{}", theme.style_muted(&"-".repeat(60)));
        } else {
            println!("{}", "-".repeat(60));
        }
    }

    /// Show summary of fixes applied
    ///
    /// # Arguments
    ///
    /// * `accepted` - Number of fixes accepted
    /// * `skipped` - Number of fixes skipped
    /// * `manual` - Number of manual fixes
    pub fn show_summary(&self, accepted: usize, skipped: usize, manual: usize) {
        println!();
        self.print_separator();

        if let Some(theme) = self.theme {
            println!("{}", "Fix Summary:".bold());
            println!(
                "  {}: {}",
                theme.style_success("Accepted"),
                accepted.to_string().bold()
            );
            println!(
                "  {}: {}",
                theme.style_info("Manual"),
                manual.to_string().bold()
            );
            println!(
                "  {}: {}",
                theme.style_warning("Skipped"),
                skipped.to_string().bold()
            );
            println!(
                "  {}: {}",
                "Total fixed".bold(),
                theme.style_success(&(accepted + manual).to_string()).bold()
            );
        } else {
            println!("Fix Summary:");
            println!("  Accepted: {accepted}");
            println!("  Manual: {manual}");
            println!("  Skipped: {skipped}");
            println!("  Total fixed: {}", accepted + manual);
        }

        self.print_separator();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_decision_equality() {
        assert_eq!(FixDecision::Skip, FixDecision::Skip);
        assert_eq!(
            FixDecision::Accept("test".to_string()),
            FixDecision::Accept("test".to_string())
        );
        assert_eq!(
            FixDecision::Manual("test".to_string()),
            FixDecision::Manual("test".to_string())
        );
        assert_ne!(FixDecision::Skip, FixDecision::Accept("test".to_string()));
    }

    #[test]
    fn test_prompter_creation() {
        use lash_tui::colors::{Theme, REGISTRY};

        let prompter = InteractivePrompter::new(None);
        assert!(prompter.theme.is_none());

        // With a theme
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        let tui_theme = Theme::new(scheme.clone());
        let cli_theme = CliTheme::new(tui_theme, true);
        let prompter = InteractivePrompter::new(Some(&cli_theme));
        assert!(prompter.theme.is_some());
    }

    // Note: Interactive tests would require mocking stdin/stdout,
    // which is complex and fragile. The interactive behavior will be
    // tested manually and through integration tests.
}
