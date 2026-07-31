use std::sync::{LazyLock, RwLock};

use ratatui::style::Color;
use zeta_theme::{
    ColorScheme, Rgba, ThemeError, ThemeLoadOptions, ThemeLoader, ThemeSnapshot, ThemeSurface,
    default_device_root, tokens,
};

static ACTIVE_THEME: LazyLock<RwLock<TuiTheme>> =
    LazyLock::new(|| RwLock::new(TuiTheme::fallback()));

/// Terminal color fidelity available to the TUI presentation adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalColorCapability {
    TrueColor,
    Ansi256,
    Ansi16,
    Monochrome,
}

impl TerminalColorCapability {
    fn detect() -> Self {
        if std::env::var_os("NO_COLOR").is_some()
            || std::env::var("CLICOLOR").is_ok_and(|value| value == "0")
        {
            return Self::Monochrome;
        }
        let color_term = std::env::var("COLORTERM")
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(color_term.as_str(), "truecolor" | "24bit") {
            return Self::TrueColor;
        }
        let term = std::env::var("TERM")
            .unwrap_or_default()
            .to_ascii_lowercase();
        if term == "dumb" {
            Self::Monochrome
        } else if term.contains("256color") {
            Self::Ansi256
        } else {
            Self::Ansi16
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TuiTheme {
    accent: Color,
    composer_chrome: Color,
    danger: Color,
    highlight: Color,
    muted: Color,
    success: Color,
    warning: Color,
}

impl TuiTheme {
    fn from_snapshot(
        snapshot: &ThemeSnapshot,
        capability: TerminalColorCapability,
    ) -> Result<Self, ThemeError> {
        let background = snapshot.required_color(tokens::TERMINAL_BACKGROUND)?;
        let projected = |token| {
            snapshot
                .required_color(token)
                .map(|color| terminal_color(color, background, capability))
        };
        Ok(Self {
            accent: projected(tokens::ACCENT_FOREGROUND)?,
            composer_chrome: projected(tokens::DESCRIPTION_FOREGROUND)?,
            danger: projected(tokens::ERROR_FOREGROUND)?,
            highlight: projected(tokens::EDITOR_TOKEN_KEYWORD)?,
            muted: projected(tokens::MUTED_FOREGROUND)?,
            success: projected(tokens::SUCCESS_FOREGROUND)?,
            warning: projected(tokens::WARNING_FOREGROUND)?,
        })
    }

    const fn fallback() -> Self {
        Self {
            accent: Color::Rgb(105, 170, 255),
            composer_chrome: Color::Rgb(155, 155, 155),
            highlight: Color::Rgb(154, 145, 235),
            muted: Color::DarkGray,
            success: Color::Rgb(95, 210, 140),
            warning: Color::Rgb(245, 190, 80),
            danger: Color::Rgb(245, 105, 105),
        }
    }
}

pub(crate) fn configure() {
    let Ok(loader) = ThemeLoader::embedded() else {
        return;
    };
    let device_root = default_device_root();
    let loaded = loader.load(ThemeLoadOptions::new(
        &device_root,
        ThemeSurface::Terminal,
        ColorScheme::Dark,
    ));
    for diagnostic in &loaded.diagnostics {
        eprintln!("theme: {}", diagnostic.message);
    }
    let Ok(theme) = TuiTheme::from_snapshot(&loaded.snapshot, TerminalColorCapability::detect())
    else {
        return;
    };
    *ACTIVE_THEME
        .write()
        .expect("TUI theme lock should not be poisoned") = theme;
}

pub(crate) fn accent() -> Color {
    active().accent
}

pub(crate) fn composer_chrome() -> Color {
    active().composer_chrome
}

pub(crate) fn danger() -> Color {
    active().danger
}

pub(crate) fn highlight() -> Color {
    active().highlight
}

pub(crate) fn muted() -> Color {
    active().muted
}

pub(crate) fn success() -> Color {
    active().success
}

pub(crate) fn warning() -> Color {
    active().warning
}

fn active() -> TuiTheme {
    *ACTIVE_THEME
        .read()
        .expect("TUI theme lock should not be poisoned")
}

fn terminal_color(
    foreground: Rgba,
    background: Rgba,
    capability: TerminalColorCapability,
) -> Color {
    let rgb = composite(foreground, background);
    match capability {
        TerminalColorCapability::TrueColor => Color::Rgb(rgb[0], rgb[1], rgb[2]),
        TerminalColorCapability::Ansi256 => Color::Indexed(nearest_ansi256(rgb)),
        TerminalColorCapability::Ansi16 => nearest_ansi16(rgb),
        TerminalColorCapability::Monochrome => Color::Reset,
    }
}

fn composite(foreground: Rgba, background: Rgba) -> [u8; 3] {
    let [red, green, blue, alpha] = foreground.components();
    if alpha == 255 {
        return [red, green, blue];
    }
    let [background_red, background_green, background_blue, _] = background.components();
    let blend = |foreground: u8, background: u8| {
        let alpha = u32::from(alpha);
        ((u32::from(foreground) * alpha + u32::from(background) * (255 - alpha) + 127) / 255) as u8
    };
    [
        blend(red, background_red),
        blend(green, background_green),
        blend(blue, background_blue),
    ]
}

fn nearest_ansi256(rgb: [u8; 3]) -> u8 {
    let mut best = (0_u8, u32::MAX);
    for index in 16_u8..=255 {
        let candidate = ansi256_rgb(index);
        let distance = color_distance(rgb, candidate);
        if distance < best.1 {
            best = (index, distance);
        }
    }
    best.0
}

fn ansi256_rgb(index: u8) -> [u8; 3] {
    if index >= 232 {
        let gray = 8 + (index - 232) * 10;
        return [gray, gray, gray];
    }
    let cube = index - 16;
    let channel = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
    [
        channel(cube / 36),
        channel((cube % 36) / 6),
        channel(cube % 6),
    ]
}

fn nearest_ansi16(rgb: [u8; 3]) -> Color {
    const COLORS: [([u8; 3], Color); 16] = [
        ([0, 0, 0], Color::Black),
        ([128, 0, 0], Color::Red),
        ([0, 128, 0], Color::Green),
        ([128, 128, 0], Color::Yellow),
        ([0, 0, 128], Color::Blue),
        ([128, 0, 128], Color::Magenta),
        ([0, 128, 128], Color::Cyan),
        ([192, 192, 192], Color::Gray),
        ([128, 128, 128], Color::DarkGray),
        ([255, 0, 0], Color::LightRed),
        ([0, 255, 0], Color::LightGreen),
        ([255, 255, 0], Color::LightYellow),
        ([0, 0, 255], Color::LightBlue),
        ([255, 0, 255], Color::LightMagenta),
        ([0, 255, 255], Color::LightCyan),
        ([255, 255, 255], Color::White),
    ];
    COLORS
        .iter()
        .min_by_key(|(candidate, _)| color_distance(rgb, *candidate))
        .map(|(_, color)| *color)
        .unwrap_or(Color::Reset)
}

fn color_distance(left: [u8; 3], right: [u8; 3]) -> u32 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| {
            let difference = i32::from(left) - i32::from(right);
            difference.unsigned_abs().pow(2)
        })
        .sum()
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
