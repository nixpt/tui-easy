//! TUI event types and polling.
//!
//! The event loop in [`crate::run_app()`] calls [`poll()`] each iteration
//! with the configured tick rate. A `TuiEvent::Tick` is emitted whenever
//! no keyboard or terminal event arrives within the timeout — this is what
//! drives periodic redraws, spinner advances, and other time-based updates.
//!
//! See also [`crate::TuiApp::handle_event()`] and [`crate::TuiApp::update()`].

use crossterm::event::{self, Event, KeyEvent};
use std::time::Duration;

/// Events produced by the TUI event loop
#[derive(Debug)]
pub enum TuiEvent {
    /// A key was pressed
    Key(KeyEvent),
    /// Terminal was resized
    Resize(u16, u16),
    /// Poll tick (no input within timeout)
    Tick,
}

/// Poll for the next TUI event with a timeout.
///
/// Returns `Tick` if no event occurs within `tick_rate`.
pub fn poll(tick_rate: Duration) -> std::io::Result<TuiEvent> {
    if event::poll(tick_rate)? {
        match event::read()? {
            Event::Key(key) => Ok(TuiEvent::Key(key)),
            Event::Resize(w, h) => Ok(TuiEvent::Resize(w, h)),
            _ => Ok(TuiEvent::Tick),
        }
    } else {
        Ok(TuiEvent::Tick)
    }
}
