use zeta_commands::AppCommandId;
use zeta_commands::CommandRegistry;
use zeta_commands::CommandRequest;
use zui::ui::ElementId;

use crate::NativeApp;
use crate::session::session_switch_trace;
use crate::shell_interaction::{
    self, ContextAction, SessionContextMenuAction, WorkspacePaneSelection,
};
use zeta_workbench::PaneSplitDirection;

pub(crate) type NativeCommandRegistry = CommandRegistry<NativeApp>;

/// Converts native UI entry points into stable product command requests.
pub(crate) fn command_request_for_element(id: ElementId) -> Option<CommandRequest> {
    if id == shell_interaction::TAB_CONTAINER_TOGGLE {
        return Some(AppCommandId::ToggleTabContainer.into());
    }
    if id == shell_interaction::WORKSPACE_PANE_TOGGLE {
        return Some(AppCommandId::ToggleWorkspacePane.into());
    }
    if id == shell_interaction::FIRST_TAB_CONTAINER_SESSION_TAB {
        return Some(AppCommandId::ActivateSessionTab.into());
    }
    if id == shell_interaction::ADD_SESSION {
        return Some(AppCommandId::AddSession.into());
    }
    if let Some(action) = WorkspacePaneSelection::from_element_id(id) {
        return Some(
            match action {
                WorkspacePaneSelection::Changes => AppCommandId::ShowAgentChanges,
                WorkspacePaneSelection::Files => AppCommandId::ShowAgentFiles,
            }
            .into(),
        );
    }
    if id == shell_interaction::AGENT_FILES_REFRESH {
        return Some(AppCommandId::RefreshAgentFiles.into());
    }
    if id == shell_interaction::AGENT_FILES_SEARCH {
        return Some(AppCommandId::ToggleAgentFileSearch.into());
    }
    if let Some(action) = SessionContextMenuAction::from_element_id(id) {
        return Some(
            match action {
                SessionContextMenuAction::Pin => AppCommandId::PinSession,
                SessionContextMenuAction::Close => AppCommandId::CloseSession,
                SessionContextMenuAction::Rename => AppCommandId::RenameSession,
                SessionContextMenuAction::Fork => AppCommandId::ForkSession,
            }
            .into(),
        );
    }
    ContextAction::from_element_id(id).map(|action| {
        match action {
            ContextAction::Location => AppCommandId::PickExecutionLocation,
            ContextAction::WorkingDirectory => AppCommandId::PickWorkingDirectory,
            ContextAction::GitBranch => AppCommandId::PickGitBranch,
            ContextAction::Diff => AppCommandId::ShowWorkspaceDiff,
        }
        .into()
    })
}

/// Builds the product's process-local command registry.
pub(crate) fn builtin_command_registry() -> NativeCommandRegistry {
    let mut registry = NativeCommandRegistry::new();
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
            AppCommandId::OpenLanguageServerSettings,
            execute_open_language_server_settings,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::ManageRemoteTunnels,
            execute_manage_remote_tunnels,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::ToggleTabContainer,
            execute_toggle_tab_container,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::ToggleWorkspacePane,
            execute_toggle_workspace_pane,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::ActivateSessionTab,
            execute_activate_session_tab,
        )
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
        .register(
            AppCommandId::PinSession,
            execute_session_context_menu_action,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::CloseSession,
            execute_session_context_menu_action,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::RenameSession,
            execute_session_context_menu_action,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            AppCommandId::ForkSession,
            execute_session_context_menu_action,
        )
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
        .register(AppCommandId::ShowWorkspaceDiff, execute_show_workspace_diff)
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

impl NativeApp {
    pub(super) fn dispatch_command(&mut self, request: CommandRequest) {
        let command_id = request.command_id();
        debug_assert!(!command_id.id().is_empty());
        let Some(handler) = self.command_registry.handler(command_id) else {
            eprintln!("command is not registered: {}", command_id.id());
            return;
        };
        handler(self, &request);
    }
}

fn execute_copy(app: &mut NativeApp, _request: &CommandRequest) {
    app.copy_keybinding_target();
}

fn execute_paste(app: &mut NativeApp, _request: &CommandRequest) {
    app.paste_keybinding_target();
}

fn execute_save(app: &mut NativeApp, _request: &CommandRequest) {
    app.save_active_workspace_file();
}

fn execute_toggle_terminal_surface(app: &mut NativeApp, _request: &CommandRequest) {
    let was_terminal = app.workspace_surface.is_terminal();
    app.workspace_surface.toggle_terminal();
    if app.workspace_surface.is_terminal() {
        if let Some(session_id) = app
            .active_session_tab_key()
            .and_then(|key| key.session_id().cloned())
        {
            let _ = app.activate_terminal_for_session(&session_id);
        }
    } else if was_terminal {
        app.restore_workspace_pane_after_terminal();
    }
    app.pending_focus = if app.workspace_surface.is_editor() {
        Some(shell_interaction::FILE_EDITOR_DOCUMENT)
    } else if app.workspace_surface.is_terminal() {
        None
    } else {
        Some(shell_interaction::COMPOSER)
    };
    app.terminal_view_mut().selection.clear();
    app.terminal_view_mut().scroll.reset();
    app.keybindings.cancel_chord();
}

fn execute_open_keyboard_shortcuts(app: &mut NativeApp, _request: &CommandRequest) {
    app.activate_session_workbench_tab();
    app.remote_connection_picker.dismiss();
    app.dismiss_remote_connection_manager();
    app.dismiss_remote_tunnel_manager();
    app.keyboard_shortcuts.toggle();
    app.keybindings.cancel_chord();
}

