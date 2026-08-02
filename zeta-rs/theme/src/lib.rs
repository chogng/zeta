//! Platform-neutral theme documents, token resolution, and immutable snapshots.

mod catalog;
mod color;
mod document;
mod loader;
mod preference;
pub mod tokens;

pub use catalog::{ThemeCatalog, ThemeError, ThemeSnapshot};
pub use color::Rgba;
pub use document::{ColorScheme, ThemeDocument};
pub use loader::LoadedTheme;
pub use loader::ThemeChoice;
pub use loader::ThemeChoiceKind;
pub use loader::ThemeChoices;
pub use loader::ThemeDiagnostic;
pub use loader::ThemeLoadOptions;
pub use loader::ThemeLoader;
pub use loader::ThemeSurface;
pub use loader::default_device_root;
pub use preference::ThemeSelectionError;

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
