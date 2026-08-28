//! The Workbench owns the app's tab/pane model, structural layout, chrome UI, and content boundary.
//!
//! Feature crates own the state and UI of the content mounted into Workbench panes. This crate
//! owns the shell around that content and the ordering of logical changes and bindings.

use zeta_ui_components::*;
use zui::ui::*;

mod command;
mod host;
mod keybinding_hint;
mod layout;
mod panepart;
mod tabpart;
mod workbench;

pub use command::{WorkbenchCommandDispatch, command_request_for_element};
pub use host::{
    ClosedPane, PaneActivation, PaneBindingId, PaneKey, PaneMount, TabContextMenuOutcome,
    WorkbenchHost,
};
pub use keybinding_hint::paint_chord_hint;
pub use layout::{
    InspectorLayoutSpec, InspectorPartState, LogicalViewport, PartVisibility, TabContainerLayout,
    TabContainerLayoutSpec, WorkbenchLayout, WorkbenchLayoutSpec, WorkbenchLayoutState,
    WorkbenchPart, WorkspaceLayout,
};
pub use panepart::PaneGroupLayout;
pub use panepart::PanePartSashes;
pub use panepart::PaneResizeState;
pub use panepart::pane_group_element_id;
pub use panepart::pane_sash_element_id;
pub use panepart::{
    Pane, PaneContainer, PaneGroup, PaneGroupId, PaneInput, PaneInputId, PaneInputKind, PaneNode,
    PanePart, PaneSplitDirection, PaneSplitId,
};
pub use tabpart::*;
pub use tabpart::{
    TabGroup, TabGroupId, TabInput, TabInputChange, TabInputKey, TabInputMetadata, TabPart,
};
pub use workbench::{ClosedTab, Workbench};

#[cfg(test)]
#[path = "host_tests.rs"]
mod host_tests;
