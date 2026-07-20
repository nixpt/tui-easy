//! Round-8 example: a multi-tab log consumer composing the round-9
//! `TabLog` helper under a tab-navigation surface.
//!
//! ## Composition
//!
//! | Region   | Source                       | Role                                                       |
//! |----------|------------------------------|------------------------------------------------------------|
//! | Title    | `tornado-spinner`            | Global `SpinnerState` driven by `Tick`-elapsed time        |
//! | TabNav   | `tornado::widget::TabNav`    | 3-row band of 5 stream titles + active-tab chevron         |
//! | Body     | `tornado::tab_log::TabLog`   | Active tab's `render_pair()` driving the scroll viewport   |
//! | Footer   | `tornado::widget`            | `status_bar` with offset + top-anchor on the active tab    |
//!
//! ## Per-decision table (round-8 design memo)
//!
//! | # | Decision | Pick |
//! |---|----------|------|
//! | D1 | Example location | `examples/multi-tab-log/` |
//! | D2 | Tab→ScrollView | One `TabLog` per tab |
//! | D3 | State | `Vec<TabLog>` + `active_tab: usize` + global `SpinnerState` |
//! | D4 | Keymap | `Tab` / `Shift+Tab` / `1..=5` / `q` |
//! | D5 | Layout | Title 1 + TabNav 3 + Body `Min(3)` + Status 2 |
//! | D6 | Tests | 6 inline `TestBackend` smoke tests |
//! | D7 | Tabs styling | `Style::dim()` inactive, `Style::fg(Cyan).bold()` active |
//! | D8 | Module split | `main.rs` (orchestration) + `tab_log.rs` (seeders) |
//! | D9 | Cargo wiring | 6-feature tornado (round 6 catch-up: `TabNav` via umbrella) |
//!
//! ## Hazards carried forward (from `.dejavue` collision captures)
//!
//! This example deliberately exercises the three round-7 / round-9
//! hazards each round-8 test implicitly defends against:
//!
//! 1. **E0499 two-step borrow** — the `route_to_active` match arms
//!    extract `scroll_state_mut().is_at_bottom()` *before* calling
//!    `set_pinned(...)`, the same fix that resolved round-9.
//! 2. **E0502 disjoint-field pair** — `frame.render_stateful_widget`
//!    in the body region routes through `TabLog::render_pair()`
//!    rather than two independent accessor calls.
//! 3. **`word_wrap_line` lifetime** — the `smoke_wrap_helper_reachable`
//!    test only invokes the helper in an ephemeral scope; the live
//!    rebuild pipeline does *not* call wrap, so the lifetime is moot.
//!
//! ## TabNav: from vendored crate to native `ratatui::widgets::Tabs`
//!
//! `TabNav` was originally backed by a vendored `crates/tornado-tabs`
//! mirror of `ratatui::widgets::Tabs`. Since ratatui 0.30 provides
//! `Tabs` natively, the vendored crate was removed and `TabNav` is
//! now a direct re-export of `ratatui::widgets::Tabs`.
//! The constructor and builder chain (`.select(...)`, `.divider(...)`,
//! `.style(...)`, `.highlight_style(...)`) are identical because the
//! vendored widget mirrored upstream 1:1 and the native API is the
//! same.
//!
//! # Run it
//!
//! ```sh
//! cargo run -p multi-tab-log
//! ```
//!
//! # Smoke test
//!
//! ```sh
//! cargo test -p multi-tab-log
//! ```

#![allow(clippy::module_name_repetitions)]

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use tornado::widget::TabNav;

/// Inline-test-only imports — gated so `cargo build` stays clean.
#[cfg(test)]
use ratatui::Terminal;
#[cfg(test)]
use ratatui::backend::TestBackend;
#[cfg(test)]
use ratatui::buffer::Buffer;

use tornado::event::TuiEvent;
use tornado::spinner::{SpinnerState, SpinnerType};
use tornado::tab_log::TabLog;
use tornado::theme::RatatuiThemeColors;
use tornado::widget::{Sparkline, SparklineBar, status_bar};
use tornado::{run_app, TuiApp};

use crate::tab_log::{append_row, seed_streams, TAB_TITLES};

