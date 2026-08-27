use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatusLineItem {
    Permissions,
    Model,
    GitBranch,
    GitChanges,
}

impl StatusLineItem {
    pub(crate) const ALL: [Self; 4] = [
        Self::Permissions,
        Self::Model,
        Self::GitBranch,
        Self::GitChanges,
    ];

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Permissions => "permissions",
            Self::Model => "model",
            Self::GitBranch => "git-branch",
            Self::GitChanges => "git-changes",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Permissions => "Permissions",
            Self::Model => "Model",
            Self::GitBranch => "Git branch",
            Self::GitChanges => "Git changes",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StatusLineSettings {
    permissions: bool,
    model: bool,
    git_branch: bool,
    git_changes: bool,
}

impl StatusLineSettings {
    pub(crate) fn enabled(self, item: StatusLineItem) -> bool {
        match item {
            StatusLineItem::Permissions => self.permissions,
            StatusLineItem::Model => self.model,
            StatusLineItem::GitBranch => self.git_branch,
            StatusLineItem::GitChanges => self.git_changes,
        }
    }

    pub(crate) fn set(&mut self, item: StatusLineItem, enabled: bool) {
        match item {
            StatusLineItem::Permissions => self.permissions = enabled,
            StatusLineItem::Model => self.model = enabled,
            StatusLineItem::GitBranch => self.git_branch = enabled,
            StatusLineItem::GitChanges => self.git_changes = enabled,
        }
    }
}

impl Default for StatusLineSettings {
    fn default() -> Self {
        Self {
            permissions: true,
            model: true,
            git_branch: true,
            git_changes: true,
        }
    }
}
