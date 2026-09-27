//! End-to-end consumer of every borrow-debris sibling shipped in tui_easy.
//!
//! Round 7 wired five vendored siblings into a single [`TuiApp`]. Round 9
//! refactored the per-tab scroll log mechanics (`rows` registry, OSC-8
//! anchor assignment, `ScrollView` + `ScrollViewState`, rebuild loop,
//! `top_anchor` lookup, `pinned_to_bottom` tracking) into a shared
//! primitive: `tui_easy::tab_log::TabLog` (behind the `log_view`
//! umbrella feature). This file is now a thin orchestrator on top of
//! that helper.
//!
//! ## Composition (after round 9)
//!
//! | Region   | Source crate        | Surface in app                                            |
//! |----------|---------------------|-----------------------------------------------------------|
//! | Title    | `tui_easy-spinner`   | animated `SpinnerState` driven by `Tick`-elapsed time     |
//! | Body     | `tui_easy::tab_log`  | `TabLog::scroll_view()` + `TabLog::scroll_state_mut()`    |
//! | Rows     | `tui_easy::tab_log`  | per-row OSC-8 anchor registry + top-of-viewport lookup    |
//! | Footer   | `tui_easy::widget`   | block-bordered `status_bar` reading offset + top anchor   |
//! | Wrap     | `tui-easy-wrap`      | `word_wrap_line` exercised in the inline smoke test       |
//!
//! # Run it
//!
//! ```sh
//! cargo run -p scroll-log
//! ```
//!
//! # Smoke test
//!
//! ```sh
//! cargo test -p scroll-log
//! ```

#![allow(clippy::module_name_repetitions)]

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// These imports are only used by the inline `#[cfg(test)]` smoke
/// tests below; gating them this way keeps `cargo build` clean.
#[cfg(test)]
use ratatui::Terminal;
#[cfg(test)]
use ratatui::backend::TestBackend;
#[cfg(test)]
use ratatui::buffer::Buffer;
#[cfg(test)]
use ratatui::layout::Position;

use tui_easy::event::TuiEvent;
use tui_easy::spinner::{SpinnerState, SpinnerType};
use tui_easy::tab_log::TabLog;
use tui_easy::theme::RatatuiThemeColors;
use tui_easy::widget::status_bar;
use tui_easy::{run_app, TuiApp};

// ── Constants ─────────────────────────────────────────────────────────────

/// Vertical resolution of the virtual log buffer. Picked to comfortably
/// hold a long session of work without scrollback exhaustion.
const SCROLL_BUF_LINES: u16 = 240;

/// Column width of the virtual log buffer.
const SCROLL_WIDTH: u16 = 96;

/// Run-loop tick rate. `SpinnerState::tick(Duration)` accepts the
/// elapsed time of one iteration and advances frames at the cadence
/// defined by `SpinnerType::interval`.
const TICK_RATE: Duration = Duration::from_millis(80);

/// How many ticks between simulated async-work completions.
const TASK_COMPLETE_EVERY: u32 = 5;

// ── Domain types ──────────────────────────────────────────────────────────

/// Log severity level — mapped to a static `category` string the
/// `TabLog` helper consumes. Only the variants the example actually
/// emits are listed; `WARN`/`ERROR` are deliberately omitted (the
/// round-9 simplify pass dropped them after a code-review found them
/// unused).
#[derive(Debug, Clone, Copy)]
enum Level {
    Info,
    Done,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Self::Info => "INFO ",
            Self::Done => "DONE ",
        }
    }
}

// ── The app ───────────────────────────────────────────────────────────────

/// Thin shell around `TabLog` + `SpinnerState`. Round 9 removed every
/// field that round-7 carried for the per-tab scroll mechanics — the
/// `TabLog` helper now owns those.
struct ScrollLogApp {
    log: TabLog,
    spinner_state: SpinnerState,
    tasks_running: usize,
    tasks_complete: usize,
    last_tick: Instant,
    tick_count: u32,
    quit: bool,
    theme: RatatuiThemeColors,
}

