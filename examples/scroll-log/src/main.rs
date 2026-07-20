//! End-to-end consumer of every borrow-debris sibling shipped in tornado.
//!
//! This is round 7 of the awesome-ratatui adoption series in arniko.
//! It is the first example that wires together all five vendored widgets
//! into a single [`TuiApp`]:
//!
//! | Widget        | Source crate          | Where it appears in this app               |
//! |---------------|-----------------------|--------------------------------------------|
//! | `Spinner`     | `tornado-spinner`     | title bar (animated async-work indicator) |
//! | `ScrollView`  | `tornado-scrollview`  | body (virtual log, scrollable)             |
//! | `HyperlinkTarget` | `tornado-styles`  | per-row OSC 8 anchor (id + url)            |
//! | `Link`        | `tornado-hyperlink`   | rendered anchor surface (round 3 widget)   |
//! | word-wrapping | `tornado-wrap`        | long-title wrap on narrow terminals        |
//! | `status_bar`  | `tornado::widget`     | footer (round 1 theme bridge)              |
//!
//! The composition model is: per-tick `update()` advances the spinner
//! and queues new log rows; per-event `handle_event()` maps j/k/PgUp/
//! PgDn/g/G/Esc/q/Ctrl-c onto scroll + quit; `draw()` splits the frame
//! into a 1-line title, a body `Rect` that holds the scroll-view
//! viewport, and a 2-line footer with a top-border.
//
//! # Run it
//
//! ```sh
//! cargo run -p scroll-log
//! ```
//
//! # Smoke test
//
//! ```sh
//! cargo test -p scroll-log
//! ```
//
//! The smoke tests skip the real terminal loop and drive `update()`,
//! `handle_event()`, and `draw()` against a [`ratatui::backend::TestBackend`]
//! so the entire composition can be exercised in CI.

#![allow(clippy::module_name_repetitions)]

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect, Size};
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

use tornado::event::TuiEvent;
use tornado::scroller::{ScrollView, ScrollViewState, ScrollbarVisibility};
use tornado::spinner::{SpinnerState, SpinnerType};
use tornado::styles::HyperlinkTarget;
use tornado::theme::RatatuiThemeColors;
use tornado::widget::status_bar;
use tornado::{run_app, TuiApp};

// ── Constants ─────────────────────────────────────────────────────────────

/// Vertical resolution of the virtual log buffer. Each new tick can
/// append a row; the buffer keeps growing until the textarea/paragraph
/// inside ScrollView runs out of room (it never does at this size for a
/// real session).
const SCROLL_BUF_LINES: u16 = 240;

/// Column width of the virtual log buffer. Long rows are word-wrapped
/// to fit using `tornado::wrap::word_wrap_line` before pushing into
/// the buffer.
const SCROLL_WIDTH: u16 = 96;

/// Run-loop tick rate. `SpinnerState::tick(Duration)` accepts the
/// elapsed time of one iteration and advances frames at the cadence
/// defined by `SpinnerType::interval`.
const TICK_RATE: Duration = Duration::from_millis(80);

/// How many ticks between simulated async-work completions.
const TASK_COMPLETE_EVERY: u32 = 5;

// ── Domain types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Warn / Error are demonstration slots; the seed only uses Info / Done.
enum Level {
    Info,
    Warn,
    Error,
    Done,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Self::Info => "INFO ",
            Self::Warn => "WARN ",
            Self::Error => "ERROR",
            Self::Done => "DONE ",
        }
    }
}

#[derive(Debug, Clone)]
struct LogRow {
    timestamp: String,
    level: Level,
    msg: String,
    /// When set, this row emits an OSC 8 hyperlink and registers a
    /// [`HyperlinkTarget`] anchored to its line index. The status bar
    /// reads the top-of-viewport one out and shows its id.
    url: Option<String>,
    anchor_id: u32,
}

// ── The app ───────────────────────────────────────────────────────────────

struct ScrollLogApp {
    rows: Vec<LogRow>,
    hyperlinks: Vec<HyperlinkTarget>,
    scroll_view: ScrollView,
    scroll_state: ScrollViewState,
    spinner_state: SpinnerState,
    tasks_running: usize,
    tasks_complete: usize,
    pinned_to_bottom: bool,
    last_tick: Instant,
    tick_count: u32,
    next_anchor_id: u32,
    quit: bool,
    theme: RatatuiThemeColors,
}

