//! Performance profiling and instrumentation for indexing operations
//!
//! This module provides structured performance tracking for the indexing engine,
//! measuring time spent in each phase and enabling analysis of performance bottlenecks.
//!
//! # Design Principles
//!
//! - **Low overhead**: Uses `tracing` spans with zero-cost abstractions when disabled
//! - **Structured data**: Outputs JSON for programmatic analysis
//! - **Non-invasive**: Minimal changes to existing code via drop-based timing
//! - **Configurable**: Can be enabled/disabled at runtime
//!
//! # Example
//!
//! ```
//! use lash_db::profiler::{IndexProfiler, ProfileReport};
//! use std::time::Duration;
//!
//! let mut profiler = IndexProfiler::new(true);
//!
//! // Time a phase
//! {
//!     let _guard = profiler.start_phase("discovery");
//!     // ... file discovery work ...
//! } // Automatically records time when guard drops
//!
//! // Record individual operation
//! profiler.record_file_parse("file.md", Duration::from_millis(5));
//!
//! // Get final report
//! let report = profiler.finish();
//! println!("{}", report.to_json_pretty());
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Main profiler for indexing operations
///
/// Tracks timing information for different phases of the indexing process.
/// All timing is done via RAII guards to ensure accurate measurement even
/// in the presence of early returns or errors.
#[derive(Debug)]
pub struct IndexProfiler {
    /// Whether profiling is enabled
    enabled: bool,
    /// Start time of the overall indexing operation
    start_time: Instant,
    /// Accumulated time for each phase
    phase_times: HashMap<String, Duration>,
    /// Per-file parse times
    file_parse_times: Vec<FileTiming>,
    /// Per-file hash computation times
    file_hash_times: Vec<FileTiming>,
    /// Database operation times
    db_operation_times: Vec<DbOperationTiming>,
    /// Current phase being timed (for nested phase detection)
    current_phase: Option<String>,
}

/// Timing information for a single file operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTiming {
    /// File path (relative to project root)
    pub path: String,
    /// Duration of the operation
    pub duration_us: u64,
}

/// Timing information for a database operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbOperationTiming {
    /// Type of operation (e.g., `insert_file`, `query_tasks`)
    pub operation: String,
    /// Number of rows affected/returned
    pub row_count: usize,
    /// Duration of the operation
    pub duration_us: u64,
}

/// RAII guard for automatic phase timing
///
/// When this guard is dropped, it automatically records the elapsed time
/// for the phase into the profiler.
#[must_use]
pub struct PhaseGuard<'a> {
    profiler: &'a mut IndexProfiler,
    phase: String,
    start: Instant,
}

impl Drop for PhaseGuard<'_> {
    fn drop(&mut self) {
        if self.profiler.enabled {
            let duration = self.start.elapsed();
            let entry = self
                .profiler
                .phase_times
                .entry(self.phase.clone())
                .or_insert(Duration::ZERO);
            *entry += duration;
            self.profiler.current_phase = None;
        }
    }
}

