//! The stable, product-level command vocabulary.

/// Stable app product command identity.
///
/// The enum is the type-safe internal representation. [`AppCommandId::id`]
/// is the persisted and externally visible string used by keybinding
/// resources.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppCommandId {
    Copy,
    Paste,
    Save,
    ToggleTerminalSurface,
    OpenKeyboardShortcuts,
    ManageRemoteTunnels,
    ToggleTabContainer,
    ToggleWorkspacePane,
    AddSession,
    ShowAgentChanges,
    ShowAgentFiles,
    RefreshAgentFiles,
    ToggleAgentFileSearch,
    PinSession,
    CloseSession,
    RenameSession,
    GroupSession,
    ForkSession,
    PickExecutionLocation,
    PickWorkingDirectory,
    PickGitBranch,
    ShowWorkspaceDiff,
    SplitTerminalHorizontal,
    SplitTerminalVertical,
    FocusNextPane,
    FocusPreviousPane,
    ClosePane,
}

impl AppCommandId {
    /// Commands that can currently be assigned a user keybinding.
    pub const BINDABLE: [Self; 18] = [
        Self::Copy,
        Self::Paste,
        Self::Save,
        Self::ToggleTerminalSurface,
        Self::OpenKeyboardShortcuts,
        Self::ManageRemoteTunnels,
        Self::PickExecutionLocation,
        Self::ToggleTabContainer,
        Self::ToggleWorkspacePane,
        Self::ShowAgentChanges,
        Self::ShowAgentFiles,
        Self::RefreshAgentFiles,
        Self::ToggleAgentFileSearch,
        Self::SplitTerminalHorizontal,
        Self::SplitTerminalVertical,
        Self::FocusNextPane,
        Self::FocusPreviousPane,
        Self::ClosePane,
    ];

    /// Every command known to the product command catalog.
    pub const ALL: [Self; 27] = [
        Self::Copy,
        Self::Paste,
        Self::Save,
        Self::ToggleTerminalSurface,
        Self::OpenKeyboardShortcuts,
        Self::ManageRemoteTunnels,
        Self::ToggleTabContainer,
        Self::ToggleWorkspacePane,
        Self::AddSession,
        Self::ShowAgentChanges,
        Self::ShowAgentFiles,
        Self::RefreshAgentFiles,
        Self::ToggleAgentFileSearch,
        Self::PinSession,
        Self::CloseSession,
        Self::RenameSession,
        Self::GroupSession,
        Self::ForkSession,
        Self::PickExecutionLocation,
        Self::PickWorkingDirectory,
        Self::PickGitBranch,
        Self::ShowWorkspaceDiff,
        Self::SplitTerminalHorizontal,
        Self::SplitTerminalVertical,
        Self::FocusNextPane,
        Self::FocusPreviousPane,
        Self::ClosePane,
    ];

    /// Returns the stable configuration and command-palette identifier.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Copy => "editor.action.clipboardCopyAction",
            Self::Paste => "editor.action.clipboardPasteAction",
            Self::Save => "workbench.action.files.save",
            Self::ToggleTerminalSurface => "workbench.action.toggleTerminal",
            Self::OpenKeyboardShortcuts => "workbench.action.openKeyboardShortcuts",
            Self::ManageRemoteTunnels => "workbench.action.manageRemoteTunnels",
            Self::ToggleTabContainer => "workbench.action.toggleTabContainer",
            Self::ToggleWorkspacePane => "workbench.action.toggleAuxiliaryBar",
            Self::AddSession => "workbench.action.newSession",
            Self::ShowAgentChanges => "workbench.action.showAgentChanges",
            Self::ShowAgentFiles => "workbench.action.showAgentFiles",
            Self::RefreshAgentFiles => "workbench.action.refreshAgentFiles",
            Self::ToggleAgentFileSearch => "workbench.action.toggleAgentFileSearch",
            Self::PinSession => "workbench.action.pinSession",
            Self::CloseSession => "workbench.action.closeSession",
            Self::RenameSession => "workbench.action.renameSession",
            Self::GroupSession => "workbench.action.groupSession",
            Self::ForkSession => "workbench.action.forkSession",
            Self::PickExecutionLocation => "workbench.action.pickExecutionLocation",
            Self::PickWorkingDirectory => "workbench.action.pickWorkingDirectory",
            Self::PickGitBranch => "workbench.action.pickGitBranch",
            Self::ShowWorkspaceDiff => "workbench.action.showWorkspaceDiff",
            Self::SplitTerminalHorizontal => "workbench.action.splitTerminalHorizontal",
            Self::SplitTerminalVertical => "workbench.action.splitTerminalVertical",
            Self::FocusNextPane => "workbench.action.focusNextPane",
            Self::FocusPreviousPane => "workbench.action.focusPreviousPane",
            Self::ClosePane => "workbench.action.closePane",
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
            Self::ManageRemoteTunnels => "Manage Remote tunnels",
            Self::ToggleTabContainer => "Toggle tab part",
            Self::ToggleWorkspacePane => "Toggle workspace pane",
            Self::AddSession => "New session",
            Self::ShowAgentChanges => "Show changes",
            Self::ShowAgentFiles => "Show files",
            Self::RefreshAgentFiles => "Refresh files",
            Self::ToggleAgentFileSearch => "Toggle file search",
            Self::PinSession => "Pin session",
            Self::CloseSession => "Close session",
            Self::RenameSession => "Rename session",
            Self::GroupSession => "Move session to new group",
            Self::ForkSession => "Fork session",
            Self::PickExecutionLocation => "Pick execution location",
            Self::PickWorkingDirectory => "Pick working directory",
            Self::PickGitBranch => "Pick Git branch",
            Self::ShowWorkspaceDiff => "Show workspace diff",
            Self::SplitTerminalHorizontal => "Split terminal horizontally",
            Self::SplitTerminalVertical => "Split terminal vertically",
            Self::FocusNextPane => "Focus next Pane",
            Self::FocusPreviousPane => "Focus previous Pane",
            Self::ClosePane => "Close Pane",
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
