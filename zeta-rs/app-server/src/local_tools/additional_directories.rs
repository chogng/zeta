use std::collections::BTreeMap;
use std::sync::RwLock;
use zeta_protocol::SessionId;
use zeta_workspace::TrustedWorkspace;

/// Publishes the exact additional roots that local file tools may use for each Session.
///
/// Workspace authority updates this registry only after trust and scope validation. Tool
/// preparation reads a snapshot and still checks the root-bound lease before using it.
#[derive(Default)]
pub(crate) struct SessionAdditionalDirectoryAccess {
    roots: RwLock<BTreeMap<SessionId, Vec<TrustedWorkspace>>>,
}

impl SessionAdditionalDirectoryAccess {
    pub(crate) fn clear(&self) {
        self.roots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    pub(crate) fn replace(&self, session_id: SessionId, roots: Vec<TrustedWorkspace>) {
        let mut current = self
            .roots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if roots.is_empty() {
            current.remove(&session_id);
        } else {
            current.insert(session_id, roots);
        }
    }

    pub(crate) fn roots(&self, session_id: &SessionId) -> Vec<TrustedWorkspace> {
        self.roots
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }
}
