use zeta_commands::CommandRegistry;
use zeta_commands::CommandRequest;
use zeta_commands::ZetermCommandId;
use zui::ElementId;

use crate::NativeApp;
use crate::session_switch_trace;
use crate::shell_interaction::{
    self, AgentSidebarPaneAction, ContextAction, SessionContextMenuAction,
};

pub(crate) type NativeCommandRegistry = CommandRegistry<NativeApp>;

/// Converts native UI entry points into stable product command requests.
pub(crate) fn command_request_for_element(id: ElementId) -> Option<CommandRequest> {
    if id == shell_interaction::SESSION_SIDEBAR_TOGGLE {
        return Some(ZetermCommandId::ToggleSessionSidebar.into());
    }
    if id == shell_interaction::AGENT_SIDEBAR_TOGGLE {
        return Some(ZetermCommandId::ToggleAgentSidebar.into());
    }
    if id == shell_interaction::LANGUAGE_SERVER_SETTINGS_TOGGLE {
        return Some(ZetermCommandId::OpenLanguageServerSettings.into());
    }
    if id == shell_interaction::ACTIVE_SESSION_TAB {
        return Some(ZetermCommandId::ActivateSessionTab.into());
    }
    if id == shell_interaction::ADD_SESSION {
        return Some(ZetermCommandId::AddSession.into());
    }
    if let Some(action) = AgentSidebarPaneAction::from_element_id(id) {
        return Some(
            match action {
                AgentSidebarPaneAction::Changes => ZetermCommandId::ShowAgentChanges,
                AgentSidebarPaneAction::Files => ZetermCommandId::ShowAgentFiles,
            }
            .into(),
        );
    }
    if id == shell_interaction::AGENT_FILES_REFRESH {
        return Some(ZetermCommandId::RefreshAgentFiles.into());
    }
    if id == shell_interaction::AGENT_FILES_SEARCH {
        return Some(ZetermCommandId::ToggleAgentFileSearch.into());
    }
    if let Some(action) = SessionContextMenuAction::from_element_id(id) {
        return Some(
            match action {
                SessionContextMenuAction::Pin => ZetermCommandId::PinSession,
                SessionContextMenuAction::Close => ZetermCommandId::CloseSession,
                SessionContextMenuAction::Rename => ZetermCommandId::RenameSession,
                SessionContextMenuAction::Fork => ZetermCommandId::ForkSession,
            }
            .into(),
        );
    }
    ContextAction::from_element_id(id).map(|action| {
        match action {
            ContextAction::Location => ZetermCommandId::PickExecutionLocation,
            ContextAction::WorkingDirectory => ZetermCommandId::PickWorkingDirectory,
            ContextAction::GitBranch => ZetermCommandId::PickGitBranch,
            ContextAction::Diff => ZetermCommandId::ShowWorkspaceDiff,
        }
        .into()
    })
}

/// Builds the product's process-local command registry.
pub(crate) fn builtin_command_registry() -> NativeCommandRegistry {
    let mut registry = NativeCommandRegistry::new();
    registry
        .register(ZetermCommandId::Copy, execute_copy)
        .expect("built-in command IDs must be unique");
    registry
        .register(ZetermCommandId::Paste, execute_paste)
        .expect("built-in command IDs must be unique");
    registry
        .register(ZetermCommandId::Save, execute_save)
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::ToggleTerminalSurface,
            execute_toggle_terminal_surface,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::OpenKeyboardShortcuts,
            execute_open_keyboard_shortcuts,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::OpenLanguageServerSettings,
            execute_open_language_server_settings,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::ManageRemoteTunnels,
            execute_manage_remote_tunnels,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::ToggleSessionSidebar,
            execute_toggle_session_sidebar,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::ToggleAgentSidebar,
            execute_toggle_agent_sidebar,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::ActivateSessionTab,
            execute_activate_session_tab,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(ZetermCommandId::AddSession, execute_add_session)
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::ShowAgentChanges,
            execute_show_agent_changes,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(ZetermCommandId::ShowAgentFiles, execute_show_agent_files)
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::RefreshAgentFiles,
            execute_refresh_agent_files,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::ToggleAgentFileSearch,
            execute_toggle_agent_file_search,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::PinSession,
            execute_session_context_menu_action,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::CloseSession,
            execute_session_context_menu_action,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::RenameSession,
            execute_session_context_menu_action,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::ForkSession,
            execute_session_context_menu_action,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::PickExecutionLocation,
            execute_pick_execution_location,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::PickWorkingDirectory,
            execute_pick_working_directory,
        )
        .expect("built-in command IDs must be unique");
    registry
        .register(ZetermCommandId::PickGitBranch, execute_pick_git_branch)
        .expect("built-in command IDs must be unique");
    registry
        .register(
            ZetermCommandId::ShowWorkspaceDiff,
            execute_show_workspace_diff,
        )
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
    app.workspace_surface.toggle_terminal();
    app.pending_focus = if app.workspace_surface.is_editor() {
        Some(shell_interaction::FILE_EDITOR_DOCUMENT)
    } else if app.workspace_surface.is_terminal() {
        None
    } else {
        Some(shell_interaction::COMPOSER)
    };
    app.terminal_selection.clear();
    app.terminal_scroll.reset();
    app.keybindings.cancel_chord();
}

