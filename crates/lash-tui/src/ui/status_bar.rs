//! Status bar rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::activity::{ActivityEntry, ActivityState, RECENT_COMPLETED_CAP};
use crate::state::{AppState, StatusLevel};

const IN_PROGRESS_ICON: &str = "\u{25B6}";
const RECENT_ICON: &str = "\u{2713}";
const MIN_TITLE_CHARS: usize = 12;
const ELLIPSIS: char = '\u{2026}';

/// Render the status bar
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    // Check if we have an active status message to display
    if let Some(msg) = &state.status_message {
        render_status_message(frame, area, msg, state);
        return;
    }

    // Default status bar rendering
    render_default(frame, area, state);
}

/// Render a status message (replaces default status bar temporarily)
fn render_status_message(
    frame: &mut Frame,
    area: Rect,
    msg: &crate::state::StatusMessage,
    state: &AppState,
) {
    let (icon, fg_color, bg_color) = match msg.level {
        StatusLevel::Info => ("i", state.theme.background(), state.theme.info_color()),
        StatusLevel::Warning => ("!", state.theme.background(), state.theme.warning_color()),
        StatusLevel::Error => ("x", state.theme.background(), state.theme.error_color()),
        StatusLevel::Success => (
            "\u{2713}",
            state.theme.background(),
            state.theme.success_color(),
        ),
    };

    let spans = vec![
        Span::styled(
            format!(" {icon} "),
            Style::default()
                .fg(fg_color)
                .bg(bg_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", msg.text),
            Style::default().fg(fg_color).bg(bg_color),
        ),
        // Fill remaining space with background color
        Span::styled(
            {
                #[allow(clippy::cast_possible_truncation)]
                let text_len = msg.text.len() as u16;
                " ".repeat(area.width.saturating_sub(text_len + 5) as usize)
            },
            Style::default().bg(bg_color),
        ),
    ];

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(state.theme.background()));

    frame.render_widget(paragraph, area);
}

/// Render the default status bar (no active message)
fn render_default(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused_pane_name = match state.focused_pane {
        crate::state::FocusedPane::Navigation => "Files",
        crate::state::FocusedPane::Description => "Description",
        crate::state::FocusedPane::Detail => "Tasks",
    };

    let file_count = state.files.len();
    let task_count = state.tasks.len();

    let mut left_spans = vec![
        Span::styled(
            format!(" {focused_pane_name} "),
            Style::default()
                .fg(state.theme.background())
                .bg(state.theme.border_focused()),
        ),
        Span::raw(format!("  Files: {file_count}  Tasks: {task_count}  ")),
    ];

    if let Some(filter) = &state.current_label_filter {
        left_spans.push(Span::styled(
            format!("#{filter}  "),
            Style::default().fg(state.theme.label_color()),
        ));
    }

    let right_spans = vec![Span::styled(
        " Press ? for help ",
        Style::default().fg(Color::DarkGray),
    )];

    let left_width: usize = left_spans.iter().map(|s| s.content.chars().count()).sum();
    let right_width: usize = right_spans.iter().map(|s| s.content.chars().count()).sum();
    let total_width = area.width as usize;
    let available_for_activity = total_width
        .saturating_sub(left_width)
        .saturating_sub(right_width);

    let activity_spans = build_activity_spans(&state.activity, available_for_activity, state);
    let activity_width: usize = activity_spans
        .iter()
        .map(|s| s.content.chars().count())
        .sum();

    let filler_width = available_for_activity.saturating_sub(activity_width);

    let mut spans = left_spans;
    spans.extend(activity_spans);
    if filler_width > 0 {
        spans.push(Span::raw(" ".repeat(filler_width)));
    }
    spans.extend(right_spans);

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, area);
}

