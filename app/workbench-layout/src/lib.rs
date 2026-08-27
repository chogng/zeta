//! Structural layout contracts for the reusable Workbench.
//!
//! This crate resolves Workbench and Pane topology into immutable geometry snapshots. It owns no
//! product content, runtime binding, interaction dispatch, renderer state, or frame scheduling.

mod layout;

pub use layout::InspectorLayoutSpec;
pub use layout::LogicalViewport;
pub use layout::PaneGroupLayout;
pub use layout::PartVisibility;
pub use layout::TabContainerLayout;
pub use layout::TabContainerLayoutSpec;
pub use layout::WorkbenchLayout;
pub use layout::WorkbenchLayoutSpec;
pub use layout::WorkbenchPart;
pub use layout::WorkspaceLayout;
