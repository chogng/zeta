use crate::ConfigError;
use serde::Deserialize;
use serde::Serialize;

pub const DEFAULT_TUI_THEME: &str = "system";

/// User preferences owned by terminal products.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TuiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

impl TuiConfig {
    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        if valid_theme(&self.theme) {
            Ok(())
        } else {
            Err(ConfigError(format!(
                "TUI theme '{}' must be lowercase kebab-case",
                self.theme
            )))
        }
    }
}

fn default_theme() -> String {
    DEFAULT_TUI_THEME.into()
}

fn valid_theme(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}