mod tab_log;

// ── Constants ─────────────────────────────────────────────────────────────

/// Run-loop tick rate. `SpinnerState::tick(Duration)` accepts the elapsed
/// time of one iteration and advances frames at the cadence defined by
/// `SpinnerType::interval`.
const TICK_RATE: Duration = Duration::from_millis(80);

/// How many ticks between per-tab "task done" appends. Round-8 keeps
/// the population of *every* tab moving: even non-active streams
/// receive occasional rows so the per-tab body content stays distinct
/// and the demo never greys out a tab on long runs.
/// How many ticks between per-tab "task done" appends. Round-8 keeps
/// the population of *every* tab moving: even non-active streams
/// receive occasional rows so the per-tab body content stays distinct
/// and the demo never greys out a tab on long runs.
const TASK_COMPLETE_EVERY: u32 = 4;

/// How many recent frames the Sparkline widget displays. Round 10
/// picked 12 because that's the natural size of the footer's
/// right flank without crowding the text half of the status bar.
/// Round 10 design memo D9: the metric ingestion advances under
/// `tick_count % TASK_COMPLETE_EVERY == 0` (the MSRV-portable form;
/// `<integer>::is_multiple_of` is stabilized in Rust 1.87.0, but the
/// workspace pins `rust-version = "1.85.0"`).
const SPARKLINE_RING_LEN: usize = 12;

// ── The app ───────────────────────────────────────────────────────────────

/// Thin shell around `Vec<TabLog>` + global `SpinnerState`. Round 8
/// lifts the per-tab scroll mechanics from round-7 into round-9's
/// `TabLog` helper, leaving this struct responsible only for tab
/// dispatch + keymap routing + global ticker state.
struct MultiTabApp {
    /// One `TabLog` per stream. Indexed parallel to `TAB_TITLES`.
    tabs: Vec<TabLog>,
    /// Index into `tabs` for the currently-displayed stream.
    active_tab: usize,
    /// Global spinner — rendered in the title bar regardless of which
    /// tab the user is reading.
    spinner_state: SpinnerState,
    /// Theme for the footer's `status_bar` widget.
    theme: RatatuiThemeColors,
    /// Tick bookkeeping — drives the global spinner advance + per-tab
    /// row-pump cadence.
    last_tick: Instant,
    tick_count: u32,
    /// Round-10: rolling-c sparkline metrics over the last
    /// `SPARKLINE_RING_LEN` frames. The widget draws this slice
    /// directly (`Sparkline::new(&ring_buffer)`); we just need to
    /// advance the buffer on each `TASK_COMPLETE_EVERY` tick.
    sparkline_ring: Vec<u64>,
    /// Set by quit keys in `handle_event`. `run_app` polls
    /// `should_quit` and tears down the loop when this flips.
    quit: bool,
}

impl MultiTabApp {
    fn new() -> Self {
        Self {
            tabs: seed_streams(),
            active_tab: 0,
            spinner_state: SpinnerState::new(SpinnerType::Dot),
            theme: tornado::theme::ThemeColors::default().to_ratatui(),
            last_tick: Instant::now(),
            tick_count: 0,
            sparkline_ring: vec![1; SPARKLINE_RING_LEN],
            quit: false,
        }
    }

    /// Round-10 (D9): ingest a fresh metric sample into the
    /// sparkline ring. Called from `update()` on each
    /// `TASK_COMPLETE_EVERY` tick and from the dedicated
    /// `smoke_sparkline_metrics_advance_on_tick` test.
    fn push_sparkline_metric(&mut self, value: u64) {
        // Carry the vector forward; rotate the oldest entry.
        self.sparkline_ring.rotate_left(1);
        let last = self.sparkline_ring.len() - 1;
        self.sparkline_ring[last] = value;
    }

    /// Mutable handle to the currently-displayed tab. Caller holds
    /// `&mut self` exclusively, so this is a fresh `&mut TabLog`
    /// each call — no overlap with prior calls.
    fn active(&mut self) -> &mut TabLog {
        &mut self.tabs[self.active_tab]
    }
}

// ─── TuiApp impl ──────────────────────────────────────────────────────────

