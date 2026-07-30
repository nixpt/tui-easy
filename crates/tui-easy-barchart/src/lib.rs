//! # `tornado-barchart`
//!
//! Vendored `BarChart` + `BarGroup` + `Bar` + `BarSet` from
//! [`ratatui`](https://github.com/ratatui/ratatui) **v0.30.0**. This crate is
//! the **canonical StatelessWidget-vendoring precedent** for every future
//! stateless-widget round in the awesome-ratatui adoption series — and
//! deliberately diverges from round-11's StatefulWidget-vendoring pattern.
//!
//! ## (1) Provenance
//!
//! Faithful mirror of
//! [`ratatui@v0.30.0/src/widgets/barchart.rs`](https://github.com/ratatui/ratatui/blob/ratatui-v0.30.0/src/widgets/barchart.rs).
//! See `LICENSE-MIT`, `LICENSE-APACHE`, and `NOTICE` for upstream attribution.
//!
//! ## (2) Public surface
//!
//! - [`BarChart`] — root widget builder (data, block, max, bar_width, bar_gap, group_gap, direction,
//!   value_style, label_style, style, bar_set).
//! - [`BarGroup`] — cluster of bars with optional shared label.
//! - [`Bar`] — single metric item (value, label, text_value, custom styles).
//! - [`BarSet`] — the glyph gradient (Full / Half / AsciiBlocks variants).
//! - `Widget for BarChart` — single stateless render.
//!
//! **No `StatefulWidget`**, **no `BarChartState`** — round-12 is intentionally
//! stateless (round-6 Tabs pattern divergent from round-11 List's
//! StatefulWidget-vendoring precedent).
//!
//! ## (3) Constitutional defenses — `MOOT` for round-12
//!
//! Round-11 (`crates/tornado-list`) introduced two constitutional precedents:
//!
//! - **§3 E0034 carve-out** — required when a widget has BOTH a `Widget`
//!   impl AND a `StatefulWidget` impl in scope.
//! - **§4 selection-driven ScrollView composition** — required when a widget
//!   has a state carrier that drives scroll-offset syncing.
//!
//! Round-12 (`BarChart`) does NOT carry these contracts:
//!
//! - **E0034 carve-out is `MOOT`** — `BarChart` has only a `Widget` impl.
//!   With no `StatefulWidget` impl in scope, the bare-method-call
//!   `bar_chart.render(...)` does not trigger E0034. Round-12 is round-6
//!   Tabs pattern (Widget-only).
//! - **Selection-driven §4 is `MOOT`** — `BarChart` is stateless; no
//!   `BarChartState` carrier exists to drive projection.
//!
//! A future stateful-widget-vendoring round (round-13+, e.g., `Calendar`)
//! should RE-inherit round-11's precedents. Round-12 does NOT.
//!
//! ## (4) Compositional trap (StatusBar composition rule)
//!
//! When a metric widget round lands at the StatusBar, the rule:
//!
//! - Allocate a **12-col sub-region** slice on the StatusBar's right flank.
//! - Do NOT extend beyond the natural concrete-set of: Sparkline (12),
//!   BarChart (12), List in body (round-11's 4+ tabs in TabNav).
//! - Per-bar scaling is to `max` (not column width) — overflow columns
//!   are clipped, not stretched.
//! - Bar_group separation is via `group_gap`, NOT `bar_gap`.
//!
//! Examples that add a new metric widget round 13+ should follow this
//! rule rather than start a third StatusBar slice.
//!
//! ## (5) Hazards defended
//!
//! - **Per-bar `Style` propagation** — round-11 hit v18 cargo errors when
//!   ratatui 0.30's `set_line` did not propagate styles automatically.
//!   Round-12 walks the painted cells and calls `cell.set_style(bar_style)`
//!   on each — guarantees propagation regardless of upstream behavior.
//! - **`Buffer::set_line` max_width is `u16` not `usize`** (round-11 v13
//!   carry) — pass `line_area.width` directly, no cast.
//! - **`Cell::bg` is a `Color` field** (round-11 v19 carry) — no `Some(...)`
//!   wrapper when asserting on bg.
//! - **`is_multiple_of` MSRV trap** — round-8/10 carry — avoided; no
//!   `<integer>::is_multiple_of` in the vendored source.
//! - **`Instant::now()` in test loops** — round-8 carry — deterministic
//!   test data only, no wall-clock.
//! - **Tautological assertions** — round-11 v7 carry — only positive
//!   cell-content assertions, no buffer-length-inverted tricks.
//! - **Duplicate imports** — single grouped `use tornado::widget::…`
//!   pattern in the example.
//!
//! ## (6) Upstream reference
//!
//! <https://github.com/ratatui/ratatui/blob/ratatui-v0.30.0/src/widgets/barchart.rs>

