//! Widget re-export hub.
//!
//! All widget types exposed by the tui_easy umbrella are re-exported here,
//! gated behind their corresponding feature flags. Consumers are encouraged
//! to import from `tui_easy::widget::*` rather than from the per-crate modules
//! (`tui_easy::spinner`, `tui_easy::popup`, etc.) to keep import paths stable
//! across upstream crate changes.
//!
//! ## Always-available helpers
//!
//! * [`status_bar`] — themed footer bar with left/right text regions.
//! * [`table_header_style`], [`table_row_style`], [`table_row_highlight_style`] —
//!   convenience style constructors for table rendering.
//!
//! ## Feature-gated re-exports
//!
//! | Feature | Re-exports |
//! |---|---|
//! | `tabs` | [`TabNav`] = `ratatui::widgets::Tabs` |
//! | `sparkline` | [`Sparkline`] = `ratatui::widgets::Sparkline` |
//! | `list` | [`List`], [`ListState`] = `ratatui::widgets` |
//! | `spinner` | [`Spinner`] = `ratatui_cheese::spinner::Spinner` |
//! | `hyperlink` | [`Link`] = `hyperrat::Link` |
//! | `scroller` | [`ScrollView`] = `tui_scrollview::ScrollView` |
//! | `popup` | [`Popup`], [`PopupState`] = `tui_popup` |
//! | `big-text` | [`BigText`], [`PixelSize`] = `tui_big_text` |
//! | `tree` | [`Tree`], [`TreeItem`], [`TreeState`], [`Flattened`] = `tui_tree_widget` |
//! | `barchart` | [`BarChart`], [`Bar`], [`BarGroup`], [`BarSet`] = `tui-easy-barchart` |
//! | `textinput` | [`TextArea`], [`TextAreaState`] = `tui-easy-textinput` |
//! | `styles` | [`styled_span`], [`styled_line`], ANSI bridge types |

#![allow(clippy::manual_is_multiple_of)]

#[cfg(feature = "styles")]
pub use crate::styles::{
    HyperlinkTarget as AnsiHyperlinkTarget, anstyle_to_ratatui_color as ansi_color,
    style_into_ratatui as ansi_to_style,
};
#[cfg(feature = "hyperlink")]
pub use crate::hyperlink::Link;
#[cfg(feature = "spinner")]
pub use crate::spinner::Spinner;
#[cfg(feature = "scroller")]
pub use crate::scroller::ScrollView;
// Round 6 catch-up (migrated): `TabNav` is now re-exported from
// `ratatui::widgets::Tabs` directly. The vendored `crates/tui_easy-tabs`
// has been removed. The `tabs` feature is kept as an empty feature
// gate for backward compatibility — enabling it makes `TabNav`
// available at `tui_easy::widget::TabNav`. The upstream
// StatefulWidget impl is not exposed; consumers use the standard
// `Widget` impl (consuming) which matches the round-6 contract.
#[cfg(feature = "tabs")]
pub use ratatui::widgets::Tabs as TabNav;
// Migrated: `Sparkline` is now re-exported from
// `ratatui::widgets::Sparkline` directly. The vendored
// `crates/tui_easy-sparkline` has been removed. Note: the native
// API differs from the vendored version — `Sparkline::new(&[u64])`
// became `Sparkline::default().data(&[u64])`, and `bar_set()`
// now accepts `ratatui::symbols::bar::Set` instead of
// `SparklineBar`. Consumers may need to update their code.
// The `sparkline` feature is kept as an empty gate for backward
// compatibility.
#[cfg(feature = "sparkline")]
pub use ratatui::widgets::Sparkline;
// Migrated: `List` and `ListState` are now re-exported from
// `ratatui::widgets` directly. The vendored `crates/tui_easy-list`
// has been removed. The native `List` accepts the same inputs
// (`Vec<Text>` via `Into<ListItem>`) and has the same builder
// API. The fully-qualified form `<List as StatefulWidget>::render`
// is still the recommended disambiguation pattern when both
// `Widget` and `StatefulWidget` are in scope (E0034 carve-out).
#[cfg(feature = "list")]
pub use ratatui::widgets::{List, ListState};
// Round 12 — vendored Popup + PopupState from joshka/tui-popup (MIT).
// Consumers reach them as `tui_easy::widget::{Popup, PopupState}`.
// The popup implements both `Widget for &Popup` and
// `StatefulWidget for &Popup` for stateless and stateful rendering.
// Stateful rendering enables mouse-drag repositioning via
// `PopupState::mouse_down/mouse_up/mouse_drag`.
#[cfg(feature = "popup")]
pub use crate::popup::{Popup, PopupState};
// Migrated from vendored `tui_easy-big-text` to upstream
// `tui-big-text` crate (v0.8, MIT/Apache-2.0). Consumers reach
// them as `tui_easy::widget::{BigText, PixelSize}`.
#[cfg(feature = "big-text")]
pub use crate::big_text::{BigText, PixelSize};
// Round 14 — vendored Tree + TreeItem + TreeState + Flattened from
// EdJoPaTo/tui-rs-tree-widget (MIT). Consumers reach them as
// `tui_easy::widget::{Tree, TreeItem, TreeState, Flattened}`.
// The widget implements both `Widget` and `StatefulWidget`.
#[cfg(feature = "tree")]
pub use crate::tree::{Flattened, Tree, TreeItem, TreeState};
// Round 12 — original BarChart/Bar/BarGroup/BarSet widget (no upstream
// migration path — this isn't a thin wrapper over a `ratatui::widgets`
// type). Consumers reach them as `tui_easy::widget::{BarChart, Bar,
// BarGroup, BarSet}`.
#[cfg(feature = "barchart")]
pub use crate::barchart::{Bar, BarChart, BarGroup, BarSet};
// Round 13 — vendored TextArea + TextAreaState from xai-ratatui-textarea
// (Apache-2.0). Consumers reach them as
// `tui_easy::widget::{TextArea, TextAreaState}`. The fully-qualified
// `<&TextArea as ratatui::widgets::StatefulWidgetRef>::render_ref(...)`
// form is the supported render path — `StatefulWidgetRef` is only
// implemented for `&TextArea`, not `TextArea` by value.
#[cfg(feature = "textinput")]
pub use crate::textinput::{TextArea, TextAreaState};
#[cfg(feature = "styles")]
use crate::styles::Style as AnsiStyle;