impl TuiApp for MultiTabApp {
    fn name(&self) -> &str {
        "multi-tab-log"
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn update(&mut self) {
        let dt = self.last_tick.elapsed();
        self.spinner_state.tick(dt);
        self.last_tick = Instant::now();

        self.tick_count = self.tick_count.wrapping_add(1);

        // Drive per-tab population on the active tab — keep the
        // demo dynamic with minimal cross-tab churn. Pinned tabs
        // receive the row at the bottom of their buffer (thanks to
        // `reset_to_bottom_if_pinned` below).
        //
        // `tick_count % TASK_COMPLETE_EVERY == 0` rather than
        // `<integer>::is_multiple_of` keeps us portable under the
        // workspace `rust-version = "1.85.0"` MSRV (is_multiple_of
        // stabilizes in 1.87.0).
        if self.tick_count % TASK_COMPLETE_EVERY == 0 {
            let title = TAB_TITLES[self.active_tab];
            append_row(
                &mut self.tabs,
                self.active_tab,
                format!("task in {title} done (tick {})", self.tick_count),
                Some(format!(
                    "https://github.com/nixpt/arniko/blob/main/{title}/tick/{}",
                    self.tick_count
                )),
            );
        }

        // Honour pinned-to-bottom across *all* tabs: every tab has
        // its own scroll offset, and only the active one is shown,
        // but a re-pin-on-rebuild is cheap and preserves the user's
        // per-tab intent (round-9 `reset_to_bottom_if_pinned`).
        for tab in self.tabs.iter_mut() {
            tab.reset_to_bottom_if_pinned();
        }

        // Round-10 (D9): metric ingestion for the rolling-c
        // Sparkline. We sample `tick_count` directly (no
        // `Instant::now()` wall-clock) so the metric shape is
        // deterministic across CI runs (carries forward the
        // round-8 `Instant::now()` test-loop trap defense).
        self.push_sparkline_metric(self.tick_count as u64);
    }

    fn handle_event(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::Tick | TuiEvent::Resize(_, _) => {}

            // Quit keys: `q`, `Esc`, `Ctrl-c`.
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            })
            | TuiEvent::Key(KeyEvent {
                code: KeyCode::Esc, ..
            })
            | TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => {
                self.quit = true;
            }

