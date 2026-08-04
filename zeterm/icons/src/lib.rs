//! Renderer-independent product icon identities and embedded SVG definitions.

mod generated;
mod library;

pub use library::{ALL_ICONS, icons};
pub use zeta_icon::Icon;
pub use zeta_icon::IconDefinition;
pub use zeta_icon::IconId;
pub use zeta_icon::IconRendering;

/// Resolves a product icon from the explicit semantic library.
pub fn icon_by_id(id: &str) -> Option<Icon> {
    ALL_ICONS
        .binary_search_by_key(&id, |icon| icon.id().as_str())
        .ok()
        .map(|index| ALL_ICONS[index])
}

#[cfg(test)]
#[path = "icon_tests.rs"]
mod tests;
