//! Progress reporting for long-running operations
//!
//! This module provides trait-based progress reporting with implementations for
//! terminal progress bars, JSON progress events, and quiet mode.

use indicatif::{ProgressBar, ProgressStyle};
use serde_json::json;
use std::time::Instant;

/// Progress reporter trait
///
/// Implementations provide progress reporting for different output modes.
///
/// # Example
///
/// ```no_run
/// use lash_cli::progress::{ProgressReporter, TerminalProgressReporter};
///
/// let mut reporter = TerminalProgressReporter::new();
/// reporter.start(100, "Processing files");
/// for i in 0..100 {
///     reporter.update(i + 1, Some(&format!("File {}", i + 1)));
/// }
/// reporter.finish(Some("Done!"));
/// ```
pub trait ProgressReporter {
    /// Begin a progress operation
    ///
    /// # Arguments
    ///
    /// * `total` - Total number of items to process
    /// * `message` - Initial status message
    fn start(&mut self, total: u64, message: &str);

    /// Update progress
    ///
    /// # Arguments
    ///
    /// * `current` - Current progress count
    /// * `message` - Optional status message
    fn update(&mut self, current: u64, message: Option<&str>);

    /// Complete the operation
    ///
    /// # Arguments
    ///
    /// * `message` - Optional completion message
    fn finish(&mut self, message: Option<&str>);

    /// Update the status message without changing progress
    ///
    /// # Arguments
    ///
    /// * `message` - New status message
    fn set_message(&mut self, message: &str);

    /// Start an indeterminate progress indicator (spinner)
    ///
    /// # Arguments
    ///
    /// * `message` - Status message
    fn start_spinner(&mut self, message: &str);
}

/// Terminal-based progress reporter using progress bars
///
/// Uses the `indicatif` crate to display progress bars in the terminal.
pub struct TerminalProgressReporter {
    pb: Option<ProgressBar>,
    start_time: Option<Instant>,
    total: u64,
}

impl TerminalProgressReporter {
    /// Create a new terminal progress reporter
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::progress::TerminalProgressReporter;
    ///
    /// let reporter = TerminalProgressReporter::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            pb: None,
            start_time: None,
            total: 0,
        }
    }

    /// Calculate items per second based on elapsed time
    fn items_per_second(&self, current: u64) -> f64 {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                return current as f64 / elapsed;
            }
        }
        0.0
    }

    /// Estimate time remaining in seconds
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn eta_seconds(&self, current: u64) -> Option<u64> {
        if current == 0 || self.total == 0 {
            return None;
        }

        let rate = self.items_per_second(current);
        if rate <= 0.0 {
            return None;
        }

        let remaining = self.total.saturating_sub(current);
        Some((remaining as f64 / rate) as u64)
    }
}

impl Default for TerminalProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for TerminalProgressReporter {
    fn start(&mut self, total: u64, message: &str) {
        self.total = total;
        self.start_time = Some(Instant::now());

        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg} [{elapsed_precise}]"
                )
                .expect("Invalid progress bar template")
                .progress_chars("=>-"),
        );
        pb.set_message(message.to_string());
        self.pb = Some(pb);
    }

    fn update(&mut self, current: u64, message: Option<&str>) {
        if let Some(pb) = &self.pb {
            pb.set_position(current);
            if let Some(msg) = message {
                // Add ETA if available
                if let Some(eta) = self.eta_seconds(current) {
                    let eta_str = format_duration(eta);
                    pb.set_message(format!("{msg} [ETA: {eta_str}]"));
                } else {
                    pb.set_message(msg.to_string());
                }
            }
        }
    }

    fn finish(&mut self, message: Option<&str>) {
        if let Some(pb) = &self.pb {
            if let Some(msg) = message {
                pb.finish_with_message(msg.to_string());
            } else {
                pb.finish_and_clear();
            }
        }
        self.pb = None;
        self.start_time = None;
    }

    fn set_message(&mut self, message: &str) {
        if let Some(pb) = &self.pb {
            pb.set_message(message.to_string());
        }
    }

    fn start_spinner(&mut self, message: &str) {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("Invalid spinner template")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        self.pb = Some(pb);
        self.start_time = Some(Instant::now());
    }
}

/// JSON-based progress reporter
///
/// Emits progress events as JSON lines to stdout.
pub struct JsonProgressReporter {
    total: u64,
    start_time: Option<Instant>,
}

