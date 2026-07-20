# multi-tab-log

A round-8 example app that composes the round-9 `TabLog` helper under a
tab-navigation surface. Demonstrates the per-tab scroll state machine
(rows + OSC-8 anchors + virtual-scroll viewport + offset preservation)
running concurrently across five distinct log streams.

## Composition

| Region | Source | Role |
|--------|--------|------|
| Title (1 row) | `tornado-spinner` | Global `SpinnerState` + tab indicator |
| TabNav (3 rows) | `ratatui::widgets::Tabs` | Five stream titles, active highlight |
| Body (`Min 3`) | `tornado::tab_log::TabLog` | Active tab's scroll viewport |
| Footer (2 rows) | `tornado::widget::status_bar` | Offset, top-anchor, pin ratio |

## Run it

```sh
cargo run -p multi-tab-log
```

## Controls

| Key             | Action                                |
|-----------------|---------------------------------------|
| `Tab`           | Advance to next tab (wraps)           |
| `BackTab` / `Shift+Tab` | Retreat to previous tab (wraps) |
| `1` … `5`       | Jump directly to that tab (1-indexed) |
| `j` / `↓`       | Scroll active tab down                |
| `k` / `↑`       | Scroll active tab up                  |
| `PgDn`          | Page down on active tab               |
| `PgUp`          | Page up on active tab                 |
| `g` / `Home`    | Jump to top of active tab             |
| `G` / `End`     | Jump to bottom of active tab          |
| `q` / `Esc` / `Ctrl-c` | Quit                          |

## Smoke tests

```sh
cargo test -p multi-tab-log
```

Six inline `TestBackend` tests cover:

1. **Four-region composition** — title, TabNav, body, footer all render
   their expected content for the launch state.
2. **Tab switch re-renders body** — switching to tab 2 ("deps") paints
   that tab's seed content and stops painting tab 0's content.
3. **Per-tab offset preservation** — setting tab 0's offset, switching
   tabs 0→1→2→3→0, and asserting tab 0's offset is unchanged.
4. **Direct-jump keys** — `1`…`5` route to the matching tab index,
   `0` is out of range, `Tab`/`BackTab` wrap correctly, `q` quits.
5. **Spinner advancement** — five ticks move the spinner frame; the
   title region reflects the new frame glyph.
6. **Scroll-arm borrow safety** — `j` / `End` / `PageUp` route through
   `route_to_active`'s two-step-borrow pattern without triggering E0499.

## Hazards carried forward (from `.dejavue` round-7 / round-9 captures)

This example explicitly designs around three collisions captured during
earlier rounds:

- **E0499 two-step borrow** (round-7 #1 / round-9 #1):
  `route_to_active` extracts `is_at_bottom()` to a local *before*
  invoking `set_pinned(...)` so neither arm races on `&mut self.active()`.
- **E0502 disjoint-field pair** (round-9 #2):
  `draw()`'s body-region handler uses `TabLog::render_pair()`, never
  two independent `&ScrollView` / `&mut ScrollViewState` accessor calls.
- **`word_wrap_line` lifetime** (round-7 #2):
  the live rebuild pipeline does *not* invoke `word_wrap_line`; only
  the inline `smoke_wrap_helper_reachable` test exercises the helper,
  within an ephemeral scope.
