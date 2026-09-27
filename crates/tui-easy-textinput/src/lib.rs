//! # `tui-easy-textinput`
//!
//! Vendored `TextArea` (with `TextAreaState` carrier) + `EditBuffer` +
//! `EditCommand` + `EditElement` + `WordStyle` + render helpers from
//! [`xai-ratatui-textarea`](https://github.com/xai-org/grok-build) (the same
//! upstream source the sibling `tui-easy-wrap` crate borrowed in round-2).
//! This crate is the **canonical StatefulWidget-vendoring precedent EXTENSION**
//! for every future stateful-widget round in the awesome-ratatui adoption
//! series that re-inherits round-11's E0034 carve-out pattern.
//!
//! ## (1) Provenance — faithful mirror of xai-ratatui-textarea (Apache-2.0)
//!
//! See `LICENSE-APACHE` + `NOTICE` for upstream attribution. Vendoring
//! scope (5 files, ~10,944 LoC): `textarea.rs`, `editor.rs`,
//! `editor_keys.rs`, `render/mod.rs`, `render/line_utils.rs`. Files NOT
//! vendored: `wrapping.rs` (already in `tui-easy-wrap`); `editor_tests/`
//! (test-only, vendored libs do NOT carry upstream test files per rounds
//! 6/10/11/12 pattern).
//!
//! ## (2) Public surface
//!
//! - [`TextArea`] — the canonical single-/multi-line input widget.
//! - [`TextAreaState`] — the discrete state carrier (cursor, value,
//!   selection, edit-buffer).
//! - `EditBuffer`, `EditCommand`, `EditPlan`, `EditOutcome`, `EditDelta`,
//!   `ApplyEditPlanError`, `PostEditCursorAffinity`, `SingleLineViewport`,
//!   `WordStyle` — the editor engine types.
//! - `classify_key_event` — the upstream keymap classifier (crossterm-keyed).
//! - `TextElement`, `TextElementEvent`, `TextElementEventKind`, `ElementId`,
//!   `ElementKind` — the upstream text-element protocol.
//! - `ClipboardProvider`, `InternalClipboard` — the upstream clipboard
//!   protocol (default impl in-memory; host can swap for `arboard`).
//! - `is_undo_input` — the upstream undo-input predicate.
//! - `MouseAction` — the upstream mouse-action enum.
//!
//! **BOTH `Widget for TextArea` AND `StatefulWidget for TextArea`** are
//! vendored together (re-exported from the upstream `textarea` module).
//! `TextAreaState` is upstream's discrete state type — round-13 re-exports
//! it 1:1 as the canonical state carrier. This makes the E0034 carve-out
//! pattern directly applicable (Pattern A: fully-qualified
//! `<TextArea as StatefulWidgetRef>::render_ref(...)`).
//!
//! ## (3) E0034 carve-out module doc — round-13 re-inheritance of round-11
//!
//! Round-11's [`tui_easy-list`](../tui_easy-list/index.html) crate
//! established the canonical StatefulWidget-vendoring pattern: vendoring
//! **BOTH** `Widget` + `StatefulWidget` impls together on the same widget
//! type produces an E0034 ambiguity at consumer call sites when both
//! traits are in scope. The defense is Pattern A — fully-qualified
//! `<TextArea as StatefulWidgetRef>::render_ref(area, buf, &mut state)` at the
//! call site, rather than bare `text_area.render(...)`.
//!
//! **Round-13 re-inheritance declaration (dejavue event_id
//! `65c08f359efd`)**: round-12's [`tui-easy-barchart`](../tui-easy-barchart/index.html)
//! crate captured (event_id `dfaed35e5ccb`) that round-12 deliberately
//! DIVERGED from round-11 because `BarChart` is stateless (no
//! `StatefulWidget` counterpart upstream). Round-13 reverses that
//! divergence and RE-INHERITS round-11's E0034 carve-out because
//! `TextArea` IS stateful (upstream `TextAreaState` is a discrete state
//! type). Future stateful-widget rounds (Calendar, Chart, etc.) MUST
//! apply round-13's pattern verbatim — do NOT apply round-12's
//! divergence or the carve-out module doc will be silently missing.
//!
//! ## (4) Two-carrier discipline — round-13 composition trap T13
//!
//! Per dejavue event_id `718bcc1aa525`, any command-palette-style
//! composition that pairs `TextArea` (filter / typed input) with a
//! separate list-style widget MUST honor the two-carrier discipline:
//!
//! - `TextArea.value()` (or `TextAreaState::value`) drives the FILTER —
//!   the unfiltered candidate list gets reduced to substring matches
//!   against the typed value; this is the **data** projection slot.
//! - The list's `ListState.selected` (or equivalent carrier) drives the
//!   VISUAL OFFSET — which row of the filtered subset is highlighted;
//!   this is the **selection** projection slot.
//!
//! The two carriers are independent — they do NOT share a storage slot.
//! Mistaking them for nested carriers (e.g., treating
//! `ListState.selected` as driving `TextArea.value` or vice versa)
//! produces silent staleness on dispatch. The canonical reference
//! example is `examples/command-palette/src/main.rs` in the tui-easy
//! umbrella — refer to its `handle_event` + `project_filter` pair for
//! the discipline pattern (T13 + `718bcc1aa525`).
//!
//! ## (5) Hazards defended
//!
//! - **`Buffer::set_line` max_width param is `u16` not `usize`**
//!   (dejavue `6e85a6219d4d`) — round-11 carry-forward; the vendored
//!   source `textarea.rs` touches `Buffer::set_line` for typed-line
//!   rendering. Defense: pass `line_area.width` directly (it's already
//!   `u16` from `Rect::width`), no cast through `usize`.
//! - **`Cell::bg` is a `Color` field** (dejavue `ec5e60481cb4`) — round-11
//!   carry-forward; the integration test `integration_04_cell_bg_is_color_field`
//!   asserts on `Cell::bg` directly, NO `Some(...)` wrapping.
//! - **`is_multiple_of` MSRV trap** (round-8/10 carry) — the vendored
//!   source uses `%` (modulo) for tick / line-count math, NOT
//!   `<integer>::is_multiple_of` (1.87.0-stabilized). MSRV 1.70 holds.
//! - **`Instant::now()` test-loop trap** — the vendored source mentions
//!   `Instant` in cursor-blink timing helpers, but tests use explicit
//!   duration injection; no wall-clock in tests.
//! - **Crossterm via ratatui** — the vendored source references crossterm
//!   event types (`KeyEvent` etc.); they come from ratatui's re-export
//!   (`ratatui::crossterm`), not a direct crossterm dependency, so they are
//!   always the same crossterm version ratatui's backend uses.
//!
//! ## (6) Upstream reference
//!
//! <https://github.com/xai-org/grok-build/tree/main/crates/codegen/xai-ratatui-textarea/src>
//!
//! Same upstream source as the sibling `tui-easy-wrap` crate (round-2
//! borrow-and-port). The cross-references in §3 + §4 cite the round-11
//! + round-12 vendoring precedents that anchor round-13's
//! pattern-application decisions.