impl IndexProfiler {
    /// Create a new profiler
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    ///
    /// let profiler = IndexProfiler::new(true);
    /// assert!(profiler.is_enabled());
    /// ```
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            start_time: Instant::now(),
            phase_times: HashMap::new(),
            file_parse_times: Vec::new(),
            file_hash_times: Vec::new(),
            db_operation_times: Vec::new(),
            current_phase: None,
        }
    }

    /// Check if profiling is enabled
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    ///
    /// let profiler = IndexProfiler::new(false);
    /// assert!(!profiler.is_enabled());
    /// ```
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Start timing a phase
    ///
    /// Returns a guard that will automatically record the phase duration
    /// when dropped. This ensures accurate timing even with early returns.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    ///
    /// let mut profiler = IndexProfiler::new(true);
    /// {
    ///     let _guard = profiler.start_phase("discovery");
    ///     // Work happens here
    /// } // Time automatically recorded
    /// ```
    pub fn start_phase(&mut self, phase: &str) -> PhaseGuard<'_> {
        if self.enabled {
            self.current_phase = Some(phase.to_string());
        }
        PhaseGuard {
            profiler: self,
            phase: phase.to_string(),
            start: Instant::now(),
        }
    }

    /// Record a file parse operation
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    /// use std::time::Duration;
    ///
    /// let mut profiler = IndexProfiler::new(true);
    /// profiler.record_file_parse("tasks/task1.md", Duration::from_micros(1500));
    /// ```
    pub fn record_file_parse(&mut self, path: &str, duration: Duration) {
        if self.enabled {
            self.file_parse_times.push(FileTiming {
                path: path.to_string(),
                duration_us: duration.as_micros() as u64,
            });
        }
    }

    /// Record a file hash computation
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    /// use std::time::Duration;
    ///
    /// let mut profiler = IndexProfiler::new(true);
    /// profiler.record_file_hash("tasks/task1.md", Duration::from_micros(500));
    /// ```
    pub fn record_file_hash(&mut self, path: &str, duration: Duration) {
        if self.enabled {
            self.file_hash_times.push(FileTiming {
                path: path.to_string(),
                duration_us: duration.as_micros() as u64,
            });
        }
    }

    /// Record a database operation
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    /// use std::time::Duration;
    ///
    /// let mut profiler = IndexProfiler::new(true);
    /// profiler.record_db_operation("insert_tasks", 10, Duration::from_micros(2000));
    /// ```
    pub fn record_db_operation(&mut self, operation: &str, row_count: usize, duration: Duration) {
        if self.enabled {
            self.db_operation_times.push(DbOperationTiming {
                operation: operation.to_string(),
                row_count,
                duration_us: duration.as_micros() as u64,
            });
        }
    }

    /// Finish profiling and generate a report
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    ///
    /// let mut profiler = IndexProfiler::new(true);
    /// let _guard = profiler.start_phase("test");
    /// drop(_guard);
    ///
    /// let report = profiler.finish();
    /// assert!(report.phase_times.contains_key("test"));
    /// ```
    #[must_use]
    pub fn finish(self) -> ProfileReport {
        let total_duration = self.start_time.elapsed();

        ProfileReport {
            total_duration_ms: total_duration.as_millis() as u64,
            phase_times: self
                .phase_times
                .into_iter()
                .map(|(phase, duration)| (phase, duration.as_micros() as u64))
                .collect(),
            file_parse_times: self.file_parse_times,
            file_hash_times: self.file_hash_times,
            db_operation_times: self.db_operation_times,
        }
    }
}

/// Performance report with detailed timing breakdowns
///
/// This structure can be serialized to JSON for analysis and visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileReport {
    /// Total duration of the indexing operation in milliseconds
    pub total_duration_ms: u64,

    /// Time spent in each phase in microseconds
    pub phase_times: HashMap<String, u64>,

    /// Per-file parse times
    pub file_parse_times: Vec<FileTiming>,

    /// Per-file hash computation times
    pub file_hash_times: Vec<FileTiming>,

    /// Database operation times
    pub db_operation_times: Vec<DbOperationTiming>,
}

impl ProfileReport {
    /// Convert report to pretty-printed JSON
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::{IndexProfiler, ProfileReport};
    ///
    /// let profiler = IndexProfiler::new(true);
    /// let report = profiler.finish();
    /// let json = report.to_json_pretty();
    /// assert!(json.contains("total_duration_ms"));
    /// ```
    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Convert report to compact JSON
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::{IndexProfiler, ProfileReport};
    ///
    /// let profiler = IndexProfiler::new(true);
    /// let report = profiler.finish();
    /// let json = report.to_json();
    /// assert!(json.contains("total_duration_ms"));
    /// ```
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get statistics for a specific phase
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    ///
    /// let mut profiler = IndexProfiler::new(true);
    /// let _guard = profiler.start_phase("discovery");
    /// drop(_guard);
    ///
    /// let report = profiler.finish();
    /// let discovery_time = report.get_phase_time("discovery");
    /// assert!(discovery_time.is_some());
    /// ```
    #[must_use]
    pub fn get_phase_time(&self, phase: &str) -> Option<u64> {
        self.phase_times.get(phase).copied()
    }

    /// Get summary statistics for file parsing
    ///
    /// Returns (`count`, `total_us`, `avg_us`, `min_us`, `max_us`)
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    /// use std::time::Duration;
    ///
    /// let mut profiler = IndexProfiler::new(true);
    /// profiler.record_file_parse("a.md", Duration::from_micros(1000));
    /// profiler.record_file_parse("b.md", Duration::from_micros(2000));
    ///
    /// let report = profiler.finish();
    /// let stats = report.parse_stats();
    /// assert_eq!(stats.0, 2); // count
    /// assert_eq!(stats.1, 3000); // total
    /// assert_eq!(stats.2, 1500); // avg
    /// ```
    #[must_use]
    pub fn parse_stats(&self) -> (usize, u64, u64, u64, u64) {
        if self.file_parse_times.is_empty() {
            return (0, 0, 0, 0, 0);
        }

        let count = self.file_parse_times.len();
        let total: u64 = self.file_parse_times.iter().map(|t| t.duration_us).sum();
        let avg = total / count as u64;
        let min = self
            .file_parse_times
            .iter()
            .map(|t| t.duration_us)
            .min()
            .unwrap_or(0);
        let max = self
            .file_parse_times
            .iter()
            .map(|t| t.duration_us)
            .max()
            .unwrap_or(0);

        (count, total, avg, min, max)
    }

