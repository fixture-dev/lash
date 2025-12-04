//! Logo rendering for the TUI
//!
//! Displays the Lash ASCII logo in the upper-left corner of the TUI,
//! along with project title and completion progress bar.

use lash_core::logo::LOGO_LINES;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::colors::Theme;
use crate::state::ProjectStats;
use crate::ui::themes;

/// The height of the logo widget (lines + border)
pub const LOGO_HEIGHT: u16 = 5; // 3 logo lines + 2 border lines

/// Width of the logo in characters
const LOGO_WIDTH: usize = 8;

/// Render the logo widget with project info
pub fn render(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    is_focused: bool,
    project_stats: &ProjectStats,
) {
    let border_style = if is_focused {
        themes::focused_border_style(theme)
    } else {
        themes::unfocused_border_style(theme)
    };

    // Calculate available width for project info (total width - logo - borders - padding)
    let inner_width = area.width.saturating_sub(2) as usize; // subtract borders
    let logo_padding = 2; // spacing between logo and project info
    let info_width = inner_width.saturating_sub(LOGO_WIDTH + logo_padding);

    // Create styled logo using the emphasis color
    let logo_style = Style::default().fg(theme.emphasis_color());
    let info_style = Style::default().fg(theme.foreground());
    let progress_style = Style::default().fg(theme.success_color());
    let progress_empty_style = Style::default().fg(theme.muted_color());

    // Build lines combining logo and project info
    let mut lines: Vec<Line<'static>> = Vec::new();

    for (i, logo_line) in LOGO_LINES.iter().enumerate() {
        let mut spans = vec![Span::styled((*logo_line).to_string(), logo_style)];

        if info_width > 4 {
            // Only show info if we have enough space
            match i {
                0 => {
                    // First line: project title
                    spans.push(Span::raw("  ")); // padding after logo
                    if let Some(title) = &project_stats.title {
                        let truncated = truncate_title(title, info_width);
                        spans.push(Span::styled(truncated, info_style));
                    }
                }
                1 => {
                    // Second line: progress bar
                    spans.push(Span::raw("  ")); // padding after logo
                    let progress_spans = build_progress_bar(
                        project_stats,
                        info_width,
                        progress_style,
                        progress_empty_style,
                        info_style,
                    );
                    spans.extend(progress_spans);
                }
                _ => {
                    // Third line: empty (just logo)
                }
            }
        }

        lines.push(Line::from(spans));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    let paragraph = Paragraph::new(lines).block(block);

    frame.render_widget(paragraph, area);
}

/// Truncate a title to fit within the available width
fn truncate_title(title: &str, max_width: usize) -> String {
    if title.len() <= max_width {
        title.to_string()
    } else if max_width > 3 {
        format!("{}...", &title[..max_width - 3])
    } else {
        title.chars().take(max_width).collect()
    }
}

/// Build progress bar spans
fn build_progress_bar(
    stats: &ProjectStats,
    width: usize,
    filled_style: Style,
    empty_style: Style,
    text_style: Style,
) -> Vec<Span<'static>> {
    let percent = stats.completion_percent();
    let percent_text = format!("{percent:>3}%");

    // Calculate bar width: width - percent text (4 chars) - brackets (2 chars) - space (1 char)
    let bar_width = width.saturating_sub(7);

    if bar_width < 2 {
        // Not enough space for a bar, just show percentage
        return vec![Span::styled(percent_text, text_style)];
    }

    // Calculate filled portion
    #[allow(clippy::cast_possible_truncation)]
    let filled = (bar_width * percent as usize / 100).min(bar_width);
    let empty = bar_width - filled;

    // Build the bar: [████░░░░] 75%
    let filled_chars: String = "█".repeat(filled);
    let empty_chars: String = "░".repeat(empty);

    vec![
        Span::raw("["),
        Span::styled(filled_chars, filled_style),
        Span::styled(empty_chars, empty_style),
        Span::raw("] "),
        Span::styled(percent_text, text_style),
    ]
}
