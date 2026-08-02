use std::path::Path;
use std::sync::LazyLock;
use std::sync::RwLock;

use crate::features::theme::ThemePickerCatalog;
use crate::features::theme::ThemePickerChoice;
use crate::features::theme::ThemePickerTarget;
use crate::features::theme::ThemePreviewPalette;
use ratatui::style::Color;
use zeta_theme::ColorScheme;
use zeta_theme::Rgba;
use zeta_theme::ThemeChoiceKind;
use zeta_theme::ThemeError;
use zeta_theme::ThemeLoadOptions;
use zeta_theme::ThemeLoader;
use zeta_theme::ThemeSnapshot;
use zeta_theme::ThemeSurface;
use zeta_theme::default_device_root;
use zeta_theme::tokens;

static ACTIVE_THEME: LazyLock<RwLock<TuiTheme>> =
    LazyLock::new(|| RwLock::new(TuiTheme::fallback()));
const DEFAULT_THEME_ENTRY: &str = "zeta-code";
const ZETA_CODE_THEMES: [(&str, &str); 7] = [
    ("Auto", "system"),
    ("Dark mode", "zeta-code-dark"),
    ("Light mode", "zeta-code-light"),
    (
        "Dark mode (colorblind-friendly)",
        "zeta-code-colorblind-dark",
    ),
    (
        "Light mode (colorblind-friendly)",
        "zeta-code-colorblind-light",
    ),
    ("Dark mode (ANSI colors only)", "zeta-code-ansi-dark"),
    ("Light mode (ANSI colors only)", "zeta-code-ansi-light"),
];

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
    background: Color,
    border: Color,
    composer_chrome: Color,
    danger: Color,
    foreground: Color,
    function: Color,
    highlight: Color,
    inserted_background: Color,
    inserted_marker: Color,
    keyword: Color,
    muted: Color,
    removed_background: Color,
    removed_marker: Color,
    string: Color,
    success: Color,
    r#type: Color,
    variable: Color,
    warning: Color,
}

impl TuiTheme {
    fn from_snapshot(
        snapshot: &ThemeSnapshot,
        capability: TerminalColorCapability,
    ) -> Result<Self, ThemeError> {
        let capability = if snapshot.id().starts_with("zeta-code-ansi-") {
            TerminalColorCapability::Ansi16
        } else {
            capability
        };
        let background = snapshot.required_color(tokens::TERMINAL_BACKGROUND)?;
        let projected = |token| {
            snapshot
                .required_color(token)
                .map(|color| terminal_color(color, background, capability))
        };
        Ok(Self {
            accent: projected(tokens::ACCENT_FOREGROUND)?,
            background: projected(tokens::TERMINAL_BACKGROUND)?,
            border: projected(tokens::BORDER)?,
            composer_chrome: projected(tokens::DESCRIPTION_FOREGROUND)?,
            danger: projected(tokens::ERROR_FOREGROUND)?,
            foreground: projected(tokens::EDITOR_FOREGROUND)?,
            function: projected(tokens::EDITOR_TOKEN_FUNCTION)?,
            highlight: projected(tokens::TUI_HIGHLIGHT_FOREGROUND)?,
            inserted_background: projected(tokens::DIFF_INSERTED_LINE_BACKGROUND)?,
            inserted_marker: projected(tokens::DIFF_INSERTED_LINE_MARKER)?,
            keyword: projected(tokens::EDITOR_TOKEN_KEYWORD)?,
            muted: projected(tokens::MUTED_FOREGROUND)?,
            removed_background: projected(tokens::DIFF_REMOVED_LINE_BACKGROUND)?,
            removed_marker: projected(tokens::DIFF_REMOVED_LINE_MARKER)?,
            string: projected(tokens::EDITOR_TOKEN_STRING)?,
            success: projected(tokens::SUCCESS_FOREGROUND)?,
            r#type: projected(tokens::EDITOR_TOKEN_TYPE)?,
            variable: projected(tokens::EDITOR_TOKEN_VARIABLE)?,
            warning: projected(tokens::WARNING_FOREGROUND)?,
        })
    }