    /// Get summary statistics for hash computation
    ///
    /// Returns (`count`, `total_us`, `avg_us`, `min_us`, `max_us`)
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    /// use std::time::Duration;
    ///
    /// let mut profiler = IndexProfiler::new(true);
    /// profiler.record_file_hash("a.md", Duration::from_micros(500));
    /// profiler.record_file_hash("b.md", Duration::from_micros(700));
    ///
    /// let report = profiler.finish();
    /// let stats = report.hash_stats();
    /// assert_eq!(stats.0, 2); // count
    /// ```
    #[must_use]
    pub fn hash_stats(&self) -> (usize, u64, u64, u64, u64) {
        if self.file_hash_times.is_empty() {
            return (0, 0, 0, 0, 0);
        }

        let count = self.file_hash_times.len();
        let total: u64 = self.file_hash_times.iter().map(|t| t.duration_us).sum();
        let avg = total / count as u64;
        let min = self
            .file_hash_times
            .iter()
            .map(|t| t.duration_us)
            .min()
            .unwrap_or(0);
        let max = self
            .file_hash_times
            .iter()
            .map(|t| t.duration_us)
            .max()
            .unwrap_or(0);

        (count, total, avg, min, max)
    }

    /// Get summary statistics for database operations
    ///
    /// Returns (`operation_count`, `total_rows`, `total_duration_us`, `avg_duration_us`)
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::profiler::IndexProfiler;
    /// use std::time::Duration;
    ///
    /// let mut profiler = IndexProfiler::new(true);
    /// profiler.record_db_operation("insert", 10, Duration::from_micros(1000));
    /// profiler.record_db_operation("query", 5, Duration::from_micros(500));
    ///
    /// let report = profiler.finish();
    /// let stats = report.db_stats();
    /// assert_eq!(stats.0, 2); // operation count
    /// assert_eq!(stats.1, 15); // total rows
    /// ```
    #[must_use]
    pub fn db_stats(&self) -> (usize, usize, u64, u64) {
        if self.db_operation_times.is_empty() {
            return (0, 0, 0, 0);
        }

        let count = self.db_operation_times.len();
        let total_rows: usize = self.db_operation_times.iter().map(|op| op.row_count).sum();
        let total_duration: u64 = self
            .db_operation_times
            .iter()
            .map(|op| op.duration_us)
            .sum();
        let avg_duration = total_duration / count as u64;

        (count, total_rows, total_duration, avg_duration)
    }

    /// Print a human-readable summary to stdout
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::profiler::IndexProfiler;
    ///
    /// let profiler = IndexProfiler::new(true);
    /// let report = profiler.finish();
    /// report.print_summary();
    /// ```
    #[allow(clippy::uninlined_format_args)] // Clarity over brevity for reports
    pub fn print_summary(&self) {
        println!("\n=== Indexing Performance Report ===");
        println!("Total duration: {}ms", self.total_duration_ms);

        if !self.phase_times.is_empty() {
            println!("\nPhase times:");
            let mut phases: Vec<_> = self.phase_times.iter().collect();
            phases.sort_by_key(|(_, &duration)| std::cmp::Reverse(duration));
            for (phase, &duration_us) in phases {
                println!("  {}: {:.2}ms", phase, duration_us as f64 / 1000.0);
            }
        }

        let (parse_count, parse_total, parse_avg, parse_min, parse_max) = self.parse_stats();
        if parse_count > 0 {
            println!("\nFile parsing:");
            println!("  Files parsed: {}", parse_count);
            println!("  Total time: {:.2}ms", parse_total as f64 / 1000.0);
            println!("  Avg per file: {:.2}ms", parse_avg as f64 / 1000.0);
            println!(
                "  Min/Max: {:.2}ms / {:.2}ms",
                parse_min as f64 / 1000.0,
                parse_max as f64 / 1000.0
            );
        }

        let (hash_count, hash_total, hash_avg, hash_min, hash_max) = self.hash_stats();
        if hash_count > 0 {
            println!("\nHash computation:");
            println!("  Files hashed: {}", hash_count);
            println!("  Total time: {:.2}ms", hash_total as f64 / 1000.0);
            println!("  Avg per file: {:.2}ms", hash_avg as f64 / 1000.0);
            println!(
                "  Min/Max: {:.2}ms / {:.2}ms",
                hash_min as f64 / 1000.0,
                hash_max as f64 / 1000.0
            );
        }

        let (db_ops, db_rows, db_total, db_avg) = self.db_stats();
        if db_ops > 0 {
            println!("\nDatabase operations:");
            println!("  Operations: {}", db_ops);
            println!("  Total rows: {}", db_rows);
            println!("  Total time: {:.2}ms", db_total as f64 / 1000.0);
            println!("  Avg per op: {:.2}ms", db_avg as f64 / 1000.0);
        }

        println!();
    }
}

