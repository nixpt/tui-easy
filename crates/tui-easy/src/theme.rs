/// An RGB colour — the renderer-agnostic representation.
///
/// Convert to `ratatui::style::Color` or a GUI colour via `From`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl From<Rgb> for ratatui::style::Color {
    fn from(c: Rgb) -> Self {
        ratatui::style::Color::Rgb(c.0, c.1, c.2)
    }
}

/// Base theme colours for a terminal or GUI application.
///
/// Uses the renderer-agnostic [`Rgb`] type so the same palette
/// drives both ratatui and bliss UIs.
#[derive(Debug, Clone)]
pub struct ThemeColors {
    /// Primary background
    pub bg: Rgb,
    /// Default foreground text
    pub fg: Rgb,
    /// Accent / primary brand colour
    pub accent: Rgb,
    /// Destructive / error status
    pub error: Rgb,
    /// Success / ok status
    pub success: Rgb,
    /// Warning / caution status
    pub warn: Rgb,
    /// Muted / dim text
    pub dim: Rgb,
    /// Border / divider colour
    pub border: Rgb,
    /// Highlight / selection background
    pub highlight: Rgb,
    /// Card / surface background
    pub surface: Rgb,
}

impl Default for ThemeColors {
    /// Dark navy/cyan palette — the default theme.
    fn default() -> Self {
        Self {
            bg: Rgb(10, 14, 26),
            fg: Rgb(200, 210, 220),
            accent: Rgb(0, 188, 212),
            error: Rgb(255, 83, 112),
            success: Rgb(76, 175, 80),
            warn: Rgb(255, 193, 7),
            dim: Rgb(100, 110, 125),
            border: Rgb(50, 60, 75),
            highlight: Rgb(30, 40, 60),
            surface: Rgb(15, 20, 35),
        }
    }
}

impl ThemeColors {
    /// Light variant of the palette.
    pub fn light() -> Self {
        Self {
            bg: Rgb(245, 245, 245),
            fg: Rgb(30, 30, 30),
            accent: Rgb(0, 150, 180),
            error: Rgb(200, 40, 60),
            success: Rgb(50, 140, 55),
            warn: Rgb(200, 150, 0),
            dim: Rgb(140, 140, 140),
            border: Rgb(190, 190, 190),
            highlight: Rgb(210, 225, 240),
            surface: Rgb(235, 235, 240),
        }
    }

    /// Convert all colours to ratatui equivalents.
    pub fn to_ratatui(&self) -> RatatuiThemeColors {
        RatatuiThemeColors {
            bg: self.bg.into(),
            fg: self.fg.into(),
            accent: self.accent.into(),
            error: self.error.into(),
            success: self.success.into(),
            warn: self.warn.into(),
            dim: self.dim.into(),
            border: self.border.into(),
            highlight: self.highlight.into(),
            surface: self.surface.into(),
        }
    }
}

/// Ratatui-specific version of [`ThemeColors`] — all fields are
/// `ratatui::style::Color`. Created via [`ThemeColors::to_ratatui`].
#[derive(Debug, Clone)]
pub struct RatatuiThemeColors {
    pub bg: ratatui::style::Color,
    pub fg: ratatui::style::Color,
    pub accent: ratatui::style::Color,
    pub error: ratatui::style::Color,
    pub success: ratatui::style::Color,
    pub warn: ratatui::style::Color,
    pub dim: ratatui::style::Color,
    pub border: ratatui::style::Color,
    pub highlight: ratatui::style::Color,
    pub surface: ratatui::style::Color,
}
