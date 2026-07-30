//! Shared terminal UI shell — terminal init, crossterm event loop, themes, and a
//! curated palette of feature-gated ratatui widgets.
//!
//! See the [crate-level README](https://github.com/nixpt/arniko/blob/main/crates/tornado/README.md)
//! for full documentation, feature flags, usage patterns, and examples.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use std::time::Duration;
//! use tornado::event::TuiEvent;
//! use tornado::{run_app, TuiApp};
//!
//! struct MyApp { quit: bool }
//!
//! impl TuiApp for MyApp {
//!     fn draw(&mut self, frame: &mut ratatui::Frame) {}
//!     fn handle_event(&mut self, event: TuiEvent) {
//!         if let TuiEvent::Key(k) = event {
//!             if k.code == crossterm::event::KeyCode::Char('q') {
//!                 self.quit = true;
//!             }
//!         }
//!     }
//!     fn should_quit(&self) -> bool { self.quit }
//! }
//! # fn main() -> std::io::Result<()> {
//! run_app(MyApp { quit: false }, Duration::from_millis(80))
//! # }
//! ```

#![doc = include_str!("../README.md")]

pub mod event;
pub mod terminal;
pub mod theme;
pub mod widget;

// Round 9 — the `TabLog` primitive (per-tab scroll log + anchor
// registry + offset preservation). Lives behind the `log_view`
// umbrella feature, which pulls in `tornado-styles` (for
// `HyperlinkTarget`), `tornado-scrollview` (for `ScrollView` and
// `ScrollViewState`), and `tornado-wrap` (for `tornado::wrap::word_wrap_line`
// consumers — exercised in tests, not in the helper body itself).
#[cfg(feature = "log_view")]
pub mod tab_log;

#[cfg(feature = "styles")]
pub use tornado_styles as styles;

#[cfg(feature = "wrap")]
pub use tornado_wrap as wrap;

// Migrated from vendored `tornado-hyperlink` to upstream
// `hyperrat` crate (v0.1, MIT/Unlicense). API is identical:
// `Link` widget with OSC 8 hyperlink support.
#[cfg(feature = "hyperlink")]
pub use hyperrat as hyperlink;

// Migrated from vendored `tornado-spinner` to upstream
// `ratatui-cheese` crate (v0.7, MIT). API is identical:
// `Spinner`, `SpinnerState`, `SpinnerType`.
#[cfg(feature = "spinner")]
pub use ratatui_cheese::spinner as spinner;

// Migrated from vendored `tornado-scrollview` to upstream
// `tui-scrollview` crate (v0.6, MIT/Apache-2.0). API is
// identical: `ScrollView`, `ScrollViewState`, `ScrollbarVisibility`.
#[cfg(feature = "scroller")]
pub use tui_scrollview as scroller;
// Round 6 catch-up migration: `tabs` feature now re-exports
// `ratatui::widgets::Tabs` directly as `tornado::widget::TabNav`
// (was vendored via `crates/tornado-tabs`, now removed).
// The re-export lives in `widget.rs` under the `TabNav` alias —
// no module-level `pub use` is needed here.
// Migrated: `sparkline` feature now uses `ratatui::widgets::Sparkline`
// directly (was vendored via `crates/tornado-sparkline`, now removed).
// Consumers access Sparkline via `ratatui::widgets::Sparkline` or
// the re-export at `tornado::widget::Sparkline`.
// No module-level `pub use` is needed — the re-export is via widget.rs.
// Migrated: `list` feature now uses `ratatui::widgets::{List, ListState}`
// directly (was vendored via `crates/tornado-list`, now removed).
// The re-export is via widget.rs. No module-level `pub use` is needed.

// Original BarChart/Bar/BarGroup widget (round-12) — no upstream
// equivalent to migrate to, unlike tabs/sparkline/list. Consumers reach
// it as `tornado::widget::{BarChart, Bar, BarGroup, BarSet}` or via
// this module alias as `tornado::barchart::*`.
#[cfg(feature = "barchart")]
pub use tornado_barchart as barchart;

// Vendored TextArea + TextAreaState from xai-ratatui-textarea
// (Apache-2.0, round-13) — ratatui ships no built-in text-area widget,
// so unlike tabs/sparkline/list there's no upstream re-export path
// here either. Consumers reach it as
// `tornado::widget::{TextArea, TextAreaState}` or via this module
// alias as `tornado::textinput::*`.
#[cfg(feature = "textinput")]
pub use tornado_textinput as textinput;
// Migrated from vendored `tornado-popup` to upstream
// `tui-popup` crate (v0.7, MIT). API is identical:
// `Popup`, `PopupState` with mouse-drag support.
#[cfg(feature = "popup")]
pub use tui_popup as popup;
// Migrated from vendored `tornado-big-text` to upstream
// `tui-big-text` crate (v0.8, MIT/Apache-2.0). API is
// identical: `BigText`, `PixelSize`.
#[cfg(feature = "big-text")]
pub use tui_big_text as big_text;
// Migrated from vendored `tornado-tree-widget` to upstream
// `tui-tree-widget` crate (v0.24, MIT). API is identical:
// `Tree`, `TreeItem`, `TreeState`, `Flattened`.
#[cfg(feature = "tree")]
pub use tui_tree_widget as tree;

use ratatui::Frame;
use std::time::Duration;

/// Trait that TUI applications implement to use the shared shell
pub trait TuiApp {
    /// Render the UI into the frame
    fn draw(&mut self, frame: &mut Frame);

    /// Handle an input event
    fn handle_event(&mut self, event: event::TuiEvent);

    /// Called each tick before draw — for polling async data, timers, etc.
    fn update(&mut self) {}

    /// Return true when the app should exit
    fn should_quit(&self) -> bool;

    /// Application name
    fn name(&self) -> &str {
        "tornado"
    }
}

/// Run a TUI application with the standard terminal setup.
///
/// Initializes the terminal, enters the draw-event loop,
/// and restores the terminal on exit or panic.
pub fn run_app(app: impl TuiApp, tick_rate: Duration) -> std::io::Result<()> {
    run_app_standalone(app, tick_rate)
}

fn run_app_standalone(mut app: impl TuiApp, tick_rate: Duration) -> std::io::Result<()> {
    let mut term = terminal::init()?;
    let result = run_loop(&mut term, &mut app, tick_rate);
    terminal::restore();
    result
}

fn run_loop(
    term: &mut terminal::ExoTerminal,
    app: &mut impl TuiApp,
    tick_rate: Duration,
) -> std::io::Result<()> {
    while !app.should_quit() {
        app.update();
        term.draw(|frame| app.draw(frame))?;
        let evt = event::poll(tick_rate)?;
        app.handle_event(evt);
    }
    Ok(())
}
