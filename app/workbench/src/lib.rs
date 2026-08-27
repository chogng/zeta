//! State and layout contracts for app's reusable workbench.
//!
//! The public [`Workbench`] is composed of an orientation-neutral [`TabPart`], a [`TitlebarPart`],
//! and one [`PaneContainer`] per tab input. Each container owns a [`PanePart`], each Pane Part owns
//! its [`PaneGroup`] leaves, and each group owns one or more pane inputs. The crate deliberately has
//! no product process runtime, agent connection, renderer node, or event loop dependency; product
//! hosts bind these contracts to feature-specific content and side effects.

mod inspector_part;
mod pane_container;
mod pane_group;
mod pane_input;
mod pane_part;
mod tab_group;
mod tab_input;
mod tab_part;
mod titlebar_part;
mod workbench;

pub use inspector_part::InspectorPartState;
pub use pane_container::PaneContainer;
pub use pane_group::{PaneGroup, PaneInputId};
pub use pane_input::{PaneInput, PaneInputKind};
pub use pane_part::{
    Pane, PaneGroupId, PaneId, PaneNode, PanePart, PaneSplitDirection, PaneSplitId,
};
pub use tab_group::{TabGroup, TabGroupId};
pub use tab_input::{TabInput, TabInputChange, TabInputKey};
pub use tab_part::TabPart;
pub use titlebar_part::TitlebarPart;
pub use workbench::{ClosedTab, Workbench};