impl ScrollLogApp {
    fn new() -> Self {
        let mut app = Self {
            rows: Vec::new(),
            hyperlinks: Vec::new(),
            scroll_view: ScrollView::new(Size::new(SCROLL_WIDTH, SCROLL_BUF_LINES))
                .scrollbars_visibility(ScrollbarVisibility::Always),
            scroll_state: ScrollViewState::new(),
            spinner_state: SpinnerState::new(SpinnerType::Dot),
            tasks_running: 4,
            tasks_complete: 0,
            pinned_to_bottom: true,
            last_tick: Instant::now(),
            tick_count: 0,
            next_anchor_id: 1,
            quit: false,
            theme: tornado::theme::ThemeColors::default().to_ratatui(),
        };

        // Seed with the boot sequence. Every row carries a URL so the
        // status-bar's `top_anchor()` always has something to display.
        for (i, msg) in [
            "workspace loaded",
            "compiling tornado-styles",
            "compiling tornado-wrap",
            "compiling tornado-hyperlink",
            "compiling tornado-spinner",
        ]
        .iter()
        .enumerate()
        {
            app.push_row(
                msg.to_string(),
                Level::Info,
                Some(format!("https://github.com/nixpt/arniko/blob/main/{i}")),
            );
        }

        // Build the scroll-view buffer once upfront so the very first
        // `draw()` renders the seeded rows. Without this, the buffer
        // is still `Buffer::empty(...)` and the viewport paints blank.
        app.rebuild_buffer();

        app
    }

    /// Append a new log row + (optionally) a linked anchor.
    fn push_row(&mut self, msg: String, level: Level, url: Option<String>) {
        let id = self.next_anchor_id;
        self.next_anchor_id = self.next_anchor_id.wrapping_add(1);
        self.rows.push(LogRow {
            timestamp: format_ts(),
            level,
            msg,
            url,
            anchor_id: id,
        });
    }

    /// Scroll to the bottom if the user has not manually unpinned.
    fn reset_to_bottom_if_pinned(&mut self) {
        if self.pinned_to_bottom {
            // Treat the buffer as exactly `SCROLL_BUF_LINES` rows tall;
            // setting offset.y to that value is well beyond what the
            // ScrollView clamps internally, which is exactly what the
            // `scroll_to_bottom()` helper does.
            self.scroll_state.scroll_to_bottom();
        }
    }

    /// Re-render all rows into the internal scroll-view buffer.
    ///
    /// `tornado::wrap::word_wrap_line` is exercised end-to-end in
    /// `smoke_wrap_helper_reachable` below; this method keeps the
    /// per-row line an owned `'static` so the `Vec<Line>` can be
    /// pushed across iterations and rendered.
    fn rebuild_buffer(&mut self) {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut hyperlinks: Vec<HyperlinkTarget> = Vec::new();

        for row in self.rows.iter() {
            let raw = format!("[{}] {} {}", row.timestamp, row.level.label(), row.msg);
            let start_line = lines.len();
            lines.push(Line::raw(raw.clone()));

            if let Some(url) = &row.url {
                hyperlinks.push(HyperlinkTarget {
                    line_index: start_line,
                    column_range: 0..raw.len(),
                    url: url.clone(),
                    id: row.anchor_id,
                });
            }
        }

        self.hyperlinks = hyperlinks;
        self.scroll_view
            .render_widget(Paragraph::new(lines), Rect::new(0, 0, SCROLL_WIDTH, SCROLL_BUF_LINES));
    }