fn execute_open_keyboard_shortcuts(app: &mut NativeApp, _request: &CommandRequest) {
    app.language_server_settings.close();
    app.remote_connection_picker.dismiss();
    app.dismiss_remote_connection_manager();
    app.dismiss_remote_tunnel_manager();
    app.keyboard_shortcuts.toggle();
    app.keybindings.cancel_chord();
}

fn execute_open_language_server_settings(app: &mut NativeApp, _request: &CommandRequest) {
    app.language_server_settings.open();
    app.keyboard_shortcuts.close();
    let _ = app.git_branch_context_menu.dismiss();
    let _ = app.workspace_path_picker.dismiss();
    let _ = app.remote_connection_picker.dismiss();
    app.dismiss_remote_connection_manager();
    app.dismiss_remote_tunnel_manager();
    app.dismiss_session_context_menu();
    app.pending_focus = Some(zeta_settings::SETTINGS_SEARCH_INPUT);
    app.keybindings.cancel_chord();
}

fn execute_manage_remote_tunnels(app: &mut NativeApp, _request: &CommandRequest) {
    if app.remote_tunnel_host.is_none() {
        eprintln!("Remote tunnels are available only in a Remote zeterm window");
        app.keybindings.cancel_chord();
        return;
    }
    let restore_focus = app.ui_dispatch.focused();
    app.keyboard_shortcuts.close();
    app.language_server_settings.close();
    app.open_remote_tunnel_manager(restore_focus);
    app.keybindings.cancel_chord();
}

fn execute_toggle_session_sidebar(app: &mut NativeApp, _request: &CommandRequest) {
    app.session_sidebar.toggle();
    session_switch_trace::event(
        None,
        "session-sidebar-toggle",
        format_args!("expanded={}", app.session_sidebar.is_expanded()),
    );
}

fn execute_toggle_agent_sidebar(app: &mut NativeApp, _request: &CommandRequest) {
    if app.workspace_surface.is_editor() && app.agent_sidebar.is_expanded() {
        app.workspace_surface.show_agent();
        app.pending_focus = Some(shell_interaction::COMPOSER);
    }
    app.agent_sidebar.toggle();
}

fn execute_activate_session_tab(_app: &mut NativeApp, _request: &CommandRequest) {
    // Concrete tab clicks are handled by NativeApp::activate_shell_element before generic command
    // dispatch because CommandRequest does not carry the clicked tab index.
}

fn execute_add_session(app: &mut NativeApp, _request: &CommandRequest) {
    app.add_session();
}

fn execute_show_agent_changes(app: &mut NativeApp, _request: &CommandRequest) {
    app.workspace_surface.show_agent();
    app.agent_sidebar_workspace
        .select_view(crate::agent_sidebar_workspace::AgentSidebarView::Changes);
    app.agent_sidebar.expand();
}

fn execute_show_agent_files(app: &mut NativeApp, _request: &CommandRequest) {
    app.workspace_surface.show_agent();
    app.agent_sidebar_workspace
        .select_view(crate::agent_sidebar_workspace::AgentSidebarView::Files);
    app.agent_sidebar.expand();
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
    let visible = !app.agent_sidebar_workspace.search_visible();
    app.agent_sidebar_workspace.set_search_visible(visible);
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
        ZetermCommandId::PinSession
            | ZetermCommandId::CloseSession
            | ZetermCommandId::RenameSession
            | ZetermCommandId::ForkSession
    ));
    let _target_session = app.session_context_menu.target_session();
    app.dismiss_session_context_menu();
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
    app.agent_sidebar_workspace
        .select_view(crate::agent_sidebar_workspace::AgentSidebarView::Changes);
    app.workspace_surface.show_agent();
    app.agent_sidebar.expand();
}

#[cfg(test)]
#[path = "command_dispatch_tests.rs"]
mod tests;
