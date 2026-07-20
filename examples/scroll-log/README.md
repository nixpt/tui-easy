# scroll-log

End-to-end example exercising **every** borrow-debris sibling shipped
in `tornado` as a single [`TuiApp`].

This is round 7 of the awesome-ratatui adoption series. The five
sibling crates were vendored in rounds 1–5:

| Widget          | Source crate        | Walmart round |
|-----------------|---------------------|---------------|
| `HyperlinkTarget` | `tornado-styles`  | 1 |
| word-wrapping   | `tornado-wrap`      | 2 |
| `Link`          | `tornado-hyperlink` | 3 |
| `Spinner`       | `tornado-spinner`   | 4 |
| `ScrollView`    | `tornado-scrollview`| 5 |

This `scroll-log` crate is the first downstream app to compose all
five into one running program, providing a worked example of how a
developer can mix-and-match them.

## What it demonstrates

* **Title bar**: `SpinnerState::tick(Duration)` reads as the
  event-loop's `tick_rate`, advancing frames over a virtual log
  stream. Round 4.
* **Body**: a `ScrollView` of fixed `(96 × 240)` cells holds 5 seeded
  rows; long lines are word-wrapped before render so the visible
  rows match what the user can read. Round 5 wrapper, round 2 wrap
  helper.
* **Per-row OSC 8 anchors**: every seeded log row registers a
  [`HyperlinkTarget`] with both a `url` and an `id`. As the user
  scrolls (j/k/↑/↓/PgUp/PgDn/g/G), the top-of-viewport anchor shifts,
  and the status bar shows the current `anchor #NNNN` id. Round 1
  target type + round 3 widget.
* **Footer**: a `tornado::widget::status_bar(left, right, theme)`
  holds the scroll offset + top anchor + frame counter + pin
  state. Round 1 theme bridge.

## Run it

```sh
cargo run -p scroll-log
```

## Controls

| Key     | Action                              |
|---------|-------------------------------------|
| `j` / `↓` | scroll one line down              |
| `k` / `↑` | scroll one line up                |
| `PgDn`  | scroll one page down                |
| `PgUp`  | scroll one page up                  |
| `g` / `Home` | scroll to top, unpin from bottom |
| `G` / `End`  | scroll to bottom, re-pin       |
| `q` / `Esc` / `Ctrl-c` | quit                |

When the cursor is pinned to the bottom, every 5 ticks the app
simulates an async-work completion: it appends a new `DONE` row,
decrements the running-counter, and increments the complete-counter.
The spinner frame also advances each iteration.

## Smoke tests

```sh
cargo test -p scroll-log
```

The crate ships six inline `#[test]`s that drive the full
`update()` / `handle_event()` / `draw()` cycle against a
`ratatui::backend::TestBackend`, so CI exercises the entire
5-vendor composition without spawning a real terminal:

* `smoke_renders_title_body_and_footer` — three regions render
  with the expected substrings.
* `smoke_update_advances_spinner_and_appends_rows` — after 6 ticks
  one task has completed and the new row is visible.
* `smoke_top_anchor_shifts_with_scroll_offset` — `top_anchor()`
  correctly resolves to anchor #1 at offset y=0 and anchor #2 at
  offset y=1.
* `smoke_quit_keys_set_should_quit` — `q` / `Esc` / `Ctrl-c` exit.
* `smoke_scroll_state_responds_to_navigation_keys` — j/k/PgUp/
  PgDn/g/G unpin/re-pin correctly.
* `smoke_wrap_helper_reachable` — round-2 `word_wrap_line` accepts
  a borrowed `Line` and returns ≥ 2 wrapped lines at width 20.

All six pass on a fresh checkout.
