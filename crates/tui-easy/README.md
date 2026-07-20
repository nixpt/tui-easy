# 🌪️ Tornado

**Shared terminal UI shell** for the [Arniko](https://github.com/nixpt/arniko) workspace — terminal init, crossterm event loop, themes, and a curated palette of ratatui widgets behind a unified feature surface.

Tornado unifies all TUI-related code across Arniko and its consumers (including [spores](https://github.com/openko-network/spores)) so every project gets the same event loop, theme system, and widget access without wiring each dependency individually.

---

## Quick start

Add tornado to your `Cargo.toml` with the features you need:

```toml
[dependencies]
tornado = { path = "../arniko/crates/tornado", features = [
    "spinner", "scroller", "big-text", "popup", "tree",
], optional = true }
```

The minimal app looks like this:

```rust
use std::time::Duration;
use tornado::event::TuiEvent;
use tornado::{run_app, TuiApp};

struct MyApp { quit: bool }

impl TuiApp for MyApp {
    fn draw(&mut self, frame: &mut ratatui::Frame) {
        // Render your widgets here
    }
    fn handle_event(&mut self, event: TuiEvent) {
        if let TuiEvent::Key(k) = event {
            if k.code == crossterm::event::KeyCode::Char('q') {
                self.quit = true;
            }
        }
    }
    fn should_quit(&self) -> bool { self.quit }
}

fn main() -> std::io::Result<()> {
    run_app(MyApp { quit: false }, Duration::from_millis(80))
}
```

---

## Feature flags

Tornado's features are entirely additive and **opt-in**: enable only what you need, and no unused dependencies are compiled.

| Feature | Provides | Upstream crate | Path |
|---|---|---|---|
| `tabs` | Tab navigation widget (empty gate for compat) | `ratatui::widgets::Tabs` | `tornado::widget::TabNav` |
| `sparkline` | Bar-chart sparkline widget (empty gate) | `ratatui::widgets::Sparkline` | `tornado::widget::Sparkline` |
| `list` | Scrollable list widget (empty gate) | `ratatui::widgets::{List, ListState}` | `tornado::widget::{List, ListState}` |
| `spinner` | Animated spinner glyph | [`ratatui-cheese`](https://crates.io/crates/ratatui-cheese) · v0.7 | `tornado::widget::Spinner` · `tornado::spinner::{SpinnerState, SpinnerType}` |
| `hyperlink` | OSC 8 hyperlink widget | [`hyperrat`](https://crates.io/crates/hyperrat) · v0.1 | `tornado::widget::Link` · `tornado::hyperlink` |
| `scroller` | Stateful scrollable viewport | [`tui-scrollview`](https://crates.io/crates/tui-scrollview) · v0.6 | `tornado::widget::ScrollView` · `tornado::scroller::{ScrollView, ScrollViewState, ScrollbarVisibility}` |
| `popup` | Popup overlay with drag support | [`tui-popup`](https://crates.io/crates/tui-popup) · v0.7 | `tornado::widget::{Popup, PopupState}` |
| `big-text` | Large pixel text (font8x8) | [`tui-big-text`](https://crates.io/crates/tui-big-text) · v0.8 | `tornado::widget::{BigText, PixelSize}` |
| `tree` | Stateful file tree with fold/expand | [`tui-tree-widget`](https://crates.io/crates/tui-tree-widget) · v0.24 | `tornado::widget::{Tree, TreeItem, TreeState, Flattened}` |
| `styles` | `anstyle` → ratatui bridge | `tornado-styles` (in-house) | `tornado::widget::{styled_span, styled_line}` · `tornado::styles` |
| `wrap` | Word-wrap helpers for `Line`/`Span` | `tornado-wrap` (in-house) | `tornado::wrap::{word_wrap_line, word_wrap_lines_borrowed}` |
| `log_view` | Per-tab scroll-log helper (aggregates `styles` + `scroller` + `wrap`) | Combo | `tornado::tab_log::TabLog` |

Enable multiple features at once:

```toml
tornado = { path = "../arniko/crates/tornado", features = [
    "spinner", "scroller", "popup", "log_view",
] }
```

Or use the umbrella's test surface to validate your feature combination:

```sh
cargo test -p tornado --features "spinner,scroller,popup,hyperlink,big-text,tree,styles,wrap,log_view"
```

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  tornado (umbrella crate)                           │
│                                                     │
│  ┌─ Core ──────────────────────────────────┐        │
│  │  TuiApp trait    ↔  run_app() loop      │        │
│  │  event::TuiEvent ↔  event::poll()       │        │
│  │  ThemeColors     ↔  RatatuiThemeColors  │        │
│  │  terminal::init  ↔  terminal::restore() │        │
│  └──────────────────────────────────────────┘        │
│                                                     │
│  ┌─ Widget re-exports ──────────────────────┐        │
│  │  widget::TabNav     (ratatui::Tabs)      │        │
│  │  widget::Sparkline  (ratatui::Sparkline) │        │
│  │  widget::{List,ListState} (ratatui)      │        │
│  │  widget::Link       (hyperrat)           │        │
│  │  widget::Spinner    (ratatui-cheese)     │        │
│  │  widget::ScrollView (tui-scrollview)     │        │
│  │  widget::Popup      (tui-popup)          │        │
│  │  widget::BigText    (tui-big-text)       │        │
│  │  widget::Tree       (tui-tree-widget)    │        │
│  └──────────────────────────────────────────┘        │
│                                                     │
│  ┌─ In-house modules ───────────────────────┐        │
│  │  tab_log::TabLog   (log_view feature)    │        │
│  │  styles::*         (styles feature)      │        │
│  │  wrap::*           (wrap feature)        │        │
│  └──────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────┘
```

### Core infrastructure

Tornado handles all the boilerplate so each app only needs to implement the [`TuiApp`] trait:

```rust
pub trait TuiApp {
    /// Render the frame — called once per tick.
    fn draw(&mut self, frame: &mut ratatui::Frame);

    /// Handle a keyboard event or tick.
    fn handle_event(&mut self, event: TuiEvent);

    /// Called each tick before draw. Default: no-op.
    fn update(&mut self) {}

    /// Return `true` to exit the event loop.
    fn should_quit(&self) -> bool;

    /// Application name (shown in panics if terminal isn't restored).
    fn name(&self) -> &str { "tornado" }
}
```

The event loop lives in [`run_app()`]:

```rust
pub fn run_app(app: impl TuiApp, tick_rate: Duration) -> std::io::Result<()>
```

**What it does for you:**
1. Enables raw mode + alternate screen
2. Installs a panic hook that always restores the terminal
3. Runs the `update → draw → poll → handle_event` loop at `tick_rate`
4. Restores the terminal on clean exit or panic

### Theme system

Themes are declared as renderer-agnostic [`ThemeColors`] (with `Rgb` values) and converted to ratatui-specific colours via [`to_ratatui()`]:

```rust
use tornado::theme::{ThemeColors, RatatuiThemeColors};

let colors: ThemeColors = ThemeColors::default();        // dark navy/cyan
let light:  ThemeColors = ThemeColors::light();           // light variant
let rt:     RatatuiThemeColors = colors.to_ratatui();
```

| Colour field | Dark default | Light default |
|---|---|---|
| `bg` | `Rgb(10, 14, 26)` | `Rgb(245, 245, 245)` |
| `fg` | `Rgb(200, 210, 220)` | `Rgb(30, 30, 30)` |
| `accent` | `Rgb(0, 188, 212) cyan` | `Rgb(0, 150, 180)` |
| `surface` | `Rgb(15, 20, 35)` | `Rgb(235, 235, 240)` |
| `highlight` | `Rgb(30, 40, 60)` | `Rgb(210, 225, 240)` |
| `border` | `Rgb(50, 60, 75)` | `Rgb(190, 190, 190)` |
| `dim` | `Rgb(100, 110, 125)` | `Rgb(140, 140, 140)` |
| `error` | `Rgb(255, 83, 112)` | `Rgb(200, 40, 60)` |
| `success` | `Rgb(76, 175, 80)` | `Rgb(50, 140, 55)` |
| `warn` | `Rgb(255, 193, 7)` | `Rgb(200, 150, 0)` |

---

## Usage patterns

### 1. Minimal app with status bar

The simplest integrated example — a title row, a body, and a themed status bar footer.

```rust
use std::time::Duration;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::Paragraph;
use tornado::event::TuiEvent;
use tornado::theme::RatatuiThemeColors;
use tornado::widget::status_bar;
use tornado::{run_app, TuiApp};

struct App { quit: bool, theme: RatatuiThemeColors }

impl TuiApp for App {
    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(area);

        // Body
        frame.render_widget(Paragraph::new("Hello tornado!"), chunks[0]);

        // Footer
        frame.render_widget(
            status_bar("tab 1/1 │ q", Some(String::new()), &self.theme),
            chunks[1],
        );
    }
    fn handle_event(&mut self, event: TuiEvent) {
        if let TuiEvent::Key(k) = event {
            if k.code == crossterm::event::KeyCode::Char('q') {
                self.quit = true;
            }
        }
    }
    fn should_quit(&self) -> bool { self.quit }
}
```

### 2. Tab navigation with TabNav

Uses `tornado::widget::TabNav` (re-export of `ratatui::widgets::Tabs`):

```rust
use tornado::widget::TabNav;
use ratatui::text::Line;

let tabs = TabNav::new(
    ["Build", "Tests", "Deploy"]
        .iter().copied().map(Line::from).collect::<Vec<Line<'static>>>(),
)
.select(self.active_tab)
.divider(" │ ")
.highlight_style(Style::new().fg(Color::Cyan).bold());
```

### 3. Spinner in the title bar

```rust
use tornado::spinner::{SpinnerState, SpinnerType};
use std::time::Instant;

let mut last_tick = Instant::now();
let mut spinner_state = SpinnerState::new(SpinnerType::Dot);

// In update():
fn update(&mut self) {
    let dt = self.last_tick.elapsed();
    self.spinner_state.tick(dt);
    self.last_tick = Instant::now();
}

// In draw():
let glyph = self.spinner_state.frame_str();
frame.render_widget(
    Paragraph::new(Line::from(Span::styled(
        format!(" {glyph} my-app │ tick {}", self.tick_count),
        Style::default().fg(Color::Cyan),
    ))),
    title_area,
);
```

### 4. Sparkline in the footer

A rolling-c sine-wave sparkline next to the status bar:

```rust
use tornado::widget::Sparkline;
use ratatui::symbols::bar;

// Update a ring buffer each tick:
fn push_sparkline_metric(&mut self, value: u64) {
    self.sparkline_ring.rotate_left(1);
    let last = self.sparkline_ring.len() - 1;
    self.sparkline_ring[last] = value;
}

// In draw(), split the footer horizontally:
let footer_split = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Min(1), Constraint::Length(12)])
    .split(footer_area);

frame.render_widget(
    Sparkline::default()
        .data(&self.sparkline_ring[..])
        .style(Style::new().fg(Color::Cyan).bold())
        .bar_set(bar::Set {
            full: "█", seven_eighths: "▇", three_quarters: "▆",
            five_eighths: "▅", half: "▄", three_eighths: "▃",
            one_quarter: "▂", one_eighth: "▁", empty: " ",
        }),
    footer_split[1],
);
```

### 5. Scrollable log with TabLog

The `log_view` feature aggregates `tornado-styles` + `tui-scrollview` + `tornado-wrap` into a single helper. Each `TabLog` owns a virtual-scroll buffer, anchor registry, and pin-to-bottom tracking.

```rust
use tornado::tab_log::TabLog;

// Create a log buffer:
let mut log = TabLog::new(80, 240);

// Push rows:
log.push_row(
    "timestamp".into(), "INFO ".into(),
    "hello world".into(),
    Some("https://example.com/1".into()),
);

// Rebuild the scroll buffer (call once per tick after all pushes):
log.rebuild_buffer();

// Render (for use with render_stateful_widget):
let pair = log.render_pair();
frame.render_stateful_widget(pair.view, body_area, pair.state);

// Scroll keys in handle_event:
log.scroll_state_mut().scroll_down();
log.set_pinned(false);       // unpin when user scrolls manually
log.reset_to_bottom_if_pinned(); // re-pin on new content
```

### 6. Popup overlay

A centered popup that appears on a keypress:

```rust
use tornado::widget::{Popup, PopupState};

let mut popup_state = PopupState::default();
let mut popup_visible = false;

// Toggle visibility:
fn handle_event(&mut self, event: TuiEvent) {
    if let TuiEvent::Key(KeyEvent { code: KeyCode::Char('p'), .. }) = event {
        self.popup_visible = !self.popup_visible;
    }
}

// In draw():
if self.popup_visible {
    let popup = Popup::new(Text::from("Popup content here"))
        .title(" Alert ")
        .style(Style::new().fg(Color::White).bg(Color::Blue))
        .borders(Borders::ALL);
    let area = centered_rect(body_area, 55, 35);
    frame.render_stateful_widget(popup, area, &mut self.popup_state);
}
```

### 7. BigText welcome screen

Large pixel text using font8x8 glyphs:

```rust
use tornado::widget::{BigText, PixelSize};

let big = BigText::builder()
    .lines(vec![Line::from("TORNADO")])
    .pixel_size(PixelSize::Full)
    .style(Style::new().fg(Color::Cyan).bold())
    .build();
frame.render_widget(big, chunks[0]);
```

### 8. File-system tree widget

A stateful tree with fold/expand:

```rust
use tornado::widget::{Tree, TreeItem, TreeState};

// Build items (TreeItem::new returns Result):
let items = vec![
    TreeItem::new("src".to_string(), Line::from("src/"), vec![
        TreeItem::new("main.rs".to_string(), Line::from("  main.rs"), vec![]).unwrap(),
    ]).unwrap(),
    TreeItem::new("README.md".to_string(), Line::from("README.md"), vec![]).unwrap(),
];

// Render:
let tree = Tree::new(&items).unwrap()
    .highlight_style(Style::new().bg(Color::DarkGray))
    .highlight_symbol("> ");
frame.render_stateful_widget(tree, area, &mut self.tree_state);

// Navigation in handle_event():
self.tree_state.key_down();   // j / ↓
self.tree_state.key_up();     // k / ↑
self.tree_state.key_right();  // l / → expand
self.tree_state.key_left();   // h / ← collapse
```

### 9. ANSI → ratatui bridge (styles feature)

If you have text styled with `anstyle::Style` (e.g. from a markdown renderer or a pipe), convert it to ratatui primitives:

```rust
use tornado::widget::{styled_span, styled_line};
use tornado_styles::Style as AnsiStyle;

let span = styled_span("bold red text", AnsiStyle::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Red))));
let line = styled_line("green italic summary", AnsiStyle::new().italic().fg_color(Some(Color::Ansi(AnsiColor::Green))));
```

### 10. Word-wrap helpers (wrap feature)

Wrap a ratatui `Line` to a max column width while preserving per-span styles:

```rust
use tornado::wrap::{word_wrap_line, RtOptions};

let line = Line::from(vec!["long text that needs wrapping ".red(), "at 30 columns".into()]);
let wrapped = word_wrap_line(&line, RtOptions::new(30).initial_indent(Line::from("→ ")));
```

---

## Examples

Three example apps ship with the crate, exercising different feature combinations:

### `tornado-showcase` (all features)

A 5-tab visual demo exercising **every** tornado widget in one app.

```sh
cargo run -p tornado-showcase
cargo test  -p tornado-showcase   # 8 TestBackend smoke tests
```

| Tab | Widgets |
|---|---|
| Welcome | `BigText` (TORNADO) + `Link` (GitHub, Ratatui) |
| Sparkline | Live sine-wave `Sparkline` with bar set |
| List | 50-item scrollable `List` with highlight |
| Tree | Mock filesystem `Tree` with fold/expand |
| Popup | Toggleable `Popup` overlay over `TabLog` |

### `multi-tab-log` (tabs, spinner, scroller, sparkline, hyperlink, log_view)

A 5-stream log viewer with tab navigation, global spinner, per-tab scroll state, and a live sparkline in the footer.

```sh
cargo run -p multi-tab-log
cargo test  -p multi-tab-log   # 6 TestBackend smoke tests
```

### `scroll-log` (spinner, scroller, log_view)

A single-stream log viewer with OSC-8 hyperlink anchors and scrollbar visibility policy.

```sh
cargo run -p scroll-log
cargo test  -p scroll-log   # inline smoke tests
```

---

## Cross-project usage

The [spores project](https://github.com/openko-network/spores) uses tornado as its TUI runtime. It depends on tornado's **core infrastructure** — the `TuiApp` trait, `run_app()`, theme types, and event system — without enabling any widget features:

```toml
# spores/Cargo.toml
[dependencies]
tornado = { path = "../../arniko/crates/tornado", optional = true }

[features]
spores-tui = ["dep:ratatui", "dep:crossterm", "dep:tornado"]
```

```rust
// spores/src/tui/mod.rs
impl tornado::TuiApp for App {
    fn draw(&mut self, frame: &mut ratatui::Frame) { /* custom ui.rs */ }
    fn handle_event(&mut self, event: tornado::event::TuiEvent) { /* ... */ }
    fn should_quit(&self) -> bool { /* ... */ }
}

fn main() {
    tornado::run_app(app, std::time::Duration::from_millis(250))?;
}
```

---

## Testing smoke tests

Every example app and the core crate include `#[cfg(test)]` smoke tests backed by `ratatui::backend::TestBackend`. These render into an off-screen buffer and assert on cell content — no real terminal needed.

```sh
# Run all tornado-related tests:
cargo test -p tornado -p tornado-showcase -p multi-tab-log -p scroll-log

# Run with all features to test every widget combination:
cargo test -p tornado --all-features
```

Integration tests live under `crates/tornado/tests/` and are gated per-feature:
- `styles_integration` — anstyle → ratatui roundtrip
- `hyperlink_integration` — OSC 8 encoding
- `spinner_integration` — frame advance via tick
- `scrollview_integration` — offset policy + viewport slicing

---

## Design decisions

| # | Area | Choice | Rationale |
|---|---|---|---|
| D1 | Widget access | Re-export from upstream crates | Avoid vendoring; stay compatible with ratatui 0.30 subcrate architecture |
| D2 | Feature gates | Each widget is an optional feature | Zero-cost for consumers that don't need a widget |
| D3 | Log_view | Aggregates styles + scroller + wrap | Single `TabLog` helper avoids repeating the same 3-dep combo |
| D4 | Empty features | `tabs` / `sparkline` / `list` are empty gates | Backward compat for consumers that already enabled them during the vendoring era |
| D5 | Event type | `TuiEvent::Tick` when poll times out | Distinguishes timer ticks from keyboard silence |
| D6 | Theme system | Renderer-agnostic `ThemeColors` + `RatatuiThemeColors` | Same palette drives both ratatui and (future) bliss GUI UIs |
| D7 | Pair return | `TabLog::render_pair()` returns disjoint borrows | Sidesteps E0502 when passing `&ScrollView` + `&mut ScrollViewState` to `render_stateful_widget` |

---

## License

MIT OR Apache-2.0 — see [LICENSE-MIT](../../LICENSE-MIT) and [LICENSE-APACHE](../../LICENSE-APACHE).

Community widget crates carry their own licenses (all permissive):
- `ratatui-cheese` · MIT
- `hyperrat` · MIT / Unlicense
- `tui-scrollview` · MIT / Apache-2.0
- `tui-popup` · MIT
- `tui-big-text` · MIT / Apache-2.0
- `tui-tree-widget` · MIT

[`TuiApp`]: https://docs.rs/tornado/latest/tornado/trait.TuiApp.html
[`run_app()`]: https://docs.rs/tornado/latest/tornado/fn.run_app.html
[`ThemeColors`]: https://docs.rs/tornado/latest/tornado/theme/struct.ThemeColors.html
[`to_ratatui()`]: https://docs.rs/tornado/latest/tornado/theme/struct.ThemeColors.html#method.to_ratatui
