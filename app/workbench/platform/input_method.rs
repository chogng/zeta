use std::time::Instant;

use zeta_settings::REMOTE_CONNECTION_SEARCH_INPUT;
use zeta_settings::REMOTE_TUNNEL_REMOTE_PORT;
use zeta_settings::RemoteConnectionManagerField;
use zeta_terminal::{KeyModifiers, TerminalCore, TerminalKey};
use zui::input::Ime;
use zui::ui::{TextInputCompositionCursor, TextInputCompositionEvent};
use zui::window::ImeCursorArea;

use crate::ProductApp;
use crate::git_branch_context_menu::GIT_BRANCH_SEARCH_INPUT;
use crate::workspace_path_picker::WORKSPACE_PATH_SEARCH_INPUT;
use crate::workspace_surface::WorkspaceSurfaceKind;
use zeta_editor_host::{FILE_EDITOR_FIND_INPUT, FILE_EDITOR_REPLACE_INPUT};
use zeta_files::FILE_SEARCH_INPUT;
use zeta_session::interaction::COMPOSER;
use zeta_settings::SETTINGS_SEARCH_INPUT;
use zeta_workbench::SESSION_SEARCH_INPUT;
use zeta_workbench::TAB_RENAME_INPUT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputMethodTarget {
    Disabled,
    Composer,
    SessionSearch,
    TabRename,
    FileSearch,
    GitBranchSearch,
    WorkspacePathSearch,
    RemoteConnectionSearch,
    RemoteConnectionName,
    RemoteConnectionHost,
    RemoteConnectionWorkspace,
    RemoteTunnelPort,
    SettingsSearch,
    FileEditor,
    FileEditorFind,
    FileEditorReplace,
    TerminalGrid,
}

#[derive(Clone, Copy, Debug)]
struct InputMethodContext {
    window_active: bool,
    workspace_surface: WorkspaceSurfaceKind,
    composer_focused: bool,
    file_editor_focused: bool,
    file_editor_find_focused: bool,
    file_editor_replace_focused: bool,
    session_search_focused: bool,
    tab_rename_focused: bool,
    file_search_focused: bool,
    git_branch_search_focused: bool,
    workspace_path_search_focused: bool,
    remote_connection_search_focused: bool,
    remote_connection_manager_field: Option<RemoteConnectionManagerField>,
    remote_tunnel_port_focused: bool,
    settings_search_focused: bool,
}

impl InputMethodTarget {
    fn for_context(context: InputMethodContext) -> Self {
        if !context.window_active {
            return Self::Disabled;
        }
        if context.session_search_focused {
            return Self::SessionSearch;
        }
        if context.tab_rename_focused {
            return Self::TabRename;
        }
        if context.file_search_focused {
            return Self::FileSearch;
        }
        if context.git_branch_search_focused {
            return Self::GitBranchSearch;
        }
        if context.workspace_path_search_focused {
            return Self::WorkspacePathSearch;
        }
        if context.remote_connection_search_focused {
            return Self::RemoteConnectionSearch;
        }
        if let Some(field) = context.remote_connection_manager_field {
            return match field {
                RemoteConnectionManagerField::Name => Self::RemoteConnectionName,
                RemoteConnectionManagerField::Host => Self::RemoteConnectionHost,
                RemoteConnectionManagerField::Workspace => Self::RemoteConnectionWorkspace,
            };
        }
        if context.remote_tunnel_port_focused {
            return Self::RemoteTunnelPort;
        }
        if context.settings_search_focused {
            return Self::SettingsSearch;
        }
        if context.file_editor_find_focused {
            return Self::FileEditorFind;
        }
        if context.file_editor_replace_focused {
            return Self::FileEditorReplace;
        }
        match context.workspace_surface {
            WorkspaceSurfaceKind::Agent if context.composer_focused => Self::Composer,
            WorkspaceSurfaceKind::Editor if context.file_editor_focused => Self::FileEditor,
            WorkspaceSurfaceKind::Terminal => Self::TerminalGrid,
            WorkspaceSurfaceKind::Agent | WorkspaceSurfaceKind::Editor => Self::Disabled,
        }
    }