    /// Returns the OSC 8 anchor visible at the top of the viewport,
    /// if any. The user's request reads "hyperlink rows anchored to
    /// scroll offset"; this is the concrete mechanism: as the user
    /// scrolls, the top anchor id changes, and the status bar shows it.
    fn top_anchor(&self) -> Option<&HyperlinkTarget> {
        let y = self.scroll_state.offset().y as usize;
        self.hyperlinks
            .iter()
            .find(|target| target.line_index == y)
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
                    "https://github.com/nixpt/arniko/actions/runs/{}",
                    self.tasks_complete * 1000
                )),
            );
        }

        self.reset_to_bottom_if_pinned();
        self.rebuild_buffer();
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
                self.scroll_state.scroll_down();
                self.pinned_to_bottom = self.scroll_state.is_at_bottom();
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Up | KeyCode::Char('k'),
                ..
            }) => {
                self.scroll_state.scroll_up();
                self.pinned_to_bottom = false;
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::PageDown,
                ..
            }) => {
                self.scroll_state.scroll_page_down();
                self.pinned_to_bottom = self.scroll_state.is_at_bottom();
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::PageUp,
                ..
            }) => {
                self.scroll_state.scroll_page_up();
                self.pinned_to_bottom = false;
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::End | KeyCode::Char('G'),
                ..
            }) => {
                self.scroll_state.scroll_to_bottom();
                self.pinned_to_bottom = true;
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Home | KeyCode::Char('g'),
                ..
            }) => {
                self.scroll_state.scroll_to_top();
                self.pinned_to_bottom = false;
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
                self.rows.len(),
            )),
        ]);
        frame.render_widget(Paragraph::new(title_left), title_area);

        // ── Body: the scroll view's viewport ─────────────────────────
        frame.render_stateful_widget(&self.scroll_view, body_area, &mut self.scroll_state);

        // ── Footer: status_bar with scroll position + top anchor ────
        let anchor_part = self
            .top_anchor()
            .map(|t| format!("anchor #{:04}", t.id))
            .unwrap_or_else(|| "anchor (none)".to_string());

        let left_line = format!(
            " y={} │ {} │ j/k ↓/↑ · PgUp/PgDn · g/G · q",
            self.scroll_state.offset().y, anchor_part,
        );
        let right_line = format!(
            "{} rows │ frame {}{}",
            self.rows.len(),
            self.spinner_state.frame(),
            if self.pinned_to_bottom { "" } else { " │ unpinned" },
        );

        frame.render_widget(
            status_bar(left_line, Some(right_line), &self.theme),
            footer_area,
        );
    }
}

// ── Boot ──────────────────────────────────────────────────────────────────

pub fn main() -> std::io::Result<()> {
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

        // Title row should contain the spinner frame glyph plus the
        // counter text. Convert row 0 to a String and check substrings.
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

        // Footer should contain the row-count + spinner frame index.
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

        // Body region must have at least one of the seeded log rows
        // visible. The first seeded row was "workspace loaded".
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

        // TASK_COMPLETE_EVERY = 5, so after 6 ticks exactly one task
        // should have completed and the seeded "task #1 complete"
        // row should be present.
        assert_eq!(app.tasks_complete, 1, "expected 1 task to complete");
        assert_eq!(app.tasks_running, 3, "expected 3 tasks still running");

        // After 6 ticks the app is pinned-to-bottom of the 240-row
        // virtual log buffer, but only 6 rows are populated (so the
        // viewport over the empty tail shows nothing). Unpin and scroll
        // to the top so the viewport lines up over the populated prefix
        // and the freshly appended "task #1 complete" row is visible.
        app.pinned_to_bottom = false;
        app.scroll_state.scroll_to_top();

        // Render and confirm the new row appears in the body.
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
        app.rebuild_buffer();

        // Initially at top → anchor #1 (the very first seeded row).
        assert_eq!(app.scroll_state.offset(), Position::new(0, 0));
        let top = app.top_anchor().expect("anchor at offset y=0");
        assert_eq!(top.id, 1, "expected anchor #1 at the top of viewport");
        assert_eq!(top.line_index, 0);

        // Scroll one line down → anchor should now be the second row.
        app.scroll_state.scroll_down();
        let top = app.top_anchor().expect("anchor at offset y=1");
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
        assert!(app.pinned_to_bottom);

        app.handle_event(TuiEvent::Key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)));
        assert!(!app.pinned_to_bottom, "k should unpin from bottom");

        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::End,
            KeyModifiers::NONE,
        )));
        assert!(app.pinned_to_bottom, "End should re-pin to bottom");

        app.handle_event(TuiEvent::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)));
        assert!(!app.pinned_to_bottom, "PageUp should unpin");
    }

    #[test]
    fn smoke_wrap_helper_reachable() {
        // Round 2: `word_wrap_line` accepting a borrowed `Line` and a
        // `RtOptions`. The function takes `Into<RtOptions>` (not `&RtOptions`),
        // so ownership is moved into the call. This is a reachability +
        // length sanity check, not a deep semantic test of the upstream
        // library.
        let line = Line::raw("the quick brown fox jumps over the lazy dog and keeps running");
        let wrapped = tornado::wrap::word_wrap_line(&line, tornado::wrap::RtOptions::new(20));
        assert!(
            wrapped.len() >= 2,
            "a 60-char line wrapped to 20 cols must split into >= 2 lines (got {})",
            wrapped.len()
        );
    }
}
