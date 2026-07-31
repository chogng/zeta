//! Platform-neutral theme documents, token resolution, and immutable snapshots.

mod catalog;
mod color;
mod document;
mod loader;
pub mod tokens;

pub use catalog::{ThemeCatalog, ThemeError, ThemeSnapshot};
pub use color::Rgba;
pub use document::{ColorScheme, ThemeDocument};
pub use loader::{
    LoadedTheme, ThemeDiagnostic, ThemeLoadOptions, ThemeLoader, ThemeSurface, default_device_root,
};

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