    const fn preview_palette(self) -> ThemePreviewPalette {
        ThemePreviewPalette {
            background: self.background,
            border: self.border,
            foreground: self.foreground,
            muted: self.muted,
            highlight: self.highlight,
            keyword: self.keyword,
            string: self.string,
            function: self.function,
            r#type: self.r#type,
            variable: self.variable,
            inserted_background: self.inserted_background,
            removed_background: self.removed_background,
            inserted_marker: self.inserted_marker,
            removed_marker: self.removed_marker,
        }
    }

    const fn fallback() -> Self {
        Self {
            accent: Color::Rgb(105, 170, 255),
            background: Color::Rgb(13, 17, 23),
            border: Color::DarkGray,
            composer_chrome: Color::Rgb(155, 155, 155),
            foreground: Color::White,
            function: Color::Rgb(210, 168, 255),
            highlight: Color::Rgb(154, 145, 235),
            inserted_background: Color::Rgb(19, 48, 28),
            inserted_marker: Color::Rgb(63, 185, 80),
            keyword: Color::Rgb(255, 123, 114),
            muted: Color::DarkGray,
            removed_background: Color::Rgb(55, 25, 27),
            removed_marker: Color::Rgb(248, 81, 73),
            string: Color::Rgb(165, 214, 255),
            success: Color::Rgb(95, 210, 140),
            r#type: Color::Rgb(210, 168, 255),
            variable: Color::Rgb(255, 166, 87),
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
    let loaded = loader.load(
        ThemeLoadOptions::new(&device_root, ThemeSurface::Terminal, detect_system_scheme())
            .with_default_entry(DEFAULT_THEME_ENTRY),
    );
    for diagnostic in &loaded.diagnostics {
        eprintln!("theme: {}", diagnostic.message);
    }
    let Ok(theme) = TuiTheme::from_snapshot(&loaded.snapshot, TerminalColorCapability::detect())
    else {
        return;
    };
    set_active(theme);
}

pub(crate) fn theme_catalog() -> Result<ThemePickerCatalog, String> {
    theme_catalog_at(
        &default_device_root(),
        TerminalColorCapability::detect(),
        detect_system_scheme(),
    )
}

fn theme_catalog_at(
    device_root: &Path,
    capability: TerminalColorCapability,
    system_scheme: ColorScheme,
) -> Result<ThemePickerCatalog, String> {
    let loader = ThemeLoader::embedded().map_err(|error| error.to_string())?;
    let options = ThemeLoadOptions::new(device_root, ThemeSurface::Terminal, system_scheme)
        .with_default_entry(DEFAULT_THEME_ENTRY);
    let available = loader.choices(options);
    let selected_is_custom = available
        .themes
        .iter()
        .any(|theme| theme.kind == ThemeChoiceKind::User && theme.id == available.selected);
    let mut choices = ZETA_CODE_THEMES
        .into_iter()
        .map(|(label, preference)| {
            preview_choice(
                &loader,
                options,
                label,
                preference,
                available.selected == preference,
                capability,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let current = loader.load(options);
    let current_palette = TuiTheme::from_snapshot(&current.snapshot, capability)
        .map_err(|error| error.to_string())?
        .preview_palette();
    choices.push(ThemePickerChoice {
        label: "Custom color theme".into(),
        palette_label: "User-defined".into(),
        target: ThemePickerTarget::CustomThemes,
        palette: current_palette,
        selected: selected_is_custom,
    });
    let custom_choices = available
        .themes
        .iter()
        .filter(|theme| theme.kind == ThemeChoiceKind::User)
        .map(|theme| {
            preview_choice(
                &loader,
                options,
                &theme.label,
                &theme.id,
                available.selected == theme.id,
                capability,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ThemePickerCatalog {
        choices,
        custom_choices,
    })
}

fn preview_choice(
    loader: &ThemeLoader,
    options: ThemeLoadOptions<'_>,
    label: &str,
    preference: &str,
    selected: bool,
    capability: TerminalColorCapability,
) -> Result<ThemePickerChoice, String> {
    let loaded = loader
        .preview(options, preference)
        .map_err(|error| error.to_string())?;
    let palette = TuiTheme::from_snapshot(&loaded.snapshot, capability)
        .map_err(|error| error.to_string())?
        .preview_palette();
    let palette_label = syntax_palette_label(preference, &loaded.snapshot);
    Ok(ThemePickerChoice {
        label: label.into(),
        palette_label,
        target: ThemePickerTarget::Preference(preference.into()),
        palette,
        selected,
    })
}

fn syntax_palette_label(preference: &str, snapshot: &ThemeSnapshot) -> String {
    let scheme = match snapshot.color_scheme() {
        ColorScheme::Dark | ColorScheme::HighContrastDark => "Dark",
        ColorScheme::Light | ColorScheme::HighContrastLight => "Light",
    };
    if preference.starts_with("zeta-code-colorblind-") {
        format!("GitHub {scheme} Colorblind")
    } else if preference.starts_with("zeta-code-ansi-") {
        format!("GitHub {scheme} · ANSI 16 colors")
    } else if preference == "system" || preference.starts_with("zeta-code-") {
        format!("GitHub {scheme}")
    } else {
        format!("User-defined · {}", snapshot.label())
    }
}

pub(crate) fn select_theme(preference: &str) -> Result<(), String> {
    select_theme_at(
        &default_device_root(),
        preference,
        TerminalColorCapability::detect(),
        detect_system_scheme(),
    )
}

fn select_theme_at(
    device_root: &Path,
    preference: &str,
    capability: TerminalColorCapability,
    system_scheme: ColorScheme,
) -> Result<(), String> {
    let loader = ThemeLoader::embedded().map_err(|error| error.to_string())?;
    let options = ThemeLoadOptions::new(device_root, ThemeSurface::Terminal, system_scheme)
        .with_default_entry(DEFAULT_THEME_ENTRY);
    if preference.is_empty() || preference.split_whitespace().count() != 1 {
        return Err("usage: /theme <theme-id>".into());
    }
    let available = loader.choices(options);
    let supported = ZETA_CODE_THEMES
        .iter()
        .any(|(_, candidate)| *candidate == preference)
        || available
            .themes
            .iter()
            .any(|theme| theme.kind == ThemeChoiceKind::User && theme.id == preference);
    if !supported {
        return Err(format!("theme '{preference}' is not a Zeta Code theme"));
    }
    let loaded = loader
        .select(options, preference)
        .map_err(|error| error.to_string())?;
    let theme =
        TuiTheme::from_snapshot(&loaded.snapshot, capability).map_err(|error| error.to_string())?;
    set_active(theme);
    Ok(())
}

fn set_active(theme: TuiTheme) {
    *ACTIVE_THEME
        .write()
        .expect("TUI theme lock should not be poisoned") = theme;
}

pub(crate) fn accent() -> Color {
    active().accent
}

pub(crate) fn background() -> Color {
    active().background
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

pub(crate) fn foreground() -> Color {
    active().foreground
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

fn detect_system_scheme() -> ColorScheme {
    std::env::var("COLORFGBG")
        .ok()
        .as_deref()
        .and_then(scheme_from_colorfgbg)
        .unwrap_or(ColorScheme::Dark)
}

fn scheme_from_colorfgbg(value: &str) -> Option<ColorScheme> {
    let background = value.rsplit(';').next()?.trim().parse::<u8>().ok()? % 16;
    Some(if matches!(background, 7 | 10..=15) {
        ColorScheme::Light
    } else {
        ColorScheme::Dark
    })
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