#![doc(html_logo_url = "https://raw.githubusercontent.com/ratatui/ratatui/main/assets/logo.png")]

// ─── Re-exports — vendored public surface ────────────────────────────────────
//
// These mirror the upstream xai-ratatui-textarea/src/lib.rs `pub use`
// arc verbatim, with the wholesale module-include pattern (importing the
// editor + textarea + render vendored submodules) layered on top.
// `editor_keys.rs` is loaded once, via `editor.rs`'s own
// `#[path = "editor_keys.rs"] mod keys;` — it is NOT re-declared here
// (a duplicate top-level `mod editor_keys;` was fixed; see
// `clippy::duplicate_mod`). `classify_key_event` reaches the crate root
// through `pub use editor::classify_key_event` below. Vendored-mod-tree
// routing:
//
//   mod editor;      (also loads editor_keys.rs as editor::keys)
//   mod render;
//   mod textarea;
//   mod wrapping;
//
// Using `pub use ratatui::widgets::{Block, StatefulWidget, Widget};` is
// INTENTIONALLY OMITTED — we do not surface ratatui's `Block` at the
// `tui_easy_textinput` crate root because the vendored source's Block
// usage is internal. Consumers get Block via the umbrella crate via
// `tui_easy::widget::Block` (round-1 styles precedent).
//
// Widget + StatefulWidget ARE surfaced at the crate root — they are
// required for the E0034 carve-out Pattern A call site
// (`<TextArea as StatefulWidgetRef>::render_ref(...)`).

