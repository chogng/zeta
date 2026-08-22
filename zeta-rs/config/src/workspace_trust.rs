use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use zeta_workspace::{WorkspaceTrustDecision, WorkspaceTrustId, WorkspaceTrustSource};

/// User-owned trust setting for one opaque canonical Workspace identity.
///
/// This deliberately cannot represent organization or host trust sources. Those authorities are
/// resolved by the host outside the editable user document. `Restricted` is the fail-closed
/// runtime result for an absent entry; it is accepted for compatibility but is not persisted.
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

/// Durable trusted roots keyed by canonical Workspace identity.
///
/// A missing root is the durable representation of Restricted mode. Older versions could write
/// explicit `restricted` entries; those entries are removed during configuration loading.
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
    /// Returns a trusted decision that was explicitly persisted for this canonical identity.
    pub fn explicit_setting_for(
        &self,
        workspace: &WorkspaceTrustId,
    ) -> Option<WorkspaceTrustSetting> {
        match self.roots.get(workspace).copied() {
            Some(WorkspaceTrustSetting::Trusted) => Some(WorkspaceTrustSetting::Trusted),
            Some(WorkspaceTrustSetting::Restricted) | None => None,
        }
    }

    /// Removes legacy Restricted entries and display metadata that has no trusted root.
    pub(crate) fn normalize_legacy_entries(&mut self) -> bool {
        let restricted_roots = self
            .roots
            .iter()
            .filter(|(_, setting)| **setting == WorkspaceTrustSetting::Restricted)
            .map(|(workspace, _)| workspace.clone())
            .collect::<Vec<_>>();
        let mut changed = false;
        for workspace in restricted_roots {
            changed |= self.roots.remove(&workspace).is_some();
            changed |= self.root_paths.remove(&workspace).is_some();
        }

        let orphaned_paths = self
            .root_paths
            .keys()
            .filter(|workspace| !self.roots.contains_key(*workspace))
            .cloned()
            .collect::<Vec<_>>();
        for workspace in orphaned_paths {
            changed |= self.root_paths.remove(&workspace).is_some();
        }
        changed
    }

    /// Returns the optional canonical root retained for management UI display.
    pub fn explicit_root_path_for(&self, workspace: &WorkspaceTrustId) -> Option<&Path> {
        self.explicit_setting_for(workspace)
            .and_then(|_| self.root_paths.get(workspace).map(PathBuf::as_path))
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
