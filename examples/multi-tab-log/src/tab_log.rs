//! Per-tab scroll-log seeders and update pumpers.
//!
//! Round-8 multi-tab log consumer composes a `Vec<TabLog>` (the
//! round-9 helper, behind the `log_view` umbrella feature) keyed to
//! five distinct log-stream titles. Each `TabLog` instance is
//! independent — switching tabs preserves per-tab scroll offsets
//! natively because the underlying `ScrollViewState` is per-instance.
//!
//! Per-tab mechanics are fully encapsulated inside `TabLog`'s public
//! API:
//! - row registry + sequential anchor-id assignment;
//! - OSC-8 anchor registration at rebuild time (`HyperlinkTarget`);
//! - `ScrollView` + `ScrollViewState` for virtual-scroll rendering;
//! - top-of-viewport anchor lookup (`top_anchor()`);
//! - pinned-to-bottom tracking (`is_pinned` / `set_pinned` /
//!   `reset_to_bottom_if_pinned`).
//!
//! This module owns the seeding recipes and the update-pumping
//! helpers; `main.rs` is a thin orchestrator on top.

use tui_easy::tab_log::TabLog;

/// The five lab streams round-8 ships out-of-the-box. Anchored to the
/// `MultiTabApp::handle_event` keymap: keys `1..=5` jump directly to
/// each stream's index (1-indexed for the user, 0-indexed internally).
///
/// Title ordering is informal: "build" first because that is what CI
/// watchers want to see on launch, followed by the streams that
/// compose / verify / package / publish the rest of the system.
pub const TAB_TITLES: &[&str] = &["build", "tests", "deps", "lint", "release"];

/// Initial scroll-buffer dimensions per tab. Picked liberally so a
/// long-lived session doesn't exhaust scrollback during demo runs.
const STREAM_WIDTH: u16 = 96;
const STREAM_HEIGHT: u16 = 240;

/// Build all five streams with seed rows. Each tab gets six initial
/// rows so the very first `draw()` shows a populated viewport for
/// every stream (the user can switch tabs immediately and see content).
pub fn seed_streams() -> Vec<TabLog> {
    TAB_TITLES
        .iter()
        .map(|title| seed_one(title, STREAM_WIDTH, STREAM_HEIGHT))
        .collect()
}

fn seed_one(title: &str, w: u16, h: u16) -> TabLog {
    let mut tab = TabLog::new(w, h);

    // Each stream boots with 6 seed rows: timestamp placeholder +
    // category + a slug reflecting the stream name. Every row carries
    // a URL so the status-bar's `top_anchor()` always has something
    // meaningful to display.
    for i in 0..6 {
        tab.push_row(
            format!("{:08x}", i as u32),
            "INFO ".to_string(),
            format!("{title} step {i}"),
            Some(format!(
                "https://github.com/nixpt/tui-easy/blob/main/{title}/{i}"
            )),
        );
    }

    // Build the scroll-view paragraph once so the first `draw()`
    // paints the seeded rows. Without this, the buffer is still
    // `Buffer::empty(...)` and the viewport paints blank.
    tab.rebuild_buffer();
    tab
}

/// Append a new row to a specific tab + rebuild that tab's buffer.
/// Updates are per-tab so active-tab switching doesn't trigger a
/// cascade of rebuilds across the entire `Vec<TabLog>`.
pub fn append_row(tabs: &mut [TabLog], idx: usize, msg: String, url: Option<String>) {
    if let Some(tab) = tabs.get_mut(idx) {
        tab.push_row(format_ts(), "DONE ".to_string(), msg, url);
        tab.rebuild_buffer();
    }
}

/// Compact 8-hex-digit timestamp placeholder. The example doesn't
/// measure wall-clock — the placeholder keeps the timestamps visually
/// distinct across rows without dragging in real-time semantics.
fn format_ts() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{:08x}", now & 0xFFFF_FFFF)
}

// ── Self-tests (deferred to integration tests in main.rs) ─────────────
//
// `MultiTabApp` is the surface tested by the inline `mod tests` in
// `main.rs`. This module is intentionally pure: no `TuiApp`-related
// plumbing leaks here, so the seeders can be reused by future
// example-app cargoes without dragging in test-only imports.