            // Tab navigation: `Tab` advances (wraps 4 → 0); `BackTab`
            // retracts (wraps 0 → length-1 = 4). Round-9 collision #4
            // recommended translating the unmodified `Tab` key (and
            // its `BackTab` companion) rather than depending on
            // `KeyModifiers::SHIFT + Tab` which is inconsistent
            // across terminals + test-time key synthesis.
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Tab, ..
            }) => {
                self.active_tab = (self.active_tab + 1) % self.tabs.len();
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::BackTab,
                ..
            }) => {
                // Wrap backwards: from tab 0 wraps to length-1 (i.e.,
                // the last tab). Standard Tab-nav convention.
                self.active_tab = self
                    .active_tab
                    .checked_sub(1)
                    .unwrap_or(self.tabs.len() - 1);
            }

            // Direct jump: `1` … `5` route to the matching title
            // (1-indexed for the user, 0-indexed internally). Out-of-
            // range digits (`0`, `6+`) fall through to scroll routing
            // so the user can still scroll while pressing them.
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
                ..
            }) if c.is_ascii_digit() => {
                let requested = (c as u32 - '0' as u32) as usize;
                if requested >= 1 && requested <= self.tabs.len() {
                    self.active_tab = requested - 1;
                } else {
                    self.route_to_active(event);
                }
            }

            // Pass-through: scroll keys (`j` / `k` / `↑` / `↓` / `PgUp`
            // / `PgDn` / `g` / `G` / `Home` / `End`) are routed to the
            // *active* tab's `ScrollViewState`. The first digit-letter
            // match arm above catches `1`..=`5` before getting here.
            _ => self.route_to_active(event),
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        // Title (1 row) → TabNav (3 rows) → Body (`Min(3)`) → Status (2).
        // The 3-row TabNav region matches D5 of the design memo.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(2),
            ])
            .split(area);

        let title_area = chunks[0];
        let tabnav_area = chunks[1];
        let body_area = chunks[2];
        let status_area = chunks[3];

        // ── Title: spinner + tab indicator + tick counter ────────────
        let frame_glyph = self.spinner_state.frame_str();
        let title_text = format!(
            " {frame_glyph} multi-tab-log │ tab {}/{} │ tick {}",
            self.active_tab + 1,
            self.tabs.len(),
            self.tick_count,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                title_text,
                ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
            ))),
            title_area,
        );

        // ── TabNav: tornado::widget::TabNav (round-6 catch-up —
        //    `crates/tornado-tabs` mirrors `ratatui::widgets::Tabs`
        //    verbatim; the umbrella surfaces it under the semantic
        //    alias `TabNav` so consumers don't have to remember
        //    whether the source widget is named Tabs or TabNav).
        //    The builder chain mirrors upstream 1:1.
        let tabs_widget = TabNav::new(
            TAB_TITLES
                .iter()
                .copied()
                .map(Line::from)
                .collect::<Vec<Line<'static>>>(),
        )
        .select(self.active_tab)
        .divider("│")
        .style(
            ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
        )
        .highlight_style(
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );
        frame.render_widget(tabs_widget, tabnav_area);

        // ── Body: active tab's render_pair (round-9 helper) ─────────
        //
        // `render_pair()` returns disjoint-borrowed refs into the
        // same `&mut TabLog` so we can hand both to
        // `render_stateful_widget` without triggering an E0502.
        // Carry-forward of round-9 collision #3.
        let pair = self.active().render_pair();
        frame.render_stateful_widget(pair.view, body_area, pair.state);

        // ── Status (2 rows): text half + sparkline half ────────────
        //
        // The footer keeps the same 2-row shape (block TOP border +
        // content row). We split the content row horizontally into
        // a text half (with the existing status_bar Paragraph) and
        // a 12-col Sparkline half (round-10 D5 + D9).
        let footer_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min((status_area.width as usize).saturating_sub(SPARKLINE_RING_LEN).max(1) as u16),
                Constraint::Length(SPARKLINE_RING_LEN as u16),
            ])
            .split(status_area);

        let anchor_part = {
            // Read-only top-anchor lookup first, before any
            // scroll-state mutables are taken — keeps the borrow
            // disjoint from the `scroll_state_mut()` block below.
            self.active()
                .top_anchor()
                .map(|t| format!("anchor #{:04}", t.id))
                .unwrap_or_else(|| "anchor (none)".to_string())
        };
        let scroll_y = {
            let s = self.active().scroll_state_mut();
            s.offset().y
        };
        let pinned_count = self.tabs.iter().filter(|t| t.is_pinned()).count();
        let left = format!(
            " y={} │ {} │ tab {}/{} │ q │ {} rows │ pins {}/{}",
            scroll_y,
            anchor_part,
            self.active_tab + 1,
            self.tabs.len(),
            self.active().row_count(),
            pinned_count,
            self.tabs.len()
        );

        frame.render_widget(
            // The right half of the footer carries the round-10
            // Sparkline widget (rendered below); the inline `right`
            // text path of `status_bar` is unused. Pass
            // `Some(String::new())` so the Option<impl Into<Line<'_>>>
            // parameter resolves — bare `None` triggers E0283
            // because Rust cannot infer the trait-bound T.
            status_bar(left, Some(String::new()), &self.theme),
            footer_split[0],
        );

        // Round-10: 12-col Sparkline on the right flank of the
        // footer's 2-row content. The rolling buffer shows the
        // tick_count history — a deterministic metric that
        // always advances, no `Instant::now()` involvement, so
        // the render is stable across CI runs.
        frame.render_widget(
            Sparkline::new(&self.sparkline_ring)
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
                .bar_set(
                    SparklineBar::new("▁▂▃▄▅▆▇█")
                        .style(
                            ratatui::style::Style::default()
                                .fg(ratatui::style::Color::Cyan)
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ),
                ),
            footer_split[1],
        );
    }
}

// ── Scroll-routing helper ─────────────────────────────────────────────────

