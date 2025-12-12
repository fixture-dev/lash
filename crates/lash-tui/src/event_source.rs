//! Event source abstraction for TUI testing
//!
//! This module provides a trait for abstracting event sources, allowing
//! the TUI to use either real terminal events (via crossterm) or synthetic
//! events from a test harness.

use crossterm::event::{self, Event};
use std::time::Duration;

use crate::error::TuiResult;

/// Trait for providing events to the TUI
///
/// This abstraction allows the TUI to use either real terminal events
/// (via `TerminalEventSource`) or synthetic test events (via `TestEventSource`).
pub trait EventSource {
    /// Poll for the next event with a timeout
    ///
    /// Returns `Ok(Some(event))` if an event is available within the timeout,
    /// `Ok(None)` if no event occurs within the timeout, or an error if
    /// polling fails.
    ///
    /// # Errors
    ///
    /// Returns error if the underlying event source encounters an I/O error.
    fn poll_event(&mut self, timeout: Duration) -> TuiResult<Option<Event>>;
}

/// Terminal event source that uses crossterm to read real terminal events
pub struct TerminalEventSource;

impl TerminalEventSource {
    /// Create a new terminal event source
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for TerminalEventSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSource for TerminalEventSource {
    fn poll_event(&mut self, timeout: Duration) -> TuiResult<Option<Event>> {
        if event::poll(timeout)? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }
}

/// Test event source that delivers events from a pre-defined sequence
///
/// This is used for headless testing of the TUI. Events are delivered
/// in order from the internal queue.
pub struct TestEventSource {
    /// Queue of events to deliver
    events: Vec<Event>,
    /// Current position in the event queue
    index: usize,
}

impl TestEventSource {
    /// Create a new test event source with the given events
    ///
    /// # Examples
    ///
    /// ```
    /// use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    /// use lash_tui::event_source::TestEventSource;
    ///
    /// let events = vec![
    ///     Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
    ///     Event::Key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
    /// ];
    /// let source = TestEventSource::new(events);
    /// ```
    #[must_use]
    pub fn new(events: Vec<Event>) -> Self {
        Self { events, index: 0 }
    }

    /// Check if there are more events in the queue
    #[must_use]
    pub fn has_more_events(&self) -> bool {
        self.index < self.events.len()
    }

    /// Get the number of events remaining in the queue
    #[must_use]
    pub fn remaining_events(&self) -> usize {
        self.events.len().saturating_sub(self.index)
    }
}

impl EventSource for TestEventSource {
    fn poll_event(&mut self, _timeout: Duration) -> TuiResult<Option<Event>> {
        if self.index < self.events.len() {
            let event = self.events[self.index].clone();
            self.index += 1;
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    #[allow(clippy::similar_names)]
    fn test_event_source_delivers_events_in_order() {
        let test_events = vec![
            Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
        ];

        let mut source = TestEventSource::new(test_events);

        // Should deliver events in order
        assert!(source.has_more_events());
        assert_eq!(source.remaining_events(), 3);

        let first = source.poll_event(Duration::from_millis(0)).unwrap();
        assert!(matches!(
            first,
            Some(Event::Key(KeyEvent {
                code: KeyCode::Char('j'),
                ..
            }))
        ));
        assert_eq!(source.remaining_events(), 2);

        let second = source.poll_event(Duration::from_millis(0)).unwrap();
        assert!(matches!(
            second,
            Some(Event::Key(KeyEvent {
                code: KeyCode::Char('k'),
                ..
            }))
        ));
        assert_eq!(source.remaining_events(), 1);

        let third = source.poll_event(Duration::from_millis(0)).unwrap();
        assert!(matches!(
            third,
            Some(Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            }))
        ));
        assert_eq!(source.remaining_events(), 0);

        // After all events are consumed, returns None
        assert!(!source.has_more_events());
        let fourth = source.poll_event(Duration::from_millis(0)).unwrap();
        assert!(fourth.is_none());
    }

    #[test]
    fn test_empty_event_source() {
        let mut source = TestEventSource::new(vec![]);

        assert!(!source.has_more_events());
        assert_eq!(source.remaining_events(), 0);

        let event = source.poll_event(Duration::from_millis(0)).unwrap();
        assert!(event.is_none());
    }
}
