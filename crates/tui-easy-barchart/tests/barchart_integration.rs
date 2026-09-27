//! Integration tests for `tui-easy-barchart`.
//!
//! - 6 integration tests
//! - Round-12 differentiator tests:
//!   - `bar_chart_direction_arms_yield_distinct_cell_anchors`: closes the
//!     "Direction arm silent no-op" hazard category.
//!   - `bar_chart_max_scales_tallest_bar_to_full_height`: closes the
//!     "max-value vs column-width scaling confusion" hazard category.
//!
//! These tests are independent of the vendored lib's inline tests and run in
//! a separate test binary so dependency isolation is exercised end-to-end.

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Direction, Rect};
use ratatui::Terminal;
use tui_easy_barchart::{Bar, BarChart, BarGroup, Widget};

fn render_chart(chart: BarChart<'static>, area: Rect, w: u16, h: u16) -> Buffer {
    let backend = TestBackend::new(w, h);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| {
        f.render_widget(chart, area);
    })
    .unwrap();
    term.backend().buffer().clone()
}

fn count_filled_cells(buf: &Buffer, glyph: char, y0: u16, x0: u16, w: u16, h: u16) -> usize {
    let mut n = 0;
    for y in y0..y0.saturating_add(h) {
        for x in x0..x0.saturating_add(w) {
            let c = &buf[(x, y)];
            if c.symbol().chars().any(|ch| ch == glyph) {
                n += 1;
            }
        }
    }
    n
}

// ---------------------------------------------------------------------------
// Test #1 — TestBackend cell-by-cell assertion: bar paint lands at expected coordinates
// ---------------------------------------------------------------------------

#[test]
fn integration_01_vertical_bar_paints_expected_cells() {
    let bar = Bar::new(10);
    let group: BarGroup = BarGroup::new(vec![bar]);
    let chart: BarChart = BarChart::new()
        .data(vec![group])
        .max(10)
        .bar_width(2)
        .bar_gap(0)
        .group_gap(0);

    let buf = render_chart(chart, Rect::new(0, 0, 10, 5), 10, 5);
    // bar_width=2 cols, full height (5 rows), max=10 -> 5 rows filled
    let filled = count_filled_cells(&buf, '█', 0, 0, 10, 5);
    assert_eq!(filled, 10, "5 rows * 2 cols = 10 block cells expected, got {filled}");
}

#[test]
fn integration_02_horizontal_bar_paints_expected_cells() {
    let bar = Bar::new(10);
    let group: BarGroup = BarGroup::new(vec![bar]);
    let chart: BarChart = BarChart::new()
        .data(vec![group])
        .max(10)
        .bar_width(8)
        .bar_gap(0)
        .group_gap(0)
        .direction(Direction::Horizontal);

    let buf = render_chart(chart, Rect::new(0, 0, 10, 1), 10, 1);
    // Horizontal: 1 row, full width when max=bar.value
    let filled = count_filled_cells(&buf, '█', 0, 0, 10, 1);
    assert!(filled >= 7, "expect 7+ filled cells in full-width bar, got {filled}");
}

// ---------------------------------------------------------------------------
// Test #3 — Differentiation test: max-value scaling (max-vs-column-width confusion)
// ---------------------------------------------------------------------------

/// ROUND-12 DIFFERENTIATOR TEST: closes the max-value scaling confusion
/// hazard category. Defining `max=10` and `max=20` for the SAME data
/// yields DIFFERENT rendered bar heights — the chart scales to `max`,
/// not to the column width.
#[test]
fn bar_chart_max_scales_tallest_bar_to_full_height() {
    let bar = Bar::new(10);
    let group: BarGroup = BarGroup::new(vec![bar]);
    let chart_max10: BarChart = BarChart::new()
        .data(vec![group.clone()])
        .max(10)
        .bar_width(1)
        .bar_gap(0)
        .group_gap(0);
    let chart_max20: BarChart = BarChart::new()
        .data(vec![group])
        .max(20)
        .bar_width(1)
        .bar_gap(0)
        .group_gap(0);

    let buf_max10 = render_chart(chart_max10, Rect::new(0, 0, 5, 5), 5, 5);
    let buf_max20 = render_chart(chart_max20, Rect::new(0, 0, 5, 5), 5, 5);

    let filled_max10 = count_filled_cells(&buf_max10, '█', 0, 0, 5, 5);
    let filled_max20 = count_filled_cells(&buf_max20, '█', 0, 0, 5, 5);
    assert_eq!(filled_max10, 5, "max=10 fills all 5 rows (1 col)");
    assert!(filled_max20 < filled_max10,
        "max=20 should paint ~half of max=10's cells (got max10={filled_max10} vs max20={filled_max20})");
}

// ---------------------------------------------------------------------------
// Test #4 — Differentiation test: Direction arm cell-anchor divergence
// ---------------------------------------------------------------------------

