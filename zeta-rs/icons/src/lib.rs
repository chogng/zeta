//! Renderer-independent product icon identities and embedded SVG definitions.

mod generated;
mod library;

pub use library::{ALL_ICONS, icons};

/// Stable semantic identity of a product icon.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IconId(&'static str);

impl IconId {
    /// Creates a stable lowercase kebab-case icon identifier.
    ///
    /// # Panics
    ///
    /// Panics when `value` is empty or is not lowercase kebab-case ASCII.
    pub const fn new(value: &'static str) -> Self {
        assert!(
            valid_icon_id(value),
            "icon ID must be lowercase kebab-case ASCII"
        );
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

const fn valid_icon_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() {
        return false;
    }
    let mut index = 1;
    while index < bytes.len() {
        let byte = bytes[index];
        if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || (byte == b'-' && bytes[index - 1] == b'-')
        {
            return false;
        }
        index += 1;
    }
    bytes[bytes.len() - 1] != b'-'
}

/// Rendering contract declared by an SVG definition.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IconRendering {
    /// Replace the SVG's symbolic coverage with a caller-provided color.
    Symbolic,
    /// Preserve fixed SVG colors while allowing black symbolic regions to follow caller tint.
    Multicolor,
}

/// Immutable SVG artwork and its rendering contract.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IconDefinition {
    svg: &'static [u8],
    rendering: IconRendering,
}

impl IconDefinition {
    pub const fn symbolic(svg: &'static [u8]) -> Self {
        Self {
            svg,
            rendering: IconRendering::Symbolic,
        }
    }

    pub const fn multicolor(svg: &'static [u8]) -> Self {
        Self {
            svg,
            rendering: IconRendering::Multicolor,
        }
    }

    pub const fn svg(self) -> &'static [u8] {
        self.svg
    }

    pub const fn rendering(self) -> IconRendering {
        self.rendering
    }
}

/// Copyable semantic icon reference with an embedded default SVG definition.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Icon {
    id: IconId,
    definition: IconDefinition,
}

impl Icon {
    pub const fn new(id: IconId, definition: IconDefinition) -> Self {
        Self { id, definition }
    }

    pub const fn id(self) -> IconId {
        self.id
    }

    pub const fn definition(self) -> IconDefinition {
        self.definition
    }
}

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
