use crate::SandboxError;
use std::collections::BTreeSet;
use zeta_file_access::Dir;

/// Filesystem authority granted to one directory inside a sandboxed process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxDirAccess {
    ReadOnly,
    ReadWrite,
}

/// One exact directory and the maximum access a sandboxed process may receive for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxDirGrant {
    dir: Dir,
    access: SandboxDirAccess,
}

impl SandboxDirGrant {
    pub fn new(dir: Dir, access: SandboxDirAccess) -> Self {
        Self { dir, access }
    }

    pub fn dir(&self) -> &Dir {
        &self.dir
    }

    pub fn access(&self) -> SandboxDirAccess {
        self.access
    }
}

/// Exact directory visibility for one sandboxed process.
///
/// The host remains readable for toolchains except beneath `hidden_dirs`. Platform backends must
/// hide each of those directories and then reopen only the listed grants. This lets one process
/// use several owned roots without exposing sibling workspaces stored under the same parent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxScope {
    command_dir: Dir,
    grants: Vec<SandboxDirGrant>,
    hidden_dirs: Vec<Dir>,
}

impl SandboxScope {
    pub fn single(command_dir: Dir) -> Self {
        Self {
            grants: vec![SandboxDirGrant::new(
                command_dir.clone(),
                SandboxDirAccess::ReadWrite,
            )],
            command_dir,
            hidden_dirs: Vec::new(),
        }
    }

    pub fn new(
        command_dir: Dir,
        grants: Vec<SandboxDirGrant>,
        hidden_dirs: Vec<Dir>,
    ) -> Result<Self, SandboxError> {
        if grants.is_empty() {
            return Err(SandboxError::InvalidScope(
                "sandbox scope requires at least one directory grant".into(),
            ));
        }
        let environment = command_dir.env();
        if grants.iter().any(|grant| grant.dir().env() != environment)
            || hidden_dirs.iter().any(|dir| dir.env() != environment)
        {
            return Err(SandboxError::InvalidScope(
                "sandbox scope directories must belong to one environment".into(),
            ));
        }
        if !grants.iter().any(|grant| grant.dir() == &command_dir) {
            return Err(SandboxError::InvalidScope(
                "sandbox command directory is not granted".into(),
            ));
        }

        let mut grant_ids = BTreeSet::new();
        for grant in &grants {
            if !grant_ids.insert(grant.dir().id()) {
                return Err(SandboxError::InvalidScope(
                    "sandbox scope contains a duplicate directory grant".into(),
                ));
            }
        }
        for (index, left) in grants.iter().enumerate() {
            for right in grants.iter().skip(index + 1) {
                if left
                    .dir()
                    .canonical_path()
                    .starts_with(right.dir().canonical_path())
                    || right
                        .dir()
                        .canonical_path()
                        .starts_with(left.dir().canonical_path())
                {
                    return Err(SandboxError::InvalidScope(
                        "sandbox directory grants must not overlap".into(),
                    ));
                }
            }
        }

        let mut hidden_ids = BTreeSet::new();
        for hidden in &hidden_dirs {
            if !hidden_ids.insert(hidden.id()) {
                return Err(SandboxError::InvalidScope(
                    "sandbox scope contains a duplicate hidden directory".into(),
                ));
            }
            if grants.iter().any(|grant| {
                hidden
                    .canonical_path()
                    .starts_with(grant.dir().canonical_path())
            }) {
                return Err(SandboxError::InvalidScope(
                    "a hidden directory cannot be the same as or below a granted directory".into(),
                ));
            }
        }

        Ok(Self {
            command_dir,
            grants,
            hidden_dirs,
        })
    }

    pub fn command_dir(&self) -> &Dir {
        &self.command_dir
    }

    pub fn grants(&self) -> &[SandboxDirGrant] {
        &self.grants
    }

    pub fn hidden_dirs(&self) -> &[Dir] {
        &self.hidden_dirs
    }

    pub(crate) fn is_single_unhidden(&self) -> bool {
        self.hidden_dirs.is_empty()
            && self.grants.len() == 1
            && self.grants[0].dir() == &self.command_dir
            && self.grants[0].access() == SandboxDirAccess::ReadWrite
    }
}

#[cfg(test)]
#[path = "scope_tests.rs"]
mod tests;
