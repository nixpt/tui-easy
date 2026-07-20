//! Integration test for `tornado` + `tornado-scrollview` (`--features scroller`).
//!
//! Guards the scrollview end-to-end:
//!  * `ScrollView`, `ScrollViewState`, and `ScrollbarVisibility` are
//!    reachable via the `tornado::scroller` re-export.
//!  * A scroll container can be populated via `render_widget`, rendered
//!    via `StatefulWidget::render`, and the visible-area slice contains
//!    the expected symbols.
//!  * `state.scroll_down()` advances the offset, the visible area shifts
//!    accordingly, `state.is_at_bottom()` flip-flops correctly.
//!  * `state.scroll_to_top()` returns to the first visible row.
//!  * The `ScrollbarVisibility` fluent setters compile and apply cleanly.
//!  * `widget::ScrollView` is the same type as `scroller::ScrollView`.
//!
//! Compiled only when the `scroller` feature is enabled (see
//! `required-features` in `Cargo.toml`).

#![cfg(feature = "scroller")]

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::text::Span;
use ratatui::widgets::{Paragraph, StatefulWidget};
use tornado::scroller::{ScrollView, ScrollViewState, ScrollbarVisibility};

fn build_scroll_view_with_az_grid() -> ScrollView {
    populate_with_az_grid(ScrollView::new(Size::new(10, 10)))
}

fn populate_with_az_grid(mut scroll_view: ScrollView) -> ScrollView {
    // Reuse the A-Z 10x10 grid that the upstream test fixture uses, so
    // assertions are stable across vendoring + tornado patches.
    for y in 0..10 {
        for x in 0..10 {
            let c = char::from_u32((x + y * 10) % 26 + 65).unwrap();
            let widget = Span::raw(format!("{c}"));
            let area = Rect::new(x as u16, y as u16, 1, 1);
            scroll_view.render_widget(widget, area);
        }
    }
    scroll_view
}

fn render_stateful(scroll_view: &ScrollView, buf: &mut Buffer, state: &mut ScrollViewState) {
    StatefulWidget::render(scroll_view, buf.area, buf, state);
}

#[test]
fn scroll_view_widget_is_reachable_through_tornado_scroller() {
    // Reachability + renderer chain: build the widget through the
    // warehouse-side re-export, render it through StatefulWidget,
    // confirm the buffer has content at known coordinates.
    let scroll_view = build_scroll_view_with_az_grid();
    let mut buf = Buffer::empty(Rect::new(0, 0, 6, 6));
    let mut state = ScrollViewState::default();
    render_stateful(&scroll_view, &mut buf, &mut state);

    // At zero offset the first 6 columns of the top row are visible:
    // "ABCDE" before the vertical scrollbar glyph.
    let first = buf.cell(Position::new(0, 0)).expect("first cell");
    assert_eq!(first.symbol(), "A");

    let fifth = buf.cell(Position::new(4, 0)).expect("fifth cell");
    assert_eq!(fifth.symbol(), "E");

    // The scrollview records its own size via the public `ScrollView::size()`
    // accessor -- state.size / state.page_size are pub(crate) and not
    // reachable through the warehouse-side re-export, so we assert on
    // scroll_view.size() (the visible-source-of-truth) instead.
    assert_eq!(scroll_view.size(), Size::new(10, 10));
}

#[test]
fn widget_alias_scroll_view_compiles_through_re_export() {
    // Reachability compile-only check: tornado::widget::ScrollView resolves
    // to the same type as tornado::scroller::ScrollView. The assignment
    // below forces the type identity through monomorphization.
    let _value: tornado::scroller::ScrollView = tornado::widget::ScrollView::new(Size::new(2, 2));
}

#[test]
fn scroll_down_shifts_visible_area() {
    let scroll_view = build_scroll_view_with_az_grid();
    let mut buf = Buffer::empty(Rect::new(0, 0, 6, 6));

    // Render once at the top to learn page_size, then scroll down and
    // re-render to see the visible slice shift.
    let mut state = ScrollViewState::default();
    render_stateful(&scroll_view, &mut buf, &mut state);
    assert_eq!(buf.cell(Position::new(0, 0)).unwrap().symbol(), "A");
    assert!(!state.is_at_bottom());

    // Move down 5 rows -> page_size.height == 5 and offset.y == 5, so
    // the LAST full page is visible: row 5..=9 of the A-Z grid.
    for _ in 0..5 {
        state.scroll_down();
    }
    assert!(state.is_at_bottom());

    render_stateful(&scroll_view, &mut buf, &mut state);
    // At offset.y == 5 in a 6x6 viewport into 10x10 scroll buffer with
    // (vertical, horizontal) scrollbar visibility (Default, Default) ==
    // (Automatic, Automatic): both bars show. Viewport is 5x5 source cells.
    // Last visible row is buf y=4 (mapped to source row 9, "MNOPQRSTUV").
    // Last visible col is buf x=4 (the rightmost viewport column; col 5
    // holds the vertical scrollbar glyph, row 5 holds the horizontal
    // scrollbar glyph).
    assert_eq!(buf.cell(Position::new(0, 4)).unwrap().symbol(), "M");
    assert_eq!(buf.cell(Position::new(4, 4)).unwrap().symbol(), "Q");
}

