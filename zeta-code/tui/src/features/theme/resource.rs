use std::path::Path;
use std::path::PathBuf;

use super::ThemePickerCatalog;
use super::ThemePickerChoice;
use super::ThemePickerTarget;
use super::ThemePreviewPalette;
use crate::render::RenderTheme;
use zeta_terminal_detection::BackgroundAppearance;
use zeta_terminal_detection::ColorLevel;
use zeta_terminal_detection::TerminalRgb;
use zeta_terminal_detection::detect_host_terminal;
use zeta_terminal_detection::resolve_background;
use zeta_theme::ColorScheme;
use zeta_theme::ThemeChoiceKind;
use zeta_theme::ThemeLoadOptions;
use zeta_theme::ThemeLoader;
use zeta_theme::ThemeSnapshot;
use zeta_theme::ThemeSurface;
use zeta_theme::default_device_root;

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

#[derive(Clone, Debug)]
pub(crate) struct ThemeResource {
    device_root: PathBuf,
    capability: ColorLevel,
    system_scheme: ColorScheme,
}

pub(crate) struct ThemeLoad {
    pub(crate) theme: RenderTheme,
    pub(crate) diagnostics: Vec<String>,
}

pub(crate) struct ThemeSelection {
    pub(crate) label: String,
    pub(crate) theme: RenderTheme,
}

impl ThemeResource {
    pub(crate) fn new(terminal_background: Option<TerminalRgb>) -> Self {
        Self {
            device_root: default_device_root(),
            capability: detect_host_terminal().color_level,
            system_scheme: detect_system_scheme(terminal_background),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(
        device_root: PathBuf,
        capability: ColorLevel,
        system_scheme: ColorScheme,
    ) -> Self {
        Self {
            device_root,
            capability,
            system_scheme,
        }
    }

    pub(crate) fn load(&self) -> Result<ThemeLoad, String> {
        let loader = ThemeLoader::embedded().map_err(|error| error.to_string())?;
        let loaded = loader.load(self.options());
        let theme = RenderTheme::from_snapshot(&loaded.snapshot, self.capability)
            .map_err(|error| error.to_string())?;
        Ok(ThemeLoad {
            theme,
            diagnostics: loaded
                .diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.message)
                .collect(),
        })
    }

    pub(crate) fn catalog(&self) -> Result<ThemePickerCatalog, String> {
        theme_catalog_at(&self.device_root, self.capability, self.system_scheme)
    }

    pub(crate) fn select(&self, preference: &str) -> Result<ThemeSelection, String> {
        select_theme_at(
            &self.device_root,
            preference,
            self.capability,
            self.system_scheme,
        )
    }

    fn options(&self) -> ThemeLoadOptions<'_> {
        ThemeLoadOptions::new(
            &self.device_root,
            ThemeSurface::Terminal,
            self.system_scheme,
        )
        .with_default_entry(DEFAULT_THEME_ENTRY)
    }
}

fn theme_catalog_at(
    device_root: &Path,
    capability: ColorLevel,
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
    choices.push(ThemePickerChoice {
        label: "Custom color theme".into(),
        palette_label: "User-defined".into(),
        target: ThemePickerTarget::CustomThemes,
        palette: preview_palette(
            RenderTheme::from_snapshot(&current.snapshot, capability)
                .map_err(|error| error.to_string())?,
        ),
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
    capability: ColorLevel,
) -> Result<ThemePickerChoice, String> {
    let loaded = loader
        .preview(options, preference)
        .map_err(|error| error.to_string())?;
    let theme = RenderTheme::from_snapshot(&loaded.snapshot, capability)
        .map_err(|error| error.to_string())?;
    Ok(ThemePickerChoice {
        label: label.into(),
        palette_label: syntax_palette_label(preference, &loaded.snapshot),
        target: ThemePickerTarget::Preference(preference.into()),
        palette: preview_palette(theme),
        selected,
    })
}

fn preview_palette(theme: RenderTheme) -> ThemePreviewPalette {
    ThemePreviewPalette {
        background: theme.background(),
        border: theme.border(),
        foreground: theme.foreground(),
        muted: theme.muted(),
        highlight: theme.highlight(),
        keyword: theme.keyword(),
        string: theme.string(),
        function: theme.function(),
        r#type: theme.r#type(),
        variable: theme.variable(),
        inserted_background: theme.inserted_background(),
        removed_background: theme.removed_background(),
        inserted_marker: theme.inserted_marker(),
        removed_marker: theme.removed_marker(),
    }
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

fn select_theme_at(
    device_root: &Path,
    preference: &str,
    capability: ColorLevel,
    system_scheme: ColorScheme,
) -> Result<ThemeSelection, String> {
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
    Ok(ThemeSelection {
        label: loaded.snapshot.label().to_owned(),
        theme: RenderTheme::from_snapshot(&loaded.snapshot, capability)
            .map_err(|error| error.to_string())?,
    })
}

fn detect_system_scheme(terminal_background: Option<TerminalRgb>) -> ColorScheme {
    match resolve_background(terminal_background).appearance {
        BackgroundAppearance::Dark => ColorScheme::Dark,
        BackgroundAppearance::Light => ColorScheme::Light,
    }
}

#[cfg(test)]
#[path = "resource_tests.rs"]
mod tests;
