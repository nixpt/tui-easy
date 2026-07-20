# Changelog — tornado

All notable changes to the `tornado` umbrella crate and its sub-crates are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/) conventions.
Tornado is pre-1.0 — breaking changes are expected and noted with `[breaking]`.

---

## [0.2.99] — 2026-07-20

### Added

- **`#![doc = include_str!("../README.md")]`** — crate-level rustdoc now embeds the full README on docs.rs. Each module (`event`, `theme`, `terminal`, `widget`, `tab_log`) gained `//!` module-level doc comments with intra-doc links and concise summaries.
- **`CHANGELOG.md`** — initial changelog documenting the full project history from creation through upstream migration.
- **`crates/tornado/README.md`** — comprehensive documentation covering all 12 features, 10 usage patterns, architecture diagram, theme reference, and every example app.

### `tornado-showcase` example

A 5-tab visual demo exercising **all** tornado widgets in one TUI app:

| Tab | Widgets |
|---|---|
| Welcome | `BigText` (TORNADO) + `Link` (GitHub, Ratatui) |
| Sparkline | Live sine-wave `Sparkline` |
| List | 50-item scrollable `List` |
| Tree | Mock filesystem `Tree` with fold/expand |
| Popup | Toggleable `Popup` overlay over `TabLog` |

Includes 8 inline `TestBackend` smoke tests.

### Upstream crate migration (all vendored crates removed)

Completed the transition from vendored copies to upstream crates.io dependencies:

| Widget | Before (vendored) | After (upstream) | Commit |
|---|---|---|---|
| Tabs | `crates/tornado-tabs` | `ratatui::widgets::Tabs` as `TabNav` | `7e465b7` |
| Spinner | `crates/tornado-spinner` | `ratatui-cheese` v0.7 | `7e465b7` |
| Sparkline | `crates/tornado-sparkline` | `ratatui::widgets::Sparkline` | `fbf3251` |
| List | `crates/tornado-list` | `ratatui::widgets::{List, ListState}` | `cc23ef5` |
| Hyperlink | `crates/tornado-hyperlink` | `hyperrat` v0.1 | `fbf3251` |
| Scrollview | `crates/tornado-scrollview` | `tui-scrollview` v0.6 | `fbf3251` |
| Popup | `crates/tornado-popup` | `tui-popup` v0.7 | `fbf3251` |
| BigText | `crates/tornado-big-text` | `tui-big-text` v0.8 | `760762b` |
| Tree | `crates/tornado-tree-widget` | `tui-tree-widget` v0.24 | `fbf3251` |

**Empty feature gates** kept for backward compat: `tabs`, `sparkline`, `list`.
**In-house crates retained:** `tornado-styles` (ANSI bridge), `tornado-wrap` (word-wrap).

### Workspace cleanup

- Removed stale vendored-dependency comments from `Cargo.toml` features section (`e3530e2`).
- Cleaned up unused deps after migration.

### `multi-tab-log` example updates

- Migrated `Sparkline` from vendored `tornado-sparkline` API (`Sparkline::new(&[u64])`) to native `ratatui` API (`Sparkline::default().data(&[u64])`).
- Migrated `bar_set()` from old `SparklineBar` to `ratatui::symbols::bar::Set`.
- Updated `TabNav` import path.

### `scroll-log` example updates

- Migrated spinner and TabNav to upstream equivalents.

### `scroll-log` example — cosmetic polish (`fcc5127`)

- Render-sparkline indent cleanup, comment polish, smoke test consistency pass.

---

## [0.2.99] — 2026-07-20 (earlier)

### Added

- **`tornado-showcase`** — comprehensive visual demo example.
- **`tornado-tabs`** — vendored copy of `ratatui::widgets::Tabs` as `TabNav` (`a9f475a`), later migrated to empty gate in `7e465b7`.

### `TabLog` helper (`log_view` feature)

The `tornado::tab_log::TabLog` primitive extracted from `scroll-log` into the umbrella crate (`2b3d3e3`):
- Per-tab virtual-scroll buffer with OSC-8 hyperlink anchor registry.
- Sequential anchor-ID assignment.
- Pin-to-bottom tracking with `reset_to_bottom_if_pinned()`.
- `render_pair()` for disjoint-borrow-safe `StatefulWidget` rendering.

Gated behind `log_view = ["dep:tornado-styles", "dep:tui-scrollview", "dep:tornado-wrap"]`.

### `multi-tab-log` example

A 5-stream log viewer composing the `TabLog` helper under tab navigation (`0028e64`):

