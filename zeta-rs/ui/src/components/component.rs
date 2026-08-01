use crate::{CornerRadii, Edges, InspectionNode, Rect, UiScene};

/// Optional inspection metadata declared by a reusable component.
///
/// Box-owning components should return [`ComponentInspection::new`] with the same resolved
/// geometry used for paint. Primitive-only helpers can keep [`ComponentInspection::NONE`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComponentInspection(Option<InspectionNode>);

impl ComponentInspection {
    /// Declares that a component has no independent box geometry to inspect.
    pub const NONE: Self = Self(None);

    /// Describes an inspectable component with its resolved name and bounds.
    pub const fn new(name: &'static str, bounds: Rect) -> Self {
        Self(Some(InspectionNode::new(name, bounds)))
    }

    /// Adds the component-owned padding used by paint and content layout.
    pub const fn with_padding(self, padding: Edges) -> Self {
        match self.0 {
            Some(node) => Self(Some(node.with_padding(padding))),
            None => self,
        }
    }

    /// Adds the component-owned corner radii used by paint.
    pub const fn with_corner_radii(self, corner_radii: CornerRadii) -> Self {
        match self.0 {
            Some(node) => Self(Some(node.with_corner_radii(corner_radii))),
            None => self,
        }
    }

    pub(crate) fn into_node(self) -> Option<InspectionNode> {
        self.0
    }
}

/// Presentation-only contract for a reusable native UI component.
///
/// Implementations translate caller-provided state into scene primitives. The product host remains
/// responsible for layout, input routing, lifecycle, async work, and authoritative domain state.
pub trait Component {
    /// Returns this component's resolved inspection metadata.
    ///
    /// Implementations that own box geometry should override this method. Callers then use
    /// [`UiScene::draw_component`] so registration and nesting happen automatically.
    fn inspection(&self) -> ComponentInspection {
        ComponentInspection::NONE
    }

    /// Emits this component's current visual representation into the scene.
    ///
    /// Product and composition callers should use [`UiScene::draw_component`] instead of invoking
    /// this method directly, so inspectable ancestors are not skipped.
    fn paint(&self, scene: &mut UiScene);
}

#[cfg(test)]
#[path = "component_tests.rs"]
mod tests;
