use crate::ConfigError;
use serde::Deserialize;
use serde::Serialize;
use zeta_protocol::Patch;

/// Three-state Desktop preference that may follow the operating system or force a value.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomaticPreference {
    Auto,
    Off,
    On,
}

/// Desktop-only durable preferences stored under `[products.desktop]`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopProductPreferences {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessibility_support: Option<AutomaticPreference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduce_motion: Option<AutomaticPreference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduce_transparency: Option<AutomaticPreference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline_links: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_delay_milliseconds: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_hover_delay_milliseconds: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sash_size: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sash_hover_delay_milliseconds: Option<u16>,
}

/// Zeta Code CLI/TUI preferences stored under `[products.code]`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodeProductPreferences {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_theme: Option<String>,
}

/// Zeterm preferences stored under `[products.zeterm]`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZetermProductPreferences {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_theme: Option<String>,
}

/// Typed product namespaces in the canonical user configuration document.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductsConfig {
    #[serde(default, skip_serializing_if = "DesktopProductPreferences::is_empty")]
    pub desktop: DesktopProductPreferences,
    #[serde(default, skip_serializing_if = "CodeProductPreferences::is_empty")]
    pub code: CodeProductPreferences,
    #[serde(default, skip_serializing_if = "ZetermProductPreferences::is_empty")]
    pub zeterm: ZetermProductPreferences,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopProductPreferencesUpdate {
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub color_theme: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub accessibility_support: Patch<AutomaticPreference>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub reduce_motion: Patch<AutomaticPreference>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub reduce_transparency: Patch<AutomaticPreference>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub underline_links: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub hover_delay_milliseconds: Patch<u16>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub reduced_hover_delay_milliseconds: Patch<u16>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub sash_size: Patch<u8>,
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub sash_hover_delay_milliseconds: Patch<u16>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodeProductPreferencesUpdate {
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub color_theme: Patch<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZetermProductPreferencesUpdate {
    #[serde(default, skip_serializing_if = "Patch::is_missing")]
    pub color_theme: Patch<String>,
}

/// Atomic typed product-preference patch accepted by the Config authority.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductsConfigUpdate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop: Option<DesktopProductPreferencesUpdate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeProductPreferencesUpdate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zeterm: Option<ZetermProductPreferencesUpdate>,
}

impl ProductsConfig {
    pub(crate) fn is_empty(&self) -> bool {
        self.desktop.is_empty() && self.code.is_empty() && self.zeterm.is_empty()
    }

    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        validate_theme(
            "products.desktop.colorTheme",
            self.desktop.color_theme.as_deref(),
        )?;
        validate_theme("products.code.colorTheme", self.code.color_theme.as_deref())?;
        validate_theme(
            "products.zeterm.colorTheme",
            self.zeterm.color_theme.as_deref(),
        )?;
        validate_range(
            "products.desktop.hoverDelayMilliseconds",
            self.desktop.hover_delay_milliseconds,
            0,
            2_000,
        )?;
        validate_range(
            "products.desktop.reducedHoverDelayMilliseconds",
            self.desktop.reduced_hover_delay_milliseconds,
            0,
            2_000,
        )?;
        validate_range("products.desktop.sashSize", self.desktop.sash_size, 1, 20)?;
        validate_range(
            "products.desktop.sashHoverDelayMilliseconds",
            self.desktop.sash_hover_delay_milliseconds,
            0,
            2_000,
        )
    }
}

impl DesktopProductPreferences {
    fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

impl CodeProductPreferences {
    fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

impl ZetermProductPreferences {
    fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

fn validate_theme(path: &str, theme: Option<&str>) -> Result<(), ConfigError> {
    if let Some(theme) = theme
        && (theme.is_empty() || theme.len() > 128 || theme.chars().any(char::is_whitespace))
    {
        return Err(ConfigError(format!(
            "{path} must be one non-empty theme id"
        )));
    }
    Ok(())
}

fn validate_range<T>(
    path: &str,
    value: Option<T>,
    minimum: T,
    maximum: T,
) -> Result<(), ConfigError>
where
    T: Copy + Ord + std::fmt::Display,
{
    if let Some(value) = value
        && (value < minimum || value > maximum)
    {
        return Err(ConfigError(format!(
            "{path} must be between {minimum} and {maximum}"
        )));
    }
    Ok(())
}