use ratatui::style::Style;
use ratatui::text::Line;
#[cfg(feature = "styles")]
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::RatatuiThemeColors;

/// Style for a table header row.
pub fn table_header_style(theme: &RatatuiThemeColors) -> Style {
    Style::default()
        .fg(theme.accent)
        .bg(theme.surface)
        .bold()
}

/// Style for a table row with zebra striping.
pub fn table_row_style(theme: &RatatuiThemeColors, index: usize) -> Style {
    if index % 2 == 0 {
        Style::default().fg(theme.fg).bg(theme.bg)
    } else {
        Style::default()
            .fg(theme.fg)
            .bg(theme.surface)
    }
}

/// Style for a highlighted / selected table row.
pub fn table_row_highlight_style(theme: &RatatuiThemeColors) -> Style {
    Style::default()
        .fg(theme.fg)
        .bg(theme.highlight)
}

/// Build a status bar paragraph spanning the full width.
///
/// Renders `left` text on a surface-coloured bar.
/// An optional `right` label is placed alongside it.
pub fn status_bar<'a>(
    left: impl Into<Line<'a>>,
    right: Option<impl Into<Line<'a>>>,
    theme: &RatatuiThemeColors,
) -> Paragraph<'a> {
    let bar_style = Style::default()
        .fg(theme.dim)
        .bg(theme.surface);

    let left_line: Line = left.into();
    let line = if let Some(r) = right {
        let mut spans = left_line.spans;
        spans.push("  ".into());
        spans.extend(r.into().spans);
        Line::from(spans)
    } else {
        left_line
    };

    Paragraph::new(line)
        .style(bar_style)
        .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(theme.border)))
}

// ─── ANSI ↔ ratatui bridge helpers (feature: `styles`) ────────────────────
//
// These helpers demonstrate the `tui-easy-styles` bridge in use inside
// `tui_easy::widget`. They produce ready-to-render ratatui primitives from a
// plain `&str` plus an `anstyle::Style`, which is the typical shape of text
// that has been styled by an upstream library (markdown, man pages, pipes,
// etc.) before it reaches the TUI shell.

/// Wrap a single string in an ANSI `Style` and return it as a ratatui `Span`.
///
/// Effectively `Span::styled(text, style_into_ratatui(ansi))`. Convenient for
/// TUI callers that already have an `anstyle::Style` in hand and want to drop
/// the styled text into a [`Line`] or [`ratatui::widgets::Paragraph`].
#[cfg(feature = "styles")]
pub fn styled_span<'a>(text: &'a str, ansi: AnsiStyle) -> Span<'a> {
    Span::styled(text, crate::styles::style_into_ratatui(ansi))
}

/// Wrap a single string in an ANSI `Style` and return it as a one-span
/// ratatui `Line`. Useful when you want to render a single styled phrase in
/// isolation (e.g. a status-bar capsule, a header chip, an inline indicator).
#[cfg(feature = "styles")]
pub fn styled_line<'a>(text: &'a str, ansi: AnsiStyle) -> Line<'a> {
    Line::from(vec![Span::styled(
        text,
        crate::styles::style_into_ratatui(ansi),
    )])
}
