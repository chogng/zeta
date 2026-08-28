use crate::AdditionalDirectory;
use crate::AdditionalDirectorySource;
use crate::WorkspaceAccessError;
use crate::WorkspaceAccessMutation;
use crate::WorkspaceAccessRevision;
use crate::WorkspaceAccessSnapshot;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use zeta_workspace::WorkspaceAuthorization;
use zeta_workspace::WorkspaceCapability;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace::WorkspaceTrustDecision;

type SourceAuthorizations = BTreeMap<AdditionalDirectorySource, WorkspaceAuthorization>;

/// Mutable authority for one primary Workspace and its additional authorized roots.
///
/// The embedding host owns where this value is stored, normally one instance per Session. Every
/// consumer freezes a capability-specific snapshot instead of retaining a second root list.
pub struct WorkspaceAccessAuthority {
    working_directory: WorkspaceRoot,
    additional_directories: Vec<AdditionalDirectory>,
    authorizations: BTreeMap<PathBuf, SourceAuthorizations>,
    revision: WorkspaceAccessRevision,
}

impl WorkspaceAccessAuthority {
    /// Creates an authority containing only the primary working directory.
    pub fn new(working_directory: WorkspaceRoot) -> Self {
        Self {
            working_directory,
            additional_directories: Vec::new(),
            authorizations: BTreeMap::new(),
            revision: WorkspaceAccessRevision::default(),
        }
    }

    /// Returns the primary root that defines project identity and cwd.
    pub fn working_directory(&self) -> &WorkspaceRoot {
        &self.working_directory
    }

    /// Returns canonical additional directories in stable path order.
    pub fn additional_directories(&self) -> &[AdditionalDirectory] {
        &self.additional_directories
    }

    /// Returns the current monotonic scope revision.
    pub fn revision(&self) -> WorkspaceAccessRevision {
        self.revision
    }

    /// Adds one host-authorized root for the exact source lifetime.
    pub fn add_directory(
        &mut self,
        authorization: WorkspaceAuthorization,
        source: AdditionalDirectorySource,
    ) -> Result<WorkspaceAccessMutation, WorkspaceAccessError> {
        let root = authorization.root().clone();
        if root == self.working_directory {
            return Err(WorkspaceAccessError::WorkingDirectoryCannotBeAdditional);
        }
        let mutation = if let Some(directory) = self
            .additional_directories
            .iter_mut()
            .find(|directory| directory.root() == &root)
        {
            if directory.add_source(source) {
                WorkspaceAccessMutation::AddedSource
            } else {
                WorkspaceAccessMutation::AlreadyPresent
            }
        } else {
            self.additional_directories
                .push(AdditionalDirectory::new(root.clone(), source));
            self.additional_directories.sort_by(|left, right| {
                left.root()
                    .canonical_path()
                    .cmp(right.root().canonical_path())
            });
            WorkspaceAccessMutation::AddedDirectory
        };
        if mutation.changes_scope() {
            self.authorizations
                .entry(root.canonical_path().to_path_buf())
                .or_default()
                .insert(source, authorization);
            self.revision.advance();
        }
        Ok(mutation)
    }

    /// Removes one source authorization and permanently revokes its capability lease.
    pub fn remove_directory(
        &mut self,
        root: &WorkspaceRoot,
        source: AdditionalDirectorySource,
    ) -> WorkspaceAccessMutation {
        let Some(index) = self
            .additional_directories
            .iter()
            .position(|directory| directory.root() == root)
        else {
            return WorkspaceAccessMutation::NotPresent;
        };
        if !self.additional_directories[index].remove_source(source) {
            return WorkspaceAccessMutation::NotPresent;
        }
        let canonical = root.canonical_path();
        if let Some(source_authorizations) = self.authorizations.get_mut(canonical) {
            if let Some(authorization) = source_authorizations.remove(&source) {
                authorization.revoke();
            }
            if source_authorizations.is_empty() {
                self.authorizations.remove(canonical);
            }
        }
        let mutation = if self.additional_directories[index].has_no_sources() {
            self.additional_directories.remove(index);
            WorkspaceAccessMutation::RemovedDirectory
        } else {
            WorkspaceAccessMutation::RemovedSource
        };
        self.revision.advance();
        mutation
    }

    /// Finds an additional root by either its requested or canonical absolute path.
    pub fn find_additional_root(&self, path: &Path) -> Option<WorkspaceRoot> {
        self.additional_directories
            .iter()
            .find(|directory| {
                directory.root().requested_path() == path
                    || directory.root().canonical_path() == path
            })
            .map(|directory| directory.root().clone())
    }

    /// Returns the host trust decision retaining one root/source pair.
    pub fn decision(
        &self,
        root: &WorkspaceRoot,
        source: AdditionalDirectorySource,
    ) -> Option<WorkspaceTrustDecision> {
        self.authorizations
            .get(root.canonical_path())
            .and_then(|authorizations| authorizations.get(&source))
            .map(WorkspaceAuthorization::decision)
    }

    /// Freezes the latest additional roots as tokens for one exact capability.
    pub fn snapshot_for(
        &self,
        capability: WorkspaceCapability,
    ) -> Result<WorkspaceAccessSnapshot, WorkspaceAccessError> {
        let mut additional_roots = Vec::with_capacity(self.additional_directories.len());
        for directory in &self.additional_directories {
            let token = self
                .authorizations
                .get(directory.root().canonical_path())
                .into_iter()
                .flat_map(BTreeMap::values)
                .find_map(|authorization| authorization.require(capability).ok())
                .ok_or_else(|| WorkspaceAccessError::CapabilityUnavailable {
                    root: directory.root().canonical_path().to_path_buf(),
                    capability,
                })?;
            additional_roots.push(token);
        }
        Ok(WorkspaceAccessSnapshot::new(
            self.revision,
            self.working_directory.clone(),
            additional_roots,
        ))
    }
}

impl Drop for WorkspaceAccessAuthority {
    fn drop(&mut self) {
        for authorization in self.authorizations.values().flat_map(BTreeMap::values) {
            authorization.revoke();
        }
    }
}

#[cfg(test)]
#[path = "authority_tests.rs"]
mod tests;
