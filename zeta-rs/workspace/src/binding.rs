use crate::WorkspaceRoot;
use crate::WorkspaceTrustId;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use ts_rs::TS;

/// Durable Session binding to one canonical Workspace authority.
///
/// Product hosts use the root to reopen the Workspace and must recompute the authority ID before
/// enabling execution. The ID is never a substitute for containment or a current trust decision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceBinding {
    pub authority_id: WorkspaceTrustId,
    pub root: PathBuf,
}

impl WorkspaceBinding {
    /// Freezes the canonical root and its corresponding trust identity.
    pub fn from_root(root: &WorkspaceRoot) -> Self {
        Self {
            authority_id: root.trust_id(),
            root: root.canonical_path().to_path_buf(),
        }
    }

    /// Returns true only when a currently opened Workspace has the same canonical authority.
    pub fn matches_root(&self, root: &WorkspaceRoot) -> bool {
        self.authority_id == root.trust_id() && self.root == root.canonical_path()
    }

    /// Returns the canonical Workspace root recorded with the Session.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
#[path = "binding_tests.rs"]
mod tests;
