//! App's desktop product application crate.
//!
//! The crate root only registers product modules and exposes the process entry point. Product
//! lifecycle, event routing, and frame composition live in [`app`]; shared backend contracts
//! remain owned by the zeta-rs crates.

mod app;

#[cfg(test)]
#[path = "component_composition_tests.rs"]
mod component_composition_tests;

// Keep product modules available at the crate boundary while making `app` the composition
// boundary. Reusable Workbench contracts are imported from their owning crates.
#[allow(unused_imports)]
pub(crate) use app::{
    PRODUCT_DISPLAY_NAME, ProductApp, app_server, command_dispatch, file_editor_input,
    file_editor_pane, git_branch_context_menu, git_branch_context_menu_input, input_method,
    keybindings, language_service_host, launch, launch_progress, mouse_wheel, product_event,
    remote_connection_cli, remote_connection_launch_input, remote_connection_manager_input,
    remote_connection_picker_input, remote_connection_process, remote_connection_tunnel,
    remote_tunnel_manager_input, remote_tunnel_process, session_catalog, session_host,
    shell_interaction, shell_scene, tab_context_menu, terminal_blocks, terminal_history,
    terminal_input, terminal_output_scroll_view, terminal_pointer, terminal_selection,
    terminal_session, thread_timeline_scroll, workspace_context, workspace_path_picker,
    workspace_path_picker_input, workspace_surface,
};

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use app::{
    launch_profile_tests, launch_progress_tests, launch_test_support, launch_tests,
    remote_connection_cli_tests, remote_connection_tunnel_tests,
};

pub use app::run;