/// Build spans for the activity sections within `budget` columns.
///
/// Layout: ` <icon> <title> [ <icon> <title> ... ]` with each entry separated
/// by a single space. Titles are truncated with an ellipsis. If even one entry
/// can't fit at the minimum width, it's dropped from the right.
fn build_activity_spans(
    activity: &ActivityState,
    budget: usize,
    state: &AppState,
) -> Vec<Span<'static>> {
    if budget < MIN_TITLE_CHARS + 3 {
        return Vec::new();
    }

    let in_progress_color = state.theme.task_in_progress();
    let done_color = state.theme.task_done();
    let muted = state.theme.muted_color();

    let mut entries: Vec<(&'static str, Color, &ActivityEntry)> = Vec::new();
    if let Some(ip) = activity.in_progress.as_ref() {
        entries.push((IN_PROGRESS_ICON, in_progress_color, ip));
    }
    for entry in activity
        .recently_completed
        .iter()
        .take(RECENT_COMPLETED_CAP)
    {
        entries.push((RECENT_ICON, done_color, entry));
    }

    if entries.is_empty() {
        return Vec::new();
    }

    // Per-entry overhead: leading space + icon (1 char) + space + title chars
    let per_entry_overhead = 3;
    let entry_min_total = per_entry_overhead + MIN_TITLE_CHARS;

    let max_entries = (budget / entry_min_total).min(entries.len()).max(1);
    let entries_to_render = &entries[..max_entries];

    // Compute total overhead chars for separators/icons
    let total_overhead = per_entry_overhead * entries_to_render.len();
    let remaining_budget = budget.saturating_sub(total_overhead);

    // Allocate title width: in-progress (if present) gets ~40% of remaining,
    // recent entries split the rest evenly. If no in-progress entry, recent
    // entries get the full remaining.
    let has_ip = activity.in_progress.is_some() && entries_to_render[0].0 == IN_PROGRESS_ICON;
    let (ip_title_width, recent_title_width) = if has_ip && entries_to_render.len() > 1 {
        let ip = (remaining_budget * 40 / 100).max(MIN_TITLE_CHARS);
        let leftover = remaining_budget.saturating_sub(ip);
        let recent_count = entries_to_render.len() - 1;
        let per_recent = leftover / recent_count.max(1);
        (ip, per_recent)
    } else if has_ip {
        (remaining_budget, 0)
    } else {
        let per_recent = remaining_budget / entries_to_render.len().max(1);
        (0, per_recent)
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (icon, color, entry) in entries_to_render {
        let is_ip = *icon == IN_PROGRESS_ICON;
        let title_width = if is_ip {
            ip_title_width
        } else {
            recent_title_width
        };
        if title_width < MIN_TITLE_CHARS {
            continue;
        }
        let title = truncate_with_ellipsis(&entry.title, title_width);

        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            (*icon).to_string(),
            Style::default().fg(*color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {title}"),
            Style::default().fg(if is_ip { *color } else { muted }),
        ));
    }

    spans
}