#[cfg(test)]
#[allow(clippy::duration_suboptimal_units)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_disabled() {
        let mut profiler = IndexProfiler::new(false);
        assert!(!profiler.is_enabled());

        let guard = profiler.start_phase("test");
        drop(guard);

        let report = profiler.finish();
        assert_eq!(report.phase_times.len(), 0);
    }

    #[test]
    fn test_profiler_enabled() {
        let mut profiler = IndexProfiler::new(true);
        assert!(profiler.is_enabled());

        {
            let _guard = profiler.start_phase("test");
            thread::sleep(Duration::from_millis(10));
        }

        let report = profiler.finish();
        assert!(report.phase_times.contains_key("test"));
        assert!(report.phase_times["test"] >= 10_000); // At least 10ms in microseconds
    }

    #[test]
    fn test_phase_accumulation() {
        let mut profiler = IndexProfiler::new(true);

        // Record same phase multiple times
        for _ in 0..3 {
            let _guard = profiler.start_phase("repeated");
            thread::sleep(Duration::from_millis(1));
        }

        let report = profiler.finish();
        assert!(report.phase_times["repeated"] >= 3_000); // At least 3ms
    }

    #[test]
    fn test_file_parse_recording() {
        let mut profiler = IndexProfiler::new(true);

        profiler.record_file_parse("a.md", Duration::from_micros(1000));
        profiler.record_file_parse("b.md", Duration::from_micros(2000));

        let report = profiler.finish();
        assert_eq!(report.file_parse_times.len(), 2);

        let stats = report.parse_stats();
        assert_eq!(stats.0, 2); // count
        assert_eq!(stats.1, 3000); // total
        assert_eq!(stats.2, 1500); // avg
        assert_eq!(stats.3, 1000); // min
        assert_eq!(stats.4, 2000); // max
    }

    #[test]
    fn test_file_hash_recording() {
        let mut profiler = IndexProfiler::new(true);

        profiler.record_file_hash("a.md", Duration::from_micros(500));
        profiler.record_file_hash("b.md", Duration::from_micros(700));

        let report = profiler.finish();
        assert_eq!(report.file_hash_times.len(), 2);

        let stats = report.hash_stats();
        assert_eq!(stats.0, 2);
        assert_eq!(stats.1, 1200);
    }

    #[test]
    fn test_db_operation_recording() {
        let mut profiler = IndexProfiler::new(true);

        profiler.record_db_operation("insert", 10, Duration::from_micros(1000));
        profiler.record_db_operation("query", 5, Duration::from_micros(500));

        let report = profiler.finish();
        assert_eq!(report.db_operation_times.len(), 2);

        let stats = report.db_stats();
        assert_eq!(stats.0, 2); // operations
        assert_eq!(stats.1, 15); // total rows
        assert_eq!(stats.2, 1500); // total duration
        assert_eq!(stats.3, 750); // avg duration
    }

    #[test]
    fn test_json_serialization() {
        let mut profiler = IndexProfiler::new(true);
        profiler.record_file_parse("test.md", Duration::from_micros(1000));

        let report = profiler.finish();
        let json = report.to_json();
        assert!(json.contains("total_duration_ms"));
        assert!(json.contains("file_parse_times"));

        // Test pretty version
        let pretty = report.to_json_pretty();
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn test_empty_stats() {
        let profiler = IndexProfiler::new(true);
        let report = profiler.finish();

        let (count, total, avg, min, max) = report.parse_stats();
        assert_eq!(count, 0);
        assert_eq!(total, 0);
        assert_eq!(avg, 0);
        assert_eq!(min, 0);
        assert_eq!(max, 0);
    }
}
