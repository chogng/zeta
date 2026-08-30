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
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRunId;
use zeta_sandboxing::SandboxDirAccess;
use zeta_sandboxing::SandboxDirGrant;
use zeta_sandboxing::SandboxScope;

/// App Server ownership of directory grants by Session tree or Thread subject.
#[derive(Default)]
pub(crate) struct DirGrants {
    session_trees: RwLock<BTreeMap<SessionId, Access>>,
    threads: RwLock<BTreeMap<ThreadId, ThreadDirs>>,
}

#[derive(Default)]
struct ThreadDirs {
    default: Option<Grant>,
    active_attempt: Option<WorkAttemptDirs>,
}

struct WorkAttemptDirs {
    identity: WorkAttemptDirIdentity,
    primary_source_dir_id: zeta_file_access::DirId,
    roots: Vec<WorkAttemptDirGrant>,
    output: Grant,
    isolation_root: Dir,
}

struct WorkAttemptDirGrant {
    source: Dir,
    managed: Grant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkAttemptDirIdentity {
    pub(crate) work_run_id: WorkRunId,
    pub(crate) attempt_id: WorkAttemptId,
    pub(crate) execution_id: WorkExecutionId,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkAttemptDirRoot {
    pub(crate) source: Dir,
    pub(crate) managed: Dir,
}

#[derive(Clone, Debug)]
pub(crate) struct ThreadDirAuthorization {
    source: Option<Dir>,
    authorization: Authorization,
}

impl ThreadDirAuthorization {
    pub(crate) fn source(&self) -> Option<&Dir> {
        self.source.as_ref()
    }

    pub(crate) fn authorization(&self) -> &Authorization {
        &self.authorization
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ThreadDirScope {
    active_attempt: Option<WorkAttemptDirIdentity>,
    roots: Vec<ThreadDirAuthorization>,
    output: Option<Authorization>,
    hidden_dirs: Vec<Dir>,
}

impl ThreadDirScope {
    pub(crate) fn is_exact(&self) -> bool {
        self.active_attempt.is_some()
    }

    pub(crate) fn primary(&self) -> &Authorization {
        &self.roots[0].authorization
    }

    pub(crate) fn roots(&self) -> &[ThreadDirAuthorization] {
        &self.roots
    }

    pub(crate) fn authorizations(&self) -> impl Iterator<Item = &Authorization> {
        self.roots
            .iter()
            .map(ThreadDirAuthorization::authorization)
            .chain(self.output.iter())
    }

    pub(crate) fn sandbox_scope(
        &self,
        command_dir: &Authorization,
    ) -> Result<Option<SandboxScope>, String> {
        if !self.is_exact() {
            return Ok(None);
        }
        let grants = self
            .authorizations()
            .map(|authorization| {
                SandboxDirGrant::new(authorization.dir().clone(), SandboxDirAccess::ReadWrite)
            })
            .collect();
        SandboxScope::new(command_dir.dir().clone(), grants, self.hidden_dirs.clone())
            .map(Some)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn resolve_source_alias(
        &self,
        path: &Path,
        default_source: &Dir,
    ) -> Option<(Authorization, std::path::PathBuf)> {
        self.roots
            .iter()
            .filter_map(|root| {
                let source = root.source.as_ref().unwrap_or(default_source);
                path.strip_prefix(source.canonical_path())
                    .or_else(|_| path.strip_prefix(source.requested_path()))
                    .ok()
                    .map(|relative| {
                        (
                            root.authorization.clone(),
                            relative.to_path_buf(),
                            source.canonical_path().components().count(),
                        )
                    })
            })
            .max_by_key(|(_, _, depth)| *depth)
            .map(|(authorization, relative, _)| (authorization, relative))
    }
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
        self.threads
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(thread_id.clone())
            .or_default()
            .default = Some(Grant::for_thread(
            thread_id.clone(),
            dir,
            GrantSource::HostConfiguration,
            thread_dir_permissions(),
        ));
    }

    pub(crate) fn unbind_thread_dir(&self, thread_id: &ThreadId) {
        let mut threads = self
            .threads
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(dirs) = threads.get_mut(thread_id) else {
            return;
        };
        if let Some(grant) = dirs.default.take() {
            grant.revoke();
        }
        if dirs.active_attempt.is_none() {
            threads.remove(thread_id);
        }
    }

    pub(crate) fn bind_work_attempt_dirs(
        &self,
        thread_id: ThreadId,
        identity: WorkAttemptDirIdentity,
        primary_source_dir_id: zeta_file_access::DirId,
        roots: Vec<WorkAttemptDirRoot>,
        output: Dir,
        isolation_root: Dir,
    ) -> Result<(), String> {
        if roots.is_empty() {
            return Err("a WorkAttempt directory scope requires at least one root".into());
        }
        if roots
            .iter()
            .all(|root| root.source.id() != primary_source_dir_id)
        {
            return Err("the WorkAttempt primary root is not in its directory scope".into());
        }
        let environment = roots[0].source.env().clone();
        let mut source_ids = std::collections::BTreeSet::new();
        let mut managed_ids = std::collections::BTreeSet::new();
        for root in &roots {
            if root.source.env() != &environment
                || root.managed.env() != &environment
                || !source_ids.insert(root.source.id())
                || !managed_ids.insert(root.managed.id())
                || root.source.id() == root.managed.id()
            {
                return Err(
                    "WorkAttempt roots are not distinct directories in one Environment".into(),
                );
            }
        }
        if output.env() != &environment
            || isolation_root.env() != &environment
            || source_ids.contains(&output.id())
            || managed_ids.contains(&output.id())
            || !output
                .canonical_path()
                .starts_with(isolation_root.canonical_path())
            || roots.iter().any(|root| {
                !root
                    .managed
                    .canonical_path()
                    .starts_with(isolation_root.canonical_path())
            })
        {
            return Err(
                "WorkAttempt managed roots and private output are not inside their isolation root"
                    .into(),
            );
        }
        let permissions = thread_dir_permissions();
        let mut managed = roots
            .into_iter()
            .map(|root| WorkAttemptDirGrant {
                source: root.source,
                managed: Grant::for_thread(
                    thread_id.clone(),
                    root.managed,
                    GrantSource::HostConfiguration,
                    permissions.clone(),
                ),
            })
            .collect::<Vec<_>>();
        managed.sort_by_key(|root| root.source.id());
        let primary_index = managed
            .iter()
            .position(|root| root.source.id() == primary_source_dir_id)
            .expect("primary source existence checked above");
        managed.swap(0, primary_index);
        let candidate = WorkAttemptDirs {
            identity: identity.clone(),
            primary_source_dir_id,
            roots: managed,
            output: Grant::for_thread(
                thread_id.clone(),
                output,
                GrantSource::HostConfiguration,
                permissions,
            ),
            isolation_root,
        };
        let mut threads = self
            .threads
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dirs = threads.entry(thread_id).or_default();
        if let Some(active) = &dirs.active_attempt {
            if same_attempt_dirs(active, &candidate) {
                revoke_attempt_dirs(candidate);
                return Ok(());
            }
            revoke_attempt_dirs(candidate);
            return Err(format!(
                "Thread already has active WorkAttempt {} execution {}",
                active.identity.attempt_id, active.identity.execution_id
            ));
        }
        dirs.active_attempt = Some(candidate);
        Ok(())
    }

    pub(crate) fn unbind_work_attempt_dirs(
        &self,
        thread_id: &ThreadId,
        identity: &WorkAttemptDirIdentity,
    ) -> Result<(), String> {
        let mut threads = self
            .threads
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dirs = threads
            .get_mut(thread_id)
            .ok_or_else(|| "Thread has no directory scope".to_string())?;
        let active = dirs
            .active_attempt
            .as_ref()
            .ok_or_else(|| "Thread has no active WorkAttempt directory scope".to_string())?;
        if &active.identity != identity {
            return Err("active WorkAttempt execution identity does not match".into());
        }
        revoke_attempt_dirs(
            dirs.active_attempt
                .take()
                .expect("active WorkAttempt checked above"),
        );
        if dirs.default.is_none() {
            threads.remove(thread_id);
        }
        Ok(())
    }

    pub(crate) fn thread_scope(
        &self,
        thread_id: &ThreadId,
        permission: Permission,
    ) -> Result<Option<ThreadDirScope>, AccessError> {
        let threads = self
            .threads
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(dirs) = threads.get(thread_id) else {
            return Ok(None);
        };
        if let Some(active) = &dirs.active_attempt {
            let roots = active
                .roots
                .iter()
                .map(|root| {
                    root.managed
                        .authorize(permission)
                        .map(|authorization| ThreadDirAuthorization {
                            source: Some(root.source.clone()),
                            authorization,
                        })
                        .map_err(|error| AccessError::PermissionUnavailable {
                            dir: error.dir().canonical_path().to_path_buf(),
                            permission,
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let output = active.output.authorize(permission).map_err(|error| {
                AccessError::PermissionUnavailable {
                    dir: error.dir().canonical_path().to_path_buf(),
                    permission,
                }
            })?;
            debug_assert_eq!(
                roots[0].source().map(Dir::id),
                Some(active.primary_source_dir_id.clone())
            );
            return Ok(Some(ThreadDirScope {
                active_attempt: Some(active.identity.clone()),
                roots,
                output: Some(output),
                hidden_dirs: std::iter::once(active.isolation_root.clone())
                    .chain(active.roots.iter().map(|root| root.source.clone()))
                    .fold(Vec::new(), |mut dirs, dir| {
                        if !dirs.iter().any(|existing: &Dir| existing == &dir) {
                            dirs.push(dir);
                        }
                        dirs
                    }),
            }));
        }
        dirs.default
            .as_ref()
            .map(|grant| {
                grant
                    .authorize(permission)
                    .map(|authorization| ThreadDirScope {
                        active_attempt: None,
                        roots: vec![ThreadDirAuthorization {
                            source: None,
                            authorization,
                        }],
                        output: None,
                        hidden_dirs: Vec::new(),
                    })
            })
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

fn thread_dir_permissions() -> Permissions {
    Permissions::new([
        Permission::ExecuteCommands,
        Permission::InspectRepository,
        Permission::MutateRepository,
    ])
}

fn same_attempt_dirs(left: &WorkAttemptDirs, right: &WorkAttemptDirs) -> bool {
    left.identity == right.identity
        && left.primary_source_dir_id == right.primary_source_dir_id
        && left.output.dir() == right.output.dir()
        && left.isolation_root == right.isolation_root
        && left.roots.len() == right.roots.len()
        && left.roots.iter().zip(&right.roots).all(|(left, right)| {
            left.source == right.source && left.managed.dir() == right.managed.dir()
        })
}

fn revoke_attempt_dirs(dirs: WorkAttemptDirs) {
    for root in dirs.roots {
        root.managed.revoke();
    }
    dirs.output.revoke();
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
