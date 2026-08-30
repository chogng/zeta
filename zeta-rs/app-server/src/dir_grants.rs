use std::collections::BTreeMap;
use std::path::Path;
use std::sync::RwLock;

use zeta_file_access::Access;
use zeta_file_access::AccessError;
use zeta_file_access::Authorization;
use zeta_file_access::Dir;
use zeta_file_access::DirSource;
use zeta_file_access::Grant;
use zeta_file_access::GrantSource;
use zeta_file_access::Mutation;
use zeta_file_access::Permission;
use zeta_file_access::Permissions;
use zeta_file_access::Snapshot;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

/// App Server ownership of directory grants by Session tree or Thread subject.
#[derive(Default)]
pub(crate) struct DirGrants {
    session_trees: RwLock<BTreeMap<SessionId, Access>>,
    threads: RwLock<BTreeMap<ThreadId, Grant>>,
}

#[derive(Clone, Debug)]
pub(crate) struct DirGrantEntry {
    dir: Dir,
    permissions: Permissions,
}

impl DirGrantEntry {
    pub(crate) fn dir(&self) -> &Dir {
        &self.dir
    }

    pub(crate) fn permissions(&self) -> &Permissions {
        &self.permissions
    }
}

impl DirGrants {
    pub(crate) fn clear(&self) {
        self.session_trees
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    pub(crate) fn clear_session(&self, session_id: &SessionId) {
        self.session_trees
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(session_id);
    }

    pub(crate) fn bind_thread_dir(&self, thread_id: ThreadId, dir: Dir) {
        let permissions = Permissions::new([
            Permission::ExecuteCommands,
            Permission::InspectRepository,
            Permission::MutateRepository,
        ]);
        self.threads
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                thread_id.clone(),
                Grant::for_thread(
                    thread_id.clone(),
                    dir,
                    GrantSource::HostConfiguration,
                    permissions,
                ),
            );
    }

    pub(crate) fn unbind_thread_dir(&self, thread_id: &ThreadId) {
        self.threads
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(thread_id);
    }

    pub(crate) fn thread_dir(
        &self,
        thread_id: &ThreadId,
        permission: Permission,
    ) -> Result<Option<Authorization>, AccessError> {
        self.threads
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(thread_id)
            .map(|grant| grant.authorize(permission))
            .transpose()
            .map_err(|error| AccessError::PermissionUnavailable {
                dir: error.dir().canonical_path().to_path_buf(),
                permission,
            })
    }

    pub(crate) fn add_dir(
        &self,
        session_id: SessionId,
        grant: Grant,
    ) -> Result<Mutation, AccessError> {
        self.session_trees
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(session_id)
            .or_insert_with(Access::new)
            .add(grant, DirSource::SessionRequest)
    }

    pub(crate) fn remove_dir(&self, session_id: &SessionId, path: &Path) -> Mutation {
        let mut session_trees = self
            .session_trees
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(access) = session_trees.get_mut(session_id) else {
            return Mutation::NotPresent;
        };
        let Some(dir) = access.find(path) else {
            return Mutation::NotPresent;
        };
        access.remove(&dir, DirSource::SessionRequest)
    }

    pub(crate) fn list(&self, session_id: &SessionId) -> Vec<DirGrantEntry> {
        let session_trees = self
            .session_trees
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(access) = session_trees.get(session_id) else {
            return Vec::new();
        };
        access
            .dirs()
            .iter()
            .filter_map(|entry| {
                access
                    .permissions(entry.dir(), DirSource::SessionRequest)
                    .cloned()
                    .map(|permissions| DirGrantEntry {
                        dir: entry.dir().clone(),
                        permissions,
                    })
            })
            .collect()
    }

    pub(crate) fn revision(&self, session_id: &SessionId) -> u64 {
        self.session_trees
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            .map(|access| access.revision().get())
            .unwrap_or(0)
    }

    pub(crate) fn set_permissions(
        &self,
        session_id: &SessionId,
        path: &Path,
        expected_revision: u64,
        permissions: Permissions,
    ) -> Result<Mutation, AccessError> {
        let mut session_trees = self
            .session_trees
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(access) = session_trees.get_mut(session_id) else {
            if expected_revision == 0 {
                return Ok(Mutation::NotPresent);
            }
            return Err(AccessError::RevisionConflict {
                expected: expected_revision,
                actual: 0,
            });
        };
        let Some(dir) = access.find(path) else {
            return Ok(Mutation::NotPresent);
        };
        access.set_permissions(
            &dir,
            DirSource::SessionRequest,
            expected_revision,
            permissions,
        )
    }

    pub(crate) fn snapshot_for(
        &self,
        session_id: &SessionId,
        permission: Permission,
    ) -> Result<Option<Snapshot>, AccessError> {
        self.session_trees
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            .map(|access| access.snapshot(permission))
            .transpose()
    }

    pub(crate) fn dirs_for(
        &self,
        permission: Permission,
    ) -> std::collections::BTreeSet<std::path::PathBuf> {
        self.session_trees
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .filter_map(|access| access.snapshot(permission).ok())
            .flat_map(|snapshot| {
                snapshot
                    .authorizations()
                    .iter()
                    .filter(|authorization| authorization.ensure_active().is_ok())
                    .map(|authorization| authorization.dir().canonical_path().to_path_buf())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub(crate) fn authorize(
        &self,
        session_id: &SessionId,
        path: &Path,
        permission: Permission,
    ) -> Result<Option<Authorization>, AccessError> {
        let Some(snapshot) = self.snapshot_for(session_id, permission)? else {
            return Ok(None);
        };
        Ok(snapshot
            .authorizations()
            .iter()
            .find(|authorization| {
                authorization.dir().canonical_path() == path
                    || authorization.dir().requested_path() == path
            })
            .cloned())
    }
}

impl zeta_skills_extension::SessionSkillSourceProvider for DirGrants {
    fn snapshot(
        &self,
        session_id: &SessionId,
    ) -> Result<zeta_skills_extension::DynamicSkillSourceSnapshot, String> {
        let generation = self.revision(session_id).max(1);
        let authorizations = self
            .snapshot_for(session_id, Permission::DiscoverSkills)
            .map_err(|error| error.to_string())?
            .into_iter()
            .flat_map(|snapshot| snapshot.authorizations().to_vec());
        let mut roots = Vec::new();
        for authorization in authorizations {
            authorization
                .ensure_active()
                .map_err(|error| error.to_string())?;
            let skill_root = authorization.dir().canonical_path().join(".zeta/skills");
            if skill_root.is_dir() {
                let suffix = authorization
                    .dir()
                    .id()
                    .as_str()
                    .strip_prefix("sha256:")
                    .unwrap_or(authorization.dir().id().as_str())
                    .chars()
                    .take(16)
                    .collect::<String>();
                let id = zeta_skills::SkillSourceId::new(format!("dir:skill-source:{suffix}"))
                    .map_err(|error| error.to_string())?;
                roots.push(
                    zeta_skills::SkillSourceRoot::directory(id, skill_root)
                        .map_err(|error| error.to_string())?,
                );
            }
        }
        Ok(zeta_skills_extension::DynamicSkillSourceSnapshot { generation, roots })
    }
}