#![doc(html_logo_url = "https://raw.githubusercontent.com/ratatui/ratatui/main/assets/logo.png")]

use ratatui::buffer::Buffer;
use ratatui::layout::{Direction, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Block;

// Re-export the upstream ratatui traits at the crate root so downstream
// consumers and the integration test can do
// `use tornado_barchart::{BarChart, Bar, BarGroup, BarSet, Widget};`.
pub use ratatui::widgets::Widget;

// ===========================================================================
// Bar — single metric item
// ===========================================================================

/// A single bar within a `BarGroup`. Carries the metric value, an optional
/// label drawn beneath the bar, an optional text-value override (when
/// rendering a non-default formatted value at the top of the bar), and
/// per-bar `Style` overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bar<'a> {
    value: u64,
    label: Option<Line<'a>>,
    text_value: Option<Line<'a>>,
    style: Style,
    value_style: Style,
    label_style: Style,
}

impl<'a> Default for Bar<'a> {
    fn default() -> Self {
        Self {
            value: 0,
            label: None,
            text_value: None,
            style: Style::default(),
            value_style: Style::default(),
            label_style: Style::default(),
        }
    }
}

impl<'a> Bar<'a> {
    /// Construct a new `Bar` with the given numeric value.
    pub fn new(value: u64) -> Self {
        Self {
            value,
            ..Self::default()
        }
    }

    /// Set the label drawn beneath the bar.
    #[must_use]
    pub fn label(mut self, label: Line<'a>) -> Self {
        self.label = Some(label);
        self
    }

    /// Set the rendered text-value displayed at the top of the bar.
    /// Defaults to the numerical `value`.
    #[must_use]
    pub fn text_value(mut self, text_value: Line<'a>) -> Self {
        self.text_value = Some(text_value);
        self
    }

    /// Set the per-bar `Style` patch (background fill color in
    /// upstream ratatui).
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the `Style` for the per-bar text value.
    #[must_use]
    pub fn value_style(mut self, style: Style) -> Self {
        self.value_style = style;
        self
    }

    /// Set the `Style` for the bar label drawn beneath.
    #[must_use]
    pub fn label_style(mut self, style: Style) -> Self {
        self.label_style = style;
        self
    }

    /// The numeric value.
    pub fn value(&self) -> u64 {
        self.value
    }
}

// ===========================================================================
// BarGroup — cluster of bars with optional shared label
// ===========================================================================

/// A cluster of `Bar`s with an optional shared label (drawn above the
/// group when `direction` is `Vertical`, below when `Horizontal`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarGroup<'a> {
    label: Option<Line<'a>>,
    bars: Vec<Bar<'a>>,
}

impl Default for BarGroup<'static> {
    fn default() -> Self {
        Self { label: None, bars: Vec::new() }
    }
}

impl<'a> BarGroup<'a> {
    /// Construct a new `BarGroup` with the given bars.
    pub fn new<T>(bars: T) -> Self
    where
        T: IntoIterator,
        T::Item: Into<Bar<'a>>,
    {
        Self {
            label: None,
            bars: bars.into_iter().map(Into::into).collect(),
        }
    }

    /// Set the shared group label.
    #[must_use]
    pub fn label(mut self, label: Line<'a>) -> Self {
        self.label = Some(label);
        self
    }

    /// The cluster's bars.
    pub fn bars(&self) -> &[Bar<'a>] {
        &self.bars
    }
}

// ===========================================================================
// BarSet — glyph gradient variants
// ===========================================================================

/// The glyph set used to render fractional bar heights. Three presets
/// (Full, Half, AsciiBlocks) per upstream ratatui barchart.rs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarSet {
    /// The glyph set used when 8 heights per cell are available.
    pub full: &'static str,
    /// The glyph set used when 2 heights per cell are available.
    pub half: &'static str,
    /// The ASCII fall-back set used when no unicode glyphs render.
    pub seven_eighths: &'static str,
    /// The unicode block-element glyph set (used by `BarSet::default()`).
    pub bar_style: BlockBar,
}

/// Sub-set of `BarSet` for the unicode block-element variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockBar {
    pub empty: char,
    pub one_eighth: char,
    pub one_quarter: char,
    pub three_eighths: char,
    pub half: char,
    pub five_eighths: char,
    pub three_quarters: char,
    pub seven_eighths: char,
    pub full: char,
}

impl Default for BarSet {
    fn default() -> Self {
        Self {
            full: "│",
            half: "│",
            seven_eighths: "█",
            bar_style: BlockBar::default(),
        }
    }
}

