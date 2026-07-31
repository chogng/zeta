use zeta_ui_dispatch::ElementId;

use crate::NativeApp;
use crate::shell_interaction::{
    self, AgentSidebarPaneAction, ContextAction, SessionContextMenuAction,
};

/// Product command identity shared by pointer, menu, and shortcut entry points.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeCommand {
    Copy,
    Paste,
    ToggleComposerMode,
    ToggleTerminalSurface,
    OpenKeyboardShortcuts,
    ToggleSessionSidebar,
    ToggleAgentSidebar,
    ActivateSessionTab,
    AddSession,
    SelectAgentPane(AgentSidebarPaneAction),
    RefreshFiles,
    ToggleFileSearch,
    SessionContextMenu(SessionContextMenuAction),
    Context(ContextAction),
}

impl NativeCommand {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Copy => "editor.action.clipboardCopyAction",
            Self::Paste => "editor.action.clipboardPasteAction",
            Self::ToggleComposerMode => "workbench.action.toggleComposerMode",
            Self::ToggleTerminalSurface => "workbench.action.toggleTerminal",
            Self::OpenKeyboardShortcuts => "workbench.action.openKeyboardShortcuts",
            Self::ToggleSessionSidebar => "workbench.action.toggleSideBar",
            Self::ToggleAgentSidebar => "workbench.action.toggleAuxiliaryBar",
            Self::ActivateSessionTab => "workbench.action.activateSession",
            Self::AddSession => "workbench.action.newSession",
            Self::SelectAgentPane(AgentSidebarPaneAction::Changes) => {
                "workbench.action.showAgentChanges"
            }
            Self::SelectAgentPane(AgentSidebarPaneAction::Files) => {
                "workbench.action.showAgentFiles"
            }
            Self::RefreshFiles => "workbench.action.refreshAgentFiles",
            Self::ToggleFileSearch => "workbench.action.toggleAgentFileSearch",
            Self::SessionContextMenu(SessionContextMenuAction::Pin) => {
                "workbench.action.pinSession"
            }
            Self::SessionContextMenu(SessionContextMenuAction::Close) => {
                "workbench.action.closeSession"
            }
            Self::SessionContextMenu(SessionContextMenuAction::Rename) => {
                "workbench.action.renameSession"
            }
            Self::SessionContextMenu(SessionContextMenuAction::Fork) => {
                "workbench.action.forkSession"
            }
            Self::Context(ContextAction::Location) => "workbench.action.pickExecutionLocation",
            Self::Context(ContextAction::WorkingDirectory) => {
                "workbench.action.pickWorkingDirectory"
            }
            Self::Context(ContextAction::GitBranch) => "workbench.action.pickGitBranch",
            Self::Context(ContextAction::Diff) => "workbench.action.showWorkspaceDiff",
        }
    }

    pub(crate) fn bindable_from_id(id: &str) -> Option<Self> {
        Self::BINDABLE
            .into_iter()
            .find(|command| command.id() == id)
    }

    pub(crate) const BINDABLE: [Self; 11] = [
        Self::Copy,
        Self::Paste,
        Self::ToggleComposerMode,
        Self::ToggleTerminalSurface,
        Self::OpenKeyboardShortcuts,
        Self::ToggleSessionSidebar,
        Self::ToggleAgentSidebar,
        Self::SelectAgentPane(AgentSidebarPaneAction::Changes),
        Self::SelectAgentPane(AgentSidebarPaneAction::Files),
        Self::RefreshFiles,
        Self::ToggleFileSearch,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::ToggleComposerMode => "Toggle composer mode",
            Self::ToggleTerminalSurface => "Toggle terminal",
            Self::OpenKeyboardShortcuts => "Keyboard shortcuts",
            Self::ToggleSessionSidebar => "Toggle session sidebar",
            Self::ToggleAgentSidebar => "Toggle agent sidebar",
            Self::ActivateSessionTab => "Activate session",
            Self::AddSession => "New session",
            Self::SelectAgentPane(AgentSidebarPaneAction::Changes) => "Show changes",
            Self::SelectAgentPane(AgentSidebarPaneAction::Files) => "Show files",
            Self::RefreshFiles => "Refresh files",
            Self::ToggleFileSearch => "Toggle file search",
            Self::SessionContextMenu(SessionContextMenuAction::Pin) => "Pin session",
            Self::SessionContextMenu(SessionContextMenuAction::Close) => "Close session",
            Self::SessionContextMenu(SessionContextMenuAction::Rename) => "Rename session",
            Self::SessionContextMenu(SessionContextMenuAction::Fork) => "Fork session",
            Self::Context(ContextAction::Location) => "Pick execution location",
            Self::Context(ContextAction::WorkingDirectory) => "Pick working directory",
            Self::Context(ContextAction::GitBranch) => "Pick Git branch",
            Self::Context(ContextAction::Diff) => "Show workspace diff",
        }
    }
}

