use crate::Dir;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use zeta_protocol::{SessionId, ThreadId};

/// Subject that receives one directory grant.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum GrantSubject {
    Environment(crate::EnvId),
    SessionTree(SessionId),
    Thread(ThreadId),
}

/// Host-owned source that issued one directory grant.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GrantSource {
    ExplicitUser,
    OrganizationPolicy,
    HostConfiguration,
}

/// Permission kind that can be granted for one directory scope.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    ReadFiles,
    WriteFiles,
    ExecuteCommands,
    WatchFiles,
    BrowseFiles,
    SearchFiles,
    LoadInstructions,
    LoadConfig,
    DiscoverSkills,
    DiscoverMcp,
    UseLanguageServices,
    DiscoverHooks,
    DiscoverPlugins,
    InspectRepository,
    MutateRepository,
}

impl fmt::Display for Permission {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ReadFiles => "read files",
            Self::WriteFiles => "write files",
            Self::ExecuteCommands => "execute commands",
            Self::WatchFiles => "watch files",
            Self::BrowseFiles => "browse files",
            Self::SearchFiles => "search files",
            Self::LoadInstructions => "load instructions",
            Self::LoadConfig => "load configuration",
            Self::DiscoverSkills => "discover skills",
            Self::DiscoverMcp => "discover MCP servers",
            Self::UseLanguageServices => "use language services",
            Self::DiscoverHooks => "discover hooks",
            Self::DiscoverPlugins => "discover plugins",
            Self::InspectRepository => "inspect a repository",
            Self::MutateRepository => "mutate a repository",
        })
    }
}

/// Complete permission set granted to one directory.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Permissions {
    entries: BTreeSet<Permission>,
}

impl Permissions {
    pub fn new(permissions: impl IntoIterator<Item = Permission>) -> Self {
        Self {
            entries: permissions.into_iter().collect(),
        }
    }

    pub fn allows(&self, permission: Permission) -> bool {
        self.entries.contains(&permission)
    }

    pub fn entries(&self) -> impl ExactSizeIterator<Item = Permission> + '_ {
        self.entries.iter().copied()
    }
}

/// Denied result of one authorization check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermissionDenied {
    dir: Dir,
    permission: Permission,
}

impl PermissionDenied {
    pub fn dir(&self) -> &Dir {
        &self.dir
    }

    pub fn permission(&self) -> Permission {
        self.permission
    }
}

impl fmt::Display for PermissionDenied {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "permission is required to {}: {}",
            self.permission,
            self.dir.canonical_path().display()
        )
    }
}

impl std::error::Error for PermissionDenied {}

#[derive(Debug)]
struct Lease {
    active: AtomicBool,
}

impl Lease {
    fn new() -> Self {
        Self {
            active: AtomicBool::new(true),
        }
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    fn revoke(&self) {
        self.active.store(false, Ordering::Release);
    }
}

/// Revocable permission grant bound to one exact directory scope.
#[derive(Clone, Debug)]
pub struct Grant {
    subject: GrantSubject,
    dir: Dir,
    source: GrantSource,
    permissions: Permissions,
    lease: Arc<Lease>,
}

impl Grant {
    pub fn for_environment(dir: Dir, source: GrantSource, permissions: Permissions) -> Self {
        let subject = GrantSubject::Environment(dir.env().clone());
        Self::new(subject, dir, source, permissions)
    }

    pub fn for_session_tree(
        session_id: SessionId,
        dir: Dir,
        source: GrantSource,
        permissions: Permissions,
    ) -> Self {
        Self::new(
            GrantSubject::SessionTree(session_id),
            dir,
            source,
            permissions,
        )
    }

    pub fn for_thread(
        thread_id: ThreadId,
        dir: Dir,
        source: GrantSource,
        permissions: Permissions,
    ) -> Self {
        Self::new(GrantSubject::Thread(thread_id), dir, source, permissions)
    }

    pub(crate) fn new(
        subject: GrantSubject,
        dir: Dir,
        source: GrantSource,
        permissions: Permissions,
    ) -> Self {
        Self {
            subject,
            dir,
            source,
            permissions,
            lease: Arc::new(Lease::new()),
        }
    }

    pub fn subject(&self) -> &GrantSubject {
        &self.subject
    }

    pub fn dir(&self) -> &Dir {
        &self.dir
    }

    pub fn source(&self) -> GrantSource {
        self.source
    }

    pub fn permissions(&self) -> &Permissions {
        &self.permissions
    }

    pub fn revoke(&self) {
        self.lease.revoke();
    }

    pub fn is_active(&self) -> bool {
        self.lease.is_active()
    }

    pub fn authorize(&self, permission: Permission) -> AuthorizationDecision {
        if !self.permissions.allows(permission) || !self.lease.is_active() {
            return Err(PermissionDenied {
                dir: self.dir.clone(),
                permission,
            });
        }
        Ok(Authorization {
            subject: self.subject.clone(),
            dir: self.dir.clone(),
            source: self.source,
            permission,
            lease: Arc::clone(&self.lease),
        })
    }
}

/// Complete allow-or-deny result of one authorization check.
pub type AuthorizationDecision = Result<Authorization, PermissionDenied>;

/// Ephemeral proof carried only from an allow decision into the checked operation.
///
/// This value is not a grant and is never persisted. Revoking its source grant invalidates it.
#[derive(Clone, Debug)]
pub struct Authorization {
    subject: GrantSubject,
    dir: Dir,
    source: GrantSource,
    permission: Permission,
    lease: Arc<Lease>,
}

impl Authorization {
    pub fn evaluate(
        subject: GrantSubject,
        dir: Dir,
        source: GrantSource,
        permissions: Permissions,
        permission: Permission,
    ) -> AuthorizationDecision {
        Grant::new(subject, dir, source, permissions).authorize(permission)
    }

    pub fn subject(&self) -> &GrantSubject {
        &self.subject
    }

    pub fn dir(&self) -> &Dir {
        &self.dir
    }

    pub fn source(&self) -> GrantSource {
        self.source
    }

    pub fn permission(&self) -> Permission {
        self.permission
    }

    pub fn ensure_active(&self) -> Result<(), PermissionDenied> {
        if self.lease.is_active() {
            Ok(())
        } else {
            Err(PermissionDenied {
                dir: self.dir.clone(),
                permission: self.permission,
            })
        }
    }

    pub fn is_active(&self) -> bool {
        self.lease.is_active()
    }
}

#[cfg(test)]
#[path = "grant_tests.rs"]
mod tests;
