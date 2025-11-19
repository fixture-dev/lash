//! Event stream processing for Markdown parsing
//!
//! This module handles the conversion of raw pulldown-cmark events into
//! semantic events meaningful for Lash task file parsing. It tracks document
//! structure and line numbers as events are processed.
//!
//! # Design
//!
//! We use pulldown-cmark's streaming event-based API rather than building a
//! full AST. This provides:
//! - Lower memory usage (no full tree in memory)
//! - Faster parsing (single pass, no tree construction)
//! - Simpler error recovery (can skip malformed sections)
//!
//! The event processor maintains state about:
//! - Current heading level
//! - Current list depth
//! - Line number tracking
//! - Section boundaries

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

/// Event processor that converts Markdown events to parse actions
///
/// This processes the stream of events from pulldown-cmark and tracks
/// the document structure as it goes.
#[derive(Debug)]
pub struct EventProcessor<'a> {
    /// The pulldown-cmark parser
    parser: Parser<'a>,

    /// Current line number (approximate, since pulldown-cmark doesn't track lines)
    current_line: usize,

    /// Current heading level (0 = not in heading)
    current_heading_level: u32,

    /// Are we currently in a list?
    in_list: bool,

    /// Current list depth
    list_depth: usize,
}

impl<'a> EventProcessor<'a> {
    /// Create a new event processor for the given Markdown content
    #[must_use]
    pub fn new(content: &'a str) -> Self {
        let parser = Parser::new(content);
        Self {
            parser,
            current_line: 1,
            current_heading_level: 0,
            in_list: false,
            list_depth: 0,
        }
    }

    /// Get the current line number estimate
    #[must_use]
    pub fn current_line(&self) -> usize {
        self.current_line
    }

    /// Check if we're currently in a list
    #[must_use]
    pub fn in_list(&self) -> bool {
        self.in_list
    }

    /// Get the current list depth
    #[must_use]
    pub fn list_depth(&self) -> usize {
        self.list_depth
    }

    /// Get the current heading level
    #[must_use]
    pub fn heading_level(&self) -> u32 {
        self.current_heading_level
    }

    /// Process the next event from the stream
    ///
    /// Returns `None` when the stream is exhausted.
    #[allow(dead_code)] // Will be used in Task #6
    pub fn next_event(&mut self) -> Option<Event<'a>> {
        let event = self.parser.next()?;

        // Update state based on event type
        match &event {
            Event::Start(Tag::Heading { level, .. }) => {
                self.current_heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
            }
            Event::End(TagEnd::Heading(_)) => {
                self.current_heading_level = 0;
            }
            Event::Start(Tag::List(_)) => {
                self.in_list = true;
                self.list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                self.list_depth = self.list_depth.saturating_sub(1);
                if self.list_depth == 0 {
                    self.in_list = false;
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                self.current_line += 1;
            }
            _ => {}
        }

        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_processor_creation() {
        let content = "# Test\n\n- [ ] Task";
        let processor = EventProcessor::new(content);

        assert_eq!(processor.current_line(), 1);
        assert_eq!(processor.heading_level(), 0);
        assert!(!processor.in_list());
        assert_eq!(processor.list_depth(), 0);
    }

    #[test]
    fn test_event_processor_heading_tracking() {
        let content = "# Title\n\nContent\n\n## Subtitle";
        let mut processor = EventProcessor::new(content);

        // Process events until we hit the H1
        while let Some(event) = processor.next_event() {
            if let Event::Start(Tag::Heading { level, .. }) = event {
                if matches!(level, HeadingLevel::H1) {
                    assert_eq!(processor.heading_level(), 1);
                    break;
                }
            }
        }
    }

    #[test]
    fn test_event_processor_list_tracking() {
        let content = "- Item 1\n- Item 2\n  - Nested";
        let mut processor = EventProcessor::new(content);

        // Process until we're in a list
        while let Some(event) = processor.next_event() {
            if let Event::Start(Tag::List(_)) = event {
                assert!(processor.in_list());
                assert!(processor.list_depth() > 0);
                break;
            }
        }
    }
}
