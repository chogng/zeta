//! Structural Workbench geometry built on backend-neutral [`zui`] layout contracts.
//!
//! The layout types resolve structural Part/Pane geometry only. Product hosts retain content,
//! identity, focus semantics, event routing, and runtime state.

mod tab_container;
mod workbench;
mod workspace;

/// Logical dimensions of a presentation viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalViewport {
    /// Width in logical UI pixels.
    pub width: f32,
    /// Height in logical UI pixels.
    pub height: f32,
}

impl LogicalViewport {
    /// Converts physical dimensions into logical UI pixels using a validated scale factor.
    pub fn from_physical(width: u32, height: u32, scale_factor: f64) -> Self {
        let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor as f32
        } else {
            1.0
        };
        Self {
            width: width as f32 / scale_factor,
            height: height as f32 / scale_factor,
        }
    }
}

/// Visibility projected by a host into a structural Part layout request.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PartVisibility {
    /// Do not include the Part leaf in the resolved layout.
    #[default]
    Collapsed,
    /// Include the Part when the available width can preserve both panes.
    Expanded,
}

pub use tab_container::TabContainerLayout;
pub use tab_container::TabContainerLayoutSpec;
pub use workbench::WorkbenchLayout;
pub use workbench::WorkbenchLayoutSpec;
pub use workbench::WorkbenchPart;
pub use workspace::InspectorLayoutSpec;
pub use workspace::WorkspaceLayout;
