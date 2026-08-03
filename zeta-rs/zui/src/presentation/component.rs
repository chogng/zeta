use super::ComponentElement;
use super::ComputedElement;
use super::UiScene;

/// Presentation-only contract for a reusable native UI component.
///
/// Implementations translate caller-provided state into scene primitives. The product host remains
/// responsible for layout, input routing, lifecycle, async work, and authoritative domain state.
pub trait Component {
    /// Returns the declarative root element owned by this component.
    ///
    /// Every component declares this root. [`UiScene::draw_component`] resolves it once and shares
    /// the resulting [`ComputedElement`] with paint and inspection, so components cannot silently
    /// omit box metadata or build a second inspection-only geometry path.
    fn element(&self) -> ComponentElement;

    /// Paints from the computed geometry produced for [`Component::element`].
    ///
    /// Implementations with internal layout should override this method and treat `element` as
    /// their sole box geometry source. Simple leaf components may use the default delegation to
    /// [`Component::paint`].
    fn paint_element(&self, scene: &mut UiScene, element: &ComputedElement) {
        let _ = element;
        self.paint(scene);
    }

    /// Emits this component's current visual representation into the scene.
    ///
    /// Product and composition callers should use [`UiScene::draw_component`] instead of invoking
    /// this method directly, so inspectable ancestors are not skipped.
    fn paint(&self, scene: &mut UiScene);
}

impl UiScene {
    /// Resolves and draws one component while automatically registering its element metadata.
    #[track_caller]
    pub fn draw_component<C: Component + ?Sized>(&mut self, component: &C) {
        self.with_element(component.element(), |scene, computed| {
            component.paint_element(scene, computed)
        });
    }
}

#[cfg(test)]
#[path = "component_tests.rs"]
mod tests;
