use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use zeta_workspace::{WorkspaceTrustDecision, WorkspaceTrustId, WorkspaceTrustSource};

/// User-owned trust setting for one opaque canonical Workspace identity.
///
/// This deliberately cannot represent organization or host trust sources. Those authorities are
/// resolved by the host outside the editable user document.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceTrustSetting {
    #[default]
    Restricted,
    Trusted,
}

impl WorkspaceTrustSetting {
    /// Converts this User-owned setting into a runtime decision with the correct source.
    pub fn into_decision(self) -> WorkspaceTrustDecision {
        match self {
            Self::Restricted => WorkspaceTrustDecision::Restricted,
            Self::Trusted => {
                WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::ExplicitUserDecision)
            }
        }
    }
}

/// Durable user trust decisions keyed by canonical Workspace identity.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceTrustConfig {
    #[serde(default)]
    pub roots: BTreeMap<WorkspaceTrustId, WorkspaceTrustSetting>,
    /// Canonical roots retained only as display metadata for the trust-management surface.
    ///
    /// Authorization always uses the opaque `WorkspaceTrustId` in `roots`; this map must never
    /// be consulted as a trust decision. Older documents may not contain a display path.
    #[serde(default)]
    pub root_paths: BTreeMap<WorkspaceTrustId, PathBuf>,
}

impl WorkspaceTrustConfig {
    /// Returns only a decision that was explicitly persisted for this canonical identity.
    pub fn explicit_setting_for(
        &self,
        workspace: &WorkspaceTrustId,
    ) -> Option<WorkspaceTrustSetting> {
        self.roots.get(workspace).copied()
    }

    /// Returns the optional canonical root retained for management UI display.
    pub fn explicit_root_path_for(&self, workspace: &WorkspaceTrustId) -> Option<&Path> {
        self.root_paths.get(workspace).map(PathBuf::as_path)
    }

    /// Resolves a user setting, failing closed when no decision has been persisted.
    pub fn setting_for(&self, workspace: &WorkspaceTrustId) -> WorkspaceTrustSetting {
        self.explicit_setting_for(workspace).unwrap_or_default()
    }

    /// Resolves the runtime trust decision for one exact canonical root identity.
    pub fn decision_for(&self, workspace: &WorkspaceTrustId) -> WorkspaceTrustDecision {
        self.setting_for(workspace).into_decision()
    }
}

#[cfg(test)]
#[path = "workspace_trust_tests.rs"]
mod tests;
