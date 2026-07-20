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

#[cfg(feature = "hyperlink")]
pub use tornado_hyperlink as hyperlink;

#[cfg(feature = "spinner")]
pub use tornado_spinner as spinner;

#[cfg(feature = "scroller")]
pub use tornado_scrollview as scroller;
#[cfg(feature = "tabs")]
pub use tornado_tabs as tabs;
// Round 10: vendored Sparkline + SparklineBar widget from ratatui
// 0.30 itself (dual MIT/Apache-2.0). The widget is fully **stateless**
// — no `StatefulWidget` counterpart exists upstream either — so the
// vendored mirror exposes only the `Widget` impl. See
// `crates/tornado-sparkline/src/lib.rs` for the verdict.
#[cfg(feature = "sparkline")]
pub use tornado_sparkline as sparkline;

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
