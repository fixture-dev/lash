//! Status command implementation
//!
//! The `lash status` command provides a quick summary of project task status,
//! including counts by status and lists of in-progress, blocked, and recently
//! completed tasks.

use anyhow::{Context, Result};
use lash::error_reporter::{ErrorDisplayMode, ErrorReporter, ErrorReporterConfig};
use lash::formatter::{OutputFormat, Verbosity};
use lash::theme::CliTheme;
use lash_db::repository::tasks::TaskRecord;
use lash_db::{open_database, StatusCounts, TaskRepository};
use lash_types::error::LashError;
use lash_types::TaskStatus;
use serde::Serialize;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::utils::file_discovery::find_project_root;

/// Arguments for the status command
#[derive(Debug, Clone)]
pub struct StatusArgs {
    /// Maximum tasks per category
    pub limit: usize,
    /// Filter by labels (not yet implemented)
    #[allow(dead_code)]
    pub labels: Vec<String>,
    /// Filter by path prefix (not yet implemented)
    #[allow(dead_code)]
    pub path: Option<PathBuf>,
    /// Filter by owner (not yet implemented)
    #[allow(dead_code)]
    pub owner: Option<String>,
    /// Recency threshold string (e.g., "1d", "1w", "today")
    pub since: String,
    /// Minimal output for agents
    pub compact: bool,
    /// JSON output mode
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root (detected automatically if None)
    pub project_root: Option<PathBuf>,
    /// Verbosity level for output
    pub verbosity: Verbosity,
}

/// Output structure for status command (JSON mode)
#[derive(Debug, Serialize)]
pub struct StatusOutput {
    pub in_progress: Vec<TaskSummary>,
    pub blocked: Vec<TaskSummary>,
    pub recently_completed: Vec<TaskSummary>,
    pub recently_created: Vec<TaskSummary>,
    pub summary: StatusSummary,
}

/// Simplified task summary for output
#[derive(Debug, Serialize)]
pub struct TaskSummary {
    pub full_id: String,
    pub title: String,
    pub owner: Option<String>,
    pub estimate: Option<String>,
    pub labels: Vec<String>,
}

/// Summary counts for output
#[derive(Debug, Serialize)]
pub struct StatusSummary {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub done: usize,
    pub waived: usize,
    pub blocked: usize,
}

impl From<TaskRecord> for TaskSummary {
    fn from(task: TaskRecord) -> Self {
        Self {
            full_id: task.full_id,
            title: task.title,
            owner: task.owner,
            estimate: task.estimate,
            labels: task.metadata.labels,
        }
    }
}

impl From<StatusCounts> for StatusSummary {
    fn from(counts: StatusCounts) -> Self {
        Self {
            total: counts.total,
            open: counts.open,
            in_progress: counts.in_progress,
            done: counts.done,
            waived: counts.waived,
            blocked: counts.blocked,
        }
    }
}

