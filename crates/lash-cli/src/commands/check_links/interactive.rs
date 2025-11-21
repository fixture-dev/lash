//! Interactive confirmation UI for applying fixes
//!
//! Prompts the user to accept, reject, or manually specify fixes for broken links.

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::io::{self, Write};

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
pub struct InteractivePrompter {
    /// Whether to disable colored output
    no_color: bool,
}

impl InteractivePrompter {
    /// Create a new interactive prompter
    ///
    /// # Arguments
    ///
    /// * `no_color` - Disable colored output
    pub fn new(no_color: bool) -> Self {
        Self { no_color }
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
        if self.no_color {
            println!("File: {file_path}");
            println!("Task: {from_task_id}");
            println!("Broken reference: {broken_ref}");
        } else {
            println!("{} {}", "File:".bold(), file_path.cyan());
            println!("{} {}", "Task:".bold(), from_task_id.yellow());
            println!("{} {}", "Broken reference:".bold(), broken_ref.red());
        }
        println!();

        // Show candidates if available
        if candidates.is_empty() {
            if self.no_color {
                println!("No similar tasks found.");
            } else {
                println!("{}", "No similar tasks found.".dimmed());
            }
        } else {
            if self.no_color {
                println!("Suggested fixes:");
            } else {
                println!("{}", "Suggested fixes:".bold());
            }

            for (idx, candidate) in candidates.iter().enumerate().take(5) {
                let confidence = (candidate.score * 100.0) as u8;
                if self.no_color {
                    println!(
                        "  {}. {} ({}% match)",
                        idx + 1,
                        candidate.task_id,
                        confidence
                    );
                } else {
                    let confidence_display = if confidence >= 85 {
                        format!("{confidence}% match").green().to_string()
                    } else if confidence >= 70 {
                        format!("{confidence}% match").yellow().to_string()
                    } else {
                        format!("{confidence}% match").dimmed().to_string()
                    };
                    println!(
                        "  {}. {} ({})",
                        format!("{}", idx + 1).bold(),
                        candidate.task_id.cyan(),
                        confidence_display
                    );
                }
            }
        }

        println!();

        // Prompt for decision
        loop {
            if candidates.is_empty() {
                if self.no_color {
                    print!("Action: [s]kip, [m]anual fix, [q]uit: ");
                } else {
                    print!(
                        "{} {}kip, {}anual fix, {}uit: ",
                        "Action:".bold(),
                        "[s]".green(),
                        "[m]".cyan(),
                        "[q]".red()
                    );
                }
            } else if self.no_color {
                print!(
                    "Action: [y]es (accept #1), [n]o (skip), [1-{}] (choose), [m]anual, [q]uit: ",
                    candidates.len().min(5)
                );
            } else {
                print!(
                    "{} {}es (accept #1), {}o (skip), {}1-{}{} (choose), {}anual, {}uit: ",
                    "Action:".bold(),
                    "[y]".green(),
                    "[n]".yellow(),
                    "[".dimmed(),
                    candidates.len().min(5),
                    "]".dimmed(),
                    "[m]".cyan(),
                    "[q]".red()
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
                    if self.no_color {
                        println!("Invalid choice. Please try again.");
                    } else {
                        println!("{}", "Invalid choice. Please try again.".red());
                    }
                }
                _ => {
                    if self.no_color {
                        println!("Invalid input. Please try again.");
                    } else {
                        println!("{}", "Invalid input. Please try again.".red());
                    }
                }
            }
        }
    }

    /// Prompt for manual fix input
    fn prompt_manual_fix(&self) -> Result<Option<FixDecision>> {
        if self.no_color {
            print!("Enter the correct task reference (or 'cancel'): ");
        } else {
            print!(
                "{} ",
                "Enter the correct task reference (or 'cancel'):".bold()
            );
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
        if self.no_color {
            println!("{}", "-".repeat(60));
        } else {
            println!("{}", "-".repeat(60).dimmed());
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

        if self.no_color {
            println!("Fix Summary:");
            println!("  Accepted: {accepted}");
            println!("  Manual: {manual}");
            println!("  Skipped: {skipped}");
            println!("  Total fixed: {}", accepted + manual);
        } else {
            println!("{}", "Fix Summary:".bold());
            println!("  {}: {}", "Accepted".green(), accepted.to_string().bold());
            println!("  {}: {}", "Manual".cyan(), manual.to_string().bold());
            println!("  {}: {}", "Skipped".yellow(), skipped.to_string().bold());
            println!(
                "  {}: {}",
                "Total fixed".bold(),
                (accepted + manual).to_string().green().bold()
            );
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
        let prompter = InteractivePrompter::new(false);
        assert!(!prompter.no_color);

        let prompter = InteractivePrompter::new(true);
        assert!(prompter.no_color);
    }

    // Note: Interactive tests would require mocking stdin/stdout,
    // which is complex and fragile. The interactive behavior will be
    // tested manually and through integration tests.
}
