use zeta_commands::AppCommandId;
use zeta_commands::CommandRegistry;
use zeta_commands::CommandRequest;

use crate::PaneSplitDirection;
use crate::ProductApp;

pub(crate) type ProductCommandRegistry = CommandRegistry<ProductApp>;

/// Builds the product's process-local command registry.
pub(crate) fn builtin_command_registry() -> ProductCommandRegistry {
    let mut registry = ProductCommandRegistry::new();
    registry
        .register(AppCommandId::Copy, execute_copy)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::Paste, execute_paste)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::Save, execute_save)
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::ToggleTerminalSurface,
            execute_toggle_terminal_surface,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::OpenKeyboardShortcuts,
            execute_open_keyboard_shortcuts,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::ManageRemoteTunnels,
            execute_manage_remote_tunnels,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::ToggleFilesPane, execute_toggle_files_pane)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::AddSession, execute_add_session)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::ShowAgentChanges, execute_show_agent_changes)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::ShowAgentFiles, execute_show_agent_files)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::RefreshAgentFiles, execute_refresh_agent_files)
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::ToggleAgentFileSearch,
            execute_toggle_agent_file_search,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::PinSession, execute_tab_context_menu_action)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::CloseSession, execute_tab_context_menu_action)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::RenameSession, execute_tab_context_menu_action)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::GroupSession, execute_tab_context_menu_action)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::ForkSession, execute_tab_context_menu_action)
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::PickExecutionLocation,
            execute_pick_execution_location,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::PickWorkingDirectory,
            execute_pick_working_directory,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::PickGitBranch, execute_pick_git_branch)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::ShowGitDiff, execute_show_git_diff)
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::SplitTerminalHorizontal,
            execute_split_terminal_horizontal,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::SplitTerminalVertical,
            execute_split_terminal_vertical,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::FocusNextPane, execute_focus_next_pane)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::FocusPreviousPane, execute_focus_previous_pane)
        .expect("built-in command IDs must be unique");
    registry
        .register(AppCommandId::ClosePane, execute_close_pane)
        .expect("built-in command IDs must be unique");
    registry
}

impl ProductApp {
    pub(super) fn dispatch_command(&mut self, request: CommandRequest) {
        if self.workbench.dispatch_command(request) == crate::WorkbenchCommandDispatch::Handled {
            return;
        }
        let command_id = request.command_id();
        debug_assert!(!command_id.id().is_empty());
        let Some(handler) = self.command_registry.handler(command_id) else {
            eprintln!("command is not registered: {}", command_id.id());
            return;
        };
        handler(self, &request);
    }
}

fn execute_copy(app: &mut ProductApp, _request: &CommandRequest) {
    app.copy_keybinding_target();
}

fn execute_paste(app: &mut ProductApp, _request: &CommandRequest) {
    app.paste_keybinding_target();
}

fn execute_save(app: &mut ProductApp, _request: &CommandRequest) {
    app.save_active_file();
}

fn execute_toggle_terminal_surface(app: &mut ProductApp, _request: &CommandRequest) {
    let was_terminal = app.main_surface.is_terminal();
    app.main_surface.toggle_terminal();
    if app.main_surface.is_terminal() {
        if let Some(session_id) = app
            .active_session_tab_key()
            .and_then(|key| key.session_id().cloned())
        {
            let _ = app.activate_terminal_for_session(&session_id);
        }
    } else if was_terminal {
        app.restore_main_pane_after_terminal();
    }
    app.pending_focus = if app.main_surface.is_editor() {
        Some(zeta_editor_host::FILE_EDITOR_DOCUMENT)
    } else if app.main_surface.is_terminal() {
        None
    } else {
        Some(zeta_session::interaction::COMPOSER)
    };
    app.terminal_view_mut().selection.clear();
    app.terminal_view_mut().scroll.reset();
    app.keybindings.cancel_chord();
}

fn execute_open_keyboard_shortcuts(app: &mut ProductApp, _request: &CommandRequest) {
    app.remote_connection_picker.dismiss();
    app.dismiss_remote_connection_manager();
    app.dismiss_remote_tunnel_manager();
    app.quick_access.open_shortcuts();
    app.settings.reset_keyboard_shortcut_recording();
    app.pending_focus = Some(zeta_settings::KEYBOARD_SHORTCUTS_SEARCH);
    app.keybindings.cancel_chord();
}

fn execute_manage_remote_tunnels(app: &mut ProductApp, _request: &CommandRequest) {
    if app.remote_tunnel_host.is_none() {
        eprintln!("Remote tunnels are available only in a Remote app window");
        app.keybindings.cancel_chord();
        return;
    }
    let restore_focus = app.ui_dispatch.focused();
    app.quick_access.close();
    app.settings.reset_keyboard_shortcut_recording();
    app.activate_session_workbench_tab();
    app.open_remote_tunnel_manager(restore_focus);
    app.keybindings.cancel_chord();
}

