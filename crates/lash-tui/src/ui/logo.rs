//! Logo rendering for the TUI
//!
//! Displays the Lash ASCII logo in the upper-left corner of the TUI.

use lash_core::logo::LOGO_LINES;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::colors::Theme;
use crate::ui::themes;

/// The height of the logo widget (lines + border)
pub const LOGO_HEIGHT: u16 = 5; // 3 logo lines + 2 border lines

/// Render the logo widget
pub fn render(frame: &mut Frame, area: Rect, theme: &Theme, is_focused: bool) {
    let border_style = if is_focused {
        themes::focused_border_style(theme)
    } else {
        themes::unfocused_border_style(theme)
    };

    // Create styled logo lines using the emphasis color
    let logo_style = Style::default().fg(theme.emphasis_color());

    let lines: Vec<Line<'static>> = LOGO_LINES
        .iter()
        .map(|line| Line::from(vec![Span::styled((*line).to_string(), logo_style)]))
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    let paragraph = Paragraph::new(lines).block(block);

    frame.render_widget(paragraph, area);
}