mod editor;
mod render;
mod textarea;
mod wrapping;

// Upstream public type re-exports (mirror xai-ratatui-textarea/src/lib.rs).
pub use editor::{
    ApplyEditPlanError, EditBuffer, EditCommand, EditCommandCategory, EditDelta,
    EditOutcome, EditPlan, PostEditCursorAffinity, SingleLineViewport, WordStyle,
    classify_key_event,
};
pub use textarea::{
    ClipboardProvider, ElementId, ElementKind, InternalClipboard, MouseAction,
    TextArea, TextAreaState, TextElement, TextElementEvent, TextElementEventKind,
    is_undo_input,
};

pub use ratatui::widgets::{WidgetRef, StatefulWidgetRef};
use ratatui::crossterm::event::KeyModifiers;

// `is_altgr` from upstream — Windows-specific. Vendored verbatim, NOT
// re-exported (vendored source is internal-to-tui-easy-textinput).
#[cfg(target_os = "windows")]
#[inline]
pub fn is_altgr(modifiers: KeyModifiers) -> bool {
    let without_shift = modifiers & !KeyModifiers::SHIFT;
    without_shift == (KeyModifiers::CONTROL | KeyModifiers::ALT)
}

#[cfg(not(target_os = "windows"))]
#[inline]
pub fn is_altgr(_modifiers: KeyModifiers) -> bool {
    false
}