    const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

impl ProductApp {
    pub(super) fn ime_input(&mut self, event: Ime) {
        self.keybindings.cancel_chord();
        let target = self.input_method_target();
        if matches!(event, Ime::Enabled) {
            if target.is_enabled() {
                self.update_ime_cursor_area();
            }
            return;
        }
        match target {
            InputMethodTarget::Disabled => {}
            InputMethodTarget::Composer => {
                if self.session_pane.composer_model_picker_visible() {
                    return;
                }
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.session_pane.apply_composer_composition(composition);
                self.composer_changed();
            }
            InputMethodTarget::SessionSearch => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.caret_blink.activity(Instant::now());
                self.session_search.apply_composition(composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::TabRename => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.caret_blink.activity(Instant::now());
                self.workbench.apply_tab_rename_composition(composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::FileSearch => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.caret_blink.activity(Instant::now());
                self.files.apply_search_composition(composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::GitBranchSearch => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.caret_blink.activity(Instant::now());
                self.git_branch_context_menu
                    .apply_search_composition(composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::WorkspacePathSearch => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.caret_blink.activity(Instant::now());
                self.workspace_path_picker
                    .apply_search_composition(composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::RemoteConnectionSearch => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.caret_blink.activity(Instant::now());
                self.remote_connection_picker
                    .apply_search_composition(composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::RemoteConnectionName
            | InputMethodTarget::RemoteConnectionHost
            | InputMethodTarget::RemoteConnectionWorkspace => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                let field = target
                    .remote_connection_manager_field()
                    .expect("Remote connection input target has a field");
                self.caret_blink.activity(Instant::now());
                self.remote_connection_manager
                    .apply_composition(field, composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::RemoteTunnelPort => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.caret_blink.activity(Instant::now());
                self.remote_tunnel_manager
                    .apply_remote_port_composition(composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::SettingsSearch => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.caret_blink.activity(Instant::now());
                self.settings.apply_search_composition(composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::FileEditor => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.apply_file_editor_composition(composition);
            }
            InputMethodTarget::FileEditorFind => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.file_editor_search.apply_query_composition(composition);
                let query = self.file_editor_search.query();
                if !query.text().is_empty() {
                    self.file_editor_host.find_nearest(&query);
                }
                self.file_editor_changed();
            }
            InputMethodTarget::FileEditorReplace => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.file_editor_search
                    .apply_replacement_composition(composition);
                self.file_editor_changed();
            }
            InputMethodTarget::TerminalGrid => {
                let Some(terminal) = self.active_terminal() else {
                    return;
                };
                let input = encode_terminal_ime_event(terminal.core(), &event);
                self.send_terminal_input(input, "could not send terminal IME commit");
            }
        }
    }

    pub(super) fn update_ime_cursor_area(&self) {
        if !self.input_method_target().is_enabled() {
            return;
        }
        let Some(bounds) = self
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.ime_cursor_area)
        else {
            return;
        };
        if let Some(window) = self.window.as_ref() {
            let _ = window.set_ime_cursor_area(ImeCursorArea::new(
                bounds.origin.x as f64,
                bounds.origin.y as f64,
                bounds.size.width as f64,
                bounds.size.height as f64,
            ));
        }
    }

    pub(super) fn sync_input_focus(&mut self) {
        let target = self.input_method_target();
        if matches!(
            target,
            InputMethodTarget::Composer
                | InputMethodTarget::SessionSearch
                | InputMethodTarget::FileSearch
                | InputMethodTarget::GitBranchSearch
                | InputMethodTarget::WorkspacePathSearch
                | InputMethodTarget::RemoteConnectionSearch
                | InputMethodTarget::RemoteConnectionName
                | InputMethodTarget::RemoteConnectionHost
                | InputMethodTarget::RemoteConnectionWorkspace
                | InputMethodTarget::RemoteTunnelPort
                | InputMethodTarget::SettingsSearch
                | InputMethodTarget::FileEditor
                | InputMethodTarget::FileEditorFind
                | InputMethodTarget::FileEditorReplace
        ) {
            self.caret_blink.focus(Instant::now());
        } else {
            self.caret_blink.blur();
        }
        if target != InputMethodTarget::Composer {
            self.session_pane.cancel_composer_composition();
        }
        if target != InputMethodTarget::SessionSearch {
            self.session_search.cancel_composition();
        }
        if target != InputMethodTarget::FileSearch {
            self.files.cancel_search_composition();
        }
        if target != InputMethodTarget::GitBranchSearch {
            self.git_branch_context_menu.cancel_search_composition();
        }
        if target != InputMethodTarget::WorkspacePathSearch {
            self.workspace_path_picker.cancel_search_composition();
        }
        if target != InputMethodTarget::RemoteConnectionSearch {
            self.remote_connection_picker.cancel_search_composition();
        }
        self.remote_connection_manager
            .cancel_compositions_except(target.remote_connection_manager_field());
        if target != InputMethodTarget::RemoteTunnelPort {
            self.remote_tunnel_manager.cancel_remote_port_composition();
        }
        if target != InputMethodTarget::SettingsSearch {
            self.settings.cancel_search_composition();
        }
        if target != InputMethodTarget::FileEditor {
            self.file_editor_host.cancel_active_composition();
        }
        if !matches!(
            target,
            InputMethodTarget::FileEditorFind | InputMethodTarget::FileEditorReplace
        ) {
            self.file_editor_search.cancel_composition();
        }
        if let Some(window) = self.window.as_ref() {
            if target.is_enabled() {
                let _ = window.enable_ime();
            } else {
                let _ = window.disable_ime();
            }
        }
    }

    fn input_method_target(&self) -> InputMethodTarget {
        InputMethodTarget::for_context(InputMethodContext {
            window_active: self.ui_dispatch.window_active(),
            workspace_surface: self.workspace_surface.active(),
            composer_focused: self.ui_dispatch.is_focused(COMPOSER),
            file_editor_focused: self
                .ui_dispatch
                .is_focused(zeta_editor_host::FILE_EDITOR_DOCUMENT),
            file_editor_find_focused: self.ui_dispatch.is_focused(FILE_EDITOR_FIND_INPUT),
            file_editor_replace_focused: self.ui_dispatch.is_focused(FILE_EDITOR_REPLACE_INPUT),
            session_search_focused: self.ui_dispatch.is_focused(SESSION_SEARCH_INPUT),
            tab_rename_focused: self.ui_dispatch.is_focused(TAB_RENAME_INPUT),
            file_search_focused: self.ui_dispatch.is_focused(FILE_SEARCH_INPUT),
            git_branch_search_focused: self.ui_dispatch.is_focused(GIT_BRANCH_SEARCH_INPUT),
            workspace_path_search_focused: self.ui_dispatch.is_focused(WORKSPACE_PATH_SEARCH_INPUT),
            remote_connection_search_focused: self
                .ui_dispatch
                .is_focused(REMOTE_CONNECTION_SEARCH_INPUT),
            remote_connection_manager_field: self
                .ui_dispatch
                .focused()
                .and_then(RemoteConnectionManagerField::from_element_id),
            remote_tunnel_port_focused: self.ui_dispatch.is_focused(REMOTE_TUNNEL_REMOTE_PORT),
            settings_search_focused: self.ui_dispatch.is_focused(SETTINGS_SEARCH_INPUT),
        })
    }
}

impl InputMethodTarget {
    const fn remote_connection_manager_field(self) -> Option<RemoteConnectionManagerField> {
        match self {
            Self::RemoteConnectionName => Some(RemoteConnectionManagerField::Name),
            Self::RemoteConnectionHost => Some(RemoteConnectionManagerField::Host),
            Self::RemoteConnectionWorkspace => Some(RemoteConnectionManagerField::Workspace),
            _ => None,
        }
    }
}

fn text_input_composition_event(event: Ime) -> Option<TextInputCompositionEvent> {
    match event {
        Ime::Preedit(text, Some((start, end))) => Some(TextInputCompositionEvent::Preedit {
            text,
            cursor: TextInputCompositionCursor::Visible(start..end),
        }),
        Ime::Preedit(text, None) => Some(TextInputCompositionEvent::Preedit {
            text,
            cursor: TextInputCompositionCursor::Hidden,
        }),
        Ime::Commit(text) => Some(TextInputCompositionEvent::Commit(text)),
        Ime::Disabled => Some(TextInputCompositionEvent::Cancel),
        Ime::Enabled => None,
    }
}

fn encode_terminal_ime_event(terminal: &TerminalCore, event: &Ime) -> Vec<u8> {
    match event {
        Ime::Commit(text) => terminal.encode_key(TerminalKey::Text(text), KeyModifiers::NONE),
        Ime::Enabled | Ime::Preedit(_, _) | Ime::Disabled => Vec::new(),
    }
}

#[cfg(test)]
#[path = "input_method_tests.rs"]
mod tests;