impl MultiTabApp {
    /// Route scroll keys to the *active* tab. Each match arm uses
    /// the **two-step borrow pattern** (extract `is_at_bottom()` to a
    /// local before calling `set_pinned(...)`) to defend against the
    /// E0499 pattern from round-9.
    fn route_to_active(&mut self, event: TuiEvent) {
        let TuiEvent::Key(KeyEvent { code, .. }) = event else {
            return;
        };
        match code {
            KeyCode::Down | KeyCode::Char('j') => {
                // E0499 two-step.
                let at_bottom = self.active().scroll_state_mut().is_at_bottom();
                self.active().scroll_state_mut().scroll_down();
                self.active().set_pinned(at_bottom);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.active().scroll_state_mut().scroll_up();
                self.active().set_pinned(false);
            }
            KeyCode::PageDown => {
                let at_bottom = self.active().scroll_state_mut().is_at_bottom();
                self.active().scroll_state_mut().scroll_page_down();
                self.active().set_pinned(at_bottom);
            }
            KeyCode::PageUp => {
                self.active().scroll_state_mut().scroll_page_up();
                self.active().set_pinned(false);
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.active().scroll_state_mut().scroll_to_bottom();
                self.active().set_pinned(true);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.active().scroll_state_mut().scroll_to_top();
                self.active().set_pinned(false);
            }
            _ => {}
        }
    }
}

// ── Boot ──────────────────────────────────────────────────────────────────

fn main() -> std::io::Result<()> {
    let app = MultiTabApp::new();
    run_app(app, TICK_RATE)
}

