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
/// use lash::progress::{ProgressReporter, TerminalProgressReporter};
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
    /// use lash::progress::TerminalProgressReporter;
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
    /// use lash::progress::JsonProgressReporter;
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
    /// use lash::progress::QuietProgressReporter;
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
    use std::time::Duration;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(1), "1s");
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(59), "59s");
        assert_eq!(format_duration(60), "1m 0s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(119), "1m 59s");
        assert_eq!(format_duration(120), "2m 0s");
        assert_eq!(format_duration(3599), "59m 59s");
        assert_eq!(format_duration(3600), "1h 0m");
        assert_eq!(format_duration(3660), "1h 1m");
        assert_eq!(format_duration(3665), "1h 1m");
        assert_eq!(format_duration(7200), "2h 0m");
        assert_eq!(format_duration(7260), "2h 1m");
        assert_eq!(format_duration(86400), "24h 0m");
    }

    #[test]
    fn test_terminal_reporter_new() {
        let reporter = TerminalProgressReporter::new();
        assert!(reporter.pb.is_none());
        assert_eq!(reporter.total, 0);
        assert!(reporter.start_time.is_none());
    }

    #[test]
    fn test_terminal_reporter_default() {
        let reporter = TerminalProgressReporter::default();
        assert!(reporter.pb.is_none());
        assert_eq!(reporter.total, 0);
        assert!(reporter.start_time.is_none());
    }

    #[test]
    fn test_terminal_reporter_items_per_second() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start_time = Some(Instant::now().checked_sub(Duration::from_secs(2)).unwrap());
        let rate = reporter.items_per_second(100);
        assert!(rate > 0.0);
        assert!(rate <= 100.0);
    }

    #[test]
    fn test_terminal_reporter_items_per_second_zero_elapsed() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start_time = Some(Instant::now());
        let rate = reporter.items_per_second(100);
        // Should return 0.0 when elapsed time is ~0
        assert!(rate >= 0.0);
    }

    #[test]
    fn test_terminal_reporter_items_per_second_no_start_time() {
        let reporter = TerminalProgressReporter::new();
        let rate = reporter.items_per_second(100);
        assert!((rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_terminal_reporter_eta() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.total = 100;
        reporter.start_time = Some(Instant::now().checked_sub(Duration::from_secs(1)).unwrap());

        // At 50% completion after 1 second, ETA should be ~1 second
        let eta = reporter.eta_seconds(50);
        assert!(eta.is_some());
        let eta_val = eta.unwrap();
        assert!(eta_val <= 2);
    }

    #[test]
    fn test_terminal_reporter_eta_zero_current() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.total = 100;
        reporter.start_time = Some(Instant::now());
        let eta = reporter.eta_seconds(0);
        assert!(eta.is_none());
    }

    #[test]
    fn test_terminal_reporter_eta_zero_total() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.total = 0;
        reporter.start_time = Some(Instant::now());
        let eta = reporter.eta_seconds(50);
        assert!(eta.is_none());
    }

    #[test]
    fn test_terminal_reporter_eta_no_rate() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.total = 100;
        reporter.start_time = None;
        let eta = reporter.eta_seconds(50);
        assert!(eta.is_none());
    }

    #[test]
    fn test_terminal_reporter_eta_complete() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.total = 100;
        reporter.start_time = Some(Instant::now().checked_sub(Duration::from_secs(1)).unwrap());
        // When current equals total, remaining is 0
        let eta = reporter.eta_seconds(100);
        assert!(eta.is_some());
        assert_eq!(eta.unwrap(), 0);
    }

    #[test]
    fn test_terminal_reporter_start() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start(100, "Processing");
        assert_eq!(reporter.total, 100);
        assert!(reporter.start_time.is_some());
        assert!(reporter.pb.is_some());
    }

    #[test]
    fn test_terminal_reporter_update() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start(100, "Processing");
        reporter.update(50, Some("Halfway done"));
        // Progress bar should still exist
        assert!(reporter.pb.is_some());
    }

    #[test]
    fn test_terminal_reporter_update_without_message() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start(100, "Processing");
        reporter.update(50, None);
        assert!(reporter.pb.is_some());
    }

    #[test]
    fn test_terminal_reporter_finish_with_message() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start(100, "Processing");
        reporter.finish(Some("Complete!"));
        assert!(reporter.pb.is_none());
        assert!(reporter.start_time.is_none());
    }

    #[test]
    fn test_terminal_reporter_finish_without_message() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start(100, "Processing");
        reporter.finish(None);
        assert!(reporter.pb.is_none());
        assert!(reporter.start_time.is_none());
    }

    #[test]
    fn test_terminal_reporter_set_message() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start(100, "Processing");
        reporter.set_message("New status");
        assert!(reporter.pb.is_some());
    }

    #[test]
    fn test_terminal_reporter_start_spinner() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start_spinner("Loading...");
        assert!(reporter.pb.is_some());
        assert!(reporter.start_time.is_some());
    }

    #[test]
    fn test_terminal_reporter_full_workflow() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start(100, "Starting");
        for i in 1..=100 {
            reporter.update(i, Some(&format!("Item {i}")));
        }
        reporter.finish(Some("Done!"));
        assert!(reporter.pb.is_none());
    }

    #[test]
    fn test_terminal_reporter_spinner_workflow() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start_spinner("Loading...");
        reporter.set_message("Still loading...");
        reporter.finish(Some("Loaded!"));
        assert!(reporter.pb.is_none());
    }

    #[test]
    fn test_json_reporter_new() {
        let reporter = JsonProgressReporter::new();
        assert_eq!(reporter.total, 0);
        assert!(reporter.start_time.is_none());
    }

    #[test]
    fn test_json_reporter_default() {
        let reporter = JsonProgressReporter::default();
        assert_eq!(reporter.total, 0);
        assert!(reporter.start_time.is_none());
    }

    #[test]
    fn test_json_reporter_start() {
        let mut reporter = JsonProgressReporter::new();
        reporter.start(100, "Processing");
        assert_eq!(reporter.total, 100);
        assert!(reporter.start_time.is_some());
    }

    #[test]
    fn test_json_reporter_update() {
        let mut reporter = JsonProgressReporter::new();
        reporter.start(100, "Processing");
        reporter.update(50, Some("Halfway"));
        // Just ensure it doesn't panic
    }

    #[test]
    fn test_json_reporter_update_without_message() {
        let mut reporter = JsonProgressReporter::new();
        reporter.start(100, "Processing");
        reporter.update(50, None);
        // Just ensure it doesn't panic
    }

    #[test]
    fn test_json_reporter_finish() {
        let mut reporter = JsonProgressReporter::new();
        reporter.start(100, "Processing");
        reporter.finish(Some("Complete"));
        assert!(reporter.start_time.is_none());
    }

    #[test]
    fn test_json_reporter_finish_without_message() {
        let mut reporter = JsonProgressReporter::new();
        reporter.start(100, "Processing");
        reporter.finish(None);
        assert!(reporter.start_time.is_none());
    }

    #[test]
    fn test_json_reporter_set_message() {
        let mut reporter = JsonProgressReporter::new();
        reporter.start(100, "Processing");
        reporter.set_message("New status");
        // Just ensure it doesn't panic
    }

    #[test]
    fn test_json_reporter_start_spinner() {
        let mut reporter = JsonProgressReporter::new();
        reporter.start_spinner("Loading...");
        assert!(reporter.start_time.is_some());
    }

    #[test]
    fn test_json_reporter_full_workflow() {
        let mut reporter = JsonProgressReporter::new();
        reporter.start(100, "Starting");
        for i in 1..=100 {
            reporter.update(i, Some(&format!("Item {i}")));
        }
        reporter.finish(Some("Done!"));
    }

    #[test]
    fn test_json_reporter_emit_event_zero_total() {
        let reporter = JsonProgressReporter::new();
        // Should handle zero total gracefully
        reporter.emit_event("test", 0, Some("message"));
    }

    #[test]
    fn test_json_reporter_emit_event_with_progress() {
        let mut reporter = JsonProgressReporter::new();
        reporter.total = 100;
        reporter.emit_event("progress", 50, Some("halfway"));
        // Just ensure it doesn't panic
    }

    #[test]
    fn test_json_reporter_percent_calculation() {
        let mut reporter = JsonProgressReporter::new();
        reporter.total = 100;
        reporter.start(100, "Test");
        reporter.update(25, None);
        reporter.update(50, None);
        reporter.update(75, None);
        reporter.update(100, None);
        // Just verify it doesn't panic with various percentages
    }

    #[test]
    fn test_quiet_reporter_new() {
        let reporter = QuietProgressReporter::new();
        // Just ensure it compiles
        let _ = reporter;
    }

    #[test]
    fn test_quiet_reporter_default() {
        let reporter = QuietProgressReporter;
        let _ = reporter;
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

    #[test]
    fn test_quiet_reporter_full_workflow() {
        let mut reporter = QuietProgressReporter::new();
        reporter.start(100, "Starting");
        for i in 1..=100 {
            reporter.update(i, Some(&format!("Item {i}")));
        }
        reporter.set_message("Final status");
        reporter.finish(Some("Done!"));
        // All operations should be silent no-ops
    }

    #[test]
    fn test_quiet_reporter_spinner_workflow() {
        let mut reporter = QuietProgressReporter::new();
        reporter.start_spinner("Loading...");
        reporter.set_message("Still loading...");
        reporter.finish(Some("Loaded!"));
        // All operations should be silent no-ops
    }

    #[test]
    fn test_progress_reporter_trait_terminal() {
        let mut reporter: Box<dyn ProgressReporter> = Box::new(TerminalProgressReporter::new());
        reporter.start(100, "Test");
        reporter.update(50, Some("Progress"));
        reporter.set_message("Update");
        reporter.finish(Some("Done"));
    }

    #[test]
    fn test_progress_reporter_trait_json() {
        let mut reporter: Box<dyn ProgressReporter> = Box::new(JsonProgressReporter::new());
        reporter.start(100, "Test");
        reporter.update(50, Some("Progress"));
        reporter.set_message("Update");
        reporter.finish(Some("Done"));
    }

    #[test]
    fn test_progress_reporter_trait_quiet() {
        let mut reporter: Box<dyn ProgressReporter> = Box::new(QuietProgressReporter::new());
        reporter.start(100, "Test");
        reporter.update(50, Some("Progress"));
        reporter.set_message("Update");
        reporter.finish(Some("Done"));
    }

    #[test]
    fn test_terminal_reporter_edge_case_zero_total() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start(0, "Empty task");
        reporter.update(0, None);
        reporter.finish(None);
    }

    #[test]
    fn test_terminal_reporter_edge_case_large_numbers() {
        let mut reporter = TerminalProgressReporter::new();
        let large_total = u64::MAX;
        reporter.start(large_total, "Huge task");
        reporter.update(large_total / 2, Some("Halfway"));
        reporter.finish(Some("Done"));
    }

    #[test]
    fn test_terminal_reporter_update_beyond_total() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start(100, "Task");
        reporter.update(150, Some("Over 100%"));
        reporter.finish(None);
    }

    #[test]
    fn test_json_reporter_edge_case_zero_total() {
        let mut reporter = JsonProgressReporter::new();
        reporter.start(0, "Empty task");
        reporter.update(0, None);
        reporter.finish(None);
    }

    #[test]
    fn test_json_reporter_edge_case_large_numbers() {
        let mut reporter = JsonProgressReporter::new();
        let large_total = u64::MAX;
        reporter.start(large_total, "Huge task");
        reporter.update(large_total / 2, Some("Halfway"));
        reporter.finish(Some("Done"));
    }

    #[test]
    fn test_format_duration_edge_cases() {
        // u64::MAX seconds = 18446744073709551615 seconds
        // = 5124095576030431 hours + 23 minutes
        assert_eq!(format_duration(u64::MAX), "5124095576030431h 0m");
    }

    #[test]
    fn test_terminal_reporter_multiple_starts() {
        let mut reporter = TerminalProgressReporter::new();
        reporter.start(100, "First task");
        reporter.finish(Some("Done"));
        reporter.start(50, "Second task");
        reporter.finish(Some("Done"));
    }

    #[test]
    fn test_json_reporter_multiple_starts() {
        let mut reporter = JsonProgressReporter::new();
        reporter.start(100, "First task");
        reporter.finish(Some("Done"));
        reporter.start(50, "Second task");
        reporter.finish(Some("Done"));
    }

    #[test]
    fn test_terminal_reporter_set_message_without_start() {
        let mut reporter = TerminalProgressReporter::new();
        // Should not panic if called before start
        reporter.set_message("message");
    }

    #[test]
    fn test_terminal_reporter_update_without_start() {
        let mut reporter = TerminalProgressReporter::new();
        // Should not panic if called before start
        reporter.update(50, Some("progress"));
    }

    #[test]
    fn test_terminal_reporter_finish_without_start() {
        let mut reporter = TerminalProgressReporter::new();
        // Should not panic if called before start
        reporter.finish(Some("done"));
    }

    #[test]
    fn test_terminal_reporter_update_with_message_no_eta() {
        let mut reporter = TerminalProgressReporter::new();
        // Start without setting start_time to ensure no ETA
        reporter.total = 100;
        reporter.pb = Some(indicatif::ProgressBar::new(100));
        reporter.start_time = None;
        // This should hit the path where there's a message but no ETA
        reporter.update(50, Some("Processing"));
    }
}
