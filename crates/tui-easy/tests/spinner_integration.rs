//! Integration test for `tornado` + `tornado-spinner` (`--features spinner`).
//!
//! Guards the spinner end-to-end:
//!  * `Spinner`, `SpinnerState`, and `SpinnerType` are reachable via the
//!    `tornado::spinner` re-export.
//!  * The default state -> `SpinnerType::Line` and the default frame -> `"│"`.
//!  * `tick(Duration)` advances frame indices at the documented intervals,
//!    accumulates remainder, and wraps when out of bounds — exactly the
//!    `run_app` invariant the upstream runner relies on.
//!  * The StatefulWidget path renders the current frame string at cell
//!    (0, 0) of the target area.
//!
//! Compiled only when the `spinner` feature is enabled (see
//! `required-features` in `Cargo.toml`).

#![cfg(feature = "spinner")]

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use tornado::spinner::{Spinner, SpinnerState, SpinnerType};

#[test]
fn spinner_widget_is_reachable_through_tornado_spinner() {
    // Reachability check at the public surface: build the widget +
    // state exactly as a downstream app would.
    let spinner = Spinner::default();
    let state = SpinnerState::new(SpinnerType::Dot);
    // Defaults to frame 0.
    assert_eq!(state.frame(), 0);
    // Dot shows a braille character at frame 0.
    assert_eq!(state.frame_str(), "\u{28fe}");

    // TypeScript compile-only check: if the re-export paths diverge from
    // the vendored tornado_spinner upstream, this assignment fails to
    // compile and the integration test fails immediately.
    let _ = Spinner::default().style(ratatui::style::Style::default());
}

#[test]
fn default_state_matches_line_preset() {
    let state = SpinnerState::default();
    // `SpinnerState::default()` -> `SpinnerType::Line` per upstream.
    assert_eq!(state.frames(), SpinnerType::Line.frames());
    assert_eq!(state.interval(), SpinnerType::Line.interval());
    assert_eq!(state.frame(), 0);
    assert_eq!(state.frame_str(), "|");
}

#[test]
fn tick_advances_one_frame_per_interval() {
    let mut state = SpinnerState::new(SpinnerType::Line); // 100ms interval
    assert_eq!(state.frame(), 0);

    // 50ms < 100ms interval — no advance.
    state.tick(Duration::from_millis(50));
    assert_eq!(state.frame(), 0);

    // Another 50ms -> 100ms total -> one frame advance.
    state.tick(Duration::from_millis(50));
    assert_eq!(state.frame(), 1);
    assert_eq!(state.frame_str(), "/");
}

#[test]
fn tick_skips_frames_on_large_dt() {
    // 250ms / 100ms interval = 2 advances (frame 0 -> 1 -> 2).
    let mut state = SpinnerState::new(SpinnerType::Line);
    state.tick(Duration::from_millis(250));
    assert_eq!(
        state.frame(),
        2,
        "a single large `tick` should advance multiple frames, simulating a stalled loop"
    );
}

#[test]
fn tick_warps_around_past_last_frame() {
    // 400ms / 100ms / 4 frames = full cycle -> wraps back to 0.
    let mut state = SpinnerState::new(SpinnerType::Line);
    state.tick(Duration::from_millis(400));
    assert_eq!(
        state.frame(),
        0,
        "tick past len(frames) should wrap modulo, not panic"
    );
}

#[test]
fn tick_accumulates_remainder_across_calls() {
    // 150ms first tick -> frame 1, 50ms remain.
    // 60ms second tick -> 50 + 60 = 110ms >= 100ms -> frame 2, 10ms remain.
    let mut state = SpinnerState::new(SpinnerType::Line);
    state.tick(Duration::from_millis(150));
    assert_eq!(state.frame(), 1);
    state.tick(Duration::from_millis(60));
    assert_eq!(
        state.frame(),
        2,
        "remainder accumulator should bridge across calls, matching run_app's variable tick rate"
    );
}

#[test]
fn stateful_widget_renders_current_frame_at_origin() {
    // The spinner writes its current frame string at (area.x, area.y).
    // Line spinner -> first frame is "|".
    let spinner = Spinner::default();
    let mut state = SpinnerState::new(SpinnerType::Line);
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
    ratatui::widgets::StatefulWidget::render(
        &spinner,
        Rect::new(0, 0, 3, 1),
        &mut buf,
        &mut state,
    );

    let first = buf.cell(Position::new(0, 0)).expect("first cell");
    assert_eq!(
        first.symbol(),
        "|",
        "StatefulWidget::render should place the current frame at (0, 0)"
    );
}

#[test]
fn stateful_widget_advances_style_after_tick() {
    // Render after `tick(Dot.interval)` -> first non-default frame.
    // Use Pulse so frame 1 -> "▓" (one of three mid-cycle chars).
    let spinner = Spinner::default();
    let mut state = SpinnerState::new(SpinnerType::Pulse);
    state.tick(SpinnerType::Pulse.interval());
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
    ratatui::widgets::StatefulWidget::render(
        &spinner,
        Rect::new(0, 0, 3, 1),
        &mut buf,
        &mut state,
    );
    let first = buf.cell(Position::new(0, 0)).expect("first cell");
    assert_eq!(state.frame(), 1);
    assert_eq!(first.symbol(), "\u{2593}"); // "▓"
}

#[test]
fn render_full_cycle_for_line_type() {
    // 4 frames cycling: |, /, -, \.
    let spinner = Spinner::default();
    let mut state = SpinnerState::new(SpinnerType::Line);
    let frames = ["|", "/", "-", "\\"];
    for (i, expected) in frames.iter().enumerate() {
        assert_eq!(state.frame(), i);
        assert_eq!(state.frame_str(), *expected);
        state.tick(SpinnerType::Line.interval());
    }
}