impl ScrollLogApp {
    fn new() -> Self {
        let mut app = Self {
            log: TabLog::new(SCROLL_WIDTH, SCROLL_BUF_LINES),
            spinner_state: SpinnerState::new(SpinnerType::Dot),
            tasks_running: 4,
            tasks_complete: 0,
            last_tick: Instant::now(),
            tick_count: 0,
            quit: false,
            theme: tui_easy::theme::ThemeColors::default().to_ratatui(),
        };

        // Seed with the boot sequence. Every row carries a URL so the
        // status-bar's `top_anchor()` always has something to display.
        for (i, msg) in [
            "workspace loaded",
            "compiling tui-easy-styles",
            "compiling tui-easy-wrap",
            "compiling tui_easy-hyperlink",
            "compiling tui_easy-spinner",
        ]
        .iter()
        .enumerate()
        {
            app.push_row(
                msg.to_string(),
                Level::Info,
                Some(format!("https://github.com/nixpt/tui-easy/blob/main/{i}")),
            );
        }

        // Build the scroll-view buffer once upfront so the very first
        // `draw()` renders the seeded rows. Without this, the buffer
        // is still `Buffer::empty(...)` and the viewport paints blank.
        app.log.rebuild_buffer();

        app
    }

    /// Append a new log row + (optionally) a linked anchor. The
    /// `Level` enum maps to a static `category` string the helper
    /// consumes; the actual `LogRow`, the `anchor_id`, and the OSC 8
    /// entry live inside `TabLog`.
    fn push_row(&mut self, msg: String, level: Level, url: Option<String>) {
        self.log
            .push_row(format_ts(), level.label().to_string(), msg, url);
    }

    fn reset_to_bottom_if_pinned(&mut self) {
        self.log.reset_to_bottom_if_pinned();
    }
}

// ─── TuiApp impl ──────────────────────────────────────────────────────────

