use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use zeta_utils_path::resolve_symlink_write_paths;
use zeta_utils_path::write_text_atomically;

use crate::ThemeError;
use crate::loader::ThemeDiagnostic;
use crate::loader::ThemeLoadOptions;
use crate::loader::read_bounded_text;

/// Failure to validate or persist a graphical theme selection.
#[derive(Debug, Error)]
pub enum ThemeSelectionError {
    #[error("invalid theme preference '{0}'")]
    InvalidPreference(String),
    #[error("theme '{0}' is unavailable")]
    Unavailable(String),
    #[error("theme '{preference}' is invalid: {source}")]
    InvalidTheme {
        preference: String,
        #[source]
        source: ThemeError,
    },
    #[error("could not read device configuration: {0}")]
    ReadConfiguration(#[source] std::io::Error),
    #[error("device configuration is invalid: {0}")]
    InvalidConfiguration(#[source] serde_json::Error),
    #[error("device configuration version {0} is unsupported")]
    UnsupportedConfigurationVersion(u8),
    #[error("could not preserve the device configuration symlink safely")]
    UnsafeConfigurationPath,
    #[error("could not encode device configuration: {0}")]
    EncodeConfiguration(#[source] serde_json::Error),
    #[error("could not persist device configuration: {0}")]
    PersistConfiguration(#[source] std::io::Error),
}

pub(crate) fn read_preference(
    options: &ThemeLoadOptions<'_>,
    diagnostics: &mut Vec<ThemeDiagnostic>,
) -> String {
    let path = options.device_root.join("configuration.json");
    let source = match read_bounded_text(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return "system".to_owned(),
        Err(error) => {
            diagnostics.push(ThemeDiagnostic {
                file: Some(path),
                message: format!("could not read device configuration: {error}"),
            });
            return "system".to_owned();
        }
    };
    let document: DeviceConfiguration = match serde_json::from_str(&source) {
        Ok(document) => document,
        Err(error) => {
            diagnostics.push(ThemeDiagnostic {
                file: Some(path),
                message: format!("device configuration is invalid: {error}"),
            });
            return "system".to_owned();
        }
    };
    if document.version != 1 {
        diagnostics.push(ThemeDiagnostic {
            file: Some(path),
            message: format!(
                "device configuration version {} is unsupported",
                document.version
            ),
        });
        return "system".to_owned();
    }
    let preference = document.string_value("workbench.colorTheme");
    match preference {
        Some(Ok(preference)) => preference.to_owned(),
        Some(Err(key)) => {
            diagnostics.push(ThemeDiagnostic {
                file: Some(path),
                message: format!("device configuration value '{key}' must be a string"),
            });
            "system".to_owned()
        }
        None => "system".to_owned(),
    }
}

pub(crate) fn write_preference(
    device_root: &Path,
    preference: &str,
) -> Result<(), ThemeSelectionError> {
    let configured_path = device_root.join("configuration.json");
    let paths = resolve_symlink_write_paths(&configured_path);
    let read_path = paths
        .read_path
        .ok_or(ThemeSelectionError::UnsafeConfigurationPath)?;
    let mut document = match read_bounded_text(&read_path) {
        Ok(source) => serde_json::from_str::<DeviceConfiguration>(&source)
            .map_err(ThemeSelectionError::InvalidConfiguration)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            DeviceConfiguration::default()
        }
        Err(error) => return Err(ThemeSelectionError::ReadConfiguration(error)),
    };
    if document.version != 1 {
        return Err(ThemeSelectionError::UnsupportedConfigurationVersion(
            document.version,
        ));
    }
    document.values.insert(
        "workbench.colorTheme".to_owned(),
        Value::String(preference.to_owned()),
    );
    let mut encoded = serde_json::to_string_pretty(&document)
        .map_err(ThemeSelectionError::EncodeConfiguration)?;
    encoded.push('\n');
    write_text_atomically(&paths.write_path, &encoded)
        .map_err(ThemeSelectionError::PersistConfiguration)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DeviceConfiguration {
    version: u8,
    values: BTreeMap<String, Value>,
}

impl DeviceConfiguration {
    fn string_value(&self, key: &'static str) -> Option<Result<&str, &'static str>> {
        self.values.get(key).map(|value| value.as_str().ok_or(key))
    }
}

impl Default for DeviceConfiguration {
    fn default() -> Self {
        Self {
            version: 1,
            values: BTreeMap::new(),
        }
    }
}
