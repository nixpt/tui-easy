//! Integration test for `tornado` + `tornado-styles` (`--features styles`).
//!
//! Guards the bridge end-to-end:
//!  * `tornado::widget::ansi_color` / `ansi_to_style` re-exports the
//!    `tornado-styles` conversion helpers.
//!  * `tornado::widget::styled_line` / `styled_span` layer the bridge on top
//!    of plain text to produce ready-to-render ratatui primitives.
//!  * The `HyperlinkTarget` re-export (`AnsiHyperlinkTarget`) is reachable
//!    from the widget module path.
//!  * A `ThemeColors` baseline + an ANSI override compose without conflict.
//!
//! Compiled only when the `styles` feature is enabled (see
//! `required-features` in `Cargo.toml`), so consumers without `styles` never
//! pull `anstyle` into their build.

#![cfg(feature = "styles")]

use ratatui::style::{Color as RColor, Modifier, Style as RStyle};
use tornado::styles::{AnsiColor, Color as AnsiRgb, RgbColor, Style as AnsiStyle};
use tornado::theme::ThemeColors;
use tornado::widget::{AnsiHyperlinkTarget, ansi_color, ansi_to_style, styled_line, styled_span};

#[test]
fn ansi_to_style_round_trips_fg_bg_and_modifiers() {
    let ansi = AnsiStyle::new()
        .fg_color(Some(AnsiRgb::Rgb(RgbColor(255, 0, 0))))
        .bg_color(Some(AnsiRgb::Ansi(AnsiColor::Blue)))
        .bold()
        .italic();
    let got = ansi_to_style(ansi);
    let expected = RStyle::default()
        .fg(RColor::Rgb(255, 0, 0))
        .bg(RColor::Blue)
        .add_modifier(Modifier::BOLD | Modifier::ITALIC);
    assert_eq!(got, expected);
}

#[test]
fn ansi_color_maps_palette_and_rgb() {
    // Standard palette: White -> anstyle -> ratatui::Gray
    assert_eq!(ansi_color(AnsiRgb::Ansi(AnsiColor::White)), RColor::Gray);
    // RGB passes through unchanged
    assert_eq!(
        ansi_color(AnsiRgb::Rgb(RgbColor(10, 20, 30))),
        RColor::Rgb(10, 20, 30)
    );
}

#[test]
fn styled_span_rewraps_ansi_style_around_plain_text() {
    let ansi = AnsiStyle::new().fg_color(Some(AnsiRgb::Rgb(RgbColor(0, 255, 0))));
    let span = styled_span("ok", ansi);
    assert_eq!(span.content.as_ref(), "ok");
    assert_eq!(span.style.fg, Some(RColor::Rgb(0, 255, 0)));
}

#[test]
fn styled_line_emits_one_styled_span() {
    let ansi = AnsiStyle::new()
        .fg_color(Some(AnsiRgb::Rgb(RgbColor(0, 128, 128))))
        .bold();
    let line = styled_line("hello", ansi);
    assert_eq!(line.spans.len(), 1);
    assert_eq!(line.spans[0].content.as_ref(), "hello");
    assert_eq!(line.spans[0].style.fg, Some(RColor::Rgb(0, 128, 128)));
    assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn theme_baseline_and_ansi_override_compose_independently() {
    // Theme gives us a baseline foreground; the ANSI bridge produces an
    // independent ratatui style that can override the theme colour.
    let theme = ThemeColors::default();
    let theme_fg = theme.to_ratatui().fg;
    let ansi = AnsiStyle::new().fg_color(Some(AnsiRgb::Rgb(RgbColor(255, 100, 50))));
    let bridged = ansi_to_style(ansi);
    assert_eq!(bridged.fg, Some(RColor::Rgb(255, 100, 50)));
    // Sanity: the theme's fg must not equal the bridge's fg override.
    // `bridged.fg` is `Option<Color>`, `theme_fg` is the plain field.
    assert_ne!(bridged.fg, Some(theme_fg));
}

#[test]
fn hyperlink_target_alias_is_constructible_and_type_equal() {
    // 1. Round-trip construction through the alias path proves the alias is
    //    in scope (i.e. `pub use crate::styles::HyperlinkTarget as
    //    AnsiHyperlinkTarget;` is wired).
    let h = AnsiHyperlinkTarget {
        line_index: 1,
        column_range: 2..8,
        url: "https://example.com".to_string(),
        id: 42,
    };
    assert_eq!(h.line_index, 1);
    assert_eq!(h.url, "https://example.com");
    assert_eq!(h.id, 42);

    // 2. Compile-time type-identity: `AnsiHyperlinkTarget` and
    //    `tornado::styles::HyperlinkTarget` must be the exact same type.
    //    If the alias were re-vended (e.g. wrapped), this coercion would
    //    fail to compile. The fn is `_`-prefixed so a dead-code lint
    //    doesn't fire.
    fn _alias_is_type_equal(h: AnsiHyperlinkTarget) -> tornado::styles::HyperlinkTarget {
        h
    }
    let _ = _alias_is_type_equal;
}
