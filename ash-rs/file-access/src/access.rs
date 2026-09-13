use crate::AccessError;
use crate::Contribution;
use crate::Contributions;
use crate::Dir;
use crate::DirEntry;
use crate::DirSource;
use crate::Grant;
use crate::Mutation;
use crate::Permission;
use crate::Permissions;
use crate::Revision;
use crate::Snapshot;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

type SourceGrants = BTreeMap<DirSource, Grant>;

/// Mutable owner of a symmetric set of directory grants.
pub struct Access {
    dirs: Vec<DirEntry>,
    grants: BTreeMap<PathBuf, SourceGrants>,
    revision: Revision,
}

impl Access {
    pub fn new() -> Self {
        Self {
            dirs: Vec::new(),
            grants: BTreeMap::new(),
            revision: Revision::default(),
        }
    }

    pub fn dirs(&self) -> &[DirEntry] {
        &self.dirs
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn add(&mut self, grant: Grant, source: DirSource) -> Result<Mutation, AccessError> {
        let dir = grant.dir().clone();
        let mutation = if let Some(entry) = self.dirs.iter_mut().find(|entry| entry.dir() == &dir) {
            if entry.add_source(source) {
                Mutation::AddedSource
            } else {
                Mutation::AlreadyPresent
            }
        } else {
            self.dirs.push(DirEntry::new(dir.clone(), source));
            self.dirs.sort_by(|left, right| {
                left.dir()
                    .canonical_path()
                    .cmp(right.dir().canonical_path())
            });
            Mutation::AddedDir
        };
        if mutation.changes_scope() {
            self.grants
                .entry(dir.canonical_path().to_path_buf())
                .or_default()
                .insert(source, grant);
            self.revision.advance();
        }
        Ok(mutation)
    }

    pub fn remove(&mut self, dir: &Dir, source: DirSource) -> Mutation {
        let Some(index) = self.dirs.iter().position(|entry| entry.dir() == dir) else {
            return Mutation::NotPresent;
        };
        if !self.dirs[index].remove_source(source) {
            return Mutation::NotPresent;
        }
        let canonical = dir.canonical_path();
        if let Some(source_grants) = self.grants.get_mut(canonical) {
            if let Some(grant) = source_grants.remove(&source) {
                grant.revoke();
            }
            if source_grants.is_empty() {
                self.grants.remove(canonical);
            }
        }
        let mutation = if self.dirs[index].has_no_sources() {
            self.dirs.remove(index);
            Mutation::RemovedDir
        } else {
            Mutation::RemovedSource
        };
        self.revision.advance();
        mutation
    }

    pub fn find(&self, path: &Path) -> Option<Dir> {
        self.dirs
            .iter()
            .find(|entry| {
                entry.dir().requested_path() == path || entry.dir().canonical_path() == path
            })
            .map(|entry| entry.dir().clone())
    }

    pub fn permissions(&self, dir: &Dir, source: DirSource) -> Option<&Permissions> {
        self.grants
            .get(dir.canonical_path())
            .and_then(|grants| grants.get(&source))
            .map(Grant::permissions)
    }

    pub fn contributions(&self, dir: &Dir) -> Contributions {
        let contributions = self
            .grants
            .get(dir.canonical_path())
            .into_iter()
            .flat_map(BTreeMap::iter)
            .filter(|(source, _)| source.allows_contributions())
            .flat_map(|(_, grant)| {
                let permissions = grant.permissions();
                [
                    permissions.allows(Permission::LoadInstructions).then_some(
                        [
                            Contribution::AgentDefinitions,
                            Contribution::ProjectInstructions,
                            Contribution::InstructionRules,
                            Contribution::LocalInstructions,
                        ]
                        .as_slice(),
                    ),
                    permissions
                        .allows(Permission::DiscoverSkills)
                        .then_some([Contribution::Skills].as_slice()),
                    permissions
                        .allows(Permission::DiscoverMcp)
                        .then_some([Contribution::McpServers].as_slice()),
                    permissions
                        .allows(Permission::UseLanguageServices)
                        .then_some([Contribution::LanguageServices].as_slice()),
                    permissions
                        .allows(Permission::DiscoverHooks)
                        .then_some([Contribution::Hooks].as_slice()),
                    permissions.allows(Permission::DiscoverPlugins).then_some(
                        [
                            Contribution::EnabledPlugins,
                            Contribution::ExtraKnownMarketplaces,
                        ]
                        .as_slice(),
                    ),
                ]
                .into_iter()
                .flatten()
                .flatten()
                .copied()
                .collect::<Vec<_>>()
            });
        Contributions::new(contributions)
    }

    pub fn set_permissions(
        &mut self,
        dir: &Dir,
        source: DirSource,
        expected_revision: u64,
        permissions: Permissions,
    ) -> Result<Mutation, AccessError> {
        if self.revision.get() != expected_revision {
            return Err(AccessError::RevisionConflict {
                expected: expected_revision,
                actual: self.revision.get(),
            });
        }
        let Some(grant) = self
            .grants
            .get_mut(dir.canonical_path())
            .and_then(|grants| grants.get_mut(&source))
        else {
            return Ok(Mutation::NotPresent);
        };
        if grant.permissions() == &permissions {
            return Ok(Mutation::AlreadyPresent);
        }
        let replacement = Grant::new(
            grant.subject().clone(),
            grant.dir().clone(),
            grant.source(),
            permissions,
        );
        grant.revoke();
        *grant = replacement;
        self.revision.advance();
        Ok(Mutation::UpdatedPermissions)
    }

    pub fn snapshot(&self, permission: Permission) -> Result<Snapshot, AccessError> {
        let mut authorizations = Vec::with_capacity(self.dirs.len());
        for entry in &self.dirs {
            let grants = self
                .grants
                .get(entry.dir().canonical_path())
                .into_iter()
                .flat_map(BTreeMap::values)
                .filter(|grant| grant.permissions().allows(permission))
                .collect::<Vec<_>>();
            if grants.is_empty() {
                continue;
            }
            let authorization = grants
                .into_iter()
                .find_map(|grant| grant.authorize(permission).ok())
                .ok_or_else(|| AccessError::PermissionUnavailable {
                    dir: entry.dir().canonical_path().to_path_buf(),
                    permission,
                })?;
            authorizations.push(authorization);
        }
        Ok(Snapshot::new(self.revision, authorizations))
    }
}

impl Default for Access {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Access {
    fn drop(&mut self) {
        for grant in self.grants.values().flat_map(BTreeMap::values) {
            grant.revoke();
        }
    }
}

#[cfg(test)]
#[path = "access_tests.rs"]
mod tests;