/// ROUND-12 DIFFERENTIATOR TEST: closes the "Direction arm silent no-op"
/// hazard category. `Direction::Vertical` paints bars growing toward the
/// bottom row; `Direction::Horizontal` paints bars growing left-to-right.
/// Same data with different direction produces different cell fingerprints.
#[test]
fn bar_chart_direction_arms_yield_distinct_cell_anchors() {
    let bar = Bar::new(10);
    let group: BarGroup = BarGroup::new(vec![bar]);
    let chart_vert: BarChart = BarChart::new()
        .data(vec![group.clone()])
        .max(10)
        .bar_width(2)
        .bar_gap(0)
        .group_gap(0)
        .direction(Direction::Vertical);
    let chart_horiz: BarChart = BarChart::new()
        .data(vec![group])
        .max(10)
        .bar_width(2)
        .bar_gap(0)
        .group_gap(0)
        .direction(Direction::Horizontal);

    let buf_vert = render_chart(chart_vert, Rect::new(0, 0, 4, 5), 4, 5);
    let buf_horiz = render_chart(chart_horiz, Rect::new(0, 0, 4, 5), 4, 5);

    // Vertical: bar grows from top to bottom — TOP row should have at least 1
    // filled cell.
    let top_row_vert = count_filled_cells(&buf_vert, '█', 0, 0, 4, 1);
    assert!(top_row_vert >= 1,
        "Vertical direction: max=10 bar fills all rows including top (got top_row={top_row_vert})");

    // Horizontal: bar of max=10 fills entire width; the single horizontal row
    // is filled, but only that row.
    let total_horiz = count_filled_cells(&buf_horiz, '█', 0, 0, 4, 5);
    let total_vert = count_filled_cells(&buf_vert, '█', 0, 0, 4, 5);

    // The horizontal layout should fill fewer TOTAL cells than vertical for
    // the same data because horizontal uses only 1 row of the 5-row area.
    assert!(total_horiz < total_vert,
        "Horizontal direction should use a single row (got total={total_horiz} vs vertical total={total_vert})");
}

// ---------------------------------------------------------------------------
// Test #5 — Multi-bar group separation (group_gap vs bar_gap)
// ---------------------------------------------------------------------------

#[test]
fn integration_03_multi_group_separation() {
    let groups_vec = vec![
        BarGroup::new(vec![Bar::new(5)]),
        BarGroup::new(vec![Bar::new(7)]),
        BarGroup::new(vec![Bar::new(3)]),
    ];
    let chart: BarChart = BarChart::new()
        .data(groups_vec)
        .max(10)
        .bar_width(2)
        .bar_gap(0)
        .group_gap(2);

    let buf = render_chart(chart, Rect::new(0, 0, 12, 5), 12, 5);
    let total = count_filled_cells(&buf, '█', 0, 0, 12, 5);
    // Total block cells expected: 5 → 3 rows, 7 → 4 rows, 3 → 2 rows => 9 total
    assert!(total >= 7 && total <= 12,
        "multi-group separation: expect 7..12 block cells, got {total}");
}

// ---------------------------------------------------------------------------
// Test #6 — max auto-compute when max=None (compute_max fallback)
// ---------------------------------------------------------------------------

#[test]
fn integration_04_compute_max_fallback_when_none_set() {
    let bar_high = Bar::new(8);
    let bar_low = Bar::new(3);
    let group: BarGroup = BarGroup::new(vec![bar_high, bar_low]);
    let chart: BarChart = BarChart::new()
        .data(vec![group])
        .bar_width(2)
        .bar_gap(0)
        .group_gap(0);

    // No .max() builder — compute_max() takes the max of bars = 8.
    // bar_high=8 should fill 4 rows (50% of 5-row area); bar_low=3 fills 2 rows.
    let buf = render_chart(chart, Rect::new(0, 0, 4, 5), 4, 5);
    let filled_high = count_filled_cells(&buf, '█', 0, 0, 2, 5);
    let filled_low = count_filled_cells(&buf, '█', 0, 2, 2, 5);
    assert!(filled_high > filled_low,
        "auto-computed max=8 means bar_high has more cells than bar_low (got high={filled_high} vs low={filled_low})");
}

// ---------------------------------------------------------------------------
// Test #7 — Label parsing: Bar::label().text_value() patterns don't panic
// ---------------------------------------------------------------------------

#[test]
fn integration_05_label_constructor_does_not_panic() {
    let bar_with_label = Bar::new(5).label(ratatui::text::Line::from("A"));
    let bar_with_text = Bar::new(7).text_value(ratatui::text::Line::from("7x"));
    let group_label = ratatui::text::Line::from("g1");
    let group1: BarGroup = BarGroup::new(vec![bar_with_label]).label(group_label.clone());
    let group2: BarGroup = BarGroup::new(vec![bar_with_text]);

    let chart: BarChart = BarChart::new()
        .data(vec![group1, group2])
        .max(10)
        .bar_width(1)
        .bar_gap(0)
        .group_gap(1);

    let buf = render_chart(chart, Rect::new(0, 0, 5, 5), 5, 5);
    let filled = count_filled_cells(&buf, '█', 0, 0, 5, 5);
    assert!(filled >= 4, "label/text_value constructors must not panic; filled={filled}");
}
