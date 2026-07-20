//! Per-tab virtual-scroll log primitive.
//!
//! Lifts the round-7 `ScrollLogApp`'s per-tab mechanics — row registry,
//! OSC-8 anchor assignment, buffer rebuild, top-of-viewport anchor
//! lookup, and offset preservation — into a reusable primitive that
//! round-8 + any future multi-stream consumer can compose under one or
//! more [`crate::widget::TabNav`] instances.
//!
//! Feature-gated behind `log_view = ["dep:tornado-styles",
//! "dep:tornado-scrollview", "dep:tornado-wrap"]`. The umbrella pulls
//! in the three vendored sub-crates that the rebuild pipeline touches:
//!
//! * `tornado-styles` for the [`HyperlinkTarget`] registry shape.
//! * `tornado-scrollview` for [`ScrollView`] + [`ScrollViewState`].
//! * `tornado-wrap` for `word_wrap` consumers that opt in (the
//!   smoke test exercises the helper end-to-end).
//!
//! Public surface is intentionally tight: most fields are private;
//! accessors expose the bits render code (`Frame`) and event code
//! (`TuiEvent::Key` match) need. Encoding ownership keeps the borrow
//! story simple in the consumer.

use ratatui::layout::{Rect, Size};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use tornado_scrollview::{ScrollView, ScrollViewState};
use tornado_styles::HyperlinkTarget;

/// One row in a [`TabLog`]'s virtual buffer.
///
/// `category` is an owned string rather than a typed enum so the
/// helper stays domain-agnostic; the consumer decides what to render
/// in that column (e.g. "INFO ", "WARN ", a timestamp prefix, an HTTP
/// status code, …).
#[derive(Debug, Clone)]
pub struct LogRow {
    pub timestamp: String,
    pub category: String,
    pub msg: String,
    /// When set, this row registers a [`HyperlinkTarget`] whose
    /// `line_index` matches the row's start position in the rendered
    /// buffer. The top-of-viewport anchor lookup picks it out.
    pub url: Option<String>,
    /// Stable id assigned at `push_row` time. The `id` is what the
    /// status bar shows so the user can tell which anchor is at the
    /// top of the viewport as they scroll.
    pub anchor_id: u32,
}

/// One streamed log: virtual buffer + anchor registry + scroll state.
///
/// Designed so a multi-tab TuiApp can hold `Vec<TabLog>` keyed to its
/// [`crate::widget::TabNav`] titles. Each [`TabLog`] is independent —
/// switching tabs preserves each one's scroll offset natively because
/// the [`ScrollViewState`] is per-instance.
#[derive(Debug)]
pub struct TabLog {
    rows: Vec<LogRow>,
    hyperlinks: Vec<HyperlinkTarget>,
    scroll_view: ScrollView,
    scroll_state: ScrollViewState,
    pinned_to_bottom: bool,
    next_anchor_id: u32,
    buf_width: u16,
    buf_height: u16,
}

/// Pair of disjoint-borrowed references into a [`TabLog`]'s scroll
/// viewport. Returned by [`TabLog::render_pair`] so a consumer can
/// hand both refs to [`ratatui::Frame::render_stateful_widget`]
/// without triggering E0502: `view` is `&ScrollView` (immutable) and
/// `state` is `&mut ScrollViewState` (mutable). The two underlying
/// fields are disjoint on the same [`TabLog`], so a pair-returning
/// reborrow is sound.
pub struct TabLogRender<'a> {
    pub view: &'a ScrollView,
    pub state: &'a mut ScrollViewState,
}

impl TabLog {
    /// Create a new virtual-scrollable log with the given buffer
    /// dimensions. The internal buffer is `Buffer::empty`; call
    /// [`push_row`](Self::push_row) + [`rebuild_buffer`](Self::rebuild_buffer)
    /// to populate it before the first `draw`.
    pub fn new(buf_width: u16, buf_height: u16) -> Self {
        Self {
            rows: Vec::new(),
            hyperlinks: Vec::new(),
            scroll_view: ScrollView::new(Size::new(buf_width, buf_height)),
            scroll_state: ScrollViewState::new(),
            // Default to pinned-to-bottom on launch — round-7's
            // behaviour. Consumers set_pinned(false) on user-unpin.
            pinned_to_bottom: true,
            next_anchor_id: 1,
            buf_width,
            buf_height,
        }
    }

