//! Renderer-independent icon asset contracts.

mod definition;
mod icon;
mod identity;

pub use definition::IconDefinition;
pub use definition::IconRendering;
pub use icon::Icon;
pub use identity::IconId;

#[cfg(test)]
#[path = "icon_tests.rs"]
mod tests;
