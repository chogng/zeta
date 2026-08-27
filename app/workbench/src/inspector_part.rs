const DEFAULT_WIDTH: f32 = 520.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum InspectorPartVisibility {
    #[default]
    Collapsed,
    Expanded,
}

/// Logical visibility and preferred width for the right Workbench Inspector.
///
/// Feature content, layout constraints, and pointer-resize interaction are owned by the product
/// host. This type contains only the state that survives presentation rebuilds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InspectorPartState {
    visibility: InspectorPartVisibility,
    preferred_width: f32,
}

impl Default for InspectorPartState {
    fn default() -> Self {
        Self {
            visibility: InspectorPartVisibility::Collapsed,
            preferred_width: DEFAULT_WIDTH,
        }
    }
}

impl InspectorPartState {
    /// Creates an expanded Inspector with the default width.
    pub const fn expanded() -> Self {
        Self {
            visibility: InspectorPartVisibility::Expanded,
            preferred_width: DEFAULT_WIDTH,
        }
    }

    /// Returns whether the Inspector is logically expanded.
    pub const fn is_expanded(self) -> bool {
        matches!(self.visibility, InspectorPartVisibility::Expanded)
    }

    /// Returns the preferred width retained across visibility changes.
    pub const fn preferred_width(self) -> f32 {
        self.preferred_width
    }

    /// Stores a finite non-negative preferred width supplied by the product host.
    pub fn set_preferred_width(&mut self, width: f32) -> bool {
        if !width.is_finite() || width < 0.0 || self.preferred_width == width {
            return false;
        }
        self.preferred_width = width;
        true
    }

    pub fn toggle(&mut self) {
        self.visibility = match self.visibility {
            InspectorPartVisibility::Collapsed => InspectorPartVisibility::Expanded,
            InspectorPartVisibility::Expanded => InspectorPartVisibility::Collapsed,
        };
    }

    pub fn expand(&mut self) {
        self.visibility = InspectorPartVisibility::Expanded;
    }

    pub fn collapse(&mut self) {
        self.visibility = InspectorPartVisibility::Collapsed;
    }
}

#[cfg(test)]
#[path = "inspector_part_tests.rs"]
mod tests;
