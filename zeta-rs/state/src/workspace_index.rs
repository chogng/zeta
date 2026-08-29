#![deny(unsafe_code)]

//! Profile-local database paths and cross-process lifecycle locks for rebuildable indexes.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use zeta_utils_path::CanonicalPathRoot;
use zeta_utils_path::NoSymlinkPathError;
use zeta_utils_path::NoSymlinkPathStatus;
use zeta_workspace::WorkspaceTrustId;

const INDEXES_DIRECTORY: &str = "indexes";
const LOCKS_DIRECTORY: &str = "locks";
const WORKSPACES_DIRECTORY: &str = "workspaces";
const GLOBAL_LOCK_FILE: &str = "indexes.lock";

/// One rebuildable index owned by a Workspace.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum WorkspaceIndexKind {
    AgentGrep,
    Codebase,
}

impl WorkspaceIndexKind {
    pub const ALL: [Self; 2] = [Self::AgentGrep, Self::Codebase];

    pub const fn directory_name(self) -> &'static str {
        match self {
            Self::AgentGrep => "agent-grep",
            Self::Codebase => "codebase",
        }
    }

    fn lock_file_name(self) -> &'static str {
        match self {
            Self::AgentGrep => "agent-grep.lock",
            Self::Codebase => "codebase.lock",
        }
    }
}

/// Result of an explicit index deletion request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClearOutcome {
    Cleared,
    AlreadyAbsent,
    InUse,
}

/// Profile-level owner of durable database paths and rebuildable Workspace index locks.
#[derive(Clone, Debug)]
pub struct StateRuntime {
    profile_root: PathBuf,
    database_path: PathBuf,
    connectors_database_path: PathBuf,
    cloud_codebase_root: PathBuf,
    writer_leases_root: PathBuf,
    cache_root: PathBuf,
    cache_boundary: CanonicalPathRoot,
    locks_root: PathBuf,
    workspaces_root: PathBuf,
}

impl StateRuntime {
    pub fn open(profile_root: impl AsRef<Path>) -> io::Result<Self> {
        fs::create_dir_all(profile_root.as_ref())?;
        let profile_root = fs::canonicalize(profile_root.as_ref())?;
        let cache_root = profile_root.join("cache");
        let locks_root = cache_root.join(LOCKS_DIRECTORY);
        let workspaces_root = cache_root.join(WORKSPACES_DIRECTORY);
        let profile_boundary = CanonicalPathRoot::new(&profile_root)?;
        ensure_directory_without_symlinks(&profile_boundary, &cache_root)?;
        let cache_boundary = CanonicalPathRoot::new(&cache_root)?;
        ensure_directory_without_symlinks(&cache_boundary, &locks_root)?;
        ensure_directory_without_symlinks(&cache_boundary, &locks_root.join(WORKSPACES_DIRECTORY))?;
        ensure_directory_without_symlinks(&cache_boundary, &workspaces_root)?;
        Ok(Self {
            database_path: profile_root.join("state.sqlite3"),
            connectors_database_path: profile_root.join("connectors.sqlite3"),
            cloud_codebase_root: profile_root.join("state").join("cloud-codebase"),
            writer_leases_root: profile_root.join("leases"),
            profile_root,
            cache_root,
            cache_boundary,
            locks_root,
            workspaces_root,
        })
    }

    /// Returns the profile root that contains durable state and rebuildable cache data.
    pub fn profile_root(&self) -> &Path {
        &self.profile_root
    }

    /// Returns the single durable profile database path.
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// Returns the durable Connector authority database path for this profile.
    pub fn connectors_database_path(&self) -> &Path {
        &self.connectors_database_path
    }

    /// Returns the root containing durable Cloud Codebase state for this profile.
    pub fn cloud_codebase_root(&self) -> &Path {
        &self.cloud_codebase_root
    }

