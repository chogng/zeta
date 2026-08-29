use std::collections::BTreeMap;
use std::path::Path;
use std::sync::RwLock;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_workspace::WorkspaceAuthorization;
use zeta_workspace::WorkspaceCapability;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace::WorkspaceTrustDecision;
use zeta_workspace_access::AdditionalDirectoryPermissions;
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
    thread_workspaces: RwLock<BTreeMap<ThreadId, WorkspaceAuthorization>>,
}

#[derive(Clone, Debug)]
pub(crate) struct SessionAdditionalDirectory {
    root: WorkspaceRoot,
    decision: WorkspaceTrustDecision,
    permissions: AdditionalDirectoryPermissions,
}

impl SessionAdditionalDirectory {
    pub(crate) fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    pub(crate) fn decision(&self) -> WorkspaceTrustDecision {
        self.decision
    }

    pub(crate) fn permissions(&self) -> &AdditionalDirectoryPermissions {
        &self.permissions
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

    pub(crate) fn bind_thread_workspace(&self, thread_id: ThreadId, root: WorkspaceRoot) {
        self.thread_workspaces
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                thread_id,
                WorkspaceAuthorization::new(
                    root,
                    WorkspaceTrustDecision::Trusted(
                        zeta_workspace::WorkspaceTrustSource::HostConfiguration,
                    ),
                ),
            );
    }

    pub(crate) fn unbind_thread_workspace(&self, thread_id: &ThreadId) {
        self.thread_workspaces
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(thread_id);
    }

    pub(crate) fn thread_workspace(
        &self,
        thread_id: &ThreadId,
        capability: WorkspaceCapability,
    ) -> Result<Option<zeta_workspace::TrustedWorkspace>, WorkspaceAccessError> {
        self.thread_workspaces
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(thread_id)
            .map(|authorization| authorization.require(capability))
            .transpose()
            .map_err(|error| WorkspaceAccessError::CapabilityUnavailable {
                root: error.root().canonical_path().to_path_buf(),
                capability,
            })
    }

    pub(crate) fn add_directory(
        &self,
        session_id: SessionId,
        working_directory: WorkspaceRoot,
        authorization: WorkspaceAuthorization,
        permissions: AdditionalDirectoryPermissions,
    ) -> Result<WorkspaceAccessMutation, WorkspaceAccessError> {
        self.authorities
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(session_id)
            .or_insert_with(|| WorkspaceAccessAuthority::new(working_directory))
            .add_directory(
                authorization,
                AdditionalDirectorySource::SessionCommand,
                permissions,
            )
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
                    .zip(
                        authority
                            .permissions(
                                directory.root(),
                                AdditionalDirectorySource::SessionCommand,
                            )
                            .cloned(),
                    )
                    .map(|(decision, permissions)| SessionAdditionalDirectory {
                        root: directory.root().clone(),
                        decision,
                        permissions,
                    })
            })
            .collect()
    }

    pub(crate) fn revision(&self, session_id: &SessionId) -> u64 {
        self.authorities
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            .map(|authority| authority.revision().get())
            .unwrap_or(0)
    }

    pub(crate) fn set_permissions(
        &self,
        session_id: &SessionId,
        path: &Path,
        expected_revision: u64,
        permissions: AdditionalDirectoryPermissions,
    ) -> Result<WorkspaceAccessMutation, WorkspaceAccessError> {
        let mut authorities = self
            .authorities
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(authority) = authorities.get_mut(session_id) else {
            if expected_revision == 0 {
                return Ok(WorkspaceAccessMutation::NotPresent);
            }
            return Err(WorkspaceAccessError::RevisionConflict {
                expected: expected_revision,
                actual: 0,
            });
        };
        let Some(root) = authority.find_additional_root(path) else {
            return Ok(WorkspaceAccessMutation::NotPresent);
        };
        authority.set_permissions(
            &root,
            AdditionalDirectorySource::SessionCommand,
            expected_revision,
            permissions,
        )
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

    pub(crate) fn roots_for(
        &self,
        capability: WorkspaceCapability,
    ) -> std::collections::BTreeSet<std::path::PathBuf> {
        self.authorities
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .filter_map(|authority| authority.snapshot_for(capability).ok())
            .flat_map(|snapshot| {
                snapshot
                    .additional_roots()
                    .iter()
                    .filter(|workspace| workspace.ensure_active().is_ok())
                    .map(|workspace| workspace.root().canonical_path().to_path_buf())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub(crate) fn workspace_for(
        &self,
        session_id: &SessionId,
        path: &Path,
        capability: WorkspaceCapability,
    ) -> Result<Option<zeta_workspace::TrustedWorkspace>, WorkspaceAccessError> {
        let Some(snapshot) = self.snapshot_for(session_id, capability)? else {
            return Ok(None);
        };
        Ok(snapshot
            .additional_roots()
            .iter()
            .find(|workspace| {
                workspace.root().canonical_path() == path
                    || workspace.root().requested_path() == path
            })
            .cloned())
    }
}

impl zeta_skills_extension::SessionSkillSourceProvider for SessionWorkspaceAccess {
    fn snapshot(
        &self,
        session_id: &SessionId,
    ) -> Result<zeta_skills_extension::DynamicSkillSourceSnapshot, String> {
        let generation = self.revision(session_id).max(1);
        let workspaces = self
            .snapshot_for(session_id, WorkspaceCapability::DiscoverSkills)
            .map_err(|error| error.to_string())?
            .into_iter()
            .flat_map(|snapshot| snapshot.additional_roots().to_vec());
        let mut roots = Vec::new();
        for workspace in workspaces {
            workspace
                .ensure_active()
                .map_err(|error| error.to_string())?;
            let skill_root = workspace.root().canonical_path().join(".zeta/skills");
            if skill_root.is_dir() {
                let suffix = workspace
                    .root()
                    .trust_id()
                    .as_str()
                    .strip_prefix("sha256:")
                    .unwrap_or(workspace.root().trust_id().as_str())
                    .chars()
                    .take(16)
                    .collect::<String>();
                let id = zeta_skills::SkillSourceId::new(format!(
                    "workspace:skill-source:additional-{suffix}"
                ))
                .map_err(|error| error.to_string())?;
                roots.push(
                    zeta_skills::SkillSourceRoot::workspace(id, skill_root)
                        .map_err(|error| error.to_string())?,
                );
            }
        }
        Ok(zeta_skills_extension::DynamicSkillSourceSnapshot { generation, roots })
    }
}
