use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use crate::ColorScheme;
use crate::ThemeCatalog;
use crate::ThemeDocument;
use crate::ThemeSnapshot;
use crate::preference::ThemeSelectionError;
use crate::preference::read_preference;
use crate::preference::write_preference;

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
#[derive(Clone, Copy)]
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
#[derive(Debug)]
pub struct LoadedTheme {
    pub snapshot: ThemeSnapshot,
    pub diagnostics: Vec<ThemeDiagnostic>,
    pub follows_system: bool,
}

/// Origin of one theme exposed to a presentation-surface picker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeChoiceKind {
    System,
    BuiltIn,
    User,
}

/// One valid theme preference available to a presentation surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThemeChoice {
    pub id: String,
    pub label: String,
    pub color_scheme: ColorScheme,
    pub kind: ThemeChoiceKind,
}

/// Available theme preferences and the effective device-local selection.
pub struct ThemeChoices {
    pub selected: String,
    pub themes: Vec<ThemeChoice>,
    pub diagnostics: Vec<ThemeDiagnostic>,
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
        match self.resolve_preference(&options, &preference, &mut diagnostics) {
            Ok((snapshot, follows_system)) => LoadedTheme {
                snapshot,
                diagnostics,
                follows_system,
            },
            Err(error) => {
                diagnostics.push(ThemeDiagnostic {
                    file: None,
                    message: format!("{error}; using default theme entry"),
                });
                LoadedTheme {
                    snapshot: self.default_snapshot(&options, &mut diagnostics),
                    diagnostics,
                    follows_system: true,
                }
            }
        }
    }

    /// Lists valid built-in and user theme preferences for one presentation surface.
    pub fn choices(&self, options: ThemeLoadOptions<'_>) -> ThemeChoices {
        let mut diagnostics = Vec::new();
        let selected = read_preference(&options, &mut diagnostics);
        self.choices_with_selected(options, selected, diagnostics)
    }

    /// Lists themes while using a product Config preference as the selected value.
    pub fn choices_for_preference(
        &self,
        options: ThemeLoadOptions<'_>,
        preference: &str,
    ) -> ThemeChoices {
        self.choices_with_selected(options, preference.to_owned(), Vec::new())
    }

    fn choices_with_selected(
        &self,
        options: ThemeLoadOptions<'_>,
        selected: String,
        mut diagnostics: Vec<ThemeDiagnostic>,
    ) -> ThemeChoices {
        let system = self.default_snapshot(&options, &mut diagnostics);
        let mut themes = vec![ThemeChoice {
            id: "system".into(),
            label: format!("System ({})", system.label()),
            color_scheme: system.color_scheme(),
            kind: ThemeChoiceKind::System,
        }];
        themes.extend(
            self.catalog
                .built_in_themes()
                .into_iter()
                .map(|snapshot| ThemeChoice {
                    id: snapshot.id().to_owned(),
                    label: snapshot.label().to_owned(),
                    color_scheme: snapshot.color_scheme(),
                    kind: ThemeChoiceKind::BuiltIn,
                }),
        );
        let documents = read_theme_documents(options.device_root, &self.catalog, &mut diagnostics);
        for document in documents.values() {
            match self.catalog.resolve_document(document) {
                Ok(snapshot) => themes.push(ThemeChoice {
                    id: snapshot.id().to_owned(),
                    label: snapshot.label().to_owned(),
                    color_scheme: snapshot.color_scheme(),
                    kind: ThemeChoiceKind::User,
                }),
                Err(error) => diagnostics.push(ThemeDiagnostic {
                    file: None,
                    message: format!("user theme '{}' is invalid: {error}", document.id()),
                }),
            }
        }
        ThemeChoices {
            selected,
            themes,
            diagnostics,
        }
    }

    /// Validates, persists, and resolves one surface-specific theme preference.
    pub fn select(
        &self,
        options: ThemeLoadOptions<'_>,
        preference: &str,
    ) -> Result<LoadedTheme, ThemeSelectionError> {
        let loaded = self.preview(options, preference)?;
        write_preference(options.device_root, options.surface, preference)?;
        Ok(loaded)
    }

    /// Resolves one theme preference without changing the device configuration.
    pub fn preview(
        &self,
        options: ThemeLoadOptions<'_>,
        preference: &str,
    ) -> Result<LoadedTheme, ThemeSelectionError> {
        if preference != "system" && !crate::document::valid_theme_id(preference) {
            return Err(ThemeSelectionError::InvalidPreference(
                preference.to_owned(),
            ));
        }
        let mut diagnostics = Vec::new();
        let (snapshot, follows_system) =
            self.resolve_preference(&options, preference, &mut diagnostics)?;
        Ok(LoadedTheme {
            snapshot,
            diagnostics,
            follows_system,
        })
    }

    fn resolve_preference(
        &self,
        options: &ThemeLoadOptions<'_>,
        preference: &str,
        diagnostics: &mut Vec<ThemeDiagnostic>,
    ) -> Result<(ThemeSnapshot, bool), ThemeSelectionError> {
        if preference == "system" {
            return Ok((self.default_snapshot(options, diagnostics), true));
        }
        if let Some(snapshot) = self
            .catalog
            .resolve_built_in_id(preference)
            .expect("embedded theme entries must resolve named light and dark variants")
        {
            return Ok((snapshot, false));
        }
        let documents = read_theme_documents(options.device_root, &self.catalog, diagnostics);
        let document = documents
            .get(preference)
            .ok_or_else(|| ThemeSelectionError::Unavailable(preference.to_owned()))?;
        self.catalog
            .resolve_document(document)
            .map(|snapshot| (snapshot, false))
            .map_err(|source| ThemeSelectionError::InvalidTheme {
                preference: preference.to_owned(),
                source,
            })
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

pub(crate) fn read_bounded_text(path: &Path) -> std::io::Result<String> {
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