impl TuiApp for ScrollLogApp {
    fn name(&self) -> &str {
        "scroll-log"
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn update(&mut self) {
        let dt = self.last_tick.elapsed();
        self.spinner_state.tick(dt);
        self.last_tick = Instant::now();

        self.tick_count = self.tick_count.wrapping_add(1);
        if self.tick_count.is_multiple_of(TASK_COMPLETE_EVERY) && self.tasks_running > 0 {
            self.tasks_running -= 1;
            self.tasks_complete += 1;
            self.push_row(
                format!(
                    "task #{} complete  ({} still running)",
                    self.tasks_complete, self.tasks_running
                ),
                Level::Done,
                Some(format!(
                    "https://github.com/nixpt/tui-easy/actions/runs/{}",
                    self.tasks_complete * 1000
                )),
            );
        }

        self.reset_to_bottom_if_pinned();
        self.log.rebuild_buffer();
    }

    fn handle_event(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::Tick | TuiEvent::Resize(_, _) => {}

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

            TuiEvent::Key(KeyEvent {
                code: KeyCode::Down | KeyCode::Char('j'),
                ..
            }) => {
                // Two-step borrow: evaluate the inner `is_at_bottom()`
                // first so `set_pinned(receiver)` and the inner
                // `scroll_state_mut()` do not collide on `&mut self.log`
                // (round-9 E0499 fix from the compiler's diagnostic).
                let at_bottom = self.log.scroll_state_mut().is_at_bottom();
                self.log.scroll_state_mut().scroll_down();
                self.log.set_pinned(at_bottom);
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Up | KeyCode::Char('k'),
                ..
            }) => {
                self.log.scroll_state_mut().scroll_up();
                self.log.set_pinned(false);
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::PageDown,
                ..
            }) => {
                // Same two-step pattern as the Down/`j` arm — see note there.
                let at_bottom = self.log.scroll_state_mut().is_at_bottom();
                self.log.scroll_state_mut().scroll_page_down();
                self.log.set_pinned(at_bottom);
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::PageUp,
                ..
            }) => {
                self.log.scroll_state_mut().scroll_page_up();
                self.log.set_pinned(false);
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::End | KeyCode::Char('G'),
                ..
            }) => {
                self.log.scroll_state_mut().scroll_to_bottom();
                self.log.set_pinned(true);
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Home | KeyCode::Char('g'),
                ..
            }) => {
                self.log.scroll_state_mut().scroll_to_top();
                self.log.set_pinned(false);
            }

            _ => {}
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        // title (1 row) + body (≥3) + status_bar (2 rows because the bar
        // itself is block-bordered TOP).
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(2),
            ])
            .split(area);

        let title_area = chunks[0];
        let body_area = chunks[1];
        let footer_area = chunks[2];

        // ── Title: spinner frame + counters ──────────────────────────
        let frame_glyph = self.spinner_state.frame_str();
        let title_left = Line::from(vec![
            Span::styled(
                format!(" {frame_glyph} "),
                ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
            ),
            Span::raw(format!(
                "scroll-log │ {} running │ {} complete │ {} rows",
                self.tasks_running,
                self.tasks_complete,
                self.log.row_count(),
            )),
        ]);
        frame.render_widget(Paragraph::new(title_left), title_area);

        // ── Body: the TabLog scroll view's viewport ──────────────────
        // `render_pair()` returns disjoint-borrowed refs into the
        // same `&mut TabLog` so we can hand both to render_stateful_widget
        // without triggering an E0502.
        let pair = self.log.render_pair();
        frame.render_stateful_widget(pair.view, body_area, pair.state);

        // ── Footer: status_bar with scroll position + top anchor ────
        let anchor_part = self
            .log
            .top_anchor()
            .map(|t| format!("anchor #{:04}", t.id))
            .unwrap_or_else(|| "anchor (none)".to_string());

        let scroll_state_y = {
            let s = self.log.scroll_state_mut();
            s.offset().y
        };
        let left_line = format!(
            " y={} │ {} │ j/k ↓/↑ · PgUp/PgDn · g/G · q",
            scroll_state_y, anchor_part,
        );
        let right_line = format!(
            "{} rows │ frame {}{}",
            self.log.row_count(),
            self.spinner_state.frame(),
            if self.log.is_pinned() {
                ""
            } else {
                " │ unpinned"
            },
        );

        frame.render_widget(
            status_bar(left_line, Some(right_line), &self.theme),
            footer_area,
        );
    }
}

// ── Boot ──────────────────────────────────────────────────────────────────

fn main() -> std::io::Result<()> {
    let app = ScrollLogApp::new();
    run_app(app, TICK_RATE)
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn format_ts() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{:08x}", now & 0xFFFF_FFFF)
}

