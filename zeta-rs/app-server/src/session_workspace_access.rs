use std::collections::BTreeMap;
use std::path::Path;
use std::sync::RwLock;
use zeta_protocol::SessionId;
use zeta_workspace::WorkspaceAuthorization;
use zeta_workspace::WorkspaceCapability;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace::WorkspaceTrustDecision;
use zeta_workspace_access::AdditionalDirectorySource;
use zeta_workspace_access::WorkspaceAccessAuthority;
use zeta_workspace_access::WorkspaceAccessError;
use zeta_workspace_access::WorkspaceAccessMutation;
use zeta_workspace_access::WorkspaceAccessSnapshot;

/// App Server ownership of one Workspace access authority per Session.
///
/// RPC mutation, model environment capture, and filesystem-capable tools all read this map. The
/// domain crate owns each authority's authorization and revision semantics.
#[derive(Default)]
pub(crate) struct SessionWorkspaceAccess {
    authorities: RwLock<BTreeMap<SessionId, WorkspaceAccessAuthority>>,
}

#[derive(Clone, Debug)]
pub(crate) struct SessionAdditionalDirectory {
    root: WorkspaceRoot,
    decision: WorkspaceTrustDecision,
}

impl SessionAdditionalDirectory {
    pub(crate) fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    pub(crate) fn decision(&self) -> WorkspaceTrustDecision {
        self.decision
    }
}

impl SessionWorkspaceAccess {
    pub(crate) fn clear(&self) {
        self.authorities
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    pub(crate) fn clear_session(&self, session_id: &SessionId) {
        self.authorities
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(session_id);
    }

    pub(crate) fn add_directory(
        &self,
        session_id: SessionId,
        working_directory: WorkspaceRoot,
        authorization: WorkspaceAuthorization,
    ) -> Result<WorkspaceAccessMutation, WorkspaceAccessError> {
        self.authorities
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(session_id)
            .or_insert_with(|| WorkspaceAccessAuthority::new(working_directory))
            .add_directory(authorization, AdditionalDirectorySource::SessionCommand)
    }

    pub(crate) fn remove_directory(
        &self,
        session_id: &SessionId,
        path: &Path,
    ) -> WorkspaceAccessMutation {
        let mut authorities = self
            .authorities
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(authority) = authorities.get_mut(session_id) else {
            return WorkspaceAccessMutation::NotPresent;
        };
        let Some(root) = authority.find_additional_root(path) else {
            return WorkspaceAccessMutation::NotPresent;
        };
        authority.remove_directory(&root, AdditionalDirectorySource::SessionCommand)
    }

    pub(crate) fn list(&self, session_id: &SessionId) -> Vec<SessionAdditionalDirectory> {
        let authorities = self
            .authorities
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(authority) = authorities.get(session_id) else {
            return Vec::new();
        };
        authority
            .additional_directories()
            .iter()
            .filter_map(|directory| {
                authority
                    .decision(directory.root(), AdditionalDirectorySource::SessionCommand)
                    .map(|decision| SessionAdditionalDirectory {
                        root: directory.root().clone(),
                        decision,
                    })
            })
            .collect()
    }

    pub(crate) fn snapshot_for(
        &self,
        session_id: &SessionId,
        capability: WorkspaceCapability,
    ) -> Result<Option<WorkspaceAccessSnapshot>, WorkspaceAccessError> {
        self.authorities
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            .map(|authority| authority.snapshot_for(capability))
            .transpose()
    }
}