- 5 stream tabs (build, tests, deps, lint, release) with independent scroll state.
- Global `SpinnerState` driven by tick-elapsed time.
- `TabNav` with active-tab highlighting.
- Per-tab row seeding + live append pump.
- 6 inline `TestBackend` smoke tests.

### `scroll-log` example

A single-stream scroll-log viewer demonstrating the five borrowed sibling crates (`cc0759c`):

- Vendor shim imports for `tornado-hyperlink`, `tornado-spinner`, `tornado-scrollview`.
- Tabula log parser with OSC-8 hyperlink anchors.
- Scrollbar visibility policy per `tui-scrollview`.
- 3 inline `TestBackend` smoke tests.

### Vendor rounds 11–14

- `tornado-list` — `StatefulWidget` precedent from ratatui 0.30, adapted `Vec<ListItem>` → `Vec<Text>` with E0034 carve-out pattern.
- `tornado-popup` — adapted from `joshka/tui-popup` (MIT).
- `tornado-big-text` — adapted from `joshka/tui-big-text` (MIT/Apache-2.0).
- `tornado-tree-widget` — adapted from `EdJoPaTo/tui-rs-tree-widget` (MIT).

### `tornado-tabs` (vendored)

Vendored `ratatui-widgets::Tabs` as `crates/tornado-tabs` — the `TabNav` alias under the `tabs` feature (`a9f475a`). Later migrated to empty gate in `7e465b7`.

### Version alignment

- Aligned tornado crate version with workspace umbrella (`0.2.99`) — commit `7ab9d8b`.

### Foundational crate creation

- **`tornado-styles`**: ANSI → ratatui style bridge. `anstyle::Style` → `ratatui::style::Style` conversion, `HyperlinkTarget` type for OSC-8 anchor registries. Borrowed from `xai-grok-markdown` (Apache-2.0).
- **`tornado-wrap`**: Word-wrap helpers over `ratatui::text::Line`/`Span`. `word_wrap_line()`, `word_wrap_lines_borrowed()`, `wrap_ranges()`/`wrap_ranges_trim()` with `RtOptions` builder. Borrowed from `xai-ratatui-textarea` (Apache-2.0).
- **`tornado-hyperlink`**: Vendored `hyperrat` v0.1 — `Link` widget with OSC 8 hyperlink support.
- **`tornado-spinner`**: Vendored `ratatui-cheese` spinner — `Spinner`, `SpinnerState`, `SpinnerType`.
- **`tornado-scrollview`**: Vendored `tui-scrollview` v0.6.7 — `ScrollView`, `ScrollViewState`.

### Core infrastructure

- `TuiApp` trait: `draw()`, `handle_event()`, `update()`, `should_quit()`, `name()`.
- `run_app()`: terminal init, event loop, panic-hook restoration.
- `event::TuiEvent`: `Key`, `Resize`, `Tick`. `event::poll()` with configurable tick rate.
- `theme::ThemeColors` / `theme::RatatuiThemeColors`: renderer-agnostic palette with dark navy/cyan default and light variant. `Rgb` type with `From<Rgb> for ratatui::style::Color`.
- `terminal::init()` / `terminal::restore()`: crossterm raw mode, alternate screen, mouse capture, panic hook.
- `widget::status_bar()`: themed footer bar with left/right text regions.
- `widget::table_header_style()`, `widget::table_row_style()`, `widget::table_row_highlight_style()`: convenience table style constructors.
- `widget::styled_span()`, `widget::styled_line()`: ANSI → ratatui bridge helpers (`styles` feature).

Initial crate (`f8308b6`) replaced `tui-shell` as the shared TUI stack in the Arniko workspace.

---

## Upgrading notes

### From vendored era (pre-7e465b7) to upstream crates

| Change | Old code | New code |
|---|---|---|
| Sparkline constructor | `Sparkline::new(&[1, 2, 3])` | `Sparkline::default().data(&[1, 2, 3])` |
| Sparkline bar set | `SparklineBar { full: "█", ... }` | `ratatui::symbols::bar::Set { full: "█", ... }` |
| Tree leaf item | `TreeItem::new_leaf(id, text)` | `TreeItem::new(id, text, vec![]).unwrap()` |
| List items | `Vec<Text>` (unchanged) | `Vec<Text>` (consumer API identical — vendored crate already adapted to `Text`) |

---

## Planned

- [ ] Publish to crates.io as a standalone crate.
- [ ] Add `tui-logger` feature for filtered log viewing.
- [ ] Add `tui-prompts` feature for interactive input prompts.
- [ ] Investigate `ratatui-textarea` for multi-line editing.