/// Execute the status command
///
/// # Arguments
///
/// * `args` - Status command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 3 (DB error)
#[allow(clippy::too_many_lines)]
pub fn execute(args: &StatusArgs) -> Result<i32> {
    // Determine project root
    let project_root = if let Some(ref root) = args.project_root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        find_project_root(&cwd)
    };

    tracing::info!(
        project_root = %project_root.display(),
        limit = args.limit,
        since = %args.since,
        "Starting status operation"
    );

    // Load theme for colored output
    let theme = CliTheme::load(None, !args.no_color)?;

    // Determine database path
    let db_path = project_root.join(".lash/lash.db");

    // Check if database exists
    if !db_path.exists() {
        let error = LashError::io_file_not_found(db_path.clone());
        let mut diag = error.to_diagnostic();
        diag.help = Some("Run `lash index` to create the database".to_string());

        if args.json {
            output_json_error(&diag.message, "E_DB_NOT_FOUND")?;
        } else {
            let reporter_config = ErrorReporterConfig {
                verbosity: args.verbosity,
                output_format: OutputFormat::Text,
                display_mode: ErrorDisplayMode::Streaming,
                theme: theme.clone(),
                show_summary: false,
            };
            let mut reporter = ErrorReporter::new(reporter_config);
            reporter.report_diagnostic(&diag);
        }
        return Ok(3); // Exit code 3 for DB error
    }

    // Open database
    let conn = match open_database(&db_path) {
        Ok(conn) => conn,
        Err(e) => {
            let error = LashError::index_corrupted(format!("Failed to open database: {e}"));
            let mut diag = error.to_diagnostic();
            diag.help = Some("Try running `lash index` to rebuild the database".to_string());

            if args.json {
                output_json_error(&diag.message, "E_DB_CORRUPTED")?;
            } else {
                let reporter_config = ErrorReporterConfig {
                    verbosity: args.verbosity,
                    output_format: OutputFormat::Text,
                    display_mode: ErrorDisplayMode::Streaming,
                    theme: theme.clone(),
                    show_summary: false,
                };
                let mut reporter = ErrorReporter::new(reporter_config);
                reporter.report_diagnostic(&diag);
            }
            return Ok(3); // Exit code 3 for DB error
        }
    };

    // Create repository
    let task_repo = TaskRepository::new(&conn);

    // Get status counts
    let counts = task_repo
        .get_status_counts()
        .context("Failed to query status counts")?;

    // Get in-progress tasks (explicitly marked as in-progress)
    let in_progress = task_repo
        .find_by_status(TaskStatus::InProgress)
        .context("Failed to query in-progress tasks")?;
    let in_progress: Vec<TaskRecord> = in_progress.into_iter().take(args.limit).collect();

    // Get blocked tasks
    let blocked = task_repo
        .find_by_status(TaskStatus::Blocked)
        .context("Failed to query blocked tasks")?;
    let blocked: Vec<TaskRecord> = blocked.into_iter().take(args.limit).collect();

    // Get recently completed tasks
    let since_timestamp = parse_since_duration(&args.since)?;
    let recently_completed = task_repo
        .find_recently_completed(since_timestamp, args.limit)
        .context("Failed to query recently completed tasks")?;

    // Get recently created tasks
    let recently_created = task_repo
        .find_recently_created(since_timestamp, args.limit)
        .context("Failed to query recently created tasks")?;

    // Output results
    if args.json {
        output_json(
            &in_progress,
            &blocked,
            &recently_completed,
            &recently_created,
            &counts,
        )?;
    } else if args.compact {
        output_compact(
            &in_progress,
            &blocked,
            &recently_completed,
            &recently_created,
            &counts,
        );
    } else {
        output_text(
            &in_progress,
            &blocked,
            &recently_completed,
            &recently_created,
            &counts,
            &args.since,
            theme.as_ref(),
        );
    }

    Ok(0)
}

/// Parse a duration string like "1d", "1w", "today" into a Unix timestamp
///
/// Returns the timestamp representing that many seconds ago from now.
fn parse_since_duration(since: &str) -> Result<i64> {
    #[allow(clippy::cast_possible_wrap)]
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("Failed to get current time")?
        .as_secs() as i64;

    let seconds_ago = match since {
        "today" => {
            // Start of today (midnight)
            let now_local = chrono::Local::now();
            let today_start = now_local.date_naive().and_hms_opt(0, 0, 0).unwrap();
            let today_start_ts = today_start
                .and_local_timezone(chrono::Local)
                .single()
                .unwrap()
                .timestamp();
            return Ok(today_start_ts);
        }
        s if s.ends_with('h') => {
            let hours: u64 = s[..s.len() - 1].parse().context("Invalid hour format")?;
            hours * 3_600
        }
        s if s.ends_with('d') => {
            let days: u64 = s[..s.len() - 1].parse().context("Invalid day format")?;
            days * 86_400
        }
        s if s.ends_with('w') => {
            let weeks: u64 = s[..s.len() - 1].parse().context("Invalid week format")?;
            weeks * 604_800
        }
        s if s.ends_with('m') => {
            let minutes: u64 = s[..s.len() - 1].parse().context("Invalid minute format")?;
            minutes * 60
        }
        _ => anyhow::bail!(
            "Invalid since format: {since}. Use format like '1d', '1w', '2h', or 'today'"
        ),
    };

    #[allow(clippy::cast_possible_wrap)]
    let seconds_ago_i64 = seconds_ago as i64;
    Ok(now - seconds_ago_i64)
}