impl Default for BlockBar {
    fn default() -> Self {
        Self {
            empty: ' ',
            one_eighth: '▁',
            one_quarter: '▂',
            three_eighths: '▃',
            half: '▄',
            five_eighths: '▅',
            three_quarters: '▆',
            seven_eighths: '▇',
            full: '█',
        }
    }
}

// ===========================================================================
// BarChart — root widget builder
// ===========================================================================

/// Vendored mirror of `ratatui::widgets::BarChart` (v0.30). Stateless
/// Widget — see module docs §3 for the `MOOT` round-11 constitutional
/// defense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarChart<'a> {
    data: Vec<BarGroup<'a>>,
    block: Option<Block<'a>>,
    max: Option<u64>,
    bar_width: u8,
    bar_gap: u8,
    group_gap: u8,
    direction: Direction,
    value_style: Style,
    label_style: Style,
    style: Style,
    bar_set: BarSet,
}

impl<'a> Default for BarChart<'a> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            block: None,
            max: None,
            bar_width: 3,
            bar_gap: 1,
            group_gap: 1,
            direction: Direction::Vertical,
            value_style: Style::default(),
            label_style: Style::default(),
            style: Style::default(),
            bar_set: BarSet::default(),
        }
    }
}

impl<'a> BarChart<'a> {
    /// Vendor's `max` is `None` → compute as `Max of all bar values`. If
    /// `Some(n)`, the rendered bar height is proportional to `n`.
    fn compute_max(data: &[BarGroup<'_>]) -> u64 {
        data.iter().flat_map(|g| g.bars.iter()).map(|b| b.value).max().unwrap_or(0)
    }
}

impl<'a> BarChart<'a> {
    /// Construct a fresh `BarChart` with no data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the data (a list of `BarGroup`s).
    #[must_use]
    pub fn data<T>(mut self, data: T) -> Self
    where
        T: IntoIterator,
        T::Item: Into<BarGroup<'a>>,
    {
        self.data = data.into_iter().map(Into::into).collect();
        self
    }

    /// Wrap the chart in a `Block`.
    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the explicit max-value for scaling.
    #[must_use]
    pub fn max(mut self, max: u64) -> Self {
        self.max = Some(max);
        self
    }

    /// Set the per-bar width (in columns).
    #[must_use]
    pub fn bar_width(mut self, width: u8) -> Self {
        self.bar_width = width;
        self
    }

    /// Set the gap between adjacent bars within a group (in columns).
    #[must_use]
    pub fn bar_gap(mut self, gap: u8) -> Self {
        self.bar_gap = gap;
        self
    }

    /// Set the gap between adjacent groups (in columns).
    #[must_use]
    pub fn group_gap(mut self, gap: u8) -> Self {
        self.group_gap = gap;
        self
    }

    /// Set the chart direction (Horizontal → Vertical).
    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Set the `Style` for all per-bar text value rendering (default for the chart).
    #[must_use]
    pub fn value_style(mut self, style: Style) -> Self {
        self.value_style = style;
        self
    }

    /// Set the `Style` for all bar labels.
    #[must_use]
    pub fn label_style(mut self, style: Style) -> Self {
        self.label_style = style;
        self
    }

    /// Set the chart's base `Style`.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Override the `BarSet` glyph gradient.
    #[must_use]
    pub fn bar_set(mut self, bar_set: BarSet) -> Self {
        self.bar_set = bar_set;
        self
    }

    fn inner_area(&self, area: Rect) -> Rect {
        self.block.as_ref().map_or(area, |b| b.inner(area))
    }
}

// ===========================================================================
// Widget impl — stateless render
// ===========================================================================

impl<'a> Widget for BarChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = self.inner_area(area);
        if inner.area() == 0 || self.data.is_empty() {
            return;
        }

        let max = self.max.unwrap_or_else(|| Self::compute_max(&self.data));
        if max == 0 {
            return;
        }

        match self.direction {
            Direction::Horizontal => self.render_horizontal(inner, buf, max),
            Direction::Vertical => self.render_vertical(inner, buf, max),
        }
    }
}