fn execute_open_language_server_settings(app: &mut NativeApp, _request: &CommandRequest) {
    app.activate_settings_tab();
}

fn execute_manage_remote_tunnels(app: &mut NativeApp, _request: &CommandRequest) {
    if app.remote_tunnel_host.is_none() {
        eprintln!("Remote tunnels are available only in a Remote app window");
        app.keybindings.cancel_chord();
        return;
    }
    let restore_focus = app.ui_dispatch.focused();
    app.keyboard_shortcuts.close();
    app.activate_session_workbench_tab();
    app.open_remote_tunnel_manager(restore_focus);
    app.keybindings.cancel_chord();
}

fn execute_toggle_tab_container(app: &mut NativeApp, _request: &CommandRequest) {
    app.workbench.toggle_tab_container();
    session_switch_trace::event(
        None,
        "tab-container-toggle",
        format_args!(
            "expanded={}",
            app.workbench.tab_container_state().is_expanded()
        ),
    );
}

fn execute_toggle_workspace_pane(app: &mut NativeApp, _request: &CommandRequest) {
    if app.workspace_surface.is_editor() {
        app.show_agent_pane();
        app.workbench.collapse_inspector();
        app.pending_focus = Some(shell_interaction::COMPOSER);
        return;
    }
    match app.active_workspace_pane_kind() {
        Some(zeta_workbench::PaneInputKind::Files) | Some(zeta_workbench::PaneInputKind::Diff) => {
            app.show_agent_pane()
        }
        _ => app.select_workspace_pane_view(crate::workspace_pane_host::WorkspacePaneView::Files),
    }
}

fn execute_activate_session_tab(_app: &mut NativeApp, _request: &CommandRequest) {
    // Concrete tab clicks are handled by NativeApp::activate_shell_element before generic command
    // dispatch because CommandRequest does not carry the clicked tab index.
}

fn execute_add_session(app: &mut NativeApp, _request: &CommandRequest) {
    app.add_session();
}

fn execute_show_agent_changes(app: &mut NativeApp, _request: &CommandRequest) {
    app.select_workspace_pane_view(crate::workspace_pane_host::WorkspacePaneView::Changes);
}

fn execute_show_agent_files(app: &mut NativeApp, _request: &CommandRequest) {
    app.select_workspace_pane_view(crate::workspace_pane_host::WorkspacePaneView::Files);
}

fn execute_refresh_agent_files(app: &mut NativeApp, _request: &CommandRequest) {
    if let Some(session) = app.agent_session.as_ref()
        && let Err(error) = session.refresh_git()
    {
        eprintln!("could not refresh Git projection: {error}");
    }
    app.refresh_files_from_app_server();
}

fn execute_toggle_agent_file_search(app: &mut NativeApp, _request: &CommandRequest) {
    let visible = !app.workspace_pane_host.search_visible();
    app.workspace_pane_host.set_search_visible(visible);
    if visible {
        app.rebuild_presentation();
        if let Some(presentation) = app.presentation.as_ref() {
            let _ = app.ui_dispatch.focus_element(
                presentation.interaction_frame(),
                shell_interaction::AGENT_FILE_SEARCH_INPUT,
            );
        }
    }
}

fn execute_session_context_menu_action(app: &mut NativeApp, request: &CommandRequest) {
    debug_assert!(matches!(
        request.command_id(),
        AppCommandId::PinSession
            | AppCommandId::CloseSession
            | AppCommandId::RenameSession
            | AppCommandId::ForkSession
    ));
    let target_tab = app.session_context_menu.target_tab().cloned();
    let command_id = request.command_id();
    app.dismiss_session_context_menu();
    if command_id == AppCommandId::CloseSession {
        if let Some(target_tab) = target_tab {
            let _ = app.close_session_tab(&target_tab);
        }
        return;
    }
    // These transitions require the future multi-Session runtime rather than mutating the single
    // PTY preview.
}

fn execute_pick_execution_location(app: &mut NativeApp, _request: &CommandRequest) {
    app.toggle_remote_connection_picker();
}

fn execute_pick_working_directory(app: &mut NativeApp, _request: &CommandRequest) {
    app.toggle_workspace_path_picker();
}

fn execute_pick_git_branch(app: &mut NativeApp, _request: &CommandRequest) {
    app.toggle_git_branch_context_menu();
}

fn execute_show_workspace_diff(app: &mut NativeApp, _request: &CommandRequest) {
    if let Some(session) = app.agent_session.as_ref()
        && let Err(error) = session.refresh_git()
    {
        eprintln!("could not refresh Git projection: {error}");
    }
    app.select_workspace_pane_view(crate::workspace_pane_host::WorkspacePaneView::Changes);
}

fn execute_split_terminal_horizontal(app: &mut NativeApp, _request: &CommandRequest) {
    app.split_active_pane(PaneSplitDirection::Horizontal);
}

fn execute_split_terminal_vertical(app: &mut NativeApp, _request: &CommandRequest) {
    app.split_active_pane(PaneSplitDirection::Vertical);
}

fn execute_focus_next_pane(app: &mut NativeApp, _request: &CommandRequest) {
    app.focus_next_pane();
}

fn execute_focus_previous_pane(app: &mut NativeApp, _request: &CommandRequest) {
    app.focus_previous_pane();
}

fn execute_close_pane(app: &mut NativeApp, _request: &CommandRequest) {
    app.close_active_pane();
}

#[cfg(test)]
#[path = "command_dispatch_tests.rs"]
mod tests;
