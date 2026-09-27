# 🌪️ Tui Easy

A ratatui convenience toolkit — shared terminal init, a crossterm event loop,
theming, and a curated palette of TUI widgets (tabs, sparklines, lists,
trees, popups, big-text, hyperlinks, scrollable views, a per-tab scroll-log
helper, and an original bar-chart / text-input widget) behind one unified,
opt-in feature surface.

Extracted from [nixpt/arniko](https://github.com/nixpt/arniko), where it
lived as the `tornado` cluster of crates. This repository carries that
cluster's git history and continues it standalone.

See [`crates/tui-easy/README.md`](crates/tui-easy/README.md) for the full
feature-flag table, usage patterns, architecture diagram, and API docs.

## Crates

| Crate | Role |
|---|---|
| `tui-easy` | Umbrella crate — `TuiApp` trait, `run_app()` event loop, themes, widget re-exports |
| `tui-easy-styles` | `anstyle` → ratatui style bridge + `HyperlinkTarget` |
| `tui-easy-wrap` | `textwrap` word-wrap helpers over ratatui `Line`/`Span` |
| `tui-easy-barchart` | Original `BarChart`/`Bar`/`BarGroup` widget |
| `tui-easy-textinput` | `TextArea` + `TextAreaState` + `EditBuffer` input widget |

Plus four example apps under `examples/`: `scroll-log`, `command-palette`,
`multi-tab-log`, and `tui-easy-showcase` (exercises every widget).

## Quick start

```toml
[dependencies]
tui-easy = { version = "0.3", features = [
    "spinner", "scroller", "big-text", "popup", "tree",
] }
```

**crossterm:** don't add your own `crossterm` dependency. tui-easy takes its
event and key types from ratatui's re-export and passes it on as
`tui_easy::crossterm`, so they always match ratatui's backend. Import from there
(`use tui_easy::crossterm::event::{KeyCode, KeyEvent};`). A direct crossterm
dependency on a different version compiles into distinct types that won't
match `TuiEvent::Key`.

```bash
cargo build --workspace
cargo test --workspace
cargo run -p tui-easy-showcase
```

## Relationship to upstream

`tui-easy-barchart` vendors `ratatui::widgets::BarChart` (MIT OR Apache-2.0).
`tui-easy-styles`, `tui-easy-textinput`, and `tui-easy-wrap` are derived from
first-party Apache-2.0-only code in xAI's grok-build project
(`xai-org/grok-build`). Upstream copyright and license terms are preserved in
[NOTICE](NOTICE) and each crate's own `NOTICE` file.

## License

Most crates: MIT OR Apache-2.0 — see [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).

Exceptions (Apache-2.0 only, derived from Apache-2.0-only xai-org/grok-build
code):

- `tui-easy-styles`
- `tui-easy-textinput`
- `tui-easy-wrap`

Third-party attributions: [NOTICE](NOTICE).
