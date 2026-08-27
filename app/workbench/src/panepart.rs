//! PanePart's structural view projection.
//!
//! Pane topology remains owned by [`crate::PanePart`]. This module converts that topology into
//! leaf and sash geometry for the current frame; it does not own pane content or its runtime.

mod layout;
mod model;

pub use layout::PaneGroupLayout;
pub use model::{
    Pane, PaneContainer, PaneGroup, PaneGroupId, PaneInput, PaneInputId, PaneInputKind, PaneNode,
    PanePart, PaneSplitDirection, PaneSplitId,
};