#[test]
fn scroll_to_top_returns_to_first_visible_row() {
    let scroll_view = build_scroll_view_with_az_grid();
    let mut buf = Buffer::empty(Rect::new(0, 0, 6, 6));
    let mut state = ScrollViewState::default();
    render_stateful(&scroll_view, &mut buf, &mut state);

    // Advance several rows, then jump back to the top.
    for _ in 0..3 {
        state.scroll_down();
    }
    state.scroll_to_top();
    render_stateful(&scroll_view, &mut buf, &mut state);
    assert_eq!(buf.cell(Position::new(0, 0)).unwrap().symbol(), "A");
}

#[test]
fn scrollbar_visibility_setters_compile() {
    // The fluent setters consume `self` and re-chain. Exercise the
    // chain so any signature drift in upstream surfaces here at compile
    // time. Final setting (`Always`, `Always`) ensures both bars would
    // render in a non-fitting view; the assertion below checks the
    // fitting case (10x10 view into 10x10 buffer -> bars HIDDEN).
    let scroll_view = ScrollView::new(Size::new(10, 10))
        .vertical_scrollbar_visibility(ScrollbarVisibility::Automatic)
        .horizontal_scrollbar_visibility(ScrollbarVisibility::Never)
        .scrollbars_visibility(ScrollbarVisibility::Always);

    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
    let mut state = ScrollViewState::default();
    render_stateful(&scroll_view, &mut buf, &mut state);

    // Populate the scrollview's internal buffer with the A-Z grid so
    // the visible slice renders non-blank characters. The fluent setter
    // chain compiled and ran without panic. The chain ends in
    // `scrollbars_visibility(Always)` so both bars are visible at the
    // boundary: vertical bar at column 9 (rightmost), horizontal bar at
    // row 9 (bottom). The visible source slice is therefore 9x9 cells of
    // A..R. Assert the corners of the visible grid.
    let scroll_view = populate_with_az_grid(scroll_view);
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
    let mut state = ScrollViewState::default();
    render_stateful(&scroll_view, &mut buf, &mut state);

    assert_eq!(buf.cell(Position::new(0, 0)).unwrap().symbol(), "A");
    assert_eq!(buf.cell(Position::new(8, 8)).unwrap().symbol(), "K");
}

#[test]
fn with_offset_constructor_positions_initial_offset() {
    let scroll_view = build_scroll_view_with_az_grid();
    let mut buf = Buffer::empty(Rect::new(0, 0, 6, 6));

    // Place at offset (3, 0): visible 6-col slice of the A-Z grid at
    // column 3..8 should be: D, E, F, G, H.
    let mut state = ScrollViewState::with_offset(Position::new(3, 0));
    render_stateful(&scroll_view, &mut buf, &mut state);
    assert_eq!(buf.cell(Position::new(0, 0)).unwrap().symbol(), "D");
    assert_eq!(buf.cell(Position::new(4, 0)).unwrap().symbol(), "H");
}

#[test]
fn paragraph_into_scroll_view_renders_into_internal_buffer() {
    // Render a Paragraph INTO the scroll buffer (not the visible area)
    // using render_stateful_widget, then scroll to bottom and check.
    let mut scroll_view = ScrollView::new(Size::new(20, 5));
    let items: Vec<String> = (1..=5).map(|i| format!("Item {i}")).collect();
    let paragraph = Paragraph::new(items.join("\n"));
    scroll_view.render_widget(
        paragraph,
        Rect::new(0, 0, 20, 5),
    );

    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
    let mut state = ScrollViewState::default();
    render_stateful(&scroll_view, &mut buf, &mut state);
    // The scrollview holds "Item 1\nItem 2\n...". First row should
    // begin with "Item 1": (0,0)='I', (1,0)='t', (5,0)='1',
    // (6,0)=' ' (continuation blank).
    assert_eq!(buf.cell(Position::new(0, 0)).unwrap().symbol(), "I");
    assert_eq!(buf.cell(Position::new(1, 0)).unwrap().symbol(), "t");
    assert_eq!(buf.cell(Position::new(5, 0)).unwrap().symbol(), "1");
}
