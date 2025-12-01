//! Output utilities for CLI commands
//!
//! This module provides helper functions for formatting output in CLI commands.

/// Create a progress bar for file processing operations
///
/// # Arguments
///
/// * `total_files` - Total number of files to process
///
/// # Returns
///
/// A configured progress bar
pub fn create_progress_bar(total_files: usize) -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new(total_files as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Invalid progress bar template")
            .progress_chars("#>-"),
    );
    pb
}
