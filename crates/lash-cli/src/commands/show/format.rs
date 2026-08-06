//! Shared status-formatting helpers for `lash show`'s text output.
//!
//! Split out of `show/mod.rs` alongside `file_view.rs`/`task_view.rs` to keep
//! that file under the project's ~500-line guideline. These three functions
//! are used by both the file and task text renderers, so they live here
//! rather than in either.

use lash_cli::theme::CliTheme;

/// Format task status with color
pub fn format_task_status(status: lash_types::TaskStatus, theme: Option<&CliTheme>) -> String {
    let status_str = match status {
        lash_types::TaskStatus::Open => "open",
        lash_types::TaskStatus::InProgress => "in-progress",
        lash_types::TaskStatus::Done => "done",
        lash_types::TaskStatus::Waived => "waived",
        lash_types::TaskStatus::Blocked => "blocked",
    };

    if let Some(theme) = theme {
        theme.style_task_status(status_str, status)
    } else {
        status_str.to_string()
    }
}

/// Format task status as icon
pub fn format_task_status_icon(status: lash_types::TaskStatus) -> &'static str {
    match status {
        lash_types::TaskStatus::Open => "[ ]",
        lash_types::TaskStatus::InProgress => "[>]",
        lash_types::TaskStatus::Done => "[x]",
        lash_types::TaskStatus::Waived => "[-]",
        lash_types::TaskStatus::Blocked => "[!]",
    }
}

/// Format file status with color
pub fn format_file_status(status: lash_types::FileStatus, theme: Option<&CliTheme>) -> String {
    let status_str = match status {
        lash_types::FileStatus::InProgress => "in-progress",
        lash_types::FileStatus::Complete => "complete",
        lash_types::FileStatus::Blocked => "blocked",
        lash_types::FileStatus::Empty => "empty",
    };

    if let Some(theme) = theme {
        match status {
            lash_types::FileStatus::InProgress => theme.style_info(status_str),
            lash_types::FileStatus::Complete => theme.style_success(status_str),
            lash_types::FileStatus::Blocked => theme.style_error(status_str),
            lash_types::FileStatus::Empty => theme.style_muted(status_str),
        }
    } else {
        status_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_task_status_icon() {
        assert_eq!(format_task_status_icon(lash_types::TaskStatus::Open), "[ ]");
        assert_eq!(format_task_status_icon(lash_types::TaskStatus::Done), "[x]");
        assert_eq!(
            format_task_status_icon(lash_types::TaskStatus::Waived),
            "[-]"
        );
        assert_eq!(
            format_task_status_icon(lash_types::TaskStatus::Blocked),
            "[!]"
        );
    }
}