/// Output status as JSON
fn output_json(
    in_progress: &[TaskRecord],
    blocked: &[TaskRecord],
    recently_completed: &[TaskRecord],
    recently_created: &[TaskRecord],
    counts: &StatusCounts,
) -> Result<()> {
    let output = StatusOutput {
        in_progress: in_progress.iter().map(|t| t.clone().into()).collect(),
        blocked: blocked.iter().map(|t| t.clone().into()).collect(),
        recently_completed: recently_completed
            .iter()
            .map(|t| t.clone().into())
            .collect(),
        recently_created: recently_created.iter().map(|t| t.clone().into()).collect(),
        summary: counts.clone().into(),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output status in compact format
fn output_compact(
    in_progress: &[TaskRecord],
    blocked: &[TaskRecord],
    recently_completed: &[TaskRecord],
    recently_created: &[TaskRecord],
    counts: &StatusCounts,
) {
    println!("in_progress: {}", in_progress.len());
    println!("blocked: {}", blocked.len());
    println!("recent_done: {}", recently_completed.len());
    println!("recent_created: {}", recently_created.len());
    println!(
        "total: {} open: {} in_progress: {} done: {} waived: {} blocked: {}",
        counts.total, counts.open, counts.in_progress, counts.done, counts.waived, counts.blocked
    );
}

/// Output status in human-readable text format
#[allow(clippy::too_many_lines)]
fn output_text(
    in_progress: &[TaskRecord],
    blocked: &[TaskRecord],
    recently_completed: &[TaskRecord],
    recently_created: &[TaskRecord],
    counts: &StatusCounts,
    since: &str,
    theme: Option<&CliTheme>,
) {
    // Header
    if let Some(t) = theme {
        println!("{}", t.style_label("Project Status"));
        println!("{}", t.style_muted("=============="));
    } else {
        println!("Project Status");
        println!("==============");
    }
    println!();

    // In Progress section
    if !in_progress.is_empty() {
        if let Some(t) = theme {
            println!("{} ({})", t.style_info("In Progress"), in_progress.len());
        } else {
            println!("In Progress ({})", in_progress.len());
        }

        for task in in_progress {
            print_task_summary(task, theme);
        }
        println!();
    }

    // Blocked section
    if !blocked.is_empty() {
        if let Some(t) = theme {
            println!("{} ({})", t.style_error("Blocked"), blocked.len());
        } else {
            println!("Blocked ({})", blocked.len());
        }

        for task in blocked {
            print_task_summary(task, theme);
        }
        println!();
    }

    // Recently Completed section
    if !recently_completed.is_empty() {
        if let Some(t) = theme {
            println!(
                "{} ({} in last {since})",
                t.style_success("Recently Completed"),
                recently_completed.len()
            );
        } else {
            println!(
                "Recently Completed ({} in last {since})",
                recently_completed.len()
            );
        }

        for task in recently_completed {
            print_task_summary(task, theme);
        }
        println!();
    }

    // Recently Created section
    if !recently_created.is_empty() {
        if let Some(t) = theme {
            println!(
                "{} ({} in last {since})",
                t.style_info("Recently Created"),
                recently_created.len()
            );
        } else {
            println!(
                "Recently Created ({} in last {since})",
                recently_created.len()
            );
        }

        for task in recently_created {
            print_task_summary(task, theme);
        }
        println!();
    }

    // Summary section - format percentages with <1% / >99% for non-zero edge cases
    let [open_pct, in_progress_pct, done_pct, waived_pct, blocked_pct] = format_percentages(
        counts.total,
        [
            counts.open,
            counts.in_progress,
            counts.done,
            counts.waived,
            counts.blocked,
        ],
    );

    if let Some(t) = theme {
        println!("{}", t.style_label("Summary"));
        println!(
            "  Total: {} | Open: {} ({open_pct}%) | In Progress: {} ({in_progress_pct}%) | Done: {} ({done_pct}%) | Waived: {} ({waived_pct}%) | Blocked: {} ({blocked_pct}%)",
            counts.total, counts.open, counts.in_progress, counts.done, counts.waived, counts.blocked
        );
    } else {
        println!("Summary");
        println!(
            "  Total: {} | Open: {} ({open_pct}%) | In Progress: {} ({in_progress_pct}%) | Done: {} ({done_pct}%) | Waived: {} ({waived_pct}%) | Blocked: {} ({blocked_pct}%)",
            counts.total, counts.open, counts.in_progress, counts.done, counts.waived, counts.blocked
        );
    }
}

/// Format percentages for display, using `<1` when a count is non-zero but rounds
/// to 0%, and `>99` when a count is less than total but rounds to 100%.
///
/// This prevents misleading output like "Open: 7 (0%) | Done: 2158 (100%)" which
/// makes it look like everything is complete when it isn't.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn format_percentages<const N: usize>(total: usize, counts: [usize; N]) -> [String; N] {
    if total == 0 {
        return std::array::from_fn(|_| "0".to_string());
    }

    let total_f = total as f64;
    let exact: Vec<f64> = counts.iter().map(|&c| c as f64 / total_f * 100.0).collect();
    let rounded: Vec<u32> = exact.iter().map(|&v| v.round() as u32).collect();

    std::array::from_fn(|i| {
        if counts[i] == 0 {
            "0".to_string()
        } else if counts[i] == total {
            "100".to_string()
        } else if rounded[i] == 0 {
            "<1".to_string()
        } else if rounded[i] >= 100 {
            ">99".to_string()
        } else {
            rounded[i].to_string()
        }
    })
}

/// Print a single task summary line
fn print_task_summary(task: &TaskRecord, theme: Option<&CliTheme>) {
    let checkbox = match task.status {
        TaskStatus::Open => "[ ]",
        TaskStatus::InProgress => "[>]",
        TaskStatus::Done => "[x]",
        TaskStatus::Waived => "[-]",
        TaskStatus::Blocked => "[!]",
    };

    let mut parts = Vec::new();

    // Checkbox
    if let Some(t) = theme {
        let styled_checkbox = match task.status {
            TaskStatus::Open => t.style_muted(checkbox),
            TaskStatus::InProgress => t.style_task_status(checkbox, task.status),
            TaskStatus::Done => t.style_success(checkbox),
            TaskStatus::Waived => t.style_warning(checkbox),
            TaskStatus::Blocked => t.style_error(checkbox),
        };
        parts.push(styled_checkbox);
    } else {
        parts.push(checkbox.to_string());
    }

    // Title
    parts.push(task.title.clone());

    // Task ID
    let task_id = format!("({})", task.full_id);
    if let Some(t) = theme {
        parts.push(t.style_muted(&task_id));
    } else {
        parts.push(task_id);
    }

    // Owner
    if let Some(ref owner) = task.owner {
        let owner_str = format!("@{owner}");
        if let Some(t) = theme {
            parts.push(t.style_muted(&owner_str));
        } else {
            parts.push(owner_str);
        }
    }

    // Estimate
    if let Some(ref estimate) = task.estimate {
        let estimate_str = format!("~{estimate}");
        if let Some(t) = theme {
            parts.push(t.style_muted(&estimate_str));
        } else {
            parts.push(estimate_str);
        }
    }

    println!("  {}", parts.join(" "));
}

/// Output a single JSON error
fn output_json_error(message: &str, code: &str) -> Result<()> {
    let json = serde_json::json!({
        "error": {
            "code": code,
            "message": message,
        },
    });
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn test_parse_since_duration_days() {
        let result = parse_since_duration("7d").unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let expected = now - (7 * 86_400);
        // Allow for small timing differences
        assert!((result - expected).abs() < 2);
    }

    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn test_parse_since_duration_weeks() {
        let result = parse_since_duration("2w").unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let expected = now - (2 * 604_800);
        assert!((result - expected).abs() < 2);
    }

    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn test_parse_since_duration_hours() {
        let result = parse_since_duration("24h").unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let expected = now - (24 * 3_600);
        assert!((result - expected).abs() < 2);
    }

    #[test]
    fn test_parse_since_duration_invalid() {
        let result = parse_since_duration("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_status_summary_from_counts() {
        let counts = StatusCounts {
            total: 100,
            open: 45,
            in_progress: 5,
            done: 30,
            waived: 15,
            blocked: 5,
        };
        let summary: StatusSummary = counts.into();
        assert_eq!(summary.total, 100);
        assert_eq!(summary.open, 45);
        assert_eq!(summary.in_progress, 5);
        assert_eq!(summary.done, 30);
        assert_eq!(summary.waived, 15);
        assert_eq!(summary.blocked, 5);
    }

    #[test]
    fn test_format_percentages_exact() {
        let result = format_percentages(100, [25, 25, 25, 25]);
        assert_eq!(result, ["25", "25", "25", "25"]);
    }

    #[test]
    fn test_format_percentages_zero_total() {
        let result = format_percentages(0, [0, 0, 0, 0]);
        assert_eq!(result, ["0", "0", "0", "0"]);
    }

    #[test]
    fn test_format_percentages_172_of_173() {
        // 172/173 = 99.42%, 1/173 = 0.58%
        let result = format_percentages(173, [0, 172, 1, 0]);
        assert_eq!(result, ["0", "99", "1", "0"]);
    }

    #[test]
    fn test_format_percentages_thirds() {
        let result = format_percentages(3, [1, 1, 1]);
        assert!(result.iter().all(|v| v == "33" || v == "34"));
    }

    #[test]
    fn test_format_percentages_all_in_one() {
        let result = format_percentages(50, [50, 0, 0, 0]);
        assert_eq!(result, ["100", "0", "0", "0"]);
    }

    #[test]
    fn test_format_percentages_small_nonzero_shows_less_than_1() {
        // 7/2165 = 0.32% — should show "<1", not "0"
        // 2158/2165 = 99.68% — should show ">99", not "100"
        let result = format_percentages(2165, [7, 2158, 0, 0]);
        assert_eq!(result, ["<1", ">99", "0", "0"]);
    }

    #[test]
    fn test_format_percentages_exactly_100_percent() {
        // Only count == total should show "100"
        let result = format_percentages(50, [0, 50, 0, 0]);
        assert_eq!(result, ["0", "100", "0", "0"]);
    }

    #[test]
    fn test_format_percentages_just_above_zero() {
        // 1/100 = 1% — should show "1", not "<1"
        let result = format_percentages(100, [1, 99, 0, 0]);
        assert_eq!(result, ["1", "99", "0", "0"]);
    }
}
