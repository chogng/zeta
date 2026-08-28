//! The Workbench owns the app's tab/pane model, structural layout, chrome UI, and content boundary.
//!
//! Feature crates own the state and UI of the content mounted into Workbench panes. This crate
//! owns the Workbench chrome around that content and the ordering of logical changes and bindings.

use zeta_ui_components::*;
use zui::ui::*;

mod command;
mod host;
mod keybinding_hint;
mod layout;
mod panepart;
mod presentation;
mod surface;
mod tabpart;

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
pub use presentation::{
    INSPECTOR_RESIZE_HANDLE, MAIN_SURFACE, PaneView, TAB_CONTAINER_RESIZE_HANDLE, TERMINAL_OUTPUT,
    WorkbenchKeybindings, WorkbenchPresentation, WorkbenchPresentationModel, WorkbenchSceneLayout,
    WorkspaceContextView, build_workbench_presentation,
    build_workbench_presentation_with_animation_bindings, draw_inspector_border,
    inspector_resize_snapshot_for_viewport, rebuild_workbench_overlays,
    terminal_grid_size_for_bounds, terminal_grid_size_for_viewport,
    terminal_mouse_position_for_viewport, terminal_pane_bounds_for_viewport,
    terminal_pane_mouse_position_for_viewport, terminal_pane_sash_for_viewport,
};
pub use surface::{WorkspaceSurface, WorkspaceSurfaceKind};
pub use tabpart::*;
pub use zeta_workbench_model::{
    ClosedTab, Pane, PaneContainer, PaneGroup, PaneGroupId, PaneInput, PaneInputId, PaneInputKind,
    PaneNode, PanePart, PaneSplitDirection, PaneSplitId, TabGroup, TabGroupId, TabId, TabInput,
    TabInputChange, TabInputKey, TabInputMetadata, TabPart, TabStatus, TabStatusKind, Workbench,
};

#[cfg(test)]
#[path = "host_tests.rs"]
mod host_tests;
#[cfg(test)]
#[path = "interaction_tests.rs"]
mod interaction_tests;
#[cfg(test)]
#[path = "presentation_tests.rs"]
mod presentation_tests;