fn execute_toggle_files_pane(app: &mut ProductApp, _request: &CommandRequest) {
    if app.main_surface.is_editor() {
        app.show_agent_pane();
        app.workbench.collapse_inspector();
        app.pending_focus = Some(zeta_session::interaction::COMPOSER);
        return;
    }
    match app.active_main_pane_kind() {
        Some(crate::PaneInputKind::Files) | Some(crate::PaneInputKind::Diff) => {
            app.show_agent_pane()
        }
        _ => app.show_files_pane(),
    }
}

fn execute_add_session(app: &mut ProductApp, _request: &CommandRequest) {
    app.add_session();
}

fn execute_show_agent_changes(app: &mut ProductApp, _request: &CommandRequest) {
    app.show_changes_pane();
}

fn execute_show_agent_files(app: &mut ProductApp, _request: &CommandRequest) {
    app.show_files_pane();
}

fn execute_refresh_agent_files(app: &mut ProductApp, _request: &CommandRequest) {
    if let Err(error) = app.refresh_git_from_app_server() {
        eprintln!("could not refresh Git snapshot: {error}");
    }
    app.refresh_files_from_app_server();
}

fn execute_toggle_agent_file_search(app: &mut ProductApp, _request: &CommandRequest) {
    let visible = !app.files.search_visible();
    app.files.set_search_visible(visible);
    if visible {
        app.rebuild_presentation();
        if let Some(presentation) = app.presentation.as_ref() {
            let _ = app.ui_dispatch.focus_element(
                presentation.interaction_frame(),
                zeta_files::FILE_SEARCH_INPUT,
            );
        }
    }
}

fn execute_tab_context_menu_action(app: &mut ProductApp, request: &CommandRequest) {
    debug_assert!(matches!(
        request.command_id(),
        AppCommandId::PinSession
            | AppCommandId::CloseSession
            | AppCommandId::RenameSession
            | AppCommandId::GroupSession
            | AppCommandId::ForkSession
    ));
    let target_tab = app
        .workbench
        .tab_context_menu()
        .target_tab()
        .cloned()
        .or_else(|| {
            app.workbench
                .workbench()
                .tab_part()
                .active_tab_key()
                .cloned()
        });
    let command_id = request.command_id();
    app.dismiss_tab_context_menu();
    if command_id == AppCommandId::CloseSession {
        if let Some(target_tab) = target_tab {
            let _ = app.close_workbench_tab(&target_tab);
        }
        return;
    }
    if command_id == AppCommandId::PinSession {
        if let Some(target_tab) = target_tab {
            let _ = app.workbench.toggle_tab_pin(&target_tab);
            app.rebuild_presentation_on_next_redraw();
        }
        return;
    }
    if command_id == AppCommandId::GroupSession
        && let Some(target_tab) = target_tab
    {
        let _ = app
            .workbench
            .move_tab_to_new_group(&target_tab, "New group");
        app.rebuild_presentation_on_next_redraw();
    }
}

fn execute_pick_execution_location(app: &mut ProductApp, _request: &CommandRequest) {
    app.toggle_remote_connection_picker();
}

fn execute_pick_working_directory(app: &mut ProductApp, _request: &CommandRequest) {
    app.toggle_directory_picker();
}

fn execute_pick_git_branch(app: &mut ProductApp, _request: &CommandRequest) {
    app.toggle_git_branch_picker();
}

fn execute_show_git_diff(app: &mut ProductApp, _request: &CommandRequest) {
    if let Err(error) = app.refresh_git_from_app_server() {
        eprintln!("could not refresh Git snapshot: {error}");
    }
    app.show_changes_pane();
}

fn execute_split_terminal_horizontal(app: &mut ProductApp, _request: &CommandRequest) {
    app.split_active_pane(PaneSplitDirection::Horizontal);
}

fn execute_split_terminal_vertical(app: &mut ProductApp, _request: &CommandRequest) {
    app.split_active_pane(PaneSplitDirection::Vertical);
}

fn execute_focus_next_pane(app: &mut ProductApp, _request: &CommandRequest) {
    app.focus_next_pane();
}

fn execute_focus_previous_pane(app: &mut ProductApp, _request: &CommandRequest) {
    app.focus_previous_pane();
}

fn execute_close_pane(app: &mut ProductApp, _request: &CommandRequest) {
    app.close_active_pane();
}

#[cfg(test)]
#[path = "command_dispatch_tests.rs"]
mod tests;
