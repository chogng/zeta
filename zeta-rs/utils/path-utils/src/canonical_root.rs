use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::comparison::normalize_canonical_for_comparison;

/// One existing host-filesystem path used as a canonical containment boundary.
///
/// Callers retain ownership of authorization and symlink policy. This type only
/// canonicalizes paths and verifies host-filesystem identity containment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalPathRoot {
    path: PathBuf,
    comparison_path: PathBuf,
}

impl CanonicalPathRoot {
    /// Canonicalizes an existing path for repeated containment checks.
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().canonicalize()?;
        let comparison_path = normalize_canonical_for_comparison(path.clone());
        Ok(Self {
            path,
            comparison_path,
        })
    }

    /// Returns the canonical host path represented by this boundary.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Canonicalizes an existing path and verifies that it is within this boundary.
    ///
    /// The boundary itself is considered contained. Symlinks are followed by
    /// canonicalization; callers that prohibit symlinks must enforce that policy
    /// before calling this method.
    pub fn canonicalize_within(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, CanonicalContainmentError> {
        let path = path
            .as_ref()
            .canonicalize()
            .map_err(CanonicalContainmentError::Unavailable)?;
        let comparison_path = normalize_canonical_for_comparison(path.clone());
        if comparison_path.starts_with(&self.comparison_path) {
            Ok(path)
        } else {
            Err(CanonicalContainmentError::OutsideRoot)
        }
    }
}

/// Failure to canonicalize an existing path within a [`CanonicalPathRoot`].
#[derive(Debug)]
pub enum CanonicalContainmentError {
    /// The candidate path could not be canonicalized.
    Unavailable(io::Error),
    /// The canonical candidate is not contained by the canonical root.
    OutsideRoot,
}

impl fmt::Display for CanonicalContainmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(error) => write!(formatter, "path is unavailable: {error}"),
            Self::OutsideRoot => formatter.write_str("path is outside the canonical root"),
        }
    }
}

impl std::error::Error for CanonicalContainmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Unavailable(error) => Some(error),
            Self::OutsideRoot => None,
        }
    }
}