impl JsonProgressReporter {
    /// Create a new JSON progress reporter
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::progress::JsonProgressReporter;
    ///
    /// let reporter = JsonProgressReporter::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            total: 0,
            start_time: None,
        }
    }

    /// Emit a JSON progress event
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn emit_event(&self, event_type: &str, current: u64, message: Option<&str>) {
        let percent = if self.total > 0 {
            (current as f64 / self.total as f64 * 100.0) as u64
        } else {
            0
        };

        let mut event = json!({
            "event": event_type,
            "current": current,
            "total": self.total,
            "percent": percent,
        });

        if let Some(msg) = message {
            event["message"] = json!(msg);
        }

        if let Ok(json_str) = serde_json::to_string(&event) {
            println!("{json_str}");
        }
    }
}

impl Default for JsonProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for JsonProgressReporter {
    fn start(&mut self, total: u64, message: &str) {
        self.total = total;
        self.start_time = Some(Instant::now());
        self.emit_event("start", 0, Some(message));
    }

    fn update(&mut self, current: u64, message: Option<&str>) {
        self.emit_event("progress", current, message);
    }

    fn finish(&mut self, message: Option<&str>) {
        self.emit_event("complete", self.total, message);
        self.start_time = None;
    }

    fn set_message(&mut self, message: &str) {
        self.emit_event("message", 0, Some(message));
    }

    fn start_spinner(&mut self, message: &str) {
        self.start_time = Some(Instant::now());
        self.emit_event("spinner", 0, Some(message));
    }
}

/// Quiet progress reporter (no-op)
///
/// Suppresses all progress output.
pub struct QuietProgressReporter;

impl QuietProgressReporter {
    /// Create a new quiet progress reporter
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::progress::QuietProgressReporter;
    ///
    /// let reporter = QuietProgressReporter::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuietProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for QuietProgressReporter {
    fn start(&mut self, _total: u64, _message: &str) {}
    fn update(&mut self, _current: u64, _message: Option<&str>) {}
    fn finish(&mut self, _message: Option<&str>) {}
    fn set_message(&mut self, _message: &str) {}
    fn start_spinner(&mut self, _message: &str) {}
}

/// Format a duration in seconds to a human-readable string
fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3665), "1h 1m");
        assert_eq!(format_duration(7200), "2h 0m");
    }

    #[test]
    fn test_terminal_reporter_new() {
        let reporter = TerminalProgressReporter::new();
        assert!(reporter.pb.is_none());
        assert_eq!(reporter.total, 0);
    }

    #[test]
    fn test_terminal_reporter_default() {
        let _reporter = TerminalProgressReporter::default();
        // Just ensure it compiles
    }

    #[test]
    fn test_terminal_reporter_items_per_second() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start_time = Some(
            Instant::now()
                .checked_sub(std::time::Duration::from_secs(2))
                .unwrap(),
        );
        let rate = reporter.items_per_second(100);
        assert!(rate > 0.0);
        assert!(rate <= 100.0);
    }

    #[test]
    fn test_terminal_reporter_eta() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.total = 100;
        reporter.start_time = Some(
            Instant::now()
                .checked_sub(std::time::Duration::from_secs(1))
                .unwrap(),
        );

        // At 50% completion after 1 second, ETA should be ~1 second
        let eta = reporter.eta_seconds(50);
        assert!(eta.is_some());
    }

    #[test]
    fn test_json_reporter_new() {
        let reporter = JsonProgressReporter::new();
        assert_eq!(reporter.total, 0);
    }

    #[test]
    fn test_json_reporter_default() {
        let _reporter = JsonProgressReporter::default();
        // Just ensure it compiles
    }

    #[test]
    fn test_quiet_reporter_new() {
        let _reporter = QuietProgressReporter::new();
        // Just ensure it compiles
    }

    #[test]
    fn test_quiet_reporter_default() {
        let _ = QuietProgressReporter;
        // Just ensure it compiles
    }

    #[test]
    fn test_quiet_reporter_no_op() {
        let mut reporter = QuietProgressReporter::new();
        // These should all be no-ops
        reporter.start(100, "test");
        reporter.update(50, Some("progress"));
        reporter.set_message("new message");
        reporter.start_spinner("spinning");
        reporter.finish(Some("done"));
    }
}
