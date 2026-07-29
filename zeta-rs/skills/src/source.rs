use crate::{SkillError, SkillErrorKind, SkillSourceId};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SkillSourceKind {
    BuiltIn,
    User,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SkillTrust {
    BuiltInVerified,
    UserManaged,
}

/// Consumer-visible Skill source provenance without its private host root.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SkillSourceView {
    id: SkillSourceId,
    kind: SkillSourceKind,
    trust: SkillTrust,
}

impl SkillSourceView {
    pub fn id(&self) -> &SkillSourceId {
        &self.id
    }

    pub fn kind(&self) -> SkillSourceKind {
        self.kind
    }

    pub fn trust(&self) -> SkillTrust {
        self.trust
    }
}

/// Validated, read-only root supplied by the host to catalog discovery.
///
/// Hosts must construct a new handle when configuration changes. The handle validates that the
/// initial root is a real directory and stores its canonical path privately; each scan rechecks
/// containment and file types because local files can still change after construction.
#[derive(Clone)]
pub struct SkillSourceRoot {
    view: SkillSourceView,
    canonical_root: PathBuf,
}

impl SkillSourceRoot {
    pub fn built_in(id: SkillSourceId, root: impl AsRef<Path>) -> Result<Self, SkillError> {
        Self::new(
            id,
            SkillSourceKind::BuiltIn,
            SkillTrust::BuiltInVerified,
            root.as_ref(),
        )
    }

    pub fn user(id: SkillSourceId, root: impl AsRef<Path>) -> Result<Self, SkillError> {
        Self::new(
            id,
            SkillSourceKind::User,
            SkillTrust::UserManaged,
            root.as_ref(),
        )
    }

    pub fn view(&self) -> &SkillSourceView {
        &self.view
    }

    pub(crate) fn canonical_root(&self) -> &Path {
        &self.canonical_root
    }

    fn new(
        id: SkillSourceId,
        kind: SkillSourceKind,
        trust: SkillTrust,
        root: &Path,
    ) -> Result<Self, SkillError> {
        let metadata = fs::symlink_metadata(root).map_err(|_| source_unavailable(&id))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(source_unavailable(&id));
        }
        let canonical_root = root.canonicalize().map_err(|_| source_unavailable(&id))?;
        Ok(Self {
            view: SkillSourceView { id, kind, trust },
            canonical_root,
        })
    }
}

impl fmt::Debug for SkillSourceRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SkillSourceRoot")
            .field("view", &self.view)
            .field("canonical_root", &"<private>")
            .finish()
    }
}

fn source_unavailable(id: &SkillSourceId) -> SkillError {
    SkillError::new(
        SkillErrorKind::SourceUnavailable,
        format!("skill source '{id}' does not exist or is not a real readable directory"),
    )
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
