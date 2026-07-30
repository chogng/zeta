use crate::UiScene;

/// Presentation-only contract for a reusable native UI component.
///
/// Implementations translate caller-provided state into scene primitives. The product host remains
/// responsible for layout, input routing, lifecycle, async work, and authoritative domain state.
pub trait Component {
    /// Emits this component's current visual representation into the scene.
    fn paint(&self, scene: &mut UiScene);
}

#[cfg(test)]
#[path = "component_tests.rs"]
mod tests;
