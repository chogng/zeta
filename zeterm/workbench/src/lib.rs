//! State and layout contracts for zeterm's reusable workbench.
//!
//! The crate owns tab selection, pane topology, and sidebar sizing. It deliberately has no
//! `NativeApp`, process runtime, agent connection, renderer node, or product event loop dependency.
//! Product hosts bind these contracts to feature-specific content and side effects.

mod pane_group;
mod pane_input;
mod sidebar_part;
mod tab_input;

pub use pane_group::{PaneGroup, PaneId, PaneSplitDirection, PaneSplitId};
pub use pane_input::{PaneInput, PaneInputKind};
pub use sidebar_part::SidebarPartState;
pub use tab_input::{TabInput, TabInputChange, TabInputKey, TabInputModel};