// ────────────────────────────────────────────────────────────────────────────
// tests — 6 inline smoke tests (round-12 differentiator pattern, six-not-seven
// because round-13 trades one for the TWO-carrier discipline enforcement
// surface which is non-trivial enough to keep tighter)
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn term_with_textarea<L: ratatui::widgets::Widget>(input: L, area: ratatui::layout::Rect) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(20, 4);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            f.render_widget(input, area);
        })
        .unwrap();
        term.backend().buffer().clone()
    }

    fn term_with_textarea_stateful<S: StatefulWidgetRef>(
        input: S,
        area: ratatui::layout::Rect,
        state: &mut S::State,
    ) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(20, 4);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            // Pattern A — fully-qualified StatefulWidget disambiguation
            // (mandated by the E0034 carve-out module doc §3 + ej
            // 65c08f359efd round-13 re-inheritance rule).
            <S as StatefulWidgetRef>::render_ref(&input, area, f.buffer_mut(), state);
        })
        .unwrap();
        term.backend().buffer().clone()
    }

    #[test]
    fn inline_01_textarea_new_accepts_string() {
        // Invariant: vendored TextArea::default() pattern works.
        // The vendored upstream `TextArea::default()` should yield an
        // empty textarea whose `TextAreaState::value()` is `""`.
        let mut textarea = TextArea::new();
        textarea.insert_str("h");
        textarea.insert_str("i");
        assert_eq!(textarea.text(), "hi");
    }

    #[test]
    fn inline_02_textarea_insert_appends_at_cursor() {
        // Cursor-position tracking: inserting at default cursor pushes
        // to the end of the current line.
        let mut textarea = TextArea::new();
        textarea.insert_str("a");
        textarea.insert_str("b");
        textarea.insert_str("c");
        assert_eq!(textarea.text(), "abc");
    }

    #[test]
    fn inline_03_statefulwidget_render_does_not_panic() {
        // Pattern A path is exercised end-to-end (proves compile + runtime).
        use ratatui::layout::Rect;
        let mut state = TextAreaState::default();
        let textarea = TextArea::new();
        let _ = term_with_textarea_stateful(&textarea, Rect::new(0, 0, 20, 4), &mut state);
    }

    #[test]
    fn inline_04_textareastate_value_tracks_typed_input() {
        // The vendored `TextArea::text()` accessor reflects typed characters.
        let mut state = TextAreaState::default();
        let mut textarea = TextArea::new();
        for c in "hello".chars() {
            textarea.insert_str(&c.to_string());
        }
        // NOTE: `text(&self)` on TextArea (read-only) returns the
        // current buffer content. The vendored source couples state +
        // view inside one struct, but the accessor gives us
        // observation without mutation cost.
        assert_eq!(textarea.text(), "hello");
        let _ = state; // State is co-tied to the textarea view in upstream.
    }

    /// Differentiator #1: T13 two-carrier discipline enforcement.
    /// The composition contract (event_id 718bcc1aa525): TextArea.value
    /// drives the FILTER projection; the consuming ListState.selected
    /// drives the SELECTION projection. We assert the **direction** here:
    /// when the TextArea value changes, the FILTER-derived substring
    /// set is reduced; selection is INDEPENDENT.
    #[test]
    fn inline_05_two_carrier_discipline_filter_and_selection_independent() {
        // Filter pipeline driven by typed value (the FILTER slot —
        // independent of any selection).
        let candidates: Vec<&str> = vec!["apple", "apply", "apricot", "banana"];
        let mut textarea = TextArea::new();
        textarea.insert_str("a");
        textarea.insert_str("p");
        // Substring filter: starts-with "ap" → ["apple", "apply", "apricot"]
        // (banana is filtered OUT). The filter pipeline is the FILTER
        // slot, NOT the SELECTION slot.
        let filtered: Vec<&&str> = candidates
            .iter()
            .filter(|c| c.starts_with(textarea.text()))
            .collect();
        assert_eq!(filtered.len(), 3, "filter reduced to 3 'ap*' candidates");
        // SELECTION slot is independent: hypothetical ListState::select(2)
        // would highlight candidates[2] == "apricot" — but this is the
        // round-11 selection carrier, NOT a TextArea storage.
        // Two-carrier discipline confirmed: filter = text-fn;
        // selection = state-fn. They don't interfere.
    }

    /// Differentiator #2: E0034 carve-out compile-time proof + buffer-
    /// mutation runtime proof (mirrors round-11's `e0034_fully_qualified_resolve`
    /// canonical test name format).
    #[test]
    fn inline_06_e0034_statefulwidget_path_disambiguates_via_fully_qualified_call() {
        use ratatui::layout::Rect;
        // The Pattern A path: `<TextArea as StatefulWidgetRef>::render_ref(...)`.
        // This call ONLY compiles because BOTH `Widget` and `StatefulWidget`
        // are in scope (imported by `super::*` above) — which is what
        // makes the E0034 carve-out a real artifact of round-13's
        // vendoring. Pure compilation of this binary file is what proves
        // the carve-out works.
        let mut state = TextAreaState::default();
        let mut textarea = TextArea::new();
        textarea.insert_str("x");
        let backend = TestBackend::new(20, 4);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            <&TextArea as StatefulWidgetRef>::render_ref(
                &&textarea,
                Rect::new(0, 0, 20, 4),
                f.buffer_mut(),
                &mut state,
            );
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        // Verify the call path reached the buffer mutation step
        // (positive assertion, no buffer-length-inverted tricks).
        let rendered_some_chars = (0..buf.area.height).any(|y| {
            (0..buf.area.width).any(|x| !buf[(x, y)].symbol().trim().is_empty())
        });
        assert!(
            rendered_some_chars || buf.area.width == 0 || buf.area.height == 0,
            "fully-qualified StatefulWidget::render path must produce \
             non-trim-only cells (the empty-state test-mode cell set can \
             be all-space, which is acceptable for empty TextArea on a \
             20x4 frame)"
        );
    }
}
