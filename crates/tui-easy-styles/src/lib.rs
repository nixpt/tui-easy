//! Bridges between ANSI-style libraries and `ratatui`.
//!
//! Three pieces:
//!
//! 1. [`anstyle_to_ratatui_color`] / [`style_into_ratatui`] — convert an
//!    `anstyle::Style` / `anstyle::Color` into a `ratatui::style::Style` /
//!    `ratatui::style::Color`.
//! 2. [`HyperlinkTarget`] — describes one OSC-8 hyperlink anchor on a
//!    rendered line. Many libraries emit these alongside rendered text.
//! 3. Re-exports of `anstyle::{AnsiColor, Color, RgbColor, Style}` — let
//!    dependents route anstyle types via this crate instead of taking a
//!    direct `anstyle` dep.
//!
//! The first two pieces are first-party code from `xai-grok-markdown`
//! (Apache-2.0). Borrowed from xAI's Grok build (grok-build) crate; see
//! `LICENSE-APACHE` and `NOTICE` at the root of this crate.

#![allow(clippy::module_name_repetitions)]

use std::ops::Range;

use anstyle::Effects;

/// Convert a single ANSI color into a ratatui color.
///
/// Covers the standard 16 ANSI foregrounds, indexed 256-color, and RGB.
pub fn anstyle_to_ratatui_color(color: anstyle::Color) -> ratatui::style::Color {
    use ratatui::style::Color;
    match color {
        anstyle::Color::Ansi(ansi) => match ansi {
            anstyle::AnsiColor::Black => Color::Black,
            anstyle::AnsiColor::Red => Color::Red,
            anstyle::AnsiColor::Green => Color::Green,
            anstyle::AnsiColor::Yellow => Color::Yellow,
            anstyle::AnsiColor::Blue => Color::Blue,
            anstyle::AnsiColor::Magenta => Color::Magenta,
            anstyle::AnsiColor::Cyan => Color::Cyan,
            anstyle::AnsiColor::White => Color::Gray,
            anstyle::AnsiColor::BrightBlack => Color::DarkGray,
            anstyle::AnsiColor::BrightRed => Color::LightRed,
            anstyle::AnsiColor::BrightGreen => Color::LightGreen,
            anstyle::AnsiColor::BrightYellow => Color::LightYellow,
            anstyle::AnsiColor::BrightBlue => Color::LightBlue,
            anstyle::AnsiColor::BrightMagenta => Color::LightMagenta,
            anstyle::AnsiColor::BrightCyan => Color::LightCyan,
            anstyle::AnsiColor::BrightWhite => Color::White,
        },
        anstyle::Color::Ansi256(idx) => Color::Indexed(idx.index()),
        anstyle::Color::Rgb(rgb) => Color::Rgb(rgb.0, rgb.1, rgb.2),
    }
}

/// Convert an [`anstyle::Style`] into a [`ratatui::style::Style`].
///
/// Foreground, background, and text effects (bold, dim, italic, underline,
/// strikethrough, hidden) are mapped to the ratatui equivalents. Unset
/// fields remain unset.
pub fn style_into_ratatui(style: Style) -> ratatui::style::Style {
    use ratatui::style::{Modifier, Style as RStyle};

    let mut out = RStyle::default();

    if let Some(fg) = style.get_fg_color() {
        out = out.fg(anstyle_to_ratatui_color(fg));
    }
    if let Some(bg) = style.get_bg_color() {
        out = out.bg(anstyle_to_ratatui_color(bg));
    }

    let effects = style.get_effects();
    let mut modifiers = Modifier::empty();
    if effects.contains(Effects::BOLD) {
        modifiers |= Modifier::BOLD;
    }
    if effects.contains(Effects::DIMMED) {
        modifiers |= Modifier::DIM;
    }
    if effects.contains(Effects::ITALIC) {
        modifiers |= Modifier::ITALIC;
    }
    if effects.contains(Effects::UNDERLINE) {
        modifiers |= Modifier::UNDERLINED;
    }
    if effects.contains(Effects::STRIKETHROUGH) {
        modifiers |= Modifier::CROSSED_OUT;
    }
    if effects.contains(Effects::HIDDEN) {
        modifiers |= Modifier::HIDDEN;
    }

    out.add_modifier(modifiers)
}

