use crate::AgentEnvironmentError;
use crate::error::absolute_path;
use std::path::Path;
use std::path::PathBuf;
use zeta_utils_absolute_path::AbsolutePathBuf;

/// Ordered filesystem roots visible to the Agent.
///
/// The primary root always remains first. Additional roots are sorted and deduplicated without
/// changing the primary working directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceRoots {
    roots: Vec<AbsolutePathBuf>,
}

impl WorkspaceRoots {
    /// Validates and orders one primary root plus zero or more additional roots.
    pub fn new(
        primary_root: PathBuf,
        additional_roots: impl IntoIterator<Item = PathBuf>,
    ) -> Result<Self, AgentEnvironmentError> {
        let primary_root = absolute_path("primary workspace root", primary_root)?;
        let mut additional_roots = additional_roots
            .into_iter()
            .map(|root| absolute_path("additional workspace root", root))
            .collect::<Result<Vec<_>, AgentEnvironmentError>>()?;
        additional_roots.retain(|root| root != &primary_root);
        additional_roots.sort();
        additional_roots.dedup();
        let mut roots = Vec::with_capacity(additional_roots.len().saturating_add(1));
        roots.push(primary_root);
        roots.extend(additional_roots);
        Ok(Self { roots })
    }

    /// Returns the unchanged primary working root.
    pub fn primary(&self) -> &Path {
        self.roots
            .first()
            .expect("WorkspaceRoots always contains its primary root")
    }

    /// Returns the primary root followed by sorted additional roots.
    pub fn as_slice(&self) -> &[AbsolutePathBuf] {
        &self.roots
    }
}

#[cfg(test)]
#[path = "workspace_roots_tests.rs"]
mod tests;