    /// Returns the directory containing Session and Thread writer lease files.
    pub fn writer_leases_root(&self) -> &Path {
        &self.writer_leases_root
    }

    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }

    pub fn acquire(
        &self,
        workspace: &WorkspaceTrustId,
        kind: WorkspaceIndexKind,
    ) -> io::Result<WorkspaceIndexLease> {
        let global_lock = self.open_global_lock()?;
        fs2::FileExt::lock_shared(&global_lock)?;

        let index_lock = self.open_index_lock(workspace, kind)?;
        fs2::FileExt::lock_shared(&index_lock)?;

        let directory = self.index_directory(workspace, kind);
        ensure_directory_without_symlinks(&self.cache_boundary, &directory)?;
        Ok(WorkspaceIndexLease {
            directory,
            _global_lock: global_lock,
            _index_lock: index_lock,
        })
    }

    pub fn clear_index(
        &self,
        workspace: &WorkspaceTrustId,
        kind: WorkspaceIndexKind,
    ) -> io::Result<ClearOutcome> {
        let global_lock = self.open_global_lock()?;
        fs2::FileExt::lock_shared(&global_lock)?;

        let index_lock = self.open_index_lock(workspace, kind)?;
        if !try_lock_exclusive(&index_lock)? {
            return Ok(ClearOutcome::InUse);
        }
        remove_directory(&self.cache_boundary, &self.index_directory(workspace, kind))
    }

    pub fn clear_workspace(&self, workspace: &WorkspaceTrustId) -> io::Result<ClearOutcome> {
        let global_lock = self.open_global_lock()?;
        fs2::FileExt::lock_shared(&global_lock)?;

        let mut index_locks = Vec::with_capacity(WorkspaceIndexKind::ALL.len());
        for kind in WorkspaceIndexKind::ALL {
            let lock = self.open_index_lock(workspace, kind)?;
            if !try_lock_exclusive(&lock)? {
                return Ok(ClearOutcome::InUse);
            }
            index_locks.push(lock);
        }
        remove_directory(&self.cache_boundary, &self.workspace_directory(workspace))
    }

    pub fn clear_all(&self) -> io::Result<ClearOutcome> {
        let global_lock = self.open_global_lock()?;
        if !try_lock_exclusive(&global_lock)? {
            return Ok(ClearOutcome::InUse);
        }
        let outcome = remove_directory(&self.cache_boundary, &self.workspaces_root)?;
        ensure_directory_without_symlinks(&self.cache_boundary, &self.workspaces_root)?;
        Ok(outcome)
    }

    pub fn index_directory(
        &self,
        workspace: &WorkspaceTrustId,
        kind: WorkspaceIndexKind,
    ) -> PathBuf {
        self.workspace_directory(workspace)
            .join(INDEXES_DIRECTORY)
            .join(kind.directory_name())
    }

    fn workspace_directory(&self, workspace: &WorkspaceTrustId) -> PathBuf {
        self.workspaces_root.join(workspace_digest(workspace))
    }

    fn open_global_lock(&self) -> io::Result<File> {
        open_lock_file(
            &self.cache_boundary,
            &self.locks_root.join(GLOBAL_LOCK_FILE),
        )
    }

    fn open_index_lock(
        &self,
        workspace: &WorkspaceTrustId,
        kind: WorkspaceIndexKind,
    ) -> io::Result<File> {
        open_lock_file(
            &self.cache_boundary,
            &self
                .locks_root
                .join(WORKSPACES_DIRECTORY)
                .join(workspace_digest(workspace))
                .join(kind.lock_file_name()),
        )
    }
}

/// Shared lifecycle lock held while an index is open or being rebuilt.
#[derive(Debug)]
pub struct WorkspaceIndexLease {
    directory: PathBuf,
    _global_lock: File,
    _index_lock: File,
}

impl WorkspaceIndexLease {
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

fn workspace_digest(workspace: &WorkspaceTrustId) -> &str {
    workspace
        .as_str()
        .strip_prefix("sha256:")
        .expect("WorkspaceTrustId always contains the sha256 prefix")
}

fn open_lock_file(boundary: &CanonicalPathRoot, path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        ensure_directory_without_symlinks(boundary, parent)?;
    }
    boundary
        .inspect_without_symlinks(path)
        .map_err(no_symlink_path_error)?;
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
}

fn try_lock_exclusive(file: &File) -> io::Result<bool> {
    match fs2::FileExt::try_lock_exclusive(file) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(false),
        Err(error) => Err(error),
    }
}

fn remove_directory(boundary: &CanonicalPathRoot, path: &Path) -> io::Result<ClearOutcome> {
    match boundary
        .inspect_without_symlinks(path)
        .map_err(no_symlink_path_error)?
    {
        NoSymlinkPathStatus::Existing => match fs::remove_dir_all(path) {
            Ok(()) => Ok(ClearOutcome::Cleared),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(ClearOutcome::AlreadyAbsent)
            }
            Err(error) => Err(error),
        },
        NoSymlinkPathStatus::Missing => Ok(ClearOutcome::AlreadyAbsent),
    }
}

fn ensure_directory_without_symlinks(boundary: &CanonicalPathRoot, path: &Path) -> io::Result<()> {
    let status = boundary
        .inspect_without_symlinks(path)
        .map_err(no_symlink_path_error)?;
    if status == NoSymlinkPathStatus::Missing {
        fs::create_dir_all(path)?;
    }
    let status = boundary
        .inspect_without_symlinks(path)
        .map_err(no_symlink_path_error)?;
    if status != NoSymlinkPathStatus::Existing || !fs::symlink_metadata(path)?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("state path is not a directory: {}", path.display()),
        ));
    }
    Ok(())
}

fn no_symlink_path_error(error: NoSymlinkPathError) -> io::Error {
    let kind = match &error {
        NoSymlinkPathError::Unavailable { source, .. } => source.kind(),
        NoSymlinkPathError::OutsideRoot(_) | NoSymlinkPathError::Symlink(_) => {
            io::ErrorKind::InvalidData
        }
    };
    io::Error::new(kind, error)
}

#[cfg(test)]
#[path = "workspace_index_tests.rs"]
mod tests;
