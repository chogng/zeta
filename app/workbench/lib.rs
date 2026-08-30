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
mod model;
mod pane_binding;
mod panepart;
mod presentation;
mod product;
mod quick_access;
mod surface;
mod tabpart;

pub(crate) use command::{WorkbenchCommandDispatch, command_request_for_element};
pub(crate) use host::{PaneKey, PaneMount, TabContextMenuOutcome, WorkbenchHost};
pub(crate) use keybinding_hint::paint_chord_hint;
pub(crate) use layout::{
    InspectorLayoutSpec, InspectorPartState, LogicalViewport, PartVisibility, TabContainerLayout,
    TabContainerLayoutSpec, WorkbenchLayout, WorkbenchLayoutSpec, WorkbenchLayoutState,
};
pub(crate) use model::{
    ClosedTab, Pane, PaneGroup, PaneGroupId, PaneInput, PaneInputId, PaneInputKind, PaneNode,
    PanePart, PaneSplitDirection, PaneSplitId, TabGroup, TabGroupId, TabId, TabInput,
    TabInputChange, TabInputKey, TabInputMetadata, TabPart, TabStatus, TabStatusKind, Workbench,
};
pub(crate) use pane_binding::PaneBinding;
pub(crate) use panepart::PaneGroupLayout;
pub(crate) use panepart::PanePartSashes;
pub(crate) use panepart::PaneResizeState;
pub(crate) use panepart::pane_group_element_id;
pub(crate) use panepart::pane_sash_element_id;
pub(crate) use presentation::{
    EnvironmentContextView, INSPECTOR_RESIZE_HANDLE, PaneView, TAB_CONTAINER_RESIZE_HANDLE,
    WorkbenchPresentation, WorkbenchPresentationModel,
    build_workbench_presentation_with_animation_bindings, inspector_resize_snapshot_for_viewport,
    rebuild_workbench_overlays, terminal_grid_size_for_bounds, terminal_grid_size_for_viewport,
    terminal_mouse_position_for_viewport, terminal_pane_bounds_for_viewport,
    terminal_pane_mouse_position_for_viewport, terminal_pane_sash_for_viewport,
};
#[cfg(test)]
pub(crate) use presentation::{
    WorkbenchKeybindings, WorkbenchSceneLayout, build_workbench_presentation, draw_inspector_border,
};
pub use product::run;
pub(crate) use quick_access::QuickAccess;
pub(crate) use surface::{MainSurface, MainSurfaceKind};
pub(crate) use tabpart::*;

#[allow(unused_imports)]
pub(crate) use product::{
    PRODUCT_DISPLAY_NAME, ProductApp, app_server, command_dispatch, environment_context,
    file_editor_input, file_editor_pane, git_branch_context_menu, git_branch_context_menu_input,
    input_method, keybindings, language_service_adapter, launch, launch_progress, mouse_wheel,
    path_picker, path_picker_input, product_event, remote_connection_cli,
    remote_connection_launch_input, remote_connection_manager_input,
    remote_connection_picker_input, remote_connection_process, remote_connection_tunnel,
    remote_tunnel_manager_input, remote_tunnel_process, session_host, tab_context_menu,
    terminal_blocks, terminal_history, terminal_input, terminal_output_scroll_view,
    terminal_pointer, terminal_selection, terminal_session, thread_timeline_scroll,
};

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use product::{
    launch_profile_tests, launch_progress_tests, launch_test_support, launch_tests,
    remote_connection_cli_tests, remote_connection_tunnel_tests,
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
#[cfg(test)]
#[path = "product_composition_tests.rs"]
mod product_composition_tests;
