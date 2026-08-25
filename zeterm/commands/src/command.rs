//! The stable, product-level command vocabulary.

/// Stable zeterm product command identity.
///
/// The enum is the type-safe internal representation. [`ZetermCommandId::id`]
/// is the persisted and externally visible string used by keybinding
/// resources.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ZetermCommandId {
    Copy,
    Paste,
    Save,
    ToggleTerminalSurface,
    OpenKeyboardShortcuts,
    OpenLanguageServerSettings,
    ManageRemoteTunnels,
    ToggleSessionSidebar,
    ToggleAgentSidebar,
    ActivateSessionTab,
    AddSession,
    ShowAgentChanges,
    ShowAgentFiles,
    RefreshAgentFiles,
    ToggleAgentFileSearch,
    PinSession,
    CloseSession,
    RenameSession,
    ForkSession,
    PickExecutionLocation,
    PickWorkingDirectory,
    PickGitBranch,
    ShowWorkspaceDiff,
}

impl ZetermCommandId {
    /// Commands that can currently be assigned a user keybinding.
    pub const BINDABLE: [Self; 14] = [
        Self::Copy,
        Self::Paste,
        Self::Save,
        Self::ToggleTerminalSurface,
        Self::OpenKeyboardShortcuts,
        Self::OpenLanguageServerSettings,
        Self::ManageRemoteTunnels,
        Self::PickExecutionLocation,
        Self::ToggleSessionSidebar,
        Self::ToggleAgentSidebar,
        Self::ShowAgentChanges,
        Self::ShowAgentFiles,
        Self::RefreshAgentFiles,
        Self::ToggleAgentFileSearch,
    ];

    /// Every command known to the product command catalog.
    pub const ALL: [Self; 23] = [
        Self::Copy,
        Self::Paste,
        Self::Save,
        Self::ToggleTerminalSurface,
        Self::OpenKeyboardShortcuts,
        Self::OpenLanguageServerSettings,
        Self::ManageRemoteTunnels,
        Self::ToggleSessionSidebar,
        Self::ToggleAgentSidebar,
        Self::ActivateSessionTab,
        Self::AddSession,
        Self::ShowAgentChanges,
        Self::ShowAgentFiles,
        Self::RefreshAgentFiles,
        Self::ToggleAgentFileSearch,
        Self::PinSession,
        Self::CloseSession,
        Self::RenameSession,
        Self::ForkSession,
        Self::PickExecutionLocation,
        Self::PickWorkingDirectory,
        Self::PickGitBranch,
        Self::ShowWorkspaceDiff,
    ];

    /// Returns the stable configuration and command-palette identifier.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Copy => "editor.action.clipboardCopyAction",
            Self::Paste => "editor.action.clipboardPasteAction",
            Self::Save => "workbench.action.files.save",
            Self::ToggleTerminalSurface => "workbench.action.toggleTerminal",
            Self::OpenKeyboardShortcuts => "workbench.action.openKeyboardShortcuts",
            Self::OpenLanguageServerSettings => "workbench.action.openLanguageServerSettings",
            Self::ManageRemoteTunnels => "workbench.action.manageRemoteTunnels",
            Self::ToggleSessionSidebar => "workbench.action.toggleSideBar",
            Self::ToggleAgentSidebar => "workbench.action.toggleAuxiliaryBar",
            Self::ActivateSessionTab => "workbench.action.activateSession",
            Self::AddSession => "workbench.action.newSession",
            Self::ShowAgentChanges => "workbench.action.showAgentChanges",
            Self::ShowAgentFiles => "workbench.action.showAgentFiles",
            Self::RefreshAgentFiles => "workbench.action.refreshAgentFiles",
            Self::ToggleAgentFileSearch => "workbench.action.toggleAgentFileSearch",
            Self::PinSession => "workbench.action.pinSession",
            Self::CloseSession => "workbench.action.closeSession",
            Self::RenameSession => "workbench.action.renameSession",
            Self::ForkSession => "workbench.action.forkSession",
            Self::PickExecutionLocation => "workbench.action.pickExecutionLocation",
            Self::PickWorkingDirectory => "workbench.action.pickWorkingDirectory",
            Self::PickGitBranch => "workbench.action.pickGitBranch",
            Self::ShowWorkspaceDiff => "workbench.action.showWorkspaceDiff",
        }
    }

    /// Returns the user-facing label used by keyboard shortcut presentation.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::Save => "Save",
            Self::ToggleTerminalSurface => "Toggle terminal",
            Self::OpenKeyboardShortcuts => "Keyboard shortcuts",
            Self::OpenLanguageServerSettings => "Language server settings",
            Self::ManageRemoteTunnels => "Manage Remote tunnels",
            Self::ToggleSessionSidebar => "Toggle session sidebar",
            Self::ToggleAgentSidebar => "Toggle agent sidebar",
            Self::ActivateSessionTab => "Activate session",
            Self::AddSession => "New session",
            Self::ShowAgentChanges => "Show changes",
            Self::ShowAgentFiles => "Show files",
            Self::RefreshAgentFiles => "Refresh files",
            Self::ToggleAgentFileSearch => "Toggle file search",
            Self::PinSession => "Pin session",
            Self::CloseSession => "Close session",
            Self::RenameSession => "Rename session",
            Self::ForkSession => "Fork session",
            Self::PickExecutionLocation => "Pick execution location",
            Self::PickWorkingDirectory => "Pick working directory",
            Self::PickGitBranch => "Pick Git branch",
            Self::ShowWorkspaceDiff => "Show workspace diff",
        }
    }

    /// Resolves a persisted command identifier to its internal command value.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.id() == id)
    }

    /// Resolves a persisted identifier only when the command is user-bindable.
    pub fn bindable_from_id(id: &str) -> Option<Self> {
        Self::BINDABLE
            .into_iter()
            .find(|command| command.id() == id)
    }
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