pub(crate) fn command_for_element(id: ElementId) -> Option<NativeCommand> {
    if id == shell_interaction::COMPOSER_MODE {
        return Some(NativeCommand::ToggleComposerMode);
    }
    if id == shell_interaction::SESSION_SIDEBAR_TOGGLE {
        return Some(NativeCommand::ToggleSessionSidebar);
    }
    if id == shell_interaction::AGENT_SIDEBAR_TOGGLE {
        return Some(NativeCommand::ToggleAgentSidebar);
    }
    if id == shell_interaction::ACTIVE_SESSION_TAB {
        return Some(NativeCommand::ActivateSessionTab);
    }
    if id == shell_interaction::ADD_SESSION {
        return Some(NativeCommand::AddSession);
    }
    if let Some(action) = AgentSidebarPaneAction::from_element_id(id) {
        return Some(NativeCommand::SelectAgentPane(action));
    }
    if id == shell_interaction::AGENT_FILES_REFRESH {
        return Some(NativeCommand::RefreshFiles);
    }
    if id == shell_interaction::AGENT_FILES_SEARCH {
        return Some(NativeCommand::ToggleFileSearch);
    }
    if let Some(action) = SessionContextMenuAction::from_element_id(id) {
        return Some(NativeCommand::SessionContextMenu(action));
    }
    ContextAction::from_element_id(id).map(NativeCommand::Context)
}

impl NativeApp {
    pub(super) fn execute_native_command(&mut self, command: NativeCommand) {
        debug_assert!(!command.id().is_empty());
        match command {
            NativeCommand::Copy => self.copy_keybinding_target(),
            NativeCommand::Paste => self.paste_keybinding_target(),
            NativeCommand::ToggleComposerMode => {
                self.composer.toggle_mode();
            }
            NativeCommand::ToggleTerminalSurface => {
                self.workspace_surface.toggle();
                self.terminal_selection.clear();
                self.terminal_scroll.reset();
                self.keybindings.cancel_chord();
            }
            NativeCommand::OpenKeyboardShortcuts => {
                self.keyboard_shortcuts.toggle();
                self.keybindings.cancel_chord();
            }
            NativeCommand::ToggleSessionSidebar => self.session_sidebar.toggle(),
            NativeCommand::ToggleAgentSidebar => self.agent_sidebar.toggle(),
            NativeCommand::ActivateSessionTab => {}
            NativeCommand::AddSession => {
                // Creating another tab requires the future multi-Session runtime to own distinct
                // PTYs and active state.
            }
            NativeCommand::SelectAgentPane(action) => {
                self.agent_sidebar_workspace.select_view(action.view());
                self.agent_sidebar.expand();
            }
            NativeCommand::RefreshFiles => {
                self.workspace_context.refresh_repository();
                self.agent_sidebar_workspace
                    .sync_repository(&self.workspace_context);
                self.agent_sidebar_workspace.refresh_files();
            }
            NativeCommand::ToggleFileSearch => {
                let visible = !self.agent_sidebar_workspace.search_visible();
                self.agent_sidebar_workspace.set_search_visible(visible);
                if visible {
                    self.rebuild_presentation();
                    if let Some(presentation) = self.presentation.as_ref() {
                        let _ = self.ui_dispatch.focus_element(
                            &presentation.interaction_frame,
                            shell_interaction::AGENT_FILE_SEARCH_INPUT,
                        );
                    }
                }
            }
            NativeCommand::SessionContextMenu(action) => {
                let _target_session = self.session_context_menu.target_session();
                self.dismiss_session_context_menu();
                match action {
                    SessionContextMenuAction::Pin
                    | SessionContextMenuAction::Close
                    | SessionContextMenuAction::Rename
                    | SessionContextMenuAction::Fork => {
                        // These transitions require the future multi-Session runtime rather than
                        // mutating the single PTY preview.
                    }
                }
            }
            NativeCommand::Context(action) => match action {
                ContextAction::WorkingDirectory => self.toggle_workspace_path_picker(),
                ContextAction::GitBranch => self.toggle_git_branch_context_menu(),
                ContextAction::Location => {
                    // Pickers are product commands layered above the dispatch foundation.
                }
                ContextAction::Diff => {
                    self.workspace_context.refresh_repository();
                    self.agent_sidebar_workspace
                        .sync_repository(&self.workspace_context);
                    self.agent_sidebar_workspace
                        .select_view(crate::agent_sidebar_workspace::AgentSidebarView::Changes);
                    self.agent_sidebar.expand();
                }
            },
        }
    }
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
