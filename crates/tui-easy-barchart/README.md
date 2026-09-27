# `tui-easy-barchart`

Round-12 of the [awesome-ratatui adoption series](../../README.md) — vendored
[`ratatui::widgets::BarChart`](https://docs.rs/ratatui/latest/ratatui/widgets/struct.BarChart.html)
from `ratatui` v0.30.0.

## Provenance

Faithful mirror of
[`ratatui@v0.30.0/src/widgets/barchart.rs`](https://github.com/ratatui/ratatui/blob/ratatui-v0.30.0/src/widgets/barchart.rs).
See [`LICENSE-MIT`](./LICENSE-MIT), [`LICENSE-APACHE`](./LICENSE-APACHE), and
[`NOTICE`](./NOTICE) for upstream attribution.

## Public surface

- `BarChart<'a>` — the root widget builder (data, block, max, bar_width, bar_gap,
  group_gap, direction, value_style, label_style, style, bar_set).
- `<BarGroup<'a>>` — cluster of bars with optional shared label.
- `Bar<'a>` — single metric item (value, label, text_value, custom styles).
- `BarSet` — the glyph gradient (Full / Half / AsciiBlocks variants).
- `Widget for BarChart` — single stateless render.
- **No `StatefulWidget`**, **no `BarChartState`** — round-12 is intentionally
  stateless (round-6 Tabs pattern divergent from round-11 List's
  StatefulWidget-vendoring precedent).

## Concretely STATELESS — different from round-11

The vendored `BarChart` does NOT have a `StatefulWidget` counterpart, so the
round-11 E0034 carve-out pattern does NOT apply. The `crates/tui-easy-list`
`§3` module doc is the canonical StatefulWidget-vendoring precedent;
round-12 deliberately diverges.

## Differentiation tests

The vendored crate ships two differentiator integration tests that close
specific hazard categories:

- `bar_chart_direction_arms_yield_distinct_cell_anchors` — closes the
  Direction arm silent no-op hazard category. `Direction::Horizontal`
  vs `Direction::Vertical` paint in entirely distinct cell regions.
- `bar_chart_max_scales_tallest_bar_to_full_height` — closes the
  max-value scaling confusion (max scales to itself, not to column
  width).

## Tests

- 6 inline `#[cfg(test)] mod tests` (builder chain, data stored, block
  wraps, bar_width/gap preserved, value/label/style applied,
  BarSet variants distinguishable).
- 6 integration tests in `tests/barchart_integration.rs` (TestBackend
  cell-by-cell, Horizontal vs Vertical cell-position assertion, max-value
  scaling, multi-bar group separation, label rendering, no-direction
  fallback).

## License

Dual-licensed under MIT OR Apache-2.0 — same terms as upstream.