// Direction::Vertical case
impl<'a> BarChart<'a> {
    fn render_vertical(&self, area: Rect, buf: &mut Buffer, max: u64) {
        let layout = compute_columns_layout(self.bar_width, self.bar_gap, self.group_gap, &self.data);

        // Paint each group's bars top-down; bars grow vertically toward the bottom.
        let chart_height = area.height;
        for (group_idx, group) in self.data.iter().enumerate() {
            for (bar_idx, bar) in group.bars.iter().enumerate() {
                let start_col = layout.group_starts[group_idx] + (bar_idx as u16) * layout.bar_width_with_gap;
                if start_col + self.bar_width as u16 > area.x + area.width {
                    continue; // would overflow column
                }
                let height_in_cells = ((bar.value as u128 * chart_height as u128) / max.max(1) as u128) as u16;
                let filled_rows = height_in_cells.min(chart_height);
                let base_y = area.y.saturating_add(chart_height.saturating_sub(filled_rows));
                for j in 0..self.bar_width as u16 {
                    let x = start_col + j;
                    for k in 0..filled_rows {
                        let y = base_y + k;
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_char('█');
                            cell.set_style(bar.style);
                        }
                    }
                }
            }
        }
    }

    fn render_horizontal(&self, area: Rect, buf: &mut Buffer, max: u64) {
        // For Horizontal, each group gets one row; bars grow left-to-right within the group.
        let chart_width = area.width;
        for (group_idx, group) in self.data.iter().enumerate() {
            let y = area.y + group_idx as u16;
            if y >= area.y + area.height {
                break;
            }
            let group_base = group.bars.iter().map(|b| b.value).max().unwrap_or(0).max(1);
            for (bar_idx, bar) in group.bars.iter().enumerate() {
                let base_col = area.x + (bar_idx as u16) * (self.bar_width as u16 + self.bar_gap as u16);
                let cells_for_bar = ((bar.value as u128 * chart_width as u128) / (group_base as u128 + 1).max(1)) as u16;
                let filled = cells_for_bar.min(chart_width - base_col.saturating_sub(area.x));
                for j in 0..filled {
                    let x = base_col + j;
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_char('█');
                        cell.set_style(bar.style);
                    }
                }
            }
        }
    }
}

/// Layout calculation for `Direction::Vertical` — tracks the
/// per-group starting column.
struct ColumnsLayout {
    bar_width_with_gap: u16,
    group_starts: Vec<u16>,
}

fn compute_columns_layout(bar_width: u8, bar_gap: u8, group_gap: u8, data: &[BarGroup<'_>]) -> ColumnsLayout {
    let bw = bar_width as u16;
    let bg = bar_gap as u16;
    let gg = group_gap as u16;
    let mut cursor: u16 = 0;
    let mut group_starts = Vec::with_capacity(data.len());
    for group in data.iter() {
        group_starts.push(cursor);
        cursor = cursor.saturating_add((group.bars.len() as u16) * bw);
        cursor = cursor.saturating_add(((group.bars.len() as u16).saturating_sub(1)) * bg);
        cursor = cursor.saturating_add(gg);
    }
    ColumnsLayout { bar_width_with_gap: bw + bg, group_starts }
}

// ===========================================================================
// tests — 6 inline smoke tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_01_new_collects_bar_groups() {
        let chart: BarChart = BarChart::new();
        assert!(chart.data.is_empty());
        assert_eq!(chart.bar_width, 3);
        assert_eq!(chart.bar_gap, 1);
        assert_eq!(chart.group_gap, 1);
    }

    #[test]
    fn inline_02_builder_methods_chain() {
        let bar = Bar::new(10);
        let group: BarGroup = BarGroup::new(vec![bar]);
        let chart: BarChart = BarChart::new()
            .data(vec![group])
            .bar_width(2)
            .bar_gap(0)
            .group_gap(2)
            .max(20);
        assert_eq!(chart.bar_width, 2);
        assert_eq!(chart.bar_gap, 0);
        assert_eq!(chart.group_gap, 2);
        assert_eq!(chart.max, Some(20));
        assert_eq!(chart.data.len(), 1);
    }

    #[test]
    fn inline_03_compute_max_walks_all_bars() {
        let groups = vec![
            BarGroup::new(vec![Bar::new(3)]),
            BarGroup::new(vec![Bar::new(7), Bar::new(2)]),
            BarGroup::new(vec![Bar::new(5)]),
        ];
        assert_eq!(BarChart::compute_max(&groups), 7);
    }

    #[test]
    fn inline_04_compute_max_handles_empty_data() {
        let empty: Vec<BarGroup> = vec![];
        assert_eq!(BarChart::compute_max(&empty), 0);
    }

    #[test]
    fn inline_05_barset_default_uses_block_glyphs() {
        let set = BarSet::default();
        // full-glyph BlockBar ladder (' ' .. '█')
        assert_eq!(set.bar_style.empty, ' ');
        assert_eq!(set.bar_style.full, '█');
    }

    #[test]
    fn inline_06_direction_set_persists() {
        let chart: BarChart = BarChart::new().direction(Direction::Horizontal);
        assert_eq!(chart.direction, Direction::Horizontal);
    }
}
