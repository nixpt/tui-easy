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
// Round 6 catch-up: the vendored `Tabs` widget is available through
// the umbrella's `tabs` feature as `tornado::widget::TabNav`. (The
// upstream StatefulWidget impl is intentionally vendored-out at
// 0.1 — see `crates/tornado-tabs/src/lib.rs` module-level comment;
// `TornadoState` does not surface here until a future round resumes
// StatefulWidget.)
#[cfg(feature = "tabs")]
pub use crate::tabs::Tabs as TabNav;
// Round 10: the vendored `Sparkline` + `SparklineBar` widgets
// surface via the umbrella's `sparkline` feature. No semantic
// alias needed (unlike the round-6 catch-up `Tabs as TabNav`
// rename) because `Sparkline` is already the canonical upstream
// verb and consumers reach it directly. The `SparklineBar` is
// also exposed because `Sparkline::bar_set(SparklineBar)` is the
// only way to override the default `"█"` symbol.
#[cfg(feature = "sparkline")]
pub use crate::sparkline::{Sparkline, SparklineBar};
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
// These helpers demonstrate the `tornado-styles` bridge in use inside
// `tornado::widget`. They produce ready-to-render ratatui primitives from a
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