// ── Smoke tests (TestBackend, no real terminal) ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_app() -> MultiTabApp {
        MultiTabApp::new()
    }

    /// Render the app once into a `TestBackend` of the given size
    /// and return the buffer for cell-by-cell assertions.
    fn render_once(app: &mut MultiTabApp, w: u16, h: u16) -> Buffer {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| app.draw(frame))
            .expect("draw into TestBackend");
        terminal.backend().buffer().clone()
    }

    /// Concatenate the cell symbols of row `y` across the full width.
    fn row_text(buf: &Buffer, y: u16) -> String {
        let mut s = String::new();
        for x in 0..buf.area.width {
            s.push_str(buf[(x, y)].symbol());
        }
        s
    }

    /// Concatenate the cell symbols of rows `[y0, y1)` across the
    /// full width.
    fn rows_text(buf: &Buffer, y0: u16, y1: u16) -> String {
        let mut s = String::new();
        for y in y0..y1 {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
        }
        s
    }

    #[test]
    fn smoke_renders_four_regions() {
        let mut app = fresh_app();
        let buf = render_once(&mut app, 120, 24);

        // 1. Title row (y=0) carries the brand.
        let title_line = row_text(&buf, 0);
        assert!(
            title_line.contains("multi-tab-log"),
            "title row missing app name: {title_line:?}"
        );
        assert!(
            title_line.contains("tab 1/5"),
            "title row missing tab indicator: {title_line:?}"
        );

        // 2. TabNav (y=1..=3) carries at least two of the five titles.
        let tabnav = rows_text(&buf, 1, 4);
        assert!(
            tabnav.contains("build"),
            "TabNav missing 'build' title: {tabnav:?}"
        );
        assert!(
            tabnav.contains("release"),
            "TabNav missing 'release' title: {tabnav:?}"
        );

        // 3. Body (y=4..height-2) contains seeded content from the active tab.
        let body = rows_text(&buf, 4, buf.area.height.saturating_sub(2));
        assert!(
            body.contains("build step 0"),
            "body missing first seed from active tab: {body:?}"
        );

        // 4. Footer (y=height-1) carries anchor + tab counter.
        let footer = row_text(&buf, buf.area.height - 1);
        assert!(
            footer.contains("anchor"),
            "footer missing anchor part: {footer:?}"
        );
        assert!(
            footer.contains("1/5"),
            "footer missing tab counter: {footer:?}"
        );
    }

    #[test]
    fn smoke_tab_switch_changes_body_content() {
        let mut app = fresh_app();

        // Pin every tab by default — unpin tab 2 ("deps") and scroll
        // to top so the viewport paints line 0 of its buffer.
        app.active_tab = 2;
        app.tabs[2].set_pinned(false);
        app.tabs[2].scroll_state_mut().scroll_to_top();

        let buf = render_once(&mut app, 120, 24);

        let body = rows_text(&buf, 4, buf.area.height.saturating_sub(2));
        assert!(
            body.contains("deps step 0"),
            "after switching to tab 2, body must show 'deps step 0': {body:?}"
        );
        assert!(
            !body.contains("build step 0"),
            "after switching to tab 2, body must no longer show 'build step 0': {body:?}"
        );
    }

    #[test]
    fn smoke_per_tab_scroll_offset_preserved() {
        let mut app = fresh_app();

        // Scroll tab 0 to top (offset (0, 0)); record it.
        app.tabs[0].set_pinned(false);
        app.tabs[0].scroll_state_mut().scroll_to_top();
        let initial_offset = app.tabs[0].scroll_state_mut().offset();

        // Round-trip tab navigation: 0 → 1 → 2 → 3 → 0.
        app.active_tab = 1;
        app.active_tab = 2;
        app.active_tab = 3;
        app.active_tab = 0;

        // Tab 0's offset must be unchanged — round-9 design memo's
        // "per-tab offset preservation" guarantee.
        let final_offset = app.tabs[0].scroll_state_mut().offset();
        assert_eq!(
            initial_offset, final_offset,
            "per-tab scroll offset must be preserved across tab switches"
        );
    }

    #[test]
    fn smoke_direct_jump_keys_route_correctly() {
        let mut app = fresh_app();

        // `3` jumps to active_tab=2 ("deps"); `5` jumps to active_tab=4 ("release").
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('3'),
            KeyModifiers::NONE,
        )));
        assert_eq!(app.active_tab, 2, "'3' must jump to active_tab=2");

        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('5'),
            KeyModifiers::NONE,
        )));
        assert_eq!(app.active_tab, 4, "'5' must jump to active_tab=4");

        // `0` is out of range — must not change active_tab.
        let before = app.active_tab;
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('0'),
            KeyModifiers::NONE,
        )));
        assert_eq!(
            app.active_tab, before,
            "'0' (out of range) must not change active_tab"
        );

        // `Tab` advances and wraps. From tab 4 → (4+1) % 5 = 0.
        app.handle_event(TuiEvent::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
        assert_eq!(app.active_tab, 0, "Tab from tab 4 wraps to active_tab=0");

        // `BackTab` retrenches and wraps. From tab 0: 0.checked_sub(1)
        // is None → falls back to length-1 = 4 (the last tab).
        app.handle_event(TuiEvent::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)));
        assert_eq!(
            app.active_tab, 4,
            "BackTab from tab 0 wraps to length-1 (= 4)"
        );

        // `q` quits.
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )));
        assert!(app.should_quit(), "q must set should_quit");
    }

    #[test]
    fn smoke_spinner_advances_in_title() {
        let mut app = fresh_app();

        let initial_frame = app.spinner_state.frame();

        // Drive the spinner's internal clock with explicit 200 ms
        // intervals (5 iterations = 1 s of accumulated `tick`). We
        // tick the spinner *directly* rather than via `update()`
        // because in tight CI loops `self.last_tick.elapsed()` is
        // effectively 0 and the spinner's 80 ms interval can't
        // accumulate enough to advance a frame — which would falsely
        // report a frame-stuck bug. Ticking the spinner directly
        // isolates the spinner advance from the elapsed-time source.
        for _ in 0..5 {
            app.spinner_state
                .tick(std::time::Duration::from_millis(200));
        }

        assert_ne!(
            app.spinner_state.frame(),
            initial_frame,
            "spinner frame should advance after 5 explicit 200ms ticks"
        );

        // And the title region must reflect the new frame.
        let buf = render_once(&mut app, 120, 24);
        let title = row_text(&buf, 0);
        let expected_glyph = app.spinner_state.frame_str();
        assert!(
            title.contains(&expected_glyph.trim()),
            "title row missing current spinner glyph {expected_glyph:?}: {title:?}"
        );
    }

    #[test]
    fn smoke_sparkline_metrics_advance_on_tick() {
        // Round 10 (D9): the rolling-c Sparkline metric advances
        // deterministically across CI runs. We tick the metric
        // directly via `push_sparkline_metric` rather than via
        // wall-clock `Instant::now()`, carrying forward the
        // round-8 `Instant::now()` test-loop trap defense
        // (`is_multiple_of` MSRV pitfall + Instant::now() near-zero
        // dt in tight loops).
        //
        // Contract: after 5 explicit `push_sparkline_metric` calls
        // with monotonically increasing values, the ring's *last*
        // entry must equal the most-recent push and the *first*
        // entry must equal the second-most-recent push (the ring
        // rotates left by one slot per push).
        let mut app = fresh_app();
        let initial_first = app.sparkline_ring[0];
        let initial_last = *app.sparkline_ring.last().unwrap();

        for tick in 100u64..105u64 {
            app.push_sparkline_metric(tick);
        }

        let final_first = app.sparkline_ring[0];
        let final_last = *app.sparkline_ring.last().unwrap();

        // `push_sparkline_metric` does `rotate_left(1) + ring[len-1] = v`,
        // so each push shifts previously `index 0` content out the back
        // and drops the new value into the back. After 5 pushes into a
        // 12-cell ring, the most-recent pushes occupy positions
        // `[len-5..len)` (= [7..12)) and the historically oldest
        // positions [0..7) retain the seeded `1` values untouched.
        assert_eq!(
            final_last, 104,
            "ring's last entry must equal the most recent push"
        );
        assert_eq!(
            app.sparkline_ring[SPARKLINE_RING_LEN - 2],
            103,
            "ring's penultimate entry must equal the second-most-recent push (104 - 1 = 103)"
        );
        assert_eq!(
            app.sparkline_ring[SPARKLINE_RING_LEN - 5],
            100,
            "ring's position [len-5] must equal the first of the 5 push values"
        );
        assert_eq!(
            final_first, initial_first,
            "ring's index 0 is below the rolling-5 shadow ring (length 12 > 5 pushes), so its initial 1 must survive"
        );
        assert_ne!(
            (initial_first, initial_last),
            (final_first, final_last),
            "after 5 pushes the ring's last entry must change (initial_last=1, final_last=104)"
        );
    }

    #[test]
    fn smoke_route_to_active_compiles_under_e0499_constraints() {
        // The `route_to_active` match arms implement a two-step borrow
        // pattern (extract `is_at_bottom()` to a local before
        // `set_pinned(...)`). This test's primary contract is that
        // the function runs to completion under the E0499-safety
        // discipline; the secondary contract is that pinned-state
        // flips are coherent.
        let mut app = fresh_app();

        // All tabs pinned on launch.
        assert!(
            app.tabs.iter().all(|t| t.is_pinned()),
            "all tabs are pinned on launch"
        );

        // `j` on a 6-row scrollview with pinned=true: scroll_down
        // clamps at the bottom of the populated body (offset stays
        // at the bottom of the buffer's content region), so
        // `is_at_bottom()` is still true and `set_pinned(true)` keeps
        // the tab pinned. The behaviour validates the two-step
        // pattern (no E0499, no panic) without asserting an
        // *unrealistic* unpin from a no-op scroll.
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('j'),
            KeyModifiers::NONE,
        )));
        assert!(
            app.active().is_pinned(),
            "after 'j' on a pinned view with body clamped at bottom, tab must stay pinned"
        );

        // `PageUp` un-pins (scroll up leaves the bottom).
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::PageUp,
            KeyModifiers::NONE,
        )));
        assert!(
            !app.active().is_pinned(),
            "PageUp should unpin the active tab"
        );

        // `End` re-pins (jumps to bottom).
        app.handle_event(TuiEvent::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)));
        assert!(
            app.active().is_pinned(),
            "End should re-pin the active tab"
        );

        // `q` quits.
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )));
        assert!(app.should_quit(), "q must set should_quit");

        // A second app instance exercises the k + End + Tab scenario
        // to confirm the same borrow discipline applies across the
        // other E0499-prone arms (k + PageDown + Home).
        let mut app = fresh_app();
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::NONE,
        )));
        assert!(
            !app.active().is_pinned(),
            "k should unpin from default-pinned"
        );
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::End,
            KeyModifiers::NONE,
        )));
        assert!(app.active().is_pinned(), "End re-pins");
    }
}
