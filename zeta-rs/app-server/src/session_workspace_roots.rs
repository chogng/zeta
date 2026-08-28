use std::collections::BTreeMap;
use std::sync::RwLock;
use zeta_protocol::SessionId;
use zeta_workspace::TrustedWorkspace;

/// Publishes the exact additional Workspace roots authorized for each Session.
///
/// Workspace authority updates this registry only after trust and scope validation. File tools
/// and model-environment capture read the same snapshot, so access and model-visible roots cannot
/// drift apart.
#[derive(Default)]
pub(crate) struct SessionWorkspaceRoots {
    additional_roots: RwLock<BTreeMap<SessionId, Vec<TrustedWorkspace>>>,
}

impl SessionWorkspaceRoots {
    pub(crate) fn clear(&self) {
        self.additional_roots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    pub(crate) fn replace_additional(&self, session_id: SessionId, roots: Vec<TrustedWorkspace>) {
        let mut current = self
            .additional_roots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if roots.is_empty() {
            current.remove(&session_id);
        } else {
            current.insert(session_id, roots);
        }
    }

    pub(crate) fn additional_roots(&self, session_id: &SessionId) -> Vec<TrustedWorkspace> {
        self.additional_roots
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }
}