// ── Smoke tests (TestBackend, no real terminal) ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_app() -> ScrollLogApp {
        ScrollLogApp::new()
    }

    fn render_once(app: &mut ScrollLogApp, w: u16, h: u16) -> Buffer {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| app.draw(frame))
            .expect("draw into TestBackend");
        terminal.backend().buffer().clone()
    }

    #[test]
    fn smoke_renders_title_body_and_footer() {
        let mut app = fresh_app();
        let buf = render_once(&mut app, 120, 24);

        // Title row check.
        let mut title_line = String::new();
        for x in 0..buf.area.width {
            title_line.push_str(buf[(x, 0)].symbol());
        }
        assert!(
            title_line.contains("scroll-log"),
            "title row missing app name: {title_line:?}"
        );
        assert!(
            title_line.contains("4 running"),
            "title row missing initial task counter: {title_line:?}"
        );
        assert!(
            title_line.contains("0 complete"),
            "title row missing initial complete-counter: {title_line:?}"
        );

        // Footer check.
        let mut footer_line = String::new();
        for x in 0..buf.area.width {
            footer_line.push_str(buf[(x, buf.area.height - 1)].symbol());
        }
        assert!(
            footer_line.contains("5 rows"),
            "footer missing seed-row-count: {footer_line:?}"
        );
        assert!(
            footer_line.contains("frame 0"),
            "footer missing spinner-frame index: {footer_line:?}"
        );

        // Body region check.
        let mut body_text = String::new();
        for y in 1..buf.area.height.saturating_sub(1) {
            for x in 0..buf.area.width {
                body_text.push_str(buf[(x, y)].symbol());
            }
        }
        assert!(
            body_text.contains("workspace loaded"),
            "scroll body does not contain seeded row 'workspace loaded'"
        );
    }

    #[test]
    fn smoke_update_advances_spinner_and_appends_rows() {
        let mut app = fresh_app();

        // Drive 6 ticks. `update()` is called directly (instead of via
        // `run_app`) so we can observe side effects from CI.
        for _ in 0..6 {
            app.update();
        }

        assert_eq!(app.tasks_complete, 1, "expected 1 task to complete");
        assert_eq!(app.tasks_running, 3, "expected 3 tasks still running");

        // After 6 ticks the app is pinned-to-bottom of the 240-row
        // virtual log buffer, but only 6 rows are populated. Unpin and
        // scroll to the top so the viewport lines up over the populated
        // prefix.
        app.log.set_pinned(false);
        app.log.scroll_state_mut().scroll_to_top();

        let buf = render_once(&mut app, 120, 24);
        let mut body_text = String::new();
        for y in 1..buf.area.height.saturating_sub(1) {
            for x in 0..buf.area.width {
                body_text.push_str(buf[(x, y)].symbol());
            }
        }
        assert!(
            body_text.contains("task #1 complete"),
            "after 6 ticks, body must show 'task #1 complete': {body_text:?}"
        );
    }

    #[test]
    fn smoke_top_anchor_shifts_with_scroll_offset() {
        let mut app = fresh_app();
        app.log.rebuild_buffer();

        // Initially at top → anchor #1 (the very first seeded row).
        assert_eq!(
            app.log.scroll_state_mut().offset(),
            Position::new(0, 0)
        );
        let top = app.log.top_anchor().expect("anchor at offset y=0");
        assert_eq!(top.id, 1, "expected anchor #1 at the top of viewport");
        assert_eq!(top.line_index, 0);

        // Scroll one line down → anchor should now be the second row.
        app.log.scroll_state_mut().scroll_down();
        let top = app.log.top_anchor().expect("anchor at offset y=1");
        assert_eq!(top.id, 2, "expected anchor #2 after one scroll_down");
    }

    #[test]
    fn smoke_quit_keys_set_should_quit() {
        let mut app = fresh_app();

        app.handle_event(TuiEvent::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)));
        assert!(app.should_quit(), "q must set should_quit");

        let mut app = fresh_app();
        app.handle_event(TuiEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(app.should_quit(), "Esc must set should_quit");

        let mut app = fresh_app();
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert!(app.should_quit(), "Ctrl-c must set should_quit");
    }

    #[test]
    fn smoke_scroll_state_responds_to_navigation_keys() {
        let mut app = fresh_app();
        assert!(app.log.is_pinned(), "default is pinned-to-bottom");

        app.handle_event(TuiEvent::Key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)));
        assert!(!app.log.is_pinned(), "k should unpin from bottom");

        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::End,
            KeyModifiers::NONE,
        )));
        assert!(app.log.is_pinned(), "End should re-pin to bottom");

        app.handle_event(TuiEvent::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)));
        assert!(!app.log.is_pinned(), "PageUp should unpin");
    }

    #[test]
    fn smoke_wrap_helper_reachable() {
        // Round 2: `word_wrap_line` accepting a borrowed `Line` and an
        // owned `RtOptions`. This is a reachability + length sanity
        // check, not a deep semantic test of the upstream library.
        let line = Line::raw("the quick brown fox jumps over the lazy dog and keeps running");
        let wrapped = tui_easy::wrap::word_wrap_line(&line, tui_easy::wrap::RtOptions::new(20));
        assert!(
            wrapped.len() >= 2,
            "a 60-char line wrapped to 20 cols must split into >= 2 lines (got {})",
            wrapped.len()
        );
    }
}