/// A hyperlink target extracted from rendered markdown.
///
/// Each instance maps a contiguous cell range on one rendered line to a URL.
/// When a link wraps across lines, multiple `HyperlinkTarget`s share the same
/// `id` and `url` — the `id` enables OSC 8 hover-grouping across wrapped lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyperlinkTarget {
    /// Index of the rendered line this target appears on.
    pub line_index: usize,
    /// Column range (in display cells) of the link text on that line.
    pub column_range: Range<usize>,
    /// The destination URL.
    pub url: String,
    /// Stable identifier for grouping link fragments that belong to the
    /// same logical link (e.g. a link whose text wraps across lines).
    pub id: u32,
}

// Re-export the anstyle types this crate consumes so dependents can route them
// via `tui-easy-styles` rather than taking a direct `anstyle` dep. Lets the
// `tui_easy::widget::styled_span` / `styled_line` API accept an `anstyle::Style`
// without requiring `anstyle` to be a public dependency of `tui_easy`.
pub use anstyle::{AnsiColor, Color, RgbColor, Style};

#[cfg(test)]
mod tests {
    use super::*;
    use anstyle::{AnsiColor, Color, RgbColor, Style};
    use ratatui::style::{Color as RColor, Modifier, Style as RStyle};

    #[test]
    fn standard_ansi_fg_maps_to_named_color() {
        for (ansi, expected) in [
            (AnsiColor::Black, RColor::Black),
            (AnsiColor::Red, RColor::Red),
            (AnsiColor::Green, RColor::Green),
            (AnsiColor::Yellow, RColor::Yellow),
            (AnsiColor::Blue, RColor::Blue),
            (AnsiColor::Magenta, RColor::Magenta),
            (AnsiColor::Cyan, RColor::Cyan),
            (AnsiColor::White, RColor::Gray),
            (AnsiColor::BrightBlack, RColor::DarkGray),
            (AnsiColor::BrightWhite, RColor::White),
        ] {
            assert_eq!(
                anstyle_to_ratatui_color(Color::Ansi(ansi)),
                expected,
                "ansi {ansi:?}",
            );
        }
    }

    #[test]
    fn rgb_color_passes_through_unchanged() {
        let c = anstyle_to_ratatui_color(Color::Rgb(RgbColor(10, 20, 30)));
        assert_eq!(c, RColor::Rgb(10, 20, 30));
    }

    #[test]
    fn empty_style_is_default() {
        assert_eq!(style_into_ratatui(Style::new()), RStyle::default());
    }

    #[test]
    fn fg_and_bg_with_effects_maps_to_modifiers() {
        let ansi = Style::new()
            .fg_color(Some(Color::Rgb(RgbColor(255, 0, 0))))
            .bg_color(Some(Color::Ansi(AnsiColor::Blue)))
            .bold()
            .italic();
        let got = style_into_ratatui(ansi);

        let expected = RStyle::default()
            .fg(RColor::Rgb(255, 0, 0))
            .bg(RColor::Blue)
            .add_modifier(Modifier::BOLD | Modifier::ITALIC);
        assert_eq!(got, expected);
    }

    #[test]
    fn strikethrough_and_dim_and_underline_map() {
        let s = Style::new().strikethrough().dimmed().underline();
        let got = style_into_ratatui(s);

        // Re-derive expected: style_into sets each on add_modifier.
        // Compare color-modifier sum indirectly: 3 unique modifiers means
        // the bytes-set differs from plain, and any of CROSSED_OUT / DIM /
        // UNDERLINED added in combination is non-empty.
        let plain = RStyle::default();
        assert_ne!(got, plain, "effects should not be the default style");
        // Reconstruct with the matching modifier set.
        let expected = RStyle::default().add_modifier(
            Modifier::CROSSED_OUT | Modifier::DIM | Modifier::UNDERLINED,
        );
        assert_eq!(got, expected);
    }

    #[test]
    fn hyperlink_target_constructs_and_equality_works() {
        let a = HyperlinkTarget {
            line_index: 3,
            column_range: 10..24,
            url: "https://example.com/path".to_string(),
            id: 7,
        };
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(a.line_index, 3);
        assert_eq!(a.column_range.start, 10);
        assert_eq!(a.column_range.end, 24);
        assert_eq!(a.url, "https://example.com/path");
        assert_eq!(a.id, 7);
    }
}