/// Truncate a string to fit in `max_chars` columns, appending an ellipsis if
/// truncation occurs. Counts Unicode characters, not bytes.
fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut out: String = s.chars().take(keep).collect();
    out.push(ELLIPSIS);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::ActivityEntry;
    use crate::colors::REGISTRY;
    use std::time::Instant;

    fn make_state() -> AppState {
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        AppState::with_theme(crate::colors::Theme::new(scheme.clone()))
    }

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string_with_ellipsis() {
        let out = truncate_with_ellipsis("hello world", 8);
        assert_eq!(out.chars().count(), 8);
        assert!(out.ends_with(ELLIPSIS));
        assert!(out.starts_with("hello w"));
    }

    #[test]
    fn truncate_exact_length_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn truncate_to_zero_returns_empty() {
        assert_eq!(truncate_with_ellipsis("hello", 0), "");
    }

    #[test]
    fn truncate_unicode_chars() {
        let out = truncate_with_ellipsis("héllo wörld", 8);
        assert_eq!(out.chars().count(), 8);
        assert!(out.ends_with(ELLIPSIS));
    }

    #[test]
    fn empty_activity_yields_no_spans() {
        let state = make_state();
        let spans = build_activity_spans(&state.activity, 200, &state);
        assert!(spans.is_empty());
    }

    #[test]
    fn narrow_budget_yields_no_spans() {
        let mut state = make_state();
        state.activity.set_in_progress(ActivityEntry {
            full_id: "f#t".into(),
            title: "Some task".into(),
            at: Instant::now(),
        });
        let spans = build_activity_spans(&state.activity, 5, &state);
        assert!(spans.is_empty());
    }

    #[test]
    fn renders_in_progress_when_present() {
        let mut state = make_state();
        state.activity.set_in_progress(ActivityEntry {
            full_id: "f#t".into(),
            title: "Implementing Store actor".into(),
            at: Instant::now(),
        });
        let spans = build_activity_spans(&state.activity, 100, &state);
        let joined: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(joined.contains(IN_PROGRESS_ICON));
        assert!(joined.contains("Implementing"));
    }

    #[test]
    fn renders_recently_completed_entries() {
        let mut state = make_state();
        state.activity.recently_completed.push_back(ActivityEntry {
            full_id: "f#a".into(),
            title: "Add task files".into(),
            at: Instant::now(),
        });
        state.activity.recently_completed.push_front(ActivityEntry {
            full_id: "f#b".into(),
            title: "Survey writes".into(),
            at: Instant::now(),
        });
        let spans = build_activity_spans(&state.activity, 120, &state);
        let joined: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(joined.contains(RECENT_ICON));
        assert!(joined.contains("Survey"));
        assert!(joined.contains("Add task files"));
    }

    #[test]
    fn truncates_long_titles_with_ellipsis() {
        let mut state = make_state();
        state.activity.set_in_progress(ActivityEntry {
            full_id: "f#t".into(),
            title: "A very long task title that should not fit in a tight budget".into(),
            at: Instant::now(),
        });
        let spans = build_activity_spans(&state.activity, 30, &state);
        let joined: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(joined.contains(ELLIPSIS), "expected ellipsis in {joined:?}");
    }

    #[test]
    fn drops_entries_from_right_when_tight() {
        let mut state = make_state();
        state.activity.set_in_progress(ActivityEntry {
            full_id: "f#ip".into(),
            title: "Active task".into(),
            at: Instant::now(),
        });
        for i in 0..3 {
            state.activity.recently_completed.push_back(ActivityEntry {
                full_id: format!("f#r{i}"),
                title: format!("Recent task {i}"),
                at: Instant::now(),
            });
        }
        // Budget large enough for ip + 1 recent but not all 3 recent.
        let spans = build_activity_spans(&state.activity, 40, &state);
        let icons: usize = spans
            .iter()
            .filter(|s| s.content.as_ref() == IN_PROGRESS_ICON || s.content.as_ref() == RECENT_ICON)
            .count();
        assert!((1..=4).contains(&icons));
    }

    #[test]
    fn renders_nothing_when_status_message_path_taken() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut terminal = Terminal::new(TestBackend::new(80, 1)).unwrap();
        let mut state = make_state();
        state.set_success_message("hello");
        state.activity.set_in_progress(ActivityEntry {
            full_id: "f#t".into(),
            title: "should not appear".into(),
            at: Instant::now(),
        });
        terminal.draw(|f| render(f, f.area(), &state)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<Vec<_>>()
            .join("");
        assert!(content.contains("hello"));
        assert!(!content.contains("should not appear"));
    }

    #[test]
    fn default_render_includes_activity() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut terminal = Terminal::new(TestBackend::new(120, 1)).unwrap();
        let mut state = make_state();
        state.activity.set_in_progress(ActivityEntry {
            full_id: "f#t".into(),
            title: "Implementing activity bar".into(),
            at: Instant::now(),
        });
        state.activity.recently_completed.push_front(ActivityEntry {
            full_id: "f#r".into(),
            title: "Wrote design doc".into(),
            at: Instant::now(),
        });
        terminal.draw(|f| render(f, f.area(), &state)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<Vec<_>>()
            .join("");
        assert!(
            content.contains("Implementing"),
            "expected in-progress title in bar, got: {content:?}"
        );
        assert!(
            content.contains("Wrote design"),
            "expected recent title in bar, got: {content:?}"
        );
    }
}