    /// Return a pair of disjoint-borrowed references into the
    /// virtual scroll viewport. Useful for `Frame::render_stateful_widget`,
    /// which simultaneously wants `&ScrollView` AND
    /// `&mut ScrollViewState`. Direct field access (`&self.scroll_view`
    /// and `&mut self.scroll_state`) sidesteps the E0502 the
    /// struct-literal-init form triggers when both helper methods
    /// were invoked on the same `&mut self`; the fields are disjoint
    /// so disjoint-borrows are sound under edition 2021+.
    pub fn render_pair(&mut self) -> TabLogRender<'_> {
        TabLogRender {
            view: &self.scroll_view,
            state: &mut self.scroll_state,
        }
    }

    /// Append a row to the log stream. `anchor_id` is assigned
    /// sequentially so the anchor registry remains stable across
    /// rebuilds. Hyperlinks are NOT registered yet — that happens at
    /// the next [`rebuild_buffer`](Self::rebuild_buffer) call.
    pub fn push_row(
        &mut self,
        timestamp: String,
        category: String,
        msg: String,
        url: Option<String>,
    ) {
        let id = self.next_anchor_id;
        self.next_anchor_id = self.next_anchor_id.wrapping_add(1);
        self.rows.push(LogRow {
            timestamp,
            category,
            msg,
            url,
            anchor_id: id,
        });
    }

    /// Re-render the accumulated rows into the internal `ScrollView`
    /// paragraph. Call once per tick after all [`push_row`](Self::push_row)
    /// calls have happened; this avoids N rebuilds per row-batch.
    ///
    /// Owned `Vec<Line<'static>>` keeps the lifecycle simple (no
    /// `word_wrap_line` cross-iteration borrow collisions — round-7
    /// collision #2 carries forward; the helper deliberately avoids
    /// calling wrap here so consumers can opt in via tests).
    pub fn rebuild_buffer(&mut self) {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut hyperlinks: Vec<HyperlinkTarget> = Vec::new();

        for row in self.rows.iter() {
            let raw = format!("[{}] {} {}", row.timestamp, row.category, row.msg);
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
        self.scroll_view.render_widget(
            Paragraph::new(lines),
            Rect::new(0, 0, self.buf_width, self.buf_height),
        );
    }

    /// Returns the OSC 8 anchor visible at the top of the viewport.
    ///
    /// As the user scrolls, `scroll_state.offset().y` advances; this
    /// picks the first hyperlink whose `line_index` matches that y.
    /// `None` while the top row has no URL set.
    pub fn top_anchor(&self) -> Option<&HyperlinkTarget> {
        let y = self.scroll_state.offset().y as usize;
        self.hyperlinks
            .iter()
            .find(|target| target.line_index == y)
    }

    /// Scroll offset to the bottom if the user has not manually
    /// unpinned; otherwise no-op. Call this in `TuiApp::update()`
    /// right before `rebuild_buffer`.
    pub fn reset_to_bottom_if_pinned(&mut self) {
        if self.pinned_to_bottom {
            self.scroll_state.scroll_to_bottom();
        }
    }

    // ── Accessors ──────────────────────────────────────────────────

    /// Mutable handle to the underlying scroll state. Used by
    /// `Frame::render_stateful_widget` and by the consumer's
    /// `handle_event` to dispatch scroll keys.
    pub fn scroll_state_mut(&mut self) -> &mut ScrollViewState {
        &mut self.scroll_state
    }

    /// Read-only view of the scroll view, passed to
    /// `Frame::render_stateful_widget` along with [`scroll_state_mut`](Self::scroll_state_mut).
    pub fn scroll_view(&self) -> &ScrollView {
        &self.scroll_view
    }

    /// `true` if the user has not manually unpinned. The consumer
    /// flips this via [`set_pinned`](Self::set_pinned) when `j` /
    /// `↑` / `PgUp` / `g` are pressed or when a scroll operation
    /// leaves the viewport away from the bottom.
    pub fn is_pinned(&self) -> bool {
        self.pinned_to_bottom
    }

    pub fn set_pinned(&mut self, pinned: bool) {
        self.pinned_to_bottom = pinned;
    }

    /// Number of rows currently in the log stream.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

// ── Smoke tests (TestBackend-free; pure unit on TabLog) ────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_row_and_rebuild_assigns_ids_and_anchors() {
        let mut log = TabLog::new(80, 200);

        log.push_row("t1".into(), "INFO".into(), "first".into(), Some("https://a/1".into()));
        log.push_row("t2".into(), "INFO".into(), "second".into(), None);
        log.push_row(
            "t3".into(),
            "DONE".into(),
            "third".into(),
            Some("https://a/3".into()),
        );
        log.rebuild_buffer();

        assert_eq!(log.row_count(), 3);

        let anchors = registry(&log);
        assert_eq!(anchors.len(), 2, "two rows carried a url");
        assert_eq!(anchors[0].id, 1);
        assert_eq!(anchors[0].line_index, 0);
        assert_eq!(anchors[0].url, "https://a/1");
        assert_eq!(anchors[1].id, 3, "anchor ids are sequential across the whole stream");
        assert_eq!(anchors[1].line_index, 2);
    }

    #[test]
    fn top_anchor_calculates_correct_offset() {
        let mut log = TabLog::new(80, 200);
        log.push_row("t1".into(), "INFO".into(), "row-zero".into(), Some("https://a/0".into()));
        log.push_row("t1".into(), "INFO".into(), "row-one".into(), Some("https://a/1".into()));
        log.rebuild_buffer();

        // Initial state: offset (0, 0) → anchor at line_index 0.
        log.scroll_state_mut().scroll_to_top();
        let top = log.top_anchor().expect("an anchor at offset y=0");
        assert_eq!(top.id, 1);

        // Scroll down past the populated prefix; no anchor matches line_index=200.
        log.scroll_state_mut().set_offset(ratatui::layout::Position::new(0, 200));
        assert!(log.top_anchor().is_none(), "no anchor at line_index 200");
    }

    #[test]
    fn pinning_preserves_bottom_intent() {
        let mut log = TabLog::new(80, 200);
        log.push_row("t1".into(), "INFO".into(), "x".into(), None);
        log.rebuild_buffer();
        log.scroll_state_mut().scroll_to_top();
        assert!(log.is_pinned(), "default is pinned-to-bottom");

        // Pinned: reset_to_bottom_if_pinned moves offset to the
        // bottom of the viewport (the precise y is clamped by the
        // ScrollView, which is library-internal — assert the
        // observable instead: `is_at_bottom()` after reset).
        log.reset_to_bottom_if_pinned();
        assert!(
            log.scroll_state_mut().is_at_bottom(),
            "pinned reset should land at the bottom of the viewport"
        );

        // Unpinned: reset leaves the offset alone.
        log.scroll_state_mut().scroll_to_top();
        log.set_pinned(false);
        log.reset_to_bottom_if_pinned();
        assert_eq!(
            log.scroll_state_mut().offset().y,
            0,
            "unpinned reset is a no-op (offset stays at top)"
        );
    }

    // ── Test helpers ────────────────────────────────────────────────

    /// Helper: pull the registered [`HyperlinkTarget`] registry out of
    /// the private field.
    fn registry(log: &TabLog) -> Vec<HyperlinkTarget> {
        log.hyperlinks.clone()
    }

    // ── Borrow-safety test (regression for round-9 E0502 fix) ──────

    /// Verify [`TabLog::render_pair`] returns a pair where `view` is
    /// `&ScrollView` and `state` is `&mut ScrollViewState` — ie the
    /// two sub-borrows are disjoint and the helper compiles under
    /// edition 2021+ disjoint-field-borrow rules.
    #[test]
    fn render_pair_yields_disjoint_borrows() {
        let mut log = TabLog::new(80, 32);
        log.push_row("t1".into(), "INFO".into(), "x".into(), None);
        log.rebuild_buffer();

        let TabLogRender { view, state } = log.render_pair();

        // Both refs are alive simultaneously — Rust enforces
        // disjointness at compile time.
        let _view_area = view.area();
        state.scroll_down();
    }
}
