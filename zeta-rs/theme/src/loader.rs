use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{ColorScheme, ThemeCatalog, ThemeDocument, ThemeSnapshot};

const MAX_THEME_FILES: usize = 128;
const MAX_DEVICE_DOCUMENT_BYTES: u64 = 1_048_576;

/// Resolves the host-local UI preference root without consulting Agent configuration.
pub fn default_device_root() -> PathBuf {
    if let Some(root) = std::env::var_os("ZETA_DEVICE_ROOT") {
        return PathBuf::from(root);
    }
    platform_device_root().unwrap_or_else(|| PathBuf::from(".zeta-device"))
}

#[cfg(target_os = "windows")]
fn platform_device_root() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("Zeta"))
}

#[cfg(target_os = "macos")]
fn platform_device_root() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|root| root.join("Library/Application Support/Zeta"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_device_root() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .map(|root| root.join("zeta"))
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|root| root.join(".config/zeta"))
        })
}

#[cfg(not(any(unix, target_os = "windows")))]
fn platform_device_root() -> Option<PathBuf> {
    None
}

/// Presentation surface selecting a device-local theme preference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeSurface {
    Graphical,
    Terminal,
}

/// Named inputs for loading one surface's selected theme.
pub struct ThemeLoadOptions<'a> {
    pub device_root: &'a Path,
    pub surface: ThemeSurface,
    pub system_scheme: ColorScheme,
    pub default_entry: &'a str,
}

impl<'a> ThemeLoadOptions<'a> {
    pub const fn new(
        device_root: &'a Path,
        surface: ThemeSurface,
        system_scheme: ColorScheme,
    ) -> Self {
        Self {
            device_root,
            surface,
            system_scheme,
            default_entry: "zeta",
        }
    }

    /// Selects the built-in entry used when the device preference follows the system.
    pub const fn with_default_entry(mut self, default_entry: &'a str) -> Self {
        self.default_entry = default_entry;
        self
    }
}

/// One isolated theme-loading diagnostic that does not invalidate other files.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThemeDiagnostic {
    pub file: Option<PathBuf>,
    pub message: String,
}

/// Selected snapshot plus non-fatal discovery and parsing diagnostics.
pub struct LoadedTheme {
    pub snapshot: ThemeSnapshot,
    pub diagnostics: Vec<ThemeDiagnostic>,
    pub follows_system: bool,
}

/// Bounded loader for shared device preferences and user theme JSON files.
pub struct ThemeLoader {
    catalog: ThemeCatalog,
}

impl ThemeLoader {
    pub fn embedded() -> Result<Self, crate::ThemeError> {
        Ok(Self {
            catalog: ThemeCatalog::embedded()?,
        })
    }

    pub fn load(&self, options: ThemeLoadOptions<'_>) -> LoadedTheme {
        let mut diagnostics = Vec::new();
        let preference = read_preference(&options, &mut diagnostics);
        if preference == "system" {
            return LoadedTheme {
                snapshot: self.default_snapshot(&options, &mut diagnostics),
                diagnostics,
                follows_system: true,
            };
        }
        if let Some(snapshot) = self
            .catalog
            .resolve_built_in_id(&preference)
            .expect("embedded theme entries must resolve named light and dark variants")
        {
            return LoadedTheme {
                snapshot,
                diagnostics,
                follows_system: false,
            };
        }
        let documents = read_theme_documents(options.device_root, &self.catalog, &mut diagnostics);
        let Some(document) = documents.get(&preference) else {
            diagnostics.push(ThemeDiagnostic {
                file: None,
                message: format!(
                    "selected theme '{preference}' is unavailable; using default theme entry"
                ),
            });
            return LoadedTheme {
                snapshot: self.default_snapshot(&options, &mut diagnostics),
                diagnostics,
                follows_system: true,
            };
        };
        match self.catalog.resolve_document(document) {
            Ok(snapshot) => LoadedTheme {
                snapshot,
                diagnostics,
                follows_system: false,
            },
            Err(error) => {
                diagnostics.push(ThemeDiagnostic {
                    file: None,
                    message: format!("selected theme '{preference}' is invalid: {error}"),
                });
                LoadedTheme {
                    snapshot: self.default_snapshot(&options, &mut diagnostics),
                    diagnostics,
                    follows_system: true,
                }
            }
        }
    }

    fn default_snapshot(
        &self,
        options: &ThemeLoadOptions<'_>,
        diagnostics: &mut Vec<ThemeDiagnostic>,
    ) -> ThemeSnapshot {
        match self
            .catalog
            .built_in_entry(options.default_entry, options.system_scheme)
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                diagnostics.push(ThemeDiagnostic {
                    file: None,
                    message: format!(
                        "default theme entry '{}' is unavailable: {error}; using zeta",
                        options.default_entry
                    ),
                });
                self.catalog
                    .built_in(options.system_scheme)
                    .expect("embedded theme catalog must resolve the zeta entry")
            }
        }
    }
}

fn read_preference(
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
    match options.surface {
        ThemeSurface::Graphical => document.values.workbench_color_theme,
        ThemeSurface::Terminal => document
            .values
            .tui_color_theme
            .or(document.values.workbench_color_theme),
    }
    .unwrap_or_else(|| "system".to_owned())
}

fn read_theme_documents(
    device_root: &Path,
    catalog: &ThemeCatalog,
    diagnostics: &mut Vec<ThemeDiagnostic>,
) -> BTreeMap<String, ThemeDocument> {
    let directory = device_root.join("themes");
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return BTreeMap::new(),
        Err(error) => {
            diagnostics.push(ThemeDiagnostic {
                file: Some(directory),
                message: format!("could not read user theme directory: {error}"),
            });
            return BTreeMap::new();
        }
    };
    let mut files = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            (file_type.is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    == Some("json"))
            .then(|| entry.path())
        })
        .collect::<Vec<_>>();
    files.sort();
    files.truncate(MAX_THEME_FILES);
    let mut documents = BTreeMap::new();
    for path in files {
        let source = match read_bounded_text(&path) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(ThemeDiagnostic {
                    file: Some(path),
                    message: format!("could not read user theme: {error}"),
                });
                continue;
            }
        };
        let document = match ThemeDocument::parse(&source) {
            Ok(document) => document,
            Err(error) => {
                diagnostics.push(ThemeDiagnostic {
                    file: Some(path),
                    message: format!("user theme is invalid: {error}"),
                });
                continue;
            }
        };
        if catalog.is_reserved_theme_id(document.id()) || documents.contains_key(document.id()) {
            diagnostics.push(ThemeDiagnostic {
                file: Some(path),
                message: format!("duplicate or reserved user theme id '{}'", document.id()),
            });
            continue;
        }
        documents.insert(document.id().to_owned(), document);
    }
    documents
}

fn read_bounded_text(path: &Path) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_DEVICE_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DEVICE_DOCUMENT_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "document exceeds the 1 MiB limit",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceConfiguration {
    version: u8,
    values: DeviceValues,
}

#[derive(Deserialize)]
struct DeviceValues {
    #[serde(rename = "workbench.colorTheme")]
    workbench_color_theme: Option<String>,
    #[serde(rename = "tui.colorTheme")]
    tui_color_theme: Option<String>,
}
